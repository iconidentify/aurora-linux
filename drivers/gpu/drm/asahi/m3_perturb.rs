// SPDX-License-Identifier: GPL-2.0-only OR MIT

//! `asahi.m3_perturb=N`: sensitivity runs of the T6030 runtime backend.
//!
//! Each nonzero value changes exactly one configuration item the runtime gives the GPU
//! firmware, or one write it makes to the GPU's own registers, to a deliberately wrong but
//! bounded value. A run then shows how the firmware fails when that one item is wrong. The
//! default, 0, changes nothing and logs nothing.
//!
//! The perturbations only shrink or remove firmware IO mappings, make one read-only, move one
//! read-only mapping inside the GPU's own register window, clear bits of the HwDataB unit masks,
//! or skip the SGX setup write. None of them touches a voltage, a performance-state table, a
//! power or thermal limit, or the power-management words of the InitData.
//!
//! They apply on T6030 only. On any other SoC the parameter is logged and ignored.

use kernel::{device, prelude::*};

use crate::{
    initdata::G15RuntimeHwDataB,
    m3_adt_config::T6030_IO_MAPPINGS,
    m3_init_storage::{IoMap, T6030_IOMAPS},
    m3_soc::{IoMapping, Soc, T6030_HWDATA_B},
    pgtable::{prot, Prot},
};

/// The HwDataB IO-mapping slot of the GFX clock generator.
const CLOCK_GEN_SLOT: usize = 29;
/// The HwDataB IO-mapping slot of the Fender window.
const FENDER_SLOT: usize = 0;
/// Index of each of those slots in `T6030_IO_MAPPINGS` and `T6030_IOMAPS`.
const CLOCK_GEN_INDEX: usize = 14;
const FENDER_INDEX: usize = 0;

/// The Fender window of `fender-window`: the T6030 window less 0x40000 (0x144000 -> 0x104000).
const FENDER_WINDOW: u32 = 0x10_4000;
/// The clock-generator block of `clkgen-moved`: SGX+0xe1c000 instead of SGX+0xe5c000, a
/// 16 KiB block inside the same SGX window.
const CLOCK_GEN_MOVED: u64 = 0x2_90e1_c000;

const _: () = {
    assert!(T6030_IO_MAPPINGS.len() == CLOCK_GEN_INDEX + 1);
    assert!(T6030_IOMAPS.len() == CLOCK_GEN_INDEX + 1);
    assert!(T6030_IO_MAPPINGS[CLOCK_GEN_INDEX].0 == CLOCK_GEN_SLOT);
    assert!(T6030_IOMAPS[CLOCK_GEN_INDEX].slot == CLOCK_GEN_SLOT);
    assert!(T6030_IO_MAPPINGS[CLOCK_GEN_INDEX].1 == 0x2_90e5_c000);
    assert!(T6030_IOMAPS[CLOCK_GEN_INDEX].physical == 0x2_90e5_c000);
    assert!(!T6030_IO_MAPPINGS[CLOCK_GEN_INDEX].4);
    assert!(T6030_IO_MAPPINGS[FENDER_INDEX].0 == FENDER_SLOT);
    assert!(T6030_IOMAPS[FENDER_INDEX].slot == FENDER_SLOT);
    assert!(T6030_IO_MAPPINGS[FENDER_INDEX].2 == 0x14_4000);
    assert!(T6030_IOMAPS[FENDER_INDEX].size == 0x14_4000);
    // The moved block stays inside the SGX window and keeps its size and VA.
    assert!(CLOCK_GEN_MOVED & 0x3fff == 0);
    assert!(CLOCK_GEN_MOVED >= 0x2_9000_0000 && CLOCK_GEN_MOVED + 0x4000 <= 0x2_9400_0000);
    // The shrunk window is page-aligned and still covers the Fender scratch SRAM
    // (`HwConfig::sram_base`/`sram_size` of T6030: Fender+0x60000..+0x80000).
    assert!(FENDER_WINDOW & 0x3fff == 0 && FENDER_WINDOW >= 0x8_0000);
};

static FENDER_IO_MAPPINGS: [IoMapping; 15] = {
    let mut m = T6030_IO_MAPPINGS;
    m[FENDER_INDEX].2 = FENDER_WINDOW;
    m[FENDER_INDEX].3 = FENDER_WINDOW;
    m
};
static FENDER_IOMAPS: [IoMap; 15] = {
    let mut m = T6030_IOMAPS;
    m[FENDER_INDEX].size = FENDER_WINDOW as usize;
    m
};
static CLOCK_GEN_MOVED_IO_MAPPINGS: [IoMapping; 15] = {
    let mut m = T6030_IO_MAPPINGS;
    m[CLOCK_GEN_INDEX].1 = CLOCK_GEN_MOVED;
    m
};
static CLOCK_GEN_MOVED_IOMAPS: [IoMap; 15] = {
    let mut m = T6030_IOMAPS;
    m[CLOCK_GEN_INDEX].physical = CLOCK_GEN_MOVED;
    m
};
static T6030_IO_MAPPINGS_STATIC: [IoMapping; 15] = T6030_IO_MAPPINGS;
static T6030_IOMAPS_STATIC: [IoMap; 15] = T6030_IOMAPS;

/// `unit-mask-a`: the low word with bit 1 cleared (0x7_0000_0003 -> 0x7_0000_0001).
static UNIT_MASK_A_HWDATA_B: G15RuntimeHwDataB = G15RuntimeHwDataB {
    unit_mask_a: 0x7_0000_0001,
    ..T6030_HWDATA_B
};
/// `unit-mask-b`: bit 2 cleared (7 -> 3).
static UNIT_MASK_B_HWDATA_B: G15RuntimeHwDataB = G15RuntimeHwDataB {
    unit_mask_b: 3,
    ..T6030_HWDATA_B
};

/// The perturbation of one boot. The discriminants are the `asahi.m3_perturb` values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Perturb {
    None = 0,
    /// Map the clock-generator block (slot 29) read-only for the firmware, at its address.
    ClockGenReadOnly = 1,
    /// Shrink the Fender window (slot 0) from 0x144000 to 0x104000 bytes.
    FenderWindow = 2,
    /// Give the firmware no clock-generator mapping (slot 29 left empty).
    ClockGenAbsent = 3,
    /// Skip the SGX setup write made before the firmware starts.
    SkipSgxSetup = 4,
    /// HwDataB `unit_mask_a` 0x7_0000_0003 -> 0x7_0000_0001.
    UnitMaskA = 5,
    /// HwDataB `unit_mask_b` 7 -> 3.
    UnitMaskB = 6,
    /// Map SGX+0xe1c000 as the clock-generator block (slot 29), read-only.
    ClockGenMoved = 7,
}

/// The highest `asahi.m3_perturb` value.
pub(crate) const LAST: u64 = Perturb::ClockGenMoved as u64;

impl Perturb {
    fn from_param(value: u64) -> Self {
        match value {
            1 => Self::ClockGenReadOnly,
            2 => Self::FenderWindow,
            3 => Self::ClockGenAbsent,
            4 => Self::SkipSgxSetup,
            5 => Self::UnitMaskA,
            6 => Self::UnitMaskB,
            7 => Self::ClockGenMoved,
            _ => Self::None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ClockGenReadOnly => "clkgen-ro",
            Self::FenderWindow => "fender-window",
            Self::ClockGenAbsent => "clkgen-absent",
            Self::SkipSgxSetup => "sgx-setup-skip",
            Self::UnitMaskA => "unit-mask-a",
            Self::UnitMaskB => "unit-mask-b",
            Self::ClockGenMoved => "clkgen-moved",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::None => "no change",
            Self::ClockGenReadOnly => {
                "IO mapping slot 29 (GFX clock generator, SGX+0xe5c000) mapped read-only"
            }
            Self::FenderWindow => "IO mapping slot 0 (Fender, SGX+0xd00000) 0x144000 -> 0x104000 bytes",
            Self::ClockGenAbsent => "IO mapping slot 29 (GFX clock generator) left empty",
            Self::SkipSgxSetup => "SGX setup write (SGX+0xd14000 = 0x70001) skipped",
            Self::UnitMaskA => "HwDataB unit_mask_a 0x700000003 -> 0x700000001",
            Self::UnitMaskB => "HwDataB unit_mask_b 0x7 -> 0x3",
            Self::ClockGenMoved => {
                "IO mapping slot 29 (GFX clock generator) SGX+0xe5c000 -> SGX+0xe1c000, read-only"
            }
        }
    }

    /// The firmware IO mappings of the InitData (`Soc::io_mappings`).
    pub(crate) fn io_mappings(self, soc: &Soc) -> Option<&'static [IoMapping]> {
        match self {
            Self::FenderWindow => Some(&FENDER_IO_MAPPINGS),
            Self::ClockGenAbsent => Some(&T6030_IO_MAPPINGS_STATIC[..CLOCK_GEN_INDEX]),
            Self::ClockGenMoved => Some(&CLOCK_GEN_MOVED_IO_MAPPINGS),
            _ => soc.io_mappings,
        }
    }

    /// The runtime's mappings of those IO mappings (`Soc::iomaps`).
    pub(crate) fn iomaps(self, soc: &Soc) -> Option<&'static [IoMap]> {
        match self {
            Self::FenderWindow => Some(&FENDER_IOMAPS),
            Self::ClockGenAbsent => Some(&T6030_IOMAPS_STATIC[..CLOCK_GEN_INDEX]),
            Self::ClockGenMoved => Some(&CLOCK_GEN_MOVED_IOMAPS),
            _ => soc.iomaps,
        }
    }

    /// The register block the InitData IO mapping of `slot` must lie in, given the HwConfig's
    /// block `base` for it.
    pub(crate) fn io_block(self, slot: usize, base: u64) -> u64 {
        match self {
            Self::ClockGenMoved if slot == CLOCK_GEN_SLOT => CLOCK_GEN_MOVED,
            _ => base,
        }
    }

    /// The firmware's page protection of the IO map of `slot`.
    pub(crate) fn mmio_prot(self, slot: usize) -> Prot {
        match self {
            Self::ClockGenReadOnly | Self::ClockGenMoved if slot == CLOCK_GEN_SLOT => {
                prot::PROT_FW_MMIO_RO
            }
            _ => prot::PROT_FW_MMIO_RW,
        }
    }

    /// The runtime HwDataB configuration words and unit masks (`Soc::hwdata_b`).
    pub(crate) fn hwdata_b(self, soc: &Soc) -> Option<&'static G15RuntimeHwDataB> {
        match self {
            Self::UnitMaskA => Some(&UNIT_MASK_A_HWDATA_B),
            Self::UnitMaskB => Some(&UNIT_MASK_B_HWDATA_B),
            _ => soc.hwdata_b,
        }
    }

    /// Whether the SGX setup write is skipped.
    pub(crate) fn skip_sgx_setup(self) -> bool {
        self == Self::SkipSgxSetup
    }
}

/// The perturbation of this boot for `soc`: `asahi.m3_perturb` on T6030, none elsewhere.
pub(crate) fn active(soc: &Soc) -> Perturb {
    if soc.chip_id != 0x6030 {
        return Perturb::None;
    }
    Perturb::from_param(crate::m3_params::perturb_param())
}

/// Log the perturbation of this boot, once at probe. Silent when `asahi.m3_perturb` is 0.
pub(crate) fn log(dev: &device::Device, soc: &Soc) {
    let value = crate::m3_params::perturb_param();
    if value == 0 {
        return;
    }
    let perturb = active(soc);
    if perturb == Perturb::None {
        dev_info!(
            dev,
            "M3 perturb: asahi.m3_perturb={} ignored on {}; the sensitivity runs are T6030-only\n",
            value,
            soc.name
        );
        return;
    }
    dev_warn!(
        dev,
        "M3 perturb: asahi.m3_perturb={} ({}): {}\n",
        value,
        perturb.name(),
        perturb.describe()
    );
}
