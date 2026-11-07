//! Document scanning and enhancement algorithms for RBD OFİS.
//!
//! Provides high-performance, offline image processing:
//! - ITU-R BT.601 standard luminance calculation
//! - Optimal bimodal Otsu threshold calculation
//! - Direct RGBA buffer grayscale conversion
//! - High-contrast document binarization separating ink from paper
//! - PDF compilation using the core converter engine

#![allow(clippy::chunks_exact_to_as_chunks)]

use crate::error::EngineError;
use std::path::{Path, PathBuf};

/// Compute standard ITU-R BT.601 luminance from RGB channels.
/// Formula: Y = 0.299 * R + 0.587 * G + 0.114 * B.
#[inline]
pub fn compute_luminance(r: u8, g: u8, b: u8) -> u8 {
    ((299 * (r as u32) + 587 * (g as u32) + 114 * (b as u32)) / 1000).min(255) as u8
}

/// Calculate optimal threshold using Otsu's method from a 256-bin intensity histogram.
///
/// When a plateau of maximum between-class variance occurs (e.g. between two distinct peaks),
/// the midpoint of the plateau is chosen for maximum noise margin.
pub fn calculate_otsu_threshold(histogram: &[u32; 256], total_pixels: usize) -> u8 {
    if total_pixels == 0 {
        return 128;
    }

    let mut sum: f64 = 0.0;
    for (i, &count) in histogram.iter().enumerate() {
        sum += (i as f64) * (count as f64);
    }

    let mut sum_b: f64 = 0.0;
    let mut weight_b: u64 = 0;
    let mut max_variance: f64 = -1.0;
    let mut first_best: usize = 128;
    let mut last_best: usize = 128;

    for (t, &count) in histogram.iter().enumerate() {
        weight_b += count as u64;
        if weight_b == 0 {
            continue;
        }

        let weight_f = match (total_pixels as u64).checked_sub(weight_b) {
            Some(w) if w > 0 => w,
            _ => break,
        };

        sum_b += (t as f64) * (count as f64);
        let mean_b = sum_b / (weight_b as f64);
        let mean_f = (sum - sum_b) / (weight_f as f64);

        let diff = mean_b - mean_f;
        let between_variance = (weight_b as f64) * (weight_f as f64) * diff * diff;

        if between_variance > max_variance + 1e-5 {
            max_variance = between_variance;
            first_best = t;
            last_best = t;
        } else if (between_variance - max_variance).abs() <= 1e-5 {
            last_best = t;
        }
    }

    ((first_best + last_best) / 2) as u8
}

/// Convert an RGBA pixel buffer to 8-bit grayscale in-place.
pub fn scan_grayscale(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) -> Result<(), EngineError> {
    if width == 0 || height == 0 {
        return Err(EngineError::InvalidParameter(
            "Width and height must be positive".into(),
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| EngineError::InvalidParameter("Width overflow".into()))?;
    if stride < row_bytes {
        return Err(EngineError::InvalidParameter(format!(
            "Stride ({stride}) must be >= width * 4 ({row_bytes})"
        )));
    }
    let required_len = stride
        .checked_mul(height as usize - 1)
        .and_then(|s| s.checked_add(row_bytes))
        .ok_or_else(|| EngineError::InvalidParameter("Buffer size overflow".into()))?;
    if buffer.len() < required_len {
        return Err(EngineError::BufferTooSmall {
            required: required_len,
            provided: buffer.len(),
        });
    }

    for y in 0..height as usize {
        let row_start = y * stride;
        let row = &mut buffer[row_start..row_start + row_bytes];
        for pixel in row.chunks_exact_mut(4) {
            let gray = compute_luminance(pixel[0], pixel[1], pixel[2]);
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
        }
    }

    Ok(())
}

/// Convert an RGBA pixel buffer to high-contrast binarized (black text, white paper) in-place
/// using Otsu thresholding.
///
/// Returns the computed optimal Otsu threshold value.
pub fn scan_binarize(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) -> Result<u8, EngineError> {
    if width == 0 || height == 0 {
        return Err(EngineError::InvalidParameter(
            "Width and height must be positive".into(),
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| EngineError::InvalidParameter("Width overflow".into()))?;
    if stride < row_bytes {
        return Err(EngineError::InvalidParameter(format!(
            "Stride ({stride}) must be >= width * 4 ({row_bytes})"
        )));
    }
    let required_len = stride
        .checked_mul(height as usize - 1)
        .and_then(|s| s.checked_add(row_bytes))
        .ok_or_else(|| EngineError::InvalidParameter("Buffer size overflow".into()))?;
    if buffer.len() < required_len {
        return Err(EngineError::BufferTooSmall {
            required: required_len,
            provided: buffer.len(),
        });
    }

    let mut histogram = [0u32; 256];
    let total_pixels = (width as usize) * (height as usize);

    // Pass 1: Build luminance histogram
    for y in 0..height as usize {
        let row_start = y * stride;
        let row = &buffer[row_start..row_start + row_bytes];
        for pixel in row.chunks_exact(4) {
            let gray = compute_luminance(pixel[0], pixel[1], pixel[2]);
            histogram[gray as usize] += 1;
        }
    }

    // Pass 2: Calculate Otsu threshold
    let threshold = calculate_otsu_threshold(&histogram, total_pixels);

    // Pass 3: Binarize pixels (<= threshold is black 0, > threshold is white 255)
    for y in 0..height as usize {
        let row_start = y * stride;
        let row = &mut buffer[row_start..row_start + row_bytes];
        for pixel in row.chunks_exact_mut(4) {
            let gray = compute_luminance(pixel[0], pixel[1], pixel[2]);
            let val = if gray <= threshold { 0 } else { 255 };
            pixel[0] = val;
            pixel[1] = val;
            pixel[2] = val;
            pixel[3] = 255;
        }
    }

    Ok(threshold)
}

/// Compile scanned images into a standard PDF document using A4 page layout and 20pt margin.
pub fn scan_generate_pdf(image_paths: &[PathBuf], output_path: &Path) -> Result<(), EngineError> {
    if image_paths.is_empty() {
        return Err(EngineError::InvalidParameter(
            "No image paths provided for PDF generation".into(),
        ));
    }
    let options = crate::converter::ImageToPdfOptions {
        page_size: crate::converter::PageSize::A4,
        margin_pt: 20.0,
    };
    crate::converter::images_to_pdf(image_paths, &options, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_luminance_standard_values() {
        // Pure White: 0.299 * 255 + 0.587 * 255 + 0.114 * 255 = 255
        assert_eq!(compute_luminance(255, 255, 255), 255);

        // Pure Black
        assert_eq!(compute_luminance(0, 0, 0), 0);

        // Pure Red: 0.299 * 255 = 76.245 -> 76
        assert_eq!(compute_luminance(255, 0, 0), 76);

        // Pure Green: 0.587 * 255 = 149.685 -> 149
        assert_eq!(compute_luminance(0, 255, 0), 149);

        // Pure Blue: 0.114 * 255 = 29.07 -> 29
        assert_eq!(compute_luminance(0, 0, 255), 29);
    }

    #[test]
    fn test_otsu_threshold_bimodal() {
        // Bimodal distribution: 500 pixels dark (40), 1000 pixels bright (220)
        let mut histogram = [0u32; 256];
        histogram[40] = 500;
        histogram[220] = 1000;
        let total = 1500;

        let threshold = calculate_otsu_threshold(&histogram, total);
        assert!(
            threshold > 40 && threshold < 220,
            "Threshold ({threshold}) should separate peaks 40 and 220"
        );
    }

    #[test]
    fn test_otsu_threshold_uniform() {
        let histogram = [10u32; 256];
        let total = 2560;

        let threshold = calculate_otsu_threshold(&histogram, total);
        assert!(threshold > 0);
    }

    #[test]
    fn test_otsu_threshold_empty() {
        let histogram = [0u32; 256];
        let threshold = calculate_otsu_threshold(&histogram, 0);
        assert_eq!(threshold, 128);
    }

    #[test]
    fn test_scan_grayscale_and_binarize() {
        let width = 2u32;
        let height = 2u32;
        let stride = 8usize; // 2 pixels * 4 bytes
        let buf = vec![
            // Row 0
            255, 0, 0, 255, // Red (lum 76)
            0, 255, 0, 255, // Green (lum 149)
            // Row 1
            0, 0, 0, 255, // Black (lum 0)
            255, 255, 255, 255, // White (lum 255)
        ];

        let mut gray_buf = buf.clone();
        scan_grayscale(&mut gray_buf, width, height, stride).expect("Grayscale should succeed");

        assert_eq!(gray_buf[0..4], [76, 76, 76, 255]);
        assert_eq!(gray_buf[4..8], [149, 149, 149, 255]);
        assert_eq!(gray_buf[8..12], [0, 0, 0, 255]);
        assert_eq!(gray_buf[12..16], [255, 255, 255, 255]);

        let mut bin_buf = buf.clone();
        let threshold =
            scan_binarize(&mut bin_buf, width, height, stride).expect("Binarize should succeed");

        // Pixels with luminance <= threshold become 0 (black), > threshold become 255 (white)
        for chunk in bin_buf.chunks_exact(4) {
            assert!(chunk[0] == 0 || chunk[0] == 255);
            assert_eq!(chunk[0], chunk[1]);
            assert_eq!(chunk[1], chunk[2]);
            assert_eq!(chunk[3], 255);
        }
        assert!(threshold > 0);
    }

    #[test]
    fn test_corrupt_invalid_inputs() {
        let mut buf = vec![0u8; 16];

        // Zero width
        assert!(scan_grayscale(&mut buf, 0, 2, 8).is_err());
        assert!(scan_binarize(&mut buf, 0, 2, 8).is_err());

        // Zero height
        assert!(scan_grayscale(&mut buf, 2, 0, 8).is_err());
        assert!(scan_binarize(&mut buf, 2, 0, 8).is_err());

        // Stride too small
        assert!(scan_grayscale(&mut buf, 2, 2, 4).is_err());
        assert!(scan_binarize(&mut buf, 2, 2, 4).is_err());

        // Buffer too small
        let mut tiny_buf = vec![0u8; 4];
        assert!(scan_grayscale(&mut tiny_buf, 2, 2, 8).is_err());
        assert!(scan_binarize(&mut tiny_buf, 2, 2, 8).is_err());

        // Empty image paths for PDF generation
        let empty_paths: Vec<PathBuf> = vec![];
        let out_pdf = PathBuf::from("nonexistent.pdf");
        assert!(scan_generate_pdf(&empty_paths, &out_pdf).is_err());

        // Non-existent image file
        let invalid_paths = vec![PathBuf::from("does_not_exist_xyz123.jpg")];
        assert!(scan_generate_pdf(&invalid_paths, &out_pdf).is_err());
    }
}
