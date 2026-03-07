//! Platform abstraction traits for screen capture.
//!
//! Each platform provides a `ScreenCapture` implementation that can capture
//! fullscreen, region, or window screenshots using native APIs.

#[cfg(target_os = "macos")]
pub mod macos;

/// A captured screenshot image.
#[derive(Debug, Clone)]
pub struct CapturedImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Raw RGBA pixel data.
    pub data: Vec<u8>,
}

/// Information about a display/monitor.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Platform-specific display identifier.
    pub id: u64,
    /// Human-readable display name.
    pub name: String,
    /// Display width in pixels.
    pub width: u32,
    /// Display height in pixels.
    pub height: u32,
    /// Display scale factor (e.g. 2.0 for Retina).
    pub scale: f32,
}

/// Screen capture backend.
pub trait ScreenCapture: Send + Sync {
    /// Capture the entire primary display.
    fn capture_fullscreen(&self) -> Result<CapturedImage, Box<dyn std::error::Error>>;

    /// Capture a rectangular region of the screen.
    fn capture_region(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Result<CapturedImage, Box<dyn std::error::Error>>;

    /// Capture a specific window by its platform ID.
    fn capture_window(&self, window_id: u64) -> Result<CapturedImage, Box<dyn std::error::Error>>;

    /// List all available displays.
    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Box<dyn std::error::Error>>;
}

/// Create a platform-specific screen capture backend.
pub fn create_capture() -> Box<dyn ScreenCapture> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSScreenCapture::new())
    }
    #[cfg(not(target_os = "macos"))]
    {
        panic!("screen capture not implemented for this platform")
    }
}
