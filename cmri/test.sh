#!/usr/bin/bash

set -e
set -v


cargo +nightly test --no-default-features
cargo +nightly test --all-features

cargo +nightly test --no-default-features --features std
cargo +nightly test --no-default-features --features serde
cargo +nightly test --no-default-features --features experimenter

cargo +nightly test --no-default-features --features std,serde
cargo +nightly test --no-default-features --features std,experimenter
cargo +nightly test --no-default-features --features serde,experimenter


cargo +stable test --no-default-features
cargo +stable test --all-features

cargo +stable test --no-default-features --features std
cargo +stable test --no-default-features --features serde
cargo +stable test --no-default-features --features experimenter

cargo +stable test --no-default-features --features std,serde
cargo +stable test --no-default-features --features std,experimenter
cargo +stable test --no-default-features --features serde,experimenter


cargo +nightly bench --quiet --all-features
