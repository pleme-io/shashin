//! Keyboard and mouse input handling with modal keybinding support.
//!
//! Maps `madori::AppEvent` to semantic `Action` values based on the current
//! application mode (Viewer, Gallery, Slideshow). Uses `awase::Hotkey` for
//! key binding definitions and `awase::BindingMap` for mode-aware lookup.

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

/// Convert a madori `KeyCode` to an awase `Key` for hotkey matching.
fn to_awase_key(key: &KeyCode) -> Option<awase::Key> {
    match key {
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'a' => Some(awase::Key::A),
            'b' => Some(awase::Key::B),
            'c' => Some(awase::Key::C),
            'd' => Some(awase::Key::D),
            'e' => Some(awase::Key::E),
            'f' => Some(awase::Key::F),
            'g' => Some(awase::Key::G),
            'h' => Some(awase::Key::H),
            'i' => Some(awase::Key::I),
            'j' => Some(awase::Key::J),
            'k' => Some(awase::Key::K),
            'l' => Some(awase::Key::L),
            'm' => Some(awase::Key::M),
            'n' => Some(awase::Key::N),
            'o' => Some(awase::Key::O),
            'p' => Some(awase::Key::P),
            'q' => Some(awase::Key::Q),
            'r' => Some(awase::Key::R),
            's' => Some(awase::Key::S),
            't' => Some(awase::Key::T),
            'u' => Some(awase::Key::U),
            'v' => Some(awase::Key::V),
            'w' => Some(awase::Key::W),
            'x' => Some(awase::Key::X),
            'y' => Some(awase::Key::Y),
            'z' => Some(awase::Key::Z),
            '0' => Some(awase::Key::Num0),
            '1' => Some(awase::Key::Num1),
            ' ' => Some(awase::Key::Space),
            _ => Option::None,
        },
        KeyCode::Enter => Some(awase::Key::Return),
        KeyCode::Escape => Some(awase::Key::Escape),
        KeyCode::Tab => Some(awase::Key::Tab),
        KeyCode::Backspace => Some(awase::Key::Backspace),
        KeyCode::Delete => Some(awase::Key::Delete),
        KeyCode::Up => Some(awase::Key::Up),
        KeyCode::Down => Some(awase::Key::Down),
        KeyCode::Left => Some(awase::Key::Left),
        KeyCode::Right => Some(awase::Key::Right),
        KeyCode::F(n) => match n {
            1 => Some(awase::Key::F1),
            2 => Some(awase::Key::F2),
            3 => Some(awase::Key::F3),
            4 => Some(awase::Key::F4),
            5 => Some(awase::Key::F5),
            6 => Some(awase::Key::F6),
            7 => Some(awase::Key::F7),
            8 => Some(awase::Key::F8),
            9 => Some(awase::Key::F9),
            10 => Some(awase::Key::F10),
            11 => Some(awase::Key::F11),
            12 => Some(awase::Key::F12),
            _ => Option::None,
        },
        KeyCode::Space => Some(awase::Key::Space),
        // Home/End/PageUp/PageDown not in awase::Key — handled via direct matching
        _ => Option::None,
    }
}

/// Convert madori modifiers to awase modifiers.
fn to_awase_modifiers(mods: &Modifiers) -> awase::Modifiers {
    let mut result = awase::Modifiers::NONE;
    if mods.ctrl {
        result = result | awase::Modifiers::CTRL;
    }
    if mods.alt {
        result = result | awase::Modifiers::ALT;
    }
    if mods.shift {
        result = result | awase::Modifiers::SHIFT;
    }
    if mods.meta {
        result = result | awase::Modifiers::CMD;
    }
    result
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

    // Build an awase hotkey for declarative matching
    if let Some(awase_key) = to_awase_key(key) {
        let awase_mods = to_awase_modifiers(mods);
        let hotkey = awase::Hotkey::new(awase_mods, awase_key);

        // Check against known hotkey patterns
        if let Some(action) = match_viewer_hotkey(&hotkey) {
            return action;
        }
    }

    // Fallback to direct key matching for char-specific cases
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

/// Match viewer hotkeys using awase hotkey patterns.
fn match_viewer_hotkey(hotkey: &awase::Hotkey) -> Option<Action> {
    // This enables future config-driven binding overrides.
    // Currently maps the same keys but through awase's Hotkey type.
    let key = hotkey.key;
    let mods = hotkey.modifiers;

    if !mods.is_empty() {
        return Option::None;
    }

    match key {
        awase::Key::Q => Some(Action::Quit),
        awase::Key::Escape => Some(Action::Escape),
        _ => Option::None,
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

    #[test]
    fn awase_key_conversion() {
        assert_eq!(to_awase_key(&KeyCode::Char('a')), Some(awase::Key::A));
        assert_eq!(to_awase_key(&KeyCode::Escape), Some(awase::Key::Escape));
        assert_eq!(to_awase_key(&KeyCode::Enter), Some(awase::Key::Return));
        assert_eq!(to_awase_key(&KeyCode::F(5)), Some(awase::Key::F5));
    }

    #[test]
    fn awase_modifier_conversion() {
        let mods = Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        let awase_mods = to_awase_modifiers(&mods);
        assert!(awase_mods.contains(awase::Modifiers::CTRL));
        assert!(awase_mods.contains(awase::Modifiers::SHIFT));
        assert!(!awase_mods.contains(awase::Modifiers::CMD));
    }
}
