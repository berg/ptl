mod config;
mod error;
mod printer;
mod render;
mod tui;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use log::debug;

use config::Config;
use printer::PtouchDevice;
use render::compose::{make_cutmark, make_padding};
use render::LabelBitmap;

#[derive(Parser)]
#[command(name = "ptl", about = "Brother P-Touch label printer", version)]
struct Cli {
    /// Font name (e.g. "sans-serif") or absolute path to a .ttf/.otf file
    #[arg(long, value_name = "FONT")]
    font: Option<String>,

    /// Font size in pixels — auto-sized to tape height if omitted
    #[arg(long, value_name = "PX")]
    fontsize: Option<f32>,

    /// Config file path [default: ~/.config/ptl/config.toml]
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Target device as hex VID:PID (e.g. 04f9:2062)
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    /// Write label to a PNG file instead of sending to the printer
    #[arg(long, value_name = "FILE.png")]
    output: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    #[command(subcommand)]
    command: Option<Sub>,
}

#[derive(Subcommand)]
enum Sub {
    /// Display printer and tape information
    Info,

    /// Launch interactive TUI label designer
    Interactive,

    /// Compose and print a label
    Print(PrintArgs),
}

#[derive(Parser, Debug)]
struct PrintArgs {
    /// Text lines to print — up to 4 lines (e.g. --text "Line 1" "Line 2" "Line 3")
    #[arg(long = "text", value_name = "LINE", num_args(1..=4))]
    text: Vec<String>,

    /// PNG image file to print (black/white, 2-color)
    #[arg(long, value_name = "FILE")]
    image: Option<PathBuf>,

    /// Pixels of blank tape to add before label content
    #[arg(long, value_name = "PX", default_value = "0")]
    pad_before: u32,

    /// Pixels of blank tape to add after label content
    #[arg(long, value_name = "PX", default_value = "0")]
    pad_after: u32,

    /// Print a dashed cut mark at the end
    #[arg(long)]
    cut: bool,

    /// Compose image after text (default: text first, then image)
    #[arg(long)]
    image_first: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.debug { "debug" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let config_path = cli.config.clone().or_else(Config::default_path);
    let mut config = match &config_path {
        Some(p) => Config::load(p).with_context(|| format!("Loading config from {}", p.display()))?,
        None => Config::default(),
    };

    // CLI flags override config values
    if let Some(font) = cli.font { config.font = font; }
    if let Some(fs) = cli.fontsize { config.fontsize = Some(fs); }
    if let Some(dev) = cli.device { config.device = Some(dev); }

    match cli.command {
        Some(Sub::Info) => cmd_info(&config),
        Some(Sub::Interactive) => tui::run_interactive(&config),
        Some(Sub::Print(args)) => cmd_print(args, &config, cli.output.as_deref()),
        None => {
            bail!(
                "No subcommand given.\n\n\
                Usage examples:\n\
                  ptl print --text \"Hello World\"\n\
                  ptl print --text \"Line 1\" \"Line 2\" \"Line 3\"\n\
                  ptl print --image label.png\n\
                  ptl info\n\
                  ptl interactive\n\
                \nRun `ptl --help` or `ptl print --help` for full options."
            );
        }
    }
}

fn cmd_info(config: &Config) -> Result<()> {
    let preferred = config.device.as_ref().and_then(|s| parse_vid_pid(s));
    let dev = PtouchDevice::open(preferred)?;

    println!("Printer:    {}", dev.info.name);
    println!("USB:        {:04x}:{:04x}", dev.info.vid, dev.info.pid);
    println!("DPI:        {}", dev.info.dpi);
    println!("Max pixels: {}", dev.info.max_px);
    println!();
    println!("Tape width:   {}mm ({} pixels)", dev.status.media_width_mm, dev.tape_width_px);
    println!("Media type:   {}", dev.status.media_type_name());
    println!("Tape color:   {}", dev.status.tape_color_name());
    println!("Text color:   {}", dev.status.text_color_name());
    println!("Mode:         0x{:02x}", dev.status.mode);
    println!("Status type:  0x{:02x}  Phase: 0x{:02x}", dev.status.status_type, dev.status.phase_type);
    if dev.status.error != 0 {
        println!("Error code:   0x{:04x}", dev.status.error);
    }
    Ok(())
}

fn cmd_print(args: PrintArgs, config: &Config, output: Option<&std::path::Path>) -> Result<()> {
    if args.text.is_empty() && args.image.is_none() {
        bail!("Nothing to print. Specify --text and/or --image.");
    }

    let preferred = config.device.as_ref().and_then(|s| parse_vid_pid(s));

    // Open printer (or fall back to 128px tape for PNG-only mode)
    let (tape_px, printer) = open_or_default_tape(preferred, output.is_some())?;

    let text_segment: Option<LabelBitmap> = if !args.text.is_empty() {
        let refs: Vec<&str> = args.text.iter().map(|s| s.as_str()).collect();
        debug!("Rendering text: {:?}", refs);
        Some(
            render::text::render_text(&refs, &config.font, config.fontsize, tape_px)
                .context("Rendering text")?,
        )
    } else {
        None
    };

    let image_segment: Option<LabelBitmap> = if let Some(ref path) = args.image {
        debug!("Loading image: {}", path.display());
        let bm = render::image::load_png(path)
            .with_context(|| format!("Loading image {}", path.display()))?;
        if bm.height > tape_px {
            return Err(error::PtlError::ImageTooWide { image_px: bm.height, tape_px }.into());
        }
        Some(bm)
    } else {
        None
    };

    // Compose everything into a single label bitmap
    let mut label: Option<LabelBitmap> = None;

    let mut push = |seg: Option<LabelBitmap>| {
        if let Some(bm) = seg {
            match label {
                None => label = Some(bm),
                Some(ref mut acc) => acc.append(&bm),
            }
        }
    };

    if args.pad_before > 0 {
        push(Some(make_padding(args.pad_before, tape_px)));
    }

    if args.image_first {
        push(image_segment);
        push(text_segment);
    } else {
        push(text_segment);
        push(image_segment);
    }

    if args.pad_after > 0 {
        push(Some(make_padding(args.pad_after, tape_px)));
    }

    if args.cut {
        push(Some(make_cutmark(tape_px)));
    }

    let label = label.expect("at least one segment was composed");

    if let Some(out_path) = output {
        label.save_png(out_path)
            .with_context(|| format!("Writing PNG to {}", out_path.display()))?;
        println!("Wrote {} ({}×{}px)", out_path.display(), label.width, label.height);
    } else {
        let dev = printer.expect("printer must be open for non-PNG output");
        dev.print_bitmap(&label.pixels, label.width, label.height)?;
        println!("Printed ({}×{}px)", label.width, label.height);
    }

    Ok(())
}

/// Open the printer, or if `png_mode` is true and no printer is connected, fall
/// back to a 128px (24mm) tape default so PNG output works without hardware.
fn open_or_default_tape(
    preferred: Option<(u16, u16)>,
    png_mode: bool,
) -> Result<(u32, Option<PtouchDevice>)> {
    match PtouchDevice::open(preferred) {
        Ok(dev) => {
            let px = dev.tape_width_px;
            Ok((px, Some(dev)))
        }
        Err(e) if png_mode => {
            eprintln!("No printer connected ({e}); assuming 24mm tape (128px) for PNG output.");
            Ok((128, None))
        }
        Err(e) => Err(e.into()),
    }
}

fn parse_vid_pid(s: &str) -> Option<(u16, u16)> {
    let (vid_str, pid_str) = s.split_once(':')?;
    let vid = u16::from_str_radix(vid_str.trim(), 16).ok()?;
    let pid = u16::from_str_radix(pid_str.trim(), 16).ok()?;
    Some((vid, pid))
}
