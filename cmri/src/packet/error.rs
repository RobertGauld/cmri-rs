/// The errors which can occur when decoding/creating a packet.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// The node address is outside the valid range of 0-127.
    #[error("Invalid Node Address (must be 0-127): {0:?}")]
    InvalidNodeAddress(u8),

    /// The unit address is outside the valid range of 65-192.
    #[error("Invalid Unit Address (must be 65-192): {0:?}")]
    InvalidUnitAddress(u8),

    #[cfg_attr(not(feature = "experimenter"), doc = "The message type within the packet is not of a known type.")]
    #[cfg_attr(feature = "experimenter", doc = "The message type is not an uppercase ASCII character.")]
    #[error("Invalid Message Type: {0:?}")]
    InvalidMessageType(u8),

    /// The packet is too short.
    #[error("Too short")]
    TooShort,

    /// The packet is too long.
    #[error("Too long")]
    TooLong,

    /// The body within the packet is too long.
    #[error("Body too long")]
    BodyTooLong,

    /// The node type (within an initialization packet) is not of a known type.
    #[error("Invalid Node Type: {0:?}")]
    InvalidNodeType(u8),

    /// The configuration within an initiasation packet is invalid.
    #[error("Invalid configuration")]
    InvalidConfiguration {
        /// What the error actually is.
        #[from]
        source: crate::node_configuration::InvalidConfigurationError,
    }
}
