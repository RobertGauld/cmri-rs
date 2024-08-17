#![cfg_attr(not(feature = "std"), no_std)]

#![doc = include_str!("../README.md")]
#![doc(html_playground_url = "https://play.rust-lang.org/")]

// See: https://lib.rs/crates/thiserror-core
// See: https://doc.rust-lang.org/unstable-book/library-features/error-in-core.html
// #![feature(error_in_core)]

// See: https://doc.rust-lang.org/unstable-book/language-features/doc-cfg.html
#![cfg_attr(toolchain = "nightly", feature(doc_cfg))]

// See: https://doc.rust-lang.org/unstable-book/library-features/test.html
// Hopefully we'll soon be able to fail a benchmark if it's too slow.
#![cfg_attr(all(toolchain = "nightly", test), feature(test))]
#[cfg(all(toolchain = "nightly", test))]
extern crate test;

/// Valid speeds for serial communications.
pub const BAUDS: [u32; 5] = [9_600, 19_200, 28_800, 57_600, 115_200];
/// Default speed for serial communications.
pub const DEFAULT_BAUD: u32 = 19_200;

mod address;
mod raw_structs;

pub mod packet;
pub mod frame;
pub mod node_configuration;

pub use address::Address;
pub use packet::Packet;
pub use node_configuration::{NodeSort, NodeConfiguration};
pub use frame::Raw as Frame;

#[cfg(test)]
mod tests {
    use super::*;

    trait IsNormal {}
    impl<T> IsNormal for T where T: Sized + Send + Sync + Unpin {}
    trait IsComparable {}
    impl<T> IsComparable for T where T: Eq + PartialEq {}
    trait IsOrderable {}
    impl<T> IsOrderable for T where T: Ord + PartialOrd {}
    trait IsHashable {}
    impl<T> IsHashable for T where T: core::hash::Hash {}
    trait IsDecent {}
    impl<T> IsDecent for T where T: IsNormal + IsComparable + IsHashable + core::fmt::Debug + Copy + Clone {}

    trait IsDecentCollection {}
    // TODO: core::iter::IntoIterator, possible Into<Vec<_>> too.
    impl<T> IsDecentCollection for T where T: IsDecent {}

    const fn is_orderable<T: IsOrderable>() {}
    const fn is_decent<T: IsDecent>() {}
    const fn is_decent_collection<T: IsDecentCollection>() {}

    const fn is_decent_error<T: IsNormal + IsComparable + core::fmt::Debug + core::fmt::Display>() {}

    #[test]
    const fn structs_are_as_expected() {
        is_decent::<Address>();
        is_orderable::<Address>();

        is_decent_collection::<frame::Raw>();
        is_decent_error::<frame::DecodeError>();
        is_decent_error::<frame::ReceiveError>();
        is_decent_error::<frame::Full>();

        is_decent::<packet::Packet>();
        is_decent_collection::<packet::Raw>();
        is_decent_collection::<packet::Data>();
        is_decent::<packet::Payload>();
        is_decent_error::<packet::Error>();

        is_decent::<node_configuration::NodeSort>();
        is_decent::<node_configuration::UsicConfiguration>();
        is_decent::<node_configuration::SusicConfiguration>();
        is_decent::<node_configuration::SminiConfiguration>();
        is_decent::<node_configuration::CpnodeConfiguration>();
        is_decent::<node_configuration::CpmegaConfiguration>();
        is_decent_collection::<node_configuration::node_cards::NodeCards>();
        is_decent::<node_configuration::node_cards::NodeCard>();
        is_decent_error::<node_configuration::node_cards::Error>();
        is_decent_error::<node_configuration::SminiConfigurationError>();
        is_decent_error::<node_configuration::CpConfigurationError>();
        is_decent_error::<node_configuration::InvalidConfigurationError>();
    }

    #[test]
    #[cfg(feature = "serde")]
    const fn is_serde() {
        const fn test<'a, T>() where T: serde::ser::Serialize + serde::de::Deserialize<'a> {}

        test::<Address>();

        // Exclude crate::frame::Raw
        test::<frame::DecodeError>();
        test::<frame::ReceiveError>();
        test::<frame::Full>();

        test::<packet::Packet>();
        // Exclude crate::packet::Raw
        test::<packet::Data>();
        test::<packet::Payload>();
        test::<packet::Error>();

        test::<node_configuration::NodeSort>();
        test::<node_configuration::UsicConfiguration>();
        test::<node_configuration::SusicConfiguration>();
        test::<node_configuration::SminiConfiguration>();
        test::<node_configuration::CpnodeConfiguration>();
        test::<node_configuration::CpmegaConfiguration>();
        test::<node_configuration::node_cards::NodeCards>();
        test::<node_configuration::node_cards::NodeCard>();
        test::<node_configuration::node_cards::Error>();
        test::<node_configuration::SminiConfigurationError>();
        test::<node_configuration::CpConfigurationError>();
        test::<node_configuration::InvalidConfigurationError>();
    }
}
