pub mod app;
pub mod ui;

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::config::Config;
use crate::error::PtlError;
use crate::printer::PtouchDevice;

use app::{App, AppMode};

pub fn run_interactive(config: &Config) -> Result<(), anyhow::Error> {
    // Connect to printer to get tape dimensions
    let preferred = config.device.as_ref().and_then(|s| parse_vid_pid(s));
    let printer = PtouchDevice::open(preferred)?;
    let tape_px = printer.tape_width_px;
    let tape_mm = printer.status.media_width_mm;

    let mut app = App::new(
        config.font.clone(),
        config.fontsize,
        tape_px,
        tape_mm,
    );
    app.refresh_preview();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, printer);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result.map_err(anyhow::Error::from)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    printer: PtouchDevice,
) -> Result<(), PtlError> {
    loop {
        terminal.draw(|f| ui::draw(f, app)).map_err(PtlError::Io)?;

        if let Event::Key(key) = event::read().map_err(PtlError::Io)? {
            match (&app.mode, key.code, key.modifiers) {
                (AppMode::Editing, KeyCode::Char('q'), _)
                | (AppMode::Editing, KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    app.mode = AppMode::Quit;
                    break;
                }
                (AppMode::Editing, KeyCode::Tab, _) => {
                    app.focused_line = (app.focused_line + 1) % 4;
                    app.status_msg.clear();
                }
                (AppMode::Editing, KeyCode::BackTab, _) => {
                    app.focused_line = app.focused_line.saturating_sub(1);
                    app.status_msg.clear();
                }
                (AppMode::Editing, KeyCode::Enter, _) => {
                    app.mode = AppMode::Printing;
                    app.status_msg = "Printing...".to_string();
                    terminal.draw(|f| ui::draw(f, app)).map_err(PtlError::Io)?;

                    let active = app.active_lines();
                    match crate::render::text::render_text(
                        &active,
                        &app.font,
                        app.fontsize,
                        app.tape_width_px,
                    ) {
                        Ok(bm) => {
                            printer.print_bitmap(&bm.pixels, bm.width, bm.height)?;
                            app.status_msg = "Printed!".to_string();
                        }
                        Err(e) => {
                            app.status_msg = format!("Error: {e}");
                        }
                    }
                    app.mode = AppMode::Editing;
                }
                (AppMode::Editing, KeyCode::Char('p'), _) => {
                    // Save preview PNG
                    if let Some(bm) = &app.preview {
                        let path = std::path::Path::new("label_preview.png");
                        match bm.save_png(path) {
                            Ok(()) => app.status_msg = format!("Saved {}", path.display()),
                            Err(e) => app.status_msg = format!("Save error: {e}"),
                        }
                    }
                }
                (AppMode::Editing, KeyCode::Char(c), _) => {
                    app.lines[app.focused_line].push(c);
                    app.refresh_preview();
                    app.status_msg.clear();
                }
                (AppMode::Editing, KeyCode::Backspace, _) => {
                    app.lines[app.focused_line].pop();
                    app.refresh_preview();
                    app.status_msg.clear();
                }
                _ => {}
            }
        }

        if app.mode == AppMode::Quit {
            break;
        }
    }
    Ok(())
}

fn parse_vid_pid(s: &str) -> Option<(u16, u16)> {
    let (vid_str, pid_str) = s.split_once(':')?;
    let vid = u16::from_str_radix(vid_str.trim(), 16).ok()?;
    let pid = u16::from_str_radix(pid_str.trim(), 16).ok()?;
    Some((vid, pid))
}
