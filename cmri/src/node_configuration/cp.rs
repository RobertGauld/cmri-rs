//! Details for CPNODEs and CPMEGAs.

/// Errors which can happen when decoding/creating a `CpnodeConfiguration` or `CpmegaConfiguration`.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// Invalid number of input/output bits for a CPNODE or CPMEGA
    #[error("Invalid input/output bits count: {0:?} not within {1:?}")]
    InvalidInputOutputBitsCount(u16, core::ops::RangeInclusive<u8>)
}
impl From<Error> for crate::packet::Error {
    fn from(source: Error) -> Self {
        let source = crate::node_configuration::InvalidConfigurationError::Cp { source };
        Self::InvalidConfiguration { source }
    }
}

macro_rules! common_implementation {
    ($name:ident, $ndp:expr, $bits:expr) => {
        paste::paste! {
            common_implementation!($name, $ndp, $bits, [<$name:camel Configuration>], [<$name:upper>]);
        }
    };
    ($name:ident, $ndp:expr, $bits:expr, $serde_name:ident, $human_name:ident) => {
        paste::paste! {
            common_implementation!($name, $ndp, $bits, stringify!($serde_name), stringify!($human_name));
        }
    };
    ($name:ident, $ndp:expr, $bits:expr, $serde_name:expr, $human_name:expr) => {
        mod $name {
            use log::trace;
            use crate::node_configuration::NodeConfiguration;
            use crate::packet::{Data as PacketData, Error as PacketError};
            use super::Error as Error;
            #[allow(unused_imports)]
            use super::super::{NDP_CPNODE, NDP_CPMEGA};

            bitflags::bitflags! {
                #[doc = concat!("Configuration options for a ", $human_name, " node.")]
                #[doc = concat!("")]
                #[doc = concat!("Poorly documented as I've not been able to find much about them.")]
                #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
                #[repr(transparent)]
                pub struct Options: u16 {
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const USE_CMRIX = 1;
                    /// The node can send back an empty `ReceiveData` packet if no inputs changed.
                    const CAN_SEND_EOT_ON_NO_INPUTS_CHANGED = 2;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const USE_BCC = 4;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_3 = 8;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_4 = 16;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_5 = 32;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_6 = 64;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_7 = 128;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_8 = 256;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_9 = 512;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_10 = 1024;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_11 = 2048;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_12 = 4096;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_13 = 8192;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_14 = 16384;
                    #[allow(missing_docs, reason = "Can't find documentation for options word.")]
                    const BIT_15 = 32768;
                }
            }

            #[doc = concat!("Configuration for a ", $human_name, " node.")]
            #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
            pub struct Configuration {
                pub(in super::super) transmit_delay: u16,
                pub(in super::super) options: Options,
                pub(in super::super) input_bytes: u8,
                pub(in super::super) output_bytes: u8
            }

            impl Configuration {
                #[doc = concat!("Create a new `Configuration`.")]
                #[doc = ""]
                #[doc = "# Errors"]
                #[doc = ""]
                #[doc = concat!("[`Error::InvalidInputOutputBitsCount`] if the total number of input and output bits is invalid for a `Configuration` (", stringify!(bits), ").")]
                pub fn try_new(transmit_delay: u16, options: Options, input_bytes: u8, output_bytes: u8) -> Result<Self, Error> {
                    // Check input_bytes + output_bytes is not too large
                    let bits: u16 = (u16::from(input_bytes) + u16::from(output_bytes)) * 8;
                    if !$bits.contains(&bits) {
                        return Err(Error::InvalidInputOutputBitsCount(bits, $bits))
                    }

                    Ok(Self {
                        transmit_delay,
                        options,
                        input_bytes,
                        output_bytes
                    })
                }

                #[doc = concat!("Create a new `Configuration` from raw bytes.")]
                #[doc = ""]
                #[doc = "# Errors"]
                #[doc = ""]
                #[doc = concat!("* [`PacketError::InvalidNodeType`] if the NDP byte isn't valid for a ", stringify!($name), " node.")]
                #[doc = "* [`PacketError::InvalidConfiguration`]:"]
                #[doc = concat!("  * [`Error::InvalidInputOutputBitsCount`] if the total number of input and output bits is invalid for a `", stringify!($name), "` (", stringify!($bits), ").")]
                pub(in super::super) fn decode(raw: &[u8]) -> Result<Self, PacketError> {
                    trace!(concat!(stringify!($name), "::decode({:?})"), raw);
                    if raw[0] != $ndp {
                        return Err(PacketError::InvalidNodeType(raw[0]))
                    }

                    Ok(Self::try_new(
                        u16::from_be_bytes([raw[1], raw[2]]),
                        Options::from_bits_retain(u16::from_le_bytes([raw[3], raw[4]])),
                        raw[5],
                        raw[6]
                    )?)
                }

                pub(in super::super) fn encode(&self) -> PacketData {
                    trace!(concat!(stringify!($name), ".encode({:?})"), self);
                    let transmit_delay = self.transmit_delay.to_be_bytes();
                    let options = self.options.bits().to_le_bytes();

                    let mut raw = PacketData::default();
                    raw.push($ndp).expect("Always pushes less than the maximum.");
                    raw.push(transmit_delay[0]).expect("Always pushes less than the maximum.");
                    raw.push(transmit_delay[1]).expect("Always pushes less than the maximum.");
                    raw.push(options[0]).expect("Always pushes less than the maximum.");
                    raw.push(options[1]).expect("Always pushes less than the maximum.");
                    raw.push(self.input_bytes).expect("Always pushes less than the maximum.");
                    raw.push(self.output_bytes).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");
                    raw.push(0xFF).expect("Always pushes less than the maximum.");

                    raw
                }

                /// Get the options as a number.
                #[must_use]
                pub const fn options(&self) -> Options {
                    self.options
                }
            }

            impl NodeConfiguration for Configuration {
                fn transmit_delay(&self) -> u16 { self.transmit_delay }
                fn input_bytes(&self) -> u16 { u16::from(self.input_bytes) }
                fn output_bytes(&self) -> u16 { u16::from(self.output_bytes) }
            }

            #[cfg(feature = "serde")]
            #[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
            #[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
            impl ::serde::Serialize for Configuration {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                    use serde::ser::SerializeStruct;
                    let mut ser = serializer.serialize_struct($serde_name, 4)?;
                    ser.serialize_field("transmit_delay", &self.transmit_delay)?;
                    ser.serialize_field("options", &self.options.bits())?;
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
                            let mut options = None;
                            let mut input_bytes = None;
                            let mut output_bytes = None;

                            while let Some(key) = map.next_key()? {
                                match key {
                                    "transmit_delay" => {
                                        if transmit_delay.is_some() {
                                            return Err(serde::de::Error::duplicate_field("transmit_delay"));
                                        }
                                        transmit_delay = Some(map.next_value()?);
                                    },
                                    "options" => {
                                        if options.is_some() {
                                            return Err(serde::de::Error::duplicate_field("options"));
                                        }
                                        options = Some(Options::from_bits_retain(map.next_value()?));
                                    },
                                    "input_bytes" => {
                                        if input_bytes.is_some() {
                                            return Err(serde::de::Error::duplicate_field("input_bytes"));
                                        }
                                        input_bytes = Some(map.next_value()?);
                                    },
                                    "output_bytes" => {
                                        if output_bytes.is_some() {
                                            return Err(serde::de::Error::duplicate_field("output_bytes"));
                                        }
                                        output_bytes = Some(map.next_value()?);
                                    },
                                    _ => {
                                        return Err(serde::de::Error::unknown_field(key, &["transmit_delay, options, input_bytes, output_bytes"]));
                                    }
                                }
                            }

                            let transmit_delay = transmit_delay.unwrap_or_default();
                            let options = options.unwrap_or_default();
                            let input_bytes = input_bytes.ok_or_else(|| serde::de::Error::missing_field("input_bytes"))?;
                            let output_bytes = output_bytes.ok_or_else(|| serde::de::Error::missing_field("output_bytes"))?;
                            Configuration::try_new(transmit_delay, options, input_bytes, output_bytes).map_err(serde::de::Error::custom)
                        }
                    }

                    deserializer.deserialize_struct($serde_name, &["transmit_delay", "options", "input_bytes", "output_bytes"], Visitor)
                }
            }
        }
    }
}

common_implementation!(cpnode, NDP_CPNODE, 16..=144);
pub use cpnode::Configuration as CpnodeConfiguration;
pub use cpnode::Options as CpnodeOptions;
common_implementation!(cpmega, NDP_CPMEGA, 0..=192);
pub use cpmega::Configuration as CpmegaConfiguration;
pub use cpmega::Options as CpmegaOptions;


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use crate::{node_configuration::NodeConfiguration, packet::Error as PacketError};
    use super::{CpnodeConfiguration, CpnodeOptions, CpmegaConfiguration, CpmegaOptions, Error};

    mod cpnode {
        use super::*;

        mod try_new {
            use super::*;

            #[test]
            fn creates() {
                assert_eq!(
                    CpnodeConfiguration::try_new(3, CpnodeOptions::from_bits_retain(4), 5, 6),
                    Ok(
                        CpnodeConfiguration {
                            transmit_delay: 3,
                            options: CpnodeOptions::from_bits_retain(4),
                            input_bytes: 5,
                            output_bytes: 6
                        }
                    )
                );
            }

            #[test]
            fn too_few_bytes() {
                assert_eq!(
                    CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 0, 1),
                    Err(Error::InvalidInputOutputBitsCount(8, 16..=144))
                );

                assert_eq!(
                    CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 1, 0),
                    Err(Error::InvalidInputOutputBitsCount(8, 16..=144))
                );
            }

            #[test]
            fn too_many_bytes() {
                assert_eq!(
                    CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 9, 10),
                    Err(Error::InvalidInputOutputBitsCount(152, 16..=144))
                );

                assert_eq!(
                    CpnodeConfiguration::try_new(0, CpnodeOptions::default(), 12, 13),
                    Err(Error::InvalidInputOutputBitsCount(200, 16..=144))
                );
            }
        }

        #[test]
        fn encode() {
            let configuration = CpnodeConfiguration {
                transmit_delay: 4080,
                options: CpnodeOptions::from_bits_retain(255),
                input_bytes: 2,
                output_bytes: 3
            };
            assert_eq!(
                configuration.encode(),
                [b'C', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255]
            );
        }

        mod decode {
            use super::*;

            #[test]
            fn decodes() {
                let raw = [b'C', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255];
                let configuration = CpnodeConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Ok(
                        CpnodeConfiguration {
                            transmit_delay: 4080,
                            options: CpnodeOptions::from_bits_retain(255),
                            input_bytes: 2,
                            output_bytes: 3
                        }
                    )
                );
            }

            #[test]
            fn too_few_bytes() {
                let raw = [b'C', 15, 240, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255];
                let configuration = CpnodeConfiguration::decode(&raw);
                assert_eq!(configuration, Err(PacketError::InvalidConfiguration { source: Error::InvalidInputOutputBitsCount(0, 16..=144).into() }));
            }

            #[test]
            fn too_many_bytes() {
                let raw = [b'C', 15, 240, 255, 0, 10, 20, 255, 255, 255, 255, 255, 255];
                let configuration = CpnodeConfiguration::decode(&raw);
                assert_eq!(configuration, Err(PacketError::InvalidConfiguration { source: Error::InvalidInputOutputBitsCount(240, 16..=144).into() }));
            }

            #[test]
            fn invalid_sort() {
                let raw = [b'Z', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255];
                let configuration = CpnodeConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Err(PacketError::InvalidNodeType(90))
                );
            }
        }

        mod getters {
            use super::*;

            #[test]
            fn options() {
                assert_eq!(
                    CpnodeConfiguration {
                        transmit_delay: 0,
                        options: CpnodeOptions::from_bits_retain(128),
                        output_bytes: 0,
                        input_bytes: 0
                    }.options(),
                    CpnodeOptions::from_bits_retain(128)
                );
            }
        }

        #[test]
        fn node_configuration() {
            let configuration = CpnodeConfiguration::try_new(
                200,
                CpnodeOptions::default(),
                4,
                5
            ).unwrap();

            assert_eq!(configuration.transmit_delay(), 200);
            assert_eq!(configuration.input_bytes(), 4);
            assert_eq!(configuration.output_bytes(), 5);
        }

        #[cfg(feature = "serde")]
        mod serde {
            use super::*;
            use serde_test::{assert_tokens, assert_de_tokens_error, Token};

            #[test]
            fn valid() {
                let configuration = CpnodeConfiguration {
                    transmit_delay: 3,
                    options: CpnodeOptions::from_bits_retain(4),
                    input_bytes: 5,
                    output_bytes: 6
                };

                assert_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(3),
                            Token::BorrowedStr("options"),
                            Token::U16(4),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(5),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(6),
                        Token::StructEnd
                    ]
                );
            }

            #[test]
            fn too_few_bytes() {
                assert_de_tokens_error::<CpnodeConfiguration>(
                    &[
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd
                    ],
                    "Invalid input/output bits count: 0 not within 16..=144"
                );

                assert_de_tokens_error::<CpnodeConfiguration>(
                    &[
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(0),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(1),
                        Token::StructEnd
                    ],
                    "Invalid input/output bits count: 8 not within 16..=144"
                );

                assert_de_tokens_error::<CpnodeConfiguration>(
                    &[
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(1),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(0),
                        Token::StructEnd
                    ],
                    "Invalid input/output bits count: 8 not within 16..=144"
                );
            }

            #[test]
            fn too_many_bytes() {
                assert_de_tokens_error::<CpnodeConfiguration>(
                    &[
                        Token::Struct { name: "CpnodeConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(18),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(1),
                        Token::StructEnd
                    ],
                    "Invalid input/output bits count: 152 not within 16..=144"
                );
            }
            mod optional_fields_for_deserialize {
                use super::*;
                use serde_test::assert_de_tokens;

                #[test]
                fn transmit_delay() { // Defaults to 0
                    let configuration = CpnodeConfiguration::try_new(
                        0,
                        CpnodeOptions::from_bits_retain(1),
                        1,
                        1
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "CpnodeConfiguration", len: 3 },
                                Token::BorrowedStr("options"),
                                Token::U16(1),
                                Token::BorrowedStr("input_bytes"),
                                Token::U16(1),
                                Token::BorrowedStr("output_bytes"),
                                Token::U16(1),
                            Token::StructEnd
                        ]
                    );
                }

                #[test]
                fn options() { // Defaults to 0
                    let configuration = CpnodeConfiguration::try_new(
                        1,
                        CpnodeOptions::default(),
                        1,
                        1
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "CpnodeConfiguration", len: 3 },
                                Token::BorrowedStr("transmit_delay"),
                                Token::U16(1),
                                Token::BorrowedStr("input_bytes"),
                                Token::U16(1),
                                Token::BorrowedStr("output_bytes"),
                                Token::U16(1),
                            Token::StructEnd
                        ]
                    );
                }
            }
        }
    }

    mod cpmega {
        use super::*;

        mod try_new {
            use super::*;

            #[test]
            fn creates() {
                assert_eq!(
                    CpmegaConfiguration::try_new(3, CpmegaOptions::from_bits_retain(4), 5, 6),
                    Ok(
                        CpmegaConfiguration {
                            transmit_delay: 3,
                            options: CpmegaOptions::from_bits_retain(4),
                            input_bytes: 5,
                            output_bytes: 6
                        }
                    )
                );
            }

            #[test]
            fn too_many_bytes() {
                assert_eq!(
                    CpmegaConfiguration::try_new(0, CpmegaOptions::default(), 12, 13),
                    Err(Error::InvalidInputOutputBitsCount(200, 0..=192))
                );

                assert_eq!(
                    CpmegaConfiguration::try_new(0, CpmegaOptions::default(), 15, 15),
                    Err(Error::InvalidInputOutputBitsCount(240, 0..=192))
                );
            }
        }

        #[test]
        fn encode() {
            let configuration = CpmegaConfiguration {
                transmit_delay: 4080,
                options: CpmegaOptions::from_bits_retain(255),
                input_bytes: 2,
                output_bytes: 3
            };
            assert_eq!(
                configuration.encode(),
                [b'O', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255]
            );
        }

        mod decode {
            use super::*;

            #[test]
            fn decodes() {
                let raw = [b'O', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255];
                let configuration = CpmegaConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Ok(
                        CpmegaConfiguration {
                            transmit_delay: 4080,
                            options: CpmegaOptions::from_bits_retain(255),
                            input_bytes: 2,
                            output_bytes: 3
                        }
                    )
                );
            }

            #[test]
            fn too_many_bytes() {
                let raw = [b'O', 15, 240, 255, 0, 10, 20, 255, 255, 255, 255, 255, 255];
                let configuration = CpmegaConfiguration::decode(&raw);
                assert_eq!(configuration, Err(PacketError::InvalidConfiguration { source: Error::InvalidInputOutputBitsCount(240, 0..=192).into() }));
            }

            #[test]
            fn invalid_sort() {
                let raw = [b'Z', 15, 240, 255, 0, 2, 3, 255, 255, 255, 255, 255, 255];
                let configuration = CpmegaConfiguration::decode(&raw);
                assert_eq!(
                    configuration,
                    Err(PacketError::InvalidNodeType(90))
                );
            }
        }

        mod getters {
            use super::*;

            #[test]
            fn options() {
                assert_eq!(
                    CpmegaConfiguration {
                        transmit_delay: 0,
                        options: CpmegaOptions::from_bits_retain(128),
                        output_bytes: 0,
                        input_bytes: 0
                    }.options(),
                    CpmegaOptions::from_bits_retain(128)
                );
            }
        }

        #[test]
        fn node_configuration() {
            let configuration = CpmegaConfiguration::try_new(
                200,
                CpmegaOptions::default(),
                4,
                5
            ).unwrap();

            assert_eq!(configuration.transmit_delay(), 200);
            assert_eq!(configuration.input_bytes(), 4);
            assert_eq!(configuration.output_bytes(), 5);
        }

        #[cfg(feature = "serde")]
        mod serde {
            use super::*;
            use serde_test::{assert_tokens, assert_de_tokens_error, Token};

            #[test]
            fn valid() {
                let configuration = CpmegaConfiguration {
                    transmit_delay: 3,
                    options: CpmegaOptions::from_bits_retain(4),
                    input_bytes: 5,
                    output_bytes: 6
                };

                assert_tokens(
                    &configuration,
                    &[
                        Token::Struct { name: "CpmegaConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(3),
                            Token::BorrowedStr("options"),
                            Token::U16(4),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(5),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(6),
                        Token::StructEnd
                    ]
                );
            }

            #[test]
            fn too_many_bytes() {
                assert_de_tokens_error::<CpmegaConfiguration>(
                    &[
                        Token::Struct { name: "CpmegaConfiguration", len: 4 },
                            Token::BorrowedStr("transmit_delay"),
                            Token::U16(0),
                            Token::BorrowedStr("options"),
                            Token::U16(0),
                            Token::BorrowedStr("input_bytes"),
                            Token::U16(24),
                            Token::BorrowedStr("output_bytes"),
                            Token::U16(1),
                        Token::StructEnd
                    ],
                    "Invalid input/output bits count: 200 not within 0..=192"
                );
            }

            mod optional_fields_for_deserialize {
                use super::*;
                use serde_test::assert_de_tokens;

                #[test]
                fn transmit_delay() { // Defaults to 0
                    let configuration = CpmegaConfiguration::try_new(
                        0,
                        CpmegaOptions::from_bits_retain(1),
                        1,
                        1
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "CpmegaConfiguration", len: 3 },
                                Token::BorrowedStr("options"),
                                Token::U16(1),
                                Token::BorrowedStr("input_bytes"),
                                Token::U16(1),
                                Token::BorrowedStr("output_bytes"),
                                Token::U16(1),
                            Token::StructEnd
                        ]
                    );
                }

                #[test]
                fn options() { // Defaults to 0
                    let configuration = CpmegaConfiguration::try_new(
                        1,
                        CpmegaOptions::default(),
                        1,
                        1
                    ).unwrap();

                    assert_de_tokens(
                        &configuration,
                        &[
                            Token::Struct { name: "CpmegaConfiguration", len: 3 },
                                Token::BorrowedStr("transmit_delay"),
                                Token::U16(1),
                                Token::BorrowedStr("input_bytes"),
                                Token::U16(1),
                                Token::BorrowedStr("output_bytes"),
                                Token::U16(1),
                            Token::StructEnd
                        ]
                    );
                }
            }
        }
    }
}
