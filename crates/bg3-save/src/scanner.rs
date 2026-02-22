use crate::lsv;
use crate::models::SaveInfo;
use crate::Error;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Default BG3 save location on Windows.
pub fn default_save_path() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\Default".to_string());
            format!(r"{}\AppData\Local", home)
        });
    PathBuf::from(local_app_data)
        .join("Larian Studios")
        .join("Baldur's Gate 3")
        .join("PlayerProfiles")
        .join("Public")
        .join("Savegames")
        .join("Story")
}

pub struct SaveScanner {
    save_root: PathBuf,
}

impl SaveScanner {
    pub fn new(save_root: PathBuf) -> Self {
        Self { save_root }
    }

    /// Find all .lsv save files in the save directory.
    pub fn find_saves(&self) -> Vec<PathBuf> {
        let mut saves = Vec::new();
        for entry in WalkDir::new(&self.save_root)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("lsv")) {
                saves.push(path.to_path_buf());
            }
        }
        saves.sort();
        saves
    }

    /// Scan all saves and extract basic info.
    pub fn scan_all(&self) -> Vec<Result<SaveInfo, Error>> {
        self.find_saves()
            .into_iter()
            .map(|path| self.scan_save(&path))
            .collect()
    }

    /// Extract info from a single save file using SaveInfo.json.
    pub fn scan_save(&self, lsv_path: &Path) -> Result<SaveInfo, Error> {
        let (mut reader, package) = lsv::open_package(lsv_path)?;

        let dir_name = lsv_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_honour_mode = dir_name.contains("HonourMode");

        let timestamp = lsv_path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Extract character name and save name from SaveInfo.json
        let (character_name, save_name) = match extract_from_save_info(&mut reader, &package) {
            Ok((name, sname)) => (name, sname),
            Err(_) => {
                // Fallback: parse from directory name
                let name = extract_name_from_dir(&dir_name);
                let sname = dir_name
                    .split("__")
                    .last()
                    .unwrap_or(&dir_name)
                    .to_string();
                (name, sname)
            }
        };

        Ok(SaveInfo {
            path: lsv_path.to_path_buf(),
            character_name,
            save_name,
            timestamp,
            is_honour_mode,
        })
    }
}

impl Default for SaveScanner {
    fn default() -> Self {
        Self::new(default_save_path())
    }
}

/// Extract character name from the directory name pattern "Name-numbers__SaveName"
fn extract_name_from_dir(dir_name: &str) -> String {
    // For patterns like "Mrugge-212312515227__Whispering Depths"
    if let Some(prefix) = dir_name.split("__").next() {
        if let Some(name) = prefix.split('-').next() {
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    // For UUID-based Honour Mode dirs, no name available
    "(unknown)".to_string()
}

/// Extract character info from SaveInfo.json inside the save package.
fn extract_from_save_info(
    reader: &mut bg3_lib::package_reader::PackageReader,
    package: &bg3_lib::package::Package,
) -> Result<(String, String), Error> {
    let pfi = package
        .files
        .iter()
        .find(|f| {
            f.name
                .to_string_lossy()
                .to_lowercase()
                .contains("saveinfo.json")
        })
        .ok_or_else(|| Error::FileNotFound("SaveInfo.json".into()))?;

    let data = reader.decompress_file(pfi).map_err(Error::Package)?;
    let text = String::from_utf8(data)?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let save_name = json
        .get("Save Name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown)")
        .to_string();

    // Build character name from the first character's info
    // The player character is usually Origin "Generic" or "DarkUrge"
    let character_name = if let Some(chars) = json
        .pointer("/Active Party/Characters")
        .and_then(|v| v.as_array())
    {
        // Find the player character (Origin: Generic or DarkUrge)
        let player = chars
            .iter()
            .find(|c| {
                let origin = c.get("Origin").and_then(|v| v.as_str()).unwrap_or("");
                origin == "Generic" || origin == "DarkUrge"
            })
            .or_else(|| chars.first());

        if let Some(pc) = player {
            let origin = pc
                .get("Origin")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let race = pc
                .get("Race")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let level = pc
                .get("Level")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let class = pc
                .get("Classes")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|c| c.get("Main"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            if origin == "Generic" || origin == "DarkUrge" {
                format!("{} Lvl{} {} {}", origin, level, race, class)
            } else {
                format!("{} Lvl{} {}", origin, level, class)
            }
        } else {
            "(no characters)".to_string()
        }
    } else {
        "(no party data)".to_string()
    };

    Ok((character_name, save_name))
}

/// Parse SaveInfo.json and return structured party data.
pub fn parse_save_info_json(json_text: &str) -> Result<crate::models::PartyData, Error> {
    let json: serde_json::Value = serde_json::from_str(json_text)?;

    let mut characters = Vec::new();

    if let Some(chars) = json
        .pointer("/Active Party/Characters")
        .and_then(|v| v.as_array())
    {
        for c in chars {
            let origin = c
                .get("Origin")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let race = c
                .get("Race")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let level = c
                .get("Level")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            // Build class string from all classes (multiclass support)
            let class = c
                .get("Classes")
                .and_then(|v| v.as_array())
                .map(|classes| {
                    classes
                        .iter()
                        .filter_map(|cls| {
                            let main = cls.get("Main").and_then(|v| v.as_str())?;
                            let sub = cls.get("Sub").and_then(|v| v.as_str()).unwrap_or("");
                            if sub.is_empty() {
                                Some(main.to_string())
                            } else {
                                Some(format!("{} ({})", main, sub))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" / ")
                })
                .unwrap_or_else(|| "Unknown".to_string());

            let is_player = origin == "Generic" || origin == "DarkUrge";

            // Companions have their origin as their name (e.g. "Shadowheart"),
            // while player characters use a generic origin that isn't a useful name.
            let name = if is_player {
                // Build a descriptive name for the player character
                let race = c.get("Race").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let class = c.get("Classes")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|cls| cls.get("Main"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                format!("{} {}", race, class)
            } else {
                origin.clone()
            };

            characters.push(crate::models::Character {
                name,
                class,
                level,
                race,
                abilities: crate::models::AbilityScores::default(),
                hp: None,
                equipment: Vec::new(),
                is_player,
            });
        }
    }

    let location = json
        .get("Current Level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(crate::models::PartyData {
        characters,
        gold: None,
        day: None,
        location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // --- Strategies ---

    const RACES: &[&str] = &[
        "Human", "Elf", "Half-Elf", "Dwarf", "Halfling", "Gnome",
        "Tiefling", "Dragonborn", "Half-Orc", "Githyanki",
    ];
    const CLASSES: &[&str] = &[
        "Fighter", "Wizard", "Cleric", "Rogue", "Ranger", "Paladin",
        "Barbarian", "Bard", "Druid", "Sorcerer", "Warlock", "Monk",
    ];
    const ORIGINS: &[&str] = &[
        "Generic", "DarkUrge", "Shadowheart", "Astarion",
        "Gale", "Laezel", "Wyll", "Karlach",
    ];

    fn arb_origin() -> impl Strategy<Value = &'static str> {
        prop::sample::select(ORIGINS)
    }

    fn arb_race() -> impl Strategy<Value = &'static str> {
        prop::sample::select(RACES)
    }

    fn arb_class() -> impl Strategy<Value = &'static str> {
        prop::sample::select(CLASSES)
    }

    #[derive(Debug, Clone)]
    struct TestChar {
        origin: &'static str,
        race: &'static str,
        class: &'static str,
        level: u32,
    }

    fn arb_char() -> impl Strategy<Value = TestChar> {
        (arb_origin(), arb_race(), arb_class(), 1..13u32).prop_map(
            |(origin, race, class, level)| TestChar {
                origin,
                race,
                class,
                level,
            },
        )
    }

    /// Build a SaveInfo.json string from test characters.
    fn build_save_json(chars: &[TestChar], location: Option<&str>) -> String {
        let char_entries: Vec<String> = chars
            .iter()
            .map(|c| {
                format!(
                    r#"{{"Origin": "{}", "Race": "{}", "Level": {}, "Classes": [{{"Main": "{}", "Sub": ""}}]}}"#,
                    c.origin, c.race, c.level, c.class
                )
            })
            .collect();

        let loc_field = location
            .map(|l| format!(r#""Current Level": "{}","#, l))
            .unwrap_or_default();

        format!(
            r#"{{{} "Active Party": {{"Characters": [{}]}}}}"#,
            loc_field,
            char_entries.join(", ")
        )
    }

    // --- Property tests ---

    proptest! {
        #[test]
        fn parse_preserves_character_count(chars in prop::collection::vec(arb_char(), 0..6)) {
            let json = build_save_json(&chars, None);
            let party = parse_save_info_json(&json).unwrap();
            prop_assert_eq!(party.characters.len(), chars.len());
        }

        #[test]
        fn parse_preserves_character_fields(chars in prop::collection::vec(arb_char(), 1..6)) {
            let json = build_save_json(&chars, Some("TestLevel"));
            let party = parse_save_info_json(&json).unwrap();

            prop_assert_eq!(party.location.as_deref(), Some("TestLevel"));

            for (input, parsed) in chars.iter().zip(party.characters.iter()) {
                let is_player = input.origin == "Generic" || input.origin == "DarkUrge";
                prop_assert_eq!(parsed.is_player, is_player);
                prop_assert_eq!(&parsed.race, input.race);
                prop_assert_eq!(parsed.level, input.level);
                prop_assert!(parsed.class.contains(input.class));

                if is_player {
                    let expected = format!("{} {}", input.race, input.class);
                    prop_assert_eq!(&parsed.name, &expected);
                } else {
                    prop_assert_eq!(&parsed.name, input.origin);
                }
            }
        }

        #[test]
        fn parse_empty_json_never_panics(json in "\\{[^}]{0,200}\\}") {
            // Should either parse or return an error, never panic
            let _ = parse_save_info_json(&json);
        }

        #[test]
        fn dir_name_extraction_never_panics(name in ".{0,100}") {
            // Should never panic regardless of input
            let _ = extract_name_from_dir(&name);
        }
    }

    // --- Hand-written edge-case tests ---

    #[test]
    fn parse_save_info_with_party() {
        let json = r#"{
            "Save Name": "Goblin Camp",
            "Current Level": "CYS_Main_A",
            "Active Party": {
                "Characters": [
                    {
                        "Origin": "Generic",
                        "Race": "Human",
                        "Level": 5,
                        "Classes": [
                            {"Main": "Fighter", "Sub": "Champion"}
                        ]
                    },
                    {
                        "Origin": "Shadowheart",
                        "Race": "Half-Elf",
                        "Level": 5,
                        "Classes": [
                            {"Main": "Cleric", "Sub": "Life"}
                        ]
                    }
                ]
            }
        }"#;

        let party = parse_save_info_json(json).unwrap();
        assert_eq!(party.characters.len(), 2);
        assert_eq!(party.location.as_deref(), Some("CYS_Main_A"));

        let player = &party.characters[0];
        assert!(player.is_player);
        assert_eq!(player.name, "Human Fighter");

        let companion = &party.characters[1];
        assert!(!companion.is_player);
        assert_eq!(companion.name, "Shadowheart");
    }

    #[test]
    fn parse_save_info_multiclass() {
        let json = r#"{
            "Active Party": {
                "Characters": [
                    {
                        "Origin": "Shadowheart",
                        "Race": "Half-Elf",
                        "Level": 8,
                        "Classes": [
                            {"Main": "Cleric", "Sub": "Life"},
                            {"Main": "Fighter", "Sub": ""}
                        ]
                    }
                ]
            }
        }"#;

        let party = parse_save_info_json(json).unwrap();
        let ch = &party.characters[0];
        assert!(ch.class.contains("Cleric"));
        assert!(ch.class.contains("Fighter"));
    }

    #[test]
    fn parse_save_info_empty_party() {
        let json = r#"{"Save Name": "Empty"}"#;
        let party = parse_save_info_json(json).unwrap();
        assert!(party.characters.is_empty());
        assert!(party.location.is_none());
    }

    #[test]
    fn parse_save_info_invalid_json() {
        let result = parse_save_info_json("not json at all {{{");
        assert!(result.is_err());
    }

    #[test]
    fn extract_name_from_directory() {
        assert_eq!(extract_name_from_dir("Mrugge-212312__Goblin Camp"), "Mrugge");
        assert_eq!(extract_name_from_dir("Player-123__QuickSave"), "Player");
        assert_eq!(extract_name_from_dir("some-uuid-name"), "some");
    }
}
