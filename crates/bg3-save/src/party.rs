use crate::lsf;
use crate::models::*;
use bg3_lib::lsf_reader::{Node, RegionArena, Resource};

/// Extract party data from a parsed globals.lsf resource.
pub fn extract_party(resource: &Resource) -> Result<PartyData, String> {
    let arena = &resource.regions;

    let characters = extract_characters(arena);

    Ok(PartyData {
        characters,
        gold: extract_gold(arena),
        day: extract_day(arena),
        location: extract_location(arena),
    })
}

/// Extract character data from the globals LSF tree.
fn extract_characters(arena: &RegionArena) -> Vec<Character> {
    let mut characters = Vec::new();

    // BG3 stores characters in various locations in the globals.lsf tree.
    // Common patterns:
    // - Characters node with child character entries
    // - Each character has PlayerData, Stats, Inventory children

    // Strategy: find all nodes that look like character definitions
    // and extract what we can from each.

    // Look for character-related nodes
    for node in &arena.node_instances {
        // Characters are often under nodes named "Character" with race/class info
        if node.name == "Character" || node.name == "PlayerCustomData" {
            if let Some(char) = try_extract_character(arena, node) {
                characters.push(char);
            }
        }
    }

    // If we didn't find characters that way, look for party members
    if characters.is_empty() {
        for node in &arena.node_instances {
            if node.name == "PartyMember" || node.name == "PartyMembers" {
                // Try to get character references from party member nodes
                if let Some(char) = try_extract_character(arena, node) {
                    characters.push(char);
                }
            }
        }
    }

    characters
}

/// Try to extract a Character from a node that might contain character data.
fn try_extract_character(arena: &RegionArena, node: &Node) -> Option<Character> {
    let name = lsf::get_string_attr(node, "Name")
        .or_else(|| lsf::get_translated_string_attr(node, "Name"))
        .or_else(|| lsf::get_string_attr(node, "CustomName"))
        .or_else(|| lsf::get_translated_string_attr(node, "CustomName"))
        .or_else(|| lsf::get_string_attr(node, "DisplayName"))
        .or_else(|| lsf::get_translated_string_attr(node, "DisplayName"))?;

    if name.is_empty() {
        return None;
    }

    let class = lsf::get_string_attr(node, "ClassId")
        .or_else(|| lsf::get_string_attr(node, "Class"))
        .unwrap_or_else(|| "Unknown".to_string());

    let level = lsf::get_uint_attr(node, "Level")
        .or_else(|| lsf::get_int_attr(node, "Level").map(|v| v as u32))
        .unwrap_or(0);

    let race = lsf::get_string_attr(node, "Race")
        .or_else(|| lsf::get_string_attr(node, "RaceId"))
        .unwrap_or_else(|| "Unknown".to_string());

    let is_player = lsf::get_bool_attr(node, "IsPlayer").unwrap_or(false)
        || lsf::get_string_attr(node, "IsPlayer")
            .map(|s| s == "True" || s == "1")
            .unwrap_or(false);

    // Try to extract ability scores from child nodes
    let abilities = extract_abilities(arena, node);

    // Try to extract HP
    let hp = extract_hp(node);

    // Try to extract equipment
    let equipment = extract_equipment(arena, node);

    Some(Character {
        name,
        class,
        level,
        race,
        abilities,
        hp,
        equipment,
        is_player,
    })
}

/// Extract ability scores from a character node or its children.
fn extract_abilities(arena: &RegionArena, node: &Node) -> AbilityScores {
    let mut scores = AbilityScores::default();

    // Try direct attributes first
    if let Some(v) = lsf::get_uint_attr(node, "Strength") {
        scores.strength = v;
    }
    if let Some(v) = lsf::get_uint_attr(node, "Dexterity") {
        scores.dexterity = v;
    }
    if let Some(v) = lsf::get_uint_attr(node, "Constitution") {
        scores.constitution = v;
    }
    if let Some(v) = lsf::get_uint_attr(node, "Intelligence") {
        scores.intelligence = v;
    }
    if let Some(v) = lsf::get_uint_attr(node, "Wisdom") {
        scores.wisdom = v;
    }
    if let Some(v) = lsf::get_uint_attr(node, "Charisma") {
        scores.charisma = v;
    }

    // Try looking in Abilities child nodes
    if let Some(ability_indices) = node.children.get("Abilities") {
        for &idx in ability_indices {
            if let Some(ability_node) = arena.get_node(idx) {
                if let (Some(id), Some(val)) = (
                    lsf::get_string_attr(ability_node, "Id")
                        .or_else(|| lsf::get_string_attr(ability_node, "Name")),
                    lsf::get_uint_attr(ability_node, "Value")
                        .or_else(|| lsf::get_uint_attr(ability_node, "Base")),
                ) {
                    match id.as_str() {
                        "Strength" => scores.strength = val,
                        "Dexterity" => scores.dexterity = val,
                        "Constitution" => scores.constitution = val,
                        "Intelligence" => scores.intelligence = val,
                        "Wisdom" => scores.wisdom = val,
                        "Charisma" => scores.charisma = val,
                        _ => {}
                    }
                }
            }
        }
    }

    scores
}

/// Extract HP from a character node.
fn extract_hp(node: &Node) -> Option<(u32, u32)> {
    let current = lsf::get_uint_attr(node, "CurrentHP")
        .or_else(|| lsf::get_int_attr(node, "CurrentHP").map(|v| v as u32))?;
    let max = lsf::get_uint_attr(node, "MaxHP")
        .or_else(|| lsf::get_int_attr(node, "MaxHP").map(|v| v as u32))
        .unwrap_or(current);
    Some((current, max))
}

/// Extract equipment from a character node's children.
fn extract_equipment(arena: &RegionArena, node: &Node) -> Vec<EquipmentSlot> {
    let mut equipment = Vec::new();

    // Look for Equipment or EquippedItems children
    let slot_keys = ["Equipment", "EquippedItems", "Equipments"];
    for key in &slot_keys {
        if let Some(indices) = node.children.get(*key) {
            for &idx in indices {
                if let Some(equip_node) = arena.get_node(idx) {
                    if let Some(slot) = try_extract_equipment_slot(equip_node) {
                        equipment.push(slot);
                    }
                    // Also check children of the equipment container
                    for (_name, child_indices) in &equip_node.children {
                        for &child_idx in child_indices {
                            if let Some(child) = arena.get_node(child_idx) {
                                if let Some(slot) = try_extract_equipment_slot(child) {
                                    equipment.push(slot);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    equipment
}

fn try_extract_equipment_slot(node: &Node) -> Option<EquipmentSlot> {
    let item_name = lsf::get_string_attr(node, "Name")
        .or_else(|| lsf::get_string_attr(node, "ItemName"))
        .or_else(|| lsf::get_translated_string_attr(node, "DisplayName"))?;

    let slot_name = lsf::get_string_attr(node, "Slot")
        .or_else(|| lsf::get_string_attr(node, "SlotName"))
        .unwrap_or_else(|| "Other".to_string());

    let template_id = lsf::get_string_attr(node, "TemplateName")
        .or_else(|| lsf::get_uuid_attr(node, "MapKey"))
        .or_else(|| lsf::get_uuid_attr(node, "TemplateID"))
        .unwrap_or_default();

    let slot = match slot_name.as_str() {
        "Helmet" | "Head" => SlotType::Head,
        "Breast" | "Chest" | "Body" => SlotType::Chest,
        "Gloves" | "Hands" => SlotType::Hands,
        "Boots" | "Feet" => SlotType::Feet,
        "Melee Main Weapon" | "MainHand" => SlotType::MainHand,
        "Melee Off-Hand Weapon" | "OffHand" => SlotType::OffHand,
        "Amulet" => SlotType::Amulet,
        "Ring" | "Ring1" => SlotType::Ring1,
        "Ring2" => SlotType::Ring2,
        "Cloak" => SlotType::Cloak,
        "Ranged Main Weapon" | "Ranged" => SlotType::Ranged,
        other => SlotType::Other(other.to_string()),
    };

    Some(EquipmentSlot {
        slot,
        item_name,
        template_id,
    })
}

/// Attempt to extract gold amount from the globals tree.
fn extract_gold(arena: &RegionArena) -> Option<u64> {
    for node in &arena.node_instances {
        if node.name == "PartyInfo" || node.name == "Party" {
            if let Some(gold) = lsf::get_uint64_attr(node, "Gold") {
                return Some(gold);
            }
            if let Some(gold) = lsf::get_uint_attr(node, "Gold") {
                return Some(gold as u64);
            }
        }
    }
    None
}

/// Attempt to extract in-game day.
fn extract_day(arena: &RegionArena) -> Option<u32> {
    for node in &arena.node_instances {
        if let Some(day) = lsf::get_uint_attr(node, "GameDay") {
            return Some(day);
        }
        if let Some(day) = lsf::get_uint_attr(node, "Day") {
            return Some(day);
        }
    }
    None
}

/// Attempt to extract current location name.
fn extract_location(arena: &RegionArena) -> Option<String> {
    for node in &arena.node_instances {
        if let Some(loc) = lsf::get_string_attr(node, "LevelName") {
            return Some(loc);
        }
        if let Some(loc) = lsf::get_string_attr(node, "CurrentLevel") {
            return Some(loc);
        }
    }
    None
}
