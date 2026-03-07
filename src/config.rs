//! Shashin configuration — uses shikumi for discovery and hot-reload.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ShashinConfig {
    pub capture: CaptureConfig,
    pub output: OutputConfig,
    pub annotation: AnnotationConfig,
    pub hotkeys: HotkeyConfig,
    pub clipboard: ClipboardConfig,
}

impl Default for ShashinConfig {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            output: OutputConfig::default(),
            annotation: AnnotationConfig::default(),
            hotkeys: HotkeyConfig::default(),
            clipboard: ClipboardConfig::default(),
        }
    }
}

/// Screen capture settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct CaptureConfig {
    /// Default capture mode: "region", "window", or "fullscreen".
    pub default_mode: String,
    /// Delay in milliseconds before capture.
    pub delay_ms: u32,
    /// Whether to include the cursor in captures.
    pub include_cursor: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            default_mode: "region".into(),
            delay_ms: 0,
            include_cursor: false,
        }
    }
}

/// Output settings for saved screenshots.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct OutputConfig {
    /// Directory to save screenshots.
    pub save_dir: PathBuf,
    /// Image format: "png", "jpg", or "webp".
    pub format: String,
    /// Image quality (1–100, applicable to jpg/webp).
    pub quality: u8,
    /// Filename template (supports strftime-style placeholders).
    pub filename_template: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            save_dir: dirs::picture_dir()
                .unwrap_or_else(|| PathBuf::from("~/Pictures"))
                .join("Screenshots"),
            format: "png".into(),
            quality: 95,
            filename_template: "shashin_%Y-%m-%d_%H-%M-%S".into(),
        }
    }
}

/// Annotation overlay settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct AnnotationConfig {
    /// Enable annotation mode after capture.
    pub enabled: bool,
    /// Default annotation color (hex string).
    pub default_color: String,
    /// Line width in pixels.
    pub line_width: f32,
    /// Font size for text annotations.
    pub font_size: f32,
}

impl Default for AnnotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_color: "#ff0000".into(),
            line_width: 2.0,
            font_size: 16.0,
        }
    }
}

/// Global hotkey bindings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct HotkeyConfig {
    /// Hotkey for fullscreen capture.
    pub fullscreen: String,
    /// Hotkey for region capture.
    pub region: String,
    /// Hotkey for window capture.
    pub window: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            fullscreen: "cmd+shift+3".into(),
            region: "cmd+shift+4".into(),
            window: "cmd+shift+5".into(),
        }
    }
}

/// Clipboard behavior settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ClipboardConfig {
    /// Automatically copy screenshot to clipboard.
    pub auto_copy: bool,
    /// Seconds after which to clear clipboard (None = never).
    pub auto_clear_secs: Option<u32>,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            auto_copy: true,
            auto_clear_secs: None,
        }
    }
}
