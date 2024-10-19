//! Details for SMINIs.

use log::trace;
use const_for::const_for;
use super::NDP_SMINI;
use crate::packet::{Data as PacketData, Error as PacketError};
use crate::node_configuration::{NodeConfiguration, InvalidConfigurationError};

/// Errors which can happen when decoding/creating an `SminiConfiguration`.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// At least one pair of set bits aren't adjacent.
    #[error("At least one pair of set bits in oscillating pairs aren't adjacent.")]
    NonAdjacent
}
impl From<Error> for PacketError {
    fn from(source: Error) -> Self {
        let source = InvalidConfigurationError::Smini { source };
        Self::InvalidConfiguration { source }
    }
}

/// Configuration for a SMINI node.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename = "SminiConfiguration"))]
pub struct Configuration {
    pub(super) transmit_delay: u16,
    pub(super) oscillating_count: u8,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_bytes", deserialize_with = "deserialize_bytes"))]
    pub(super) oscillating_pairs: [u8; 6],
}

impl Configuration {
    /// Get the oscillating pairs for the node.
    /// If both outputs of a pair are on the the pair oscilates, allowing a red/green bicolour LED to display yellow.
    ///
    /// Returns &[Card0PortA, Card0PortB, Card0PortC, Card1PortA, Card1PortB, Card1PortC]
    #[expect(clippy::doc_markdown)]
    #[must_use]
    pub const fn oscillating_pairs(&self) -> &[u8; 6] {
        &self.oscillating_pairs
    }

    /// Create a new `Configuration`.
    ///
    /// # Errors
    ///
    /// [`Error::NonAdjacent`] if `oscillating_pairs` has a pair of set bits which aren't adjacent.
    pub const fn try_new(transmit_delay: u16, oscillating_pairs: [u8; 6]) -> Result<Self, Error> {
        match Self::get_oscillating_pairs_count(&oscillating_pairs) {
            Err(err) => Err(err),
            Ok(oscillating_count) => Ok(Self { transmit_delay, oscillating_count, oscillating_pairs })
        }
    }

    /// Get the number of adjacent pairs of set bits within `oscillating_pairs`.
    ///
    /// # Errors
    ///
    /// [`Error::NonAdjacent`] if `oscillating_pairs` has a pair of set bits which aren't adjacent.
    pub const fn get_oscillating_pairs_count(oscillating_pairs: &[u8; 6]) -> Result<u8, Error> {
        let check = u64::from_be_bytes([0, 0, oscillating_pairs[0], oscillating_pairs[1], oscillating_pairs[2], oscillating_pairs[3], oscillating_pairs[4], oscillating_pairs[5]]);
        let mut oscillating_count = 0;
        let mut streak = 0;
        const_for!( i in 0..64 => {
            if check & (1 << i) > 0 {
                // Streak of 1s has started/continued
                oscillating_count += 1;
                streak += 1;
            } else {
                // Streak of 1s has ended
                if streak % 2 != 0 {
                    return Err(Error::NonAdjacent)
                }
                streak = 0;
            }
        });
        Ok(oscillating_count / 2)
    }

    /// Create a new `Configuration` from raw bytes.
    ///
    /// # Errors
    ///
    /// * [`PacketError::InvalidNodeType`] if the NDP byte isn't valid for an SMINI node.
    /// * [`PacketError::TooShort`] if the slice isn't long enough.
    /// * [`PacketError::InvalidConfiguration`]:
    ///   * [`Error::NonAdjacent`] if `oscillating_pairs` has a pair of true bits which aren't adjacent.
    #[expect(clippy::missing_panics_doc)]
    pub(super) fn decode(raw: &[u8]) -> Result<Self, PacketError> {
        trace!("SminiConfiguration::decode({raw:?})");
        if raw[0] != NDP_SMINI {
            return Err(PacketError::InvalidNodeType(raw[0]))
        }

        if raw.len() < 4 || (raw[3] > 0 && raw.len() < 10) {
            return Err(PacketError::TooShort);
        }

        Ok(Self::try_new(
            u16::from_be_bytes([raw[1], raw[2]]),
            if raw[3] == 0 { [0; 6] } else { raw[4..=9].try_into().expect("Already checked it's long enough") }
        )?)
    }

    #[expect(clippy::missing_panics_doc, reason = "Never pushes more than Data::MAX_LEN")]
    pub(super) fn encode(&self) -> PacketData {
        trace!("SminiConfiguration.encode({self:?})");
        let mut raw = PacketData::default();
        raw.push(NDP_SMINI).expect("Always pushes less than the maximum.");

        let transmit_delay = self.transmit_delay.to_be_bytes();
        raw.push(transmit_delay[0]).expect("Always pushes less than the maximum.");
        raw.push(transmit_delay[1]).expect("Always pushes less than the maximum.");

        if self.oscillating_count == 0 {
            raw.push(0).expect("Always pushes less than the maximum.");
        } else {
            raw.push(self.oscillating_count).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[0]).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[1]).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[2]).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[3]).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[4]).expect("Always pushes less than the maximum.");
            raw.push(self.oscillating_pairs[5]).expect("Always pushes less than the maximum.");
        }
        raw
    }
}

impl NodeConfiguration for Configuration {
    fn transmit_delay(&self) -> u16 { self.transmit_delay }
    fn input_bytes(&self) -> u16 { 3 }
    fn output_bytes(&self) -> u16 { 6 }
}

#[cfg(feature = "serde")]
#[expect(clippy::missing_errors_doc)]
fn serialize_bytes<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    serializer.serialize_bytes(value)
}

#[cfg(feature = "serde")]
#[expect(clippy::missing_errors_doc)]
fn deserialize_bytes<'de, D>(deserializer: D) -> Result<[u8; 6], D::Error> where D: serde::Deserializer<'de> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = [u8; 6];

        fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(formatter, "6 bytes")
        }

        fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error {
            if value.len() != 6 {
                return Err(serde::de::Error::invalid_length(value.len(), &"6 bytes"))
            }
            value.try_into().map_err(|err: core::array::TryFromSliceError| serde::de::Error::custom(err))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de> {
            let mut value = [0; 6];
            #[expect(clippy::needless_range_loop, reason = "Used to ensure exactly 6 elements.")]
            for i in 0..6 {
                match seq.next_element() {
                    Err(error) => return Err(serde::de::Error::custom(error)),
                    Ok(None) => return Err(serde::de::Error::invalid_length(i, &self)),
                    Ok(Some(byte)) => value[i] = byte
                };
            }
            if seq.next_element().is_ok_and(|a: Option<u8>| a.is_some()) {
                Err(serde::de::Error::invalid_length(7, &self))
            } else {
                Ok(value)
            }
        }
    }

    deserializer.deserialize_bytes(Visitor)
}

#[cfg(feature = "serde")]
impl<'de> serde::de::Deserialize<'de> for Configuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Configuration;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "a SminiConfiguration")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de> {
                struct OscilatingPairs(Option<[u8; 6]>);
                impl<'de> serde::de::Deserialize<'de> for OscilatingPairs {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                        Ok(Self(Some(deserialize_bytes(deserializer)?)))
                    }
                }

                let mut transmit_delay = None;
                let mut oscillating_pairs = OscilatingPairs(None);

                while let Some(key) = map.next_key()? {
                    match key {
                        "transmit_delay" => {
                            if transmit_delay.is_some() {
                                return Err(serde::de::Error::duplicate_field("transmit_delay"));
                            }
                            transmit_delay = Some(map.next_value()?);
                        },
                        "oscillating_pairs" => {
                            if oscillating_pairs.0.is_some() {
                                return Err(serde::de::Error::duplicate_field("oscillating_pairs"));
                            }
                            oscillating_pairs = map.next_value()?;
                        },
                        "oscillating_count" =>  {
                            // Ignored as calculated from oscillating_pairs
                            let _:u8 = map.next_value()?;
                        },
                        _ => {
                            return Err(serde::de::Error::unknown_field(key, &["transmit_delay, oscillating_pairs"]));
                        }
                    }
                }

                let transmit_delay = transmit_delay.unwrap_or_default();
                let oscillating_pairs = oscillating_pairs.0.unwrap_or_default();
                Configuration::try_new(transmit_delay, oscillating_pairs).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_struct("SminiConfiguration", &["transmit_delay", "cards"], Visitor)
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use super::*;

    mod try_new {
        use super::*;

        #[test]
        fn creates() {
            assert_eq!(
                Configuration::try_new(4, [0, 0, 3, 6, 0, 0]),
                Ok(
                    Configuration {
                        transmit_delay: 4,
                        oscillating_count: 2,
                        oscillating_pairs: [0, 0, 3, 6, 0, 0]
                    }
                )
            );
        }

        mod invalid_oscillating_pairs {
            use super::*;

            #[test]
            fn pair_not_adjacent() {
                assert_eq!(
                    Configuration::try_new(4, [0b0010_1000, 0, 0, 0, 0, 0]),
                    Err(Error::NonAdjacent)
                );
            }
        }
    }

    mod get_oscillating_pairs_count {
        use super::*;

        #[test]
        fn valid() {
            assert_eq!(
                Configuration::get_oscillating_pairs_count(&[0, 0, 3, 6, 0, 0]),
                Ok(2)
            );
        }

        #[test]
        fn invalid() {
            assert_eq!(
                Configuration::get_oscillating_pairs_count(&[0b0010_1000, 0, 0, 0, 0, 0]),
                Err(Error::NonAdjacent)
            );
        }
    }

    mod decode {
        use super::*;

        #[test]
        fn with_oscillating_pairs() {
            let raw = [b'M', 0, 0, 6, 3, 6, 12, 24, 48, 96];
            assert_eq!(
                Configuration::decode(&raw),
                Ok(
                    Configuration {
                        transmit_delay: 0,
                        oscillating_count: 6,
                        oscillating_pairs: [3, 6, 12, 24, 48, 96]
                    }
                )
            );
        }

        #[test]
        fn with_oscillating_pairs_accross_bytes() {
            let raw = [b'M', 0, 0, 1, 0b0000_0001, 0b1000_0000, 0, 0, 0, 0, 0];
            assert_eq!(
                Configuration::decode(&raw),
                Ok(
                    Configuration {
                        transmit_delay: 0,
                        oscillating_count: 1,
                        oscillating_pairs: [1, 128, 0, 0, 0, 0]
                    }
                )
            );
        }

        #[test]
        fn without_oscillating_pairs() {
            let raw = [b'M', 0x01, 0x2C, 0];
            assert_eq!(
                Configuration::decode(&raw),
                Ok(
                    Configuration {
                        transmit_delay: 300,
                        oscillating_count: 0,
                        oscillating_pairs: [0; 6]
                    }
                )
            );
        }

        #[test]
        fn invalid_oscillating_pairs() {
            let raw = [b'M', 0, 0, 1, 0, 0, 0, 0, 0, 1];
            assert_eq!(
                Configuration::decode(&raw),
                Err(PacketError::InvalidConfiguration { source: InvalidConfigurationError::Smini { source: Error::NonAdjacent } })
            );
        }

        #[test]
        fn invalid_ndp() {
            let raw = [b'Z', 0x01, 0x2C, 0];
            let configuration = Configuration::decode(&raw);
            assert_eq!(
                configuration,
                Err(PacketError::InvalidNodeType(90))
            );
        }

        mod too_short {
            use super::*;

            #[test]
            fn without_oscillating_pairs() {
                assert!(Configuration::decode(&[b'M', 0, 0, 0]).is_ok());
                assert_eq!(
                    Configuration::decode(&[b'M', 0, 0]),
                    Err(PacketError::TooShort)
                );
            }

            #[test]
            fn with_oscillating_pairs() {
                assert!(Configuration::decode(&[b'M', 0, 0, 1, 3, 0, 0, 0, 0, 0]).is_ok());
                assert_eq!(
                    Configuration::decode(&[b'M', 0, 0, 1, 3, 0, 0, 0, 0]),
                    Err(PacketError::TooShort)
                );
            }
        }
    }

    mod encode {
        use super::*;

        #[test]
        fn with_oscillating_pairs() {
            assert_eq!(
                Configuration {
                    transmit_delay: 0,
                    oscillating_count: 2,
                    oscillating_pairs: [0, 0, 0, 0, 6, 3]
                }.encode(),
                [b'M', 0, 0, 2, 0, 0, 0, 0, 6, 3]
            );
        }

        #[test]
        fn without_oscillating_pairs() {
            assert_eq!(
                Configuration {
                    transmit_delay: 0,
                    oscillating_count: 0,
                    oscillating_pairs: [0; 6]
                }.encode(),
                [b'M', 0, 0, 0]
            );
        }
    }

    #[test]
    fn oscillating_pairs() {
        assert_eq!(
            Configuration {
                transmit_delay: 0,
                oscillating_count: 6,
                oscillating_pairs: [3, 6, 12, 24, 48, 96]
            }.oscillating_pairs(),
            &[3, 6, 12, 24, 48, 96]
        );
    }

    #[test]
    fn node_configuration() {
        let configuration = Configuration::try_new(
            200,
            [0; 6],
        ).unwrap();

        assert_eq!(configuration.transmit_delay(), 200);
        assert_eq!(configuration.input_bytes(), 3);
        assert_eq!(configuration.output_bytes(), 6);
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, assert_de_tokens, Token};

        #[test]
        fn valid() {
            let configuration = Configuration {
                transmit_delay: 0,
                oscillating_count: 0,
                oscillating_pairs: [0, 0, 0, 0, 0, 0]
            };

            assert_tokens(
                &configuration,
                &[
                    Token::Struct { name: "SminiConfiguration", len: 3 },
                        Token::BorrowedStr("transmit_delay"),
                        Token::U16(0),
                        Token::BorrowedStr("oscillating_count"),
                        Token::U8(0),
                        Token::BorrowedStr("oscillating_pairs"),
                        Token::BorrowedBytes(&[0; 6]),
                    Token::StructEnd
                ]
            );
        }

        #[test]
        fn when_oscillating_pairs_presented_as_sequence() {
            let configuration = Configuration {
                transmit_delay: 0,
                oscillating_count: 0,
                oscillating_pairs: [0, 0, 0, 0, 0, 0]
            };

            assert_de_tokens(
                &configuration,
                &[
                    Token::Struct { name: "SminiConfiguration", len: 3 },
                        Token::BorrowedStr("transmit_delay"),
                        Token::U16(0),
                        Token::BorrowedStr("oscillating_count"),
                        Token::U8(0),
                        Token::BorrowedStr("oscillating_pairs"),
                        Token::Seq { len: None },
                            Token::U8(0),
                            Token::U8(0),
                            Token::U8(0),
                            Token::U8(0),
                            Token::U8(0),
                            Token::U8(0),
                        Token::SeqEnd,
                    Token::StructEnd
                ]
            );
        }

        mod invalid_oscillating_pairs {
            use super::*;
            use serde_test::assert_de_tokens_error;

            #[test]
            fn odd_number_of_bits() {
                assert_de_tokens_error::<Configuration>(
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0b0000_0001, 0, 0, 0, 0, 0]),
                        Token::StructEnd
                    ],
                    "At least one pair of set bits in oscillating pairs aren't adjacent."
                );
            }

            #[test]
            fn pair_not_adjacent() {
                assert_de_tokens_error::<Configuration>(
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0b0000_0101, 0, 0, 0, 0, 0]),
                        Token::StructEnd
                    ],
                    "At least one pair of set bits in oscillating pairs aren't adjacent."
                );
            }

            #[test]
            fn too_many_bytes() {
                assert_de_tokens_error::<Configuration>(
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0; 7]),
                        Token::StructEnd
                    ],
                    "invalid length 7, expected 6 bytes"
                );
            }

            #[test]
            fn too_few_bytes() {
                assert_de_tokens_error::<Configuration>(
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0; 5]),
                        Token::StructEnd
                    ],
                    "invalid length 5, expected 6 bytes"
                );
            }
        }

        #[test]
        fn ignored_fields_for_deserialize() {
            // Value oscillating_count calculated form oscillating_pairs.
            let configuration = Configuration::try_new(
                0,
                [0b0000_0011, 0, 0, 0, 0, 0]
            ).unwrap();

            assert_de_tokens(
                &configuration,
                &[
                    Token::Struct { name: "SminiConfiguration", len: 3 },
                        Token::BorrowedStr("transmit_delay"),
                        Token::U16(0),
                        Token::BorrowedStr("oscillating_count"),
                        Token::U8(0),
                        Token::BorrowedStr("oscillating_pairs"),
                        Token::BorrowedBytes(&[0b0000_0011, 0, 0, 0, 0, 0]),
                    Token::StructEnd
                ]
            );
        }

        mod optional_fields_for_deserialize {
            use super::*;

            #[test]
            fn transmit_delay() { // Defaults to 0
                let configuration = Configuration::try_new(
                    0,
                    [0b0000_0011, 0, 0, 0, 0, 0]
                ).unwrap();

                assert_de_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("oscillating_pairs"),
                            Token::BorrowedBytes(&[0b0000_0011, 0, 0, 0, 0, 0]),
                        Token::StructEnd
                    ]
                );
            }

            #[test]
            fn oscillating_pairs() { // Defaults to none
                let configuration = Configuration::try_new(
                    1,
                    [0; 6]
                ).unwrap();

                assert_de_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "SminiConfiguration", len: 3 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(1),
                        Token::StructEnd
                    ]
                );
            }
        }
    }
}
