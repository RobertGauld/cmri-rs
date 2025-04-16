use crate::NodeSort;
use super::{Error, Data};
use log::trace;

/// The payload within a CMRInet `Packet`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Payload {
    /// An initialization packet should be the first one sent to a node.
    /// It is used to configure the node's parameters.
    Initialization {
        /// The type of the node being initialized.
        node_sort: NodeSort
    },

    /// The poll request packet is sent to a node to cause a receive data packet to be sent.
    PollRequest,

    /// A receive data packet is sent by a node (in response to a poll request packet)
    /// and is used to send the node's input states to the controller.
    ReceiveData {
        /// The data which was read from the inputs.
        data: Data
    },

    /// A transmit data packet is sent to a node to cause it's output states to be changed.
    TransmitData {
        /// The data to be written to the outputs.
        data: Data
    },

    #[cfg(feature = "experimenter")]
    #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "experimenter")))]
    #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature experimenter only.**\n\n")]
    /// An unknown message type.
    Unknown {
        /// The packet's message type.
        message_type: u8,
        /// The packet's body.
        body: Data
    }
}

impl Payload{
    /// Decode from an unescaped packet payload
    ///
    /// # Errors
    ///
    ///   * [`Error::InvalidMessageType`] if the MT byte is invalid.
    ///   * For `ReceiveData` and `TransmitData` packets:
    ///     * [`Error::BodyTooLong`] if the raw data is too long to be a packet.
    ///   * For `Initialization` packets
    ///   *   Anything from [`NodeSort::try_decode`]
    pub(super) fn try_decode(raw: &[u8]) -> Result<Self, Error> {
        trace!("Payload::decode(#{raw:?})");
        match raw[0] {
            b'I' => Ok(Self::Initialization { node_sort: NodeSort::try_decode(&raw[1..(raw.len())])? }),
            b'P' => Ok(Self::PollRequest),
            b'R' => {
                Data::try_from(&raw[1..(raw.len())]).map_or(Err(Error::BodyTooLong), |data|
                    Ok(Self::ReceiveData { data })
                )
            },
            b'T' => {
                Data::try_from(&raw[1..(raw.len())]).map_or(Err(Error::BodyTooLong), |data|
                    Ok(Self::TransmitData { data })
                )
            },
            _ => {
                #[cfg(feature = "experimenter")]
                if raw[0].is_ascii_uppercase() {
                    return Ok(Self::Unknown { message_type: raw[0], body: Data::try_from(&raw[1..(raw.len())])? })
                }
                Err(Error::InvalidMessageType(raw[0]))
            }
        }
    }

    /// Encode to an unescaped packet payload
    pub(super) fn encode(&self) -> Data {
        trace!("Payload.encode({self:?})");
        let mut raw = Data::default();
        match self {
            Self::Initialization { node_sort } => {
                let _ = raw.push(b'I');
                for &item in &node_sort.encode() {
                    let _ = raw.push(item);
                }
            },
            Self::PollRequest => {
                let _ = raw.push(b'P');
            },
            Self::ReceiveData { data } => {
                let _ = raw.push(b'R');
                for &item in data {
                    let _ = raw.push(item);
                }
            },
            Self::TransmitData { data } => {
                let _ = raw.push(b'T');
                for &item in data {
                    let _ = raw.push(item);
                }
            },
            #[cfg(feature = "experimenter")]
            Self::Unknown { message_type, body } => {
                let _ = raw.push(*message_type);
                for &item in body {
                    let _ = raw.push(item);
                }
            }
        }
        raw
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use crate::node_configuration::{CpnodeConfiguration, CpnodeOptions};
    use super::*;

    mod encode {
        use super::*;

        #[test]
        fn initialization() {
            let payload = Payload::Initialization {
                node_sort: NodeSort::Cpnode {
                    configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::from_bits_retain(1), 2, 3).unwrap()
                }
            };
            assert_eq!(
                payload.encode(),
                [b'I', b'C', 0, 0, 1, 0, 2, 3, 255, 255, 255, 255, 255, 255]
            );
        }

        #[test]
        fn poll_request() {
            assert_eq!(
                Payload::PollRequest.encode(),
                [b'P']
            );
        }

        #[test]
        fn receive_data() {
            let payload = Payload::ReceiveData { data: Data::try_from(&[1,2,3]).unwrap() };
            assert_eq!(
                payload.encode(),
                [b'R', 1, 2, 3]
            );
        }

        #[test]
        fn transmit_data() {
            let payload = Payload::TransmitData { data: Data::try_from(&[1,2,3]).unwrap() };
            assert_eq!(
                payload.encode(),
                [b'T', 1, 2, 3]
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown() {
            let payload = Payload::Unknown {
                message_type: 250,
                body: Data::try_from(&[50, 100, 150, 200]).unwrap()
            };
            assert_eq!(payload.encode(), [250, 50, 100, 150, 200]);
        }
    }

    mod try_decode {
        use super::*;

        #[test]
        fn initialization() {
            let payload = Payload::Initialization {
                node_sort: NodeSort::Cpnode {
                    configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::from_bits_retain(1), 2, 3).unwrap()
                }
            };
            assert_eq!(
                Payload::try_decode(&[b'I', b'C', 0, 0, 1, 0, 2, 3, 255, 255, 255, 255, 255, 255]),
                Ok(payload)
            );
        }

        #[test]
        #[expect(clippy::byte_char_slices)]
        fn poll_request() {
            assert_eq!(
                Payload::try_decode(&[b'P']),
                Ok(Payload::PollRequest)
            );
        }

        #[test]
        fn receive_data() {
            assert_eq!(
                Payload::try_decode(&[b'R', 1, 2, 3]),
                Ok(Payload::ReceiveData { data: Data::try_from(&[1,2,3]).unwrap() })
            );
        }

        #[test]
        fn transmit_data() {
            assert_eq!(
                Payload::try_decode(&[b'T', 1, 2, 3]),
                Ok(Payload::TransmitData { data: Data::try_from(&[1,2,3]).unwrap() })
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown() {
            assert_eq!(
                Payload::try_decode(&[b'A', 5, 4, 3, 2, 1]),
                Ok(Payload::Unknown { message_type: b'A', body: Data::try_from(&[5, 4, 3, 2, 1]).unwrap() })
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn invalid() {
            assert_eq!(
                Payload::try_decode(&[b'a', 5, 4, 3, 2, 1]),
                Err(Error::InvalidMessageType(b'a'))
            );
        }

        #[test]
        fn data_too_long() {
            let mut data = [0; Data::MAX_LEN + 2]; // 256 data + 1 for message type + 1 for too much data

            data[0] = b'R';
            assert_eq!(
                Payload::try_decode(&data),
                Err(Error::BodyTooLong)
            );

            data[0] = b'T';
            assert_eq!(
                Payload::try_decode(&data),
                Err(Error::BodyTooLong)
            );
        }

        #[cfg(not(feature = "experimenter"))]
        #[test]
        fn invalid_message_type() {
            assert_eq!(
                Payload::try_decode(&[0, 1, 2, 3, 4]),
                Err(Error::InvalidMessageType(0))
            );
        }
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, assert_de_tokens_error, Token};

        #[test]
        fn initialization_() {
            let payload =  Payload::Initialization {
                node_sort: NodeSort::Cpnode {
                    configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::from_bits_retain(1), 2, 3).unwrap()
                }
            };

            assert_tokens(
                &payload,
                &[
                    Token::StructVariant { name: "Payload", variant: "Initialization", len: 1 },
                        Token::String("node_sort"),
                        Token::StructVariant { name: "NodeSort", variant: "Cpnode", len: 1 },
                            Token::Str("configuration"),
                            Token::Struct { name: "CpnodeConfiguration", len: 4, },
                                Token::BorrowedStr("transmit_delay"),
                                Token::U16(0),
                                Token::BorrowedStr("options"),
                                Token::U16(1),
                                Token::BorrowedStr("input_bytes"),
                                Token::U16(2),
                                Token::BorrowedStr("output_bytes"),
                                Token::U16(3),
                            Token::StructEnd,
                        Token::StructVariantEnd,
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn poll_request() {
            let payload =  Payload::PollRequest;

            assert_tokens(
                &payload,
                &[
                    Token::UnitVariant { name: "Payload", variant: "PollRequest" }
                ]
            );
        }

        #[test]
        fn receive_data() {
            let data = &[1, 2];
            let payload =  Payload::ReceiveData { data: Data::try_from(data).unwrap() };

            assert_tokens(
                &payload,
                &[
                    Token::StructVariant { name: "Payload", variant: "ReceiveData", len: 1 },
                    Token::Str("data"),
                    Token::Bytes(data),
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn transmit_data() {
            let data = &[1, 2];
            let payload =  Payload::TransmitData { data: Data::try_from(data).unwrap() };

            assert_tokens(
                &payload,
                &[
                    Token::StructVariant { name: "Payload", variant: "TransmitData", len: 1 },
                    Token::Str("data"),
                    Token::Bytes(data),
                    Token::StructVariantEnd
                ]
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown() {
            let body = &[1, 2];
            let payload =  Payload::Unknown { message_type: b'Z', body: Data::try_from(body).unwrap() };

            assert_tokens(
                &payload,
                &[
                    Token::StructVariant { name: "Payload", variant: "Unknown", len: 2 },
                    Token::Str("message_type"),
                    Token::U8(b'Z'),
                    Token::Str("body"),
                    Token::Bytes(body),
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn invalid_variant() {
            #[cfg(feature = "experimenter")]
            let error = "unknown variant `Zzzz`, expected one of `Initialization`, `PollRequest`, `ReceiveData`, `TransmitData`, `Unknown`";
            #[cfg(not(feature = "experimenter"))]
            let error = "unknown variant `Zzzz`, expected one of `Initialization`, `PollRequest`, `ReceiveData`, `TransmitData`";

            let tokens = &[Token::StructVariant { name: "Payload", variant: "Zzzz", len: 0 }];

            assert_de_tokens_error::<Payload>(tokens, error);
        }
    }
}
