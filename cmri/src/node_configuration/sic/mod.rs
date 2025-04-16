//! Details for USICs and SUSICs.

pub mod node_cards;

use node_cards::{NodeCards, NodeCard, Error as NodeCardsError};

macro_rules! common_implementation {
    ($name:ident, $ndp:expr, $bpc:expr) => {
        pastey::paste! {
            common_implementation!($name, $ndp, $bpc, [<$name:camel Configuration>], [<$name:upper>]);
        }
    };
    ($name:ident, $ndp:expr, $bpc:expr, $serde_name:ident, $human_name:ident) => {
        pastey::paste! {
            common_implementation!($name, $ndp, $bpc, stringify!($serde_name), stringify!($human_name));
        }
    };
    ($name:ident, $ndp:expr, $bpc:expr, $serde_name:expr, $human_name:expr) => {
        mod $name {
            use log::trace;
            use crate::node_configuration::NodeConfiguration;
            use crate::packet::{Data as PacketData, Error as PacketError};
            use super::{NodeCards, NodeCard, NodeCardsError};
            #[allow(unused_imports)]
            use super::super::{NDP_USIC, NDP_SUSIC};

            #[doc = concat!("Configuration for a (S)USIC node with ", $bpc, " bit cards.")]
            #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
            pub struct Configuration {
                pub(in super::super) transmit_delay: u16,
                pub(in super::super) cards: NodeCards
            }

            impl Configuration {
                /// Bytes per card.
                const BPC: u8 = $bpc / 8;

                #[doc = concat!("Create a new `", stringify!($name), "`.")]
                #[doc = ""]
                #[doc = "# Errors"]
                #[doc = ""]
                #[doc = "* [`NodeCardsError::CardAfterNone`] if there's an Input or Output card after a None card."]
                #[doc = "* [`NodeCardsError::TooManyCards`] if there's more than 64 input/output cards."]
                pub fn try_new(transmit_delay: u16, cards: &[NodeCard]) -> Result<Self, NodeCardsError> {
                    let cards = NodeCards::try_new(cards)?;
                    Ok(Self { transmit_delay, cards })
                }

                /// The cards connected to the node, including the Nones at the end.
                #[must_use]
                pub fn cards(&self) -> &[NodeCard] {
                    self.cards.as_slice()
                }

                #[doc = concat!("Create a new `", stringify!($name), "` from raw bytes.")]
                #[doc = ""]
                #[doc = "# Errors"]
                #[doc = ""]
                #[doc = concat!("* [`PacketError::InvalidNodeType`] if the NDP byte isn't valid for a ", stringify!($name), " node.")]
                #[doc = "* [`PacketError::InvalidConfiguration`]:"]
                #[doc = "  * [`NodeCardsError::InvalidCardType`] if there's an invalid card type in the card types sequence."]
                #[doc = "  * [`NodeCardsError::CardAfterNone`] if there's an Input or Output card after the first None card."]
                #[doc = "  * [`NodeCardsError::TooManyCards`] if there's more than 64 input/output cards."]
                pub(in super::super) fn decode(raw: &[u8]) -> Result<Self, PacketError> {
                    trace!(concat!(stringify!($name), "::decode({:?})"), raw);
                    if raw[0] != $ndp {
                        return Err(PacketError::InvalidNodeType(raw[0]))
                    }

                    let mut cards = [NodeCard::None; 64];
                    for (index, &byte) in raw.iter().skip(4).enumerate() {
                        for i in 0..4 {
                            match (byte >> (2 * i)) & 0b11 {
                                0b00 => (),
                                0b01 => cards[(index * 4) + i] = NodeCard::Input,
                                0b10 => cards[(index * 4) + i] = NodeCard::Output,
                                _ => return Err(NodeCardsError::InvalidCardType.into())
                            }
                        }
                    }

                    Ok(Self::try_new(
                        u16::from_be_bytes([raw[1], raw[2]]),
                        &cards
                    )?)
                }

                pub(in super::super) fn encode(&self) -> PacketData {
                    trace!(concat!(stringify!($name), ".encode({:?})"), self);
                    let mut raw = PacketData::default();

                    raw.push($ndp).expect("Always pushes less than the maximum.");

                    let transmit_delay = self.transmit_delay.to_be_bytes();
                    raw.push(transmit_delay[0]).expect("Always pushes less than the maximum.");
                    raw.push(transmit_delay[1]).expect("Always pushes less than the maximum.");

                    // Set initial count of sets of 4 cards
                    let count_index = raw.len();
                    raw.push(0).expect("Always pushes less than the maximum.");

                    for chunk in self.cards.as_slice().chunks(4) {
                        let mut byte = 0;
                        for (i, &card) in chunk.iter().enumerate() {
                            byte |= (card as u8) << u8::try_from(2 * i).expect("Upto 64 cards * 2 = Upto 128, which is less than 255.");
                        }
                        if byte == 0 { break } // There are no cards at all in this set of 4
                        raw[count_index] += 1;
                        raw.push(byte).expect("Always pushes less than the maximum.");
                        if byte & 0b1100_0000 == 0 { break } // This is the last set of 4 cards
                    }

                    raw
                }
            }

            impl NodeConfiguration for Configuration {
                fn transmit_delay(&self) -> u16 { self.transmit_delay }
                fn input_bytes(&self) -> u16 { u16::from(self.cards.input_cards()) * u16::from(Self::BPC) }
                fn output_bytes(&self) -> u16 { u16::from(self.cards.output_cards()) * u16::from(Self::BPC) }
            }

            #[cfg(feature = "serde")]
            #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
            #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
            impl ::serde::Serialize for Configuration {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                    use serde::ser::SerializeStruct;
                    let mut ser = serializer.serialize_struct($serde_name, 4)?;
                    ser.serialize_field("transmit_delay", &self.transmit_delay)?;
                    ser.serialize_field("cards", &self.cards)?;
                    ser.serialize_field("input_bytes", &self.input_bytes())?;
                    ser.serialize_field("output_bytes", &self.output_bytes())?;
                    ser.end()
                }
            }

            #[cfg(feature = "serde")]
            impl<'de> serde::de::Deserialize<'de> for Configuration {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                    struct Visitor;
                    impl<'de> serde::de::Visitor<'de> for Visitor {
                        type Value = Configuration;

                        fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                            write!(formatter, concat!("a ", $serde_name))
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de> {
                            let mut transmit_delay = None;
                            let mut cards: Option<NodeCards> = None;

                            while let Some(key) = map.next_key()? {
                                match key {
                                    "transmit_delay" => {
                                        if transmit_delay.is_some() {
                                            return Err(serde::de::Error::duplicate_field("transmit_delay"));
                                        }
                                        transmit_delay = Some(map.next_value()?);
                                    },
                                    "cards" => {
                                        if cards.is_some() {
                                            return Err(serde::de::Error::duplicate_field("cards"));
                                        }
                                        cards = Some(map.next_value()?);
                                    },
                                    "input_bytes" | "output_bytes" =>  {
                                        // Ignored as calculated from cards
                                        let _:u8 = map.next_value()?;
                                    },
                                    _ => {
                                        return Err(serde::de::Error::unknown_field(key, &["transmit_delay, cards"]));
                                    }
                                }
                            }

                            let transmit_delay = transmit_delay.unwrap_or_default();
                            let cards = cards.ok_or_else(|| serde::de::Error::missing_field("cards"))?;
                            Configuration::try_new(transmit_delay, cards.as_slice()).map_err(serde::de::Error::custom)
                        }
                    }

                    deserializer.deserialize_struct($serde_name, &["transmit_delay", "cards"], Visitor)
                }
            }
        }
    }
}


common_implementation!(usic, NDP_USIC, 24);
pub use usic::Configuration as UsicConfiguration;
common_implementation!(susic, NDP_SUSIC, 32);
pub use susic::Configuration as SusicConfiguration;


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use super::{UsicConfiguration, SusicConfiguration, NodeCards, NodeCard, NodeCardsError};
    use crate::{packet::Error as PacketError, node_configuration::NodeConfiguration};

    mod usic {
        use super::*;

        mod try_new {
            use super::*;

            #[test]
            fn creates() {
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Output;
                cards[2] = NodeCard::Output;

                assert_eq!(
                    UsicConfiguration::try_new(10, &cards),
                    Ok(
                        UsicConfiguration {
                            transmit_delay: 10,
                            cards: NodeCards::try_new(&cards).unwrap()
                        }
                    )
                );
            }

            #[test]
            fn too_many_cards() {
                let cards = [NodeCard::None; 65];
                assert_eq!(
                    SusicConfiguration::try_new(0, &cards),
                    Err(NodeCardsError::TooManyCards)
                );
            }

            #[test]
            fn card_after_first_none() {
                assert_eq!(
                    UsicConfiguration::try_new(0, &[NodeCard::None, NodeCard::Input]),
                    Err(NodeCardsError::CardAfterNone)
                );
            }
        }

        mod decode {
            use super::*;

            #[test]
            fn nocards() {
                let raw = [b'N', 0x01, 0xF4, 0];
                assert_eq!(
                    UsicConfiguration::decode(&raw),
                    Ok(
                        UsicConfiguration {
                            transmit_delay: 500,
                            cards: NodeCards::default()
                        }
                    )
                );
            }

            #[test]
            fn cards_ioox() {
                let raw = [b'N', 0, 0, 1, 0b0010_1001];
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Output;
                cards[2] = NodeCard::Output;

                assert_eq!(
                    UsicConfiguration::decode(&raw),
                    Ok(
                        UsicConfiguration {
                            transmit_delay: 0,
                            cards: NodeCards::try_new(&cards).unwrap()
                        }
                    )
                );
            }

            #[test]
            fn card_after_first_none() {
                let raw = [b'N', 0, 0, 1, 0b1000_0000];
                assert_eq!(
                    UsicConfiguration::decode(&raw),
                    Err(PacketError::InvalidConfiguration { source: NodeCardsError::CardAfterNone.into() })
                );
            }

            #[test]
            fn invalid_card_type() {
                let raw = [b'N', 0, 0, 1, 0b0000_0011];
                assert_eq!(
                    UsicConfiguration::decode(&raw),
                    Err(PacketError::InvalidConfiguration { source: NodeCardsError::InvalidCardType.into() })
                );
            }

            #[test]
            fn invalid_ndp() {
                let raw = [b'Z', 0x01, 0x2C, 0];
                let configuration = UsicConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Err(PacketError::InvalidNodeType(90))
                );
            }
        }

        mod encode {
            use super::*;

            #[test]
            fn nocards() {
                let configuration = UsicConfiguration {
                    transmit_delay: 500,
                    cards: NodeCards::default()
                };
                assert_eq!(
                    configuration.encode(),
                    [b'N', 0x01, 0xF4, 0]
                );
            }

            #[test]
            fn cards_ioox() {
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Output;
                cards[2] = NodeCard::Output;

                let configuration = UsicConfiguration {
                    transmit_delay: 0,
                    cards: NodeCards::try_new(&cards).unwrap()
                };

                assert_eq!(
                    configuration.encode(),
                    [b'N', 0, 0, 1, 0b0010_1001]
                );
            }
        }

        #[test]
        fn node_configuration() {
            let configuration = UsicConfiguration::try_new(
                200,
                &[NodeCard::Input, NodeCard::Output, NodeCard::Output],
            ).unwrap();

            assert_eq!(configuration.transmit_delay(), 200);
            assert_eq!(configuration.input_bytes(), 3);
            assert_eq!(configuration.output_bytes(), 6);
        }

        #[test]
        fn cards() {
            let cards = NodeCards::try_new(&[NodeCard::Input]).unwrap();
            let configuration = UsicConfiguration::try_new(0, cards.as_slice()).unwrap();
            assert_eq!(configuration.cards(), cards.as_slice());
        }

        #[cfg(feature = "serde")]
        mod serde {
            use super::*;
            use serde_test::{assert_tokens, assert_de_tokens, assert_de_tokens_error, Token};

            #[test]
            fn valid() {
                let configuration = UsicConfiguration::try_new(
                    1,
                    &[NodeCard::Output, NodeCard::Output, NodeCard::Input]
                ).unwrap();

                assert_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "UsicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(1),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Input" },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(3),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(6),
                        Token::StructEnd
                    ]
                );
            }

            #[test]
            fn ignored_fields_for_deserialize() {
                // Values of input_bytes and output_bytes are calculated from cards.
                let configuration = UsicConfiguration::try_new(
                    1,
                    &[NodeCard::Output, NodeCard::Output, NodeCard::Input]
                ).unwrap();

                assert_de_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "UsicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(1),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Input" },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd
                    ]
                );
            }

            mod optional_fields_for_deserialize {
                use super::*;

                #[test]
                fn transmit_delay() { // Defaults to 0
                    let configuration = UsicConfiguration::try_new(
                        0,
                        &[]
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "UsicConfiguration", len: 4 },
                                Token::BorrowedStr("cards"),
                                Token::Seq { len: None },
                                Token::SeqEnd,
                            Token::StructEnd
                        ]
                    );
                }
            }

            #[test]
            fn too_many_cards() {
                assert_de_tokens_error::<UsicConfiguration>(
                    &[
                        Token::Struct { name: "UsicConfiguration", len: 1 },
                        Token::BorrowedStr("cards"),
                        Token::Seq { len: None },
                            // 65 cards is 1 too many.
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                        Token::SeqEnd
                    ],
                    "invalid length 65, expected no more than 64 cards"
                );

                assert_de_tokens_error::<UsicConfiguration>(
                    &[
                        Token::Struct { name: "UsicConfiguration", len: 1 },
                        Token::BorrowedStr("cards"),
                        Token::Seq { len: Some(65) },
                    ],
                    "invalid length 65, expected no more than 64 cards"
                );
            }

            #[test]
            fn card_after_first_none() {
                assert_de_tokens_error::<UsicConfiguration>(
                    &[
                        Token::Struct { name: "UsicConfiguration", len: 1 },
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "None" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::SeqEnd
                    ],
                    "expected no cards after the first none"
                );
            }
        }
    }

    mod susic {
        use super::*;

        mod try_new {
            use super::*;

            #[test]
            fn creates() {
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Input;
                cards[2] = NodeCard::Output;

                assert_eq!(
                    SusicConfiguration::try_new(10, &cards),
                    Ok(
                        SusicConfiguration {
                            transmit_delay: 10,
                            cards: NodeCards::try_new(&cards).unwrap()
                        }
                    )
                );
            }

            #[test]
            fn too_many_cards() {
                let cards = [NodeCard::None; 65];
                assert_eq!(
                    SusicConfiguration::try_new(0, &cards),
                    Err(NodeCardsError::TooManyCards)
                );
            }

            #[test]
            fn card_after_first_none() {
                assert_eq!(
                    SusicConfiguration::try_new(0, &[NodeCard::None, NodeCard::Input]),
                    Err(NodeCardsError::CardAfterNone)
                );
            }
        }

        mod decode {
            use super::*;

            #[test]
            fn nocards() {
                let raw = [b'X', 0x01, 0xF4, 0];
                assert_eq!(
                    SusicConfiguration::decode(&raw),
                    Ok(
                        SusicConfiguration {
                            transmit_delay: 500,
                            cards: NodeCards::default()
                        }
                    )
                );
            }

            #[test]
            fn cards_ioox() {
                let raw = [b'X', 0, 0, 1, 0b0010_1001];
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Output;
                cards[2] = NodeCard::Output;

                assert_eq!(
                    SusicConfiguration::decode(&raw),
                    Ok(
                        SusicConfiguration {
                            transmit_delay: 0,
                            cards: NodeCards::try_new(&cards).unwrap()
                        }
                    )
                );
            }

            #[test]
            fn card_after_first_none() {
                let raw = [b'X', 0, 0, 1, 0b1000_0000];
                assert_eq!(
                    SusicConfiguration::decode(&raw),
                    Err(PacketError::InvalidConfiguration { source: NodeCardsError::CardAfterNone.into() })
                );
            }

            #[test]
            fn invalid_card_type() {
                let raw = [b'X', 0, 0, 1, 0b0000_0011];
                assert_eq!(
                    SusicConfiguration::decode(&raw),
                    Err(PacketError::InvalidConfiguration { source: NodeCardsError::InvalidCardType.into() })
                );
            }


            #[test]
            fn invalid_ndp() {
                let raw = [b'Z', 0x01, 0x2C, 0];
                let configuration = SusicConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Err(PacketError::InvalidNodeType(90))
                );
            }
        }

        mod encode {
            use super::*;

            #[test]
            fn nocards() {
                let configuration = SusicConfiguration {
                    transmit_delay: 500,
                    cards: NodeCards::default()
                };
                assert_eq!(
                    configuration.encode(),
                    [b'X', 0x01, 0xF4, 0]
                );
            }

            #[test]
            fn cards_ioox() {
                let mut cards = [NodeCard::None; 64];
                cards[0] = NodeCard::Input;
                cards[1] = NodeCard::Output;
                cards[2] = NodeCard::Output;

                let configuration = SusicConfiguration {
                    transmit_delay: 0,
                    cards: NodeCards::try_new(&cards).unwrap()
                };

                assert_eq!(
                    configuration.encode(),
                    [b'X', 0, 0, 1, 0b0010_1001]
                );
            }
        }

        #[test]
        fn node_configuration() {
            let configuration = SusicConfiguration::try_new(
                200,
                &[NodeCard::Input, NodeCard::Output, NodeCard::Output]
            ).unwrap();

            assert_eq!(configuration.transmit_delay(), 200);
            assert_eq!(configuration.input_bytes(), 4);
            assert_eq!(configuration.output_bytes(), 8);
        }

        #[test]
        fn cards() {
            let cards = NodeCards::try_new(&[NodeCard::Output]).unwrap();
            let configuration = UsicConfiguration::try_new(0, cards.as_slice()).unwrap();
            assert_eq!(configuration.cards(), cards.as_slice());
        }

        #[cfg(feature = "serde")]
        mod serde {
            use super::*;
            use serde_test::{assert_tokens, assert_de_tokens, assert_de_tokens_error, Token};

            #[test]
            fn valid() {
                let configuration = SusicConfiguration::try_new(
                    1,
                    &[NodeCard::Output, NodeCard::Output, NodeCard::Input]
                ).unwrap();

                assert_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "SusicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(1),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Input" },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(4),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(8),
                        Token::StructEnd
                    ]
                );
            }

            #[test]
            fn ignored_fields_for_deserialize() {
                // Values of input_bytes and output_bytes are calculated form cards.
                let configuration = SusicConfiguration::try_new(
                    1,
                    &[NodeCard::Output, NodeCard::Output, NodeCard::Input]
                ).unwrap();

                assert_de_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "SusicConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(1),
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                                Token::UnitVariant { name: "NodeCard", variant: "Input" },
                            Token::SeqEnd,
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd
                    ]
                );
            }

            mod optional_fields_for_deserialize {
                use super::*;

                #[test]
                fn transmit_delay() { // Defaults to 0
                    let configuration = UsicConfiguration::try_new(
                        0,
                        &[]
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "UsicConfiguration", len: 4 },
                                Token::BorrowedStr("cards"),
                                Token::Seq { len: None },
                                Token::SeqEnd,
                            Token::StructEnd
                        ]
                    );
                }
            }

            #[test]
            fn too_many_cards() {
                assert_de_tokens_error::<SusicConfiguration>(
                    &[
                        Token::Struct { name: "SusicConfiguration", len: 1 },
                        Token::BorrowedStr("cards"),
                        Token::Seq { len: None },
                            // 65 cards is 1 too many.
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                        Token::SeqEnd
                    ],
                    "invalid length 65, expected no more than 64 cards"
                );

                assert_de_tokens_error::<SusicConfiguration>(
                    &[
                        Token::Struct { name: "SusicConfiguration", len: 1 },
                        Token::BorrowedStr("cards"),
                        Token::Seq { len: Some(65) },
                    ],
                    "invalid length 65, expected no more than 64 cards"
                );
            }

            #[test]
            fn card_after_first_none() {
                assert_de_tokens_error::<SusicConfiguration>(
                    &[
                        Token::Struct { name: "SusicConfiguration", len: 1 },
                            Token::BorrowedStr("cards"),
                            Token::Seq { len: None },
                                Token::UnitVariant { name: "NodeCard", variant: "None" },
                                Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::SeqEnd
                    ],
                    "expected no cards after the first none"
                );
            }
        }
    }
}
