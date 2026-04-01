mod config;
mod error;
mod printer;
mod render;
mod tui;

use std::io::BufRead;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use log::debug;
use serde::Deserialize;

use config::Config;
use error::PtlError;
use printer::PtouchDevice;
use render::LabelBitmap;
use render::compose::{make_cutmark, make_padding};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "ptl", about = "Brother P-Touch label printer", version)]
struct Cli {
    /// Show printer and tape information
    #[arg(long)]
    info: bool,

    /// Launch interactive TUI label designer
    #[arg(long)]
    interactive: bool,

    /// Read label specs from stdin as newline-delimited JSON
    #[arg(long)]
    json: bool,

    /// Add a cut mark before the first label and between labels
    #[arg(long)]
    cut: bool,

    /// Pixels of blank tape padding around each label
    #[arg(long, default_value = "0", value_name = "PX")]
    pad: u32,

    /// Font name (e.g. "sans-serif") or path to a .ttf/.otf file
    #[arg(long, value_name = "FONT")]
    font: Option<String>,

    /// Font size in pixels (auto-sized to tape height if omitted)
    #[arg(long, value_name = "PX")]
    fontsize: Option<f32>,

    /// Write output to a PNG file instead of printing
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Target device as hex VID:PID (e.g. 04f9:2062)
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    /// Config file [default: ~/.config/ptl/config.toml]
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Label content: text lines and/or image paths.
    /// Use bare `--` to separate multiple labels.
    /// Example: ptl "Hello" "World" -- "Next label"
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Label spec
// ---------------------------------------------------------------------------

/// A single label to be printed.
#[derive(Debug, Default)]
struct LabelSpec {
    lines: Vec<String>,
    image: Option<PathBuf>,
}

impl LabelSpec {
    fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.image.is_none()
    }
}

// ---------------------------------------------------------------------------
// JSON input format
// ---------------------------------------------------------------------------

/// Each line of `--json` stdin is one of these.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum JsonLine {
    /// `["line1", "line2"]`  — text label
    TextArray(Vec<String>),
    /// `{"type":"image","path":"..."}` or `{"lines":[...],"image":"..."}`
    Object(JsonLabelObject),
}

#[derive(Deserialize, Debug)]
struct JsonLabelObject {
    lines: Option<Vec<String>>,
    image: Option<String>,
    // "type":"image" + "path":"..." is an alternate image-only form
    #[serde(rename = "type")]
    kind: Option<String>,
    path: Option<String>,
}

impl JsonLine {
    fn into_spec(self) -> Result<LabelSpec> {
        match self {
            JsonLine::TextArray(lines) => Ok(LabelSpec { lines, image: None }),
            JsonLine::Object(obj) => {
                let image = if obj.kind.as_deref() == Some("image") {
                    obj.path.map(PathBuf::from)
                } else {
                    obj.image.map(PathBuf::from)
                };
                Ok(LabelSpec {
                    lines: obj.lines.unwrap_or_default(),
                    image,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.debug { "debug" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let config_path = cli.config.clone().or_else(Config::default_path);
    let mut config = match &config_path {
        Some(p) => Config::load(p).with_context(|| format!("Loading config from {}", p.display()))?,
        None => Config::default(),
    };

    if let Some(f) = cli.font    { config.font = f; }
    if let Some(s) = cli.fontsize { config.fontsize = Some(s); }
    if let Some(d) = cli.device  { config.device = Some(d); }

    // --- dispatch -----------------------------------------------------------

    if cli.info {
        return cmd_info(&config);
    }

    if cli.interactive {
        return tui::run_interactive(&config);
    }

    if cli.json {
        let specs = parse_json_stdin()?;
        if specs.is_empty() {
            bail!("No labels received on stdin");
        }
        return cmd_print(specs, &config, cli.cut, cli.pad, cli.output.as_deref());
    }

    // Positional args: split on bare "--" to get label groups
    if cli.args.is_empty() {
        bail!(
            "Nothing to print.\n\
            \n\
            Examples:\n\
              ptl \"Hello World\"\n\
              ptl \"Line 1\" \"Line 2\" \"Line 3\"\n\
              ptl \"Label A\" -- \"Label B\" --cut\n\
              ptl --info\n\
              ptl --interactive\n\
            \nRun `ptl --help` for all options."
        );
    }

    let specs = parse_positional_args(&cli.args);
    cmd_print(specs, &config, cli.cut, cli.pad, cli.output.as_deref())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_info(config: &Config) -> Result<()> {
    let preferred = config.device.as_ref().and_then(|s| parse_vid_pid(s));
    let dev = PtouchDevice::open(preferred)?;

    println!("Printer:      {}", dev.info.name);
    println!("USB:          {:04x}:{:04x}", dev.info.vid, dev.info.pid);
    println!("DPI:          {}", dev.info.dpi);
    println!("Max pixels:   {}", dev.info.max_px);
    println!();
    println!("Tape width:   {}mm ({} pixels)", dev.status.media_width_mm, dev.tape_width_px);
    println!("Media type:   {}", dev.status.media_type_name());
    println!("Tape color:   {}", dev.status.tape_color_name());
    println!("Text color:   {}", dev.status.text_color_name());
    println!("Mode:         0x{:02x}", dev.status.mode);
    println!("Status type:  0x{:02x}  Phase: 0x{:02x}", dev.status.status_type, dev.status.phase_type);
    if dev.status.error != 0 {
        println!("Error:        0x{:04x}", dev.status.error);
    }
    Ok(())
}

fn cmd_print(
    specs: Vec<LabelSpec>,
    config: &Config,
    cut: bool,
    pad: u32,
    output: Option<&std::path::Path>,
) -> Result<()> {
    let preferred = config.device.as_ref().and_then(|s| parse_vid_pid(s));
    let (tape_px, printer) = open_or_default_tape(preferred, output.is_some())?;

    // Render each label spec into a bitmap
    let mut label_bitmaps: Vec<LabelBitmap> = Vec::new();
    for spec in &specs {
        label_bitmaps.push(render_spec(spec, config, tape_px)?);
    }

    // Compose: [cut][pad] LABEL [pad] [cut][pad] LABEL [pad] ...
    let composed = compose_labels(label_bitmaps, pad, cut, tape_px);

    if let Some(out_path) = output {
        composed.save_png(out_path)
            .with_context(|| format!("Writing PNG to {}", out_path.display()))?;
        println!("Wrote {} ({}×{}px)", out_path.display(), composed.width, composed.height);
    } else {
        let dev = printer.expect("printer open for non-PNG output");
        dev.print_bitmap(&composed.pixels, composed.width, composed.height)?;
        println!("Printed ({}×{}px)", composed.width, composed.height);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render_spec(spec: &LabelSpec, config: &Config, tape_px: u32) -> Result<LabelBitmap> {
    let mut segments: Vec<LabelBitmap> = Vec::new();

    if !spec.lines.is_empty() {
        let refs: Vec<&str> = spec.lines.iter().map(|s| s.as_str()).collect();
        debug!("Rendering text: {:?}", refs);
        segments.push(
            render::text::render_text(&refs, &config.font, config.fontsize, tape_px)
                .context("Rendering text")?,
        );
    }

    if let Some(ref path) = spec.image {
        debug!("Loading image: {}", path.display());
        let bm = render::image::load_png(path)
            .with_context(|| format!("Loading image {}", path.display()))?;
        if bm.height > tape_px {
            return Err(PtlError::ImageTooWide { image_px: bm.height, tape_px }.into());
        }
        segments.push(bm);
    }

    segments
        .into_iter()
        .reduce(|mut a, b| { a.append(&b); a })
        .ok_or_else(|| anyhow::anyhow!("Empty label spec"))
}

/// Compose labels with optional cut marks and padding.
///
/// Layout: `[cut][pad] LABEL [pad]` repeated for each label — cut before
/// every label, padding on both sides, no trailing cut.
fn compose_labels(labels: Vec<LabelBitmap>, pad: u32, cut: bool, tape_px: u32) -> LabelBitmap {
    let mut acc: Option<LabelBitmap> = None;

    for label in labels {
        if cut {
            push(&mut acc, make_cutmark(tape_px));
        }
        if pad > 0 {
            push(&mut acc, make_padding(pad, tape_px));
        }
        push(&mut acc, label);
        if pad > 0 {
            push(&mut acc, make_padding(pad, tape_px));
        }
    }

    acc.expect("at least one label was composed")
}

fn push(acc: &mut Option<LabelBitmap>, seg: LabelBitmap) {
    match acc {
        None => *acc = Some(seg),
        Some(ref mut bm) => bm.append(&seg),
    }
}

// ---------------------------------------------------------------------------
// Arg parsing
// ---------------------------------------------------------------------------

/// Split positional args on bare `--` into label groups, then classify
/// each token as an image path (if it looks like one) or a text line.
fn parse_positional_args(args: &[String]) -> Vec<LabelSpec> {
    args.split(|a| a == "--")
        .filter(|group| !group.is_empty())
        .map(|group| {
            let mut spec = LabelSpec::default();
            for arg in group {
                if looks_like_image(arg) {
                    spec.image = Some(PathBuf::from(arg));
                } else {
                    spec.lines.push(arg.clone());
                }
            }
            spec
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn looks_like_image(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

/// Parse newline-delimited JSON from stdin; each line is one LabelSpec.
fn parse_json_stdin() -> Result<Vec<LabelSpec>> {
    let stdin = std::io::stdin();
    let mut specs = Vec::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let line = line.context("Reading stdin")?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let json: JsonLine = serde_json::from_str(line)
            .with_context(|| format!("Parsing JSON on line {}", i + 1))?;
        let spec = json.into_spec()?;
        if !spec.is_empty() {
            specs.push(spec);
        }
    }
    Ok(specs)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_or_default_tape(
    preferred: Option<(u16, u16)>,
    png_mode: bool,
) -> Result<(u32, Option<PtouchDevice>)> {
    match PtouchDevice::open(preferred) {
        Ok(dev) => { let px = dev.tape_width_px; Ok((px, Some(dev))) }
        Err(e) if png_mode => {
            eprintln!("No printer found ({e}); assuming 24mm tape (128px) for PNG output.");
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
