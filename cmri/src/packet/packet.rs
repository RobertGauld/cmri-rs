use log::trace;
use crate::{Address, NodeSort, frame::Raw as RawFrame};
use super::{Payload, Data, Raw};
#[cfg(feature = "experimenter")]
use super::Error;

/// A CMRInet packet.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Packet {
    address: Address,
    payload: Payload
}

impl Packet {
    /// Create a new initialization packet.
    #[must_use]
    pub const fn new_initialization(address: Address, node_sort: NodeSort) -> Self {
        let payload = Payload::Initialization { node_sort };
        Self { address, payload }
    }

    /// Create a new poll request packet.
    #[must_use]
    pub const fn new_poll_request(address: Address) -> Self {
        let payload = Payload::PollRequest;
        Self { address, payload }
    }

    /// Create a new receive data (node inputs → controller) packet.
    #[must_use]
    pub const fn new_receive_data(address: Address, data: Data) -> Self {
        let payload = Payload::ReceiveData { data };
        Self { address, payload }
    }

    /// Create a new transmit data (controller → node outputs) packet.
    #[must_use]
    pub const fn new_transmit_data(address: Address, data: Data) -> Self {
        let payload = Payload::TransmitData { data };
        Self { address, payload }
    }

    #[cfg(feature = "experimenter")]
    #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "experimenter")))]
    #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature experimenter only.**\n\n")]
    /// Create a new custom packet.
    ///
    /// Message type must be between 63 (b'A') and 90 (b'Z'),
    /// it should not conflict with a type defined in the CMRInet specification,
    /// (C, M, N, O, X).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidMessageType`] if the message type is invalid (not uppercase ASCII).
    pub const fn try_new_unknown(address: Address, message_type: u8, body: Data) -> Result<Self, Error> {
        if !message_type.is_ascii_uppercase() { return Err(super::Error::InvalidMessageType(message_type)) }
        let payload = Payload::Unknown { message_type, body };
        Ok(Self { address, payload })
    }

    /// Encode into a `RawPacket` (without escaping or framing).
    ///
    /// # Example
    ///
    /// ```
    /// use cmri::{Address, packet::{Packet, Payload}};
    /// let address = Address::try_from_node_address(0).unwrap();
    /// let packet = Packet::new_poll_request(address);
    /// let raw = packet.encode_packet();
    /// assert_eq!(raw, [65, b'P']);
    /// ```
    #[must_use]
    pub fn encode_packet(&self) -> Raw {
        trace!("Packet.encode_packet({self:?})");
        let mut raw = Raw::new();
        let _ = raw.push(self.address.as_unit_address()); // First byte will always fit.
        let _ = raw.push_all(&self.payload.encode()); // Payload will always fit.
        raw
    }

    /// Encode into a `RawFrame` (ready for writing to a CMRInet network).
    ///
    /// # Example
    ///
    /// ```
    /// use cmri::{Address, packet::{Packet, Payload}};
    /// let address = Address::try_from_node_address(0).unwrap();
    /// let packet = Packet::new_poll_request(address);
    /// let raw = packet.encode_frame();
    /// assert_eq!(raw, [0xFF, 0xFF, 0x02, 65, b'P', 0x03]);
    /// ```
    #[expect(clippy::missing_panics_doc)]
    #[must_use]
    pub fn encode_frame(&self) -> RawFrame {
        trace!("Packet.encode_frame({self:?})");
        self.encode_packet().try_into().expect("An always valid Packet, will always produce a valid RawPacket, which will always produce a valid RawFrame.")
    }

    /// The address this packet is being sent to / has been received from.
    #[must_use]
    pub const fn address(&self) -> Address {
        self.address
    }

    /// The packet's payload.
    #[must_use]
    pub const fn payload(&self) -> &Payload {
        &self.payload
    }
}


impl TryFrom<Raw> for Packet {
    type Error = super::Error;
    fn try_from(raw: Raw) -> Result<Self, Self::Error> {
        // <UA> <MT> <Payload 0>..<Payload n>
        if raw.len() < 2 { return Err(super::Error::TooShort) }
        let address = raw.address().ok_or(Self::Error::InvalidUnitAddress(raw[0]))?;
        let payload = Payload::try_decode(&raw[1..])?;
        Ok(Self { address, payload })
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use crate::node_configuration::{CpnodeConfiguration, CpnodeOptions};
    use super::super::Error;
    use super::*;

    #[test]
    fn getters() {
        let address = Address::try_from_node_address(10).unwrap();
        let payload = Payload::PollRequest;
        let packet = Packet { address, payload };

        assert_eq!(packet.address(), address);
        assert_eq!(packet.payload(), &payload);
    }

    mod constructors {
        use super::*;

        #[test]
        fn initialization() {
            let configuration = CpnodeConfiguration::try_new(
                0,
                CpnodeOptions::default(),
                1,
                1
            ).unwrap();

            assert_eq!(
                Packet::new_initialization(Address::try_from_node_address(25).unwrap(), NodeSort::Cpnode { configuration }),
                Packet {
                    address: Address::try_from_node_address(25).unwrap(),
                    payload: Payload::Initialization {
                        node_sort: NodeSort::Cpnode { configuration }
                    }
                }
            );
        }

        #[test]
        fn poll_request() {
            assert_eq!(
                Packet::new_poll_request(Address::try_from_node_address(1).unwrap()),
                Packet { address: Address::try_from_node_address(1).unwrap(), payload: Payload::PollRequest }
            );
        }

        #[test]
        fn receive_data() {
            let packet = Packet::new_receive_data(
                Address::try_from_node_address(1).unwrap(),
                [1, 2, 3, 4].try_into().unwrap()
            );
            assert_eq!(
                packet,
                Packet {
                    address: Address::try_from_node_address(1).unwrap(),
                    payload: Payload::ReceiveData {
                        data: Data::try_from(&[1, 2, 3, 4]).unwrap()
                    }
                }
            );
        }

        #[test]
        fn transmit_data() {
            let packet = Packet::new_transmit_data(
                Address::try_from_node_address(1).unwrap(),
                [1, 2, 3, 4].try_into().unwrap()
            );
            assert_eq!(
                packet,
                Packet {
                    address: Address::try_from_node_address(1).unwrap(),
                    payload: Payload::TransmitData {
                        data: Data::try_from(&[1, 2, 3, 4]).unwrap()
                    }
                }
            );
        }

        #[cfg(feature = "experimenter")]
        mod unknown {
            use super::*;

            #[test]
            fn created() {
                let packet = Packet::try_new_unknown(
                    Address::try_from_node_address(1).unwrap(),
                    b'A',
                    [1, 2, 3, 4].try_into().unwrap()
                ).unwrap();
                assert_eq!(
                    packet,
                    Packet {
                        address: Address::try_from_node_address(1).unwrap(),
                        payload: Payload::Unknown {
                            message_type: b'A',
                            body: Data::try_from(&[1, 2, 3, 4]).unwrap()
                        }
                    }
                );
            }

            #[test]
            fn invalid_message_type() {
                assert_eq!(
                    Packet::try_new_unknown(
                        Address::try_from_node_address(0).unwrap(),
                        b'a',
                        [].try_into().unwrap()
                    ),
                    Err(crate::packet::Error::InvalidMessageType(b'a'))
                );
            }
        }
    }

    #[test]
    fn encode_packet() {
        assert_eq!(
            Packet { address: Address::try_from_node_address(64).unwrap(), payload: Payload::PollRequest }.encode_packet(),
            [129, b'P']
        );
    }

    mod encode_frame {
        use super::*;

        #[test]
        fn adds_header() {
            let raw = Packet::new_poll_request(Address::try_from_node_address(1).unwrap()).encode_frame();
            assert_eq!(
                raw[0..3],
                [0xFF, 0xFF, 0x02]
            );
        }

        #[test]
        fn adds_trailer() {
            let raw = Packet::new_poll_request(Address::try_from_node_address(0).unwrap()).encode_frame();
            assert_eq!(
                raw[raw.len() - 1],
                0x03
            );
        }

        #[test]
        fn escapes_stx() {
            let raw = Packet::new_receive_data(
                Address::try_from_node_address(0).unwrap(),
                [0x02].try_into().unwrap()
            ).encode_frame();
            assert_eq!(
                raw[5..=6],
                [0x10, 0x02]
            );
        }

        #[test]
        fn escapes_etx() {
            let raw = Packet::new_receive_data(
                Address::try_from_node_address(0).unwrap(),
                [0x03].try_into().unwrap()
            ).encode_frame();
            assert_eq!(
                raw[5..=6],
                [0x10, 0x03]
            );
        }

        #[test]
        fn escapes_dle() {
            let raw = Packet::new_receive_data(
                Address::try_from_node_address(0).unwrap(),
                [0x10].try_into().unwrap()
            ).encode_frame();
            assert_eq!(
                &raw[5..=6],
                [0x10, 0x10]
            );
        }
    }

    mod try_from_raw_packet {
        use super::*;

        #[test]
        fn success() {
            let packet = Packet::try_from(Raw::try_from(&[66, b'P']).unwrap()).unwrap();
            assert_eq!(packet.address().as_node_address(), 1);
            assert_eq!(packet.payload(), &Payload::PollRequest);
        }

        #[test]
        fn bad_address() {
            assert_eq!(
                Packet::try_from(Raw::try_from(&[0, b'P']).unwrap()),
                Err(Error::InvalidUnitAddress(0))
            );
        }

        #[test]
        fn bad_payload() {
            assert_eq!(
                Packet::try_from(Raw::try_from(&[66, 0]).unwrap()),
                Err(Error::InvalidMessageType(0))
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde() {
        use serde_test::{assert_tokens, Token};

        assert_tokens(
            &Packet::new_poll_request(Address::try_from_node_address(23).unwrap()),
            &[
                Token::Struct { name: "Packet", len: 2 },
                    Token::String("address"),
                    Token::U8(23),
                    Token::String("payload"),
                    Token::UnitVariant { name: "Payload", variant: "PollRequest" },
                Token::StructEnd
            ]
        );
    }
}
