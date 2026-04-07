# ptl

A command-line tool for printing labels on Brother P-Touch printers directly from Linux and macOS — no Windows, no P-touch Editor required.

## Attribution

This project is almost entirely based on [ptouch-print](https://dominic.familie-radermacher.ch/projekte/ptouch-print/) by Dominic Radermacher, which reverse-engineered the Brother P-Touch USB raster protocol. The device table, raster encoding logic, and protocol understanding all derive from that work. Many thanks to Dominic for the original research and implementation.

**Please note:** This project is not maintained by Dominic and he is probably completely unaware of its existence. He is a busy person with very little time, and while I am inspired by his work I do not wish to bother him. Please **do not** send him questions or pester him about this software. Any bugs are my fault.

## License

GPLv3 — see [LICENSE](LICENSE).

## Supported Devices

| Model | VID:PID |
|-------|---------|
| PT-2420PC | 04f9:2007 |
| PT-1230PC | 04f9:202c |
| PT-2430PC | 04f9:202d |
| PT-2730 | 04f9:2041 |
| PT-E500 | 04f9:205f |
| PT-P700 | 04f9:2061 |
| PT-P750W | 04f9:2062 |
| PT-D450 | 04f9:2073 |
| PT-D600 | 04f9:2074 |

P-Lite USB mode (alternate PIDs) is detected but not supported — switch the device to standard mode first.

## Installation

### Homebrew (macOS and Linux)

```
brew tap berg/ptl
brew install ptl
```

### Build from source

Requires Rust (edition 2021).

```
cargo build --release
```

The binary lands in `target/release/ptl`.

#### Linux

You need `libusb-1.0` development headers:

```
# Debian/Ubuntu
sudo apt-get install libusb-1.0-0-dev

# Fedora/RHEL
sudo dnf install libusb1-devel
```

Alternatively, build with the vendored libusb (no system library needed):

```
cargo build --release --features vendored-libusb
```

#### macOS

libusb is pulled in automatically via Homebrew or the vendored feature. No extra steps needed.

#### Linux USB permissions

Without udev rules, accessing the printer requires root. To allow regular users:

```
# /etc/udev/rules.d/99-ptouch.rules
SUBSYSTEM=="usb", ATTRS{idVendor}=="04f9", MODE="0666"
```

Then reload: `sudo udevadm control --reload-rules && sudo udevadm trigger`

## Usage

```
ptl [OPTIONS] [ARGS]...
```

### Modes

**Print text label(s):**

```
ptl "Hello World"
ptl "Line 1" "Line 2" "Line 3"
```

**Print multiple labels in one job** (separate with `--`):

```
ptl "Label A" -- "Label B" -- "Label C"
```

**Print an image** (PNG/JPG, scaled to tape height):

```
ptl logo.png
```

**Print from JSON on stdin** (`--json`):

```
echo '["Hello", "World"]' | ptl --json
echo '{"lines":["L1"],"image":"icon.png"}' | ptl --json
```

Each line of stdin is one label. Supported JSON shapes:
- `["line1", "line2"]` — text label
- `{"lines": ["line1"], "image": "path.png"}` — text + image
- `{"type": "image", "path": "path.png"}` — image only

**Show printer/tape info:**

```
ptl --info
```

**Interactive TUI label designer:**

```
ptl --interactive
```

**Render to PNG instead of printing:**

```
ptl --output preview.png "Hello World"
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--cut` | off | Add cut marks before and between labels |
| `--pad <PX>` | `0` | Blank tape padding (pixels) on each side of every label |
| `--font <FONT>` | `sans-serif` | Font name (e.g. `"DejaVu Sans"`) or path to a `.ttf`/`.otf` file |
| `--fontsize <PX>` | auto | Font size in pixels (auto-sizes to tape height if omitted) |
| `--output <FILE>` | — | Write PNG instead of printing |
| `--device <VID:PID>` | auto | Target a specific printer by USB VID:PID (e.g. `04f9:2062`) |
| `--config <FILE>` | `~/.config/ptl/config.toml` | Config file path |
| `--info` | — | Print device and tape information then exit |
| `--interactive` | — | Launch TUI label designer |
| `--json` | — | Read newline-delimited JSON label specs from stdin |
| `--debug` | off | Enable debug logging |

### Configuration file

`~/.config/ptl/config.toml` — all fields are optional:

```toml
font = "sans-serif"       # font name or path to .ttf/.otf
fontsize = 48.0           # pixels; omit for auto-sizing
device = "04f9:2062"      # preferred printer VID:PID
log_level = "warn"        # trace | debug | info | warn | error
```

CLI flags override config file values.

## Examples

```sh
# Single-line label with cut marks and padding
ptl --cut --pad 10 "Storage Room A"

# Two-line label rendered to PNG for preview
ptl --output preview.png "Bryan Berg" "Engineer"

# Batch print from a file of JSON specs
cat labels.jsonl | ptl --json --cut

# Use a specific font at a fixed size
ptl --font "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" --fontsize 60 "FRAGILE"

# Target a specific printer if multiple are attached
ptl --device 04f9:2073 "PT-D450 only"
```
