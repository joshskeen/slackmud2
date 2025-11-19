use crate::models::Player;

#[derive(Debug, Clone)]
pub struct Social {
    pub name: String,
    pub messages: SocialMessages,
}

#[derive(Debug, Clone)]
pub struct SocialMessages {
    /// Message to actor when no target (line 1)
    pub char_no_arg: String,
    /// Message to room when no target (line 2)
    pub others_no_arg: String,
    /// Message to actor when target found (line 3)
    pub char_found: String,
    /// Message to room when target found (line 4)
    pub others_found: String,
    /// Message to victim when targeted (line 5)
    pub vict_found: String,
    /// Message to actor when target not found (line 6)
    pub char_not_found: String,
    /// Message to actor when targeting self (line 7)
    pub char_auto: String,
    /// Message to room when actor targets self (line 8)
    pub others_auto: String,
}

impl SocialMessages {
    /// Replace variables in a message with actual values
    pub fn substitute(
        &self,
        message: &str,
        actor: &Player,
        target: Option<&Player>,
    ) -> String {
        let mut result = message.to_string();

        // Actor substitutions
        result = result.replace("$n", &actor.name);
        result = result.replace("$m", &get_object_pronoun(actor));
        result = result.replace("$s", &get_possessive(actor));
        result = result.replace("$e", &get_subject_pronoun(actor));
        result = result.replace("$mself", &get_reflexive(actor));

        // Target substitutions
        if let Some(target) = target {
            result = result.replace("$N", &target.name);
            result = result.replace("$M", &target.name);
            result = result.replace("$S", &get_possessive(target));
            result = result.replace("$E", &get_subject_pronoun(target));
        }

        result
    }

    /// Get the message to send to the actor (first person perspective)
    pub fn get_actor_message(
        &self,
        actor: &Player,
        target: Option<&Player>,
    ) -> String {
        let message = if let Some(target) = target {
            if target.slack_user_id == actor.slack_user_id {
                // Targeting self
                &self.char_auto
            } else {
                // Targeting someone else
                &self.char_found
            }
        } else {
            // No target
            &self.char_no_arg
        };

        self.substitute(message, actor, target)
    }

    /// Get the message to send to the target (second person perspective)
    pub fn get_target_message(
        &self,
        actor: &Player,
        target: &Player,
    ) -> String {
        self.substitute(&self.vict_found, actor, Some(target))
    }

    /// Get the message to broadcast to the room (third person perspective)
    pub fn get_room_message(
        &self,
        actor: &Player,
        target: Option<&Player>,
    ) -> String {
        let message = if let Some(target) = target {
            if target.slack_user_id == actor.slack_user_id {
                // Targeting self
                &self.others_auto
            } else {
                // Targeting someone else
                &self.others_found
            }
        } else {
            // No target
            &self.others_no_arg
        };

        self.substitute(message, actor, target)
    }
}

/// Get object pronoun (him/her/them)
fn get_object_pronoun(player: &Player) -> String {
    match player.gender.as_deref() {
        Some("male") => "him".to_string(),
        Some("female") => "her".to_string(),
        _ => "them".to_string(),
    }
}

/// Get possessive (his/her/their)
fn get_possessive(player: &Player) -> String {
    match player.gender.as_deref() {
        Some("male") => "his".to_string(),
        Some("female") => "her".to_string(),
        _ => "their".to_string(),
    }
}

/// Get subject pronoun (he/she/they)
fn get_subject_pronoun(player: &Player) -> String {
    match player.gender.as_deref() {
        Some("male") => "he".to_string(),
        Some("female") => "she".to_string(),
        _ => "they".to_string(),
    }
}

/// Get reflexive (himself/herself/themself)
fn get_reflexive(player: &Player) -> String {
    match player.gender.as_deref() {
        Some("male") => "himself".to_string(),
        Some("female") => "herself".to_string(),
        _ => "themself".to_string(),
    }
}
