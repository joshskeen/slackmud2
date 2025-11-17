use crate::AppState;
use crate::slack::SlashCommand;
use crate::db::player::PlayerRepository;
use crate::db::object::{ObjectRepository, ObjectInstanceRepository};
use crate::models::EquipmentSlot;
use std::sync::Arc;
use anyhow::Result;

/// Handle wear command - wear armor/jewelry/clothing
pub async fn handle_wear(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud wear <item>`\\nExample: `/mud wear helm`"
        ).await?;
        return Ok(());
    }

    // Find item in inventory
    let inventory_instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    let mut found_instance = None;
    let mut found_object = None;

    for instance in inventory_instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    let (instance, object) = match (found_instance, found_object) {
        (Some(i), Some(o)) => (i, o),
        _ => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You aren't carrying '{}'.", item_name)
            ).await?;
            return Ok(());
        }
    };

    // Get valid slots for this item
    let valid_slots = EquipmentSlot::from_wear_flags(&object.wear_flags);

    if valid_slots.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You can't wear {}.", object.short_description)
        ).await?;
        return Ok(());
    }

    // Find first available slot
    let mut chosen_slot = None;
    for slot in &valid_slots {
        let existing = object_instance_repo.get_item_in_slot(&player.slack_user_id, slot.to_db_string()).await?;
        if existing.is_none() {
            chosen_slot = Some(slot);
            break;
        }
    }

    let slot = match chosen_slot {
        Some(s) => s,
        None => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You're already wearing something in all available slots for {}.", object.short_description)
            ).await?;
            return Ok(());
        }
    };

    // Equip the item
    object_instance_repo.equip_item(
        instance.id,
        &player.slack_user_id,
        slot.to_db_string(),
    ).await?;

    // Send success message
    let wear_message = format!("You wear {} {}.", object.short_description, get_slot_location_text(slot));
    state.slack_client.send_dm(&command.user_id, &wear_message).await?;

    // Broadcast action to room
    if let Some(room_id) = player.current_channel_id {
        let third_person = format!("_{} wears {}._", player.name, object.short_description);
        let first_person = format!("_You wear {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&command.user_id),
            Some(&first_person),
        ).await?;
    }

    Ok(())
}

/// Handle wield command - wield a weapon
pub async fn handle_wield(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud wield <weapon>`\\nExample: `/mud wield sword`"
        ).await?;
        return Ok(());
    }

    // Find item in inventory
    let inventory_instances = object_instance_repo.get_in_player_inventory(&player.slack_user_id).await?;

    let mut found_instance = None;
    let mut found_object = None;

    for instance in inventory_instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    let (instance, object) = match (found_instance, found_object) {
        (Some(i), Some(o)) => (i, o),
        _ => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You aren't carrying '{}'.", item_name)
            ).await?;
            return Ok(());
        }
    };

    // Check if item can be wielded
    if !object.wear_flags.to_lowercase().contains("wield") {
        state.slack_client.send_dm(
            &command.user_id,
            &format!("You can't wield {}.", object.short_description)
        ).await?;
        return Ok(());
    }

    // Check if wield slot is occupied
    if let Some(_existing) = object_instance_repo.get_item_in_slot(&player.slack_user_id, "wield").await? {
        state.slack_client.send_dm(
            &command.user_id,
            "You're already wielding something. Remove it first with `/mud remove <weapon>`."
        ).await?;
        return Ok(());
    }

    // Equip the weapon
    object_instance_repo.equip_item(
        instance.id,
        &player.slack_user_id,
        "wield",
    ).await?;

    // Send success message
    state.slack_client.send_dm(
        &command.user_id,
        &format!("You wield {}.", object.short_description)
    ).await?;

    // Broadcast action to room
    if let Some(room_id) = player.current_channel_id {
        let third_person = format!("_{} wields {}._", player.name, object.short_description);
        let first_person = format!("_You wield {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&command.user_id),
            Some(&first_person),
        ).await?;
    }

    Ok(())
}

/// Handle remove command - remove equipped item
pub async fn handle_remove(state: Arc<AppState>, command: SlashCommand, args: &str) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    let item_name = args.trim();
    if item_name.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "Usage: `/mud remove <item>`\\nExample: `/mud remove helm`"
        ).await?;
        return Ok(());
    }

    // Find item in equipped items
    let equipped_instances = object_instance_repo.get_equipped(&player.slack_user_id).await?;

    let mut found_instance = None;
    let mut found_object = None;

    for instance in equipped_instances {
        if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
            if object.matches_keyword(item_name) {
                found_instance = Some(instance);
                found_object = Some(object);
                break;
            }
        }
    }

    let (instance, object) = match (found_instance, found_object) {
        (Some(i), Some(o)) => (i, o),
        _ => {
            state.slack_client.send_dm(
                &command.user_id,
                &format!("You aren't wearing '{}'.", item_name)
            ).await?;
            return Ok(());
        }
    };

    // Unequip the item
    object_instance_repo.unequip_item(instance.id, &player.slack_user_id).await?;

    // Send success message
    state.slack_client.send_dm(
        &command.user_id,
        &format!("You remove {}.", object.short_description)
    ).await?;

    // Broadcast action to room
    if let Some(room_id) = player.current_channel_id {
        let third_person = format!("_{} removes {}._", player.name, object.short_description);
        let first_person = format!("_You remove {}._", object.short_description);
        super::broadcast_room_action(
            &state,
            &room_id,
            &third_person,
            Some(&command.user_id),
            Some(&first_person),
        ).await?;
    }

    Ok(())
}

/// Handle equipment command - show what you're wearing
pub async fn handle_equipment(state: Arc<AppState>, command: SlashCommand) -> Result<()> {
    let player_repo = PlayerRepository::new(state.db_pool.clone());
    let object_repo = ObjectRepository::new(state.db_pool.clone());
    let object_instance_repo = ObjectInstanceRepository::new(state.db_pool.clone());

    // Get player
    let real_name = state.slack_client.get_user_real_name(&command.user_id).await?;
    let player = player_repo.get_or_create(command.user_id.clone(), real_name).await?;

    // Get all equipped items
    let equipped_instances = object_instance_repo.get_equipped(&player.slack_user_id).await?;

    if equipped_instances.is_empty() {
        state.slack_client.send_dm(
            &command.user_id,
            "*Equipment:*\\nYou aren't wearing anything."
        ).await?;
        return Ok(());
    }

    let mut equipment_text = String::from("*You are using:*\\n");

    // Display in slot order
    for slot in EquipmentSlot::all_slots_in_order() {
        let slot_str = slot.to_db_string();

        // Find item in this slot
        if let Some(instance) = equipped_instances.iter().find(|i| {
            i.equipped_slot.as_ref().map(|s| s.as_str()) == Some(slot_str)
        }) {
            if let Some(object) = object_repo.get_by_vnum(instance.object_vnum).await? {
                equipment_text.push_str(&format!(
                    "{:<20} {}\\n",
                    slot.display_label(),
                    object.short_description
                ));
            }
        }
    }

    state.slack_client.send_dm(&command.user_id, &equipment_text).await?;

    Ok(())
}

/// Get location text for a slot (e.g., "on your head", "in your hand")
fn get_slot_location_text(slot: &EquipmentSlot) -> &str {
    match slot {
        EquipmentSlot::Light => "as a light",
        EquipmentSlot::FingerL | EquipmentSlot::FingerR => "on your finger",
        EquipmentSlot::Neck1 | EquipmentSlot::Neck2 => "around your neck",
        EquipmentSlot::Body => "on your body",
        EquipmentSlot::Head => "on your head",
        EquipmentSlot::Legs => "on your legs",
        EquipmentSlot::Feet => "on your feet",
        EquipmentSlot::Hands => "on your hands",
        EquipmentSlot::Arms => "on your arms",
        EquipmentSlot::Shield => "as a shield",
        EquipmentSlot::About => "about your body",
        EquipmentSlot::Waist => "about your waist",
        EquipmentSlot::WristL | EquipmentSlot::WristR => "around your wrist",
        EquipmentSlot::Wield => "in your hand",
        EquipmentSlot::Hold => "in your hand",
        EquipmentSlot::Float => "floating nearby",
    }
}
