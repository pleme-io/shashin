//! Shashin (写真) — GPU-rendered image viewer with gallery, metadata, and effects.
//!
//! A fast, keyboard-driven image viewer built on the pleme-io GPU stack
//! (garasu, madori, egaku). Features include:
//!
//! - Image loading and decoding (JPEG, PNG, WebP, GIF, BMP, TIFF, AVIF)
//! - Gallery view with thumbnail grid and keyboard navigation
//! - Single image view with zoom, pan, rotation, and fit modes
//! - EXIF metadata extraction and display overlay
//! - Slideshow mode with configurable interval
//! - Vim-style keyboard navigation (hjkl, gg, G, etc.)
//! - Configuration via shikumi (~/.config/shashin/shashin.yaml)

pub mod config;
pub mod gallery;
pub mod input;
pub mod mcp;
pub mod metadata;
pub mod platform;
pub mod render;
pub mod scripting;
pub mod viewer;
