/// A single 128-pixel wide raster column (16 bytes).
///
/// Bit layout mirrors the C implementation's `rasterline_setpixel`:
///   byte index = 15 - (pixel / 8)
///   bit index  = pixel % 8  (LSB = left within each group of 8)
///
/// So pixel 0 is byte[15] bit 0, pixel 127 is byte[0] bit 7.
#[derive(Debug, Clone, Default)]
pub struct RasterLine([u8; 16]);

impl RasterLine {
    pub fn new() -> Self {
        Self([0u8; 16])
    }

    /// Set a single pixel (0 = leftmost). Out-of-range pixels are silently ignored.
    pub fn set_pixel(&mut self, pixel: usize) {
        if pixel >= 128 {
            return;
        }
        self.0[15 - (pixel / 8)] |= 1 << (pixel % 8);
    }

/// Encode for standard (non-PackBits) printers:
    ///   0x47 [0x10] 0x00 [16 bytes]
    pub fn encode_standard(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(19);
        out.push(0x47);
        out.push(16);
        out.push(0x00);
        out.extend_from_slice(&self.0);
        out
    }

    /// Encode for PackBits-capable printers (trivial/uncompressed run):
    ///   0x47 [0x11] 0x00 [0x0F] [16 bytes]
    ///
    /// The PackBits literal-run byte 0x0F means "copy next 16 bytes" (N+1 bytes).
    pub fn encode_packbits(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(20);
        out.push(0x47);
        out.push(17);   // total payload length (1 header + 16 data)
        out.push(0x00); // reserved
        out.push(15);   // PackBits: literal run of 15+1 = 16 bytes
        out.extend_from_slice(&self.0);
        out
    }

    /// Empty raster line command (no printing, tape advances)
    pub fn encode_empty() -> &'static [u8] {
        crate::printer::protocol::CMD_EMPTY_RASTER_LINE
    }
}

/// Build a vec of raster lines from a label bitmap, applying centering.
///
/// `bitmap_pixels` is row-major (row 0 = top), 0 = white, non-zero = ink.
/// `bitmap_width` = number of columns, `bitmap_height` = tape height in pixels.
/// `max_px` = printer's maximum raster width (usually 128).
///
/// The image is centered vertically within the 128-pixel raster field.
pub fn bitmap_to_raster_lines(
    bitmap_pixels: &[u8],
    bitmap_width: u32,
    bitmap_height: u32,
    max_px: u32,
) -> Vec<RasterLine> {
    let offset = (max_px / 2).saturating_sub(bitmap_height / 2) as usize;
    let w = bitmap_width as usize;
    let h = bitmap_height as usize;

    (0..w)
        .map(|col| {
            let mut line = RasterLine::new();
            for i in 0..h {
                // Mirror the row axis to match the C reference (reads image bottom-to-top
                // while placing into raster top-to-bottom): row = h-1-i
                let row = h - 1 - i;
                if bitmap_pixels[row * w + col] > 0 {
                    line.set_pixel(offset + i);
                }
            }
            line
        })
        .collect()
}
