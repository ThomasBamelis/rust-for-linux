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
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;

#[cfg(CONFIG_REGMAP_MMIO)]
use crate::io_mem::IoMem;

#[derive(Copy, Clone)]
pub enum RegmapEndian {
	RegmapEndianDefault = 0,
	RegmapEndianBig,
	RegmapEndianLittle,
	RegmapEndianNative,
}

impl From<RegmapEndian> for u32 {
    fn from(value: RegmapEndian) -> Self {
        value as u32
    }
}

#[derive(Copy, Clone)]
pub enum RegcacheType {
    None = 0,
    RBTREE,
    COMPRESSED,
    FLAT
}

impl From<RegcacheType> for u32 {
    fn from(value: RegcacheType) -> Self {
        value as u32
    }
}

/// Default value for a register.
type RegDefault = bindings::reg_default;
// // Safety: must have same memory layout as reg_default
// #[repr(C)]
// pub struct RegDefault {
//     /// Register address.
//     reg: u32,
//     /// Register default value.
//     def: u32
// }

type RegmapRange = bindings::regmap_range;
// // Safety: must have same memory layout as regmap_range
// #[repr(C)]
// pub struct RegmapRange {
//     range_min: u32,
//     range_max: u32
// }

pub struct RegmapAccessTable<'a> {
    yes_ranges: &'a[RegmapRange],
    no_ranges: &'a[RegmapRange]
}


impl<'a> RegmapAccessTable<'a> {
    unsafe fn to_binding(&self) -> bindings::regmap_access_table {
        bindings::regmap_access_table {
            yes_ranges: self.yes_ranges.as_ptr(),
            n_yes_ranges: self.yes_ranges.len() as u32,
            no_ranges: self.no_ranges.as_ptr(),
            n_no_ranges: self.no_ranges.len() as u32,
        }
    }
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

impl RegmapRangeConfig {
    /// Converts to binding.
    /// Necessary to wrap const char into CStr.
    /// Safety: self.name must exist as long as the return value
    unsafe fn to_binding(&self) -> bindings::regmap_range_cfg {
        bindings::regmap_range_cfg {
            name: self.name.as_char_ptr(),
            range_min: self.range_min,
            range_max: self.range_max,
            selector_reg: self.selector_reg,
            selector_mask: self.selector_mask,
            selector_shift: self.selector_shift,
            window_start: self.window_start,
            window_len: self.window_len,
        }
    }
}

/// Holds the binding equivalents of the Rust structs in RegmapConfig
struct RegmapConfigBindings {
    wr_table: Option<bindings::regmap_access_table>,
    rd_table: Option<bindings::regmap_access_table>,
    volatile_table: Option<bindings::regmap_access_table>,
    precious_table: Option<bindings::regmap_access_table>,
    wr_noinc_table: Option<bindings::regmap_access_table>,
    rd_noinc_table: Option<bindings::regmap_access_table>,
    ranges: Option<Vec<bindings::regmap_range_cfg>>,
}

/// Configuration for the register map of a device.
/// A value of None for an Option<i32> or Option<u32> will
/// default to 0 unless otherwise specified.
/// Missing options are as of yet unsupported.
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
    /// This regmap is either protected by external means or
    /// is guaranteed not to be accessed from multiple threads.
    /// Don't use any locking mechanisms.
    disable_locking: bool,
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
    /*
    /// Power on reset values for registers (for use with
    /// register cache support).
    reg_defaults_raw: (),
    /// Number of elements in reg_defaults_raw.
    num_reg_defaults_raw: (),
    */
    /// Mask to be set in the top bytes of the register when doing
    /// a read.
    read_flag_mask: Option<u64>,
    /// Mask to be set in the top bytes of the register when doing
    /// a write. If both read_flag_mask and write_flag_mask are
    /// empty and zero_flag_mask is not set the regmap_bus default
    /// masks are used.
    write_flag_mask: Option<u64>,
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
    reg_format_endian: RegmapEndian,
    /// Endianness for formatted register values. If this is
    /// DEFAULT, the @reg_format_endian_default value from the
    /// regmap bus is used.
    val_format_endian: RegmapEndian,
    /// Array of configuration entries for virtual address ranges.
    ranges: Option<&'a[RegmapRangeConfig]>,
    /* TODO unsure about safety
    /// Indicate if a hardware spinlock should be used.
    use_hwlock: Option<bool>,
    /// Indicate if a raw spinlock should be used.
    use_raw_spinlock: Option<bool>,
    /// Specify the hardware spinlock id.
    hwlock_id: Option<u32>,
    /// The hardware spinlock mode, should be HWLOCK_IRQSTATE,
    /// HWLOCK_IRQ or 0.
    hwlock_mode: Option<u32>,
    */
    /// Optional, specifies whether regmap operations can sleep.
    can_sleep: Option<bool>
}

impl<'a> RegmapConfig<'a> {
    pub fn new(reg_bits: i32, val_bits: i32) -> Self {
        RegmapConfig {
            name: None,
            reg_bits,
            reg_stride: None,
            reg_downshift: None,
            reg_base: None,
            pad_bits: None,
            val_bits,
            max_raw_read: None,
            max_raw_write: None,
            fast_io: None,
            io_port: None,
            max_register: None,
            wr_table: None,
            rd_table: None,
            volatile_table: None,
            precious_table: None,
            wr_noinc_table: None,
            rd_noinc_table: None,
            reg_defaults: None,
            cache_type: RegcacheType::None,
            read_flag_mask: None,
            write_flag_mask: None,
            zero_flag_mask: None,
            use_single_read: None,
            use_single_write: None,
            use_relaxed_mmio: None,
            can_multi_write: None,
            reg_format_endian: RegmapEndian::RegmapEndianDefault,
            val_format_endian: RegmapEndian::RegmapEndianDefault,
            ranges: None,
            can_sleep: None,
        }
    }

    ///
    /// Transform to regmap_config binding.
    /// None values inidicate fields that are yet unsupported.
    unsafe fn to_binding(&self) -> Result<(bindings::regmap_config, RegmapConfigBindings)> {
        unsafe {
            let binds = RegmapConfigBindings {
                wr_table: self.wr_table.as_ref().map(|r| r.to_binding()),
                rd_table: self.rd_table.as_ref().map(|r| r.to_binding()),
                volatile_table: self.volatile_table.as_ref().map(|r| r.to_binding()),
                precious_table: self.precious_table.as_ref().map(|r| r.to_binding()),
                wr_noinc_table: self.wr_noinc_table.as_ref().map(|r| r.to_binding()),
                rd_noinc_table: self.rd_noinc_table.as_ref().map(|r| r.to_binding()),
                ranges: if let Some(r) = self.ranges {
                        let mut v = Vec::try_with_capacity(r.len())?;
                        for i in r {
                            v.try_push(i.to_binding())?
                        }
                        Some(v)
                    }
                    else {
                        None
                    }
            };
            let cfg = bindings::regmap_config {
                name: if let Some(n) = self.name {n.as_char_ptr()} else {ptr::null()},
                reg_bits: self.reg_bits,
                reg_stride: self.reg_stride.unwrap_or(0),
                reg_downshift: self.reg_downshift.unwrap_or(0),
                reg_base: self.reg_base.unwrap_or(0),
                pad_bits: self.pad_bits.unwrap_or(0),
                val_bits: self.val_bits,
                writeable_reg: None,
                readable_reg: None,
                volatile_reg: None,
                precious_reg: None,
                writeable_noinc_reg: None,
                readable_noinc_reg: None,
                disable_locking: false,
                lock: None,
                unlock: None,
                lock_arg: ptr::null_mut(),
                reg_read: None,
                reg_write: None,
                reg_update_bits: None,
                read: None,
                write: None,
                max_raw_read: self.max_raw_read.unwrap_or(0),
                max_raw_write: self.max_raw_write.unwrap_or(0),
                fast_io: self.fast_io.unwrap_or(false),
                io_port: self.io_port.unwrap_or(false),
                max_register: self.max_register.unwrap_or(0),
                wr_table: if let Some(x) = binds.wr_table.as_ref() {x} else {ptr::null()},
                rd_table: if let Some(x) = binds.rd_table.as_ref() {x} else {ptr::null()},
                volatile_table: if let Some(x) = binds.volatile_table.as_ref() {x} else {ptr::null()},
                precious_table: if let Some(x) = binds.precious_table.as_ref() {x} else {ptr::null()},
                wr_noinc_table: if let Some(x) = binds.wr_noinc_table.as_ref() {x} else {ptr::null()},
                rd_noinc_table: if let Some(x) = binds.rd_noinc_table.as_ref() {x} else {ptr::null()},
                reg_defaults: if let Some(x) = self.reg_defaults.as_ref() {x.as_ptr()} else {ptr::null()},
                num_reg_defaults: if let Some(x) = self.reg_defaults.as_ref() {x.len() as u32} else {0},
                cache_type: self.cache_type.into(), 
                reg_defaults_raw: ptr::null(),
                num_reg_defaults_raw: 0,
                read_flag_mask: self.read_flag_mask.unwrap_or(0),
                write_flag_mask: self.write_flag_mask.unwrap_or(0),
                zero_flag_mask: self.zero_flag_mask.unwrap_or(false),
                use_single_read: self.use_single_read.unwrap_or(false),
                use_single_write: self.use_single_write.unwrap_or(false),
                use_relaxed_mmio: self.use_relaxed_mmio.unwrap_or(false), // TODO unsafe?
                can_multi_write: self.can_multi_write.unwrap_or(false),
                reg_format_endian: self.reg_format_endian.into(),
                val_format_endian: self.val_format_endian.into(),
                ranges: if let Some(x) = binds.ranges.as_ref() {x.as_ptr()} else {ptr::null()},
                num_ranges: if let Some(x) = self.ranges.as_ref() {x.len() as u32} else {0},
                use_hwlock: false,
                use_raw_spinlock: false, // TODO unsafe?
                hwlock_id: 0,
                hwlock_mode: 0,
                can_sleep: self.can_sleep.unwrap_or(false)
            };
            Ok((cfg, binds))
        }
    }
}


/// Holds a Regmap device
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
                // Safety: for the currently supported options for the config, no field of the
                // config has to exist after the regmap has been initialised
                unsafe{
                    // TODO unsupported for CONFIG_LOCKDEP
                    let (config, internal_bindings) = config.to_binding()?;
                    bindings::__regmap_init_mmio_clk(dev.ptr, ptr::null(), mmio.ptr as *mut c_void, &config, ptr::null_mut(), ptr::null())
                }
            )?;
        
        Ok(Self {
            ptr,
            bus: mmio
        })
    }
}

impl<T> Regmap<T> {

    // TODO there is a trait already for from_ptr, use it if necessary

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
        //core::mem::drop(self.bus)
    }
}
