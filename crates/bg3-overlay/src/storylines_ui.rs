use bg3_save::ipc::{self, ModConfig, ModStatus};
use bg3_save::storylines::{self, StorylineCategory, StorylineConfig, StorylineDefinition};
use egui_overlay::egui;
use egui::Color32;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEFAULT_STORYLINES_TOML: &str = include_str!("../../../storylines.toml");

pub struct StorylinePanel {
    definitions: Vec<StorylineDefinition>,
    config: StorylineConfig,
    mod_status: Option<ModStatus>,
    last_status_check: Option<Instant>,
    load_error: Option<String>,
    storylines_path: Option<PathBuf>,
}

impl StorylinePanel {
    pub fn new() -> Self {
        let mut panel = Self {
            definitions: Vec::new(),
            config: StorylineConfig::default(),
            mod_status: None,
            last_status_check: None,
            load_error: None,
            storylines_path: None,
        };
        panel.load_definitions();
        panel.load_config_from_ipc();
        panel
    }

    fn load_definitions(&mut self) {
        // Try loading from file next to the executable first
        if let Ok(exe) = std::env::current_exe() {
            let toml_path = exe.with_file_name("storylines.toml");
            if toml_path.exists() {
                match storylines::load_storylines(&toml_path) {
                    Ok(defs) => {
                        self.storylines_path = Some(toml_path);
                        self.definitions = defs;
                        self.load_error = None;
                        return;
                    }
                    Err(e) => {
                        self.load_error = Some(format!("Error loading storylines.toml: {}", e));
                    }
                }
            }
        }

        // Fall back to embedded default
        match storylines::load_storylines_from_str(DEFAULT_STORYLINES_TOML) {
            Ok(defs) => {
                self.definitions = defs;
                if self.load_error.is_none() {
                    self.load_error = None;
                }
            }
            Err(e) => {
                self.load_error = Some(format!("Error parsing embedded storylines: {}", e));
            }
        }
    }

    fn load_config_from_ipc(&mut self) {
        // Read existing config from IPC dir so we reflect the current state
        if let Some(path) = ipc::config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mod_config) = serde_json::from_str::<ModConfig>(&content) {
                    for id in &mod_config.disabled_storylines {
                        self.config.set_enabled(id, false);
                    }
                }
            }
        }
    }

    fn write_config(&self) {
        let disabled: Vec<String> = self
            .definitions
            .iter()
            .filter(|d| self.config.is_disabled(&d.id))
            .map(|d| d.id.clone())
            .collect();

        let mod_config = ModConfig {
            disabled_storylines: disabled,
        };

        if let Err(e) = ipc::write_config(&mod_config) {
            eprintln!("Failed to write IPC config: {}", e);
        }
    }

    fn check_mod_status(&mut self) {
        let should_check = self
            .last_status_check
            .is_none_or(|t| t.elapsed() > Duration::from_secs(3));

        if should_check {
            self.mod_status = ipc::read_status();
            self.last_status_check = Some(Instant::now());
        }
    }

    fn blocked_count_for(&self, storyline_id: &str) -> usize {
        self.mod_status
            .as_ref()
            .map(|s| {
                s.blocked_events
                    .iter()
                    .filter(|e| e.storyline_id == storyline_id)
                    .count()
            })
            .unwrap_or(0)
    }

    pub fn update(&mut self, ui: &mut egui::Ui) {
        self.check_mod_status();

        // Mod connection status
        ui.horizontal(|ui| {
            if let Some(status) = &self.mod_status {
                if status.active {
                    ui.colored_label(Color32::GREEN, "Mod connected");
                    let total_blocked: usize = self
                        .definitions
                        .iter()
                        .map(|d| self.blocked_count_for(&d.id))
                        .sum();
                    if total_blocked > 0 {
                        ui.label(format!("({} events blocked)", total_blocked));
                    }
                } else {
                    ui.colored_label(Color32::YELLOW, "Mod inactive");
                }
            } else {
                ui.colored_label(Color32::GRAY, "Mod not detected");
            }
        });

        if let Some(err) = &self.load_error {
            ui.colored_label(Color32::RED, err);
        }

        ui.separator();

        if self.definitions.is_empty() {
            ui.label("No storyline definitions loaded.");
            return;
        }

        // Group by category
        let categories = [
            StorylineCategory::MainQuest,
            StorylineCategory::CompanionQuest,
            StorylineCategory::SideQuest,
            StorylineCategory::WorldEvent,
        ];

        let mut config_changed = false;
        let definitions = self.definitions.clone();

        for category in &categories {
            let in_category: Vec<&StorylineDefinition> = definitions
                .iter()
                .filter(|d| d.category == *category)
                .collect();

            if in_category.is_empty() {
                continue;
            }

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(category.label())
                    .strong()
                    .color(Color32::WHITE),
            );

            for def in &in_category {
                let is_enabled = !self.config.is_disabled(&def.id);
                let blocked = self.blocked_count_for(&def.id);

                ui.horizontal(|ui| {
                    let mut enabled = is_enabled;
                    if ui.checkbox(&mut enabled, "").changed() {
                        self.config.set_enabled(&def.id, enabled);
                        config_changed = true;
                    }

                    let label_color = if is_enabled {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(255, 100, 100)
                    };

                    let label_text = if !is_enabled && blocked > 0 {
                        format!("{} (blocked {})", def.name, blocked)
                    } else {
                        def.name.clone()
                    };

                    let response =
                        ui.label(egui::RichText::new(&label_text).color(label_color));

                    if !def.description.is_empty() {
                        response.on_hover_text(&def.description);
                    }
                });
            }
        }

        if config_changed {
            self.write_config();
        }

        // Show IPC path info at the bottom
        ui.add_space(8.0);
        ui.separator();
        if let Some(dir) = ipc::ipc_dir() {
            ui.label(
                egui::RichText::new(format!("IPC: {}", dir.display()))
                    .small()
                    .color(Color32::DARK_GRAY),
            );
        }
        if let Some(path) = &self.storylines_path {
            ui.label(
                egui::RichText::new(format!("Defs: {}", path.display()))
                    .small()
                    .color(Color32::DARK_GRAY),
            );
        } else {
            ui.label(
                egui::RichText::new("Defs: embedded default")
                    .small()
                    .color(Color32::DARK_GRAY),
            );
        }
    }
}
