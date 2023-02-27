// SPDX-License-Identifier: GPL-2.0

//! This module provides safer and higher level abstraction over the kernel's SPI types
//! and functions.
//!
//! C header: [`include/linux/spi/spi.h`](../../../../include/linux/spi/spi.h)

use crate::bindings;
use crate::error::{code::*, Error, Result};
use crate::str::CStr;
use alloc::boxed::Box;
use core::pin::Pin;

/// Wrapper struct around the kernel's `spi_device`.
pub struct Regmap(*mut bindings::regmap);