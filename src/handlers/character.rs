use crate::AppState;
use crate::slack::{SlashCommand, Block};
use crate::db::player::PlayerRepository;
use crate::db::class::ClassRepository;
use crate::db::race::RaceRepository;
use std::sync::Arc;
use anyhow::Result;

pub async fn handle_character(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let class_repo = ClassRepository::new(state.db_pool.clone());
    let race_repo = RaceRepository::new(state.db_pool.clone());

    // Get or create the player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Get available classes and races
    let classes = class_repo.get_all().await?;
    let races = race_repo.get_all().await?;

    // Build character info message
    let mut blocks = vec![
        Block::section("*Your Character*"),
    ];

    // Show current character details
    let mut char_info = format!("*Name:* {}\n*Level:* {}\n*XP:* {}",
        player.name, player.level, player.experience_points);

    if let Some(class_id) = player.class_id {
        if let Some(class) = class_repo.get_by_id(class_id).await? {
            char_info.push_str(&format!("\n*Class:* {}", class.name));
        }
    } else {
        char_info.push_str("\n*Class:* _Not set_");
    }

    if let Some(race_id) = player.race_id {
        if let Some(race) = race_repo.get_by_id(race_id).await? {
            char_info.push_str(&format!("\n*Race:* {}", race.name));
        }
    } else {
        char_info.push_str("\n*Race:* _Not set_");
    }

    if let Some(gender) = &player.gender {
        char_info.push_str(&format!("\n*Gender:* {}", gender));
    } else {
        char_info.push_str("\n*Gender:* _Not set_");
    }

    blocks.push(Block::section(&char_info));

    // Show available classes
    let mut classes_text = String::from("*Available Classes:*\n");
    for class in &classes {
        classes_text.push_str(&format!("• *{}* - {}\n", class.name, class.description));
    }
    blocks.push(Block::section(&classes_text));

    // Show available races
    let mut races_text = String::from("*Available Races:*\n");
    for race in &races {
        races_text.push_str(&format!("• *{}* - {}\n", race.name, race.description));
    }
    blocks.push(Block::section(&races_text));

    // Instructions for customization
    // Note: In a full implementation, we'd use Slack's interactive components
    // For now, provide instructions for a future implementation
    let instructions = "*Character Customization*\n\n\
        _Character customization with interactive menus is coming soon!_\n\n\
        For now, your character is automatically created with default settings. \
        You can start playing with `/mud look` to explore the world!";

    blocks.push(Block::section(instructions));

    let dm_text = "Character Information";
    state.slack_client.send_dm_with_blocks(&command.user_id, dm_text, blocks).await?;

    Ok(())
}
