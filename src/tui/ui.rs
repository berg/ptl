use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),     // preview
            Constraint::Length(8),  // edit pane
            Constraint::Length(3),  // status/help bar
        ])
        .split(area);

    draw_preview(frame, app, chunks[0]);
    draw_editor(frame, app, chunks[1]);
    draw_statusbar(frame, app, chunks[2]);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Label Preview ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if let Some(bm) = &app.preview {
        // Render bitmap as half-block characters (▀ = top pixel, ▄ = bottom pixel).
        // Each terminal cell = 2 pixel rows. Scale bitmap to fit the inner area.
        let cell_w = inner.width as u32;
        let cell_h = (inner.height as u32) * 2; // two pixel rows per cell row

        let scale_x = bm.width as f32 / cell_w as f32;
        let scale_y = bm.height as f32 / cell_h as f32;

        let mut lines: Vec<Line> = Vec::new();

        for cell_row in 0..inner.height {
            let mut spans: Vec<Span> = Vec::new();
            for cell_col in 0..inner.width {
                let top_px_y = ((cell_row as f32 * 2.0) * scale_y) as u32;
                let bot_px_y = (((cell_row as f32 * 2.0) + 1.0) * scale_y) as u32;
                let px_x = (cell_col as f32 * scale_x) as u32;

                let top = bm.get_pixel(px_x.min(bm.width - 1), top_px_y.min(bm.height - 1)) > 0;
                let bot = bm.get_pixel(px_x.min(bm.width - 1), bot_px_y.min(bm.height - 1)) > 0;

                let (ch, fg, bg) = match (top, bot) {
                    (true, true)   => ('█', Color::Black, Color::White),
                    (true, false)  => ('▀', Color::Black, Color::White),
                    (false, true)  => ('▄', Color::Black, Color::White),
                    (false, false) => (' ', Color::White, Color::White),
                };
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(fg).bg(bg),
                ));
            }
            lines.push(Line::from(spans));
        }

        let para = Paragraph::new(lines);
        frame.render_widget(para, inner);
    } else {
        let para = Paragraph::new(Span::styled(
            " (type text below to see a preview)",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(para, inner);
    }
}

fn draw_editor(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Edit Label ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let row_constraints = [Constraint::Length(1); 4];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    for (i, (row_area, text)) in rows.iter().zip(app.lines.iter()).enumerate() {
        let focused = i == app.focused_line;
        let label = format!("Line {}: ", i + 1);
        let content = format!("{label}{text}{}", if focused { "█" } else { "" });

        let style = if focused {
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let para = Paragraph::new(Span::styled(content, style));
        frame.render_widget(para, *row_area);
    }
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let tape_info = format!("Tape: {}mm ({}px)", app.tape_width_mm, app.tape_width_px);
    let font_info = format!(
        "Font: {}  Size: {}",
        app.font,
        app.fontsize.map_or("auto".to_string(), |s| format!("{:.0}px", s))
    );
    let msg = if app.status_msg.is_empty() {
        format!("{tape_info}  {font_info}")
    } else {
        app.status_msg.clone()
    };
    let help = " [Tab] Next  [Enter] Print  [p] PNG  [q] Quit ";

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(help.len() as u16)])
        .split(area);

    frame.render_widget(
        Paragraph::new(msg).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(help, Style::default().fg(Color::DarkGray)))
            .block(Block::default().borders(Borders::ALL)),
        chunks[1],
    );
}
