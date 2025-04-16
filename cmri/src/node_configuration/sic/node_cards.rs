//! Work with the I/O cards (both individual and as a collection) in a USIC/SUSIC.

/// Errors which can happen when decoding/creating a `CpnodeConfiguration` or `CpmegaConfiguration`.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// Too many cards (must be 64 or less) for a classic USIC or a SUSIC.
    #[error("Too many cards")]
    TooManyCards,

    /// An input or output card appears after the first none.
    #[error("An input or output card appears after a none card")]
    CardAfterNone,

    /// Invalid card type for a classic USIC or a SUSIC.
    /// Caused by the bits for a card being 11 (only 00, 01, 10 are defined).
    #[error("Invalid card type")]
    InvalidCardType
}
impl From<Error> for crate::packet::Error {
    fn from(source: Error) -> Self {
        let source = crate::node_configuration::InvalidConfigurationError::Sic { source };
        Self::InvalidConfiguration { source }
    }
}


/// Create and work with valid a collection of `NodeCard`s.
#[derive(Clone, Copy, Debug, Eq)]
pub struct NodeCards{
    cards: [NodeCard; 64],
    input_cards: u8,
    output_cards: u8
}

impl NodeCards {
    /// Create a new `NodeCards`, also counts the input and output cards.
    ///
    /// # Errors
    ///
    /// * [`Error::CardAfterNone`] if there's an Input or Output card after a None card.
    /// * [`Error::TooManyCards`] if there's more than 64 input/output cards.
    ///
    /// # Example
    /// ```
    /// use cmri::node_configuration::node_cards::{NodeCards, NodeCard};
    /// let cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
    /// assert_eq!(cards.input_cards(), 1);
    /// assert_eq!(cards.output_cards(), 2);
    /// ```
    #[expect(clippy::missing_panics_doc, reason = "Returns error if too many cards, precenting the panic.")]
    pub fn try_new(cards: &[NodeCard]) -> Result<Self, Error> {
        if cards.len() > 64 {
            return Err(Error::TooManyCards)
        }

        let mut node_cards = Self::default();
        let mut none_seen = false;
        for &card in cards {
            match card {
                NodeCard::None => {
                    none_seen = true;
                },
                NodeCard::Input | NodeCard::Output => {
                    if none_seen { return Err(Error::CardAfterNone) }
                    node_cards.try_push(card).expect("We've already checked there's not too many");
                }
            }
        }

        Ok(node_cards)
    }

    /// Push a new card into the collection.
    ///
    /// # Errors
    /// If the collection is already full.
    #[expect(clippy::result_unit_err)]
    pub fn try_push(&mut self, card: NodeCard) -> Result<(), ()> {
        let index = self.len();
        if index >= 64 { return Err(()); }

        match card {
            NodeCard::Input  => self.input_cards += 1,
            NodeCard::Output => self.output_cards += 1,
            NodeCard::None   => ()
        }
        self.cards[index] = card;
        Ok(())
    }

    /// The number of `NodeCard`s currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.input_cards + self.output_cards)
    }

    /// Check whether the `NodeCards` is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.input_cards == 0 && self.output_cards == 0
    }

    /// Extracts a slice of the cards.
    #[must_use]
    pub fn as_slice(&self) -> &[NodeCard] {
        &self.cards[..(self.len())]
    }

    /// The number of input cards in the collection.
    #[must_use]
    pub const fn input_cards(&self) -> u8 {
        self.input_cards
    }

    /// The number of output cards in the collection.
    #[must_use]
    pub const fn output_cards(&self) -> u8 {
        self.output_cards
    }
}

impl Default for NodeCards {
    fn default() -> Self {
        Self {
            cards: [NodeCard::None; 64],
            input_cards: 0,
            output_cards: 0
        }
    }
}

impl core::ops::Deref for NodeCards {
    type Target = [NodeCard];
    fn deref(&self) -> &Self::Target {
        &self.cards[0..(self.len())]
    }
}

impl core::ops::Index<usize> for NodeCards {
    type Output = NodeCard;
    fn index(&self, index: usize) -> &Self::Output {
        let len = self.len();
        assert!(index < len, "index out of bounds: the len is {len} but the index is {index}");
        &self.cards[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for NodeCards {
    type Output = [NodeCard];
    fn index(&self, index: core::ops::Range<usize>) -> &Self::Output {
        let len = self.len();
        assert!(index.start < len, "range start index {} out of range for data of length {}", index.start, len);
        assert!(index.end <= self.len(), "range end index {} out of range for data of length {}", index.end - 1, len);
        &self.cards[index]
    }
}

impl core::ops::Index<core::ops::RangeInclusive<usize>> for NodeCards {
    type Output = [NodeCard];
    fn index(&self, index: core::ops::RangeInclusive<usize>) -> &Self::Output {
        let len = self.len();
        assert!(index.start() < &len, "range start index {} out of range for data of length {}", index.start(), len);
        assert!(index.end() < &len, "range end index {} out of range for data of length {}", index.end(), len);
        &self.cards[index]
    }
}

impl core::ops::Index<core::ops::RangeFrom<usize>> for NodeCards {
    type Output = [NodeCard];
    fn index(&self, index: core::ops::RangeFrom<usize>) -> &Self::Output {
        let len = self.len();
        assert!(index.start < len, "range start index {} out of range for data of length {}", index.start, len);
        &self.cards[(index.start)..len]
    }
}

impl<'a> core::iter::IntoIterator for &'a NodeCards {
    type Item = &'a NodeCard;
    type IntoIter = core::slice::Iter<'a, NodeCard>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<const N: usize> core::cmp::PartialEq<[NodeCard; N]> for NodeCards {
    fn eq(&self, other: &[NodeCard; N]) -> bool {
        other.eq(&self.as_slice())
    }
}

impl core::cmp::PartialEq<&[NodeCard]> for NodeCards {
    fn eq(&self, other: &&[NodeCard]) -> bool {
        other.eq(&self.as_slice())
    }
}

impl core::cmp::PartialEq for NodeCards {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl core::hash::Hash for NodeCards {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.as_slice(), state);
    }
}

#[cfg(feature = "std")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "std")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature std only.**\n\n")]
impl From<&NodeCards> for Vec<NodeCard> {
    fn from(value: &NodeCards) -> Self {
        value.as_slice().into()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "std")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature std only.**\n\n")]
impl From<NodeCards> for Vec<NodeCard> {
    fn from(value: NodeCards) -> Self {
        value.as_slice().into()
    }
}



#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl serde::ser::Serialize for NodeCards {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeSeq;
        let mut ser = serializer.serialize_seq(None)?;
        for card in &self.cards {
            if *card == NodeCard::None { break }
            ser.serialize_element(card)?;
        }
        ser.end()
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(any(docsrs, toolchain = "nightly"), doc(cfg(feature = "serde")))]
#[cfg_attr(not(toolchain = "nightly"), doc = "**Available on crate feature serde only.**\n\n")]
impl<'de> serde::de::Deserialize<'de> for NodeCards {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = NodeCards;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "a list of upto 64 NodeCard")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de> {
                if let Some(size_hint) = seq.size_hint() {
                    if size_hint > 64 {
                        return Err(serde::de::Error::invalid_length(size_hint, &"no more than 64 cards"));
                    }
                }

                let mut value = NodeCards::default();
                let mut seen_none = false;
                while let Some(item) = seq.next_element()? {
                    if seen_none && item != NodeCard::None {
                        return Err(serde::de::Error::custom("expected no cards after the first none"));
                    }
                    if item == NodeCard::None { seen_none = true; }
                    if value.try_push(item).is_err() {
                        return Err(serde::de::Error::invalid_length(65, &"no more than 64 cards"));
                    }
                }

                Ok(value)
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}


/// The types of cards which can be inserted into a Classic USIC or SUSIC node.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeCard {
    /// The card slot is empty.
    None   = 0b00,
    /// The card slot contains an input card.
    Input  = 0b01,
    /// The card slot contains an output card.
    Output = 0b10
}

impl TryFrom<u8> for NodeCard {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(Self::None),
            0b01 => Ok(Self::Input),
            0b10 => Ok(Self::Output),
            _ => Err(())
        }
    }
}

impl From<NodeCard> for u8 {
    fn from(value: NodeCard) -> Self {
        match value {
            NodeCard::None   => 0b00,
            NodeCard::Input  => 0b01,
            NodeCard::Output => 0b10
        }
    }
}


#[allow(clippy::missing_panics_doc, reason = "tests")]
#[cfg(test)]
mod tests {
    use super::{NodeCards, NodeCard, Error};

    mod try_new {
        use super::*;

        #[test]
        fn valid() {
            let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
            assert_eq!(node_cards.input_cards(), 1);
            assert_eq!(node_cards.output_cards(), 2);
        }

        #[test]
        fn too_many_cards() {
            let cards = [NodeCard::Input; 65];
            assert_eq!(
                NodeCards::try_new(&cards),
                Err(Error::TooManyCards)
            );
        }

        #[test]
        fn card_after_none() {
            let cards = [NodeCard::None, NodeCard::Input];
            assert_eq!(
                NodeCards::try_new(&cards),
                Err(Error::CardAfterNone)
            );
        }
    }

    mod try_push {
        use super::*;

        #[test]
        fn success() {
            let mut node_cards = NodeCards::default();
            assert_eq!(node_cards.input_cards, 0);
            assert_eq!(node_cards.output_cards, 0);

            assert!(node_cards.try_push(NodeCard::Input).is_ok());
            assert_eq!(node_cards.input_cards, 1);
            assert_eq!(node_cards.output_cards, 0);

            assert!(node_cards.try_push(NodeCard::Output).is_ok());
            assert!(node_cards.try_push(NodeCard::Output).is_ok());
            assert_eq!(node_cards.input_cards, 1);
            assert_eq!(node_cards.output_cards, 2);

            assert_eq!(
                node_cards.as_slice(),
                [NodeCard::Input, NodeCard::Output, NodeCard::Output].as_slice()
            );
        }

        #[test]
        fn already_full() {
            let mut node_cards = NodeCards::try_new(&[NodeCard::Input; 64]).unwrap();
            assert_eq!(
                node_cards.try_push(NodeCard::Output),
                Err(())
            );
        }

        #[test]
        fn ignores_none() {
            let mut node_cards = NodeCards::default();
            node_cards.try_push(NodeCard::Output).unwrap();
            node_cards.try_push(NodeCard::None).unwrap();
            node_cards.try_push(NodeCard::Input).unwrap();
            assert_eq!(node_cards.len(), 2);
            assert_eq!(
                node_cards.as_slice(),
                [NodeCard::Output, NodeCard::Input].as_slice()
            );
        }
    }

    #[test]
    fn len(){
        assert_eq!(
            NodeCards::try_new(&[]).unwrap().len(),
            0
        );

        assert_eq!(
            NodeCards::try_new(&[NodeCard::Input, NodeCard::Output, NodeCard::None]).unwrap().len(),
            2
        );

        assert_eq!(
            NodeCards::try_new(&[NodeCard::Input; 32]).unwrap().len(),
            32
        );
    }

    #[test]
    fn is_empty(){
        assert!(NodeCards::try_new(&[]).unwrap().is_empty());
        assert!(!NodeCards::try_new(&[NodeCard::Input]).unwrap().is_empty());
    }

    #[test]
    fn as_slice(){
        let cards = [NodeCard::Input, NodeCard::Output, NodeCard::Output, NodeCard:: Input, NodeCard::Output];
        let node_cards = NodeCards::try_new(&cards).unwrap();
        assert_eq!(
            node_cards.as_slice(),
            cards.as_slice()
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            NodeCards::default(),
            NodeCards {
                cards: [NodeCard::None; 64],
                input_cards: 0,
                output_cards: 0
            }
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn iter() {
        let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
        let result: Vec<NodeCard> = node_cards.iter().copied().collect();
        assert_eq!(node_cards.as_slice(), result.as_slice());
    }

    #[test]
    fn deref() {
        let node_cards = NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap();
        assert_eq!(*node_cards, [NodeCard::Output, NodeCard::Input]);
    }

    mod partial_eq {
        use super::*;

        #[test]
        fn node_cards() {
            assert_eq!(
                NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap(),
                NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap()
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap(),
                NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap()
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input]).unwrap(),
                NodeCards::try_new(&[NodeCard::Input, NodeCard::Input]).unwrap()
            );
        }

        #[test]
        fn slice_node_card() {
            assert_eq!(
                NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap(),
                [NodeCard::Output, NodeCard::Input]
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap(),
                [NodeCard::Output, NodeCard::Input]
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input]).unwrap(),
                [NodeCard::Input, NodeCard::Input]
            );

            assert_eq!(
                NodeCards::try_new(&[NodeCard::Output, NodeCard::Input]).unwrap(),
                [NodeCard::Output, NodeCard::Input].as_slice()
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap(),
                [NodeCard::Output, NodeCard::Input].as_slice()
            );
            assert_ne!(
                NodeCards::try_new(&[NodeCard::Input]).unwrap(),
                [NodeCard::Input, NodeCard::Input].as_slice()
            );
        }
    }

    mod index {
        use super::*;

        mod by_usize {
            use super::*;

            #[test]
            fn valid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
                assert_eq!(node_cards[0], NodeCard::Input);
                assert_eq!(node_cards[1], NodeCard::Output);
            }

            #[test]
            #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
            fn invalid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = node_cards[2];
            }
        }

        mod by_range_usize {
            use super::*;

            #[test]
            fn valid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
                assert_eq!(node_cards[0..2], [NodeCard::Input, NodeCard::Input]);
                assert_eq!(node_cards[2..4], [NodeCard::Output, NodeCard::Output]);
            }

            #[test]
            #[should_panic(expected = "range end index 2 out of range for data of length 2")]
            fn ends_after_end() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = &node_cards[1..3];
            }

            #[test]
            #[should_panic(expected = "range start index 2 out of range for data of length 2")]
            fn starts_after_end() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = &node_cards[2..3];
            }
        }

        mod by_range_inclusive_usize {
            use super::*;

            #[test]
            fn valid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
                assert_eq!(node_cards[0..=2], [NodeCard::Input, NodeCard::Input, NodeCard::Output]);
                assert_eq!(node_cards[2..=3], [NodeCard::Output, NodeCard::Output]);
            }

            #[test]
            #[should_panic(expected = "range end index 2 out of range for data of length 2")]
            fn ends_after_end() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = &node_cards[1..=2];
            }

            #[test]
            #[should_panic(expected = "range start index 2 out of range for data of length 2")]
            fn starts_after_end() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = &node_cards[2..=3];
            }
        }

        mod by_range_from_usize {
            use super::*;

            #[test]
            fn valid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Input, NodeCard::Output, NodeCard::Output]).unwrap();
                assert_eq!(node_cards[0..], [NodeCard::Input, NodeCard::Input, NodeCard::Output, NodeCard::Output]);
                assert_eq!(node_cards[2..], [NodeCard::Output, NodeCard::Output]);
            }

            #[test]
            fn when_partially_full() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Input, NodeCard::Output]).unwrap();
                assert_eq!(node_cards[0..], [NodeCard::Input, NodeCard::Input, NodeCard::Output]);
            }

            #[test]
            #[should_panic(expected = "range start index 2 out of range for data of length 2")]
            fn invalid() {
                let node_cards = NodeCards::try_new(&[NodeCard::Input, NodeCard::Output]).unwrap();
                #[expect(unused_variables, reason="Trigger the panic")]
                let a = &node_cards[2..];
            }
        }
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_test::{assert_tokens, assert_de_tokens_error, Token};

        mod valid {
            use super::*;

            #[test]
            fn empty() {
                assert_tokens(
                    &NodeCards::default(),
                    &[
                        Token::Seq { len: None },
                        Token::SeqEnd
                    ]
                );
            }

            #[test]
            fn some() {
                assert_tokens(
                    &NodeCards::try_new(&[NodeCard::Input, NodeCard::Output, NodeCard::Input]).unwrap(),
                    &[
                        Token::Seq { len: None },
                            Token::UnitVariant { name: "NodeCard", variant: "Input" },
                            Token::UnitVariant { name: "NodeCard", variant: "Output" },
                            Token::UnitVariant { name: "NodeCard", variant: "Input" },
                        Token::SeqEnd,
                    ]
                );
            }
        }

        mod invalid {
            use super::*;

            #[test]
            fn bad_card() {
                assert_de_tokens_error::<NodeCards>(
                    &[
                        Token::Seq { len: None },
                            Token::UnitVariant { name: "NodeCard", variant: "Zzzz" },
                    ],
                    "unknown variant `Zzzz`, expected one of `None`, `Input`, `Output`"
                );
            }

            #[test]
            fn too_many_with_len() {
                assert_de_tokens_error::<NodeCards>(
                    &[
                        Token::Seq { len: Some(65) }
                    ],
                    "invalid length 65, expected no more than 64 cards"
                );
            }

            #[test]
            fn too_many_without_len() {
                let mut tokens = [Token::UnitVariant { name: "NodeCard", variant: "Input" }; 66];
                tokens[0] = Token::Seq { len: None };
                assert!(tokens.len() == 66, "tokens must be 1 for seq, then 65 cards.");
                assert_de_tokens_error::<NodeCards>(
                    &tokens,
                    "invalid length 65, expected no more than 64 cards"
                );
            }

            #[test]
            fn card_after_none() {
                assert_de_tokens_error::<NodeCards>(
                    &[
                        Token::Seq { len: None },
                            Token::UnitVariant { name: "NodeCard", variant: "None" },
                            Token::UnitVariant { name: "NodeCard", variant: "Input" },
                    ],
                    "expected no cards after the first none"
                );
            }
        }
    }

    mod node_card {
        use super::NodeCard;

        #[test]
        fn converts_to_u8() {
            let mut value: u8 = NodeCard::None.into();
            assert_eq!(value, 0b00);

            value = NodeCard::Input.into();
            assert_eq!(value, 0b01);

            value = NodeCard::Output.into();
            assert_eq!(value, 0b10);
        }

        #[test]
        fn converts_to_usize() {
            assert_eq!(NodeCard::None as usize, 0b00);
            assert_eq!(NodeCard::Input as usize, 0b01);
            assert_eq!(NodeCard::Output as usize, 0b10);
        }

        #[test]
        fn converts_from_usize() {
            let mut node_card = 0b00.try_into();
            assert_eq!(node_card, Ok(NodeCard::None));

            node_card = 0b01.try_into();
            assert_eq!(node_card, Ok(NodeCard::Input));

            node_card = 0b10.try_into();
            assert_eq!(node_card, Ok(NodeCard::Output));

            node_card = 0b11.try_into();
            assert_eq!(node_card, Err(()));
        }

        #[cfg(feature = "serde")]
        mod serde {
            use super::*;
            use serde_test::{assert_tokens, assert_de_tokens_error, Token};

            mod valid {
                use super::*;

                #[test]
                fn none() {
                    assert_tokens(
                        &NodeCard::None,
                        &[Token::UnitVariant { name: "NodeCard", variant: "None" }]
                    );
                }

                #[test]
                fn input() {
                    assert_tokens(
                        &NodeCard::Input,
                        &[Token::UnitVariant { name: "NodeCard", variant: "Input" }]
                    );
                }

                #[test]
                fn output() {
                    assert_tokens(
                        &NodeCard::Output,
                        &[Token::UnitVariant { name: "NodeCard", variant: "Output" }]
                    );
                }
            }

            #[test]
            fn invalid() {
                assert_de_tokens_error::<NodeCard>(
                    &[Token::UnitVariant { name: "NodeCard", variant: "Zzzz" }],
                    "unknown variant `Zzzz`, expected one of `None`, `Input`, `Output`"
                );
            }
        }
    }
}
