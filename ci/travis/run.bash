#!/bin/bash

set -o errexit

export CPP_TO_RUST_QUIET=1
export RUST_BACKTRACE=1
export CPP_TO_RUST_TEMP_TEST_DIR=$HOME/cpp_to_rust_temp_test_dir
mkdir -p "$CPP_TO_RUST_TEMP_TEST_DIR"

source "$TRAVIS_BUILD_DIR/ci/travis/setup_clang.bash"

cd "$TRAVIS_BUILD_DIR/cpp_to_rust/cpp_utils"
cargo update && cargo test -v

cd "$TRAVIS_BUILD_DIR/cpp_to_rust/cpp_to_rust_common"
cargo update && cargo test -v

cd "$TRAVIS_BUILD_DIR/cpp_to_rust/cpp_to_rust_build_tools"
cargo update && cargo test -v

cd "$TRAVIS_BUILD_DIR/cpp_to_rust/cpp_to_rust_generator"
cargo update && cargo test -v -- --nocapture

cd "$TRAVIS_BUILD_DIR/qt_generator/qt_generator_common"
cargo update && cargo test -v

cd "$TRAVIS_BUILD_DIR/qt_generator/qt_build_tools"
cargo update && cargo test -v

cd "$TRAVIS_BUILD_DIR/qt_generator/qt_generator"
cargo update && cargo test -v
