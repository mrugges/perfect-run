use crate::Error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// IPC protocol version. Both overlay and mod must agree on this.
pub const IPC_VERSION: u32 = 1;

/// Returns the IPC directory used for overlay <-> mod communication.
/// `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\Script Extender\perfect-run\`
pub fn ipc_dir() -> Option<PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    Some(
        PathBuf::from(local_app_data)
            .join("Larian Studios")
            .join("Baldur's Gate 3")
            .join("Script Extender")
            .join("perfect-run"),
    )
}

/// Path to the config file the overlay writes and the mod reads.
pub fn config_path() -> Option<PathBuf> {
    ipc_dir().map(|d| d.join("config.json"))
}

/// Path to the status file the mod writes and the overlay reads.
pub fn status_path() -> Option<PathBuf> {
    ipc_dir().map(|d| d.join("status.json"))
}

/// Config written by the overlay, read by the Lua mod.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModConfig {
    /// IPC protocol version.
    #[serde(default)]
    pub version: u32,
    /// List of storyline IDs that should be blocked.
    pub disabled_storylines: Vec<String>,
}

/// Status written by the Lua mod, read by the overlay.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModStatus {
    /// IPC protocol version.
    #[serde(default)]
    pub version: u32,
    /// Whether the mod is currently active and polling.
    pub active: bool,
    /// Timestamp (Unix epoch seconds) of last status update.
    pub last_update: u64,
    /// Log of recently blocked events.
    pub blocked_events: Vec<BlockedEvent>,
}

/// A single event that was blocked by the mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedEvent {
    /// Which storyline triggered this block.
    pub storyline_id: String,
    /// Human-readable description of what was blocked.
    pub description: String,
    /// Unix epoch seconds when the event was blocked.
    pub timestamp: u64,
}

/// Atomically write a JSON file by writing to a temp file then renaming.
fn atomic_write_json(path: &std::path::Path, json: &str) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Write the mod config to the IPC directory. Creates the directory if needed.
/// Uses atomic write (temp file + rename) to prevent partial reads.
pub fn write_config(config: &ModConfig) -> Result<(), Error> {
    let path = config_path()
        .ok_or_else(|| Error::Other("Could not determine IPC directory".into()))?;
    let mut config = config.clone();
    config.version = IPC_VERSION;
    let json = serde_json::to_string_pretty(&config)?;
    atomic_write_json(&path, &json)
}

/// Read the mod status from the IPC directory. Returns None if file doesn't exist.
pub fn read_status() -> Option<ModStatus> {
    let path = status_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // --- Strategies ---

    fn arb_blocked_event() -> impl Strategy<Value = BlockedEvent> {
        ("[a-z_]{1,30}", ".{0,100}", 0..u64::MAX).prop_map(
            |(storyline_id, description, timestamp)| BlockedEvent {
                storyline_id,
                description,
                timestamp,
            },
        )
    }

    fn arb_mod_config() -> impl Strategy<Value = ModConfig> {
        (0..10u32, prop::collection::vec("[a-z_]{1,30}", 0..20)).prop_map(
            |(version, disabled_storylines)| ModConfig {
                version,
                disabled_storylines,
            },
        )
    }

    fn arb_mod_status() -> impl Strategy<Value = ModStatus> {
        (
            0..10u32,
            any::<bool>(),
            any::<u64>(),
            prop::collection::vec(arb_blocked_event(), 0..10),
        )
            .prop_map(|(version, active, last_update, blocked_events)| {
                ModStatus {
                    version,
                    active,
                    last_update,
                    blocked_events,
                }
            })
    }

    // --- Property tests ---

    proptest! {
        #[test]
        fn mod_config_json_round_trip(config in arb_mod_config()) {
            let json = serde_json::to_string(&config).unwrap();
            let parsed: ModConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(config, parsed);
        }

        #[test]
        fn mod_status_json_round_trip(status in arb_mod_status()) {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: ModStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(status, parsed);
        }

        #[test]
        fn mod_config_pretty_round_trip(config in arb_mod_config()) {
            let json = serde_json::to_string_pretty(&config).unwrap();
            let parsed: ModConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(config, parsed);
        }

        #[test]
        fn blocked_event_round_trip(event in arb_blocked_event()) {
            let json = serde_json::to_string(&event).unwrap();
            let parsed: BlockedEvent = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(event, parsed);
        }
    }

    // --- Edge-case tests ---

    #[test]
    fn mod_config_default_version() {
        // Config without version field (e.g. from older overlay)
        let json = r#"{"disabled_storylines": ["test"]}"#;
        let config: ModConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 0);
        assert_eq!(config.disabled_storylines, vec!["test"]);
    }

    #[test]
    fn mod_status_empty_events() {
        let json = r#"{"version": 1, "active": false, "last_update": 0, "blocked_events": []}"#;
        let status: ModStatus = serde_json::from_str(json).unwrap();
        assert!(!status.active);
        assert!(status.blocked_events.is_empty());
    }

    #[test]
    fn atomic_write_creates_file() {
        let dir = std::env::temp_dir().join("perfect-run-test-ipc");
        let _ = std::fs::remove_dir_all(&dir);

        let path = dir.join("test.json");
        atomic_write_json(&path, r#"{"test": true}"#).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("test"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
