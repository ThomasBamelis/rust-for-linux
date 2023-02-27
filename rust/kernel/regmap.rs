// SPDX-License-Identifier: GPL-2.0

//! This module provides safer and higher level abstraction over the kernel's SPI types
//! and functions.
//!
//! C header: [`include/linux/spi/spi.h`](../../../../include/linux/spi/spi.h)

use crate::c_str;
use crate::bindings;
use crate::error::{code::*, Error, Result, to_result};
use crate::str::{CStr};
use alloc::boxed::Box;
use core::pin::Pin;
use core::convert::{From, Into};

pub struct RegmapConfig {
    ///  Optional name of the regmap. Useful when a device has multiple
    /// register regions.
    name: Option<&'static CStr>,
    reg_bits: ,

}

impl core::default::Default for RegmapConfig {
    fn default() -> Self {
        RegmapConfig {
            name: None
        }
    }
}

impl RegmapConfig {
    pub fn lol(&mut self) {
        self.0.
    }
}

impl Into<bindings::regmap_config> for Regmap {
    fn into(self) -> bindings::regmap_config {
        bindings::regmap_config {
            name: (),
            reg_bits: (),
            reg_stride: (),
            reg_downshift: (),
            reg_base: (),
            pad_bits: (),
            val_bits: (),
            writeable_reg: (),
            readable_reg: (),
            volatile_reg: (),
            precious_reg: (),
            writeable_noinc_reg: (),
            readable_noinc_reg: (),
            disable_locking: (),
            lock: (),
            unlock: (),
            lock_arg: (),
            reg_read: (),
            reg_write: (),
            reg_update_bits: (),
            read: (),
            write: (),
            max_raw_read: (),
            max_raw_write: (),
            fast_io: (),
            io_port: (),
            max_register: (),
            wr_table: (),
            rd_table: (),
            volatile_table: (),
            precious_table: (),
            wr_noinc_table: (),
            rd_noinc_table: (),
            reg_defaults: (),
            num_reg_defaults: (),
            cache_type: (),
            reg_defaults_raw: (),
            num_reg_defaults_raw: (),
            read_flag_mask: (),
            write_flag_mask: (),
            zero_flag_mask: (),
            use_single_read: (),
            use_single_write: (),
            use_relaxed_mmio: (),
            can_multi_write: (),
            reg_format_endian: (),
            val_format_endian: (),
            ranges: (),
            num_ranges: (),
            use_hwlock: (),
            use_raw_spinlock: (),
            hwlock_id: (),
            hwlock_mode: (),
            can_sleep: ()
        }
    }
}


/// Wrapper struct around the kernel's `spi_device`.
pub struct Regmap(*mut bindings::regmap);

impl Regmap {

    /// Precondition: ptr is a valid pointer to a regmap struct and never
    /// again after.
    pub unsafe fn from_ptr(ptr: *mut bindings::regmap) -> Self {
        Self(ptr)
    }

    /// Returns the raw pointer.
    /// Not pub so the pointer never gets handed of again.
    unsafe fn to_ptr(&mut self) -> *mut bindings::regmap {
        self.0
    }

    // mut because reading from addresses can have side effects (resetting flags)
    pub fn regmap_read(&mut self, reg: u32) -> Result<u32> {
        let mut val: u32 = 0;
        // Safety:
        match unsafe {bindings::regmap_read(self.to_ptr(), reg, &mut val)} {
            0 => Ok(val),
            e => Err(Error::from_kernel_errno(e))
        }
    }

    pub fn regmap_write(&mut self, reg: u32, val: u32) -> Result<()> {
        to_result(unsafe {bindings::regmap_write(self.to_ptr(), reg, val)})
    }

    /// Safety: by consuming the Regmap, the pointer shall not be used again
    /// under the from_ptr preconditions.
    fn regmap_exit(self) {
        unsafe {bindings::regmap_exit(self.0)}
    }
}