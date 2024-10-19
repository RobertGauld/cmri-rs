use log::trace;
use crate::packet::{Data as PacketData, Error as PacketError};
use crate::node_configuration::{
    NodeConfiguration,
    SusicConfiguration, UsicConfiguration, sic::node_cards::NodeCard,
    SminiConfiguration,
    CpmegaConfiguration, CpmegaOptions, CpnodeConfiguration, CpnodeOptions
};


/// Possible types of node, with their unique configuration options.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeSort {
    /// A classic USIC with upto 64 x 24 bit cards,
    /// or a SUSIC with upto 64 x 24 bit input/output cards.
    Usic {
        /// Configuration of the USIC.
        configuration: UsicConfiguration
    },

    /// A SUSIC with upto 64 x 32 bit input/output cards.
    Susic {
        /// Configuration of the SUSIC.
        configuration: SusicConfiguration
    },

    /// A SMINI with 24 input bits and 48 output bits.
    Smini {
        /// Configuration of the SMINI.
        configuration: SminiConfiguration
    },

    /// A CPNODE with 16-144 input/output bits.
    Cpnode {
        /// Configuration of the CPNODE.
        configuration: CpnodeConfiguration
    },

    /// A CPMEGA with 0-192 input/output bits.
    Cpmega {
        /// Configuration of the CPMEGA.
        configuration: CpmegaConfiguration
    },

    #[cfg(feature = "experimenter")]
    #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "experimenter")))]
    #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature experimenter only.**\n\n")]
    /// An unknown Node Description Parameter.
    Unknown {
        /// The packet's body.
        body: PacketData
    }
}

impl NodeSort {
    /// Create a new USIC.
    ///
    /// See: [`UsicConfiguration::try_new`]
    #[expect(clippy::missing_errors_doc)]
    pub fn try_new_usic(transmit_delay: u16, cards: &[NodeCard]) -> Result<Self, crate::node_configuration::sic::node_cards::Error> {
        Ok(Self::Usic { configuration: UsicConfiguration::try_new(transmit_delay, cards)? })
    }

    /// Create a new SUSIC.
    ///
    /// See: [`SusicConfiguration::try_new`]
    #[expect(clippy::missing_errors_doc)]
    pub fn try_new_susic(transmit_delay: u16, cards: &[NodeCard]) -> Result<Self, crate::node_configuration::sic::node_cards::Error> {
        Ok(Self::Susic { configuration: SusicConfiguration::try_new(transmit_delay, cards)? })
    }

    /// Create a new SMINI.
    ///
    /// See: [`SminiConfiguration::try_new`]
    #[expect(clippy::missing_errors_doc)]
    pub const fn try_new_smini(transmit_delay: u16, oscillating_pairs: [u8; 6]) -> Result<Self, crate::node_configuration::SminiConfigurationError> {
        match SminiConfiguration::try_new(transmit_delay, oscillating_pairs) {
            Err(err) => Err(err),
            Ok(configuration) => Ok(Self::Smini { configuration })
        }
    }

    /// Create a new CPNODE.
    ///
    /// See: [`CpnodeConfiguration::try_new`]
    #[expect(clippy::missing_errors_doc)]
    pub const fn try_new_cpnode(transmit_delay: u16, options: CpnodeOptions, input_bytes: u8, output_bytes: u8) -> Result<Self, crate::node_configuration::CpConfigurationError> {
        match CpnodeConfiguration::try_new(transmit_delay, options, input_bytes, output_bytes) {
            Err(err) => Err(err),
            Ok(configuration) => Ok(Self::Cpnode { configuration })
        }
    }

    /// Create a new CPMEGA.
    ///
    /// See: [`CpmegaConfiguration::try_new`]
    #[expect(clippy::missing_errors_doc)]
    pub const fn try_new_cpmega(transmit_delay: u16, options: CpmegaOptions, input_bytes: u8, output_bytes: u8) -> Result<Self, crate::node_configuration::CpConfigurationError> {
        match CpmegaConfiguration::try_new(transmit_delay, options, input_bytes, output_bytes) {
            Err(err) => Err(err),
            Ok(configuration) => Ok(Self::Cpmega { configuration })
        }
    }

    #[cfg(feature = "experimenter")]
    #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "experimenter")))]
    #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature experimenter only.**\n\n")]
    /// Create a new unknown module.
    ///
    /// # Errors
    ///
    /// `PacketError::BodyTooLong` if slice is over 256 bytes.
    pub fn try_new_unknown(data: &[u8]) -> Result<Self, PacketError> {
        Ok(Self::Unknown { body: data.try_into()? })
    }

    /// Get the configuration for the `NodeSort`.
    #[cfg_attr(feature = "experimenter", doc = "\n\n# Panics\n\nIf the node type is the Unknown variant.\n")]
    #[must_use]
    pub fn configuration(&self) -> &dyn NodeConfiguration {
        match self {
            Self::Cpnode { configuration } => configuration,
            Self::Cpmega { configuration } => configuration,
            Self::Smini  { configuration } => configuration,
            Self::Usic   { configuration } => configuration,
            Self::Susic  { configuration } => configuration,
            #[cfg(feature = "experimenter")]
            Self::Unknown { body } => panic!("Unknown node type 0x{:02X}", body[0])
        }
    }

    /// Decode from an unescaped packet payload
    ///
    /// # Errors
    ///
    ///   * [`PacketError::InvalidNodeType`] if the NDP byte isn't valid.
    ///   * [`PacketError::TooShort`] if the slice isn't long enough.
    ///   * [`PacketError::InvalidConfiguration`]
    ///     * For USIC/SUSIC nodes:
    ///       * [`crate::node_configuration::NodeCardsError::InvalidCardType`] if there's an invalid card type in the card types sequence.
    ///       * [`crate::node_configuration::NodeCardsError::CardAfterNone`] if there's an Input or Output card after the first None card.
    ///       * [`crate::node_configuration::NodeCardsError::TooManyCards`] if there's more than 64 input/output cards.
    ///     * For SMINI node:
    ///       * [`crate::node_configuration::SminiConfigurationError::NonAdjacent`] if `oscillating_pairs` has an odd number of true bits.
    ///       * [`crate::node_configuration::SminiConfigurationError::NonAdjacent`] if `oscillating_pairs` has a pair of true bits which aren't adjacent.
    ///     * For CPNODE/CPMEGA nodes:
    ///       * [`crate::node_configuration::CpConfigurationError::InvalidInputOutputBitsCount`] if the total number of input and output bits is invalid for a `", stringify!($name), "` (", stringify!($bpc), ").")]
    pub(crate) fn try_decode(raw: &[u8]) -> Result<Self, PacketError> {
        trace!("NodeSort::decode({raw:?})");
        match raw[0] {
            super::NDP_CPNODE => Ok(Self::Cpnode { configuration: CpnodeConfiguration::decode(raw)? }),
            super::NDP_CPMEGA => Ok(Self::Cpmega { configuration: CpmegaConfiguration::decode(raw)? }),
            super::NDP_SMINI  => Ok(Self::Smini  { configuration: SminiConfiguration::decode(raw)? }),
            super::NDP_USIC   => Ok(Self::Usic   { configuration: UsicConfiguration::decode(raw)? }),
            super::NDP_SUSIC  => Ok(Self::Susic  { configuration: SusicConfiguration::decode(raw)? }),
            _ => {
                #[cfg(feature = "experimenter")]
                if raw[0].is_ascii_alphabetic() {
                    return Ok(Self::Unknown { body: raw.try_into()? });
                }
                Err(PacketError::InvalidNodeType(raw[0]))
            }
        }
    }

    pub(crate) fn encode(&self) -> PacketData {
        trace!("NodeSort.encode({self:?})");
        match self {
            Self::Cpnode { configuration } => configuration.encode(),
            Self::Cpmega { configuration } => configuration.encode(),
            Self::Smini { configuration } => configuration.encode(),
            Self::Usic { configuration } => configuration.encode(),
            Self::Susic { configuration } => configuration.encode(),
            #[cfg(feature = "experimenter")]
            Self::Unknown { body } => *body
        }
    }
}

impl core::fmt::Display for NodeSort {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Cpnode { .. } => write!(f, "CPNODE"),
            Self::Cpmega { .. } => write!(f, "CPMEGA"),
            Self::Smini { .. } => f.write_str("SMINI"),
            Self::Usic { .. } => write!(f, "USIC"),
            Self::Susic { .. } => write!(f, "SUSIC"),
            #[cfg(feature = "experimenter")]
            Self::Unknown { body } => if body[0].is_ascii_uppercase() {
                write!(f, "Experimental ({})", char::from(body[0]))
            } else {
                write!(f, "Unknown ({})", body[0])
            },
        }
    }
}

#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use crate::NodeSort;
    use crate::packet::Error as PacketError;
    #[cfg(feature = "experimenter")]
    use crate::packet::Data as PacketData;
    use crate::node_configuration::{*, sic::node_cards::{NodeCards, NodeCard}};

    mod creating {
        use super::*;

        #[test]
        fn try_new_usic() {
            let cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
            assert_eq!(
                NodeSort::try_new_usic(25, cards.as_slice()).unwrap(),
                NodeSort::Usic { configuration: UsicConfiguration { transmit_delay: 25, cards } }
            );
        }

        #[test]
        fn try_new_susic() {
            let cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
            assert_eq!(
                NodeSort::try_new_susic(50, cards.as_slice()).unwrap(),
                NodeSort::Susic { configuration: SusicConfiguration { transmit_delay: 50, cards } }
            );
        }

        #[test]
        fn try_new_smini() {
            let oscillating_pairs = [3, 6, 12, 24, 48, 99];
            assert_eq!(
                NodeSort::try_new_smini(75, oscillating_pairs).unwrap(),
                NodeSort::Smini { configuration: SminiConfiguration { transmit_delay: 75, oscillating_count: 7, oscillating_pairs } }
            );
        }

        #[test]
        fn try_new_cpnode() {
            assert_eq!(
                NodeSort::try_new_cpnode(100, CpnodeOptions::from_bits_retain(8), 1, 2).unwrap(),
                NodeSort::Cpnode { configuration: CpnodeConfiguration { transmit_delay: 100, options: CpnodeOptions::from_bits_retain(8), input_bytes: 1, output_bytes: 2 } }
            );
        }

        #[test]
        fn try_new_cpmega() {
            assert_eq!(
                NodeSort::try_new_cpmega(125, CpmegaOptions::from_bits_retain(16), 2, 4).unwrap(),
                NodeSort::Cpmega { configuration: CpmegaConfiguration { transmit_delay: 125, options: CpmegaOptions::from_bits_retain(16), input_bytes: 2, output_bytes: 4 } }
            );
        }
    }

    mod configuration {
        use super::*;

        #[test]
        fn usic() {
            let configuration = UsicConfiguration::try_new(2, &[]).unwrap();
            let node_type = NodeSort::Usic { configuration };
            assert_eq!(configuration.transmit_delay(), node_type.configuration().transmit_delay());
            assert_eq!(configuration.input_bytes(), node_type.configuration().input_bytes());
            assert_eq!(configuration.output_bytes(), node_type.configuration().output_bytes());
        }

        #[test]
        fn susic() {
            let configuration = SusicConfiguration::try_new(3, &[]).unwrap();
            let node_type = NodeSort::Susic { configuration };
            assert_eq!(configuration.transmit_delay(), node_type.configuration().transmit_delay());
            assert_eq!(configuration.input_bytes(), node_type.configuration().input_bytes());
            assert_eq!(configuration.output_bytes(), node_type.configuration().output_bytes());
        }

        #[test]
        fn mini() {
            let configuration = SminiConfiguration::try_new(4, [0, 0, 0, 0, 0, 0]).unwrap();
            let node_type = NodeSort::Smini { configuration };
            assert_eq!(configuration.transmit_delay(), node_type.configuration().transmit_delay());
            assert_eq!(configuration.input_bytes(), node_type.configuration().input_bytes());
            assert_eq!(configuration.output_bytes(), node_type.configuration().output_bytes());
        }

        #[test]
        fn cpnode() {
            let configuration = CpnodeConfiguration::try_new(5, CpnodeOptions::default(), 6, 7).unwrap();
            let node_type = NodeSort::Cpnode { configuration };
            assert_eq!(configuration.transmit_delay(), node_type.configuration().transmit_delay());
            assert_eq!(configuration.input_bytes(), node_type.configuration().input_bytes());
            assert_eq!(configuration.output_bytes(), node_type.configuration().output_bytes());
        }

        #[test]
        fn cpmega() {
            let configuration = CpmegaConfiguration::try_new(5, CpmegaOptions::default(), 6, 7).unwrap();
            let node_type = NodeSort::Cpmega { configuration };
            assert_eq!(configuration.transmit_delay(), node_type.configuration().transmit_delay());
            assert_eq!(configuration.input_bytes(), node_type.configuration().input_bytes());
            assert_eq!(configuration.output_bytes(), node_type.configuration().output_bytes());
        }
    }

    mod try_decode {
        use super::*;

        #[test]
        fn usic() {
            assert_eq!(
                NodeSort::try_decode(&[b'N', 0, 0, 0]),
                Ok(NodeSort::Usic { configuration: UsicConfiguration::try_new(0, &[]).unwrap() })
            );
        }

        #[test]
        fn susic() {
            assert_eq!(
                NodeSort::try_decode(&[b'X', 0, 0, 0]),
                Ok(NodeSort::Susic { configuration: SusicConfiguration::try_new(0, &[]).unwrap() })
            );
        }

        #[test]
        fn mini() {
            assert_eq!(
                NodeSort::try_decode(&[b'M', 0, 0, 0]),
                Ok(NodeSort::Smini { configuration: SminiConfiguration::try_new(0, [0, 0, 0, 0, 0, 0]).unwrap() })
            );
        }

        #[test]
        fn cpnode() {
            assert_eq!(
                NodeSort::try_decode(&[b'C', 0, 0, 0, 0, 1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
                Ok(NodeSort::Cpnode { configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 1, 2).unwrap() })
            );
        }

        #[test]
        fn cpmega() {
            assert_eq!(
                NodeSort::try_decode(&[b'O', 0, 0, 0, 0, 2, 1, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
                Ok(NodeSort::Cpmega { configuration: CpmegaConfiguration::try_new(0, CpmegaOptions::default(), 2, 1).unwrap() })
            );
        }

        #[cfg(feature = "experimenter")]
        mod experimenter {
            use super::*;

            #[test]
            fn unknown_type() {
                assert_eq!(
                    NodeSort::try_decode(&[b'A', 0, 10, 20, 30]),
                    Ok(NodeSort::Unknown { body: PacketData::try_from(&[b'A', 0, 10, 20, 30]).unwrap() })
                );

                assert_eq!(
                    NodeSort::try_decode(&[b'z', 0, 10, 20, 30]),
                    Ok(NodeSort::Unknown { body: PacketData::try_from(&[b'z', 0, 10, 20, 30]).unwrap() })
                );
            }

            #[test]
            fn invalid_type() {
                assert_eq!(
                    NodeSort::try_decode(&[b'5', 1, 2, 3]),
                    Err(PacketError::InvalidNodeType(b'5'))
                );
            }
            }

        #[cfg(not(feature = "experimenter"))]
        #[test]
        fn invalid_type() {
            assert_eq!(
                NodeSort::try_decode(&[b'A', 1, 2, 3]),
                Err(PacketError::InvalidNodeType(65))
            );
        }
    }

    mod encode {
        use super::*;

        #[test]
        fn usic() {
            let cards = [
                NodeCard::Input,
                NodeCard::Input,
                NodeCard::Input,
                NodeCard::Input,
                NodeCard::Output,
                NodeCard::Output
            ];

            let node_type = NodeSort::Usic {
                configuration: UsicConfiguration::try_new(
                    0,
                    &cards
                ).unwrap()
            };
            assert_eq!(
                node_type.encode(),
                [b'N', 0, 0, 2, 0b0101_0101, 0b0000_1010]
            );
        }

        #[test]
        fn susic() {
            let node_type = NodeSort::Susic {
                configuration: SusicConfiguration::try_new(
                    0,
                    &[NodeCard::Input, NodeCard::Input, NodeCard::Output, NodeCard::Output]
                ).unwrap()
            };
            assert_eq!(
                node_type.encode(),
                [b'X', 0, 0, 1, 0b1010_0101]
            );
        }

        #[test]
        fn mini() {
            assert_eq!(
                NodeSort::Smini { configuration: SminiConfiguration::try_new(0, [0, 0, 0, 0, 0, 0]).unwrap() }.encode(),
                [b'M', 0, 0, 0]
            );
        }

        #[test]
        fn cpnode() {
            assert_eq!(
                NodeSort::Cpnode { configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::from_bits_retain(1), 2, 3).unwrap() }.encode(),
                [b'C', 0, 0, 1, 0, 2, 3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
            );
        }

        #[test]
        fn cpmega() {
            assert_eq!(
                NodeSort::Cpmega { configuration: CpmegaConfiguration::try_new(0, CpmegaOptions::from_bits_retain(1), 2, 3).unwrap() }.encode(),
                [b'O', 0, 0, 1, 0, 2, 3, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
            );
        }

        #[cfg(feature = "experimenter")]
        #[test]
        fn unknown() {
            assert_eq!(
                NodeSort::Unknown { body: PacketData::try_from(&[b'A', 100, 50, 75, 25]).unwrap() }.encode(),
                [b'A', 100, 50, 75, 25]
            );
        }
    }

    #[cfg(feature = "std")]
    mod display {
        use super::*;

        #[test]
        fn cpnode() {
            let node = NodeSort::Cpnode {
                configuration: CpnodeConfiguration::try_new(0, CpnodeOptions::from_bits_retain(0), 1, 2).unwrap()
            };
            assert_eq!(format!("{node}"), "CPNODE");
        }

        #[test]
        fn cpmega() {
            let node = NodeSort::Cpmega {
                configuration: CpmegaConfiguration::try_new(0, CpmegaOptions::from_bits_retain(0), 2, 1).unwrap()
            };
            assert_eq!(format!("{node}"), "CPMEGA");
        }

        #[test]
        fn smini() {
            let node = NodeSort::Smini {
                configuration: SminiConfiguration::try_new(0, [0, 0, 0, 0, 0, 0]).unwrap()
            };
            assert_eq!(format!("{node}"), "SMINI");
        }

        #[test]
        fn usic() {
            let node = NodeSort::Usic {
                configuration: UsicConfiguration::try_new(0, &[NodeCard::Input, NodeCard::Input, NodeCard::Output]).unwrap()
            };
            assert_eq!(format!("{node}"), "USIC");
        }

        #[test]
        fn susic() {
            let node = NodeSort::Susic {
                configuration: SusicConfiguration::try_new(0, &[NodeCard::Output, NodeCard::Output, NodeCard::Input]).unwrap()
            };
            assert_eq!(format!("{node}"), "SUSIC");
        }

        #[cfg(feature = "experimenter")]
        mod unknown {
            use super::*;

            #[test]
            fn is_ascii_uppercase() {
                let node = NodeSort::Unknown {
                    body: PacketData::try_from(&[b'Z', 1, 2]).unwrap()
                };
                assert_eq!(format!("{node}"), "Experimental (Z)");
            }

            #[test]
            fn not_ascii_uppercase() {
                let node = NodeSort::Unknown {
                    body: PacketData::try_from(&[250, 1, 2]).unwrap()
                };
                assert_eq!(format!("{node}"), "Unknown (250)");
            }
        }
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, Token};

        #[test]
        fn usic() {
            let configuration = UsicConfiguration::try_new(0, &[]).unwrap();
            let node = NodeSort::Usic { configuration };
            assert_tokens(
                &node,
                &[
                    Token::StructVariant { name: "NodeSort", variant: "Usic", len: 1 },
                        Token::Str("configuration"),
                        Token::Struct { name: "UsicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd,
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn susic() {
            let configuration = SusicConfiguration::try_new(0, &[]).unwrap();
            let node = NodeSort::Susic { configuration };
            assert_tokens(
                &node,
                &[
                    Token::StructVariant { name: "NodeSort", variant: "Susic", len: 1 },
                        Token::Str("configuration"),
                        Token::Struct { name: "SusicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd,
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn smini() {
            let configuration = SminiConfiguration::try_new(0, [0; 6]).unwrap();
            let node = NodeSort::Smini { configuration };
            assert_tokens(
                &node,
                &[
                    Token::StructVariant { name: "NodeSort", variant: "Smini", len: 1 },
                        Token::Str("configuration"),
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("oscillating_count"),
                            Token::U8(0),
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0; 6]),
                        Token::StructEnd,
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn cpnode() {
            let configuration = CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 1, 1).unwrap();
            let node = NodeSort::Cpnode { configuration };
            assert_tokens(
                &node,
                &[
                    Token::StructVariant { name: "NodeSort", variant: "Cpnode", len: 1 },
                        Token::Str("configuration"),
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(1),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(1),
                        Token::StructEnd,
                    Token::StructVariantEnd
                ]
            );
        }

        #[test]
        fn cpmega() {
            let configuration = CpmegaConfiguration::try_new(0, CpmegaOptions::default(), 0, 0).unwrap();
            let node = NodeSort::Cpmega { configuration };
            assert_tokens(
                &node,
                &[
                    Token::StructVariant { name: "NodeSort", variant: "Cpmega", len: 1 },
                        Token::Str("configuration"),
                        Token::Struct { name: "CpmegaConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd,
                    Token::StructVariantEnd
                ]
            );
        }
    }
}
