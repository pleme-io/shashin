# Shashin (写真) — Screenshot Tool

## Build & Test

```bash
cargo build                    # compile
cargo test --lib               # unit tests
cargo test                     # all tests
cargo run -- fullscreen        # capture fullscreen
cargo run -- region            # capture region
cargo run -- window            # capture window
cargo run -- annotate <file>   # annotate existing screenshot
```

## Architecture

### Pipeline

```
Capture Request → Platform Backend → CapturedImage (RGBA)
                                          ↓
                           Save to disk (png/jpg/webp)
                                          ↓
                    Optional: Copy to clipboard + Annotation UI
```

### Configuration

Uses **shikumi** for config discovery and hot-reload:
- Config file: `~/.config/shashin/shashin.yaml`
- Env override: `$SHASHIN_CONFIG`
- Env vars: `SHASHIN_` prefix (e.g. `SHASHIN_CAPTURE__DEFAULT_MODE=fullscreen`)
- Hot-reload on file change (nix-darwin symlink aware)

### Platform Isolation (`src/platform/`)

| Trait | macOS Impl | Purpose |
|-------|------------|---------|
| `ScreenCapture` | `MacOSScreenCapture` | Fullscreen, region, window capture |

Linux implementations will be added under `src/platform/linux/`.

### Config Struct (`src/config.rs`)

| Section | Fields |
|---------|--------|
| `capture` | `default_mode`, `delay_ms`, `include_cursor` |
| `output` | `save_dir`, `format`, `quality`, `filename_template` |
| `annotation` | `enabled`, `default_color`, `line_width`, `font_size` |
| `hotkeys` | `fullscreen`, `region`, `window` |
| `clipboard` | `auto_copy`, `auto_clear_secs` |

## File Map

| Path | Purpose |
|------|---------|
| `src/config.rs` | Config struct (uses shikumi) |
| `src/platform/mod.rs` | Platform trait definitions + `ScreenCapture` |
| `src/platform/macos/mod.rs` | macOS screen capture backend |
| `src/main.rs` | CLI entry point (clap subcommands) |
| `src/lib.rs` | Library root (re-exports config + platform) |
| `module/default.nix` | HM module with typed options + YAML generation |
| `flake.nix` | Nix flake (packages, overlay, HM module, devShell) |

## Design Decisions

### Configuration Language: YAML
- YAML is the primary and only configuration format
- Config file: `~/.config/shashin/shashin.yaml`
- Nix HM module generates YAML via `lib.generators.toYAML` from typed options
- `extraSettings` escape hatch for raw attrset merge

### Nix Integration
- Flake exports: `packages`, `overlays.default`, `homeManagerModules.default`, `devShells`
- HM module at `blackmatter.components.shashin` with fully typed options
- YAML generated via `lib.generators.toYAML`
- Cross-platform: `mkLaunchdService` (macOS) + `mkSystemdService` (Linux)
- Uses substrate's `hm-service-helpers.nix` for service generation

### Cross-Platform Strategy
- Platform-specific capture: behind `ScreenCapture` trait in `src/platform/`
- macOS: ScreenCaptureKit / CGWindowList APIs
- Linux: (planned) PipeWire / X11 / Wayland screenshot portals
- Image processing: `image` crate (cross-platform)
