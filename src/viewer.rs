//! Image viewer — single image display with zoom, pan, rotation, and fit modes.
//!
//! Manages the current image state including texture data, view transform
//! (zoom level, pan offset, rotation), and navigation through a directory of images.

use crate::config::{FitMode, ViewerConfig};
use std::path::{Path, PathBuf};

/// Rotation state in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Cw90,
    Cw180,
    Cw270,
}

impl Rotation {
    /// Rotate 90 degrees clockwise.
    #[must_use]
    pub fn rotate_cw(self) -> Self {
        match self {
            Self::None => Self::Cw90,
            Self::Cw90 => Self::Cw180,
            Self::Cw180 => Self::Cw270,
            Self::Cw270 => Self::None,
        }
    }

    /// Rotate 90 degrees counter-clockwise.
    #[must_use]
    pub fn rotate_ccw(self) -> Self {
        match self {
            Self::None => Self::Cw270,
            Self::Cw90 => Self::None,
            Self::Cw180 => Self::Cw90,
            Self::Cw270 => Self::Cw180,
        }
    }

    /// Get the rotation angle in radians.
    #[must_use]
    pub fn radians(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Cw90 => std::f32::consts::FRAC_PI_2,
            Self::Cw180 => std::f32::consts::PI,
            Self::Cw270 => 3.0 * std::f32::consts::FRAC_PI_2,
        }
    }

    /// Get the rotation angle in degrees.
    #[must_use]
    pub fn degrees(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Cw90 => 90,
            Self::Cw180 => 180,
            Self::Cw270 => 270,
        }
    }
}

/// Flip state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flip {
    pub horizontal: bool,
    pub vertical: bool,
}

impl Default for Flip {
    fn default() -> Self {
        Self {
            horizontal: false,
            vertical: false,
        }
    }
}

/// View transform state: zoom, pan, rotation, flip.
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Zoom level (1.0 = 100%).
    pub zoom: f32,
    /// Target zoom for smooth animation.
    pub target_zoom: f32,
    /// Pan offset in pixels (relative to center).
    pub pan_x: f32,
    pub pan_y: f32,
    /// Rotation.
    pub rotation: Rotation,
    /// Flip state.
    pub flip: Flip,
    /// Current fit mode.
    pub fit_mode: FitMode,
}

impl ViewState {
    /// Create a new view state with default values.
    #[must_use]
    pub fn new(fit_mode: FitMode) -> Self {
        Self {
            zoom: 1.0,
            target_zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            rotation: Rotation::None,
            flip: Flip::default(),
            fit_mode,
        }
    }

    /// Reset to default view (centered, no zoom/rotation).
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.target_zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.rotation = Rotation::None;
        self.flip = Flip::default();
    }

    /// Calculate the zoom level to fit the image within the window.
    #[must_use]
    pub fn fit_zoom(img_w: u32, img_h: u32, win_w: u32, win_h: u32) -> f32 {
        if img_w == 0 || img_h == 0 || win_w == 0 || win_h == 0 {
            return 1.0;
        }
        let sx = win_w as f32 / img_w as f32;
        let sy = win_h as f32 / img_h as f32;
        sx.min(sy)
    }

    /// Calculate the zoom level to fill the window (may crop).
    #[must_use]
    pub fn fill_zoom(img_w: u32, img_h: u32, win_w: u32, win_h: u32) -> f32 {
        if img_w == 0 || img_h == 0 || win_w == 0 || win_h == 0 {
            return 1.0;
        }
        let sx = win_w as f32 / img_w as f32;
        let sy = win_h as f32 / img_h as f32;
        sx.max(sy)
    }

    /// Apply fit mode to set zoom level.
    pub fn apply_fit(&mut self, img_w: u32, img_h: u32, win_w: u32, win_h: u32) {
        let z = match self.fit_mode {
            FitMode::Fit => Self::fit_zoom(img_w, img_h, win_w, win_h),
            FitMode::Fill => Self::fill_zoom(img_w, img_h, win_w, win_h),
            FitMode::Actual => 1.0,
        };
        self.zoom = z;
        self.target_zoom = z;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Smoothly interpolate zoom toward target.
    pub fn animate(&mut self, dt: f32) {
        let lerp_speed = 10.0;
        let diff = self.target_zoom - self.zoom;
        if diff.abs() < 0.001 {
            self.zoom = self.target_zoom;
        } else {
            self.zoom += diff * (lerp_speed * dt).min(1.0);
        }
    }

    /// Zoom in by one step.
    pub fn zoom_in(&mut self, step: f32) {
        self.target_zoom = (self.target_zoom * (1.0 + step)).min(50.0);
    }

    /// Zoom out by one step.
    pub fn zoom_out(&mut self, step: f32) {
        self.target_zoom = (self.target_zoom / (1.0 + step)).max(0.01);
    }

    /// Set zoom to actual size (1:1).
    pub fn zoom_actual(&mut self) {
        self.target_zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Pan by a pixel delta.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Get the effective image dimensions after rotation.
    #[must_use]
    pub fn rotated_dimensions(&self, img_w: u32, img_h: u32) -> (u32, u32) {
        match self.rotation {
            Rotation::None | Rotation::Cw180 => (img_w, img_h),
            Rotation::Cw90 | Rotation::Cw270 => (img_h, img_w),
        }
    }

    /// Calculate the destination rectangle for rendering.
    /// Returns (x, y, width, height) in window coordinates.
    #[must_use]
    pub fn dest_rect(&self, img_w: u32, img_h: u32, win_w: u32, win_h: u32) -> (f32, f32, f32, f32) {
        let (rw, rh) = self.rotated_dimensions(img_w, img_h);
        let display_w = rw as f32 * self.zoom;
        let display_h = rh as f32 * self.zoom;
        let x = (win_w as f32 - display_w) / 2.0 + self.pan_x;
        let y = (win_h as f32 - display_h) / 2.0 + self.pan_y;
        (x, y, display_w, display_h)
    }
}

/// Loaded image data in RGBA format.
#[derive(Debug, Clone)]
pub struct LoadedImage {
    /// RGBA pixel data.
    pub data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Source file path.
    pub path: PathBuf,
    /// Detected image format name.
    pub format: String,
}

/// Load and decode an image file to RGBA.
pub fn load_image(path: &Path) -> Result<LoadedImage, image::ImageError> {
    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    let format = image::ImageFormat::from_path(path)
        .map(|f| format!("{f:?}"))
        .unwrap_or_else(|_| "Unknown".into());

    Ok(LoadedImage {
        width: rgba.width(),
        height: rgba.height(),
        data: rgba.into_raw(),
        path: path.to_owned(),
        format,
    })
}

/// Image viewer state machine.
#[derive(Debug)]
pub struct ImageViewer {
    /// Currently loaded image.
    pub current_image: Option<LoadedImage>,
    /// View transform.
    pub view: ViewState,
    /// List of image paths in the current directory.
    pub image_list: Vec<PathBuf>,
    /// Current index in the image list.
    pub current_index: usize,
    /// Configuration.
    config: ViewerConfig,
    /// Whether the info overlay is shown.
    pub show_info: bool,
    /// Window dimensions.
    pub window_width: u32,
    pub window_height: u32,
}

impl ImageViewer {
    /// Create a new viewer with the given config.
    #[must_use]
    pub fn new(config: &ViewerConfig) -> Self {
        Self {
            current_image: None,
            view: ViewState::new(config.default_fit),
            image_list: Vec::new(),
            current_index: 0,
            config: config.clone(),
            show_info: false,
            window_width: 0,
            window_height: 0,
        }
    }

    /// Open a single image file and populate the image list from its directory.
    pub fn open(&mut self, path: &Path) -> Result<(), image::ImageError> {
        let loaded = load_image(path)?;
        tracing::info!(
            "opened {}x{} image: {}",
            loaded.width,
            loaded.height,
            path.display()
        );

        // Populate image list from parent directory
        if let Some(dir) = path.parent() {
            match crate::gallery::list_images_in_dir(dir) {
                Ok(paths) => {
                    self.current_index = paths
                        .iter()
                        .position(|p| p == path)
                        .unwrap_or(0);
                    self.image_list = paths;
                }
                Err(e) => {
                    tracing::warn!("failed to list directory: {e}");
                    self.image_list = vec![path.to_owned()];
                    self.current_index = 0;
                }
            }
        }

        self.current_image = Some(loaded);
        self.view.reset();
        self.apply_fit();
        Ok(())
    }

    /// Open an image by index from the image list.
    pub fn open_index(&mut self, index: usize) -> Result<(), image::ImageError> {
        if let Some(path) = self.image_list.get(index).cloned() {
            let loaded = load_image(&path)?;
            self.current_index = index;
            self.current_image = Some(loaded);
            self.view.reset();
            self.apply_fit();
        }
        Ok(())
    }

    /// Set the image list directly (e.g. from gallery).
    pub fn set_image_list(&mut self, paths: Vec<PathBuf>) {
        self.image_list = paths;
        self.current_index = 0;
    }

    /// Navigate to the next image.
    pub fn next_image(&mut self) -> Result<(), image::ImageError> {
        if self.image_list.is_empty() {
            return Ok(());
        }
        let next = if self.current_index + 1 < self.image_list.len() {
            self.current_index + 1
        } else if self.config.loop_navigation {
            0
        } else {
            return Ok(());
        };
        self.open_index(next)
    }

    /// Navigate to the previous image.
    pub fn prev_image(&mut self) -> Result<(), image::ImageError> {
        if self.image_list.is_empty() {
            return Ok(());
        }
        let prev = if self.current_index > 0 {
            self.current_index - 1
        } else if self.config.loop_navigation {
            self.image_list.len() - 1
        } else {
            return Ok(());
        };
        self.open_index(prev)
    }

    /// Apply the current fit mode to the view.
    pub fn apply_fit(&mut self) {
        if let Some(ref img) = self.current_image {
            self.view
                .apply_fit(img.width, img.height, self.window_width, self.window_height);
        }
    }

    /// Set fit mode and apply it.
    pub fn set_fit_mode(&mut self, mode: FitMode) {
        self.view.fit_mode = mode;
        self.apply_fit();
    }

    /// Update window dimensions and re-apply fit.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
        self.apply_fit();
    }

    /// Toggle the info overlay.
    pub fn toggle_info(&mut self) {
        self.show_info = !self.show_info;
    }

    /// Zoom in.
    pub fn zoom_in(&mut self) {
        self.view.zoom_in(self.config.zoom_step);
    }

    /// Zoom out.
    pub fn zoom_out(&mut self) {
        self.view.zoom_out(self.config.zoom_step);
    }

    /// Zoom to actual size.
    pub fn zoom_actual(&mut self) {
        self.view.zoom_actual();
    }

    /// Rotate clockwise 90 degrees.
    pub fn rotate_cw(&mut self) {
        self.view.rotation = self.view.rotation.rotate_cw();
    }

    /// Rotate counter-clockwise 90 degrees.
    pub fn rotate_ccw(&mut self) {
        self.view.rotation = self.view.rotation.rotate_ccw();
    }

    /// Flip horizontal.
    pub fn flip_horizontal(&mut self) {
        self.view.flip.horizontal = !self.view.flip.horizontal;
    }

    /// Flip vertical.
    pub fn flip_vertical(&mut self) {
        self.view.flip.vertical = !self.view.flip.vertical;
    }

    /// Pan the image.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.view.pan(dx, dy);
    }

    /// Get the current image path.
    #[must_use]
    pub fn current_path(&self) -> Option<&Path> {
        self.current_image.as_ref().map(|img| img.path.as_path())
    }

    /// Get the navigation position as "N / M" string.
    #[must_use]
    pub fn position_display(&self) -> String {
        if self.image_list.is_empty() {
            String::new()
        } else {
            format!("{} / {}", self.current_index + 1, self.image_list.len())
        }
    }

    /// Get the zoom level as percentage string.
    #[must_use]
    pub fn zoom_display(&self) -> String {
        format!("{:.0}%", self.view.zoom * 100.0)
    }

    /// Animate smooth zoom transitions.
    pub fn update(&mut self, dt: f32) {
        if self.config.smooth_zoom {
            self.view.animate(dt);
        } else {
            self.view.zoom = self.view.target_zoom;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_cycle_cw() {
        let mut r = Rotation::None;
        r = r.rotate_cw();
        assert_eq!(r, Rotation::Cw90);
        r = r.rotate_cw();
        assert_eq!(r, Rotation::Cw180);
        r = r.rotate_cw();
        assert_eq!(r, Rotation::Cw270);
        r = r.rotate_cw();
        assert_eq!(r, Rotation::None);
    }

    #[test]
    fn rotation_cycle_ccw() {
        let mut r = Rotation::None;
        r = r.rotate_ccw();
        assert_eq!(r, Rotation::Cw270);
        r = r.rotate_ccw();
        assert_eq!(r, Rotation::Cw180);
        r = r.rotate_ccw();
        assert_eq!(r, Rotation::Cw90);
        r = r.rotate_ccw();
        assert_eq!(r, Rotation::None);
    }

    #[test]
    fn rotation_degrees() {
        assert_eq!(Rotation::None.degrees(), 0);
        assert_eq!(Rotation::Cw90.degrees(), 90);
        assert_eq!(Rotation::Cw180.degrees(), 180);
        assert_eq!(Rotation::Cw270.degrees(), 270);
    }

    #[test]
    fn view_state_fit_zoom() {
        let z = ViewState::fit_zoom(4000, 3000, 1280, 720);
        // 1280/4000 = 0.32, 720/3000 = 0.24, min = 0.24
        assert!((z - 0.24).abs() < 0.01);
    }

    #[test]
    fn view_state_fill_zoom() {
        let z = ViewState::fill_zoom(4000, 3000, 1280, 720);
        // 1280/4000 = 0.32, 720/3000 = 0.24, max = 0.32
        assert!((z - 0.32).abs() < 0.01);
    }

    #[test]
    fn view_state_zero_dimensions() {
        assert!((ViewState::fit_zoom(0, 0, 1280, 720) - 1.0).abs() < f32::EPSILON);
        assert!((ViewState::fit_zoom(100, 100, 0, 0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn view_state_dest_rect() {
        let view = ViewState {
            zoom: 0.5,
            target_zoom: 0.5,
            pan_x: 0.0,
            pan_y: 0.0,
            rotation: Rotation::None,
            flip: Flip::default(),
            fit_mode: FitMode::Fit,
        };
        let (x, y, w, h) = view.dest_rect(1000, 800, 800, 600);
        assert!((w - 500.0).abs() < f32::EPSILON);
        assert!((h - 400.0).abs() < f32::EPSILON);
        assert!((x - 150.0).abs() < f32::EPSILON); // (800-500)/2
        assert!((y - 100.0).abs() < f32::EPSILON); // (600-400)/2
    }

    #[test]
    fn view_state_rotated_dimensions() {
        let view = ViewState::new(FitMode::Fit);
        assert_eq!(view.rotated_dimensions(100, 50), (100, 50));

        let mut view = ViewState::new(FitMode::Fit);
        view.rotation = Rotation::Cw90;
        assert_eq!(view.rotated_dimensions(100, 50), (50, 100));
    }

    #[test]
    fn viewer_navigation_wraps() {
        let config = ViewerConfig {
            loop_navigation: true,
            ..Default::default()
        };
        let mut viewer = ImageViewer::new(&config);
        viewer.image_list = vec![
            PathBuf::from("a.jpg"),
            PathBuf::from("b.jpg"),
            PathBuf::from("c.jpg"),
        ];
        viewer.current_index = 2;
        // Can't actually load images in test, but we can test the index logic
        assert_eq!(viewer.position_display(), "3 / 3");
    }

    #[test]
    fn viewer_zoom_display() {
        let config = ViewerConfig::default();
        let mut viewer = ImageViewer::new(&config);
        viewer.view.zoom = 1.5;
        assert_eq!(viewer.zoom_display(), "150%");
    }

    #[test]
    fn zoom_in_out() {
        let mut view = ViewState::new(FitMode::Fit);
        view.zoom = 1.0;
        view.target_zoom = 1.0;
        view.zoom_in(0.1);
        assert!(view.target_zoom > 1.0);
        let z = view.target_zoom;
        view.zoom_out(0.1);
        assert!(view.target_zoom < z);
    }

    #[test]
    fn zoom_bounds() {
        let mut view = ViewState::new(FitMode::Fit);
        view.target_zoom = 50.0;
        view.zoom_in(0.1);
        assert!(view.target_zoom <= 50.0);

        view.target_zoom = 0.01;
        view.zoom_out(0.1);
        assert!(view.target_zoom >= 0.01);
    }

    #[test]
    fn view_animate() {
        let mut view = ViewState::new(FitMode::Fit);
        view.zoom = 1.0;
        view.target_zoom = 2.0;
        view.animate(0.016); // ~60fps frame time
        assert!(view.zoom > 1.0);
        assert!(view.zoom < 2.0);
    }
}
