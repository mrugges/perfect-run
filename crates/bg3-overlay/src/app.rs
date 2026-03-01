use crate::storylines_ui::StorylinePanel;
use bg3_save::models::{Character, PartyData, SaveInfo};
use bg3_save::{lsf, lsv, party, scanner, SaveScanner};
use egui_overlay::egui;
use egui::Color32;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
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
    save_info: Option<SaveInfo>,
    last_load: Option<Instant>,
    error: Option<String>,
    expanded_chars: Vec<bool>,
    visible: bool,
    watcher_rx: Option<mpsc::Receiver<()>>,
    _watcher: Option<RecommendedWatcher>,
    active_tab: Tab,
    storyline_panel: StorylinePanel,
    // Save selector state
    available_saves: Vec<SaveInfo>,
    show_save_picker: bool,
    should_exit: bool,
}

impl OverlayApp {
    pub fn new(save_path: Option<PathBuf>) -> Self {
        let scanner = SaveScanner::default();
        let available_saves = scan_all_saves(&scanner);

        let mut app = Self {
            party: None,
            save_path: None,
            save_info: None,
            last_load: None,
            error: None,
            expanded_chars: Vec::new(),
            visible: true,
            watcher_rx: None,
            _watcher: None,
            active_tab: Tab::Party,
            storyline_panel: StorylinePanel::new(),
            available_saves,
            show_save_picker: false,
            should_exit: false,
        };

        if let Some(path) = save_path {
            app.load_save(path);
        } else {
            // Auto-detect most recent save
            app.load_most_recent();
        }

        app
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn setup_watcher(&mut self) {
        if let Some(save_path) = &self.save_path {
            let watch_dir = save_path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(save_path.as_path())
                .to_path_buf();

            let (tx, rx) = mpsc::channel();

            match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let _ = tx.send(());
                    }
                }
            }) {
                Ok(mut watcher) => {
                    if watcher.watch(&watch_dir, RecursiveMode::Recursive).is_ok() {
                        self.watcher_rx = Some(rx);
                        self._watcher = Some(watcher);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create file watcher: {}", e);
                }
            }
        }
    }

    fn load_most_recent(&mut self) {
        if let Some(most_recent) = self
            .available_saves
            .iter()
            .max_by_key(|s| s.timestamp)
        {
            let path = most_recent.path.clone();
            self.load_save(path);
        } else {
            self.error = Some("No save files found".to_string());
        }
    }

    fn load_save(&mut self, path: PathBuf) {
        // Scan save info for display
        let scanner = SaveScanner::default();
        self.save_info = scanner.scan_save(&path).ok();

        match load_party_from_save(&path) {
            Ok(data) => {
                self.expanded_chars = vec![false; data.characters.len()];
                self.party = Some(data);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
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
                    if ui.small_button("Saves").clicked() {
                        // Refresh save list when opening picker
                        let scanner = SaveScanner::default();
                        self.available_saves = scan_all_saves(&scanner);
                        self.show_save_picker = !self.show_save_picker;
                    }
                    if ui
                        .small_button("X")
                        .on_hover_text("Exit overlay")
                        .clicked()
                    {
                        self.should_exit = true;
                    }
                });

                // Current save info
                if let Some(info) = &self.save_info {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} - {}{}",
                            info.character_name,
                            info.save_name,
                            if info.is_honour_mode { " [Honour]" } else { "" }
                        ))
                        .small()
                        .color(Color32::GRAY),
                    );
                }

                // Save picker
                if self.show_save_picker {
                    ui.separator();
                    ui.label(egui::RichText::new("Select Save").strong());

                    let mut selected_path = None;
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for save in &self.available_saves {
                                let is_current = self
                                    .save_path
                                    .as_ref()
                                    .is_some_and(|p| *p == save.path);

                                let label = format!(
                                    "{} - {}{}",
                                    save.character_name,
                                    save.save_name,
                                    if save.is_honour_mode {
                                        " [H]"
                                    } else {
                                        ""
                                    }
                                );

                                if ui.selectable_label(is_current, &label).clicked() {
                                    selected_path = Some(save.path.clone());
                                }
                            }
                        });

                    if let Some(path) = selected_path {
                        self.load_save(path);
                        self.show_save_picker = false;
                    }
                }

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
                                    ui.label(format!("Location: {}", loc));
                                }
                                if let Some(gold) = party.gold {
                                    ui.label(format!("Gold: {}", gold));
                                }
                                if let Some(day) = party.day {
                                    ui.label(format!("Day {}", day));
                                }
                            });

                            ui.separator();

                            // Character list
                            for i in 0..party.characters.len() {
                                let ch = &party.characters[i];
                                let header = format_character_header(ch);
                                let expanded =
                                    self.expanded_chars.get(i).copied().unwrap_or(false);
                                let response = ui.selectable_label(expanded, &header);
                                if response.clicked() {
                                    if let Some(exp) = self.expanded_chars.get_mut(i) {
                                        *exp = !*exp;
                                    }
                                }
                                if expanded {
                                    Self::render_character_details(ui, ch, i);
                                }
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

    fn render_character_details(ui: &mut egui::Ui, ch: &Character, idx: usize) {
        ui.indent(egui::Id::new(format!("char_{}", idx)), |ui| {
            ui.label(format!("{} - {}", ch.race, ch.class));

            if let Some((cur, max)) = ch.hp {
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

            let a = &ch.abilities;
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

            if !ch.equipment.is_empty() {
                ui.collapsing("Equipment", |ui| {
                    for eq in &ch.equipment {
                        ui.label(format!("{:?}: {}", eq.slot, eq.item_name));
                    }
                });
            }
        });
    }
}

/// Format character header line with name for companions, race for custom chars.
fn format_character_header(ch: &Character) -> String {
    let name_display = friendly_name(&ch.name);
    format!(
        "{} - Lvl {} {}{}",
        name_display,
        ch.level,
        ch.class,
        if ch.is_player { " *" } else { "" }
    )
}

/// Convert internal origin/race names to friendly display names.
fn friendly_name(name: &str) -> &str {
    // Known companion origins
    match name {
        "Shadowheart" | "Astarion" | "Gale" | "Karlach" | "Wyll" | "Halsin" | "Minthara"
        | "Jaheira" | "Minsc" => name,
        // Custom characters show as race — clean up internal race names
        n if n.contains('_') => {
            // "Gnome_Deep" -> "Deep Gnome", "Elf_WoodElf" -> "Wood Elf", etc.
            n
        }
        _ => name,
    }
}

/// Scan all saves sorted by most recent first.
fn scan_all_saves(scanner: &SaveScanner) -> Vec<SaveInfo> {
    let save_paths = scanner.find_saves();
    let mut saves: Vec<SaveInfo> = save_paths
        .iter()
        .filter_map(|p| scanner.scan_save(p).ok())
        .collect();
    saves.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    saves
}

fn load_party_from_save(path: &std::path::Path) -> Result<PartyData, bg3_save::Error> {
    let (mut reader, package) = lsv::open_package(path)?;

    // Primary: extract from SaveInfo.json (reliable, handles all save versions)
    let save_info_result = package
        .files
        .iter()
        .find(|f| {
            f.name
                .to_string_lossy()
                .to_lowercase()
                .contains("saveinfo.json")
        })
        .ok_or_else(|| bg3_save::Error::FileNotFound("SaveInfo.json".into()))
        .and_then(|pfi| {
            let data = reader
                .decompress_file(pfi)
                .map_err(bg3_save::Error::Package)?;
            let text = String::from_utf8(data)?;
            scanner::parse_save_info_json(&text)
        });

    match save_info_result {
        Ok(data) => Ok(data),
        Err(_) => {
            // Fallback: try Globals.lsf (may fail on newer save formats)
            let resource = lsf::load_globals(&mut reader, &package)?;
            Ok(party::extract_party(&resource))
        }
    }
}
