#!/bin/bash

set -o errexit

export RUST_BACKTRACE=1
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir

source "$TRAVIS_BUILD_DIR/ci/travis/setup_clang.bash"

cd "$TRAVIS_BUILD_DIR"
rustup component add clippy
rustup component add rustfmt
cargo clippy --all-targets -- -D warnings
cargo test -v
cargo fmt -- --check
