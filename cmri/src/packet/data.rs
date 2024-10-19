use core::fmt::Write;
use crate::packet::Raw as RawPacket;

/// Used to hold the data of a packet.
///
/// For example:
///  * The output states in a Transmit Data instruction
///  * The input states in a Receive Data repsonse
///  * The raw contents of an Initialization instruction
#[derive(Clone, Copy, Eq)]
pub struct Data {
    raw: [u8; Self::MAX_LEN],
    len: usize
}

crate::raw_structs::common_implementation!(Data, 256);

impl Data {
    /// Create a zeroed `Data` of an initial length.
    ///
    /// Useful for creating user controllable inputs/outputs once an initialization packet is received.
    ///
    /// # Panics
    ///
    /// If passed length is greater than 256.
    ///
    /// # Example
    ///
    /// ```
    /// use cmri::packet::Data;
    /// assert_eq!(Data::new(4).as_slice(), [0, 0, 0, 0].as_slice())
    /// ```
    #[must_use]
    pub fn new(len: usize) -> Self {
        assert!(len <= Self::MAX_LEN, "Length must be ≤ {}", Self::MAX_LEN);
        Self {
            raw: [0; Self::MAX_LEN],
            len
        }
    }

    /// Tests if a given bit in the data is true or false.
    ///
    /// # Panics
    ///
    /// If the index requires reading a byte which doesn't exist.
    #[must_use]
    pub fn get_bit(&self, index: usize) -> bool {
        let indexes = (index / 8, index % 8);
        assert!(indexes.0 < self.len, "index out of bounds: the len is {} but the index is {}", self.len * 8, index);
        self[indexes.0] & (1 << indexes.1) > 0
    }

    /// Sets a given bit in the data to true or false.
    /// So that it can be used to build up the contents one bit at a time,
    /// bytes will be added as needed (upto 256).
    ///
    /// # Panics
    ///
    /// If the index requires writing a byte outside the 256 byte limit.
    pub fn set_bit(&mut self, index: usize, value: bool) {
        assert!(index <= 2047, "index out of bounds: the len is 2048 but the index is {index}");
        let indexes = (index / 8, index % 8);
        if indexes.0 >= self.len {
            self.len = indexes.0 + 1;
        }
        if value {
            self[indexes.0] |= 1 << indexes.1;      // Set the bit
        } else {
            self[indexes.0] &= !(1 << indexes.1);   // Clear the bit
        }
    }

    /// Toggle a given bit in the data between true and false.
    ///
    /// # Panics
    ///
    /// If the index requires accessing a byte which doesn't exist.
    pub fn toggle_bit(&mut self, index: usize) {
        let indexes = (index / 8, index % 8);
        assert!(indexes.0 < self.len, "index out of bounds: the len is {} but the index is {}", self.len * 8, index);
        let mask = 1 << indexes.1;
        self[indexes.0] ^= mask;
    }

    /// Add a new byte to the end.
    ///
    /// # Errors
    ///
    /// If the Data is full, returning the value which couldn't be pushed.
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
    /// If the Data is full, returning the number of bytes available.
    pub fn push_all(&mut self, items: &[u8]) -> Result<(), usize> {
        if self.available() >= items.len() {
            self.raw[self.len..(self.len + items.len())].clone_from_slice(items);
            self.len += items.len();
            Ok(())
        } else {
            Err(self.available())
        }
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl serde::Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: ::serde::Serializer {
        serializer.serialize_bytes(self)
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl<'de> serde::Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: ::serde::Deserializer<'de> {
        struct Visitor;
        impl ::serde::de::Visitor<'_> for Visitor {
            type Value = Data;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "Upto {} bytes", Self::Value::MAX_LEN)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error {
                if v.len() > Self::Value::MAX_LEN {
                    return Err(serde::de::Error::invalid_length(v.len(), &self));
                }

                Self::Value::try_from(v).map_err(|err|
                    serde::de::Error::custom(err)
                )
            }
        }

        deserializer.deserialize_bytes(Visitor)
    }
}


impl core::default::Default for Data {
    fn default() -> Self {
        Self::new(0)
    }
}

impl core::fmt::Debug for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Data")
         .field("len", &self.len)
         .field("raw", &self.as_slice())
         .finish()
    }
}

impl core::fmt::LowerHex for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_char('[')?;
        for (i, &v) in self.as_slice().iter().enumerate() {
            if i > 0 { f.write_str(", ")?; }
            f.write_fmt(format_args!("{v:#04x}"))?;
        }
        f.write_char(']')
    }
}

impl core::fmt::UpperHex for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_char('[')?;
        for (i, &v) in self.as_slice().iter().enumerate() {
            if i > 0 { f.write_str(", ")?; }
            f.write_fmt(format_args!("{v:#04X}"))?;
        }
        f.write_char(']')
    }
}

impl TryFrom<&[u8]> for Data {
    type Error = super::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let len = value.len();
        if len > Self::MAX_LEN { return Err(super::Error::BodyTooLong) }

        let mut raw = [0; Self::MAX_LEN];
        raw[0..len].clone_from_slice(value);

        Ok(Self { raw, len })
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for Data {
    type Error = super::Error;

    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

impl<const N: usize> TryFrom<[u8; N]> for Data {
    type Error = super::Error;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        TryFrom::try_from(&value[..])
    }
}

impl From<&RawPacket> for Data {
    fn from(value: &RawPacket) -> Self {
        value.body().try_into().expect("RawPacket.body() can never be invalid.")
    }
}

impl From<RawPacket> for Data {
    fn from(value: RawPacket) -> Self {
        value.body().try_into().expect("RawPacket.body() can never be invalid.")
    }
}


#[allow(unused_must_use, clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use super::super::Error;
    use super::*;

    mod new {
        use super::*;

        #[test]
        fn good_size() {
            assert_eq!(Data::new(0).as_slice(), [].as_slice());
            assert_eq!(Data::new(5).as_slice(), [0; 5].as_slice());
            assert_eq!(Data::new(256).as_slice(), [0; 256].as_slice());
        }

        #[test]
        #[should_panic = "Length must be ≤ 256"]
        fn bad_size() {
            Data::new(257);
        }
    }

    mod get_bit {
        use super::*;

        #[test]
        #[expect(clippy::bool_assert_comparison, reason = "asier to follow along.")]
        fn valid_index() {
            let packet_data = Data::try_from(
                //  0000_0000    1111_1100
                //  7654_3210    5432_1098
                //  ffff_Tfff    Tfff_fffT
                &[0b0000_1000, 0b1000_0001]
            ).unwrap();
            assert_eq!(packet_data.get_bit(0), false);
            assert_eq!(packet_data.get_bit(3), true);
            assert_eq!(packet_data.get_bit(8), true);
            assert_eq!(packet_data.get_bit(15), true);
        }

        #[test]
        #[should_panic(expected = "index out of bounds: the len is 8 but the index is 8")]
        fn invalid_index() {
            let packet_data = Data::try_from(&[0]).unwrap();
            packet_data.get_bit(8);
        }
    }

    mod set_bit {
        use super::*;

        #[test]
        fn set_bit() {
            let mut packet_data = Data::try_from(&[0b0000_1111]).unwrap();
            assert_eq!(&packet_data, &[15]);

            packet_data.set_bit(4, true);
            assert_eq!(&packet_data, &[31]);

            packet_data.set_bit(4, false);
            assert_eq!(&packet_data, &[15]);
        }

        #[test]
        fn creates_more_space() {
            let mut packet_data = Data::try_from(&[0]).unwrap();

            // Byte 1, bit 2
            packet_data.set_bit(9, true);
            assert_eq!(*packet_data, [0, 2]);

            // Byte 3, bit 1
            packet_data.set_bit(24, true);
            assert_eq!(*packet_data, [0, 2, 0, 1]);

            // Byte 4, bit 7
            packet_data.set_bit(39, true);
            assert_eq!(*packet_data, [0, 2, 0, 1, 128]);

            // Last available bit
            packet_data.set_bit(2047, true);
            assert_eq!(packet_data[255], 0b1000_0000);
        }

        #[test]
        #[should_panic(expected = "index out of bounds: the len is 2048 but the index is 2048")]
        fn invalid_index() {
            let mut packet_data = Data::try_from(&[0]).unwrap();
            packet_data.set_bit(2048, true);
        }
    }

    mod toggle_bit {
        use super::*;

        #[test]
        fn toggle_bit() {
            let mut packet_data = Data::try_from(&[0b0000_1111]).unwrap();
            assert_eq!(&packet_data, &[15]);

            packet_data.toggle_bit(4);
            assert_eq!(&packet_data, &[31]);

            packet_data.toggle_bit(4);
            assert_eq!(&packet_data, &[15]);
        }

        #[test]
        #[should_panic(expected = "index out of bounds: the len is 8 but the index is 8")]
        fn invalid_index() {
            let mut packet_data = Data::try_from(&[0]).unwrap();
            packet_data.toggle_bit(8);
        }
    }

    mod push {
        use super::*;

        #[test]
        fn has_space() {
            let mut packet_data = Data::default();
            assert_eq!(packet_data.push(1), Ok(1));
            packet_data.push(2).unwrap();
            packet_data.push(3).unwrap();
            assert_eq!(packet_data.as_slice(), &[1, 2, 3]);
        }

        #[test]
        fn full() {
            let mut packet_data = Data::try_from(&[0; Data::MAX_LEN - 1]).unwrap();
            assert_eq!(packet_data.push(1), Ok(1));
            assert_eq!(packet_data.push(2), Err(2));
            assert_eq!(packet_data[Data::MAX_LEN - 1], 1);
        }
    }

    mod push_all {
        use super::*;

        #[test]
        fn works() {
            let mut packet_data = Data::default();
            assert_eq!(packet_data.push_all(&[1, 2]), Ok(()));
            packet_data.push_all(&[3, 4, 5]).unwrap();
            assert_eq!(packet_data.as_slice(), &[1, 2, 3, 4, 5]);
        }

        #[test]
        fn full() {
            let mut packet_data = Data::try_from(&[0; Data::MAX_LEN - 2]).unwrap();
            assert_eq!(packet_data.push_all(&[1, 2]), Ok(()));
            assert_eq!(packet_data.push_all(&[3, 4]), Err(0));
            assert_eq!(packet_data[(Data::MAX_LEN - 2)..], [1, 2]);
        }

        #[test]
        fn too_full() { // Pushing 2 bytes when there's only space for 1
            let mut packet_data = Data::try_from(&[0; Data::MAX_LEN - 3]).unwrap();
            assert_eq!(packet_data.push_all(&[1, 2]), Ok(()));
            assert_eq!(packet_data.push_all(&[3, 4]), Err(1));
            assert_eq!(packet_data[(packet_data.len() - 2)..], [1, 2]);
        }
    }

    mod try_from_slice_u8 {
        use super::*;

        #[test]
        fn works() {
            let bytes = [1, 255];
            assert_eq!(Data::try_from(&bytes).unwrap().as_slice(), &bytes);
            assert_eq!(Data::try_from(&bytes[0..2]).unwrap().as_slice(), &bytes);
            assert_eq!(Data::try_from(bytes).unwrap().as_slice(), &bytes);
        }

        #[test]
        fn too_long() {
            let slice = [0; Data::MAX_LEN + 1];
            let result: Result<Data, Error> = (&slice).try_into();
            assert_eq!(result, Err(Error::BodyTooLong));

            let result: Result<Data, Error> = (&slice[0..257]).try_into();
            assert_eq!(result, Err(Error::BodyTooLong));

            let result: Result<Data, Error> = slice.try_into();
            assert_eq!(result, Err(Error::BodyTooLong));
        }
    }

    #[test]
    fn from_raw_packet() {
        let raw_packet = RawPacket::try_from(&[65, b'T', 1, 2, 3, 4, 5]).unwrap();
        assert_eq!(*Data::from(&raw_packet), [1, 2, 3, 4, 5]);
        assert_eq!(*Data::from(raw_packet), [1, 2, 3, 4, 5]);
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, assert_de_tokens_error, Token};

        #[test]
        fn valid() {
            let data = &[1, 2];
            assert_tokens(
                &Data::try_from(data).unwrap(),
                &[
                    Token::Bytes(data),
                ]
            );
        }

        #[test]
        fn too_long() {
            assert_de_tokens_error::<Data>(
                &[
                    Token::Bytes(&[0_u8; Data::MAX_LEN + 1])
                ],
                "invalid length 257, expected Upto 256 bytes"
            );
        }
    }

    #[test]
    fn default() {
        let packet_data = Data::default();
        assert_eq!(packet_data.len, 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn lower_hex() {
        assert_eq!(
            format!("{:x}", &Data::try_from(&[4, 15, 32]).unwrap()),
            "[0x04, 0x0f, 0x20]"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn upper_hex() {
        assert_eq!(
            format!("{:X}", &Data::try_from(&[4, 15, 32]).unwrap()),
            "[0x04, 0x0F, 0x20]"
        );
    }
}
