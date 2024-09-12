# CMRI Tools

[![Crates](https://img.shields.io/crates/v/cmri_tools.svg)](https://crates.io/crates/cmri_tools)
[![Checks](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri_tools.yml/badge.svg?branch=main)](https://github.com/RobertGauld/cmri-rs/actions/workflows/commit_checks-cmri_tools.yml)

A collection of tools I found useful for experimenting with CMRInet (as defined in [NMRA Specification LCS-9.10.1](https://www.nmra.org/sites/default/files/standards/sandrp/Other_Specifications/lcs-9.10.1_cmrinet_v1.1.pdf)).

## The Binaries

### hub

A CLI/GUI application which provids the means to interconnect devices/software which have differing
connection requirements, anything received on a connection is written to all the others.

Use the \-\-help command line flag for usage information.

### monitor

A GUI application which interprets the packets on a C/MRI network and provides:

* The number of packets (in total and by type) seen.
* A list of seen nodes.
* For each node a detailed view including:
  * The number of packets (in total and by type) seen.
  * Type and configuration (if the initialization packet was seen).
  * Their input states (from the last receive data packet seen).
  * Their output states (from the last transmit data packet seen).

Use the \-\-help command line flag for usage information.

If compiled with the experimenter feature then the packets over time plots also show unknown packets.

### controller

A GUI application for controlling the nodes of a CMRInet.
The user can view the inputs and set the outputs of each node.

Use the \-\-help command line flag for usage information.

### nodes

A GUI application for "simulating" the nodes of a CMRInet.
The user can view the outputs set by the controller and set the inputs.

Use the \-\-help command line flag for usage information.

### node

A GUI application for "simulating" a single node on a CMRInet.
The user can view the outputs set by the controller and set the inputs.

Use the \-\-help command line flag for usage information.

## Testing

Tested against all tier 1 targets (except Windows using GNU build) from <https://doc.rust-lang.org/nightly/rustc/platform-support.html> on nightly (at time of pushing to GitHub) rust.
