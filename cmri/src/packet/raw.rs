use crate::Address;
use super::{Packet, Error};

/// The binary representation of a packet,
/// without the escaping and framing required for actually sending it.
#[derive(Clone, Copy, Eq)]
pub struct Raw {
    raw: [u8; Self::MAX_LEN],
    len: usize
}

crate::raw_structs::common_implementation!(Raw, 258);

impl Raw {
    /// Create a new, empty Raw.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            raw: [0; Self::MAX_LEN],
            len: 0
        }
    }

    /// Get the address the packet is sent to/from (if it's valid).
    #[must_use]
    pub fn address(&self) -> Option<Address> {
        Address::try_from_unit_address(self.raw[0]).ok()
    }

    /// Get the message type of the packet (if it's valid).
    #[must_use]
    pub fn message_type(&self) -> Option<char> {
        check_message_type(self.raw[1]).ok().map(core::convert::Into::into)
    }

    /// Get the body of the packet.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.raw[2..(self.len())]
    }

    /// Add a new byte to the end.
    ///
    /// Returns the value added.
    ///
    /// # Errors
    ///
    /// `Error::Full` if the Raw is full.
    pub fn push(&mut self, value: u8) -> Result<u8, u8> {
        if self.available() == 0 { return Err(value) }
        self.raw[self.len] = value;
        self.len += 1;
        Ok(value)
    }

    /// Add new bytes at the end.
    ///
    /// # Errors
    ///
    /// Returns the available space if there's insufficent space remaining.
    pub fn push_all(&mut self, items: &[u8]) -> Result<(), usize> {
        if self.available() >= items.len() {
            self.raw[self.len..(self.len + items.len())].clone_from_slice(items);
            self.len += items.len();
            Ok(())
        } else {
            Err(self.available())
        }
    }

    /// Decode this Raw into a `Packet`.
    ///
    /// # Errors
    ///
    ///   * [`Error::InvalidUnitAddress`] if the UA byte is invalid.
    ///   * [`Error::InvalidMessageType`] if the MT byte is invalid.
    ///   * For `ReceiveData` and `TransmitData` packets:
    ///     * [`Error::BodyTooLong`] if the raw data is too long to be a packet.
    ///   * For `Initialization` packets
    ///     * [`Error::InvalidNodeType`] if the NDP byte isn't valid.
    ///     * [`Error::TooShort`] if the slice isn't long enough.
    ///     * [`Error::InvalidConfiguration`]:
    ///       * For USIC/SUSIC nodes:
    ///         * [`crate::node_configuration::NodeCardsError::InvalidCardType`] if there's an invalid card type in the card types sequence.
    ///         * [`crate::node_configuration::NodeCardsError::CardAfterNone`] if there's an Input or Output card after the first None card.
    ///         * [`crate::node_configuration::NodeCardsError::TooManyCards`] if there's more than 64 input/output cards.
    ///       * For SMINI node:
    ///         * [`crate::node_configuration::SminiConfigurationError::NonAdjacent`] if `oscillating_pairs` has an odd number of true bits.
    ///         * [`crate::node_configuration::SminiConfigurationError::NonAdjacent`] if `oscillating_pairs` has a pair of true bits which aren't adjacent.
    ///       * For CPNODE/CPMEGA nodes:
    ///         * [`crate::node_configuration::CpConfigurationError::InvalidInputOutputBitsCount`] if the total number of input and output bits is invalid for a `", stringify!($name), "` (", stringify!($bpc), ").")]
    pub fn try_decode(self) -> Result<Packet, Error> {
        Packet::try_from(self)
    }

    /// Apply escaping and framing to create a `Raw`, ready for sending onto a CMRInet network.
    #[expect(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub fn try_as_raw_frame(&self) -> Result<crate::frame::Raw, Error> {
        let mut raw_frame = crate::frame::Raw::new();
        raw_frame.begin(
            Address::try_from_unit_address(self.raw[0])?,
            check_message_type(self.raw[1])?
        );

        for byte in self.body() {
            raw_frame.push(*byte).expect("RawFrame can always accomodate a maximally escaped maximum length Raw.");
        }

        raw_frame.finish().expect("RawFrame can always accomodate a maximally escaped maximum length Raw.");
        Ok(raw_frame)
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
         .field("len", &self.len)
         .field("raw", &self.as_slice())
         .finish()
    }
}

impl TryFrom<&[u8]> for Raw {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let len = value.len();
        if len < 2 { return Err(Error::TooShort) }
        if len > Self::MAX_LEN { return Err(Error::TooLong) }

        let mut raw = [0; Self::MAX_LEN];
        raw[0..len].clone_from_slice(value);

        Ok(Self { raw, len })
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for Raw {
    type Error = Error;

    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

impl<const N: usize> TryFrom<[u8; N]> for Raw {
    type Error = Error;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

#[cfg(not(feature = "experimenter"))]
#[expect(clippy::missing_errors_doc)]
fn check_message_type(byte: u8) -> Result<u8, Error> {
    if [b'I', b'P', b'R', b'T'].contains(&byte) {
        Ok(byte)
    } else {
        Err(Error::InvalidMessageType(byte))
    }
}
#[cfg(feature = "experimenter")]
#[expect(clippy::missing_errors_doc)]
const fn check_message_type(byte: u8) -> Result<u8, Error> {
    if byte.is_ascii_uppercase() {
        Ok(byte)
    } else {
        Err(Error::InvalidMessageType(byte))
    }
}


#[allow(clippy::missing_panics_doc)]
#[cfg(test)]
mod tests {
    use crate::{NodeSort, node_configuration::{CpnodeConfiguration, CpnodeOptions}};
    use super::super::{Data, Payload};
    use super::*;

    mod try_decode {
        use super::*;

        #[test]
        fn address() {
            assert_eq!(
                Raw::try_from(&[65, b'P']).unwrap().try_decode().unwrap().address(),
                Address::try_from_node_address(0).unwrap()
            );
            assert_eq!(
                Raw::try_from(&[66, b'P']).unwrap().try_decode().unwrap().address(),
                Address::try_from_node_address(1).unwrap()
            );
            assert_eq!(
                Raw::try_from(&[129, b'P']).unwrap().try_decode().unwrap().address(),
                Address::try_from_node_address(64).unwrap()
            );
            assert_eq!(
                Raw::try_from(&[191, b'P']).unwrap().try_decode().unwrap().address(),
                Address::try_from_node_address(126).unwrap()
            );
            assert_eq!(
                Raw::try_from(&[192, b'P']).unwrap().try_decode().unwrap().address(),
                Address::try_from_node_address(127).unwrap()
            );
            assert_eq!(
                Raw::try_from(&[193, b'P']).unwrap().try_decode(),
                Err(Error::InvalidUnitAddress(193))
            );
        }

        #[test]
        fn initialization() {
            let raw_packet = Raw::try_from(&[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 0x02, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
            let packet = raw_packet.try_decode().unwrap();
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

        mod initialization {
            use super::*;

            #[test]
            fn transmit_delay() {
                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 0x02, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().transmit_delay(),
                    0
                );

                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x01, 0xF4, 0x00, 0x00, 0x02, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().transmit_delay(),
                    500
                );
            }

            #[test]
            fn input_bytes() {
                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 9, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().input_bytes(),
                    9
                );

                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 18, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().input_bytes(),
                    18
                );
            }

            #[test]
            fn output_bytes() {
                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 0x00, 9, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().output_bytes(),
                    9
                );

                assert_eq!(
                    node_from_packet(
                        &[65, b'I', b'C', 0x00, 0x00, 0x00, 0x00, 0x00, 18, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
                    ).configuration().output_bytes(),
                    18
                );
            }

            fn node_from_packet(raw: &[u8]) -> NodeSort {
                match Raw::try_from(raw).unwrap().try_decode().unwrap().payload() {
                    Payload::Initialization { node_sort } => *node_sort,
                    _ => panic!("Can't get a configuration from that packet.")
                }
            }
        }

        #[test]
        fn poll_request() {
            let raw_packet = Raw::try_from(&[65, b'P']).unwrap();
            let packet = raw_packet.try_decode().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(packet.payload(), &Payload::PollRequest);
        }

        #[test]
        fn receive_data() {
            let raw_packet = Raw::try_from(&[65, b'R', 0x01, 0x02]).unwrap();
            let packet = raw_packet.try_decode().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::ReceiveData {
                    data: Data::try_from(&[1, 2]).unwrap()
                }
            );
        }

        #[test]
        fn transmit_data() {
            let raw_packet = Raw::try_from(&[65, b'T', 0x01, 0x02]).unwrap();
            let packet = raw_packet.try_decode().unwrap();
            assert_eq!(packet.address().as_node_address(), 0);
            assert_eq!(
                packet.payload(),
                &Payload::TransmitData {
                    data: Data::try_from(&[1, 2]).unwrap()
                }
            );
        }

        #[test]
        fn zero_length() {
            let raw_packet = Raw::new();
            let result = raw_packet.try_decode();
            assert_eq!(result, Err(Error::TooShort));
        }

        #[test]
        fn mising_address() {
            let raw_packet = Raw::new();
            let result = raw_packet.try_decode();
            assert_eq!(result, Err(Error::TooShort));
        }

        #[test]
        fn mising_message_type() {
            let mut raw_packet = Raw::new();
            raw_packet.push(65).unwrap();
            let result = raw_packet.try_decode();
            assert_eq!(result, Err(Error::TooShort));
        }

        #[cfg(toolchain = "nightly")]
        mod benchmarks {
            use super::*;
            use test::Bencher;

            #[bench]
            // Smallest packet - 5 ns/iter (+/- 6)
            fn poll_request(bencher: &mut test::Bencher) {
                let raw = [65, b'P'];
                let raw_packet: Raw = raw.try_into().unwrap();
                bencher.iter(|| {
                    raw_packet.try_decode().unwrap();
                });
            }

            #[bench]
            // Largest packet - 31 ns/iter (+/- 17)
            fn transmit_data(bencher: &mut Bencher) {
                let raw = [
                    65, b'T',
                    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E, 0x8F, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xDB, 0xDC, 0xDD, 0xDE, 0xDF, 0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF
                ];
                let raw_packet: Raw = raw.try_into().unwrap();
                bencher.iter(|| {
                    raw_packet.try_decode().unwrap();
                });
            }
        }
    }

    mod try_as_raw_frame {
        use super::*;

        #[test]
        fn wraps_data() {
            let raw_packet = Raw::try_from(&[65, b'T', 0, 127]).unwrap();
            let raw_frame = raw_packet.try_as_raw_frame().unwrap();
            assert_eq!(
                *raw_frame,
                [0xFF, 0xFF, 0x02, 65, b'T', 0, 127, 0x03]
            );
        }

        #[test]
        fn escapes_stx() {
            let raw_packet = Raw::try_from(&[65, b'T', 0x02, 127]).unwrap();
            let raw_frame = raw_packet.try_as_raw_frame().unwrap();
            assert_eq!(
                *raw_frame,
                [0xFF, 0xFF, 0x02, 65, b'T', 0x10, 0x02, 127, 0x03]
            );
        }

        #[test]
        fn escapes_etx() {
            let raw_packet = Raw::try_from(&[65, b'T', 0x03, 127]).unwrap();
            let raw_frame = raw_packet.try_as_raw_frame().unwrap();
            assert_eq!(
                *raw_frame,
                [0xFF, 0xFF, 0x02, 65, b'T', 0x10, 0x03, 127, 0x03]
            );
        }

        #[test]
        fn escapes_dle() {
            let raw_packet = Raw::try_from(&[65, b'T', 0x10, 127]).unwrap();
            let raw_frame = raw_packet.try_as_raw_frame().unwrap();
            assert_eq!(
                *raw_frame,
                [0xFF, 0xFF, 0x02, 65, b'T', 0x10, 0x10, 127, 0x03]
            );
        }

        #[test]
        fn invalid_address() {
            let raw_packet = Raw::try_from(&[255, b'P']).unwrap();
            assert_eq!(
                raw_packet.try_as_raw_frame(),
                Err(Error::InvalidUnitAddress(255))
            );
        }

        #[test]
        fn invalid_message_type() {
            let raw_packet = Raw::try_from(&[65, 255]).unwrap();
            assert_eq!(
                raw_packet.try_as_raw_frame(),
                Err(Error::InvalidMessageType(255))
            );
        }
    }

    mod push {
        use super::*;

        #[test]
        fn works() {
            let mut raw_packet = Raw::new();
            assert_eq!(raw_packet.push(1), Ok(1));
            raw_packet.push(2).unwrap();
            raw_packet.push(3).unwrap();
            assert_eq!(raw_packet.as_slice(), &[1, 2, 3]);
        }

        #[test]
        fn full() {
            let mut raw_packet = Raw::try_from(&[0; Raw::MAX_LEN - 1]).unwrap();
            assert_eq!(raw_packet.push(1), Ok(1));
            assert_eq!(raw_packet.push(2), Err(2));
            assert_eq!(raw_packet[Raw::MAX_LEN - 1], 1);
        }
    }

    mod push_all {
        use super::*;

        #[test]
        fn works() {
            let mut raw_packet = Raw::new();
            assert_eq!(raw_packet.push_all(&[1, 2]), Ok(()));
            raw_packet.push_all(&[3, 4, 5]).unwrap();
            assert_eq!(raw_packet.as_slice(), &[1, 2, 3, 4, 5]);
        }

        #[test]
        fn full() {
            let mut raw_packet = Raw::try_from(&[0; Raw::MAX_LEN - 2]).unwrap();
            assert_eq!(raw_packet.push_all(&[1, 2]), Ok(()));
            assert_eq!(raw_packet.push_all(&[3, 4]), Err(0));
            assert_eq!(raw_packet[(Raw::MAX_LEN - 2)..], [1, 2]);
        }

        #[test]
        fn too_full() { // Pushing 2 bytes when there's only space for 1
            let mut raw_packet = Raw::try_from(&[0; Raw::MAX_LEN - 3]).unwrap();
            assert_eq!(raw_packet.push_all(&[1, 2]), Ok(()));
            assert_eq!(raw_packet.push_all(&[3, 4]), Err(1));
            assert_eq!(raw_packet[(raw_packet.len() - 2)..], [1, 2]);
        }
    }

    mod address {
        use super::*;

        #[test]
        fn valid() {
            let raw_packet = Raw::try_from(&[67, b'P']).unwrap();
            assert_eq!(raw_packet.address(), Some(Address::try_from_node_address(2).unwrap()));
        }

        #[test]
        fn invalid() {
            let raw_packet = Raw::try_from(&[200, b'P']).unwrap();
            assert_eq!(raw_packet.address(), None);
        }
    }

    mod message_type {
        use super::*;

        #[test]
        fn valid() {
            let raw_packet = Raw::try_from(&[65, b'P']).unwrap();
            assert_eq!(raw_packet.message_type(), Some('P'));
        }

        #[test]
        fn invalid() {
            let raw_packet = Raw::try_from(&[65, b'Z'+1]).unwrap();
            assert_eq!(raw_packet.message_type(), None);
        }

        #[cfg(not(feature = "experimenter"))]
        #[test]
        fn unknown_message_type() {
            let raw_packet = Raw::try_from(&[65, b'Z']).unwrap();
            assert_eq!(
                raw_packet.message_type(),
                None
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown_message_type() {
            let raw_packet = Raw::try_from(&[65, b'Z']).unwrap();
            assert_eq!(
                raw_packet.message_type(),
                Some('Z')
            );
        }
    }

    #[test]
    fn body() {
        let raw_packet = Raw::try_from(&[65, b'T', 5, 6, 7, 8, 9]).unwrap();
        assert_eq!(raw_packet.body(), [5, 6, 7, 8, 9]);

        let raw_packet = Raw::try_from(&[65, b'P']).unwrap();
        assert_eq!(raw_packet.body().len(), 0);
    }

    mod check_message_type {
        use super::{check_message_type, Error::InvalidMessageType};

        #[test]
        fn valid() {
            assert_eq!(
                check_message_type(b'P'),
                Ok(80)
            );
        }

        #[test]
        fn invalid() {
            assert_eq!(
                check_message_type(b'Z'+1),
                Err(InvalidMessageType(91))
            );
        }

        #[cfg(not(feature = "experimenter"))]
        #[test]
        fn unknown_message_type() {
            assert_eq!(
                check_message_type(b'Z'),
                Err(InvalidMessageType(90))
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown_message_type() {
            assert_eq!(
                check_message_type(b'Z'),
                Ok(90)
            );
        }
    }

    mod try_from_slice_u8 {
        use super::*;

        #[test]
        fn works() {
            let bytes = [65, b'P'];
            assert_eq!(Raw::try_from(&bytes).unwrap().as_slice(), &bytes);
            assert_eq!(Raw::try_from(&bytes[0..2]).unwrap().as_slice(), &bytes);
            assert_eq!(Raw::try_from(bytes).unwrap().as_slice(), &bytes);
        }

        #[test]
        fn too_short() {
            let slice = &[];
            let result: Result<Raw, Error> = slice.try_into();
            assert_eq!(result, Err(Error::TooShort));
        }

        #[test]
        fn too_long() {
            let slice = &[0; Raw::MAX_LEN + 1];
            let result: Result<Raw, Error> = slice.try_into();
            assert_eq!(result, Err(Error::TooLong));
        }
    }

    #[test]
    fn default() {
        let raw_packet = Raw::default();
        assert_eq!(raw_packet.len, 0);
    }
}
