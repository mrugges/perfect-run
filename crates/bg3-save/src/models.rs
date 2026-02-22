use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveInfo {
    pub path: PathBuf,
    pub character_name: String,
    pub save_name: String,
    pub timestamp: SystemTime,
    pub is_honour_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyData {
    pub characters: Vec<Character>,
    pub gold: Option<u64>,
    pub day: Option<u32>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub race: String,
    pub abilities: AbilityScores,
    pub hp: Option<(u32, u32)>,
    pub equipment: Vec<EquipmentSlot>,
    pub is_player: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityScores {
    pub strength: u32,
    pub dexterity: u32,
    pub constitution: u32,
    pub intelligence: u32,
    pub wisdom: u32,
    pub charisma: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSlot {
    pub slot: SlotType,
    pub item_name: String,
    pub template_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlotType {
    Head,
    Chest,
    Hands,
    Feet,
    MainHand,
    OffHand,
    Amulet,
    Ring1,
    Ring2,
    Cloak,
    Ranged,
    Other(String),
}
