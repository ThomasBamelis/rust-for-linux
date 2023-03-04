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

use crate::bindings;
use crate::device::Device;
use crate::error::{Error, Result, to_result, from_kernel_err_ptr};
use crate::str::CStr;
use core::ffi::c_void;
use core::ptr;

#[cfg(CONFIG_REGMAP_MMIO)]
use crate::io_mem::IoMem;

pub enum RegcacheType {
    None,
    RBTREE,
    COMPRESSED,
    FLAT
}

/// Default value for a register.
pub struct RegDefault {
    /// Register address.
    reg: u32,
    /// Register default value.
    def: u32
}

#[repr(C)]
pub struct RegmapRange {
    range_min: u32,
    range_max: u32
}

pub struct RegmapAccessTable<'a> {
    yes_ranges: &'a[RegmapRange],
    no_ranges: &'a[RegmapRange]
}

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

/// Configuration for the register map of a device.
/// A value of None for an Option<i32> or Option<u32> will
/// default to 0 unless otherwise specified.
pub struct RegmapConfig<'a> {
    ///  Optional name of the regmap. Useful when a device has multiple
    /// register regions.
    name: Option<&'static CStr>,
    /// Number of bits in a register address, mandatory.
    reg_bits: i32,
    /// The register address stride. Valid register addresses are a
    /// multiple of this value. If set to None, a value of 1 will be
    /// used.
    reg_stride: Option<i32>,
    /// The number of bits to downshift the register before
    /// performing any operations.
    reg_downshift: Option<i32>,
    /// Value to be added to every register address before performing any
    /// operation.
    reg_base: Option<u32>,
    /// Number of bits of padding between register and value.
    pad_bits: Option<i32>,
    /// Number of bits in a register value, mandatory.
    val_bits: i32,
    /*
    /// Optional callback returning true if the register
    /// can be written to. If this field is NULL but wr_table
    /// (see below) is not, the check is performed on such table
    /// (a register is writeable if it belongs to one of the ranges
    /// specified by wr_table).
    writeable_reg: None,
    /// Optional callback returning true if the register
    /// can be read from. If this field is NULL but rd_table
    /// (see below) is not, the check is performed on such table
    /// (a register is readable if it belongs to one of the ranges
    /// specified by rd_table).
    readable_reg: None,
    /// Optional callback returning true if the register
    /// value can't be cached. If this field is NULL but
    /// volatile_table (see below) is not, the check is performed on
    /// such table (a register is volatile if it belongs to one of
    /// the ranges specified by volatile_table).
    volatile_reg: None,
    /// Optional callback returning true if the register
    /// should not be read outside of a call from the driver
    /// (e.g., a clear on read interrupt status register). If this
    /// field is NULL but precious_table (see below) is not, the
    /// check is performed on such table (a register is precious if
    /// it belongs to one of the ranges specified by precious_table).
    precious_reg: None,
    /// Optional callback returning true if the register
    /// supports multiple write operations without incrementing
    /// the register number. If this field is NULL but
    /// wr_noinc_table (see below) is not, the check is
    /// performed on such table (a register is no increment
    /// writeable if it belongs to one of the ranges specified
    /// by wr_noinc_table).
    writeable_noinc_reg: None,
    /// Optional callback returning true if the register
    /// supports multiple read operations without incrementing
    /// the register number. If this field is NULL but
    /// rd_noinc_table (see below) is not, the check is
    /// performed on such table (a register is no increment
    /// readable if it belongs to one of the ranges specified
    /// by rd_noinc_table).
    readable_noinc_reg: None,
    */
    /// This regmap is either protected by external means or
    /// is guaranteed not to be accessed from multiple threads.
    /// Don't use any locking mechanisms.
    disable_locking: bool,
    /*
    /// Optional lock callback (overrides regmap's default lock
    /// function, based on spinlock or mutex).
    lock: (),
    /// As above for unlocking.
    unlock: (),
    /// this field is passed as the only argument of lock/unlock
    /// functions (ignored in case regular lock/unlock functions
    /// are not overridden).
    lock_arg: (),
    /// Optional callback that if filled will be used to perform
    /// all the reads from the registers. Should only be provided for
    /// devices whose read operation cannot be represented as a simple
    /// read operation on a bus such as SPI, I2C, etc. Most of the
    /// devices do not need this.
    reg_read: (),
    /// Same as reg_read for writing.
    reg_write: (),
    /// Optional callback that if filled will be used to perform
    /// all the update_bits(rmw) operation. Should only be provided
    /// if the function require special handling with lock and reg
    /// handling and the operation cannot be represented as a simple
    /// update_bits operation on a bus such as SPI, I2C, etc.
    reg_update_bits: (),
    /// Optional callback that if filled will be used to perform all the
    /// bulk reads from the registers. Data is returned in the buffer used
    /// to transmit data.
    read: (),
    /// Same as read for writing.
    write: (),
    */
    /// Max raw read size that can be used on the device.
    max_raw_read: Option<usize>,
    /// Max raw write size that can be used on the device.
    max_raw_write: Option<usize>,
    /// Register IO is fast. Use a spinlock instead of a mutex
    /// to perform locking. This field is ignored if custom lock/unlock
    /// functions are used (see fields lock/unlock of struct regmap_config).
    /// This field is a duplicate of a similar file in
    /// 'struct regmap_bus' and serves exact same purpose.
    /// Use it onle for "no-bus" cases.
    fast_io: Option<bool>,
    /// Support IO port accessors. Makes sense only when MMIO vs. IO port
    /// access can be distinguished.
    io_port: Option<bool>,
    /// Optional, specifies the maximum valid register address.
    max_register: Option<u32>,
    /// Optional, points to a struct regmap_access_table specifying
    /// valid ranges for write access.
    wr_table: Option<RegmapAccessTable<'a>>,
    /// As above, for read access.
    rd_table: Option<RegmapAccessTable<'a>>,
    /// As above, for volatile registers.
    volatile_table: Option<RegmapAccessTable<'a>>,
    /// As above, for precious registers.
    precious_table: Option<RegmapAccessTable<'a>>,
    /// As above, for no increment writeable registers.
    wr_noinc_table: Option<RegmapAccessTable<'a>>,
    /// As above, for no increment readable registers.
    rd_noinc_table: Option<RegmapAccessTable<'a>>,
    /// Power on reset values for registers (for use with
    /// register cache support).
    reg_defaults: Option<&'a[RegDefault]>,
    /// The actual cache type.
    cache_type: RegcacheType,
    /// Power on reset values for registers (for use with
    /// register cache support).
    reg_defaults_raw: (),
    /// Number of elements in reg_defaults_raw.
    num_reg_defaults_raw: (),
    /// Mask to be set in the top bytes of the register when doing
    /// a read.
    read_flag_mask: Option<u32>,
    /// Mask to be set in the top bytes of the register when doing
    /// a write. If both read_flag_mask and write_flag_mask are
    /// empty and zero_flag_mask is not set the regmap_bus default
    /// masks are used.
    write_flag_mask: Option<u32>,
    /// If set, read_flag_mask and write_flag_mask are used even
    /// if they are both empty.
    zero_flag_mask: Option<bool>,
    /// If set, converts the bulk read operation into a series of
    /// single read operations. This is useful for a device that
    /// does not support  bulk read.
    use_single_read: Option<bool>,
    /// If set, converts the bulk write operation into a series of
    /// single write operations. This is useful for a device that
    /// does not support bulk write.
    use_single_write: Option<bool>,
    /// If set, MMIO R/W operations will not use memory barriers.
    /// This can avoid load on devices which don't require strict
    /// orderings, but drivers should carefully add any explicit
    /// memory barriers when they may require them.
    use_relaxed_mmio: Option<bool>,
    /// If set, the device supports the multi write mode of bulk
    /// write operations, if clear multi write requests will be
    /// split into individual write operations
    can_multi_write: Option<bool>,
    /// Endianness for formatted register addresses. If this is
    /// DEFAULT, the @reg_format_endian_default value from the
    /// regmap bus is used.
    reg_format_endian: (),
    /// Endianness for formatted register values. If this is
    /// DEFAULT, the @reg_format_endian_default value from the
    /// regmap bus is used.
    val_format_endian: (),
    /// Array of configuration entries for virtual address ranges.
    ranges: Option<&'a[RegmapRangeConfig]>,
    /// Indicate if a hardware spinlock should be used.
    use_hwlock: Option<bool>,
    /// Indicate if a raw spinlock should be used.
    use_raw_spinlock: Option<bool>,
    /// Specify the hardware spinlock id.
    hwlock_id: Option<u32>,
    /// The hardware spinlock mode, should be HWLOCK_IRQSTATE,
    /// HWLOCK_IRQ or 0.
    hwlock_mode: Option<u32>,
    /// Optional, specifies whether regmap operations can sleep.
    can_sleep: Option<bool>
}

impl<'a> RegmapConfig<'a> {
    pub fn new(reg_bits: i32, val_bits: i32) -> Self {
        todo!()
    }
}

impl<'a> core::default::Default for RegmapConfig<'a> {
    fn default() -> Self {
        RegmapConfig::new(8, 8)
    }
}

/*
impl<'a> Into<bindings::regmap_config> for RegmapConfig<'a> {
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
*/


/// Wrapper struct around the kernel's `spi_device`.
pub struct Regmap<T> {
    ptr: *mut bindings::regmap,
    /// Holds the bus so that it does not get dropped until the regmap gets dropped.
    bus: T
}

#[cfg(CONFIG_REGMAP_MMIO)]
impl<const SIZE: usize> Regmap<IoMem<SIZE>> {
    ///
    /// TODO: does this do iounmap automatically?
    pub fn from_mmio(dev: &mut Device, mmio: IoMem<SIZE>, config: &RegmapConfig<'_>) -> Result<Self> {
        let ptr =
            from_kernel_err_ptr(
                // Safety: device and IOmem are legal
                unsafe{
                    // TODO unsupported for CONFIG_LOCKDEP
                    bindings::__regmap_init_mmio_clk(dev.ptr, ptr::null(), mmio.ptr as *mut c_void, config, ptr::null_mut(), ptr::null())
                }
            )?;
        
        Ok(Self {
            ptr,
            bus: mmio
        })
    }
}

impl<T> Regmap<T> {


    // mut because reading from addresses can have side effects (resetting flags)
    pub fn regmap_read(&mut self, reg: u32) -> Result<u32> {
        let mut val: u32 = 0;
        // Safety:
        match unsafe {bindings::regmap_read(self.ptr, reg, &mut val)} {
            0 => Ok(val),
            e => Err(Error::from_kernel_errno(e))
        }
    }

    pub fn regmap_write(&mut self, reg: u32, val: u32) -> Result<()> {
        to_result(unsafe {bindings::regmap_write(self.ptr, reg, val)})
    }
}

impl<T> Drop for Regmap<T> {
    /// Safety: drop can only be called so it takes ownership (core::mem::drop).
    /// By consuming the Regmap, the pointer shall not be used again
    /// under the from_ptr preconditions.
    fn drop(&mut self) {
        unsafe {bindings::regmap_exit(self.ptr)}
        core::mem::drop(self.bus)
    }
}