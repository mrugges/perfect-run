use crate::storylines_ui::StorylinePanel;
use bg3_save::models::{Character, PartyData};
use bg3_save::{lsf, lsv, party, SaveScanner};
use egui_overlay::egui;
use egui::Color32;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Party,
    Storylines,
}

pub struct OverlayApp {
    party: Option<PartyData>,
    save_path: Option<PathBuf>,
    last_load: Option<Instant>,
    error: Option<String>,
    expanded_chars: Vec<bool>,
    visible: bool,
    watcher_rx: Option<mpsc::Receiver<()>>,
    active_tab: Tab,
    storyline_panel: StorylinePanel,
}

impl OverlayApp {
    pub fn new(save_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            party: None,
            save_path: None,
            last_load: None,
            error: None,
            expanded_chars: Vec::new(),
            visible: true,
            watcher_rx: None,
            active_tab: Tab::Party,
            storyline_panel: StorylinePanel::new(),
        };

        if let Some(path) = save_path {
            app.load_save(path);
        } else {
            // Auto-detect most recent save
            app.load_most_recent();
        }

        app
    }

    pub fn setup_watcher(&mut self) {
        if let Some(save_path) = &self.save_path {
            let watch_dir = save_path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(save_path.as_path())
                .to_path_buf();

            let (tx, rx) = mpsc::channel();

            std::thread::spawn(move || {
                use notify::{RecursiveMode, Watcher};
                let mut watcher =
                    notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = res {
                            if event.kind.is_modify() || event.kind.is_create() {
                                let _ = tx.send(());
                            }
                        }
                    })
                    .expect("Failed to create file watcher");

                watcher
                    .watch(&watch_dir, RecursiveMode::Recursive)
                    .expect("Failed to watch directory");

                // Keep watcher alive
                loop {
                    std::thread::sleep(Duration::from_secs(60));
                }
            });

            self.watcher_rx = Some(rx);
        }
    }

    fn load_most_recent(&mut self) {
        let scanner = SaveScanner::default();
        let saves = scanner.find_saves();

        if let Some(most_recent) = saves.into_iter().max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        }) {
            self.load_save(most_recent);
        } else {
            self.error = Some("No save files found".to_string());
        }
    }

    fn load_save(&mut self, path: PathBuf) {
        match load_party_from_save(&path) {
            Ok(data) => {
                self.expanded_chars = vec![false; data.characters.len()];
                self.party = Some(data);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
        self.save_path = Some(path);
        self.last_load = Some(Instant::now());
    }

    fn check_for_updates(&mut self) {
        if let Some(rx) = &self.watcher_rx {
            if rx.try_recv().is_ok() {
                // Debounce: only reload if last load was >2 seconds ago
                if self
                    .last_load
                    .is_none_or(|t| t.elapsed() > Duration::from_secs(2))
                {
                    if let Some(path) = self.save_path.clone() {
                        self.load_save(path);
                    }
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.check_for_updates();

        if !self.visible {
            // Minimal indicator when hidden
            egui::Area::new(egui::Id::new("toggle_hint"))
                .fixed_pos(egui::pos2(5.0, 5.0))
                .show(ctx, |ui| {
                    if ui
                        .small_button("PR")
                        .on_hover_text("Click to show overlay")
                        .clicked()
                    {
                        self.visible = true;
                    }
                });
            return;
        }

        egui::Window::new("Perfect Run")
            .default_pos(egui::pos2(10.0, 10.0))
            .default_width(300.0)
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                // Header with controls
                ui.horizontal(|ui| {
                    if ui.small_button("Hide").clicked() {
                        self.visible = false;
                    }
                    if ui.small_button("Refresh").clicked() {
                        if let Some(path) = self.save_path.clone() {
                            self.load_save(path);
                        }
                    }
                    if let Some(path) = &self.save_path {
                        ui.label(
                            egui::RichText::new(
                                path.parent()
                                    .and_then(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            )
                            .small()
                            .color(Color32::GRAY),
                        );
                    }
                });

                // Tab bar
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, Tab::Party, "Party");
                    ui.selectable_value(&mut self.active_tab, Tab::Storylines, "Storylines");
                });

                ui.separator();

                match self.active_tab {
                    Tab::Party => {
                        if let Some(err) = &self.error {
                            ui.colored_label(Color32::RED, err);
                        }

                        if let Some(party) = &self.party {
                            // Party summary
                            ui.horizontal(|ui| {
                                if let Some(loc) = &party.location {
                                    ui.label(format!("📍 {}", loc));
                                }
                                if let Some(gold) = party.gold {
                                    ui.label(format!("💰 {}", gold));
                                }
                                if let Some(day) = party.day {
                                    ui.label(format!("📅 Day {}", day));
                                }
                            });

                            ui.separator();

                            // Character list
                            let party_clone = party.clone();
                            for (i, char) in party_clone.characters.iter().enumerate() {
                                self.render_character(ui, char, i);
                            }
                        } else if self.error.is_none() {
                            ui.label("Loading...");
                        }
                    }
                    Tab::Storylines => {
                        self.storyline_panel.update(ui);
                    }
                }
            });
    }

    fn render_character(&mut self, ui: &mut egui::Ui, char: &Character, idx: usize) {
        let header = format!(
            "{} - Lvl {} {} {}",
            char.name,
            char.level,
            char.class,
            if char.is_player { "⭐" } else { "" }
        );

        let expanded = self.expanded_chars.get(idx).copied().unwrap_or(false);

        let response = ui.selectable_label(expanded, &header);
        if response.clicked() {
            if let Some(exp) = self.expanded_chars.get_mut(idx) {
                *exp = !*exp;
            }
        }

        if expanded {
            ui.indent(egui::Id::new(format!("char_{}", idx)), |ui| {
                ui.label(format!("Race: {}", char.race));

                if let Some((cur, max)) = char.hp {
                    let hp_frac = cur as f32 / max.max(1) as f32;
                    let bar = egui::ProgressBar::new(hp_frac)
                        .text(format!("HP: {}/{}", cur, max))
                        .fill(if hp_frac > 0.5 {
                            Color32::DARK_GREEN
                        } else if hp_frac > 0.25 {
                            Color32::from_rgb(200, 150, 0)
                        } else {
                            Color32::DARK_RED
                        });
                    ui.add(bar);
                }

                let a = &char.abilities;
                if a.strength > 0 {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(format!("STR {}", a.strength));
                        ui.label(format!("DEX {}", a.dexterity));
                        ui.label(format!("CON {}", a.constitution));
                        ui.label(format!("INT {}", a.intelligence));
                        ui.label(format!("WIS {}", a.wisdom));
                        ui.label(format!("CHA {}", a.charisma));
                    });
                }

                if !char.equipment.is_empty() {
                    ui.collapsing("Equipment", |ui| {
                        for eq in &char.equipment {
                            ui.label(format!("{:?}: {}", eq.slot, eq.item_name));
                        }
                    });
                }
            });
        }
    }
}

fn load_party_from_save(path: &std::path::Path) -> Result<PartyData, String> {
    let (mut reader, package) = lsv::open_package(path)?;
    let resource = lsf::load_globals(&mut reader, &package)?;
    party::extract_party(&resource)
}
