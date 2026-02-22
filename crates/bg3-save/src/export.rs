use crate::models::{Character, PartyData, SaveInfo};

/// Export party data as JSON string.
pub fn to_json(party: &PartyData) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(party)
}

/// Export party data as a markdown-formatted string optimized for LLM context.
pub fn to_markdown(save_info: &SaveInfo, party: &PartyData) -> String {
    let mut md = String::new();

    md.push_str(&format!("# BG3 Save: {}\n\n", save_info.save_name));
    md.push_str(&format!("- **Player**: {}\n", save_info.character_name));
    md.push_str(&format!(
        "- **Mode**: {}\n",
        if save_info.is_honour_mode {
            "Honour Mode"
        } else {
            "Normal"
        }
    ));

    if let Some(loc) = &party.location {
        md.push_str(&format!("- **Location**: {}\n", loc));
    }
    if let Some(gold) = party.gold {
        md.push_str(&format!("- **Gold**: {}\n", gold));
    }
    if let Some(day) = party.day {
        md.push_str(&format!("- **Day**: {}\n", day));
    }

    md.push_str("\n## Party Members\n\n");

    for char in &party.characters {
        format_character(&mut md, char);
    }

    md
}

fn format_character(md: &mut String, char: &Character) {
    md.push_str(&format!(
        "### {} {}\n",
        char.name,
        if char.is_player { "(Player)" } else { "" }
    ));
    md.push_str(&format!(
        "- **Class**: {} | **Level**: {} | **Race**: {}\n",
        char.class, char.level, char.race
    ));

    if let Some((cur, max)) = char.hp {
        md.push_str(&format!("- **HP**: {}/{}\n", cur, max));
    }

    let a = &char.abilities;
    if a.strength > 0 || a.dexterity > 0 {
        md.push_str(&format!(
            "- **Stats**: STR {} | DEX {} | CON {} | INT {} | WIS {} | CHA {}\n",
            a.strength, a.dexterity, a.constitution, a.intelligence, a.wisdom, a.charisma
        ));
    }

    if !char.equipment.is_empty() {
        md.push_str("- **Equipment**:\n");
        for eq in &char.equipment {
            md.push_str(&format!("  - {:?}: {}\n", eq.slot, eq.item_name));
        }
    }

    md.push('\n');
}
