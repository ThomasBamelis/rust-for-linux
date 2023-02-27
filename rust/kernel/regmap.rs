// SPDX-License-Identifier: GPL-2.0

//! This module provides safer and higher level abstraction over the kernel's SPI types
//! and functions.
//!
//! C header: [`include/linux/spi/spi.h`](../../../../include/linux/spi/spi.h)

/*
    TODO: make impl from for i2c/spi/ struct. Put TODO there to make it consume the i2c_device etc.. from other pull requests.
    in the case of the latter, you won't be able to use the trait because it can't get the regmap?
    Or make wrapper struct that will work in both cases?
    
    TODO: figure out what types to give config fields and which to include or not.
    unimplemented ones just become None in RegmapConfig
    

    TODO: much later, implement dev managened one and allow for configs which require dev argument
 */

use crate::c_str;
use crate::bindings;
use crate::error::{code::*, Error, Result, to_result};
use crate::str::{CStr};
use alloc::boxed::Box;
use core::pin::Pin;
use core::convert::{From, Into};

/**
Configuration for indirectly accessed or paged registers.

Registers, mapped to this virtual range, are accessed in two steps:
    1. page selector register update;
    2. access through data windowregisters.
 */
pub struct RegmapRangeConfig {
    /// Descriptive name for diagnostics
    name: &'static CStr,
    /// Address of the lowest register address in virtual range.
	range_min: u32,
    /// Address of the highest register in virtual range.
	range_max: u32,
    /// Register with selector field.
	selector_reg: u32,
    /// Bit mask for selector value.
	selector_mask: u32,
    /// Bit shift for selector value.
	selector_shift: i32,
    /// Address of first (lowest) register in data window.
	window_start: u32,
    /// Number of registers in data window.
	window_len: u32,
}

pub struct RegmapConfig {
    ///  Optional name of the regmap. Useful when a device has multiple
    /// register regions.
    name: Option<&'static CStr>,
    reg_bits: ,
    ranges: &[RegmapRangeConfig],

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
            writeable_reg: None,
            readable_reg: None,
            volatile_reg: None,
            precious_reg: None,
            writeable_noinc_reg: None,
            readable_noinc_reg: None,
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