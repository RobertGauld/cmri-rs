//! Handling of frames on a CMRInet network.

mod error;
mod raw;

pub use error::{DecodeError, ReceiveError, Full};
pub use raw::Raw;

/// Value of a Synchronization byte in a frame.
const SYN: u8 = 0xFF;
/// Value of a Start-of-Text byte in a frame.
const STX: u8 = 0x02;
/// Value of a End-of-Text byte in a frame.
const ETX: u8 = 0x03;
/// Value of a Data-Link-Escape byte in a frame.
const DLE: u8 = 0x10;
