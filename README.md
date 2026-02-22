# Perfect Run

BG3 save file analyzer with an in-game overlay and a Script Extender mod for blocking storylines.

## Features

- **Save parser** — Extract party data, character stats, equipment from BG3 save files
- **CLI tool** — Scan saves, export party data as JSON/markdown, search unpacked game files
- **Overlay** — Real-time egui overlay showing party info, with auto-reload on save changes
- **Storyline toggle** — Disable entire storylines (Guardian/Emperor dreams, companion quests, etc.) via overlay toggles that control a BG3 Script Extender mod

## Architecture

```mermaid
%%{init: {'theme': 'neutral'}}%%
flowchart TB
    player(["Player\n(BG3 multiplayer host)"])

    subgraph perfect_run ["Perfect Run"]
        overlay["Overlay\n(Rust / egui)"]
        cli["CLI\n(Rust / clap)"]
        lib["bg3-save\n(Rust library)"]
        mod["BG3SE Mod\n(Lua)"]
    end

    bg3[/"Baldur's Gate 3\n(Osiris engine)"\]
    bg3se["Script Extender"]
    saves[("Save Files\n(.lsv in AppData)")]

    player -- "toggles storylines,\nviews party" --> overlay
    player -- "scans saves,\nsearches game files" --> cli
    overlay --> lib
    cli --> lib
    lib -- "reads/parses" --> saves
    overlay -- "config.json" --> mod
    mod -- "status.json" --> overlay
    bg3se -- "loads" --> mod
    mod -- "blocks events" --> bg3
```

```mermaid
%%{init: {'theme': 'neutral'}}%%
flowchart TB
    overlay["bg3-overlay\n(Rust binary)\negui overlay with\nParty + Storylines tabs"]
    cli["bg3-cli\n(Rust binary)\nSave analysis + game file search"]
    lib["bg3-save\n(Rust library)\nModels, LSV/LSF parsing,\nstoryline defs, IPC"]

    subgraph mod_boundary ["BG3SE Mod (Lua)"]
        bootstrap["BootstrapServer\nEntry point, starts polling"]
        config_reader["ConfigReader\nReads config.json every 2s"]
        blocker["StorylineBlocker\nOsiris listeners for\nflags/dialogs/quests"]
        event_log["EventLog\nLogs blocked events"]
    end

    config[("config.json\nDisabled storyline IDs")]
    status[("status.json\nMod status + blocked log")]
    toml[("storylines.toml\nStoryline definitions")]

    overlay --> lib
    cli --> lib
    overlay -- "include_str!" --> toml
    overlay -- "writes" --> config
    overlay -- "reads" --> status
    config_reader -- "reads" --> config
    event_log -- "writes" --> status
    bootstrap --> config_reader
    bootstrap --> blocker
    config_reader -- "disabled list" --> blocker
    blocker -- "logs" --> event_log
```

```mermaid
%%{init: {'theme': 'neutral'}}%%
flowchart TB
    subgraph bg3_save ["bg3-save (Rust library)"]
        models["models.rs\nSaveInfo, PartyData,\nCharacter, AbilityScores,\nEquipmentSlot"]
        scanner["scanner.rs\nSaveScanner, find_saves(),\nparse_save_info_json()"]
        lsv["lsv.rs\nopen_package(),\nlist_files()"]
        lsf["lsf.rs\nload_lsf(), load_globals(),\nfind_nodes_by_path(),\nattribute getters"]
        party["party.rs\nextract_party(),\ncharacters, abilities,\nHP, equipment, gold"]
        export["export.rs\nto_json(), to_markdown()"]
        storylines["storylines.rs\nStorylineDefinition,\nStoryHook, StorylineConfig,\nload from TOML"]
        ipc["ipc.rs\nipc_dir(), ModConfig,\nModStatus, BlockedEvent,\nread/write helpers"]
    end

    subgraph bg3_overlay ["bg3-overlay (Rust binary)"]
        app["app.rs\nOverlayApp, tab system,\nparty view, file watcher"]
        storylines_ui["storylines_ui.rs\nStorylinePanel, toggle UI,\nIPC config write,\nstatus polling"]
    end

    subgraph bg3_cli ["bg3-cli (Rust binary)"]
        main_cli["main.rs\nscan, party, export,\ndump, files, extract,\nsearch-flags/dialogs/goals"]
    end

    app --> models
    app --> lsv
    app --> lsf
    app --> party
    storylines_ui --> storylines
    storylines_ui --> ipc
    main_cli --> scanner
    main_cli --> lsv
    main_cli --> lsf
    main_cli --> party
    main_cli --> export
    scanner --> lsv
    party --> lsf
```

```mermaid
%%{init: {'theme': 'neutral'}}%%
flowchart LR
    subgraph ipc_flow ["IPC Data Flow (file-based)"]
        direction TB

        subgraph overlay_side ["Overlay Process"]
            ui["StorylinePanel\n(storylines_ui.rs)"]
        end

        subgraph files ["Filesystem"]
            config_file["config.json\n{\n  disabled_storylines:\n    ['guardian_emperor']\n}"]
            status_file["status.json\n{\n  active: true,\n  blocked_events: [...]\n}"]
        end

        subgraph mod_side ["BG3 Process (Script Extender)"]
            reader["ConfigReader.lua\npolls every 2s"]
            blocker["StorylineBlocker.lua"]
            logger["EventLog.lua"]
        end

        ui -- "writes on toggle" --> config_file
        reader -- "reads" --> config_file
        reader -- "disabled list" --> blocker
        blocker -- "blocked event" --> logger
        logger -- "writes" --> status_file
        ui -- "reads every 3s" --> status_file
    end
```

## Development Setup (Windows 11)

### Prerequisites

1. **Rust** — Install from https://rustup.rs (default options are fine)

2. **Visual Studio Build Tools** — Required for compiling native dependencies (GLFW, zstd)
   - Download from https://visualstudio.microsoft.com/downloads/ (scroll to "Tools for Visual Studio", then "Build Tools for Visual Studio")
   - In the installer, select the **"Desktop development with C++"** workload
   - This installs: MSVC compiler (`cl.exe`), linker (`link.exe`), Windows SDK (`rc.exe`), `nmake`

3. **CMake** — Required for building GLFW
   - Download from https://cmake.org/download/ (Windows x64 installer)
   - During install, select "Add CMake to the system PATH"

4. **Git** — https://git-scm.com/download/win

### Clone and build

```bash
git clone <repo-url> perfect-run
cd perfect-run
build.bat build
```

Or from Git Bash:

```bash
cmd //c "build.bat build"
```

### What `build.bat` does

You **cannot** run `cargo build` directly because Git for Windows ships a `link.exe` (Unix utility) that shadows MSVC's `link.exe`, causing linker failures.

`build.bat` handles this by:
1. Using `vswhere.exe` to find your VS installation (works with any VS version/edition)
2. Calling `vcvarsall.bat x64` to set up the MSVC environment (PATH, LIB, INCLUDE)
3. Prepending the MSVC bin directory to PATH so MSVC's `link.exe` is found first
4. Setting `CMAKE_GENERATOR=NMake Makefiles` (the Visual Studio CMake generator doesn't work with Build Tools-only installs)
5. Running `cargo` with the correct environment

### Build commands

All commands go through `build.bat`:

```bash
build.bat build              # Debug build
build.bat build --release    # Release build
build.bat check              # Type-check without linking
build.bat check -p bg3-save  # Check a single crate
build.bat test               # Run tests
build.bat clippy             # Lint
```

## Project Structure

```
crates/
  bg3-save/       Core library (save parsing, storyline model, IPC types)
  bg3-cli/        CLI tool
  bg3-overlay/    egui in-game overlay
mod/
  PerfectRun/     BG3 Script Extender mod (Lua)
storylines.toml   Storyline definitions
build.bat         Build wrapper (sets up MSVC environment)
```

## Usage

### CLI

```bash
# List all saves
build.bat run -p bg3-cli -- scan

# Show party details for a save
build.bat run -p bg3-cli -- party path/to/save.lsv

# Export as markdown
build.bat run -p bg3-cli -- export path/to/save.lsv --markdown

# Search unpacked game files for flag GUIDs
build.bat run -p bg3-cli -- search-flags guardian --dir path/to/unpacked/Gustav
```

### Overlay

```bash
# Run with auto-detected most recent save
build.bat run -p bg3-overlay

# Run with a specific save
build.bat run -p bg3-overlay -- path/to/save.lsv
```

### Storyline Mod

The overlay's **Storylines** tab writes a config file that the BG3 Script Extender mod reads to block storyline events in real time.

1. Install [BG3 Script Extender](https://github.com/Norbyte/bg3se)
2. Copy `mod/PerfectRun/` to your BG3 mods directory
3. Enable the mod in your mod manager
4. Run the overlay and toggle storylines in the **Storylines** tab

Storyline definitions are in `storylines.toml`. Flag GUIDs need to be discovered from unpacked game files — use the CLI search commands to find them.

## Unpacking Game Files

To discover flag GUIDs and dialog names for `storylines.toml`:

1. Download [LSLib](https://github.com/Norbyte/lslib/releases) (Norbyte's tools)
2. Use `ConverterApp.exe` to unpack `Gustav.pak`
3. Use LSLib's story tools to decompile `story.div.osi`
4. Use the CLI search commands:
   ```bash
   build.bat run -p bg3-cli -- search-flags guardian --dir path/to/unpacked
   build.bat run -p bg3-cli -- search-dialogs dream --dir path/to/unpacked
   build.bat run -p bg3-cli -- search-goals emperor --dir path/to/decompiled
   ```
