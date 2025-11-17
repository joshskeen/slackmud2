use super::types::*;
use std::iter::Peekable;
use std::str::Lines;

pub fn parse_area_file(content: &str) -> Result<AreaFile, ParseError> {
    let mut lines = content.lines().peekable();
    let mut area = AreaFile::default();

    while let Some(line) = lines.peek() {
        let trimmed = line.trim();

        match trimmed {
            "#AREA" => {
                lines.next(); // Consume #AREA
                area.header = parse_area_header(&mut lines)?;
            }
            "#ROOMS" => {
                lines.next(); // Consume #ROOMS
                area.rooms = parse_rooms(&mut lines)?;
            }
            "#$" => break, // End of file
            _ => {
                lines.next(); // Skip unknown sections
            }
        }
    }

    Ok(area)
}

fn parse_area_header<'a, I>(lines: &mut Peekable<I>) -> Result<AreaHeader, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    let filename = read_until_tilde(lines)?;
    let name = read_until_tilde(lines)?;
    let credits = read_until_tilde(lines)?;

    // Parse vnum range line
    let vnum_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let parts: Vec<&str> = vnum_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(ParseError::MissingField("vnum range".to_string()));
    }

    let min_vnum = parts[0].parse::<i32>()?;
    let max_vnum = parts[1].parse::<i32>()?;

    Ok(AreaHeader {
        filename,
        name,
        credits,
        min_vnum,
        max_vnum,
    })
}

fn parse_rooms<'a, I>(lines: &mut Peekable<I>) -> Result<Vec<AreaRoom>, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    let mut rooms = Vec::new();

    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();

        // Check for section end
        if trimmed.starts_with("#RESETS")
            || trimmed.starts_with("#MOBILES")
            || trimmed.starts_with("#OBJECTS")
            || trimmed.starts_with("#SHOPS")
            || trimmed.starts_with("#SPECIALS")
            || trimmed.starts_with("#$")
            || trimmed == "#0"
        {
            break;
        }

        if trimmed.starts_with('#') && trimmed.len() > 1 {
            // This is a vnum marker - parse the room
            rooms.push(parse_single_room(lines)?);
        } else {
            lines.next(); // Skip non-vnum lines
        }
    }

    Ok(rooms)
}

fn parse_single_room<'a, I>(lines: &mut Peekable<I>) -> Result<AreaRoom, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    // Parse #vnum
    let vnum_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let vnum = parse_vnum(vnum_line)?;

    // Parse name (tilde-terminated)
    let name = read_until_tilde(lines)?;

    // Parse description (tilde-terminated, may be multi-line)
    let description = read_until_tilde(lines)?;

    // Parse room attributes line: "area_vnum flags sector"
    let attr_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let (area_vnum, room_flags, sector_type) = parse_room_attributes(attr_line)?;

    // Parse exits and extra descriptions until 'S'
    let mut exits = Vec::new();
    let mut extra_descs = Vec::new();

    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();

        if trimmed == "S" {
            lines.next(); // Consume the 'S'
            break;
        } else if trimmed.starts_with('D') {
            exits.push(parse_exit(lines)?);
        } else if trimmed == "E" {
            extra_descs.push(parse_extra_desc(lines)?);
        } else {
            lines.next(); // Skip unknown lines
        }
    }

    Ok(AreaRoom {
        vnum,
        name,
        description,
        area_vnum,
        room_flags,
        sector_type,
        exits,
        extra_descs,
    })
}

fn parse_exit<'a, I>(lines: &mut Peekable<I>) -> Result<AreaExit, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    // Parse "D<direction>" line
    let dir_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let direction = parse_direction(dir_line)?;

    // Parse exit description (tilde-terminated)
    let description = read_until_tilde(lines)?;

    // Parse keyword (tilde-terminated, may be empty)
    let keyword_raw = read_until_tilde(lines)?;
    let keyword = if keyword_raw.is_empty() {
        None
    } else {
        Some(keyword_raw)
    };

    // Parse "door_flags key_vnum to_room" line
    let exit_data = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let parts: Vec<&str> = exit_data.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(ParseError::InvalidExitData);
    }

    let door_flags = parts[0].parse::<i32>()?;
    let key_vnum = parts[1].parse::<i32>()?;
    let to_room = parts[2].parse::<i32>()?;

    Ok(AreaExit {
        direction,
        description,
        keyword,
        door_flags,
        key_vnum,
        to_room,
    })
}

fn parse_extra_desc<'a, I>(lines: &mut Peekable<I>) -> Result<ExtraDescription, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    // Consume "E" line
    lines.next();

    // Parse keywords (tilde-terminated)
    let keywords_raw = read_until_tilde(lines)?;
    let keywords: Vec<String> = keywords_raw
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    // Parse description (tilde-terminated)
    let description = read_until_tilde(lines)?;

    Ok(ExtraDescription {
        keywords,
        description,
    })
}

/// Helper function to read multi-line text until tilde
fn read_until_tilde<'a, I>(lines: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    let mut result = String::new();

    loop {
        let line = lines.next().ok_or(ParseError::UnexpectedEof)?;

        if line.trim_end().ends_with('~') {
            // Remove the tilde and add final line
            let without_tilde = line.trim_end().trim_end_matches('~');
            if !without_tilde.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(without_tilde);
            }
            break;
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }

    Ok(result.trim().to_string())
}

fn parse_room_attributes(line: &str) -> Result<(i32, RoomFlags, SectorType), ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(ParseError::InvalidRoomAttributes);
    }

    let area_vnum = parts[0].parse::<i32>()?;
    let flags = RoomFlags::from_str(parts[1]);
    let sector = SectorType::from_code(parts[2].parse::<i32>()?)
        .ok_or(ParseError::InvalidSectorType)?;

    Ok((area_vnum, flags, sector))
}

fn parse_direction(line: &str) -> Result<Direction, ParseError> {
    let trimmed = line.trim();

    if !trimmed.starts_with('D') || trimmed.len() < 2 {
        return Err(ParseError::InvalidDirection);
    }

    let dir_char = &trimmed[1..2];
    let dir_code = dir_char.parse::<i32>().map_err(|_| ParseError::InvalidDirection)?;

    Direction::from_code(dir_code).ok_or(ParseError::InvalidDirection)
}

fn parse_vnum(line: &str) -> Result<i32, ParseError> {
    let trimmed = line.trim();

    if !trimmed.starts_with('#') {
        return Err(ParseError::InvalidVnum);
    }

    trimmed[1..]
        .parse::<i32>()
        .map_err(|_| ParseError::InvalidVnum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vnum() {
        assert_eq!(parse_vnum("#3001").unwrap(), 3001);
        assert_eq!(parse_vnum("  #3050  ").unwrap(), 3050);
        assert!(parse_vnum("3001").is_err());
        assert!(parse_vnum("#abc").is_err());
    }

    #[test]
    fn test_parse_direction() {
        assert_eq!(parse_direction("D0").unwrap(), Direction::North);
        assert_eq!(parse_direction("D1").unwrap(), Direction::East);
        assert_eq!(parse_direction("D5").unwrap(), Direction::Down);
        assert!(parse_direction("D6").is_err());
        assert!(parse_direction("X0").is_err());
    }

    #[test]
    fn test_read_until_tilde() {
        let content = "First line\nSecond line~\nExtra";
        let mut lines = content.lines().peekable();
        let result = read_until_tilde(&mut lines).unwrap();
        assert_eq!(result, "First line\nSecond line");
    }

    #[test]
    fn test_parse_room_attributes() {
        let (area, flags, sector) = parse_room_attributes("0 CDS 0").unwrap();
        assert_eq!(area, 0);
        assert!(flags.contains(RoomFlags::NO_RECALL));
        assert!(flags.contains(RoomFlags::DARK));
        assert!(flags.contains(RoomFlags::SAFE));
        assert_eq!(sector, SectorType::Inside);
    }
}
