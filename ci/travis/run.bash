#!/bin/bash

set -o errexit

export CPP_TO_RUST_QUIET=1
export RUST_BACKTRACE=1

source "$TRAVIS_BUILD_DIR/ci/travis/setup_clang.bash"

cd "$TRAVIS_BUILD_DIR"
cargo update
cargo test -v

