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

pub async fn handle_give(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Check if player has a current room
    let room_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "You need to be in a room first!"
            ).await?;
            return Ok(());
        }
    };

    // Parse args: "give <item> <player>" or "give <item> to <player>"
    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud give <item> <player>`\nExample: `/mud give sword bob`"
        ).await?;
        return Ok(());
    }

    // Split on "to" if present, otherwise split on whitespace
    let (item_name, target_name) = if let Some(to_pos) = args.find(" to ") {
        let (item, target) = args.split_at(to_pos);
        (item.trim(), target[4..].trim()) // Skip " to "
    } else {
        // Split on last whitespace to get item and target
        if let Some(last_space) = args.rfind(' ') {
            let (item, target) = args.split_at(last_space);
            (item.trim(), target.trim())
        } else {
            state.slack_client.send_dm(
                &command.user_id,
                "Usage: `/mud give <item> <player>`\nExample: `/mud give sword bob`"
            ).await?;
            return Ok(());
        }
    };

    if item_name.is_empty() || target_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud give <item> <player>`\nExample: `/mud give sword bob`"
        ).await?;
        return Ok(());
    }

    // Find the item in player's inventory or equipped
    let instances = object_instance_repo.get_by_owner(&player.slack_user_id).await?;

    let mut item_to_give = None;
    for instance in instances {
        let object = object_repo.get_by_vnum(instance.object_vnum).await?;
        if let Some(obj) = object {
            if obj.matches_keyword(item_name) {
                item_to_give = Some((instance, obj));
                break;
            }
        }
    }

    let (instance, object) = match item_to_give {
        Some(pair) => pair,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                "That's not yours to give!"
            ).await?;
            return Ok(());
        }
    };

    // Find target player in same room
    let target = find_player_in_room(&state, &room_id, target_name).await?;

    let target_player = match target {
        Some(p) => p,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You don't see '{}' here.", target_name)
            ).await?;
            return Ok(());
        }
    };

    // Can't give to yourself
    if target_player.slack_user_id == player.slack_user_id {
        state.slack_client.send_dm(
            &command.user_id,
            "You can't give items to yourself!"
        ).await?;
        return Ok(());
    }

    // Transfer the item
    object_instance_repo.transfer_to_player(instance.id, &target_player.slack_user_id).await?;

    // Send messages
    let first_person = format!("You give {} to {}.", object.short_description, target_player.name);
    let second_person = format!("{} gives you {}.", player.name, object.short_description);
    let third_person = format!("_{} gives {} to {}._", player.name, object.short_description, target_player.name);

    state.slack_client.send_dm(&command.user_id, &first_person).await?;
    state.slack_client.send_dm(&target_player.slack_user_id, &second_person).await?;

    super::broadcast_room_action(
        &state,
        &room_id,
        &third_person,
        Some(&command.user_id),
        Some(&first_person),
    ).await?;

    Ok(())
}

pub async fn handle_give_dm(
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
    let room_id = match &player.current_channel_id {
        Some(id) => id.clone(),
        None => {
            state.slack_client.send_dm(
                &user_id,
                "You need to be in a room first!"
            ).await?;
            return Ok(());
        }
    };

    // Parse args
    let args = args.trim();
    if args.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `give <item> <player>`\nExample: `give sword bob`"
        ).await?;
        return Ok(());
    }

    // Split on "to" if present, otherwise split on whitespace
    let (item_name, target_name) = if let Some(to_pos) = args.find(" to ") {
        let (item, target) = args.split_at(to_pos);
        (item.trim(), target[4..].trim())
    } else {
        if let Some(last_space) = args.rfind(' ') {
            let (item, target) = args.split_at(last_space);
            (item.trim(), target.trim())
        } else {
            state.slack_client.send_dm(
                &user_id,
                "Usage: `give <item> <player>`\nExample: `give sword bob`"
            ).await?;
            return Ok(());
        }
    };

    if item_name.is_empty() || target_name.is_empty() {
        state.slack_client.send_dm(
            &user_id,
            "Usage: `give <item> <player>`\nExample: `give sword bob`"
        ).await?;
        return Ok(());
    }

    // Find the item in player's inventory or equipped
    let instances = object_instance_repo.get_by_owner(&player.slack_user_id).await?;

    let mut item_to_give = None;
    for instance in instances {
        let object = object_repo.get_by_vnum(instance.object_vnum).await?;
        if let Some(obj) = object {
            if obj.matches_keyword(item_name) {
                item_to_give = Some((instance, obj));
                break;
            }
        }
    }

    let (instance, object) = match item_to_give {
        Some(pair) => pair,
        None => {
            state.slack_client.send_dm(
                &user_id,
                "That's not yours to give!"
            ).await?;
            return Ok(());
        }
    };

    // Find target player in same room
    let target = find_player_in_room(&state, &room_id, target_name).await?;

    let target_player = match target {
        Some(p) => p,
        None => {
            state.slack_client.send_dm(
                &user_id,
                &format!("You don't see '{}' here.", target_name)
            ).await?;
            return Ok(());
        }
    };

    // Can't give to yourself
    if target_player.slack_user_id == player.slack_user_id {
        state.slack_client.send_dm(
            &user_id,
            "You can't give items to yourself!"
        ).await?;
        return Ok(());
    }

    // Transfer the item
    object_instance_repo.transfer_to_player(instance.id, &target_player.slack_user_id).await?;

    // Send messages
    let first_person = format!("You give {} to {}.", object.short_description, target_player.name);
    let second_person = format!("{} gives you {}.", player.name, object.short_description);
    let third_person = format!("_{} gives {} to {}._", player.name, object.short_description, target_player.name);

    state.slack_client.send_dm(&user_id, &first_person).await?;
    state.slack_client.send_dm(&target_player.slack_user_id, &second_person).await?;

    super::broadcast_room_action(
        &state,
        &room_id,
        &third_person,
        Some(&user_id),
        Some(&first_person),
    ).await?;

    Ok(())
}

/// Find a player in the same room by name (case-insensitive)
async fn find_player_in_room(
    state: &Arc<AppState>,
    room_id: &str,
    target_name: &str,
) -> Result<Option<crate::models::Player>> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let players = player_repo.get_players_in_room(room_id).await?;

    let target_lower = target_name.to_lowercase();
    for player in players {
        if player.name.to_lowercase() == target_lower {
            return Ok(Some(player));
        }
    }

    Ok(None)
}
