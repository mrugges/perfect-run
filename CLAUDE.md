# Perfect Run - BG3 Save Analyzer & Storyline Toggle Mod

## Build

**Must use `build.bat`** — it calls `vcvarsall.bat` to set up the MSVC toolchain, then runs `cargo`. Running `cargo build` directly will fail because Git's `link.exe` shadows MSVC's.

```bash
# From Git Bash
cmd //c "build.bat build"
cmd //c "build.bat check"
cmd //c "build.bat build --release"
cmd //c "build.bat check -p bg3-save"
```

### build.bat details

- Uses `vswhere.exe` to find the VS installation (works across versions/editions)
- Calls `vcvarsall.bat x64` for PATH, LIB, INCLUDE
- Re-prepends MSVC bin dir to PATH so `link.exe` resolves to MSVC's, not Git's
- Sets `CMAKE_GENERATOR=NMake Makefiles` (the VS generator auto-detect is broken with Build Tools-only installs)

### Known gotcha: Git `link.exe` shadowing

Git for Windows ships `/usr/bin/link.exe` (a Unix utility) which is on PATH and shadows MSVC's `link.exe`. This causes `cargo build` to fail with "extra operand" errors. `build.bat` handles this by prepending the MSVC bin dir.

## Project Structure

```
crates/
  bg3-save/       Core library: save parsing, storyline model, IPC types
  bg3-cli/        CLI: scan, party, export, dump, search-flags/dialogs/goals
  bg3-overlay/    egui overlay: party view + storylines toggle tab
mod/
  PerfectRun/     BG3 Script Extender mod (Lua): reads config, blocks events
storylines.toml   Storyline definitions (TOML, embedded in overlay at compile time)
```

## Testing

```bash
build.bat test -p bg3-save    # Fast: library tests only (~0.3s, 27 tests incl. proptest)
build.bat test                # Full: all crates
```

A pre-push git hook runs `test -p bg3-save` automatically. Bypass with `git push --no-verify`.

CI runs on GitHub Actions: library tests on Ubuntu (fast), full build + clippy on Windows.

## Key Architecture Decisions

- **egui version**: The overlay uses `egui_overlay`'s re-exported egui (v0.22), NOT a separate `egui` crate. All overlay code must `use egui_overlay::egui;`.
- **`Frame::none()`** not `Frame::NONE` (v0.22 API).
- **IPC**: Overlay writes `config.json`, Lua mod writes `status.json`, both in `%LOCALAPPDATA%\Larian Studios\Baldur's Gate 3\Script Extender\perfect-run\`.
- **Storyline definitions**: Loaded from `storylines.toml` next to the exe, or falls back to the embedded default compiled via `include_str!`.
