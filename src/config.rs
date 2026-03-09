//! Shashin configuration — uses shikumi for discovery and hot-reload.
//!
//! Config struct oriented toward the image viewer use case with viewer,
//! gallery, metadata, slideshow, and output sections.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for shashin image viewer.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ShashinConfig {
    pub viewer: ViewerConfig,
    pub gallery: GalleryConfig,
    pub metadata: MetadataConfig,
    pub slideshow: SlideshowConfig,
    pub output: OutputConfig,
    pub theme: ThemeConfig,
}

impl Default for ShashinConfig {
    fn default() -> Self {
        Self {
            viewer: ViewerConfig::default(),
            gallery: GalleryConfig::default(),
            metadata: MetadataConfig::default(),
            slideshow: SlideshowConfig::default(),
            output: OutputConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

/// How the image fits within the viewer window.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    /// Scale image to fit entirely within the window (letterbox).
    Fit,
    /// Scale image to fill the window completely (crop overflow).
    Fill,
    /// Display at original pixel size (1:1).
    Actual,
}

impl Default for FitMode {
    fn default() -> Self {
        Self::Fit
    }
}

/// Viewer display settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ViewerConfig {
    /// Default fit mode when opening an image.
    pub default_fit: FitMode,
    /// Background color behind the image (hex).
    pub background: String,
    /// Zoom step per scroll increment (fraction of current zoom).
    pub zoom_step: f32,
    /// Whether to animate zoom transitions.
    pub smooth_zoom: bool,
    /// Whether navigation wraps at end of directory.
    pub loop_navigation: bool,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            default_fit: FitMode::Fit,
            background: "#2e3440".into(),
            zoom_step: 0.1,
            smooth_zoom: true,
            loop_navigation: true,
        }
    }
}

/// Sort order for gallery and navigation.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Name,
    Date,
    Size,
    Dimensions,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Name
    }
}

/// Gallery view settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct GalleryConfig {
    /// Thumbnail dimension in pixels (square).
    pub thumbnail_size: u32,
    /// Number of columns (0 = auto based on window width).
    pub columns: u32,
    /// Default sort order.
    pub sort: SortOrder,
    /// Whether to reverse sort direction.
    pub sort_reverse: bool,
    /// Show filenames below thumbnails.
    pub show_filenames: bool,
    /// Thumbnail cache directory (None = ~/.cache/shashin/thumbnails).
    pub cache_dir: Option<PathBuf>,
}

impl Default for GalleryConfig {
    fn default() -> Self {
        Self {
            thumbnail_size: 256,
            columns: 0,
            sort: SortOrder::Name,
            sort_reverse: false,
            show_filenames: true,
            cache_dir: None,
        }
    }
}

/// Metadata overlay settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct MetadataConfig {
    /// Opacity of the metadata overlay panel (0.0 - 1.0).
    pub overlay_opacity: f32,
    /// Whether to auto-show metadata when opening an image.
    pub show_on_open: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            overlay_opacity: 0.85,
            show_on_open: false,
        }
    }
}

/// Slideshow settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct SlideshowConfig {
    /// Interval between images in seconds.
    pub interval_secs: f32,
    /// Whether to shuffle image order.
    pub shuffle: bool,
    /// Whether to loop back to start.
    pub loop_slideshow: bool,
}

impl Default for SlideshowConfig {
    fn default() -> Self {
        Self {
            interval_secs: 5.0,
            shuffle: false,
            loop_slideshow: true,
        }
    }
}

/// Output settings for saving/exporting images.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct OutputConfig {
    /// Directory to save screenshots.
    pub save_dir: PathBuf,
    /// Image format: "png", "jpg", or "webp".
    pub format: String,
    /// Image quality (1-100, applicable to jpg/webp).
    pub quality: u8,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            save_dir: dirs::picture_dir()
                .unwrap_or_else(|| PathBuf::from("~/Pictures"))
                .join("Screenshots"),
            format: "png".into(),
            quality: 95,
        }
    }
}

/// Theme overrides.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct ThemeConfig {
    /// Background color for the viewer area (hex).
    pub background: String,
    /// Foreground/text color (hex).
    pub foreground: String,
    /// Accent/selection color (hex).
    pub accent: String,
    /// Muted/secondary text color (hex).
    pub muted: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: "#2e3440".into(),
            foreground: "#e5e9f0".into(),
            accent: "#88c0d0".into(),
            muted: "#4c566a".into(),
        }
    }
}

/// Load configuration via shikumi, falling back to defaults.
pub fn load_config() -> ShashinConfig {
    match shikumi::ConfigDiscovery::new("shashin")
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = ShashinConfig::default();
        assert_eq!(config.viewer.default_fit, FitMode::Fit);
        assert!((config.viewer.zoom_step - 0.1).abs() < f32::EPSILON);
        assert!(config.viewer.loop_navigation);
        assert_eq!(config.gallery.thumbnail_size, 256);
        assert_eq!(config.gallery.sort, SortOrder::Name);
        assert!((config.slideshow.interval_secs - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fit_mode_default() {
        assert_eq!(FitMode::default(), FitMode::Fit);
    }

    #[test]
    fn sort_order_default() {
        assert_eq!(SortOrder::default(), SortOrder::Name);
    }

    #[test]
    fn serde_roundtrip() {
        let config = ShashinConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let config2: ShashinConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config2.viewer.default_fit, config.viewer.default_fit);
        assert_eq!(config2.gallery.thumbnail_size, config.gallery.thumbnail_size);
    }

    #[test]
    fn fit_mode_serde() {
        let json = serde_json::to_string(&FitMode::Fill).unwrap();
        assert_eq!(json, "\"fill\"");
        let parsed: FitMode = serde_json::from_str("\"actual\"").unwrap();
        assert_eq!(parsed, FitMode::Actual);
    }

    #[test]
    fn sort_order_serde() {
        let json = serde_json::to_string(&SortOrder::Date).unwrap();
        assert_eq!(json, "\"date\"");
        let parsed: SortOrder = serde_json::from_str("\"size\"").unwrap();
        assert_eq!(parsed, SortOrder::Size);
    }
}
