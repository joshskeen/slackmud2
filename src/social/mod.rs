mod parser;
mod types;

pub use parser::parse_socials;
pub use types::Social;

use std::collections::HashMap;
use once_cell::sync::Lazy;

// Lazy-loaded global socials map
pub static SOCIALS: Lazy<HashMap<String, Social>> = Lazy::new(|| {
    let content = include_str!("../../resources/social.are");
    tracing::info!("Loading socials from social.are ({} bytes)", content.len());
    match parse_socials(content) {
        Ok(socials) => {
            tracing::info!("Successfully parsed {} social commands", socials.len());
            socials
        }
        Err(e) => {
            tracing::error!("Failed to parse socials: {}", e);
            HashMap::new()
        }
    }
});

/// Get a social command by name
pub fn get_social(name: &str) -> Option<&Social> {
    SOCIALS.get(&name.to_lowercase())
}

/// Get all social command names
pub fn get_all_social_names() -> Vec<String> {
    tracing::info!("SOCIALS map contains {} entries", SOCIALS.len());
    let mut names: Vec<String> = SOCIALS.keys().cloned().collect();
    names.sort();
    names
}
