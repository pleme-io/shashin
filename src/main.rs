//! Shashin (写真) — GPU-rendered image viewer.
//!
//! CLI entry point: opens images in viewer, gallery, or slideshow mode.

mod config;
mod gallery;
mod input;
mod mcp;
mod metadata;
mod platform;
mod render;
mod viewer;

use clap::{Parser, Subcommand};
use madori::{App, AppEvent, EventResponse};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use crate::config::{FitMode, ShashinConfig};
use crate::input::{Action, Mode};
use crate::render::ShashinRenderer;

#[derive(Parser)]
#[command(
    name = "shashin",
    about = "Shashin (写真) — GPU-rendered image viewer"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Image file or directory to open (default: current directory gallery).
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Open a specific image file in the viewer.
    Open {
        /// Path to the image file.
        path: PathBuf,
    },
    /// Open a directory in gallery view.
    Gallery {
        /// Path to the directory containing images.
        path: Option<PathBuf>,
    },
    /// Start a slideshow from a directory.
    Slideshow {
        /// Path to the directory containing images.
        path: Option<PathBuf>,
        /// Interval between images in seconds.
        #[arg(short, long)]
        interval: Option<f32>,
    },
    /// Run as MCP server (stdio transport) for Claude Code integration.
    Mcp,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Handle MCP subcommand before loading GUI config
    if let Some(Command::Mcp) = cli.command {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(mcp::run())
            .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        return Ok(());
    }

    let config = config::load_config();

    // Determine what to open and in which mode
    match cli.command {
        Some(Command::Mcp) => unreachable!("handled above"),
        Some(Command::Open { path }) => run_viewer(&config, &path),
        Some(Command::Gallery { path }) => {
            let dir = path.unwrap_or_else(|| PathBuf::from("."));
            run_gallery(&config, &dir)
        }
        Some(Command::Slideshow { path, interval }) => {
            let dir = path.unwrap_or_else(|| PathBuf::from("."));
            run_slideshow(&config, &dir, interval)
        }
        None => {
            // No subcommand: check if path argument was given
            if let Some(path) = cli.path {
                if path.is_dir() {
                    run_gallery(&config, &path)
                } else {
                    run_viewer(&config, &path)
                }
            } else {
                // Default: open gallery in current directory
                run_gallery(&config, &PathBuf::from("."))
            }
        }
    }
}

/// Run in single-image viewer mode.
fn run_viewer(config: &ShashinConfig, path: &PathBuf) -> anyhow::Result<()> {
    let path = std::fs::canonicalize(path)?;
    tracing::info!("opening image: {}", path.display());

    let mut renderer = ShashinRenderer::new(config);
    renderer.mode = Mode::Viewer;

    // Load the image (will be uploaded after GPU init)
    if let Err(e) = renderer.viewer.open(&path) {
        tracing::error!("failed to open image: {e}");
        return Err(anyhow::anyhow!("failed to open image: {e}"));
    }

    let title = format!(
        "shashin — {}",
        path.file_name()
            .map_or("image".into(), |n| n.to_string_lossy().into_owned())
    );

    run_app(renderer, &title)
}

/// Run in gallery mode.
fn run_gallery(config: &ShashinConfig, dir: &PathBuf) -> anyhow::Result<()> {
    let dir = std::fs::canonicalize(dir)?;
    tracing::info!("opening gallery: {}", dir.display());

    let mut renderer = ShashinRenderer::new(config);
    renderer.mode = Mode::Gallery;

    match renderer.gallery.scan_directory(&dir) {
        Ok(count) => tracing::info!("found {count} images in gallery"),
        Err(e) => {
            tracing::error!("failed to scan directory: {e}");
            return Err(anyhow::anyhow!("failed to scan directory: {e}"));
        }
    }

    let title = format!(
        "shashin — {}",
        dir.file_name()
            .map_or("gallery".into(), |n| n.to_string_lossy().into_owned())
    );

    run_app(renderer, &title)
}

/// Run in slideshow mode.
fn run_slideshow(
    config: &ShashinConfig,
    dir: &PathBuf,
    interval: Option<f32>,
) -> anyhow::Result<()> {
    let dir = std::fs::canonicalize(dir)?;
    tracing::info!("starting slideshow: {}", dir.display());

    let mut renderer = ShashinRenderer::new(config);
    renderer.mode = Mode::Viewer;

    // Set slideshow interval if overridden
    if let Some(interval) = interval {
        renderer.slideshow.interval = interval;
    }

    // Load images from directory
    let images = gallery::list_images_in_dir(&dir)?;
    if images.is_empty() {
        return Err(anyhow::anyhow!("no images found in {}", dir.display()));
    }

    renderer.viewer.set_image_list(images);
    if let Err(e) = renderer.viewer.open_index(0) {
        tracing::error!("failed to open first image: {e}");
        return Err(anyhow::anyhow!("failed to open first image: {e}"));
    }

    // Start slideshow immediately
    renderer.slideshow.toggle();

    let title = format!(
        "shashin — slideshow: {}",
        dir.file_name()
            .map_or("slideshow".into(), |n| n.to_string_lossy().into_owned())
    );

    run_app(renderer, &title)
}

/// Run the madori application loop with the configured renderer.
fn run_app(renderer: ShashinRenderer, title: &str) -> anyhow::Result<()> {
    App::builder(renderer)
        .title(title)
        .size(1280, 720)
        .on_event(handle_event)
        .run()
        .map_err(|e| anyhow::anyhow!("application error: {e}"))
}

/// Main event handler dispatching to viewer/gallery actions.
fn handle_event(event: &AppEvent, renderer: &mut ShashinRenderer) -> EventResponse {
    match event {
        AppEvent::Key(key_event) if key_event.pressed => {
            let action = input::map_key(&key_event.key, &key_event.modifiers, renderer.mode);
            handle_action(action, renderer)
        }
        AppEvent::Mouse(madori::event::MouseEvent::Scroll { dy, .. }) => {
            let action = input::map_scroll(*dy);
            handle_action(action, renderer)
        }
        AppEvent::Resized { width, height } => {
            renderer.viewer.resize(*width, *height);
            renderer.gallery.update_layout(*width, *height);
            EventResponse::consumed()
        }
        _ => EventResponse::ignored(),
    }
}

/// Execute a semantic action on the renderer.
fn handle_action(action: Action, renderer: &mut ShashinRenderer) -> EventResponse {
    match action {
        // -- Navigation --
        Action::NextImage => {
            if let Err(e) = renderer.viewer.next_image() {
                tracing::warn!("next image failed: {e}");
            }
            // Flag that we need to re-upload after GPU is available
            // (This will be handled on next render when we detect image changed)
            EventResponse::consumed()
        }
        Action::PrevImage => {
            if let Err(e) = renderer.viewer.prev_image() {
                tracing::warn!("prev image failed: {e}");
            }
            EventResponse::consumed()
        }
        Action::FirstImage => {
            match renderer.mode {
                Mode::Gallery => renderer.gallery.select_first(),
                _ => {
                    if !renderer.viewer.image_list.is_empty() {
                        if let Err(e) = renderer.viewer.open_index(0) {
                            tracing::warn!("first image failed: {e}");
                        }
                    }
                }
            }
            EventResponse::consumed()
        }
        Action::LastImage => {
            match renderer.mode {
                Mode::Gallery => renderer.gallery.select_last(),
                _ => {
                    let last = renderer.viewer.image_list.len().saturating_sub(1);
                    if let Err(e) = renderer.viewer.open_index(last) {
                        tracing::warn!("last image failed: {e}");
                    }
                }
            }
            EventResponse::consumed()
        }

        // -- Zoom --
        Action::ZoomIn => {
            renderer.viewer.zoom_in();
            EventResponse::consumed()
        }
        Action::ZoomOut => {
            renderer.viewer.zoom_out();
            EventResponse::consumed()
        }
        Action::ZoomActual => {
            renderer.viewer.zoom_actual();
            EventResponse::consumed()
        }
        Action::FitToWindow => {
            renderer.viewer.set_fit_mode(FitMode::Fit);
            EventResponse::consumed()
        }
        Action::FillWindow => {
            renderer.viewer.set_fit_mode(FitMode::Fill);
            EventResponse::consumed()
        }

        // -- Transform --
        Action::RotateCw => {
            renderer.viewer.rotate_cw();
            EventResponse::consumed()
        }
        Action::RotateCcw => {
            renderer.viewer.rotate_ccw();
            EventResponse::consumed()
        }
        Action::FlipHorizontal => {
            renderer.viewer.flip_horizontal();
            EventResponse::consumed()
        }
        Action::FlipVertical => {
            renderer.viewer.flip_vertical();
            EventResponse::consumed()
        }

        // -- Pan --
        Action::PanLeft => {
            renderer.viewer.pan(-50.0, 0.0);
            EventResponse::consumed()
        }
        Action::PanRight => {
            renderer.viewer.pan(50.0, 0.0);
            EventResponse::consumed()
        }
        Action::PanUp => {
            renderer.viewer.pan(0.0, -50.0);
            EventResponse::consumed()
        }
        Action::PanDown => {
            renderer.viewer.pan(0.0, 50.0);
            EventResponse::consumed()
        }

        // -- View --
        Action::ToggleInfo => {
            renderer.viewer.toggle_info();
            EventResponse::consumed()
        }
        Action::ToggleSlideshow => {
            renderer.slideshow.toggle();
            tracing::info!(
                "slideshow {}",
                if renderer.slideshow.active {
                    "started"
                } else {
                    "stopped"
                }
            );
            EventResponse::consumed()
        }
        Action::SwitchToGallery => {
            renderer.mode = Mode::Gallery;
            // If we haven't scanned yet, scan the current image's directory
            if renderer.gallery.is_empty() {
                if let Some(path) = renderer.viewer.current_path() {
                    if let Some(dir) = path.parent() {
                        if let Err(e) = renderer.gallery.scan_directory(dir) {
                            tracing::warn!("gallery scan failed: {e}");
                        }
                    }
                }
            }
            EventResponse::consumed()
        }
        Action::SwitchToViewer => {
            renderer.mode = Mode::Viewer;
            EventResponse::consumed()
        }

        // -- Gallery --
        Action::GalleryLeft => {
            renderer.gallery.move_left();
            EventResponse::consumed()
        }
        Action::GalleryRight => {
            renderer.gallery.move_right();
            EventResponse::consumed()
        }
        Action::GalleryUp => {
            renderer.gallery.move_up();
            EventResponse::consumed()
        }
        Action::GalleryDown => {
            renderer.gallery.move_down();
            EventResponse::consumed()
        }
        Action::GalleryOpen => {
            if let Some(path) = renderer.gallery.selected_path() {
                let path = path.to_owned();
                renderer.mode = Mode::Viewer;
                renderer
                    .viewer
                    .set_image_list(renderer.gallery.all_paths());
                if let Err(e) = renderer.viewer.open(&path) {
                    tracing::warn!("failed to open image: {e}");
                }
            }
            EventResponse::consumed()
        }
        Action::GalleryCycleSort => {
            renderer.gallery.cycle_sort();
            EventResponse::consumed()
        }
        Action::GalleryToggleMark => {
            renderer.gallery.toggle_mark();
            EventResponse::consumed()
        }

        // -- App --
        Action::Quit => EventResponse {
            consumed: true,
            exit: true,
            set_title: None,
        },
        Action::Escape => {
            // In slideshow: stop slideshow
            if renderer.slideshow.active {
                renderer.slideshow.toggle();
            } else if renderer.mode == Mode::Gallery {
                renderer.mode = Mode::Viewer;
            }
            EventResponse::consumed()
        }
        Action::None => EventResponse::ignored(),
    }
}
