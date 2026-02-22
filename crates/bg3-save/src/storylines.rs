use crate::Error;
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
        trigger_flag: String,
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
pub fn load_storylines(path: &Path) -> Result<Vec<StorylineDefinition>, Error> {
    let content = std::fs::read_to_string(path)?;
    let file: StorylineFile = toml::from_str(&content)?;
    Ok(file.storylines)
}

/// Load storyline definitions from a TOML string (for embedded defaults).
pub fn load_storylines_from_str(content: &str) -> Result<Vec<StorylineDefinition>, Error> {
    let file: StorylineFile = toml::from_str(content)?;
    Ok(file.storylines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const TEST_TOML: &str = r#"
[[storyline]]
id = "test_quest"
name = "Test Quest"
description = "A test storyline"
category = "main_quest"

    [[storyline.hooks]]
    type = "block_flag"
    guid = "flag-guid-123"
    description = "Block a flag"

    [[storyline.hooks]]
    type = "block_dialog"
    pattern = "TestDialog*"
    description = "Block a dialog"

    [[storyline.hooks]]
    type = "clear_flag_on_event"
    trigger_flag = "trigger-guid"
    flag_guid = "target-guid"
    description = "Clear flag on event"

[[storyline]]
id = "companion_test"
name = "Companion Quest"
description = "A companion quest"
category = "companion_quest"
"#;

    // --- Strategies ---

    fn arb_category() -> impl Strategy<Value = StorylineCategory> {
        prop_oneof![
            Just(StorylineCategory::MainQuest),
            Just(StorylineCategory::CompanionQuest),
            Just(StorylineCategory::SideQuest),
            Just(StorylineCategory::WorldEvent),
        ]
    }

    /// Generate a storyline definition that can round-trip through TOML.
    /// Hooks are excluded since tagged enum TOML round-trip requires careful formatting.
    fn arb_storyline_def() -> impl Strategy<Value = StorylineDefinition> {
        ("[a-z_]{1,20}", "[A-Za-z ]{1,30}", ".{0,80}", arb_category()).prop_map(
            |(id, name, description, category)| StorylineDefinition {
                id,
                name,
                description,
                category,
                hooks: vec![],
            },
        )
    }

    // --- Property tests ---

    proptest! {
        #[test]
        fn config_last_write_wins(
            id in "[a-z_]{1,20}",
            ops in prop::collection::vec(any::<bool>(), 1..50)
        ) {
            let mut config = StorylineConfig::default();
            let mut last = true; // default is enabled (not disabled)

            for enabled in &ops {
                config.set_enabled(&id, *enabled);
                last = *enabled;
            }

            prop_assert_eq!(config.is_disabled(&id), !last);
        }

        #[test]
        fn config_independent_keys(
            ids in prop::collection::hash_set("[a-z]{1,10}", 2..10),
            enabled in any::<bool>()
        ) {
            let mut config = StorylineConfig::default();
            let ids: Vec<_> = ids.into_iter().collect();

            // Disable just the first key
            config.set_enabled(&ids[0], enabled);

            // All other keys should still be at default (not disabled)
            for id in &ids[1..] {
                prop_assert!(!config.is_disabled(id));
            }
            prop_assert_eq!(config.is_disabled(&ids[0]), !enabled);
        }

        #[test]
        fn storyline_def_toml_round_trip(defs in prop::collection::vec(arb_storyline_def(), 1..5)) {
            // Build a StorylineFile, serialize to TOML, parse back
            let file = StorylineFile {
                storylines: defs.clone(),
            };
            let toml_str = toml::to_string(&file).unwrap();
            let parsed: StorylineFile = toml::from_str(&toml_str).unwrap();

            prop_assert_eq!(defs.len(), parsed.storylines.len());
            for (orig, rt) in defs.iter().zip(parsed.storylines.iter()) {
                prop_assert_eq!(&orig.id, &rt.id);
                prop_assert_eq!(&orig.name, &rt.name);
                prop_assert_eq!(&orig.description, &rt.description);
                prop_assert_eq!(orig.category, rt.category);
            }
        }
    }

    // --- Hand-written tests ---

    #[test]
    fn parse_storyline_toml() {
        let defs = load_storylines_from_str(TEST_TOML).unwrap();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].id, "test_quest");
        assert_eq!(defs[0].category, StorylineCategory::MainQuest);
        assert_eq!(defs[0].hooks.len(), 3);
        assert_eq!(defs[1].id, "companion_test");
        assert_eq!(defs[1].category, StorylineCategory::CompanionQuest);
        assert!(defs[1].hooks.is_empty());
    }

    #[test]
    fn parse_hook_types() {
        let defs = load_storylines_from_str(TEST_TOML).unwrap();
        let hooks = &defs[0].hooks;

        match &hooks[0] {
            StoryHook::BlockFlag { guid, .. } => assert_eq!(guid, "flag-guid-123"),
            other => panic!("Expected BlockFlag, got {:?}", other),
        }
        match &hooks[1] {
            StoryHook::BlockDialog { pattern, .. } => assert_eq!(pattern, "TestDialog*"),
            other => panic!("Expected BlockDialog, got {:?}", other),
        }
        match &hooks[2] {
            StoryHook::ClearFlagOnEvent {
                trigger_flag,
                flag_guid,
                ..
            } => {
                assert_eq!(trigger_flag, "trigger-guid");
                assert_eq!(flag_guid, "target-guid");
            }
            other => panic!("Expected ClearFlagOnEvent, got {:?}", other),
        }
    }

    #[test]
    fn parse_embedded_storylines_toml() {
        let content = include_str!("../../../storylines.toml");
        let defs = load_storylines_from_str(content).unwrap();
        assert!(!defs.is_empty(), "Embedded storylines.toml should have entries");

        let guardian = defs.iter().find(|d| d.id == "guardian_emperor");
        assert!(guardian.is_some(), "Should have guardian_emperor storyline");
        assert_eq!(guardian.unwrap().category, StorylineCategory::MainQuest);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = load_storylines_from_str("this is not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn category_labels() {
        assert_eq!(StorylineCategory::MainQuest.label(), "Main Quest");
        assert_eq!(StorylineCategory::CompanionQuest.label(), "Companion Quest");
        assert_eq!(StorylineCategory::SideQuest.label(), "Side Quest");
        assert_eq!(StorylineCategory::WorldEvent.label(), "World Event");
    }
}
