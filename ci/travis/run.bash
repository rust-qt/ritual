#!/bin/bash

set -o errexit

export RUST_BACKTRACE=1
export CPP_TO_RUST_TEMP_TEST_DIR=$HOME/cpp_to_rust_temp_test_dir
mkdir -p "$CPP_TO_RUST_TEMP_TEST_DIR"

cd "$TRAVIS_BUILD_DIR"

source ci/travis/setup_clang.bash

rustup component add clippy
rustup component add rustfmt
cargo clippy --all-targets
cargo test -v
cargo fmt -- --check
