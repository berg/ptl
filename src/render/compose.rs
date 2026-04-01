use super::LabelBitmap;

impl LabelBitmap {
    /// Append `other` to the right of this bitmap (horizontal concatenation).
    /// Both bitmaps must have the same height.
    pub fn append(&mut self, other: &LabelBitmap) {
        assert_eq!(
            self.height, other.height,
            "Cannot append bitmaps of different heights ({} vs {})",
            self.height, other.height
        );
        let new_width = self.width + other.width;
        let mut new_pixels = Vec::with_capacity((new_width * self.height) as usize);

        for row in 0..self.height as usize {
            let self_start = row * self.width as usize;
            let self_end = self_start + self.width as usize;
            new_pixels.extend_from_slice(&self.pixels[self_start..self_end]);

            let other_start = row * other.width as usize;
            let other_end = other_start + other.width as usize;
            new_pixels.extend_from_slice(&other.pixels[other_start..other_end]);
        }

        self.pixels = new_pixels;
        self.width = new_width;
    }

    /// Save the bitmap as a grayscale PNG for --output mode.
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), crate::error::PtlError> {
        // Invert: our convention is 0=white, 255=black; PNG luma 0=black, 255=white
        let img = image::GrayImage::from_fn(self.width, self.height, |x, y| {
            let val = self.get_pixel(x, y);
            image::Luma([255 - val])
        });
        img.save(path)?;
        Ok(())
    }
}

/// Create a blank padding bitmap of `pixels` width and `height` height.
pub fn make_padding(pixels: u32, height: u32) -> LabelBitmap {
    LabelBitmap::new_white(pixels, height)
}

/// Create a dashed vertical cut-mark bitmap (5 pixels wide).
pub fn make_cutmark(height: u32) -> LabelBitmap {
    let width = 5u32;
    let mut bm = LabelBitmap::new_white(width, height);
    // Dashed pattern: draw a center line with dashes of 4 on / 4 off
    let cx = width / 2;
    for y in 0..height {
        if (y / 4) % 2 == 0 {
            bm.set_pixel(cx, y);
        }
    }
    bm
}
