//! The configuration of nodes on a CMRInet.

mod node_sort;
mod cp;
mod smini;
mod sic;

pub use node_sort::*;
pub use cp::{CpnodeConfiguration, CpnodeOptions, CpmegaConfiguration, CpmegaOptions, Error as CpConfigurationError};
pub use smini::{Configuration as SminiConfiguration, Error as SminiConfigurationError};
pub use sic::{UsicConfiguration, SusicConfiguration, node_cards::Error as NodeCardsError, node_cards};

/// NDP for a Classic USIC or SUSIC with 0-1536 inputs/outputs using 24 bit cards.
const NDP_USIC: u8 = b'N';
/// NDP for a SUSIC with 0-2048 inputs/outputs using 32 bit cards.
const NDP_SUSIC: u8 = b'X';
/// NDP for a SMINI with fixed 24 inputs and 48 outputs.
const NDP_SMINI: u8 = b'M';
/// NDP for a CPNODE has 16-144 inputs/outputs using 8 bit cards.
const NDP_CPNODE: u8 = b'C';
/// NDP for a CPMEGA has 0-192 inputs/outputs using 8 bit cards.
const NDP_CPMEGA: u8 = b'O';


/// Information common to the configuration of all node types.
pub trait NodeConfiguration {
    /// The time the node should leave between receiving a request
    /// and sending the reply (to a precision of 10µs, upto 655,350µs).
    fn transmit_delay(&self) -> u16;

    /// The number of input bytes on the node.
    fn input_bytes(&self) -> u16;

    /// The number of output bytes on the node.
    fn output_bytes(&self) -> u16;

    /// The number of input bits on the node.
    fn input_bits(&self) -> u16 { self.input_bytes() * 8 }

    /// The number of output bits on the node.
    fn output_bits(&self) -> u16 { self.output_bytes() * 8 }
}


/// The errors which can occur when decoding/creating a node's initialization data within a packet.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum InvalidConfigurationError {
    #[error(transparent)]
    Sic {
        #[from]
        source: crate::node_configuration::sic::node_cards::Error,
    },
    #[error(transparent)]
    Smini {
        #[from]
        source: crate::node_configuration::SminiConfigurationError,
    },
    #[error(transparent)]
    Cp {
        #[from]
        source: crate::node_configuration::CpConfigurationError,
    },
}
