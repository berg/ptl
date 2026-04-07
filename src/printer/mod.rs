// Based on ptouch-print — Copyright (C) 2013-2026 Dominic Radermacher
// <https://dominic.familie-radermacher.ch/projekte/ptouch-print/>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

pub mod devices;
pub mod protocol;
pub mod raster;

use std::time::Duration;

use log::debug;
use rusb::{DeviceHandle, GlobalContext};

use crate::error::PtlError;
use devices::{DeviceFlags, DeviceInfo, find_device, tape_mm_to_px};
use protocol::{
    CMD_COMPRESSION_PACKBITS, CMD_INIT, CMD_PRINT_CUT,
    CMD_RASTER_MODE_P700, CMD_RASTER_MODE_STANDARD, CMD_STATUS_REQUEST,
    PrinterStatus,
};
use raster::{RasterLine, bitmap_to_raster_lines};

const ENDPOINT_OUT: u8 = 0x02;
const ENDPOINT_IN: u8 = 0x81;
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(5);
const STATUS_RETRIES: u32 = 10;
const STATUS_RETRY_DELAY: Duration = Duration::from_millis(100);

pub struct PtouchDevice {
    handle: DeviceHandle<GlobalContext>,
    pub info: &'static DeviceInfo,
    pub status: PrinterStatus,
    pub tape_width_px: u32,
}

impl PtouchDevice {
    /// Open the first supported P-Touch printer found on USB.
    /// If `preferred_vid_pid` is set (e.g. `Some((0x04f9, 0x2062))`), prefer that device.
    pub fn open(preferred: Option<(u16, u16)>) -> Result<Self, PtlError> {
        let devices = rusb::devices()?;

        // Collect candidates
        let mut candidates: Vec<(rusb::Device<GlobalContext>, &'static DeviceInfo)> = Vec::new();

        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if let Some(info) = find_device(desc.vendor_id(), desc.product_id()) {
                if info.flags.contains(DeviceFlags::PLITE) {
                    return Err(PtlError::PliteMode);
                }
                candidates.push((device, info));
            }
        }

        if candidates.is_empty() {
            return Err(PtlError::NoPrinterFound);
        }

        // Prefer the requested VID:PID if specified
        let (device, info) = if let Some((vid, pid)) = preferred {
            candidates
                .into_iter()
                .find(|(_, i)| i.vid == vid && i.pid == pid)
                .ok_or(PtlError::NoPrinterFound)?
        } else {
            candidates.into_iter().next().unwrap()
        };

        debug!("Opening device: {} ({:04x}:{:04x})", info.name, info.vid, info.pid);

        let handle = device.open()?;

        // Detach kernel driver if needed (Linux)
        if handle.kernel_driver_active(0).unwrap_or(false) {
            handle.detach_kernel_driver(0)?;
        }
        handle.claim_interface(0)?;

        let mut dev = PtouchDevice {
            handle,
            info,
            status: PrinterStatus::default(),
            tape_width_px: 0,
        };

        dev.initialize()?;
        dev.status = dev.read_status()?;
        dev.tape_width_px = tape_mm_to_px(dev.status.media_width_mm)
            .ok_or(PtlError::UnknownTapeWidth(dev.status.media_width_mm))?;

        debug!("Tape width: {}mm = {}px", dev.status.media_width_mm, dev.tape_width_px);

        Ok(dev)
    }

    fn send(&self, data: &[u8]) -> Result<(), PtlError> {
        // USB bulk transfers are limited; send in ≤128-byte chunks
        for chunk in data.chunks(128) {
            self.handle
                .write_bulk(ENDPOINT_OUT, chunk, TRANSFER_TIMEOUT)?;
        }
        Ok(())
    }

    fn initialize(&self) -> Result<(), PtlError> {
        debug!("Sending init");
        self.send(CMD_INIT)
    }

    fn read_status(&self) -> Result<PrinterStatus, PtlError> {
        self.send(CMD_STATUS_REQUEST)?;

        for attempt in 0..STATUS_RETRIES {
            std::thread::sleep(STATUS_RETRY_DELAY);
            let mut buf = [0u8; 32];
            match self.handle.read_bulk(ENDPOINT_IN, &mut buf, TRANSFER_TIMEOUT) {
                Ok(32) => {
                    if let Some(status) = PrinterStatus::from_bytes(&buf) {
                        return Ok(status);
                    }
                    debug!("Status read attempt {}: invalid response header", attempt + 1);
                }
                Ok(n) => {
                    debug!("Status read attempt {}: short read ({} bytes)", attempt + 1, n);
                }
                Err(e) => {
                    debug!("Status read attempt {}: {}", attempt + 1, e);
                }
            }
        }
        Err(PtlError::Usb(rusb::Error::Timeout))
    }

    /// Print a pre-composed label bitmap and cut the tape.
    pub fn print_bitmap(
        &self,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), PtlError> {
        let use_packbits = self.info.flags.contains(DeviceFlags::RASTER_PACKBITS);
        let use_p700_init = self.info.flags.contains(DeviceFlags::P700_INIT);

        // Select raster mode
        if use_p700_init {
            self.send(CMD_RASTER_MODE_P700)?;
        } else {
            self.send(CMD_RASTER_MODE_STANDARD)?;
        }

        if use_packbits {
            self.send(CMD_COMPRESSION_PACKBITS)?;
        }

        // Send leading blank lines for models that need them
        for _ in 0..self.info.pre_print_padding_px {
            self.send(RasterLine::encode_empty())?;
        }

        // Convert and send raster lines
        let lines = bitmap_to_raster_lines(pixels, width, height, self.info.max_px);
        for line in &lines {
            let encoded = if use_packbits {
                line.encode_packbits()
            } else {
                line.encode_standard()
            };
            self.send(&encoded)?;
        }

        // Print and cut
        self.send(CMD_PRINT_CUT)?;

        Ok(())
    }

}

impl Drop for PtouchDevice {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(0);
    }
}
