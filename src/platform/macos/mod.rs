//! macOS screen capture implementation using ScreenCaptureKit / CGWindow APIs.

use crate::platform::{CapturedImage, DisplayInfo, ScreenCapture};

/// macOS screen capture backend.
pub struct MacOSScreenCapture;

impl MacOSScreenCapture {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapture for MacOSScreenCapture {
    fn capture_fullscreen(&self) -> Result<CapturedImage, Box<dyn std::error::Error>> {
        // TODO: implement via ScreenCaptureKit or CGDisplayCreateImage
        tracing::warn!("fullscreen capture not yet implemented");
        Ok(CapturedImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        })
    }

    fn capture_region(
        &self,
        _x: u32,
        _y: u32,
        _w: u32,
        _h: u32,
    ) -> Result<CapturedImage, Box<dyn std::error::Error>> {
        // TODO: implement via CGWindowListCreateImage with region rect
        tracing::warn!("region capture not yet implemented");
        Ok(CapturedImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        })
    }

    fn capture_window(
        &self,
        _window_id: u64,
    ) -> Result<CapturedImage, Box<dyn std::error::Error>> {
        // TODO: implement via CGWindowListCreateImage with window ID
        tracing::warn!("window capture not yet implemented");
        Ok(CapturedImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        })
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Box<dyn std::error::Error>> {
        // TODO: implement via CGGetActiveDisplayList
        tracing::warn!("display listing not yet implemented");
        Ok(Vec::new())
    }
}
