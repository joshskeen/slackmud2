use std::fmt;

/// Equipment slot where an item can be worn/wielded
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquipmentSlot {
    Light,      // Light source
    FingerL,    // Left finger (ring)
    FingerR,    // Right finger (ring)
    Neck1,      // Neck slot 1
    Neck2,      // Neck slot 2
    Body,       // Torso armor
    Head,       // Helmet
    Legs,       // Leg armor
    Feet,       // Boots
    Hands,      // Gloves
    Arms,       // Arm armor
    Shield,     // Shield (off-hand)
    About,      // Cloak/cape
    Waist,      // Belt
    WristL,     // Left wrist
    WristR,     // Right wrist
    Wield,      // Primary weapon
    Hold,       // Held item (off-hand)
    Float,      // Floating nearby
}

impl EquipmentSlot {
    /// Get the display label for this slot (as shown in "look" output)
    pub fn display_label(&self) -> &str {
        match self {
            EquipmentSlot::Light => "<used as light>",
            EquipmentSlot::FingerL => "<worn on finger>",
            EquipmentSlot::FingerR => "<worn on finger>",
            EquipmentSlot::Neck1 => "<worn around neck>",
            EquipmentSlot::Neck2 => "<worn around neck>",
            EquipmentSlot::Body => "<worn on body>",
            EquipmentSlot::Head => "<worn on head>",
            EquipmentSlot::Legs => "<worn on legs>",
            EquipmentSlot::Feet => "<worn on feet>",
            EquipmentSlot::Hands => "<worn on hands>",
            EquipmentSlot::Arms => "<worn on arms>",
            EquipmentSlot::Shield => "<worn as shield>",
            EquipmentSlot::About => "<worn about body>",
            EquipmentSlot::Waist => "<worn about waist>",
            EquipmentSlot::WristL => "<worn around wrist>",
            EquipmentSlot::WristR => "<worn around wrist>",
            EquipmentSlot::Wield => "<wielded>",
            EquipmentSlot::Hold => "<held>",
            EquipmentSlot::Float => "<floating nearby>",
        }
    }

    /// Get all slots in display order (top to bottom on character)
    pub fn all_slots_in_order() -> Vec<EquipmentSlot> {
        vec![
            EquipmentSlot::Light,
            EquipmentSlot::FingerL,
            EquipmentSlot::FingerR,
            EquipmentSlot::Neck1,
            EquipmentSlot::Neck2,
            EquipmentSlot::Body,
            EquipmentSlot::Head,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
            EquipmentSlot::Hands,
            EquipmentSlot::Arms,
            EquipmentSlot::Shield,
            EquipmentSlot::About,
            EquipmentSlot::Waist,
            EquipmentSlot::WristL,
            EquipmentSlot::WristR,
            EquipmentSlot::Wield,
            EquipmentSlot::Hold,
            EquipmentSlot::Float,
        ]
    }

    /// Parse a slot from its database string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(EquipmentSlot::Light),
            "finger_l" => Some(EquipmentSlot::FingerL),
            "finger_r" => Some(EquipmentSlot::FingerR),
            "neck_1" => Some(EquipmentSlot::Neck1),
            "neck_2" => Some(EquipmentSlot::Neck2),
            "body" => Some(EquipmentSlot::Body),
            "head" => Some(EquipmentSlot::Head),
            "legs" => Some(EquipmentSlot::Legs),
            "feet" => Some(EquipmentSlot::Feet),
            "hands" => Some(EquipmentSlot::Hands),
            "arms" => Some(EquipmentSlot::Arms),
            "shield" => Some(EquipmentSlot::Shield),
            "about" => Some(EquipmentSlot::About),
            "waist" => Some(EquipmentSlot::Waist),
            "wrist_l" => Some(EquipmentSlot::WristL),
            "wrist_r" => Some(EquipmentSlot::WristR),
            "wield" => Some(EquipmentSlot::Wield),
            "hold" => Some(EquipmentSlot::Hold),
            "float" => Some(EquipmentSlot::Float),
            _ => None,
        }
    }

    /// Convert to database string representation
    pub fn to_db_string(&self) -> &str {
        match self {
            EquipmentSlot::Light => "light",
            EquipmentSlot::FingerL => "finger_l",
            EquipmentSlot::FingerR => "finger_r",
            EquipmentSlot::Neck1 => "neck_1",
            EquipmentSlot::Neck2 => "neck_2",
            EquipmentSlot::Body => "body",
            EquipmentSlot::Head => "head",
            EquipmentSlot::Legs => "legs",
            EquipmentSlot::Feet => "feet",
            EquipmentSlot::Hands => "hands",
            EquipmentSlot::Arms => "arms",
            EquipmentSlot::Shield => "shield",
            EquipmentSlot::About => "about",
            EquipmentSlot::Waist => "waist",
            EquipmentSlot::WristL => "wrist_l",
            EquipmentSlot::WristR => "wrist_r",
            EquipmentSlot::Wield => "wield",
            EquipmentSlot::Hold => "hold",
            EquipmentSlot::Float => "float",
        }
    }

    /// Get valid slots for a wear flag string from ROM area files
    /// Returns all possible slots for an item based on its wear_flags
    pub fn from_wear_flags(wear_flags: &str) -> Vec<EquipmentSlot> {
        let mut slots = Vec::new();
        let flags_lower = wear_flags.to_lowercase();

        // Check for each wear flag
        if flags_lower.contains("take") {
            // Item can be picked up (not necessarily wearable)
        }
        if flags_lower.contains("finger") {
            slots.push(EquipmentSlot::FingerL);
            slots.push(EquipmentSlot::FingerR);
        }
        if flags_lower.contains("neck") {
            slots.push(EquipmentSlot::Neck1);
            slots.push(EquipmentSlot::Neck2);
        }
        if flags_lower.contains("body") {
            slots.push(EquipmentSlot::Body);
        }
        if flags_lower.contains("head") {
            slots.push(EquipmentSlot::Head);
        }
        if flags_lower.contains("legs") {
            slots.push(EquipmentSlot::Legs);
        }
        if flags_lower.contains("feet") {
            slots.push(EquipmentSlot::Feet);
        }
        if flags_lower.contains("hands") {
            slots.push(EquipmentSlot::Hands);
        }
        if flags_lower.contains("arms") {
            slots.push(EquipmentSlot::Arms);
        }
        if flags_lower.contains("shield") {
            slots.push(EquipmentSlot::Shield);
        }
        if flags_lower.contains("about") {
            slots.push(EquipmentSlot::About);
        }
        if flags_lower.contains("waist") {
            slots.push(EquipmentSlot::Waist);
        }
        if flags_lower.contains("wrist") {
            slots.push(EquipmentSlot::WristL);
            slots.push(EquipmentSlot::WristR);
        }
        if flags_lower.contains("wield") {
            slots.push(EquipmentSlot::Wield);
        }
        if flags_lower.contains("hold") {
            slots.push(EquipmentSlot::Hold);
        }
        if flags_lower.contains("float") {
            slots.push(EquipmentSlot::Float);
        }

        slots
    }
}

impl fmt::Display for EquipmentSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}
