//! EXIF metadata extraction and display formatting.
//!
//! Parses EXIF data from JPEG and TIFF images using the `kamadak-exif` crate.
//! Provides a structured `ImageMetadata` type with formatted display strings.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Extracted image metadata from EXIF tags.
#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    /// Camera make (e.g. "Canon", "Nikon").
    pub camera_make: Option<String>,
    /// Camera model (e.g. "EOS R5").
    pub camera_model: Option<String>,
    /// Lens model.
    pub lens_model: Option<String>,
    /// Focal length in mm (e.g. "50.0 mm").
    pub focal_length: Option<String>,
    /// Aperture (e.g. "f/2.8").
    pub aperture: Option<String>,
    /// Shutter speed (e.g. "1/250 s").
    pub shutter_speed: Option<String>,
    /// ISO sensitivity (e.g. "400").
    pub iso: Option<String>,
    /// Date/time the photo was taken.
    pub date_taken: Option<String>,
    /// Image orientation from EXIF.
    pub orientation: Option<u32>,
    /// GPS latitude in decimal degrees.
    pub gps_latitude: Option<f64>,
    /// GPS longitude in decimal degrees.
    pub gps_longitude: Option<f64>,
    /// Image width from EXIF (may differ from decoded).
    pub exif_width: Option<u32>,
    /// Image height from EXIF (may differ from decoded).
    pub exif_height: Option<u32>,
    /// Software used to create/edit.
    pub software: Option<String>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Exposure compensation.
    pub exposure_compensation: Option<String>,
    /// White balance mode.
    pub white_balance: Option<String>,
    /// Flash status.
    pub flash: Option<String>,
}

impl ImageMetadata {
    /// Extract EXIF metadata from an image file.
    ///
    /// Returns `None` if the file cannot be read or has no EXIF data.
    /// Does not fail on missing tags; each field is independently optional.
    pub fn from_file(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut reader = BufReader::new(file);
        let exif_reader = exif::Reader::new();
        let exif = exif_reader.read_from_container(&mut reader).ok()?;

        let mut meta = Self::default();

        // Camera info
        meta.camera_make = get_string_field(&exif, exif::Tag::Make);
        meta.camera_model = get_string_field(&exif, exif::Tag::Model);
        meta.lens_model = get_string_field(&exif, exif::Tag::LensModel);

        // Exposure settings
        meta.focal_length = get_rational_field(&exif, exif::Tag::FocalLength)
            .map(|v| format!("{v:.1} mm"));

        meta.aperture = get_rational_field(&exif, exif::Tag::FNumber)
            .map(|v| format!("f/{v:.1}"));

        meta.shutter_speed = get_shutter_speed(&exif);

        meta.iso = exif
            .get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());

        // Date
        meta.date_taken = get_string_field(&exif, exif::Tag::DateTimeOriginal);

        // Orientation
        meta.orientation = exif
            .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| match &f.value {
                exif::Value::Short(v) => v.first().map(|&x| u32::from(x)),
                _ => None,
            });

        // GPS
        let (lat, lon) = extract_gps(&exif);
        meta.gps_latitude = lat;
        meta.gps_longitude = lon;

        // Dimensions from EXIF
        meta.exif_width = get_uint_field(&exif, exif::Tag::PixelXDimension)
            .or_else(|| get_uint_field(&exif, exif::Tag::ImageWidth));
        meta.exif_height = get_uint_field(&exif, exif::Tag::PixelYDimension)
            .or_else(|| get_uint_field(&exif, exif::Tag::ImageLength));

        // Other
        meta.software = get_string_field(&exif, exif::Tag::Software);
        meta.copyright = get_string_field(&exif, exif::Tag::Copyright);
        meta.exposure_compensation = get_string_field(&exif, exif::Tag::ExposureBiasValue);
        meta.white_balance = exif
            .get_field(exif::Tag::WhiteBalance, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());
        meta.flash = exif
            .get_field(exif::Tag::Flash, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());

        Some(meta)
    }

    /// Format metadata as display lines for the info overlay.
    /// Returns pairs of (label, value).
    #[must_use]
    pub fn display_lines(&self) -> Vec<(String, String)> {
        let mut lines = Vec::new();

        if let Some(ref v) = self.camera_make {
            lines.push(("Make".into(), v.clone()));
        }
        if let Some(ref v) = self.camera_model {
            lines.push(("Model".into(), v.clone()));
        }
        if let Some(ref v) = self.lens_model {
            lines.push(("Lens".into(), v.clone()));
        }
        if let Some(ref v) = self.focal_length {
            lines.push(("Focal Length".into(), v.clone()));
        }
        if let Some(ref v) = self.aperture {
            lines.push(("Aperture".into(), v.clone()));
        }
        if let Some(ref v) = self.shutter_speed {
            lines.push(("Shutter".into(), v.clone()));
        }
        if let Some(ref v) = self.iso {
            lines.push(("ISO".into(), v.clone()));
        }
        if let Some(ref v) = self.date_taken {
            lines.push(("Date".into(), v.clone()));
        }
        if let (Some(lat), Some(lon)) = (self.gps_latitude, self.gps_longitude) {
            lines.push(("GPS".into(), format!("{lat:.6}, {lon:.6}")));
        }
        if let (Some(w), Some(h)) = (self.exif_width, self.exif_height) {
            lines.push(("EXIF Size".into(), format!("{w} x {h}")));
        }
        if let Some(ref v) = self.software {
            lines.push(("Software".into(), v.clone()));
        }
        if let Some(ref v) = self.copyright {
            lines.push(("Copyright".into(), v.clone()));
        }
        if let Some(ref v) = self.white_balance {
            lines.push(("White Balance".into(), v.clone()));
        }
        if let Some(ref v) = self.flash {
            lines.push(("Flash".into(), v.clone()));
        }

        lines
    }

    /// Returns true if any metadata field is populated.
    #[must_use]
    pub fn has_any(&self) -> bool {
        self.camera_make.is_some()
            || self.camera_model.is_some()
            || self.date_taken.is_some()
            || self.iso.is_some()
            || self.aperture.is_some()
    }
}

/// Get a string-valued EXIF field.
fn get_string_field(exif: &exif::Exif, tag: exif::Tag) -> Option<String> {
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string().trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Get a rational-valued EXIF field as f64.
fn get_rational_field(exif: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    exif.get_field(tag, exif::In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Rational(v) => v.first().map(|r| f64::from(r.num) / f64::from(r.denom)),
            _ => None,
        })
}

/// Get a uint-valued EXIF field.
fn get_uint_field(exif: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    exif.get_field(tag, exif::In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Long(v) => v.first().copied(),
            exif::Value::Short(v) => v.first().map(|&x| u32::from(x)),
            _ => None,
        })
}

/// Format shutter speed from `ExposureTime` rational.
fn get_shutter_speed(exif: &exif::Exif) -> Option<String> {
    let field = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)?;
    match &field.value {
        exif::Value::Rational(v) => {
            let r = v.first()?;
            if r.denom == 0 {
                return None;
            }
            if r.num == 0 {
                return Some("0 s".into());
            }
            if r.num >= r.denom {
                let secs = f64::from(r.num) / f64::from(r.denom);
                Some(format!("{secs:.1} s"))
            } else {
                // Express as fraction: 1/X
                let denom = r.denom / r.num;
                Some(format!("1/{denom} s"))
            }
        }
        _ => None,
    }
}

/// Extract GPS coordinates from EXIF as decimal degrees.
fn extract_gps(exif: &exif::Exif) -> (Option<f64>, Option<f64>) {
    let lat = gps_to_decimal(exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
    let lon = gps_to_decimal(exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);
    (lat, lon)
}

/// Convert GPS DMS (degrees-minutes-seconds) rational values to decimal degrees.
fn gps_to_decimal(exif: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag) -> Option<f64> {
    let field = exif.get_field(coord_tag, exif::In::PRIMARY)?;
    let rationals = match &field.value {
        exif::Value::Rational(v) if v.len() >= 3 => v,
        _ => return None,
    };

    let degrees = f64::from(rationals[0].num) / f64::from(rationals[0].denom);
    let minutes = f64::from(rationals[1].num) / f64::from(rationals[1].denom);
    let seconds = f64::from(rationals[2].num) / f64::from(rationals[2].denom);

    let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;

    // Check reference for sign (S and W are negative)
    if let Some(ref_field) = exif.get_field(ref_tag, exif::In::PRIMARY) {
        let ref_str = ref_field.display_value().to_string();
        if ref_str.contains('S') || ref_str.contains('W') {
            decimal = -decimal;
        }
    }

    Some(decimal)
}

/// File-level metadata that doesn't require EXIF (always available).
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Last modified time (as string).
    pub modified: Option<String>,
    /// Image dimensions (width x height) from decoding.
    pub width: u32,
    /// Image height from decoding.
    pub height: u32,
    /// Image format name.
    pub format: String,
}

impl FileInfo {
    /// Create file info from a path and decoded image dimensions.
    pub fn from_path(path: &Path, width: u32, height: u32, format: &str) -> Self {
        let name = path
            .file_name()
            .map_or_else(|| "unknown".into(), |n| n.to_string_lossy().into_owned());

        let metadata = std::fs::metadata(path).ok();
        let size = metadata.as_ref().map_or(0, |m| m.len());
        let modified = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let duration = t
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = duration.as_secs();
            // Simple timestamp formatting
            format_timestamp(secs)
        });

        Self {
            name,
            size,
            modified,
            width,
            height,
            format: format.to_owned(),
        }
    }

    /// Format file size as human-readable string.
    #[must_use]
    pub fn size_display(&self) -> String {
        format_file_size(self.size)
    }

    /// Format dimensions as "WxH".
    #[must_use]
    pub fn dimensions_display(&self) -> String {
        format!("{} x {}", self.width, self.height)
    }

    /// Return display lines for file info overlay.
    #[must_use]
    pub fn display_lines(&self) -> Vec<(String, String)> {
        let mut lines = vec![
            ("File".into(), self.name.clone()),
            ("Size".into(), self.size_display()),
            ("Dimensions".into(), self.dimensions_display()),
            ("Format".into(), self.format.clone()),
        ];
        if let Some(ref modified) = self.modified {
            lines.push(("Modified".into(), modified.clone()));
        }
        lines
    }
}

/// Format byte count as human-readable file size.
#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Simple timestamp formatter (YYYY-MM-DD HH:MM:SS from unix seconds).
fn format_timestamp(unix_secs: u64) -> String {
    // Simple date formatting without chrono dependency.
    // We compute year/month/day from days since epoch.
    let secs_in_day: u64 = 86400;
    let mut days = unix_secs / secs_in_day;
    let time_of_day = unix_secs % secs_in_day;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since 1970-01-01 to Y-M-D (simplified civil calendar)
    let mut year: u64 = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month: u64 = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    let day = days + 1;

    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_size_formatting() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1500), "1.5 KB");
        assert_eq!(format_file_size(1_500_000), "1.4 MB");
        assert_eq!(format_file_size(1_500_000_000), "1.4 GB");
    }

    #[test]
    fn metadata_default_has_no_fields() {
        let meta = ImageMetadata::default();
        assert!(!meta.has_any());
        assert!(meta.display_lines().is_empty());
    }

    #[test]
    fn metadata_with_fields_has_display_lines() {
        let meta = ImageMetadata {
            camera_make: Some("Canon".into()),
            camera_model: Some("EOS R5".into()),
            aperture: Some("f/2.8".into()),
            iso: Some("400".into()),
            ..Default::default()
        };
        assert!(meta.has_any());
        let lines = meta.display_lines();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].0, "Make");
        assert_eq!(lines[0].1, "Canon");
    }

    #[test]
    fn file_info_display() {
        let info = FileInfo {
            name: "test.jpg".into(),
            size: 2_500_000,
            modified: Some("2024-01-15 10:30:00".into()),
            width: 4000,
            height: 3000,
            format: "JPEG".into(),
        };
        assert_eq!(info.size_display(), "2.4 MB");
        assert_eq!(info.dimensions_display(), "4000 x 3000");
        assert_eq!(info.display_lines().len(), 5);
    }

    #[test]
    fn timestamp_formatting() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        let ts = format_timestamp(1_704_067_200);
        assert!(ts.starts_with("2024-01-01"));
    }

    #[test]
    fn gps_display_format() {
        let meta = ImageMetadata {
            gps_latitude: Some(48.8566),
            gps_longitude: Some(2.3522),
            ..Default::default()
        };
        let lines = meta.display_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "GPS");
        assert!(lines[0].1.contains("48.856600"));
    }
}
