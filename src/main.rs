mod config;
mod platform;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::config::ShashinConfig;
use crate::platform::create_capture;

#[derive(Parser)]
#[command(name = "shashin", about = "Shashin (写真) — screenshot tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Capture the entire screen.
    Fullscreen,
    /// Capture a selected region.
    Region,
    /// Capture a specific window.
    Window,
    /// Annotate an existing screenshot.
    Annotate {
        /// Path to the image file to annotate.
        file: String,
    },
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config via shikumi
    let config = match shikumi::ConfigDiscovery::new("shashin")
        .env_override("SHASHIN_CONFIG")
        .discover()
    {
        Ok(path) => {
            tracing::info!("loading config from {}", path.display());
            let store = shikumi::ConfigStore::<ShashinConfig>::load(&path, "SHASHIN_")
                .unwrap_or_else(|e| {
                    tracing::warn!("failed to load config: {e}, using defaults");
                    let tmp = std::env::temp_dir().join("shashin-default.yaml");
                    std::fs::write(&tmp, "{}").ok();
                    shikumi::ConfigStore::load(&tmp, "SHASHIN_").unwrap()
                });
            ShashinConfig::clone(&store.get())
        }
        Err(_) => {
            tracing::info!("no config file found, using defaults");
            ShashinConfig::default()
        }
    };

    let capture = create_capture();

    match cli.command {
        Some(Command::Fullscreen) => {
            tracing::info!("capturing fullscreen");
            if config.capture.delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(
                    u64::from(config.capture.delay_ms),
                ));
            }
            match capture.capture_fullscreen() {
                Ok(img) => tracing::info!("captured {}x{} image", img.width, img.height),
                Err(e) => tracing::error!("capture failed: {e}"),
            }
        }
        Some(Command::Region) => {
            tracing::info!("capturing region");
            // TODO: interactive region selection
            match capture.capture_region(0, 0, 800, 600) {
                Ok(img) => tracing::info!("captured {}x{} image", img.width, img.height),
                Err(e) => tracing::error!("capture failed: {e}"),
            }
        }
        Some(Command::Window) => {
            tracing::info!("capturing window");
            // TODO: interactive window selection
            match capture.capture_window(0) {
                Ok(img) => tracing::info!("captured {}x{} image", img.width, img.height),
                Err(e) => tracing::error!("capture failed: {e}"),
            }
        }
        Some(Command::Annotate { file }) => {
            tracing::info!("annotating {file}");
            // TODO: open annotation UI
        }
        None => {
            // Default: use configured default mode
            tracing::info!(
                "no command specified, using default mode: {}",
                config.capture.default_mode
            );
        }
    }
}
