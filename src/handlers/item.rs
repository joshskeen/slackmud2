use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::object::{ObjectRepository, ObjectInstanceRepository};
use std::sync::Arc;
use anyhow::Result;

/// Handle get/take command - pick up an object from the room
pub async fn handle_get(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud get <item>`\nExample: `/mud get barrel`"
        ).await?;
        return Ok(());
    }

    // Get all object instances in the room
    let instances = object_instance_repo.get_in_room(&room_id).await?;

    // Find matching object
    let mut found_instance = None;
    let mut found_object = None;

    for instance in instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    if let (Some(instance), Some(object)) = (found_instance, found_object) {
        // Move object from room to player inventory
        object_instance_repo.update_location(
            instance.id,
            "player",
            &player.slack_user_id,
        ).await?;

        // Send success message
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You pick up {}.", object.short_description)
        ).await?;

        // Broadcast action to room
        let third_person = format!("_{} picks up {}._", player.name, object.short_description);
        let first_person = format!("_You pick up {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&command.user_id),
            Some(&first_person),
        ).await?;
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You don't see '{}' here.", item_name)
        ).await?;
    }

    Ok(())
}

/// Handle get command from DM
pub async fn handle_get_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `get <item>`\nExample: `get barrel`"
        ).await?;
        return Ok(());
    }

    // Get all object instances in the room
    let instances = object_instance_repo.get_in_room(&room_id).await?;

    // Find matching object
    let mut found_instance = None;
    let mut found_object = None;

    for instance in instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    if let (Some(instance), Some(object)) = (found_instance, found_object) {
        // Move object from room to player inventory
        object_instance_repo.update_location(
            instance.id,
            "player",
            &player.slack_user_id,
        ).await?;

        // Send success message
        state.slack_client.send_dm(
            &user_id,
            &format!("You pick up {}.", object.short_description)
        ).await?;

        // Broadcast action to room
        let third_person = format!("_{} picks up {}._", player.name, object.short_description);
        let first_person = format!("_You pick up {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&user_id),
            Some(&first_person),
        ).await?;
    } else {
        state.slack_client.send_dm(
            &user_id,
            &format!("You don't see '{}' here.", item_name)
        ).await?;
    }

    Ok(())
}

/// Handle drop command - drop an object from inventory into the room
pub async fn handle_drop(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud drop <item>`\nExample: `/mud drop barrel`"
        ).await?;
        return Ok(());
    }

    // Get all object instances in player's inventory
    let instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    // Find matching object
    let mut found_instance = None;
    let mut found_object = None;

    for instance in instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    if let (Some(instance), Some(object)) = (found_instance, found_object) {
        // Move object from player inventory to room
        object_instance_repo.update_location(
            instance.id,
            "room",
            &room_id,
        ).await?;

        // Send success message
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You drop {}.", object.short_description)
        ).await?;

        // Broadcast action to room
        let third_person = format!("_{} drops {}._", player.name, object.short_description);
        let first_person = format!("_You drop {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&command.user_id),
            Some(&first_person),
        ).await?;
    } else {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You aren't carrying '{}'.", item_name)
        ).await?;
    }

    Ok(())
}

/// Handle drop command from DM
pub async fn handle_drop_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `drop <item>`\nExample: `drop barrel`"
        ).await?;
        return Ok(());
    }

    // Get all object instances in player's inventory
    let instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    // Find matching object
    let mut found_instance = None;
    let mut found_object = None;

    for instance in instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    if let (Some(instance), Some(object)) = (found_instance, found_object) {
        // Move object from player inventory to room
        object_instance_repo.update_location(
            instance.id,
            "room",
            &room_id,
        ).await?;

        // Send success message
        state.slack_client.send_dm(
            &user_id,
            &format!("You drop {}.", object.short_description)
        ).await?;

        // Broadcast action to room
        let third_person = format!("_{} drops {}._", player.name, object.short_description);
        let first_person = format!("_You drop {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&user_id),
            Some(&first_person),
        ).await?;
    } else {
        state.slack_client.send_dm(
            &user_id,
            &format!("You aren't carrying '{}'.", item_name)
        ).await?;
    }

    Ok(())
}

/// Handle inventory command - show what player is carrying
pub async fn handle_inventory(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Get all object instances in player's inventory
    let instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    if instances.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "*Inventory:*\nYou aren't carrying anything."
        ).await?;
    } else {
        let mut inventory_text = String::from("*Inventory:*\n");
        for instance in instances {
            if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                inventory_text.push_str(&format!("• {}\n", object.short_description));
            }
        }
        state.slack_client.send_dm(&command.user_id, &inventory_text).await?;
    }

    Ok(())
}

/// Handle inventory command from DM
pub async fn handle_inventory_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Get all object instances in player's inventory
    let instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    if instances.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "*Inventory:*\nYou aren't carrying anything."
        ).await?;
    } else {
        let mut inventory_text = String::from("*Inventory:*\n");
        for instance in instances {
            if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                inventory_text.push_str(&format!("• {}\n", object.short_description));
            }
        }
        state.slack_client.send_dm(&user_id, &inventory_text).await?;
    }

    Ok(())
}

const WIZARD_LEVEL: i32 = 50;

/// Handle manifest command - wizard creates an item by vnum or name
pub async fn handle_manifest(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You must be a wizard (level {}) to manifest items.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first! Use `/mud look` in a channel to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let search_term = args.trim();
    if search_term.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud manifest <vnum|name>`\nExample: `/mud manifest 3001` or `/mud manifest sword`"
        ).await?;
        return Ok(());
    }

    // Try to parse as vnum first
    let object = if let Ok(vnum) = search_term.parse::<i32>() {
        object_repo.get_by_vnum(vnum).await?
    } else {
        // Search by keyword in all objects
        find_object_by_keyword(&object_repo, search_term).await?
    };

    match object {
        Some(obj) => {
            // Create object instance in the room
            let instance = crate::models::ObjectInstance::new_in_room(obj.vnum, room_id.clone());
            object_instance_repo.create(&instance).await?;

            // Send success message to wizard
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You manifest {}.", obj.short_description)
            ).await?;

            // Broadcast dramatic action to room
            let third_person = format!(
                "_{} utters a strange incantation and performs a series of hand gestures. {} springs into existence!_",
                player.name,
                obj.short_description
            );
            let first_person = format!(
                "_You utter a strange incantation and perform a series of hand gestures. {} springs into existence!_",
                obj.short_description
            );
            super::broadcast_room_action(
                &state,
                &room_id,
                &third_person,
                Some(&command.user_id),
                Some(&first_person),
            ).await?;
        }
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("No item found matching '{}'. Use `/mud listitems` to see available items.", search_term)
            ).await?;
        }
    }

    Ok(())
}

/// Helper function to find an object by keyword
async fn find_object_by_keyword(
    object_repo: &ObjectRepository,
    keyword: &str,
) -> Result<Option<crate::models::Object>> {
    // Query all objects and find first match
    let objects = sqlx::query_as::<_, crate::models::Object>(
        "SELECT * FROM objects ORDER BY vnum"
    )
    .fetch_all(object_repo.pool())
    .await?;

    for object in objects {
        if object.matches_keyword(keyword) {
            return Ok(Some(object));
        }
    }

    Ok(None)
}

pub async fn handle_manifest_dm(
    state: Arc<AppState>,
    user_id: String,
    user_name: String,
    args: &str,
) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let player = player_repo.get_or_create(user_id.clone(), user_name).await?;

    // Check if player is a wizard
    if player.level < WIZARD_LEVEL {
        state.slack_client.send_dm(
            &user_id,
            &format!("You must be a wizard (level {}) to manifest items.", WIZARD_LEVEL)
        ).await?;
        return Ok(());
    }

    // Check if player has a current room
    let room_id = match player.current_channel_id {
        Some(id) => id,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first! Use `look` to enter a room."
            ).await?;
            return Ok(());
        }
    };

    let search_term = args.trim();
    if search_term.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `manifest <vnum|name>`\nExample: `manifest 3001` or `manifest sword`"
        ).await?;
        return Ok(());
    }

    // Try to parse as vnum first
    let object = if let Ok(vnum) = search_term.parse::<i32>() {
        object_repo.get_by_vnum(vnum).await?
    } else {
        // Search by keyword in all objects
        find_object_by_keyword(&object_repo, search_term).await?
    };

    match object {
        Some(obj) => {
            // Create object instance in the room
            let instance = crate::models::ObjectInstance::new_in_room(obj.vnum, room_id.clone());
            object_instance_repo.create(&instance).await?;

            // Send success message to wizard
            state.slack_client.send_dm(
                &user_id,
                &format!("You manifest {}.", obj.short_description)
            ).await?;

            // Broadcast dramatic action to room
            let third_person = format!(
                "_{} utters a strange incantation and performs a series of hand gestures. {} springs into existence!_",
                player.name,
                obj.short_description
            );
            let first_person = format!(
                "_You utter a strange incantation and perform a series of hand gestures. {} springs into existence!_",
                obj.short_description
            );
            super::broadcast_room_action(
                &state,
                &room_id,
                &third_person,
                Some(&user_id),
                Some(&first_person),
            ).await?;
        }
        None => {
            state.slack_client.send_dm(
                &user_id,
                &format!("No item found matching '{}'. Use `listitems` to see available items.", search_term)
            ).await?;
        }
    }

    Ok(())
}
