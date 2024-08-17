use crate::packet::Error;

/// The address of a packet/node.
///
/// Allows easy conversion between node address (human facing) and unit address (the byte on the wire).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address {
    value: u8
}

impl Address {
    /// Create an `Address` from a node address (human facing).
    ///
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNodeAddress`] if address is not between 0 and 127 (inclusive).
    pub const fn try_from_node_address(address: u8) -> Result<Self, Error> {
        if address <= 127 {
            Ok(Self { value: address })
        } else {
            Err(Error::InvalidNodeAddress(address))
        }
    }

    /// Create an `Address` from a unit address (the byte on the wire).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidUnitAddress`] if address is not between 65 and 192 (inclusive).
    pub const fn try_from_unit_address(address: u8) -> Result<Self, Error> {
        if address >= 65 && address <= 192 {
            Ok(Self { value: address - 65 })
        } else {
            Err(Error::InvalidUnitAddress(address))
        }
    }

    /// Get the address in "human facing" form.
    #[must_use]
    #[inline]
    pub const fn as_node_address(&self) -> u8 {
        self.value
    }

    /// Get the address in "on the wire" form.
    #[must_use]
    #[inline]
    pub const fn as_unit_address(&self) -> u8 {
        self.value + 65
    }
}

impl core::fmt::Debug for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.value))
    }
}

impl core::fmt::Display for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.value))
    }
}


#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl serde::Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.value)
    }
}
#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl<'de> serde::Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Address;
            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "between 0 and 127 (inclusive)")
            }
            fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> where E: serde::de::Error {
                Address::try_from_node_address(value).map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value.into()), &Self)
                })
            }
            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> where E: serde::de::Error, {
                u8::try_from(value).map_or(
                    Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value.into()), &Self)),
                    |value| self.visit_u8(value)
                )
            }
            fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> where E: serde::de::Error, {
                u8::try_from(value).map_or(
                    Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value.into()), &Self)),
                    |value| self.visit_u8(value)
                )
            }
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> where E: serde::de::Error, {
                u8::try_from(value).map_or(
                    Err(serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value), &Self)),
                    |value| self.visit_u8(value)
                )
            }
        }

        deserializer.deserialize_u8(Visitor)
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use super::*;

    mod try_from_node_address {
        use super::*;

        #[test]
        fn valid() {
            assert_eq!(Address::try_from_node_address(0), Ok(Address { value: 0 }));
            assert_eq!(Address::try_from_node_address(127), Ok(Address { value: 127 }));
        }

        #[test]
        fn invalid() {
            assert_eq!(Address::try_from_node_address(128), Err(Error::InvalidNodeAddress(128)));
            assert_eq!(Address::try_from_node_address(255), Err(Error::InvalidNodeAddress(255)));
        }
    }

    mod try_from_unit_address {
        use super::*;

        #[test]
        fn valid() {
            assert_eq!(Address::try_from_unit_address(65), Ok(Address { value: 0 }));
            assert_eq!(Address::try_from_unit_address(192), Ok(Address { value: 127 }));
        }

        #[test]
        fn invalid() {
            assert_eq!(Address::try_from_unit_address(0), Err(Error::InvalidUnitAddress(0)));
            assert_eq!(Address::try_from_unit_address(64), Err(Error::InvalidUnitAddress(64)));
            // 65 - 192 (inclusive) are valid
            assert_eq!(Address::try_from_unit_address(193), Err(Error::InvalidUnitAddress(193)));
            assert_eq!(Address::try_from_unit_address(255), Err(Error::InvalidUnitAddress(255)));
        }
    }

    #[test]
    fn as_node_address() {
        assert_eq!(Address { value: 10 }.as_node_address(), 10);
    }

    #[test]
    fn as_unit_address() {
        assert_eq!(Address { value: 10 }.as_unit_address(), 75);
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, assert_de_tokens_error, Token};

        #[test]
        fn valid() {
            assert_tokens(
                &Address { value: 1 },
                &[
                    Token::U8(1)
                ]
            );
        }

        #[test]
        fn too_high() {
            assert_de_tokens_error::<Address>(
                &[
                    Token::U8(128)
                ],
                "invalid value: integer `128`, expected between 0 and 127 (inclusive)"
            );
        }

        #[test]
        fn too_low() {
            assert_de_tokens_error::<Address>(
                &[
                    Token::I8(-1)
                ],
                "invalid type: integer `-1`, expected between 0 and 127 (inclusive)"
            );
        }
    }
}
