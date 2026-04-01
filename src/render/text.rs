use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use log::debug;

use crate::error::PtlError;
use super::LabelBitmap;

/// Render one or more lines of text into a LabelBitmap that fits within
/// `tape_width_px` pixels tall.
///
/// - `lines`: 1–4 text strings
/// - `font_name`: font family name (e.g. "sans-serif") or an absolute path
/// - `fontsize_override`: if `Some`, skip auto-sizing
/// - `tape_width_px`: height of the tape in pixels
pub fn render_text(
    lines: &[&str],
    font_name: &str,
    fontsize_override: Option<f32>,
    tape_width_px: u32,
) -> Result<LabelBitmap, PtlError> {
    if lines.is_empty() {
        return Err(PtlError::Render("No text lines provided".into()));
    }

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    let num_lines = lines.len();
    let line_height_budget = tape_width_px as f32 / num_lines as f32;

    // Determine font size: use override, or find the largest size that fits
    let font_size = match fontsize_override {
        Some(s) => s,
        None => lines
            .iter()
            .map(|line| find_font_size(&mut font_system, line, font_name, line_height_budget))
            .fold(f32::MAX, f32::min),
    };
    debug!("Using font size: {:.1}px", font_size);

    // Metrics: line_height = our budget per line
    let metrics = Metrics::new(font_size, line_height_budget);

    // Measure total label width needed
    let label_width = measure_label_width(&mut font_system, lines, font_name, metrics)?;
    if label_width == 0 {
        return Err(PtlError::Render("Text rendered to zero width".into()));
    }

    let mut bitmap = LabelBitmap::new_white(label_width, tape_width_px);

    // Render each line into the bitmap at the correct vertical offset
    for (i, &line_text) in lines.iter().enumerate() {
        let y_offset = (i as f32 * line_height_budget) as i32;
        render_line_into(
            &mut font_system,
            &mut swash_cache,
            &mut bitmap,
            line_text,
            font_name,
            metrics,
            y_offset,
        )?;
    }

    Ok(bitmap)
}

/// Find the largest font size (starting from 4, stepping by 1) whose rendered
/// height does not exceed `max_height`.
fn find_font_size(
    font_system: &mut FontSystem,
    text: &str,
    font_name: &str,
    max_height: f32,
) -> f32 {
    let mut size = 4.0f32;
    loop {
        let metrics = Metrics::new(size, size * 1.2);
        let height = measure_text_height(font_system, text, font_name, metrics);
        if height > max_height || size > 500.0 {
            return (size - 1.0).max(4.0);
        }
        size += 1.0;
    }
}

/// Measure the rendered height (ascent + descent) of a single line at given metrics.
fn measure_text_height(
    font_system: &mut FontSystem,
    text: &str,
    font_name: &str,
    metrics: Metrics,
) -> f32 {
    let mut buffer = make_buffer(font_system, text, font_name, metrics, None);
    buffer.shape_until_scroll(font_system, false);

    buffer
        .layout_runs()
        .map(|run| run.line_height)
        .fold(0.0f32, f32::max)
}

/// Measure total pixel width needed for all lines combined (max across lines).
fn measure_label_width(
    font_system: &mut FontSystem,
    lines: &[&str],
    font_name: &str,
    metrics: Metrics,
) -> Result<u32, PtlError> {
    let max_width = lines
        .iter()
        .map(|&line| {
            let mut buf = make_buffer(font_system, line, font_name, metrics, None);
            buf.shape_until_scroll(font_system, false);
            buf.layout_runs()
                .map(|run| run.line_w)
                .fold(0.0f32, f32::max)
        })
        .fold(0.0f32, f32::max);

    Ok(max_width.ceil() as u32 + 2) // +2px margin
}

/// Render a single line of text into a region of the bitmap starting at y_offset.
fn render_line_into(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    bitmap: &mut LabelBitmap,
    text: &str,
    font_name: &str,
    metrics: Metrics,
    y_offset: i32,
) -> Result<(), PtlError> {
    let mut buffer = make_buffer(font_system, text, font_name, metrics, Some(bitmap.width as f32));
    buffer.shape_until_scroll(font_system, false);

    buffer.draw(font_system, swash_cache, Color::rgb(0, 0, 0), |x, y, w, h, color| {
        // color.a() is the coverage alpha (0=transparent, 255=opaque)
        if color.a() < 64 {
            return;
        }
        let py = y_offset + y;
        for dy in 0..h as i32 {
            for dx in 0..w as i32 {
                let px = x + dx;
                let prow = py + dy;
                if px >= 0 && prow >= 0 {
                    bitmap.set_pixel(px as u32, prow as u32);
                }
            }
        }
    });

    Ok(())
}

/// Create a cosmic-text Buffer for the given text with the right font and metrics.
fn make_buffer(
    font_system: &mut FontSystem,
    text: &str,
    font_name: &str,
    metrics: Metrics,
    width: Option<f32>,
) -> Buffer {
    let attrs = if font_name.starts_with('/') || font_name.ends_with(".ttf") || font_name.ends_with(".otf") {
        // Absolute path — cosmic-text uses Family::Name for lookup; the font must
        // have already been loaded into the FontSystem. We load it on demand below.
        Attrs::new().family(Family::Name(font_name))
    } else {
        match font_name.to_lowercase().as_str() {
            "sans-serif" | "sans" => Attrs::new().family(Family::SansSerif),
            "serif" => Attrs::new().family(Family::Serif),
            "monospace" | "mono" => Attrs::new().family(Family::Monospace),
            _ => Attrs::new().family(Family::Name(font_name)),
        }
    };

    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_size(font_system, width, None);
    buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
    buffer
}
