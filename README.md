# cmri-rs

[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks.yml)

A selection of crates for working with CMRInet as defined in [NMRA Specification LCS-9.10.1](https://www.nmra.org/sites/default/files/standards/sandrp/Other_Specifications/lcs-9.10.1_cmrinet_v1.1.pdf).

## cmri

[![Docs](https://docs.rs/pg_filters/badge.svg)](https://docs.rs/pg_filters/latest/pg_filters/)
[![Crates](https://img.shields.io/crates/v/cmri.svg)](https://crates.io/crates/cmri)
[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri.yml)

Used for parsing/generating CMRInet packets and frames.

## cmri_tools

[![Crates](https://img.shields.io/crates/v/cmri_tools.svg)](https://crates.io/crates/cmri_tools)
[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri_tools.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri_tools.yml)

A selection of tools useful for developing/debugging software for a CMRInet.

## test_nodes (not released as crate)

[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-test_nodes.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-test_nodes.yml)

A selection of simple binaries for a raspberry pi pico, used whilst developing cmri_tools,
and trying out the "feel" of cmri's API in a nostd environment.

* test_node: A CPMEGA node (address 5) with 1 input and 1 output byte.
* poller: A controller which polls the test_node.
* simulator: Generates fake traffic between a controller and 5 nodes (1 each of: USIC, SUSIC, SMINI, CPNODE, and CPMEGA).
