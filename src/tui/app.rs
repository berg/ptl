use crate::render::LabelBitmap;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Editing,
    Printing,
    Quit,
}

pub struct App {
    pub lines: [String; 4],
    pub focused_line: usize,
    pub font: String,
    pub fontsize: Option<f32>,
    pub tape_width_px: u32,
    pub tape_width_mm: u8,
    pub preview: Option<LabelBitmap>,
    pub mode: AppMode,
    pub status_msg: String,
}

impl App {
    pub fn new(font: String, fontsize: Option<f32>, tape_width_px: u32, tape_width_mm: u8) -> Self {
        Self {
            lines: [String::new(), String::new(), String::new(), String::new()],
            focused_line: 0,
            font,
            fontsize,
            tape_width_px,
            tape_width_mm,
            preview: None,
            mode: AppMode::Editing,
            status_msg: String::new(),
        }
    }

    /// Active (non-empty) text lines
    pub fn active_lines(&self) -> Vec<&str> {
        let v: Vec<&str> = self.lines.iter()
            .map(|s| s.as_str())
            .collect();
        // Keep at least 1 line
        let non_empty: Vec<&str> = v.iter().copied().filter(|s| !s.is_empty()).collect();
        if non_empty.is_empty() { vec![""] } else { non_empty }
    }

    /// Rebuild the preview bitmap from current text
    pub fn refresh_preview(&mut self) {
        let active = self.active_lines();
        match crate::render::text::render_text(
            &active,
            &self.font.clone(),
            self.fontsize,
            self.tape_width_px,
        ) {
            Ok(bm) => self.preview = Some(bm),
            Err(e) => {
                self.status_msg = format!("Preview error: {e}");
                self.preview = None;
            }
        }
    }
}
