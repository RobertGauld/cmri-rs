//! Handling of CMRInet packets.

mod data;
mod error;
#[expect(clippy::module_inception)]
mod packet;
mod payload;
mod raw;

pub use data::Data;
pub use error::Error;
pub use packet::Packet;
pub use payload::Payload;
pub use raw::Raw;
