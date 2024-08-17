use core::fmt::Write;
use log::trace;
use crate::Address;
use crate::packet::{Packet, Error as PacketError, Raw as RawPacket};
use super::{SYN, STX, DLE, ETX, DecodeError, ReceiveError, Full};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ReceiveFrameState {
    WaitingForSyn,
    WaitingForSynSyn,
    WaitingForSynSynStx,
    Receiving,
    ReceivingEscaped,
    Received
}

/// Holds a packet as it appears on the CMRInet network.
///
/// # Examples
///
/// ## Sending a frame
///
/// ```
/// use cmri::{frame::Raw, Address};
///
/// // Build a transmit data frame to node 5.
/// let address = Address::try_from_node_address(5).unwrap();
/// let mut frame = Raw::new();
/// frame.begin(address, b'T');
/// frame.push(127); // Byte 0 - nothing special.
/// frame.push(16);  // Byte 1 - happens to have special meaning on a CMRInet bus,
///                  // so the frame grows by 2 byts as it needs escaping.
/// frame.finish();
/// // Now write the data to a connection.
/// ```
///
/// ## Receiving a frame
///
/// ```
/// use cmri::frame::Raw;
/// let mut frame = Raw::new();
///
/// // Read the data from a connection.
/// let buffer = [0xFF, 0xFF, 0x02, 75, b'T', 127, 0x10, 16, 0x03];
///
/// // Then build the frame.
/// for byte in buffer {
///     frame.receive(byte); // Will return Ok(true) when a complete frame has been received.
/// }
///
/// // Now convert the frame to a packet fur further use.
/// let packet = frame.try_as_packet().unwrap();
/// ```
///
/// ## Microcontroller
///
/// When running on a microcontroller you may want to save the
/// effort of decoding packets that aren't for you.
/// ```
/// use cmri::{Address, packet::{Packet, Payload}, frame::Raw};
///
/// let address = Address::try_from_node_address(2).unwrap();
/// let received = &[0xFF, 0xFF, 0x02, 67, b'P', 0x03];
/// let raw_frame: Raw = received.try_into().unwrap();
/// if raw_frame.address() == Some(2) {
///     assert_eq!(
///         raw_frame.try_as_packet().unwrap(),
///         Packet::new_poll_request(address)
///     )
/// } else {
///     panic!("Not for us");
/// }
/// ```
#[derive(Clone, Copy, Eq)]
pub struct Raw {
    len: usize,
    raw: [u8; Self::MAX_LEN],
    receive_state: ReceiveFrameState,
    packet_len: usize
}

crate::raw_structs::common_implementation!(Raw, 518);

impl Raw {
    /// Create a new, empty Raw.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            raw: [0; Self::MAX_LEN],
            len: 0,
            receive_state: ReceiveFrameState::WaitingForSyn,
            packet_len: 0
        }
    }

    /// Get the node address the contained packet is sent to/from (if it's valid).
    #[must_use]
    pub fn address(&self) -> Option<u8> {
        Address::try_from_unit_address(self.raw[3])
            .map(|a| a.as_node_address())
            .ok()
    }

    /// Get the message type of the contained packet (if it's valid).
    #[must_use]
    pub fn message_type(&self) -> Option<char> {
        #[cfg(not(feature = "experimenter"))]
        if [b'I', b'P', b'R', b'T'].contains(&self.raw[4]) {
            Some(self.raw[4].into())
        } else {
            None
        }

        #[cfg(feature = "experimenter")]
        if self.raw[4].is_ascii_uppercase() {
            Some(self.raw[4].into())
        } else {
            None
        }
    }

    /// Begin building a Raw, ready for putting onto a CMRInet Network.
    ///
    /// Example:
    /// ```
    /// use cmri::frame::Raw;
    /// use cmri::Address;
    /// let mut raw = Raw::new();
    /// let address = Address::try_from_node_address(0).unwrap();
    /// raw.begin(address, b'T'); // Now contains [SYN, SYN, STX, 65, b'T']
    /// raw.push(16);
    /// raw.finish(); // Now contains a transmit data packet for node 0 (1 byte of 0001 000, which needs to be escaped)
    /// assert_eq!(raw.as_slice(), &[0xFF, 0xFF, 0x02, 65, b'T', 0x10, 16, 0x03]);
    /// ```
    pub fn begin(&mut self, address: Address, message_type: u8) {
        self.raw[0..5].copy_from_slice(&[SYN, SYN, STX, address.as_unit_address(), message_type]);
        self.len = 5;
    }

    /// Add a new byte to the end.
    ///
    /// Returns the number of bytes added.
    ///
    /// # Errors
    ///
    /// If the Raw is full.
    pub fn push(&mut self, value: u8) -> Result<usize, Full> {
        let escape = [SYN, STX, DLE, ETX].contains(&value);
        let count = if escape { 2 } else { 1 };
        if self.available() < count { return Err(Full) }

        if escape {
            self.raw[self.len] = DLE;
            self.len += 1;
        }

        self.raw[self.len] = value;
        self.len += 1;
        Ok(count)
    }

    /// Finishes building a Raw, ready for putting onto a CMRInet Network.
    ///
    /// # Errors
    ///
    /// If the Raw is full.
    pub fn finish(&mut self) -> Result<(), Full> {
        if self.available() < 1 { return Err(Full) }
        self.raw[self.len] = ETX;
        self.len += 1;
        Ok(())
    }

    /// Updates the Raw with a byte received from a CMRInet network.
    ///
    /// Returns whether the frame is now complete.
    ///
    /// # Errors
    ///
    ///   * [`ReceiveError::TooShort`] if the Raw is completed but too short to contain a valid packet.
    ///   * [`ReceiveError::TooLong`] if the Raw is full (the Raw's state is also reset, ready to try receiving a new frame).
    ///   * [`ReceiveError::AlreadyComplete`] if the Raw is already complete.
    ///
    /// # Example
    ///
    /// ```
    /// use cmri::frame::{ReceiveError, Raw};
    /// let mut raw = Raw::new();
    /// let data = [0xFF, 0xFF, 0x02, 65, b'P'];
    /// for byte in data {
    ///     assert_eq!(raw.receive(byte), Ok(false));
    /// }
    /// assert_eq!(raw.receive(0x03), Ok(true));
    /// assert_eq!(raw.as_slice(), &[0xFF, 0xFF, 0x02, 65, b'P', 0x03]);
    /// assert_eq!(raw.receive(0xFF), Err(ReceiveError::AlreadyComplete));
    /// ```
    #[expect(clippy::match_same_arms, reason = "Easier to follow along without combining the noop arms.")]
    pub fn receive(&mut self, byte: u8) -> Result<bool, ReceiveError> {
        let state = self.receive_state;
        let mut accept_byte = |state: ReceiveFrameState| {
            self.raw[self.len] = byte;
            self.len += 1;
            self.receive_state = state;
        };

        match (state, byte) {
            // Already received a complete frame
            (ReceiveFrameState::Received, _) => {
                return Err(ReceiveError::AlreadyComplete);
            },

            // Seen preamble [] so far.
            (ReceiveFrameState::WaitingForSyn, super::SYN) => accept_byte(ReceiveFrameState::WaitingForSynSyn),
            (ReceiveFrameState::WaitingForSyn, _)         => (),
            // Seen preamble [SYN] so far.
            (ReceiveFrameState::WaitingForSynSyn, super::SYN) => accept_byte(ReceiveFrameState::WaitingForSynSynStx),
            (ReceiveFrameState::WaitingForSynSyn, _)         => self.reset(),
            // Seen preamble [SYN, SYN] so far.
            (ReceiveFrameState::WaitingForSynSynStx, super::STX) => accept_byte(ReceiveFrameState::Receiving),
            (ReceiveFrameState::WaitingForSynSynStx, super::SYN) => (), // We've still seen 2 consecutive SYN bytes
            (ReceiveFrameState::WaitingForSynSynStx, _)         => self.reset(),

            // Handle special bytes, unless the previous byte was an escape.
            (ReceiveFrameState::Receiving, super::ETX)  => {
                // We've got a complete frame
                accept_byte(ReceiveFrameState::Received);
                trace!("Completed frame {:?}", self.as_slice());
                if self.len < 6 {
                    self.reset();
                    return Err(ReceiveError::TooShort);
                }
                return Ok(true);
            },
            (ReceiveFrameState::Receiving, super::DLE)  => accept_byte(ReceiveFrameState::ReceivingEscaped),
            // Handle a "bog standard" byte by resetting the escape sequence and checking we've not received too much.
            (ReceiveFrameState::Receiving, _)        => {
                if self.packet_len >= RawPacket::MAX_LEN {
                    self.reset();
                    return Err(ReceiveError::TooLong);
                }
                self.packet_len += 1;
                accept_byte(ReceiveFrameState::Receiving);
            },
            (ReceiveFrameState::ReceivingEscaped, _) => accept_byte(ReceiveFrameState::Receiving)
        }
        Ok(false)
    }

    /// Reset the Raw, ready to try receiving a new frame from a CMRInet Network.
    pub fn reset(&mut self) {
        self.len = 0;
        self.receive_state = ReceiveFrameState::WaitingForSyn;
    }

    /// Decode this Raw into a `Packet`.
    ///
    /// # Errors
    ///
    ///   * [`DecodeError::TooShort`] if the frame is so short it couldn't possibly contain a packet.
    ///   * [`DecodeError::MissingSynchronisation`] if the frame doesn't start with 2 SYN bytes.
    ///   * [`DecodeError::MissingStart`] if the YN bytes aren't followed by a STX byte.
    ///   * [`DecodeError::MissingEnd`] if the frame doesn't end with an ETX byte.
    ///   * [`DecodeError::InvalidPacket`] if the packet inside the frame is invalid.
    ///
    pub fn try_as_packet(&self) -> Result<Packet, DecodeError> {
        // SYN SYN STX <escaped packet data> ETX
        trace!("Raw.as_packet({self:?})");
        if self.len < 4 {
            return Err(DecodeError::TooShort);
        }
        if self.raw[0..2] != [SYN, SYN] {
            return Err(DecodeError::MissingSynchronisation);
        }
        if self.raw[2] != STX {
            return Err(DecodeError::MissingStart);
        }
        if self.raw[self.len - 1] != ETX {
            return Err(DecodeError::MissingEnd);
        }

        if self.len < 6 { return Err(PacketError::TooShort)? }
        let mut raw_packet = RawPacket::new();
        let mut escape = false;
        for &byte in &self.raw[3..(self.len-1)] {
            if byte == DLE && !escape {
                escape = true;
            } else {
                escape = false;
                if raw_packet.push(byte).is_err() { return Err(PacketError::TooLong)? };
            }
        }
        Ok(raw_packet.try_decode()?)
    }
}

impl core::default::Default for Raw {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Raw {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Raw")
         .field("receive_state", &self.receive_state)
         .field("len", &self.len)
         .field("packet_len", &self.packet_len)
         .field("raw", &self.as_slice())
         .finish()
    }
}

impl core::fmt::LowerHex for Raw {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_char('[')?;
        for (i, &v) in self.as_slice().iter().enumerate() {
            if i > 0 { f.write_str(", ")?; }
            f.write_fmt(format_args!("{v:#04x}"))?;
        }
        f.write_char(']')
    }
}

impl core::fmt::UpperHex for Raw {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_char('[')?;
        for (i, &v) in self.as_slice().iter().enumerate() {
            if i > 0 { f.write_str(", ")?; }
            f.write_fmt(format_args!("{v:#04X}"))?;
        }
        f.write_char(']')
    }
}

impl TryFrom<&[u8]> for Raw {
    type Error = DecodeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let len = value.len();
        if len < 4 { return Err(DecodeError::TooShort) }
        if len > Self::MAX_LEN { return Err(DecodeError::TooLong) }

        let mut raw_frame = Self::new();
        raw_frame.raw[0..len].clone_from_slice(value);
        raw_frame.len = len;
        Ok(raw_frame)
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for Raw {
    type Error = DecodeError;

    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

impl<const N: usize> TryFrom<[u8; N]> for Raw {
    type Error = DecodeError;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

impl TryFrom<&RawPacket> for Raw {
    type Error = PacketError;

    fn try_from(value: &RawPacket) -> Result<Self, Self::Error> {
        value.try_as_raw_frame()
    }
}

impl TryFrom<RawPacket> for Raw {
    type Error = PacketError;

    fn try_from(value: RawPacket) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value)
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use crate::{NodeSort, node_configuration::{CpnodeConfiguration, CpnodeOptions}};
    use crate::packet::{Data as PacketData, Payload};
    use super::*;

    mod try_as_packet {
        use super::*;

        #[test]
        fn initialization() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, DLE, 0x02, DLE, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::Initialization {
                    node_sort: NodeSort::Cpnode {
                        configuration: CpnodeConfiguration::try_new(
                            0,
                            CpnodeOptions::default(),
                            2,
                            2
                        ).unwrap()
                    }
                }
            );
        }

        #[test]
        fn poll_request() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'P', ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(packet.payload(), &Payload::PollRequest);
        }

        #[test]
        fn receive_data() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'R', 0x00, 0x00, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::ReceiveData {
                    data: PacketData::try_from(&[0, 0]).unwrap()
                }
            );
        }

        #[test]
        fn transmit_data() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'T', 0x00, 0x00, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::TransmitData {
                    data: PacketData::try_from(&[0, 0]).unwrap()
                }
            );
        }

        #[test]
        fn zero_length() {
            let raw_frame = Raw::new();
            let result = raw_frame.try_as_packet();
            assert_eq!(result, Err(DecodeError::TooShort));
        }

        #[test]
        fn missing_synchronisation() {
            let raw_frame = Raw::try_from(&[STX, 65, b'P', ETX]).unwrap();
            let result = raw_frame.try_as_packet();
            assert_eq!(result, Err(DecodeError::MissingSynchronisation));
        }

        #[test]
        fn missing_start() {
            let raw_frame = Raw::try_from(&[SYN, SYN, 65, b'P', ETX]).unwrap();
            let result = raw_frame.try_as_packet();
            assert_eq!(result, Err(DecodeError::MissingStart));
        }

        #[test]
        fn missing_end() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'P']).unwrap();
            let result = raw_frame.try_as_packet();
            assert_eq!(result, Err(DecodeError::MissingEnd));
        }

        #[test]
        fn empty() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, ETX]).unwrap();
            let result = raw_frame.try_as_packet();
            assert_eq!(result, Err(DecodeError::InvalidPacket { source: PacketError::TooShort }));
        }

        #[test]
        fn unescapes_stx() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'T', DLE, 0x02, 0x00, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::TransmitData {
                    data: PacketData::try_from(&[2, 0]).unwrap()
                }
            );
        }

        #[test]
        fn unescapes_etx() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'T', DLE, 0x03, 0x00, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::TransmitData {
                    data: PacketData::try_from(&[3, 0]).unwrap()
                }
            );
        }

        #[test]
        fn unescapes_dle() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'T', DLE, 0x10, 0x00, ETX]).unwrap();
            let packet = raw_frame.try_as_packet().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::TransmitData {
                    data: PacketData::try_from(&[16, 0]).unwrap()
                }
            );
        }

        #[cfg(toolchain = "nightly")]
        mod benchmarks {
            use super::*;
            use test::Bencher;

            #[bench]
            // Smallest packet - 22 ns/iter (+/- 17)
            fn poll_request(bencher: &mut test::Bencher) {
                let raw = [SYN, SYN, STX, 65, b'P', ETX];
                let raw_frame: Raw = raw.try_into().unwrap();
                bencher.iter(|| {
                    raw_frame.try_as_packet().unwrap();
                });
            }

            #[bench]
            // Largest packet - 516 ns/iter (+/- 497)
            fn transmit_data(bencher: &mut Bencher) {
                let raw = [
                    SYN, SYN, STX, 65, b'T',
                    0x00, 0x01, DLE, 0x02, DLE, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, DLE, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E, 0x8F, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE, 0xDF, 0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF,
                    ETX
                ];
                let raw_frame: Raw = raw.try_into().unwrap();
                bencher.iter(|| {
                    raw_frame.try_as_packet().unwrap();
                });
            }
        }
    }

    mod receive {
        use super::*;

        mod waits_for_first_syn {
            use super::*;

            const fn starting() -> Raw {
                Raw::new()
            }

            #[test]
            fn got_syn() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(SYN), Ok(false));
                assert_eq!(raw_frame.len, 1);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSynSyn);
            }

            #[test]
            fn got_something_else() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(5), Ok(false));
                assert_eq!(raw_frame.len, 0);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSyn);
            }
        }

        mod waits_for_second_syn {
            use super::*;

            fn starting() -> Raw {
                let mut raw_frame = Raw::new();
                raw_frame.receive(SYN).unwrap();
                raw_frame
            }

            #[test]
            fn got_syn() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(SYN), Ok(false));
                assert_eq!(raw_frame.len, 2);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSynSynStx);
            }

            #[test]
            fn got_something_else() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(5), Ok(false));
                assert_eq!(raw_frame.len, 0);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSyn);
            }
        }

        mod waits_for_stx {
            use super::*;

            fn starting() -> Raw {
                let mut raw_frame = Raw::new();
                raw_frame.receive(SYN).unwrap();
                raw_frame.receive(SYN).unwrap();
                raw_frame
            }

            #[test]
            fn got_stx() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(STX), Ok(false));
                assert_eq!(raw_frame.len, 3);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::Receiving);
            }

            #[test]
            fn got_syn() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(SYN), Ok(false));
                assert_eq!(raw_frame.len, 2);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSynSynStx);
            }

            #[test]
            fn got_something_else() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(5), Ok(false));
                assert_eq!(raw_frame.len, 0);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSyn);
            }
        }

        mod receiving {
            use super::*;

            fn starting() -> Raw {
                let mut raw_frame = Raw::new();
                raw_frame.receive(SYN).unwrap();
                raw_frame.receive(SYN).unwrap();
                raw_frame.receive(STX).unwrap();
                raw_frame.receive(65).unwrap();
                raw_frame.receive(b'T').unwrap();
                raw_frame
            }

            #[test]
            fn unescaped() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(100), Ok(false));
                assert_eq!(raw_frame.len, 6);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::Receiving);
            }

            #[test]
            fn escaped_etx() {
                let mut raw_frame = starting();

                assert_eq!(raw_frame.receive(DLE), Ok(false));
                assert_eq!(raw_frame.len, 6);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::ReceivingEscaped);

                assert_eq!(raw_frame.receive(ETX), Ok(false));
                assert_eq!(raw_frame.len, 7);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::Receiving);
            }

            #[test]
            fn unescaped_etx() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(ETX), Ok(true));
                assert_eq!(raw_frame.len, 6);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::Received);
            }

            #[test]
            fn too_short() {
                let mut raw_frame = Raw::new();
                raw_frame.receive(SYN).unwrap();
                raw_frame.receive(SYN).unwrap();
                raw_frame.receive(STX).unwrap();
                raw_frame.receive(65).unwrap();
                assert_eq!(raw_frame.receive(ETX), Err(ReceiveError::TooShort));

                // Resets ready to try again
                assert_eq!(raw_frame.len, 0);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSyn);
            }

            #[test]
            fn too_long() {
                let mut raw_frame = starting();
                for _ in 0..256 { raw_frame.receive(0).unwrap(); }

                // Fails fast
                assert_eq!(raw_frame.receive(0), Err(ReceiveError::TooLong));

                // Resets ready to try again
                assert_eq!(raw_frame.len, 0);
                assert_eq!(raw_frame.receive_state, ReceiveFrameState::WaitingForSyn);
            }

            #[test]
            fn already_complete() {
                let mut raw_frame = starting();
                assert_eq!(raw_frame.receive(ETX), Ok(true));
                assert_eq!(raw_frame.receive(SYN), Err(ReceiveError::AlreadyComplete));
            }
        }
    }

    mod build {
        use super::*;

        mod begin {
            use super::*;

            #[test]
            fn from_empty() {
                let mut raw_frame = Raw::new();
                raw_frame.begin(Address::try_from_node_address(0).unwrap(), b'T');
                assert_eq!(raw_frame.as_slice(), &[SYN, SYN, STX, 65, b'T']);
            }

            #[test]
            fn from_non_empty() {
                let mut raw_frame = Raw::try_from(&[1, 2, 4, 8, 16, 32, 64, 128]).unwrap();
                raw_frame.begin(Address::try_from_node_address(1).unwrap(), b'T');
                assert_eq!(raw_frame.as_slice(), &[SYN, SYN, STX, 66, b'T']);
            }
        }

        mod push {
            use super::*;

            #[test]
            fn has_space() {
                let mut raw_frame = Raw::new();
                assert_eq!(raw_frame.push(101), Ok(1));
                raw_frame.push(102).unwrap();
                raw_frame.push(103).unwrap();
                assert_eq!(raw_frame.as_slice(), &[101, 102, 103]);
            }

            #[test]
            fn full() {
                let mut raw_frame = Raw::try_from(&[0; Raw::MAX_LEN - 1]).unwrap();
                assert_eq!(raw_frame.push(1), Ok(1));
                assert_eq!(raw_frame.push(1), Err(Full));
                assert_eq!(raw_frame[Raw::MAX_LEN - 1], 1);
            }
        }

        mod finish {
            use super::*;

            #[test]
            fn has_space() {
                let mut raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'P']).unwrap();
                assert_eq!(raw_frame.finish(), Ok(()));
                assert_eq!(raw_frame.as_slice(), &[SYN, SYN, STX, 65, b'P', ETX]);
            }

            #[test]
            fn full() {
                let mut raw_frame = Raw::try_from(&[0; Raw::MAX_LEN]).unwrap();
                assert_eq!(raw_frame.finish(), Err(Full));
                assert_eq!(raw_frame[Raw::MAX_LEN - 1], 0);
            }
        }
    }

    mod address {
        use super::*;

        #[test]
        fn valid() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 67, b'P', ETX]).unwrap();
            assert_eq!(raw_frame.address(), Some(2));
        }

        #[test]
        fn invalid() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 200, b'P', ETX]).unwrap();
            assert_eq!(raw_frame.address(), None);
        }
    }

    mod message_type {
        use super::*;

        #[test]
        fn valid() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'P', ETX]).unwrap();
            assert_eq!(raw_frame.message_type(), Some('P'));
        }

        #[test]
        fn invalid() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'Z'+1, ETX]).unwrap();
            assert_eq!(raw_frame.message_type(), None);
        }

        #[cfg(not(feature = "experimenter"))]
        #[test]
        fn unknown_message_type() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'Z', ETX]).unwrap();
            assert_eq!(
                raw_frame.message_type(),
                None
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown_message_type() {
            let raw_frame = Raw::try_from(&[SYN, SYN, STX, 65, b'Z', ETX]).unwrap();
            assert_eq!(
                raw_frame.message_type(),
                Some('Z')
            );
        }

    }

    #[test]
    fn reset() {
        let bytes = [SYN, SYN, STX, 70];
        let mut frame = Raw::new();
        for byte in bytes { frame.receive(byte).unwrap(); }
        assert_eq!(frame.len, 4);
        assert_ne!(frame.receive_state, ReceiveFrameState::WaitingForSyn);

        frame.reset();
        assert_eq!(frame.len, 0);
        assert_eq!(frame.receive_state, ReceiveFrameState::WaitingForSyn);
    }


    mod try_from_slice_u8 {
        use super::*;

        #[test]
        fn works() {
            let bytes = [SYN, SYN, SYN, STX, 65, b'P', ETX];
            assert_eq!(Raw::try_from(&bytes).unwrap().as_slice(), &bytes);
            assert_eq!(Raw::try_from(&bytes[0..7]).unwrap().as_slice(), &bytes);
            assert_eq!(Raw::try_from(bytes).unwrap().as_slice(), &bytes);
        }

        #[test]
        fn too_short() {
            let slice = &[SYN, SYN, 0x00];
            let result: Result<Raw, DecodeError> = slice.try_into();
            assert_eq!(result, Err(DecodeError::TooShort));
        }

        #[test]
        fn too_long() {
            let slice = &[0xFF; Raw::MAX_LEN + 1];
            let result: Result<Raw, DecodeError> = slice.try_into();
            assert_eq!(result, Err(DecodeError::TooLong));
        }
    }

    #[test]
    fn try_from_raw_packet() {
        let raw_packet = RawPacket::try_from(&[65, b'T', 100, 200]).unwrap();
        assert_eq!(*Raw::try_from(&raw_packet).unwrap(), [SYN, SYN, STX, 65, b'T', 100, 200, ETX]);
        assert_eq!(*Raw::try_from(raw_packet).unwrap(), [SYN, SYN, STX, 65, b'T', 100, 200, ETX]);
    }

    #[test]
    fn default() {
        let raw_frame = Raw::default();
        assert_eq!(raw_frame.len, 0);
        assert_eq!(raw_frame.packet_len, 0);
        assert_eq!(raw_frame.receive_state, super::ReceiveFrameState::WaitingForSyn);
    }

    #[test]
    #[cfg(feature = "std")]
    fn lower_hex() {
        assert_eq!(
            format!("{:x}", &Raw::try_from(&[SYN, SYN, STX, 65, b'P', ETX]).unwrap()),
            "[0xff, 0xff, 0x02, 0x41, 0x50, 0x03]"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn upper_hex() {
        assert_eq!(
            format!("{:X}", &Raw::try_from(&[SYN, SYN, STX, 65, b'P', ETX]).unwrap()),
            "[0xFF, 0xFF, 0x02, 0x41, 0x50, 0x03]"
        );
    }
}
