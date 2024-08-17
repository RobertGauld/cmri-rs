#!/usr/bin/bash

set -e
set -v


cargo +nightly test --no-default-features
cargo +nightly test --features experimenter


cargo +stable test --no-default-features
cargo +stable test --features experimenter
