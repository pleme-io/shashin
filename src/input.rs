//! Keyboard and mouse input handling with modal keybinding support.
//!
//! Maps `madori::AppEvent` to semantic `Action` values based on the current
//! application mode (Viewer, Gallery, Effects).

use madori::event::{KeyCode, Modifiers};

/// Application modes that determine keybinding context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Viewing a single image.
    Viewer,
    /// Browsing the thumbnail gallery.
    Gallery,
    /// Slideshow is running (subset of viewer controls).
    Slideshow,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Viewer
    }
}

/// Semantic actions the application can perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // -- Navigation --
    NextImage,
    PrevImage,
    FirstImage,
    LastImage,

    // -- Zoom --
    ZoomIn,
    ZoomOut,
    ZoomActual,
    FitToWindow,
    FillWindow,

    // -- Transform --
    RotateCw,
    RotateCcw,
    FlipHorizontal,
    FlipVertical,

    // -- Pan (shift + hjkl) --
    PanLeft,
    PanRight,
    PanUp,
    PanDown,

    // -- View --
    ToggleInfo,
    ToggleSlideshow,
    SwitchToGallery,
    SwitchToViewer,

    // -- Gallery navigation --
    GalleryLeft,
    GalleryRight,
    GalleryUp,
    GalleryDown,
    GalleryOpen,
    GalleryCycleSort,
    GalleryToggleMark,

    // -- App --
    Quit,
    Escape,

    /// No action mapped.
    None,
}

/// Map a key event to a semantic action based on the current mode.
#[must_use]
pub fn map_key(key: &KeyCode, mods: &Modifiers, mode: Mode) -> Action {
    match mode {
        Mode::Viewer | Mode::Slideshow => map_viewer_key(key, mods),
        Mode::Gallery => map_gallery_key(key, mods),
    }
}

/// Map a key press in viewer mode.
fn map_viewer_key(key: &KeyCode, mods: &Modifiers) -> Action {
    // Shift + direction = pan
    if mods.shift {
        return match key {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::FlipHorizontal,
            KeyCode::Left => Action::PanLeft,
            KeyCode::Right => Action::PanRight,
            KeyCode::Up => Action::PanUp,
            KeyCode::Down => Action::PanDown,
            _ => Action::None,
        };
    }

    match key {
        // Navigation
        KeyCode::Char('j') | KeyCode::Char('n') | KeyCode::Right => Action::NextImage,
        KeyCode::Char('k') | KeyCode::Char('p') | KeyCode::Left => Action::PrevImage,

        // Zoom
        KeyCode::Char('+') | KeyCode::Char('=') => Action::ZoomIn,
        KeyCode::Char('-') => Action::ZoomOut,
        KeyCode::Char('0') => Action::ZoomActual,
        KeyCode::Char('f') => Action::FitToWindow,
        KeyCode::Char('F') => Action::FillWindow,

        // Transform
        KeyCode::Char('r') => Action::RotateCw,
        KeyCode::Char('R') => Action::RotateCcw,
        KeyCode::Char('h') => Action::FlipHorizontal,
        KeyCode::Char('H') => Action::FlipVertical,

        // View
        KeyCode::Char('i') => Action::ToggleInfo,
        KeyCode::Space => Action::ToggleSlideshow,
        KeyCode::Char('g') => Action::SwitchToGallery,

        // Navigation shortcuts
        KeyCode::Home => Action::FirstImage,
        KeyCode::End => Action::LastImage,

        // App
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Escape => Action::Escape,

        _ => Action::None,
    }
}

/// Map a key press in gallery mode.
fn map_gallery_key(key: &KeyCode, mods: &Modifiers) -> Action {
    if mods.shift {
        return match key {
            KeyCode::Char('G') => Action::LastImage,
            _ => Action::None,
        };
    }

    match key {
        // Grid navigation
        KeyCode::Char('h') | KeyCode::Left => Action::GalleryLeft,
        KeyCode::Char('l') | KeyCode::Right => Action::GalleryRight,
        KeyCode::Char('j') | KeyCode::Down => Action::GalleryDown,
        KeyCode::Char('k') | KeyCode::Up => Action::GalleryUp,

        // Open selected
        KeyCode::Enter => Action::GalleryOpen,

        // Sort
        KeyCode::Char('s') => Action::GalleryCycleSort,

        // Mark
        KeyCode::Char('m') => Action::GalleryToggleMark,

        // Jump
        KeyCode::Char('g') => Action::FirstImage,
        KeyCode::Char('G') => Action::LastImage,
        KeyCode::Home => Action::FirstImage,
        KeyCode::End => Action::LastImage,

        // Back to viewer
        KeyCode::Escape => Action::SwitchToViewer,
        KeyCode::Char('q') => Action::Quit,

        _ => Action::None,
    }
}

/// Map a scroll event to a zoom action.
#[must_use]
pub fn map_scroll(dy: f64) -> Action {
    if dy > 0.0 {
        Action::ZoomIn
    } else if dy < 0.0 {
        Action::ZoomOut
    } else {
        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_mods() -> Modifiers {
        Modifiers::default()
    }

    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Default::default()
        }
    }

    #[test]
    fn viewer_navigation_keys() {
        assert_eq!(
            map_key(&KeyCode::Char('j'), &no_mods(), Mode::Viewer),
            Action::NextImage
        );
        assert_eq!(
            map_key(&KeyCode::Char('k'), &no_mods(), Mode::Viewer),
            Action::PrevImage
        );
        assert_eq!(
            map_key(&KeyCode::Char('n'), &no_mods(), Mode::Viewer),
            Action::NextImage
        );
        assert_eq!(
            map_key(&KeyCode::Char('p'), &no_mods(), Mode::Viewer),
            Action::PrevImage
        );
    }

    #[test]
    fn viewer_zoom_keys() {
        assert_eq!(
            map_key(&KeyCode::Char('+'), &no_mods(), Mode::Viewer),
            Action::ZoomIn
        );
        assert_eq!(
            map_key(&KeyCode::Char('-'), &no_mods(), Mode::Viewer),
            Action::ZoomOut
        );
        assert_eq!(
            map_key(&KeyCode::Char('0'), &no_mods(), Mode::Viewer),
            Action::ZoomActual
        );
    }

    #[test]
    fn viewer_fit_modes() {
        assert_eq!(
            map_key(&KeyCode::Char('f'), &no_mods(), Mode::Viewer),
            Action::FitToWindow
        );
        assert_eq!(
            map_key(&KeyCode::Char('F'), &no_mods(), Mode::Viewer),
            Action::FillWindow
        );
    }

    #[test]
    fn viewer_transform_keys() {
        assert_eq!(
            map_key(&KeyCode::Char('r'), &no_mods(), Mode::Viewer),
            Action::RotateCw
        );
        assert_eq!(
            map_key(&KeyCode::Char('R'), &no_mods(), Mode::Viewer),
            Action::RotateCcw
        );
    }

    #[test]
    fn viewer_toggle_keys() {
        assert_eq!(
            map_key(&KeyCode::Char('i'), &no_mods(), Mode::Viewer),
            Action::ToggleInfo
        );
        assert_eq!(
            map_key(&KeyCode::Space, &no_mods(), Mode::Viewer),
            Action::ToggleSlideshow
        );
        assert_eq!(
            map_key(&KeyCode::Char('g'), &no_mods(), Mode::Viewer),
            Action::SwitchToGallery
        );
    }

    #[test]
    fn viewer_quit() {
        assert_eq!(
            map_key(&KeyCode::Char('q'), &no_mods(), Mode::Viewer),
            Action::Quit
        );
        assert_eq!(
            map_key(&KeyCode::Escape, &no_mods(), Mode::Viewer),
            Action::Escape
        );
    }

    #[test]
    fn viewer_shift_flip() {
        assert_eq!(
            map_key(&KeyCode::Char('h'), &shift(), Mode::Viewer),
            Action::FlipHorizontal
        );
        assert_eq!(
            map_key(&KeyCode::Char('H'), &shift(), Mode::Viewer),
            Action::FlipHorizontal
        );
    }

    #[test]
    fn gallery_navigation_keys() {
        assert_eq!(
            map_key(&KeyCode::Char('h'), &no_mods(), Mode::Gallery),
            Action::GalleryLeft
        );
        assert_eq!(
            map_key(&KeyCode::Char('j'), &no_mods(), Mode::Gallery),
            Action::GalleryDown
        );
        assert_eq!(
            map_key(&KeyCode::Char('k'), &no_mods(), Mode::Gallery),
            Action::GalleryUp
        );
        assert_eq!(
            map_key(&KeyCode::Char('l'), &no_mods(), Mode::Gallery),
            Action::GalleryRight
        );
    }

    #[test]
    fn gallery_open() {
        assert_eq!(
            map_key(&KeyCode::Enter, &no_mods(), Mode::Gallery),
            Action::GalleryOpen
        );
    }

    #[test]
    fn gallery_sort_and_mark() {
        assert_eq!(
            map_key(&KeyCode::Char('s'), &no_mods(), Mode::Gallery),
            Action::GalleryCycleSort
        );
        assert_eq!(
            map_key(&KeyCode::Char('m'), &no_mods(), Mode::Gallery),
            Action::GalleryToggleMark
        );
    }

    #[test]
    fn scroll_mapping() {
        assert_eq!(map_scroll(1.0), Action::ZoomIn);
        assert_eq!(map_scroll(-1.0), Action::ZoomOut);
        assert_eq!(map_scroll(0.0), Action::None);
    }

    #[test]
    fn unknown_keys_map_to_none() {
        assert_eq!(
            map_key(&KeyCode::F(12), &no_mods(), Mode::Viewer),
            Action::None
        );
        assert_eq!(
            map_key(&KeyCode::Tab, &no_mods(), Mode::Gallery),
            Action::None
        );
    }
}
