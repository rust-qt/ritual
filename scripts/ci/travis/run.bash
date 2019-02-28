#!/bin/bash

set -o errexit

export RUST_BACKTRACE=1
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir

export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/Users/travis/ritual_temp_test_dir/test_full_run/install/lib

source "$TRAVIS_BUILD_DIR/scripts/ci/travis/setup_clang.bash"

cd "$TRAVIS_BUILD_DIR"
rustup component add clippy
rustup component add rustfmt
cargo clippy --all-targets -- -D warnings
cargo test -v
cargo fmt -- --check
