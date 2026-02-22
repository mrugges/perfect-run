use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A storyline that can be toggled on/off in the mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: StorylineCategory,
    #[serde(default)]
    pub hooks: Vec<StoryHook>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorylineCategory {
    MainQuest,
    CompanionQuest,
    SideQuest,
    WorldEvent,
}

impl StorylineCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::MainQuest => "Main Quest",
            Self::CompanionQuest => "Companion Quest",
            Self::SideQuest => "Side Quest",
            Self::WorldEvent => "World Event",
        }
    }
}

/// A hook describing what the mod should intercept for a storyline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryHook {
    BlockFlag {
        guid: String,
        description: String,
    },
    BlockDialog {
        pattern: String,
        description: String,
    },
    BlockQuest {
        quest_id: String,
        description: String,
    },
    ClearFlagOnEvent {
        event: String,
        flag_guid: String,
        description: String,
    },
    CustomLua {
        code: String,
        description: String,
    },
}

/// Top-level TOML structure for storyline definitions file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineFile {
    #[serde(rename = "storyline")]
    pub storylines: Vec<StorylineDefinition>,
}

/// Runtime config: which storylines are disabled.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorylineConfig {
    /// storyline id -> enabled (true = storyline plays normally, false = blocked)
    pub storylines: HashMap<String, bool>,
}

impl StorylineConfig {
    pub fn is_disabled(&self, id: &str) -> bool {
        self.storylines.get(id).copied() == Some(false)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        self.storylines.insert(id.to_string(), enabled);
    }
}

/// Load storyline definitions from a TOML file.
pub fn load_storylines(path: &Path) -> Result<Vec<StorylineDefinition>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let file: StorylineFile =
        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;
    Ok(file.storylines)
}

/// Load storyline definitions from a TOML string (for embedded defaults).
pub fn load_storylines_from_str(content: &str) -> Result<Vec<StorylineDefinition>, String> {
    let file: StorylineFile =
        toml::from_str(content).map_err(|e| format!("Failed to parse TOML: {}", e))?;
    Ok(file.storylines)
}
