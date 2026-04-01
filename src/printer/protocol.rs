/// Initialize the printer (ESC @)
pub const CMD_INIT: &[u8] = &[0x1B, 0x40];

/// Request status (ESC i S)
pub const CMD_STATUS_REQUEST: &[u8] = &[0x1B, 0x69, 0x53];

/// Enter raster graphics mode — standard printers
pub const CMD_RASTER_MODE_STANDARD: &[u8] = &[0x1B, 0x69, 0x52, 0x01];

/// Enter raster graphics mode — PT-P700 / PT-P750W
pub const CMD_RASTER_MODE_P700: &[u8] = &[0x1B, 0x69, 0x61, 0x01];

/// Enable PackBits compression
pub const CMD_COMPRESSION_PACKBITS: &[u8] = &[0x4D, 0x02];

/// Empty raster line (tape advances one column without printing)
pub const CMD_EMPTY_RASTER_LINE: &[u8] = &[0x5A];

/// Print and cut
pub const CMD_PRINT_CUT: &[u8] = &[0x1A];

/// 32-byte status response from the printer
#[derive(Debug, Default, Clone)]
pub struct PrinterStatus {
    pub error: u16,
    pub media_width_mm: u8,
    pub media_type: u8,
    pub mode: u8,
    pub status_type: u8,
    pub phase_type: u8,
    pub tape_color: u8,
    pub text_color: u8,
}

impl PrinterStatus {
    pub fn from_bytes(buf: &[u8; 32]) -> Option<Self> {
        // Validate magic header bytes
        if buf[0] != 0x80 || buf[1] != 0x20 || buf[2] != b'B' {
            return None;
        }
        Some(Self {
            error: u16::from_le_bytes([buf[8], buf[9]]),
            media_width_mm: buf[10],
            media_type: buf[11],
            mode: buf[15],
            status_type: buf[18],
            phase_type: buf[19],
            tape_color: buf[24],
            text_color: buf[25],
        })
    }

    pub fn tape_color_name(&self) -> &'static str {
        match self.tape_color {
            0x01 => "white",
            0x02 => "other",
            0x03 => "clear",
            0x04 => "red",
            0x05 => "blue",
            0x06 => "yellow",
            0x07 => "green",
            0x08 => "black",
            0x09 => "clear (white text)",
            0x20 => "matte white",
            0x21 => "matte clear",
            0x22 => "matte silver",
            0x23 => "satin gold",
            0x24 => "satin silver",
            0x30 => "blue (D)",
            0x31 => "red (D)",
            0x40 => "fluorescent orange",
            0x41 => "fluorescent yellow",
            0x50 => "berry (pink)",
            0x51 => "light grey",
            0x52 => "lime green",
            0x60 => "yellow (F)",
            0x61 => "pink (F)",
            0x62 => "blue (F)",
            0x70 => "white (heat shrink)",
            0x90 => "white (flex. ID)",
            0x91 => "yellow (flex. ID)",
            0xf0 => "cleaning",
            0xf1 => "stencil",
            0xff => "incompatible",
            _ => "unknown",
        }
    }

    pub fn text_color_name(&self) -> &'static str {
        match self.text_color {
            0x01 => "white",
            0x04 => "red",
            0x05 => "blue",
            0x08 => "black",
            0x0a => "gold",
            0x62 => "blue (F)",
            0xf0 => "cleaning",
            0xf1 => "stencil",
            0xff => "incompatible",
            _ => "unknown",
        }
    }

    pub fn media_type_name(&self) -> &'static str {
        match self.media_type {
            0x00 => "no media",
            0x01 => "laminated tape",
            0x03 => "non-laminated tape",
            0x11 => "heat-shrink tube",
            0xff => "incompatible",
            _ => "unknown",
        }
    }
}
