use crate::AppState;
use crate::db::player::PlayerRepository;
use crate::db::class::ClassRepository;
use crate::db::race::RaceRepository;
use crate::models::Player;
use crate::{CharCreationStep, CharCreationState};
use std::sync::Arc;
use anyhow::Result;

const TOWN_SQUARE_VNUM: &str = "vnum_3001"; // Midgaard town square

/// Check if a user ID is in the wizards list
fn is_wizard(user_id: &str) -> bool {
    // Check environment variable for wizards list
    if let Ok(wizards_env) = std::env::var("WIZARDS") {
        for id in wizards_env.split(',') {
            if id.trim() == user_id {
                return true;
            }
        }
    }
    false
}

/// Start the character creation process for a new player
pub async fn start_character_creation(state: Arc<AppState>, user_id: &str) -> Result<()> {
    // Initialize character creation state
    {
        let mut states = state.char_creation_states.lock().unwrap();
        states.insert(user_id.to_string(), CharCreationState::new());
    }

    // Send welcome message and ask for name
    let welcome_msg = r#"*Welcome to SlackMUD!*

Let's create your character. First, what would you like your character's name to be?

_Your name must be a single word and will be unique to you._

Please type your desired character name:"#;

    state.slack_client.send_dm(user_id, welcome_msg).await?;
    Ok(())
}

/// Handle a message during character creation
pub async fn handle_character_creation_input(
    state: Arc<AppState>,
    user_id: &str,
    input: &str,
) -> Result<bool> {
    let input = input.trim();

    // Get current state
    let current_state = {
        let states = state.char_creation_states.lock().unwrap();
        states.get(user_id).cloned()
    };

    let Some(mut char_state) = current_state else {
        return Ok(false); // Not in character creation
    };

    match char_state.step {
        CharCreationStep::Name => {
            handle_name_input(state.clone(), user_id, input, &mut char_state).await?;
        }
        CharCreationStep::Gender => {
            handle_gender_input(state.clone(), user_id, input, &mut char_state).await?;
        }
        CharCreationStep::Race => {
            handle_race_input(state.clone(), user_id, input, &mut char_state).await?;
        }
        CharCreationStep::Class => {
            handle_class_input(state.clone(), user_id, input, &mut char_state).await?;
        }
    }

    Ok(true)
}

async fn handle_name_input(
    state: Arc<AppState>,
    user_id: &str,
    name: &str,
    char_state: &mut CharCreationState,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());

    // Validate name: single word
    if name.contains(char::is_whitespace) {
        state.slack_client.send_dm(
            user_id,
            "Your name must be a single word with no spaces. Please try again:"
        ).await?;
        return Ok(());
    }

    // Validate name: only letters
    if !name.chars().all(|c| c.is_alphabetic()) {
        state.slack_client.send_dm(
            user_id,
            "Your name can only contain letters. Please try again:"
        ).await?;
        return Ok(());
    }

    // Validate name: reasonable length
    if name.len() < 2 || name.len() > 20 {
        state.slack_client.send_dm(
            user_id,
            "Your name must be between 2 and 20 characters. Please try again:"
        ).await?;
        return Ok(());
    }

    // Check if name is already taken
    if player_repo.is_name_taken(name).await? {
        state.slack_client.send_dm(
            user_id,
            &format!("The name '{}' is already taken. Please choose another name:", name)
        ).await?;
        return Ok(());
    }

    // Name is valid!
    char_state.name = Some(name.to_string());
    char_state.step = CharCreationStep::Gender;

    // Update state
    {
        let mut states = state.char_creation_states.lock().unwrap();
        states.insert(user_id.to_string(), char_state.clone());
    }

    // Ask for gender
    let gender_msg = format!(
        r#"Great! Your character's name will be *{}*.

What is your character's gender?

Please type one of the following:
• `male`
• `female`
• `neutral`"#,
        name
    );

    state.slack_client.send_dm(user_id, &gender_msg).await?;
    Ok(())
}

async fn handle_gender_input(
    state: Arc<AppState>,
    user_id: &str,
    input: &str,
    char_state: &mut CharCreationState,
) -> Result<()> {
    let gender = input.to_lowercase();

    if !matches!(gender.as_str(), "male" | "female" | "neutral") {
        state.slack_client.send_dm(
            user_id,
            "Please choose `male`, `female`, or `neutral`:"
        ).await?;
        return Ok(());
    }

    char_state.gender = Some(gender.clone());
    char_state.step = CharCreationStep::Race;

    // Update state
    {
        let mut states = state.char_creation_states.lock().unwrap();
        states.insert(user_id.to_string(), char_state.clone());
    }

    // Get all races and ask for selection
    let race_repo = RaceRepository::new(state.db_pool.clone());
    let races = race_repo.get_all().await?;

    let mut race_msg = format!("You selected *{}*.\n\nChoose your race:\n\n", gender);
    for race in &races {
        race_msg.push_str(&format!("• `{}` - {}\n", race.name.to_lowercase(), race.description));
    }
    race_msg.push_str("\nPlease type the name of your race:");

    state.slack_client.send_dm(user_id, &race_msg).await?;
    Ok(())
}

async fn handle_race_input(
    state: Arc<AppState>,
    user_id: &str,
    input: &str,
    char_state: &mut CharCreationState,
) -> Result<()> {
    let race_repo = RaceRepository::new(state.db_pool.clone());
    let races = race_repo.get_all().await?;

    let race_name = input.to_lowercase();
    let selected_race = races.iter().find(|r| r.name.to_lowercase() == race_name);

    match selected_race {
        Some(race) => {
            char_state.race_id = Some(race.id);
            char_state.step = CharCreationStep::Class;

            // Update state
            {
                let mut states = state.char_creation_states.lock().unwrap();
                states.insert(user_id.to_string(), char_state.clone());
            }

            // Get all classes and ask for selection
            let class_repo = ClassRepository::new(state.db_pool.clone());
            let classes = class_repo.get_all().await?;

            let mut class_msg = format!("You selected *{}*.\n\nChoose your class:\n\n", race.name);
            for class in &classes {
                class_msg.push_str(&format!("• `{}` - {}\n", class.name.to_lowercase(), class.description));
            }
            class_msg.push_str("\nPlease type the name of your class:");

            state.slack_client.send_dm(user_id, &class_msg).await?;
        }
        None => {
            state.slack_client.send_dm(
                user_id,
                &format!("'{}' is not a valid race. Please choose from the list above:", input)
            ).await?;
        }
    }

    Ok(())
}

async fn handle_class_input(
    state: Arc<AppState>,
    user_id: &str,
    input: &str,
    char_state: &mut CharCreationState,
) -> Result<()> {
    let class_repo = ClassRepository::new(state.db_pool.clone());
    let classes = class_repo.get_all().await?;

    let class_name = input.to_lowercase();
    let selected_class = classes.iter().find(|c| c.name.to_lowercase() == class_name);

    match selected_class {
        Some(class) => {
            char_state.class_id = Some(class.id);

            // Character creation complete! Create the player
            let player_repo = PlayerRepository::new(state.db_pool.clone());

            let mut player = Player::new(
                user_id.to_string(),
                char_state.name.as_ref().unwrap().clone(),
            );
            player.gender = char_state.gender.clone();
            player.race_id = char_state.race_id;
            player.class_id = char_state.class_id;
            player.current_channel_id = Some(TOWN_SQUARE_VNUM.to_string());

            // Check if this user is a wizard
            let is_wizard_user = is_wizard(user_id);
            if is_wizard_user {
                player.level = 50;
            }

            player_repo.create(&player).await?;

            // Remove from character creation state
            {
                let mut states = state.char_creation_states.lock().unwrap();
                states.remove(user_id);
            }

            // Broadcast arrival to the room
            let (arrival_msg, first_person_msg) = if is_wizard_user {
                // Wizard arrival - use god/goddess based on gender
                let deity_title = match char_state.gender.as_deref() {
                    Some("male") => "god",
                    Some("female") => "goddess",
                    _ => "deity",
                };
                (
                    format!("_The {} {} materializes!_", deity_title, player.name),
                    format!("_You materialize as a {} in the town square._", deity_title),
                )
            } else {
                // Normal player arrival
                (
                    format!("_{} fades into existence!_", player.name),
                    "_You fade into existence in the town square._".to_string(),
                )
            };

            let _ = crate::handlers::broadcast_room_action(
                &state,
                TOWN_SQUARE_VNUM,
                &arrival_msg,
                Some(user_id),
                Some(&first_person_msg),
            ).await;

            // Send completion message
            let mut completion_msg = format!(
                r#"*Character Created!*

Name: *{}*
Gender: *{}*
Race: *{}*
Class: *{}*{}"#,
                player.name,
                char_state.gender.as_ref().unwrap(),
                classes.iter().find(|c| Some(c.id) == char_state.race_id).map(|c| c.name.as_str()).unwrap_or("Unknown"),
                class.name,
                if is_wizard_user { "\nLevel: *50 (Wizard)*" } else { "" }
            );

            completion_msg.push_str("\n\nYou awaken in the town square of Midgaard. Your adventure begins now!\n\n");
            completion_msg.push_str("Type `/mud look` to see your surroundings, or `/mud help` for a list of commands.");

            state.slack_client.send_dm(user_id, &completion_msg).await?;
        }
        None => {
            state.slack_client.send_dm(
                user_id,
                &format!("'{}' is not a valid class. Please choose from the list above:", input)
            ).await?;
        }
    }

    Ok(())
}

/// Check if a user is currently in character creation
pub fn is_in_character_creation(state: &Arc<AppState>, user_id: &str) -> bool {
    let states = state.char_creation_states.lock().unwrap();
    states.contains_key(user_id)
}
