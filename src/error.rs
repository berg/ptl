use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtlError {
    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("No supported P-Touch printer found")]
    NoPrinterFound,

    #[error("Printer is in P-Lite mode — toggle the mode switch or press the P-Lite button")]
    PliteMode,

    #[error("Image too wide: {image_px}px > tape {tape_px}px")]
    ImageTooWide { image_px: u32, tape_px: u32 },

    #[error("Font not found: {0}")]
    #[allow(dead_code)]
    FontNotFound(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("Unknown tape width: {0}mm")]
    UnknownTapeWidth(u8),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Config error: {0}")]
    Config(String),
}
