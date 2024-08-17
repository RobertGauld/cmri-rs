#!/usr/bin/bash

set -e
set -v

(cd cmri && ./test.sh)

(cd cmri_tools && ./test.sh)

cargo +nightly clippy --package cmri --no-default-features --no-deps --all-targets -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::missing_const_for_fn -W clippy::cargo -A clippy::multiple_crate_versions
cargo +nightly clippy --package cmri --all-features --no-deps --all-targets -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::missing_const_for_fn -W clippy::cargo -A clippy::multiple_crate_versions
cargo +nightly clippy --no-deps --all-targets -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::missing_const_for_fn -W clippy::cargo -A clippy::multiple_crate_versions

cargo +nightly doc --all-features --no-deps --document-private-items
