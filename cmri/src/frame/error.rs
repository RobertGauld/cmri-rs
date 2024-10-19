/// The errors which can occur on decoding a frame.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::module_name_repetitions)]
pub enum DecodeError {
    /// The frame is too short.
    #[error("Frame is too short")]
    TooShort,

    /// The frame is missing the synchronisation bytes
    #[error("Frame is missing the synchronisation bytes")]
    MissingSynchronisation,

    /// The frame is missing the start byte
    #[error("Frame is missing the start byte")]
    MissingStart,

    /// The frame is missing the end byte
    #[error("Frame is missing the end byte")]
    MissingEnd,

    /// The raw frame is too long.
    #[error("Raw frame is too long")]
    TooLong,

    /// The frame was valid, but contained an invalid packet
    #[error("Invalid packet")]
    InvalidPacket {
        /// How the packet is invalid
        #[from]
        source: crate::packet::Error
    }
}


/// The errors which can occur on receiving a frame.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::module_name_repetitions)]
pub enum ReceiveError {
    /// The frame is too short.
    #[error("Frame is too short")]
    TooShort,

    /// The raw frame is too long.
    #[error("Frame is too long")]
    TooLong,

    /// The frame is already complete.
    #[error("Frame is already complete")]
    AlreadyComplete
}

/// The frame is already full.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Full;
impl core::fmt::Display for Full {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Full")
    }
}
