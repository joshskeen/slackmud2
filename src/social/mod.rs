mod parser;
mod types;

pub use parser::parse_socials;
pub use types::Social;

use std::collections::HashMap;
use once_cell::sync::Lazy;

// Lazy-loaded global socials map
pub static SOCIALS: Lazy<HashMap<String, Social>> = Lazy::new(|| {
    let content = include_str!("../../resources/social.are");
    parse_socials(content).unwrap_or_default()
});

/// Get a social command by name
pub fn get_social(name: &str) -> Option<&Social> {
    SOCIALS.get(&name.to_lowercase())
}

/// Get all social command names
pub fn get_all_social_names() -> Vec<String> {
    let mut names: Vec<String> = SOCIALS.keys().cloned().collect();
    names.sort();
    names
}
