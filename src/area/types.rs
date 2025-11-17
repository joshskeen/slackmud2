use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct AreaFile {
    pub header: AreaHeader,
    pub rooms: Vec<AreaRoom>,
}

#[derive(Debug, Clone, Default)]
pub struct AreaHeader {
    pub filename: String,
    pub name: String,
    pub credits: String,
    pub min_vnum: i32,
    pub max_vnum: i32,
}

#[derive(Debug, Clone)]
pub struct AreaRoom {
    pub vnum: i32,
    pub name: String,
    pub description: String,
    pub area_vnum: i32,
    pub room_flags: RoomFlags,
    pub sector_type: SectorType,
    pub exits: Vec<AreaExit>,
    pub extra_descs: Vec<ExtraDescription>,
}

#[derive(Debug, Clone)]
pub struct AreaExit {
    pub direction: Direction,
    pub description: String,
    pub keyword: Option<String>,
    pub door_flags: i32,
    pub key_vnum: i32,
    pub to_room: i32,
}

#[derive(Debug, Clone)]
pub struct ExtraDescription {
    pub keywords: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
    Up = 4,
    Down = 5,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::North => "north",
            Direction::East => "east",
            Direction::South => "south",
            Direction::West => "west",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }

    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Direction::North),
            1 => Some(Direction::East),
            2 => Some(Direction::South),
            3 => Some(Direction::West),
            4 => Some(Direction::Up),
            5 => Some(Direction::Down),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorType {
    Inside = 0,
    City = 1,
    Field = 2,
    Forest = 3,
    Hills = 4,
    Mountain = 5,
    WaterSwim = 6,
    WaterNoSwim = 7,
    Underwater = 8,
    Air = 9,
    Desert = 10,
}

impl SectorType {
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(SectorType::Inside),
            1 => Some(SectorType::City),
            2 => Some(SectorType::Field),
            3 => Some(SectorType::Forest),
            4 => Some(SectorType::Hills),
            5 => Some(SectorType::Mountain),
            6 => Some(SectorType::WaterSwim),
            7 => Some(SectorType::WaterNoSwim),
            8 => Some(SectorType::Underwater),
            9 => Some(SectorType::Air),
            10 => Some(SectorType::Desert),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SectorType::Inside => "inside",
            SectorType::City => "city",
            SectorType::Field => "field",
            SectorType::Forest => "forest",
            SectorType::Hills => "hills",
            SectorType::Mountain => "mountain",
            SectorType::WaterSwim => "water (shallow)",
            SectorType::WaterNoSwim => "water (deep)",
            SectorType::Underwater => "underwater",
            SectorType::Air => "air",
            SectorType::Desert => "desert",
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RoomFlags: u32 {
        const DARK        = 1 << 0;  // D
        const NO_MOB      = 1 << 2;  // K
        const INDOORS     = 1 << 3;  // I
        const PRIVATE     = 1 << 9;  // J
        const SAFE        = 1 << 10; // S
        const SOLITARY    = 1 << 11; // O
        const PET_SHOP    = 1 << 12; // P
        const NO_RECALL   = 1 << 13; // C
        const IMP_ONLY    = 1 << 14;
        const GODS_ONLY   = 1 << 15; // G
        const HEROES_ONLY = 1 << 16; // H
        const NEWBIES_ONLY= 1 << 17; // N
        const LAW         = 1 << 18; // L
        const NOWHERE     = 1 << 19;
        const BANK        = 1 << 20; // B
        const ARENA       = 1 << 21; // A
    }
}

impl RoomFlags {
    pub fn from_str(flags_str: &str) -> Self {
        let mut flags = RoomFlags::empty();

        for ch in flags_str.chars() {
            match ch {
                'D' => flags |= RoomFlags::DARK,
                'C' => flags |= RoomFlags::NO_RECALL,
                'S' => flags |= RoomFlags::SAFE,
                'B' => flags |= RoomFlags::BANK,
                'J' => flags |= RoomFlags::PRIVATE,
                'K' => flags |= RoomFlags::NO_MOB,
                'L' => flags |= RoomFlags::LAW,
                'A' => flags |= RoomFlags::ARENA,
                'G' => flags |= RoomFlags::GODS_ONLY,
                'H' => flags |= RoomFlags::HEROES_ONLY,
                'N' => flags |= RoomFlags::NEWBIES_ONLY,
                'O' => flags |= RoomFlags::SOLITARY,
                'P' => flags |= RoomFlags::PET_SHOP,
                'I' => flags |= RoomFlags::INDOORS,
                _ => {} // Ignore unknown flags
            }
        }

        flags
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("Invalid vnum format")]
    InvalidVnum,

    #[error("Invalid direction code")]
    InvalidDirection,

    #[error("Invalid sector type")]
    InvalidSectorType,

    #[error("Invalid room attributes")]
    InvalidRoomAttributes,

    #[error("Invalid exit data")]
    InvalidExitData,

    #[error("Parse integer error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Missing required field: {0}")]
    MissingField(String),
}
