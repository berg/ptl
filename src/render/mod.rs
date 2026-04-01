pub mod compose;
pub mod image;
pub mod text;

/// A monochrome label bitmap in row-major order.
/// - `width`  = number of columns (label length along the tape)
/// - `height` = tape width in pixels
/// - `pixels` = row-major, 0 = white, 255 = black ink
#[derive(Debug, Clone)]
pub struct LabelBitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl LabelBitmap {
    pub fn new_white(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; (width * height) as usize],
        }
    }

    /// Get pixel value at (x=col, y=row)
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u8 {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Set pixel at (x=col, y=row) to ink (255)
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = 255;
        }
    }
}
