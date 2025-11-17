use super::types::{PostMessageRequest, PostEphemeralRequest, Block};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct SlackClient {
    client: Client,
    bot_token: String,
}

impl SlackClient {
    pub fn new(bot_token: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
        }
    }

    /// Send a direct message to a user
    pub async fn send_dm(&self, user_id: &str, text: &str) -> Result<()> {
        // First, open a DM channel with the user
        let dm_channel = self.open_dm_channel(user_id).await?;

        // Then send the message to that channel
        self.post_message(&dm_channel, text, None).await
    }

    /// Send a DM with blocks for richer formatting
    pub async fn send_dm_with_blocks(&self, user_id: &str, text: &str, blocks: Vec<Block>) -> Result<()> {
        let dm_channel = self.open_dm_channel(user_id).await?;
        self.post_message(&dm_channel, text, Some(blocks)).await
    }

    /// Open a DM channel with a user and return the channel ID
    async fn open_dm_channel(&self, user_id: &str) -> Result<String> {
        let response = self
            .client
            .post("https://slack.com/api/conversations.open")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&json!({
                "users": user_id
            }))
            .send()
            .await
            .context("Failed to open DM channel")?;

        let json: serde_json::Value = response.json().await?;

        if !json["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("Slack API error: {}", json["error"].as_str().unwrap_or("unknown"));
        }

        let channel_id = json["channel"]["id"]
            .as_str()
            .context("No channel ID in response")?
            .to_string();

        Ok(channel_id)
    }

    /// Post a message to a channel
    pub async fn post_message(&self, channel: &str, text: &str, blocks: Option<Vec<Block>>) -> Result<()> {
        let payload = PostMessageRequest {
            channel: channel.to_string(),
            text: text.to_string(),
            blocks,
        };

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&payload)
            .send()
            .await
            .context("Failed to post message")?;

        let json: serde_json::Value = response.json().await?;

        if !json["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("Slack API error: {}", json["error"].as_str().unwrap_or("unknown"));
        }

        Ok(())
    }

    /// Post an ephemeral message (only visible to specific user)
    pub async fn post_ephemeral(&self, channel: &str, user: &str, text: &str, blocks: Option<Vec<Block>>) -> Result<()> {
        let payload = PostEphemeralRequest {
            channel: channel.to_string(),
            user: user.to_string(),
            text: text.to_string(),
            blocks,
        };

        let response = self
            .client
            .post("https://slack.com/api/chat.postEphemeral")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&payload)
            .send()
            .await
            .context("Failed to post ephemeral message")?;

        let json: serde_json::Value = response.json().await?;

        if !json["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("Slack API error: {}", json["error"].as_str().unwrap_or("unknown"));
        }

        Ok(())
    }

    /// Get user info from Slack
    pub async fn get_user_real_name(&self, user_id: &str) -> Result<String> {
        let response = self
            .client
            .get("https://slack.com/api/users.info")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .query(&[("user", user_id)])
            .send()
            .await
            .context("Failed to get user info")?;

        let json: serde_json::Value = response.json().await?;

        if !json["ok"].as_bool().unwrap_or(false) {
            anyhow::bail!("Slack API error: {}", json["error"].as_str().unwrap_or("unknown"));
        }

        let real_name = json["user"]["real_name"]
            .as_str()
            .or_else(|| json["user"]["name"].as_str())
            .context("No name found for user")?
            .to_string();

        Ok(real_name)
    }
}
