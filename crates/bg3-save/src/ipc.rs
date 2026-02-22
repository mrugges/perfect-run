use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModConfig {
    /// List of storyline IDs that should be blocked.
    pub disabled_storylines: Vec<String>,
}

/// Status written by the Lua mod, read by the overlay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModStatus {
    /// Whether the mod is currently active and polling.
    pub active: bool,
    /// Timestamp (Unix epoch seconds) of last status update.
    pub last_update: u64,
    /// Log of recently blocked events.
    pub blocked_events: Vec<BlockedEvent>,
}

/// A single event that was blocked by the mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedEvent {
    /// Which storyline triggered this block.
    pub storyline_id: String,
    /// Human-readable description of what was blocked.
    pub description: String,
    /// Unix epoch seconds when the event was blocked.
    pub timestamp: u64,
}

/// Write the mod config to the IPC directory. Creates the directory if needed.
pub fn write_config(config: &ModConfig) -> Result<(), String> {
    let path = config_path().ok_or("Could not determine IPC directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create IPC directory: {}", e))?;
    }
    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))
}

/// Read the mod status from the IPC directory. Returns None if file doesn't exist.
pub fn read_status() -> Option<ModStatus> {
    let path = status_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}
