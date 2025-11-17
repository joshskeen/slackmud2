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
            "#OBJECTS" => {
                lines.next(); // Consume #OBJECTS
                area.objects = parse_objects(&mut lines)?;
            }
            "#RESETS" => {
                lines.next(); // Consume #RESETS
                area.resets = parse_resets(&mut lines)?;
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

fn parse_objects<'a, I>(lines: &mut Peekable<I>) -> Result<Vec<AreaObject>, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    let mut objects = Vec::new();

    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();

        // Check for section end
        if trimmed.starts_with("#RESETS")
            || trimmed.starts_with("#MOBILES")
            || trimmed.starts_with("#ROOMS")
            || trimmed.starts_with("#SHOPS")
            || trimmed.starts_with("#SPECIALS")
            || trimmed.starts_with("#$")
            || trimmed == "#0"
        {
            break;
        }

        if trimmed.starts_with('#') && trimmed.len() > 1 {
            // This is a vnum marker - parse the object
            objects.push(parse_single_object(lines)?);
        } else {
            lines.next(); // Skip non-vnum lines
        }
    }

    Ok(objects)
}

fn parse_single_object<'a, I>(lines: &mut Peekable<I>) -> Result<AreaObject, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    // Parse #vnum
    let vnum_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let vnum = parse_vnum(vnum_line)?;

    // Parse keywords (tilde-terminated)
    let keywords = read_until_tilde(lines)?;

    // Parse short description (tilde-terminated)
    let short_description = read_until_tilde(lines)?;

    // Parse long description (tilde-terminated)
    let long_description = read_until_tilde(lines)?;

    // Parse material (tilde-terminated)
    let material = read_until_tilde(lines)?;

    // Parse item type line: "type extra_flags wear_flags"
    let type_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let (item_type, extra_flags, wear_flags) = parse_object_type_line(type_line)?;

    // Parse values line (format varies by item type)
    let values_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let (value0, value1, value2, value3, value4) = parse_object_values_line(values_line)?;

    // Parse weight/cost/level/condition line
    let weight_line = lines.next().ok_or(ParseError::UnexpectedEof)?;
    let (weight, cost, level, condition) = parse_object_weight_line(weight_line)?;

    // Parse optional extra descriptions
    let mut extra_descriptions = Vec::new();
    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();

        if trimmed == "E" {
            extra_descriptions.push(parse_extra_desc(lines)?);
        } else if trimmed.starts_with('#') {
            // Next object
            break;
        } else {
            lines.next(); // Skip unknown lines
        }
    }

    Ok(AreaObject {
        vnum,
        keywords,
        short_description,
        long_description,
        material,
        item_type,
        extra_flags,
        wear_flags,
        value0,
        value1,
        value2,
        value3,
        value4,
        weight,
        cost,
        level,
        condition,
        extra_descriptions,
    })
}

fn parse_object_type_line(line: &str) -> Result<(String, String, String), ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return Err(ParseError::InvalidObjectType);
    }

    let item_type = parts[0].to_string();
    let extra_flags = parts.get(1).unwrap_or(&"0").to_string();
    let wear_flags = parts.get(2).unwrap_or(&"A").to_string();

    Ok((item_type, extra_flags, wear_flags))
}

fn parse_object_values_line(line: &str) -> Result<(i32, i32, String, i32, i32), ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Values vary by item type, but we'll parse them generically
    // value0 and value1 are usually integers
    // value2 might be a string (e.g., liquid type) or integer
    // value3 and value4 are integers

    let value0 = parts.get(0).unwrap_or(&"0").parse::<i32>().unwrap_or(0);
    let value1 = parts.get(1).unwrap_or(&"0").parse::<i32>().unwrap_or(0);

    // value2 might be a quoted string or a number
    let value2_raw = parts.get(2).unwrap_or(&"0");
    let value2 = if value2_raw.starts_with('\'') || value2_raw.starts_with('"') {
        // It's a string - remove quotes
        value2_raw.trim_matches(|c| c == '\'' || c == '"').to_string()
    } else {
        value2_raw.to_string()
    };

    let value3 = parts.get(3).unwrap_or(&"0").parse::<i32>().unwrap_or(0);
    let value4 = parts.get(4).unwrap_or(&"0").parse::<i32>().unwrap_or(0);

    Ok((value0, value1, value2, value3, value4))
}

fn parse_object_weight_line(line: &str) -> Result<(i32, i32, i32, String), ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 4 {
        return Err(ParseError::InvalidObjectWeightCost);
    }

    let weight = parts[0].parse::<i32>()?;
    let cost = parts[1].parse::<i32>()?;
    let level = parts[2].parse::<i32>()?;
    let condition = parts[3].to_string();

    Ok((weight, cost, level, condition))
}

fn parse_resets<'a, I>(lines: &mut Peekable<I>) -> Result<Vec<Reset>, ParseError>
where
    I: Iterator<Item = &'a str>,
{
    let mut resets = Vec::new();

    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();

        // Check for section end
        if trimmed.starts_with("#SHOPS")
            || trimmed.starts_with("#SPECIALS")
            || trimmed.starts_with("#$")
            || trimmed == "S"
        {
            break;
        }

        // Skip comments
        if trimmed.starts_with('*') || trimmed.is_empty() {
            lines.next();
            continue;
        }

        // Parse reset command
        if let Some(reset) = parse_single_reset(line)? {
            resets.push(reset);
        }
        lines.next();
    }

    Ok(resets)
}

fn parse_single_reset(line: &str) -> Result<Option<Reset>, ParseError> {
    let line = line.trim();

    // Remove trailing comment if present
    let line = if let Some(idx) = line.find('*') {
        &line[..idx].trim()
    } else {
        line
    };

    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(None);
    }

    let command = parts[0];

    match command {
        "M" => {
            // Mobile: M <if_flag> <mob_vnum> <limit> <room_vnum> <max_in_room>
            if parts.len() < 6 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::Mobile {
                if_flag: parts[1].parse()?,
                mob_vnum: parts[2].parse()?,
                limit: parts[3].parse()?,
                room_vnum: parts[4].parse()?,
                max_in_room: parts[5].parse()?,
            }))
        }
        "O" => {
            // Object in room: O <if_flag> <obj_vnum> <limit> <room_vnum>
            if parts.len() < 5 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::ObjectInRoom {
                if_flag: parts[1].parse()?,
                obj_vnum: parts[2].parse()?,
                limit: parts[3].parse()?,
                room_vnum: parts[4].parse()?,
            }))
        }
        "G" => {
            // Give object: G <if_flag> <obj_vnum> <limit>
            if parts.len() < 4 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::GiveObject {
                if_flag: parts[1].parse()?,
                obj_vnum: parts[2].parse()?,
                limit: parts[3].parse()?,
            }))
        }
        "E" => {
            // Equip object: E <if_flag> <obj_vnum> <limit> <wear_location>
            if parts.len() < 5 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::EquipObject {
                if_flag: parts[1].parse()?,
                obj_vnum: parts[2].parse()?,
                limit: parts[3].parse()?,
                wear_location: parts[4].parse()?,
            }))
        }
        "P" => {
            // Put in container: P <if_flag> <obj_vnum> <limit> <container_vnum>
            if parts.len() < 5 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::PutInContainer {
                if_flag: parts[1].parse()?,
                obj_vnum: parts[2].parse()?,
                limit: parts[3].parse()?,
                container_vnum: parts[4].parse()?,
            }))
        }
        "D" => {
            // Door: D <room_vnum> <direction> <state>
            if parts.len() < 4 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::Door {
                room_vnum: parts[1].parse()?,
                direction: parts[2].parse()?,
                state: parts[3].parse()?,
            }))
        }
        "R" => {
            // Randomize exits: R <room_vnum> <num_exits>
            if parts.len() < 3 {
                return Err(ParseError::InvalidResetCommand);
            }
            Ok(Some(Reset::RandomizeExits {
                room_vnum: parts[1].parse()?,
                num_exits: parts[2].parse()?,
            }))
        }
        _ => Ok(None), // Unknown command, skip it
    }
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
