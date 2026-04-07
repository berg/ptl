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

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceFlags: u32 {
        /// Uses "fake" PackBits framing for raster lines
        const RASTER_PACKBITS = 1 << 0;
        /// Requires the PT-P700/P750W special raster-mode init sequence
        const P700_INIT       = 1 << 1;
        /// Device is in P-Lite USB mode (unsupported)
        const PLITE           = 1 << 2;
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    /// Maximum printable pixels across the tape (always 128 for current models)
    pub max_px: u32,
    pub dpi: u32,
    pub flags: DeviceFlags,
    /// Blank raster lines to prepend before label content (some models need this)
    pub pre_print_padding_px: u32,
}

pub static DEVICES: &[DeviceInfo] = &[
    DeviceInfo {
        vid: 0x04f9, pid: 0x2007, name: "PT-2420PC",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x202c, name: "PT-1230PC",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::empty(),
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2030, name: "PT-1230PC (PLite)",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::PLITE,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x202d, name: "PT-2430PC",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::empty(),
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2031, name: "PT-2430PC (PLite)",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::PLITE,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2041, name: "PT-2730",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::empty(),
        pre_print_padding_px: 48,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x205f, name: "PT-E500",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS,
        pre_print_padding_px: 48,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2061, name: "PT-P700",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS.union(DeviceFlags::P700_INIT),
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2064, name: "PT-P700 (PLite)",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::PLITE,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2062, name: "PT-P750W",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS.union(DeviceFlags::P700_INIT),
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2065, name: "PT-P750W (PLite)",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::PLITE,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2073, name: "PT-D450",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS,
        pre_print_padding_px: 0,
    },
    DeviceInfo {
        vid: 0x04f9, pid: 0x2074, name: "PT-D600",
        max_px: 128, dpi: 180,
        flags: DeviceFlags::RASTER_PACKBITS,
        pre_print_padding_px: 0,
    },
];

/// Convert tape width in mm to pixel width
pub fn tape_mm_to_px(mm: u8) -> Option<u32> {
    match mm {
        6  => Some(32),
        9  => Some(52),
        12 => Some(76),
        18 => Some(120),
        24 => Some(128),
        36 => Some(192),
        _  => None,
    }
}

/// Find device info by VID/PID
pub fn find_device(vid: u16, pid: u16) -> Option<&'static DeviceInfo> {
    DEVICES.iter().find(|d| d.vid == vid && d.pid == pid)
}
