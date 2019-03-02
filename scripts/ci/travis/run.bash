#!/bin/bash

set -o errexit
set -x

export RUST_BACKTRACE=1
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir

export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/Library/Developer/CommandLineTools/usr/lib

cd "$TRAVIS_BUILD_DIR"
rustup component add clippy
rustup component add rustfmt

cargo clippy --all-targets -- -D warnings
cargo test -v
cargo fmt -- --check
