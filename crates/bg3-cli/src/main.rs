use bg3_save::{export, lsf, lsv, party, SaveScanner};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "bg3-cli", about = "Baldur's Gate 3 save file explorer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all saves with character names
    Scan {
        /// Custom save directory (default: standard BG3 location)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Show full party details for a save
    Party {
        /// Path to an .lsv save file
        save: PathBuf,
    },
    /// Export party data as JSON
    Export {
        /// Path to an .lsv save file
        save: PathBuf,
        /// Output as markdown instead of JSON
        #[arg(long)]
        markdown: bool,
    },
    /// Dump raw LSF tree structure for exploration
    Dump {
        /// Path to an .lsv save file
        save: PathBuf,
        /// Which .lsf file to dump (default: meta.lsf)
        #[arg(short, long, default_value = "meta.lsf")]
        file: String,
        /// Maximum tree depth to display
        #[arg(short, long, default_value_t = 5)]
        depth: usize,
    },
    /// List files contained in an LSV package
    Files {
        /// Path to an .lsv save file
        save: PathBuf,
    },
    /// Extract a file from an LSV package and print its contents
    Extract {
        /// Path to an .lsv save file
        save: PathBuf,
        /// File to extract (e.g. SaveInfo.json)
        #[arg(short, long)]
        file: String,
    },
    /// Search unpacked flag definition files for a keyword
    SearchFlags {
        /// Keyword to search for (case-insensitive)
        pattern: String,
        /// Directory containing unpacked game files (e.g. Gustav/Public)
        #[arg(short, long)]
        dir: PathBuf,
    },
    /// Search unpacked dialog files for a keyword
    SearchDialogs {
        /// Keyword to search for (case-insensitive)
        pattern: String,
        /// Directory containing unpacked game files
        #[arg(short, long)]
        dir: PathBuf,
    },
    /// Search decompiled Osiris goals for a keyword
    SearchGoals {
        /// Keyword to search for (case-insensitive)
        pattern: String,
        /// Directory containing decompiled story goals
        #[arg(short, long)]
        dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { path } => cmd_scan(path),
        Commands::Party { save } => cmd_party(&save),
        Commands::Export { save, markdown } => cmd_export(&save, markdown),
        Commands::Dump { save, file, depth } => cmd_dump(&save, &file, depth),
        Commands::Files { save } => cmd_files(&save),
        Commands::Extract { save, file } => cmd_extract(&save, &file),
        Commands::SearchFlags { pattern, dir } => cmd_search_files(&dir, &pattern, &["Flags"], &["lsx", "lsf.lsx"]),
        Commands::SearchDialogs { pattern, dir } => cmd_search_files(&dir, &pattern, &["Dialogs", "Dialog"], &["lsj", "lsx"]),
        Commands::SearchGoals { pattern, dir } => cmd_search_files(&dir, &pattern, &["Goals", "Story"], &["txt", "div", "goal"]),
    }
}

fn cmd_scan(path: Option<PathBuf>) {
    let scanner = match path {
        Some(p) => SaveScanner::new(p),
        None => SaveScanner::default(),
    };

    let saves = scanner.find_saves();
    println!("Found {} save files\n", saves.len());
    println!(
        "{:<60} {:<20} {:<15} SAVE NAME",
        "PATH", "CHARACTER", "TYPE"
    );
    println!("{}", "-".repeat(110));

    for save_path in &saves {
        match scanner.scan_save(save_path) {
            Ok(info) => {
                // Shorten path for display
                let display_path = save_path
                    .file_name()
                    .and_then(|f| save_path.parent().and_then(|p| p.file_name()).map(|d| {
                        format!("{}/{}", d.to_string_lossy(), f.to_string_lossy())
                    }))
                    .unwrap_or_else(|| save_path.to_string_lossy().to_string());

                let mode = if info.is_honour_mode {
                    "Honour"
                } else {
                    "Normal"
                };

                println!(
                    "{:<60} {:<20} {:<15} {}",
                    display_path, info.character_name, mode, info.save_name
                );
            }
            Err(e) => {
                let display_path = save_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| save_path.to_string_lossy().to_string());
                println!("{:<60} ERROR: {}", display_path, e);
            }
        }
    }
}

fn cmd_party(save: &Path) {
    let (mut reader, package) = match lsv::open_package(save) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open save: {}", e);
            return;
        }
    };

    // Primary: extract from SaveInfo.json (reliable, always works)
    let data = match extract_party_from_save_info(&mut reader, &package) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to extract party from SaveInfo.json: {}", e);
            // Fallback: try Globals.lsf
            match lsf::load_globals(&mut reader, &package) {
                Ok(resource) => party::extract_party(&resource),
                Err(e2) => {
                    eprintln!("Also failed Globals.lsf: {}", e2);
                    return;
                }
            }
        }
    };

    println!("=== Party Data ===\n");
    if let Some(loc) = &data.location {
        println!("Location: {}", loc);
    }
    if let Some(gold) = data.gold {
        println!("Gold: {}", gold);
    }
    if let Some(day) = data.day {
        println!("Day: {}", day);
    }
    println!("\n--- Characters ({}) ---\n", data.characters.len());
    for ch in &data.characters {
        println!(
            "  {} - Lvl {} {} {} {}",
            ch.name,
            ch.level,
            ch.race,
            ch.class,
            if ch.is_player { "(Player)" } else { "" }
        );
        let a = &ch.abilities;
        if a.strength > 0 {
            println!(
                "    STR:{} DEX:{} CON:{} INT:{} WIS:{} CHA:{}",
                a.strength, a.dexterity, a.constitution, a.intelligence, a.wisdom, a.charisma
            );
        }
        if let Some((cur, max)) = ch.hp {
            println!("    HP: {}/{}", cur, max);
        }
        if !ch.equipment.is_empty() {
            println!("    Equipment:");
            for eq in &ch.equipment {
                println!("      {:?}: {}", eq.slot, eq.item_name);
            }
        }
        println!();
    }

    if data.characters.is_empty() {
        println!("  No characters found.");
    }
}

fn extract_party_from_save_info(
    reader: &mut bg3_save::bg3_lib::package_reader::PackageReader,
    package: &bg3_save::bg3_lib::package::Package,
) -> Result<bg3_save::PartyData, bg3_save::Error> {
    let pfi = package
        .files
        .iter()
        .find(|f| {
            f.name
                .to_string_lossy()
                .to_lowercase()
                .contains("saveinfo.json")
        })
        .ok_or_else(|| bg3_save::Error::FileNotFound("SaveInfo.json".into()))?;

    let data = reader.decompress_file(pfi).map_err(bg3_save::Error::Package)?;
    let text = String::from_utf8(data)?;
    bg3_save::scanner::parse_save_info_json(&text)
}

fn cmd_export(save: &Path, markdown: bool) {
    let scanner = SaveScanner::default();
    let save_info = match scanner.scan_save(save) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to scan save: {}", e);
            return;
        }
    };

    let (mut reader, package) = match lsv::open_package(save) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open save: {}", e);
            return;
        }
    };

    let data = match extract_party_from_save_info(&mut reader, &package) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to extract party: {}", e);
            return;
        }
    };

    if markdown {
        println!("{}", export::to_markdown(&save_info, &data));
    } else {
        match export::to_json(&data) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON serialization error: {}", e),
        }
    }
}

fn cmd_dump(save: &Path, file: &str, depth: usize) {
    let (mut reader, package) = match lsv::open_package(save) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open save: {}", e);
            return;
        }
    };

    let resource = match lsf::load_lsf(&mut reader, &package, file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to load {}: {}", file, e);
            eprintln!("\nAvailable files in this save:");
            for f in lsv::list_files(&package) {
                eprintln!("  {}", f);
            }
            return;
        }
    };

    println!("=== {} tree (depth {}) ===\n", file, depth);
    println!(
        "Regions: {} | Total nodes: {}\n",
        resource.regions.region_count(),
        resource.regions.node_instances.len()
    );
    print!("{}", lsf::dump_tree(&resource.regions, depth));
}

fn cmd_extract(save: &Path, file: &str) {
    let (mut reader, package) = match lsv::open_package(save) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open save: {}", e);
            return;
        }
    };

    let pfi = match package
        .files
        .iter()
        .find(|f| f.name.to_string_lossy().to_lowercase().contains(&file.to_lowercase()))
    {
        Some(f) => f,
        None => {
            eprintln!("File '{}' not found. Available:", file);
            for f in lsv::list_files(&package) {
                eprintln!("  {}", f);
            }
            return;
        }
    };

    match reader.decompress_file(pfi) {
        Ok(data) => {
            // Try to print as UTF-8 text, fall back to hex dump
            match String::from_utf8(data.clone()) {
                Ok(text) => println!("{}", text),
                Err(_) => {
                    println!("Binary data ({} bytes), first 256 bytes:", data.len());
                    for (i, byte) in data.iter().take(256).enumerate() {
                        if i % 16 == 0 && i > 0 {
                            println!();
                        }
                        print!("{:02x} ", byte);
                    }
                    println!();
                }
            }
        }
        Err(e) => eprintln!("Failed to extract: {}", e),
    }
}

fn cmd_files(save: &Path) {
    let (_reader, package) = match lsv::open_package(save) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to open save: {}", e);
            return;
        }
    };

    let files = lsv::list_files(&package);
    println!("Files in save ({}):\n", files.len());
    for f in &files {
        println!("  {}", f);
    }
}

fn cmd_search_files(dir: &Path, pattern: &str, subdir_hints: &[&str], extensions: &[&str]) {
    if !dir.exists() {
        eprintln!("Directory does not exist: {}", dir.display());
        eprintln!();
        eprintln!("To unpack BG3 game files:");
        eprintln!("  1. Download LSLib from https://github.com/Norbyte/lslib/releases");
        eprintln!("  2. Use ConverterApp.exe to unpack Gustav.pak");
        eprintln!("  3. Point --dir at the unpacked directory");
        return;
    }

    let pattern_lower = pattern.to_lowercase();
    let mut match_count = 0;

    println!("Searching for \"{}\" in {} ...\n", pattern, dir.display());

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Check file extension matches
        let ext_match = extensions.iter().any(|ext| {
            path.extension()
                .is_some_and(|e| e.to_string_lossy().to_lowercase() == *ext)
        });
        if !ext_match {
            continue;
        }

        // Optionally prioritize files in hint subdirectories, but search all
        let path_str = path.to_string_lossy().to_lowercase();
        let in_hint_dir = subdir_hints
            .iter()
            .any(|hint| path_str.contains(&hint.to_lowercase()));

        // Read the file and search for the pattern
        if let Ok(content) = std::fs::read_to_string(path) {
            let content_lower = content.to_lowercase();
            if content_lower.contains(&pattern_lower) {
                let priority = if in_hint_dir { "*" } else { " " };

                // Find matching lines
                for (line_num, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pattern_lower) {
                        println!(
                            "{} {}:{}: {}",
                            priority,
                            path.display(),
                            line_num + 1,
                            line.trim()
                        );
                        match_count += 1;

                        if match_count > 500 {
                            println!("\n... truncated (>500 matches). Refine your search pattern.");
                            return;
                        }
                    }
                }
            }
        }
    }

    if match_count == 0 {
        println!("No matches found.");
        println!("Searched extensions: {:?}", extensions);
        println!("Hint subdirectories: {:?}", subdir_hints);
    } else {
        println!("\n{} matching lines found.", match_count);
        println!("Lines prefixed with * are in priority directories ({:?}).", subdir_hints);
    }
}
