use super::types::{Social, SocialMessages};
use std::collections::HashMap;
use anyhow::{Result, bail};

pub fn parse_socials(content: &str) -> Result<HashMap<String, Social>> {
    let mut socials = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    // Skip until we find #SOCIALS
    while i < lines.len() && !lines[i].starts_with("#SOCIALS") {
        i += 1;
    }
    i += 1; // Skip the #SOCIALS line

    while i < lines.len() {
        let line = lines[i].trim();

        // Check for end marker
        if line.starts_with("#0") || line.starts_with("#$") {
            break;
        }

        // Skip empty lines and lines starting with #
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        // This should be a social command name (possibly with flags)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            i += 1;
            continue;
        }

        let command_name = parts[0].to_lowercase();
        i += 1;

        // Now read up to 8 message lines (some socials end early with # terminator)
        let mut messages: Vec<String> = Vec::new();
        for _ in 0..8 {
            if i >= lines.len() {
                // End of file - fill remaining with empty strings
                break;
            }

            let msg_line = lines[i].trim();

            // Check for early terminator
            if msg_line == "#" {
                i += 1; // Skip the # terminator
                break;
            }

            // Handle empty messages (marked with $ or empty line)
            let message = if msg_line == "$" || msg_line.is_empty() {
                String::new()
            } else {
                lines[i].to_string()
            };
            messages.push(message);
            i += 1;
        }

        // Ensure we have exactly 8 messages
        while messages.len() < 8 {
            messages.push(String::new());
        }

        let social = Social {
            name: command_name.clone(),
            messages: SocialMessages {
                char_no_arg: messages[0].clone(),
                others_no_arg: messages[1].clone(),
                char_found: messages[2].clone(),
                others_found: messages[3].clone(),
                vict_found: messages[4].clone(),
                char_not_found: messages[5].clone(),
                char_auto: messages[6].clone(),
                others_auto: messages[7].clone(),
            },
        };

        socials.insert(command_name, social);
    }

    Ok(socials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_social() {
        let content = r#"#SOCIALS

kiss 0 0
Isn't there someone you want to kiss?
$
You kiss $M.
$n kisses $N.
$n kisses you.
Never around when required.
All the lonely people :(
#

#0
"#;

        let socials = parse_socials(content).unwrap();
        assert_eq!(socials.len(), 1);

        let kiss = socials.get("kiss").unwrap();
        assert_eq!(kiss.name, "kiss");
        assert_eq!(kiss.messages.char_no_arg, "Isn't there someone you want to kiss?");
        assert_eq!(kiss.messages.char_found, "You kiss $M.");
        assert_eq!(kiss.messages.others_found, "$n kisses $N.");
    }

    #[test]
    fn test_parse_multiple_socials() {
        let content = r#"#SOCIALS

smile 1 0
You smile happily.
$n smiles happily.
You smile at $M.
$n beams a smile at $N.
$n smiles at you.
There's no one by that name around.
You smile at yourself.
$n smiles at $mself.

laugh 0 0
You fall down laughing.
$n falls down laughing.
You laugh at $N mercilessly.
$n laughs at $N mercilessly.
$n laughs at you mercilessly.  Hmmmmph.
You can't find the butt of your joke.
You laugh at yourself.  I would, too.
$n laughs at $mself.  Let's all join in!!!

#0
"#;

        let socials = parse_socials(content).unwrap();
        assert_eq!(socials.len(), 2);
        assert!(socials.contains_key("smile"));
        assert!(socials.contains_key("laugh"));
    }
}
