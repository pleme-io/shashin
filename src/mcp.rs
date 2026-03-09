//! MCP server for shashin image viewer.
//!
//! Provides tools for opening images, browsing galleries, reading metadata,
//! and controlling slideshow playback.

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;

// ── Tool input types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OpenImageInput {
    #[schemars(description = "Absolute path to the image file to open.")]
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GalleryListInput {
    #[schemars(description = "Directory path to list images from. Defaults to current directory.")]
    path: Option<String>,
    #[schemars(description = "Sort order: 'name', 'date', 'size', or 'dimensions'.")]
    sort: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetMetadataInput {
    #[schemars(description = "Path to the image file to read metadata from.")]
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SlideshowInput {
    #[schemars(description = "Directory path for slideshow images.")]
    path: Option<String>,
    #[schemars(description = "Interval between images in seconds.")]
    interval: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ConfigGetInput {
    #[schemars(description = "Config key to retrieve (e.g., 'viewer.default_fit', 'gallery.thumbnail_size'). Omit for full config.")]
    key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ConfigSetInput {
    #[schemars(description = "Config key to set.")]
    key: String,
    #[schemars(description = "Value to set (as JSON string).")]
    value: String,
}

// ── MCP Server ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ShashinMcp {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl ShashinMcp {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // ── Standard tools ──────────────────────────────────────────────────────

    #[tool(description = "Get shashin application status and health information.")]
    async fn status(&self) -> String {
        serde_json::json!({
            "status": "running",
            "app": "shashin",
            "note": "MCP server is operational. GUI state queries require a running shashin instance with IPC."
        })
        .to_string()
    }

    #[tool(description = "Get shashin version information.")]
    async fn version(&self) -> String {
        serde_json::json!({
            "name": "shashin",
            "version": env!("CARGO_PKG_VERSION"),
            "description": env!("CARGO_PKG_DESCRIPTION"),
            "renderer": "wgpu (Metal/Vulkan)",
            "supported_formats": ["jpeg", "png", "webp", "gif", "bmp", "tiff", "avif"],
        })
        .to_string()
    }

    #[tool(description = "Get a shashin configuration value. Pass a key for a specific value, or omit for the full config.")]
    async fn config_get(&self, Parameters(input): Parameters<ConfigGetInput>) -> String {
        match input.key {
            Some(key) => serde_json::json!({
                "key": key,
                "value": null,
                "note": "Config queries require IPC to a running shashin instance."
            })
            .to_string(),
            None => serde_json::json!({
                "note": "Full config retrieval requires IPC to a running shashin instance.",
                "config_path": "~/.config/shashin/shashin.yaml"
            })
            .to_string(),
        }
    }

    #[tool(description = "Set a shashin configuration value at runtime.")]
    async fn config_set(&self, Parameters(input): Parameters<ConfigSetInput>) -> String {
        serde_json::json!({
            "key": input.key,
            "value": input.value,
            "applied": false,
            "note": "Config mutations require IPC to a running shashin instance."
        })
        .to_string()
    }

    // ── Image viewer tools ──────────────────────────────────────────────────

    #[tool(description = "Open an image file in the viewer. Accepts absolute path to any supported image format (JPEG, PNG, WebP, GIF, BMP, TIFF, AVIF).")]
    async fn open_image(&self, Parameters(input): Parameters<OpenImageInput>) -> String {
        let path = std::path::Path::new(&input.path);
        if !path.exists() {
            return serde_json::json!({
                "error": format!("file not found: {}", input.path)
            })
            .to_string();
        }
        serde_json::json!({
            "path": input.path,
            "ok": false,
            "note": "Image opening requires IPC to a running shashin instance."
        })
        .to_string()
    }

    #[tool(description = "List images in a directory for gallery view. Returns file paths, names, sizes, and modification dates.")]
    async fn gallery_list(&self, Parameters(input): Parameters<GalleryListInput>) -> String {
        let dir = input.path.unwrap_or_else(|| ".".to_string());
        let path = std::path::Path::new(&dir);
        if !path.is_dir() {
            return serde_json::json!({
                "error": format!("not a directory: {dir}")
            })
            .to_string();
        }

        // List image files in the directory
        let mut images = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(
                        ext_lower.as_str(),
                        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "avif"
                    ) {
                        let meta = entry.metadata().ok();
                        images.push(serde_json::json!({
                            "path": entry_path.display().to_string(),
                            "name": entry_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                            "size_bytes": meta.as_ref().map(|m| m.len()),
                        }));
                    }
                }
            }
        }

        serde_json::json!({
            "directory": dir,
            "sort": input.sort.unwrap_or_else(|| "name".to_string()),
            "count": images.len(),
            "images": images,
        })
        .to_string()
    }

    #[tool(description = "Get EXIF/XMP metadata for an image file. Returns camera info, dimensions, GPS coordinates, and dates.")]
    async fn get_metadata(&self, Parameters(input): Parameters<GetMetadataInput>) -> String {
        let path = std::path::Path::new(&input.path);
        if !path.exists() {
            return serde_json::json!({
                "error": format!("file not found: {}", input.path)
            })
            .to_string();
        }

        // Basic file metadata
        let file_meta = std::fs::metadata(path).ok();
        serde_json::json!({
            "path": input.path,
            "size_bytes": file_meta.as_ref().map(|m| m.len()),
            "exif": null,
            "note": "Full EXIF extraction requires the shashin library. Basic file metadata returned."
        })
        .to_string()
    }

    #[tool(description = "Start a slideshow from a directory of images. Optionally specify the interval between images.")]
    async fn slideshow_start(&self, Parameters(input): Parameters<SlideshowInput>) -> String {
        serde_json::json!({
            "path": input.path,
            "interval_secs": input.interval.unwrap_or(5.0),
            "ok": false,
            "note": "Slideshow control requires IPC to a running shashin instance."
        })
        .to_string()
    }

    #[tool(description = "Stop the currently running slideshow.")]
    async fn slideshow_stop(&self) -> String {
        serde_json::json!({
            "ok": false,
            "note": "Slideshow control requires IPC to a running shashin instance."
        })
        .to_string()
    }
}

#[tool_handler]
impl ServerHandler for ShashinMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Shashin GPU image viewer — image browsing, metadata extraction, and slideshow control."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let server = ShashinMcp::new().serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
