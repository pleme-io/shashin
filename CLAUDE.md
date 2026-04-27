# Shashin (写真) -- GPU Image Viewer

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


Binary: `shashin` | Crate: `shashin` | Config: `~/.config/shashin/shashin.yaml`

**NOTE:** Despite the existing code being a screenshot tool, the target vision for
shashin is a **GPU-rendered image viewer** with gallery, metadata, effects, and
optional screenshot capabilities. The current screenshot-tool scaffold will be
refactored into a full image viewer application.

## Build & Test

```bash
cargo build                          # compile
cargo test --lib                     # unit tests
cargo test                           # all tests
cargo run                            # launch GUI viewer
cargo run -- open /path/to/image.jpg # open specific image
cargo run -- gallery /path/to/dir    # open gallery for directory
cargo run -- slideshow /path/to/dir  # start slideshow
nix build                            # Nix build
nix run .#regenerate                 # regenerate Cargo.nix after dep changes
```

## Current State

Minimal screenshot-tool scaffold. Needs fundamental refactoring toward image viewer:

- `config.rs` -- `ShashinConfig` with capture/output/annotation/hotkey/clipboard
  sections. Working shikumi integration. Config struct needs reworking for viewer use case.
- `platform/mod.rs` -- `ScreenCapture` trait + `CapturedImage` type. This trait
  will be repurposed for screenshot/clipboard integration within the viewer.
- `platform/macos/mod.rs` -- Stub `MacOSScreenCapture`. All methods return empty images.
- `main.rs` -- CLI with Fullscreen/Region/Window/Annotate subcommands. Needs
  replacement with viewer-oriented CLI.
- `lib.rs` -- Re-exports config + platform.

**Missing entirely:** GPU rendering, image loading, gallery, metadata, effects,
zoom/pan, keyboard navigation. The app needs to be rebuilt from the viewer perspective.

## Competitive Landscape

| Competitor | Stack | Strengths | Weaknesses vs shashin |
|-----------|-------|-----------|----------------------|
| feh | C/Xlib | Lightweight, scriptable, background setter, montage | X11-only, no Wayland, no GPU, minimal UI |
| imv | C/Wayland | Wayland-native, fast, keyboard-driven, pipe-friendly | Minimal UI, no metadata, no effects, no annotations |
| nsxiv | C/Xlib | Scriptable, thumbnails, vim-like, lightweight | X11-only, C scripts not Rhai, no GPU |
| Loupe | Rust/GTK4 | GNOME default, smooth zoom/pan, metadata, print | GTK dependency, no vim keys, no scripting, no effects |
| Nomacs | C++/Qt | Annotations, batch processing, sync, RAW support | Qt heavy, no GPU shaders, no scripting |
| viu | Rust/terminal | Terminal image viewer, sixel/kitty protocol | Terminal-only, no interactive UI |

**Key differentiators:**
- GPU-rendered via garasu (Metal/Vulkan), not GTK/Qt/X11
- WGSL shader effects pipeline (brightness, contrast, blur, custom user shaders)
- MCP server for AI-assisted image workflows (batch rename, organize, tag)
- Rhai scripting for custom image processing pipelines
- Vim-modal navigation in both viewer and gallery
- Nix-configured, declarative, hot-reloadable

## Architecture

### Data Flow

```
  Image File(s) ──> image crate Decoder ──> RGBA Buffer
                                                |
                                                v
  Gallery Scanner ──> Thumbnail Cache    wgpu Texture Upload
       |                   |                    |
       v                   v                    v
  Metadata Index    Thumbnail Grid       Viewer Pipeline
  (EXIF/XMP)       (egaku ListView)     (zoom/pan/rotate)
                                                |
                                                v
                                        Shader Effects
                                        (WGSL pipeline)
                                                |
                                                v
                                        winit Window
                                        (Metal/Vulkan)
```

### Module Map

| Module | Responsibility | Key Types | pleme-io Deps |
|--------|---------------|-----------|---------------|
| `viewer` | Image display: texture upload, zoom, pan, rotate, fit modes | `ImageViewer`, `ViewState`, `FitMode` | garasu, madori |
| `gallery` | Thumbnail grid, directory scanning, sorting, filtering | `Gallery`, `ThumbnailCache`, `SortOrder` | egaku |
| `metadata` | EXIF/XMP parsing, display overlay, GPS coordinates | `ImageMetadata`, `ExifData`, `GpsCoord` | -- |
| `effects` | WGSL shader pipeline for image adjustments | `EffectPipeline`, `Effect`, `ShaderEffect` | garasu |
| `platform` | Screen capture trait (screenshot/clipboard integration) | `ScreenCapture`, `CapturedImage` | -- |
| `config` | Config struct, shikumi integration | `ShashinConfig` | shikumi |
| `mcp` | (planned) MCP server for automation | -- | kaname |
| `plugin` | (planned) Rhai scripting engine | -- | soushi |

### Planned Source Layout

```
src/
  main.rs             # CLI entry point (clap)
  config.rs           # ShashinConfig + shikumi
  lib.rs              # Library root
  viewer/
    mod.rs            # ImageViewer: texture management, view state
    zoom.rs           # Zoom/pan math, fit modes (fit, fill, 1:1, custom)
    rotate.rs         # Rotation + flip transforms
    navigation.rs     # Next/prev image, directory walking
  gallery/
    mod.rs            # Gallery: thumbnail grid, async scanning
    thumbnail.rs      # Thumbnail generation + disk cache
    sort.rs           # Sort by name, date, size, type
    filter.rs         # Filter by format, date range, tags
  metadata/
    mod.rs            # Metadata extraction orchestrator
    exif.rs           # EXIF tag parsing (kamadak-exif or rexiv2)
    xmp.rs            # XMP sidecar parsing
    display.rs        # Metadata overlay rendering
  effects/
    mod.rs            # Effect pipeline orchestrator
    builtin.rs        # Built-in effects (brightness, contrast, etc.)
    shader.rs         # Custom WGSL shader loading
  platform/
    mod.rs            # ScreenCapture trait
    macos/mod.rs      # macOS capture backend
  render/
    mod.rs            # Render orchestration (madori RenderCallback)
    layout.rs         # UI layout (viewer, gallery, status bar)
  mcp.rs              # MCP server (kaname)
  plugin.rs           # Rhai scripting (soushi)
```

## pleme-io Library Integration

| Library | Role in shashin |
|---------|----------------|
| **shikumi** | Config discovery + hot-reload for `ShashinConfig` |
| **garasu** | GPU context, texture upload, shader pipeline for effects |
| **madori** | App framework: event loop, render loop, input dispatch |
| **egaku** | Widgets: thumbnail grid, status bar, metadata overlay, modals |
| **irodzuki** | Base16 theme to GPU uniforms for UI consistency |
| **mojiban** | Rich text for metadata display, filename rendering |
| **kaname** | MCP server scaffold for automation tools |
| **soushi** | Rhai scripting for batch processing and custom pipelines |
| **awase** | Hotkey registration and parsing (global screenshot hotkeys) |
| **hasami** | Clipboard for copy-to-clipboard, paste-from-clipboard |
| **tsuuchi** | Notifications for screenshot saved, batch complete |

Libraries NOT used:
- **oto** -- no audio in an image viewer
- **tsunagu** -- no daemon mode needed (hotkey listener runs in-process)
- **todoku** -- no HTTP API calls

## Implementation Phases

### Phase 1: Image Viewer Core
Build the GPU image viewer, replacing the screenshot-tool scaffold:
1. Refactor CLI: `shashin [open <path>] [gallery <dir>] [slideshow <dir>]`
2. Refactor `ShashinConfig` for viewer use case (viewer, gallery, effects sections)
3. Image loading via `image` crate (JPEG, PNG, WebP, BMP, TIFF, GIF, ICO)
4. wgpu texture upload via garasu
5. madori app shell with `RenderCallback` for viewer
6. Zoom: scroll wheel, `+`/`-` keys, pinch gesture
7. Pan: click-drag, `hjkl` with shift
8. Fit modes: fit-to-window, fill, actual-size, custom percentage
9. Rotation: 90/180/270 degree rotation, horizontal/vertical flip
10. Navigation: next/prev image in directory, wrapping

### Phase 2: Gallery View
Thumbnail grid for browsing directories:
1. Async directory scanner (tokio::fs)
2. Thumbnail generation (downscale to 256x256 or configurable)
3. Thumbnail disk cache (`~/.cache/shashin/thumbnails/` with content-hash keys)
4. egaku grid layout with keyboard navigation (hjkl)
5. Sort by name, date modified, file size, image dimensions
6. Filter by format, date range
7. Transition: Enter opens image in viewer, Esc returns to gallery

### Phase 3: Metadata Display
EXIF/XMP metadata extraction and overlay:
1. EXIF parsing via `kamadak-exif` crate
2. Fields: camera model, lens, focal length, aperture, shutter, ISO, date, GPS
3. XMP sidecar support (`.xmp` files alongside images)
4. Info overlay panel (toggle with `i`): semi-transparent panel over image
5. GPS: display coordinates, optional link to map (open in browser)
6. File info: dimensions, color depth, file size, format

### Phase 4: Shader Effects Pipeline
GPU-based image adjustments via WGSL:
1. Built-in effects: brightness, contrast, saturation, hue shift
2. Built-in filters: blur (Gaussian), sharpen (unsharp mask), grayscale, sepia, invert
3. Effect stack: multiple effects applied in order
4. Non-destructive: effects applied at render time, original image unchanged
5. Custom shaders: load from `~/.config/shashin/shaders/*.wgsl`
6. Shader uniforms: time, resolution, mouse position, effect-specific parameters

### Phase 5: Screenshot Integration
Reintegrate capture functionality as a secondary feature:
1. Retain `ScreenCapture` trait and platform backends
2. Global hotkeys via awase for capture triggers
3. Capture modes: fullscreen, region (interactive selection), window
4. Captured images open directly in the viewer for annotation
5. Annotation mode: draw arrows, rectangles, text, blur regions
6. Copy to clipboard via hasami, save to configured directory

### Phase 6: MCP Server
Embedded MCP server via kaname (stdio transport):
1. Standard tools: `status`, `config_get`, `config_set`, `version`
2. Viewer tools: `open`, `next`, `prev`, `zoom`, `rotate`, `fit`, `get_metadata`
3. Gallery tools: `list_gallery`, `sort`, `filter`, `select`
4. Effect tools: `apply_effect`, `clear_effects`, `list_effects`
5. Capture tools: `screenshot`, `copy_to_clipboard`
6. Batch tools: `batch_resize`, `batch_convert`, `batch_rename`

### Phase 7: Plugin System
Rhai scripting via soushi:
1. Script loading from `~/.config/shashin/scripts/*.rhai`
2. Rhai API: `shashin.open(path)`, `shashin.next()`, `shashin.prev()`,
   `shashin.zoom(level)`, `shashin.rotate(degrees)`, `shashin.fit(mode)`,
   `shashin.metadata()`, `shashin.effect(name, params)`,
   `shashin.slideshow(interval)`, `shashin.gallery(dir)`
3. Batch processing: `shashin.batch(dir, fn)` -- apply function to all images
4. Event hooks: `on_image_open`, `on_gallery_enter`, `on_capture`
5. Custom command registration for command palette

## Hotkey System

Modal keybindings via awase. Modes:

**Viewer mode (default):**
| Key | Action |
|-----|--------|
| `j` / `k` or `n` / `p` | Next / previous image |
| `+` / `-` or scroll | Zoom in / out |
| `0` | Actual size (1:1) |
| `f` | Fit to window |
| `F` | Fill window |
| `r` | Rotate 90 clockwise |
| `R` | Rotate 90 counter-clockwise |
| `h` / `H` | Flip horizontal / vertical |
| `i` | Toggle info overlay |
| `e` | Enter effects mode |
| `g` | Switch to gallery view |
| `Space` | Toggle slideshow |
| `/` | Search in current directory |
| `y` | Copy image to clipboard |
| `:` | Enter command mode |
| `q` | Quit |

**Gallery mode:**
| Key | Action |
|-----|--------|
| `h` / `j` / `k` / `l` | Navigate grid |
| `Enter` | Open selected image in viewer |
| `/` | Search / filter |
| `s` | Cycle sort order |
| `m` | Mark/unmark for batch operations |
| `d` | Delete marked (with confirmation) |
| `Esc` | Exit gallery |

**Effects mode:**
| Key | Action |
|-----|--------|
| `b` | Brightness adjust |
| `c` | Contrast adjust |
| `s` | Saturation adjust |
| `g` | Grayscale toggle |
| `u` | Undo last effect |
| `U` | Clear all effects |
| `Esc` | Exit effects mode |

**Command mode:**
`:open <path>`, `:gallery <dir>`, `:slideshow [interval]`, `:sort <field>`,
`:effect <name> [params]`, `:export <path>`, `:resize <WxH>`, `:quit`

## Configuration

### Config Struct (target -- replaces current screenshot config)

```yaml
# ~/.config/shashin/shashin.yaml
viewer:
  default_fit: fit             # fit, fill, actual, custom
  background: "#2e3440"        # background color behind image
  zoom_step: 0.1               # zoom increment per step
  smooth_zoom: true            # animate zoom transitions
  loop_navigation: true        # wrap around at end of directory
gallery:
  thumbnail_size: 256          # thumbnail dimension in pixels
  columns: 0                   # 0 = auto based on window width
  sort: name                   # name, date, size, dimensions
  sort_reverse: false
  show_filenames: true
  cache_dir: null              # null = ~/.cache/shashin/thumbnails
metadata:
  overlay_opacity: 0.85
  show_on_open: false          # auto-show metadata on image open
  gps_link: true               # open GPS coords in browser
effects:
  shader_dir: null             # null = ~/.config/shashin/shaders
slideshow:
  interval_secs: 5
  shuffle: false
  loop: true
capture:
  default_mode: region         # region, window, fullscreen
  delay_ms: 0
  include_cursor: false
output:
  save_dir: ~/Pictures/Screenshots
  format: png                  # png, jpg, webp
  quality: 95
  filename_template: "shashin_%Y-%m-%d_%H-%M-%S"
clipboard:
  auto_copy: true
  auto_clear_secs: null
hotkeys:
  fullscreen: "cmd+shift+3"
  region: "cmd+shift+4"
  window: "cmd+shift+5"
keybindings: {}                # override default keybindings
```

### Env Overrides

- `SHASHIN_CONFIG=/path/to/config.yaml` -- full config path override
- `SHASHIN_VIEWER__DEFAULT_FIT=fill` -- nested field (double underscore)
- `SHASHIN_SLIDESHOW__INTERVAL_SECS=10` -- nested field

## Image Format Support

Via the `image` crate (pure Rust, no system dependencies):

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| JPEG | yes | yes | Lossy, EXIF metadata |
| PNG | yes | yes | Lossless, transparency |
| WebP | yes | yes | Both lossy and lossless |
| BMP | yes | yes | Uncompressed |
| TIFF | yes | yes | Multi-page support |
| GIF | yes | no | Animated GIF: first frame only initially |
| ICO | yes | no | Favicon format |
| AVIF | yes | no | Modern format (via `image` feature flag) |
| RAW | no | no | Consider `rawloader` crate for future RAW support |

## Nix Integration

### Flake Structure

Uses substrate `rust-tool-release-flake.nix` for multi-platform packages.

**Exports:**
- `packages.{system}.{shashin,default}` -- the binary
- `overlays.default` -- `pkgs.shashin`
- `homeManagerModules.default` -- HM module at `blackmatter.components.shashin`
- `devShells.{system}.default` -- dev environment
- `apps.{system}.{check-all,bump,publish,release,regenerate}` -- substrate apps

### HM Module (`module/default.nix`)

Already exists with typed options for the screenshot-tool config. Needs updating
when the config struct changes to the viewer-oriented layout. Current options:

- `blackmatter.components.shashin.enable`
- `blackmatter.components.shashin.package`
- `blackmatter.components.shashin.capture.*`
- `blackmatter.components.shashin.output.*`
- `blackmatter.components.shashin.annotation.*`
- `blackmatter.components.shashin.hotkeys.*`
- `blackmatter.components.shashin.clipboard.*`
- `blackmatter.components.shashin.extraSettings`

Generates YAML via `lib.generators.toYAML` to `xdg.configFile."shashin/shashin.yaml"`.
Includes launchd/systemd service for hotkey listener daemon.

## Design Decisions

### Image Loading
- **`image` crate**: Pure Rust, broad format support, no system dependencies.
  Nix-friendly. Handles decode to RGBA buffer which uploads directly to wgpu texture.
- **No ImageMagick/libvips**: Heavy C dependencies, complex Nix packaging.
  The `image` crate covers all common formats.

### Texture Management
- **Single texture for current image**: Upload on image change. Reuse texture
  if same dimensions. Large images (>8K): downscale to GPU max texture size.
- **Thumbnail cache**: Pre-generated thumbnails stored as PNG in cache directory.
  Content-hash keys avoid rebuilding when files haven't changed.

### Effects Pipeline
- **Non-destructive**: All effects applied via GPU shader at render time.
  Original image bytes never modified. Export creates a new file.
- **Composable**: Effects stack in order. Each effect is a WGSL shader pass.
  Built-in effects use garasu `ShaderPipeline` with uniform buffers for parameters.
- **User shaders**: Custom `.wgsl` files in config directory. Same interface as
  garasu shader plugins (input_texture, input_sampler, uniforms).

### Platform Strategy
- **macOS first**: ScreenCaptureKit for capture, Core Graphics for window listing.
  Pure safe Rust via objc2 bindings (same pattern as mado).
- **Linux planned**: PipeWire for capture, X11/Wayland screenshot portals.
  Behind `ScreenCapture` trait -- no platform leakage into viewer/gallery code.
- **Image viewing is cross-platform**: Only capture is platform-specific.
  The viewer, gallery, metadata, and effects modules are fully cross-platform.

## Testing Strategy

- **Unit tests**: Config parsing, metadata extraction, sort/filter logic,
  zoom/pan math, effect parameter validation.
- **Integration tests**: Load real image files from `tests/fixtures/`,
  verify decode + thumbnail generation + metadata extraction.
- **Visual tests**: Screenshot comparison for GPU rendering via garasu test utilities.
- **Platform tests**: Capture backend tests gated behind `#[cfg(target_os = "macos")]`.

## Error Handling

- Module-specific error enums with `thiserror` (viewer, gallery, metadata, effects).
- Top-level uses `anyhow::Result` for CLI error reporting.
- Graceful degradation: corrupt images show error placeholder, missing EXIF shows "N/A".
- Tracing: `tracing::{info,debug,warn,error}` with structured fields throughout.
