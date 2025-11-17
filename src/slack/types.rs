use serde::{Deserialize, Serialize};

/// Represents a Slack slash command payload
#[derive(Debug, Clone, Deserialize)]
pub struct SlashCommand {
    pub token: String,
    pub team_id: String,
    pub team_domain: String,
    pub channel_id: String,
    pub channel_name: String,
    pub user_id: String,
    pub user_name: String,
    pub command: String,
    pub text: String,
    pub api_app_id: String,
    pub response_url: String,
    pub trigger_id: String,
}

impl SlashCommand {
    /// Extract the subcommand and arguments from the text field
    /// For example: "/mud look" -> ("look", "")
    ///              "/mud attack goblin" -> ("attack", "goblin")
    pub fn parse_subcommand(&self) -> (&str, &str) {
        let text = self.text.trim();
        if let Some(space_idx) = text.find(' ') {
            let (cmd, args) = text.split_at(space_idx);
            (cmd, args.trim())
        } else {
            (text, "")
        }
    }
}

/// Message visibility determines if a message is public or private
#[derive(Debug, Clone, Copy)]
pub enum MessageVisibility {
    /// Message visible only to the user who triggered the command
    Ephemeral,
    /// Message visible to everyone in the channel
    InChannel,
}

/// Payload for posting a message to Slack
#[derive(Debug, Serialize)]
pub struct PostMessageRequest {
    pub channel: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<Block>>,
}

/// Payload for posting an ephemeral message (only visible to one user)
#[derive(Debug, Serialize)]
pub struct PostEphemeralRequest {
    pub channel: String,
    pub user: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<Block>>,
}

/// Slack Block Kit block (simplified version)
#[derive(Debug, Serialize)]
pub struct Block {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextObject>,
}

#[derive(Debug, Serialize)]
pub struct TextObject {
    #[serde(rename = "type")]
    pub text_type: String,
    pub text: String,
}

impl Block {
    pub fn section(text: &str) -> Self {
        Self {
            block_type: "section".to_string(),
            text: Some(TextObject {
                text_type: "mrkdwn".to_string(),
                text: text.to_string(),
            }),
        }
    }
}

/// Slack Events API callback event
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum EventWrapper {
    #[serde(rename = "url_verification")]
    UrlVerification { challenge: String },
    #[serde(rename = "event_callback")]
    EventCallback {
        event: Event,
        event_id: String,
    },
}

/// Inner event types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "message")]
    Message(MessageEvent),
}

/// Message event from Slack
#[derive(Debug, Clone, Deserialize)]
pub struct MessageEvent {
    pub user: Option<String>,
    pub text: String,
    pub channel: String,
    pub channel_type: String,
    #[serde(default)]
    pub bot_id: Option<String>,
}

impl MessageEvent {
    /// Check if this is a DM (direct message)
    pub fn is_dm(&self) -> bool {
        self.channel_type == "im"
    }

    /// Check if message is from a bot (to avoid loops)
    pub fn is_from_bot(&self) -> bool {
        self.bot_id.is_some()
    }

    /// Parse command from message text
    /// "look" -> ("look", "")
    /// "attack goblin" -> ("attack", "goblin")
    pub fn parse_command(&self) -> (&str, &str) {
        let text = self.text.trim();
        if let Some(space_idx) = text.find(' ') {
            let (cmd, args) = text.split_at(space_idx);
            (cmd, args.trim())
        } else {
            (text, "")
        }
    }
}
