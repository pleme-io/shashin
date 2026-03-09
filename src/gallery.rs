//! Gallery view — thumbnail grid with directory scanning, sorting, and filtering.
//!
//! Provides `Gallery` which manages a list of image entries from a directory,
//! generates thumbnails, and tracks grid selection state for keyboard navigation.

use crate::config::{GalleryConfig, SortOrder};
use std::path::{Path, PathBuf};

/// Supported image file extensions for gallery scanning.
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "avif", "ico",
];

/// A single image entry in the gallery.
#[derive(Debug, Clone)]
pub struct GalleryEntry {
    /// Full path to the image file.
    pub path: PathBuf,
    /// File name for display.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modified timestamp (unix seconds).
    pub modified: u64,
    /// Whether a thumbnail has been generated.
    pub thumbnail_ready: bool,
    /// Decoded thumbnail RGBA data (if loaded).
    pub thumbnail_data: Option<ThumbnailData>,
    /// Whether this entry is marked for batch operations.
    pub marked: bool,
}

/// RGBA thumbnail pixel data.
#[derive(Debug, Clone)]
pub struct ThumbnailData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Gallery state managing a directory of images.
#[derive(Debug)]
pub struct Gallery {
    /// All entries from the scanned directory.
    entries: Vec<GalleryEntry>,
    /// Currently selected entry index.
    selected: usize,
    /// Number of visible columns in the grid.
    columns: usize,
    /// Number of visible rows.
    visible_rows: usize,
    /// Scroll offset in rows.
    scroll_offset: usize,
    /// Current sort order.
    sort_order: SortOrder,
    /// Whether sort is reversed.
    sort_reverse: bool,
    /// The directory being viewed.
    directory: PathBuf,
    /// Thumbnail size for generation.
    thumbnail_size: u32,
    /// Search/filter query.
    filter_query: String,
    /// Indices of entries matching the filter (empty = all shown).
    filtered_indices: Vec<usize>,
}

impl Gallery {
    /// Create a new empty gallery.
    #[must_use]
    pub fn new(config: &GalleryConfig) -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            columns: if config.columns == 0 {
                4
            } else {
                config.columns as usize
            },
            visible_rows: 4,
            scroll_offset: 0,
            sort_order: config.sort,
            sort_reverse: config.sort_reverse,
            directory: PathBuf::new(),
            thumbnail_size: config.thumbnail_size,
            filter_query: String::new(),
            filtered_indices: Vec::new(),
        }
    }

    /// Scan a directory for image files and populate the gallery.
    pub fn scan_directory(&mut self, dir: &Path) -> Result<usize, std::io::Error> {
        self.directory = dir.to_owned();
        self.entries.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        self.filter_query.clear();
        self.filtered_indices.clear();

        let entries = std::fs::read_dir(dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_lowercase);

            if let Some(ext) = ext {
                if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map_or(0, |m| m.len());
                    let modified = metadata
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs());

                    let name = path
                        .file_name()
                        .map_or_else(|| String::new(), |n| n.to_string_lossy().into_owned());

                    self.entries.push(GalleryEntry {
                        path,
                        name,
                        size,
                        modified,
                        thumbnail_ready: false,
                        thumbnail_data: None,
                        marked: false,
                    });
                }
            }
        }

        self.apply_sort();
        Ok(self.entries.len())
    }

    /// Sort entries according to current sort settings.
    fn apply_sort(&mut self) {
        match self.sort_order {
            SortOrder::Name => self.entries.sort_by(|a, b| {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }),
            SortOrder::Date => self.entries.sort_by(|a, b| a.modified.cmp(&b.modified)),
            SortOrder::Size => self.entries.sort_by(|a, b| a.size.cmp(&b.size)),
            SortOrder::Dimensions => {
                // Dimensions sort falls back to name until thumbnails load
                self.entries.sort_by(|a, b| {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                });
            }
        }
        if self.sort_reverse {
            self.entries.reverse();
        }
    }

    /// Cycle to the next sort order.
    pub fn cycle_sort(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Name => SortOrder::Date,
            SortOrder::Date => SortOrder::Size,
            SortOrder::Size => SortOrder::Dimensions,
            SortOrder::Dimensions => SortOrder::Name,
        };
        self.apply_sort();
        tracing::info!("gallery sort: {:?}", self.sort_order);
    }

    /// Toggle sort direction.
    pub fn toggle_sort_reverse(&mut self) {
        self.sort_reverse = !self.sort_reverse;
        self.apply_sort();
    }

    /// Apply a text filter to entries.
    pub fn set_filter(&mut self, query: &str) {
        self.filter_query = query.to_lowercase();
        if self.filter_query.is_empty() {
            self.filtered_indices.clear();
        } else {
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&self.filter_query))
                .map(|(i, _)| i)
                .collect();
        }
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Clear the active filter.
    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.filtered_indices.clear();
    }

    // -- Navigation --

    /// Get the effective entry list (filtered or all).
    fn effective_len(&self) -> usize {
        if self.filtered_indices.is_empty() && self.filter_query.is_empty() {
            self.entries.len()
        } else {
            self.filtered_indices.len()
        }
    }

    /// Map selected index to actual entry index.
    fn actual_index(&self, visual_idx: usize) -> Option<usize> {
        if self.filtered_indices.is_empty() && self.filter_query.is_empty() {
            if visual_idx < self.entries.len() {
                Some(visual_idx)
            } else {
                None
            }
        } else {
            self.filtered_indices.get(visual_idx).copied()
        }
    }

    /// Move selection right by one cell.
    pub fn move_right(&mut self) {
        let len = self.effective_len();
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
            self.selected += 1;
            self.ensure_visible();
        }
    }

    /// Move selection left by one cell.
    pub fn move_left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
        }
    }

    /// Move selection down by one row.
    pub fn move_down(&mut self) {
        let len = self.effective_len();
        if len == 0 {
            return;
        }
        let next = self.selected + self.columns;
        if next < len {
            self.selected = next;
            self.ensure_visible();
        }
    }

    /// Move selection up by one row.
    pub fn move_up(&mut self) {
        if self.selected >= self.columns {
            self.selected -= self.columns;
            self.ensure_visible();
        }
    }

    /// Jump to first entry.
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.ensure_visible();
    }

    /// Jump to last entry.
    pub fn select_last(&mut self) {
        let len = self.effective_len();
        if len > 0 {
            self.selected = len - 1;
            self.ensure_visible();
        }
    }

    /// Toggle mark on the selected entry.
    pub fn toggle_mark(&mut self) {
        if let Some(idx) = self.actual_index(self.selected) {
            if let Some(entry) = self.entries.get_mut(idx) {
                entry.marked = !entry.marked;
            }
        }
    }

    /// Ensure selected item is within the visible scroll region.
    fn ensure_visible(&mut self) {
        let row = self.selected / self.columns;
        if row < self.scroll_offset {
            self.scroll_offset = row;
        } else if row >= self.scroll_offset + self.visible_rows {
            self.scroll_offset = row + 1 - self.visible_rows;
        }
    }

    /// Update visible geometry based on window size.
    pub fn update_layout(&mut self, window_width: u32, window_height: u32) {
        let cell_size = self.thumbnail_size + 20; // padding
        if cell_size > 0 {
            self.columns = (window_width / cell_size).max(1) as usize;
            self.visible_rows = (window_height / cell_size).max(1) as usize;
        }
    }

    // -- Accessors --

    /// Get the currently selected entry.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&GalleryEntry> {
        self.actual_index(self.selected)
            .and_then(|idx| self.entries.get(idx))
    }

    /// Get the path of the selected entry.
    #[must_use]
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_entry().map(|e| e.path.as_path())
    }

    /// Get the selected visual index.
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Get the number of columns.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Get the scroll offset in rows.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get the visible row count.
    #[must_use]
    pub fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    /// Get total entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if gallery is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the directory being viewed.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Get all entries.
    #[must_use]
    pub fn entries(&self) -> &[GalleryEntry] {
        &self.entries
    }

    /// Get entries visible in the current scroll viewport.
    #[must_use]
    pub fn visible_entries(&self) -> Vec<(usize, &GalleryEntry)> {
        let start = self.scroll_offset * self.columns;
        let end = ((self.scroll_offset + self.visible_rows) * self.columns)
            .min(self.effective_len());

        (start..end)
            .filter_map(|vis_idx| {
                self.actual_index(vis_idx)
                    .and_then(|idx| self.entries.get(idx).map(|e| (vis_idx, e)))
            })
            .collect()
    }

    /// Get all image paths in the gallery (for navigation from viewer).
    #[must_use]
    pub fn all_paths(&self) -> Vec<PathBuf> {
        self.entries.iter().map(|e| e.path.clone()).collect()
    }

    /// Get the current sort order.
    #[must_use]
    pub fn sort_order(&self) -> SortOrder {
        self.sort_order
    }

    /// Get the thumbnail size.
    #[must_use]
    pub fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }

    /// Get the current filter query.
    #[must_use]
    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }

    /// Get the number of marked entries.
    #[must_use]
    pub fn marked_count(&self) -> usize {
        self.entries.iter().filter(|e| e.marked).count()
    }

    /// Store a generated thumbnail for an entry by path.
    pub fn set_thumbnail(&mut self, path: &Path, data: ThumbnailData) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == path) {
            entry.thumbnail_data = Some(data);
            entry.thumbnail_ready = true;
        }
    }
}

/// Generate a thumbnail from an image file.
///
/// Returns RGBA pixel data scaled to fit within `max_size x max_size`.
pub fn generate_thumbnail(path: &Path, max_size: u32) -> Result<ThumbnailData, image::ImageError> {
    let img = image::open(path)?;
    let thumb = img.thumbnail(max_size, max_size);
    let rgba = thumb.to_rgba8();
    Ok(ThumbnailData {
        width: rgba.width(),
        height: rgba.height(),
        data: rgba.into_raw(),
    })
}

/// Check if a file path has a supported image extension.
#[must_use]
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Collect all image file paths from a directory, sorted by name.
pub fn list_images_in_dir(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .filter(|e| e.path().is_file() && is_image_file(&e.path()))
        .map(|e| e.path())
        .collect();
    paths.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
    });
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GalleryConfig;

    #[test]
    fn is_image_file_accepts_valid_extensions() {
        assert!(is_image_file(Path::new("photo.jpg")));
        assert!(is_image_file(Path::new("photo.JPEG")));
        assert!(is_image_file(Path::new("photo.png")));
        assert!(is_image_file(Path::new("photo.webp")));
        assert!(is_image_file(Path::new("photo.gif")));
        assert!(is_image_file(Path::new("photo.bmp")));
        assert!(is_image_file(Path::new("photo.tiff")));
        assert!(is_image_file(Path::new("photo.avif")));
    }

    #[test]
    fn is_image_file_rejects_non_images() {
        assert!(!is_image_file(Path::new("document.pdf")));
        assert!(!is_image_file(Path::new("video.mp4")));
        assert!(!is_image_file(Path::new("noext")));
    }

    #[test]
    fn gallery_new_is_empty() {
        let config = GalleryConfig::default();
        let gallery = Gallery::new(&config);
        assert!(gallery.is_empty());
        assert_eq!(gallery.len(), 0);
        assert!(gallery.selected_entry().is_none());
    }

    #[test]
    fn gallery_navigation() {
        let config = GalleryConfig {
            columns: 3,
            ..Default::default()
        };
        let mut gallery = Gallery::new(&config);
        // Simulate entries
        for i in 0..9 {
            gallery.entries.push(GalleryEntry {
                path: PathBuf::from(format!("img{i}.jpg")),
                name: format!("img{i}.jpg"),
                size: 1000,
                modified: 0,
                thumbnail_ready: false,
                thumbnail_data: None,
                marked: false,
            });
        }

        assert_eq!(gallery.selected_index(), 0);
        gallery.move_right();
        assert_eq!(gallery.selected_index(), 1);
        gallery.move_down();
        assert_eq!(gallery.selected_index(), 4); // 1 + 3
        gallery.move_left();
        assert_eq!(gallery.selected_index(), 3);
        gallery.move_up();
        assert_eq!(gallery.selected_index(), 0);
    }

    #[test]
    fn gallery_select_first_last() {
        let config = GalleryConfig::default();
        let mut gallery = Gallery::new(&config);
        for i in 0..5 {
            gallery.entries.push(GalleryEntry {
                path: PathBuf::from(format!("img{i}.jpg")),
                name: format!("img{i}.jpg"),
                size: 1000,
                modified: 0,
                thumbnail_ready: false,
                thumbnail_data: None,
                marked: false,
            });
        }
        gallery.select_last();
        assert_eq!(gallery.selected_index(), 4);
        gallery.select_first();
        assert_eq!(gallery.selected_index(), 0);
    }

    #[test]
    fn gallery_filter() {
        let config = GalleryConfig::default();
        let mut gallery = Gallery::new(&config);
        gallery.entries.push(GalleryEntry {
            path: PathBuf::from("sunset.jpg"),
            name: "sunset.jpg".into(),
            size: 1000,
            modified: 0,
            thumbnail_ready: false,
            thumbnail_data: None,
            marked: false,
        });
        gallery.entries.push(GalleryEntry {
            path: PathBuf::from("mountain.png"),
            name: "mountain.png".into(),
            size: 2000,
            modified: 0,
            thumbnail_ready: false,
            thumbnail_data: None,
            marked: false,
        });

        gallery.set_filter("sun");
        assert_eq!(gallery.effective_len(), 1);
        gallery.clear_filter();
        assert_eq!(gallery.effective_len(), 2);
    }

    #[test]
    fn gallery_toggle_mark() {
        let config = GalleryConfig::default();
        let mut gallery = Gallery::new(&config);
        gallery.entries.push(GalleryEntry {
            path: PathBuf::from("img.jpg"),
            name: "img.jpg".into(),
            size: 1000,
            modified: 0,
            thumbnail_ready: false,
            thumbnail_data: None,
            marked: false,
        });
        assert_eq!(gallery.marked_count(), 0);
        gallery.toggle_mark();
        assert_eq!(gallery.marked_count(), 1);
        gallery.toggle_mark();
        assert_eq!(gallery.marked_count(), 0);
    }
}
