# CMRI

[![Docs](https://docs.rs/cmri/badge.svg)](https://docs.rs/cmri/latest/cmri/)
[![Crates](https://img.shields.io/crates/v/cmri.svg)](https://crates.io/crates/cmri)
[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri.yml)

Pure-Rust decoding/encoding of packets for CMRInet (as defined in [NMRA Specification LCS-9.10.1](https://www.nmra.org/sites/default/files/standards/sandrp/Other_Specifications/lcs-9.10.1_cmrinet_v1.1.pdf)).

## Features

### std

This is a std crate by default.

### serde

The serde feature adds serializating and deserializating of all items using serde.

### experimenter

The experimenter feature is intended for people who are using nonstandard packets/nodes,
perhaps in their own experimentation. It enables the following changes:

* Instead of getting a [`packet::Error::InvalidMessageType`] for an otherwise valid message type (ASCII uppercase), you'll get a [`packet::Payload::Unknown`].
* Instead of getting a [`packet::Error::InvalidNodeType`] for an otherwise valid node definition parameter (ASCII alphabetic), you'll get a [`node_configuration::NodeSort::Unknown`].
* [`packet::Packet::try_new_unknown`]

## Testing

The test suite is run for all Tier 1 targets from <https://doc.rust-lang.org/nightly/rustc/platform-support.html> on nightly (at time of pushing to GitHub) rust.

The ability to build is tested on nightly (at time of pushing to GitHub) rust, for tier 2 targets with host tools, plus:

* wasm32-unknown-emscripten
* wasm32-unknown-unknown
* wasm32-wasip1

Additionally nostd building is tested for:

* mips-unknown-linux-gnu **only as `no_std`**
* mips64-unknown-linux-gnuabi64 **only as `no_std`**
* mips64el-unknown-linux-gnuabi64 **only as `no_std`**
* mipsel-unknown-none **only as `no_std`**
* avr-unknown-gnu-atmega328 - AVR **only as `no_std`**
