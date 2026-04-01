use std::path::Path;

use crate::error::PtlError;
use super::LabelBitmap;

/// Load a PNG image and threshold it to a 1-bit LabelBitmap.
/// Any pixel with luma < 128 is treated as ink; anything lighter is white.
pub fn load_png(path: &Path) -> Result<LabelBitmap, PtlError> {
    let img = image::open(path)?.into_luma8();
    let width = img.width();
    let height = img.height();
    let pixels: Vec<u8> = img
        .pixels()
        .map(|p| if p.0[0] < 128 { 255u8 } else { 0u8 })
        .collect();
    Ok(LabelBitmap { width, height, pixels })
}
