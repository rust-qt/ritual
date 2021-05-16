#!/bin/bash

set -o errexit
set -x

export RUST_BACKTRACE=1
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir

cd "$TRAVIS_BUILD_DIR"

# Make sure the correct toolchain is used on Windows.
echo "$TRAVIS_RUST_VERSION" > rust-toolchain

rustup component add rustfmt
cargo fmt -- --check

if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
    export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/Library/Developer/CommandLineTools/usr/lib
elif [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
    export LLVM_CONFIG_PATH=/usr/lib/llvm-10/bin/llvm-config
elif [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
    curl -o "$TEMP/sqlite.zip" "https://www.sqlite.org/2016/sqlite-dll-win64-x64-3150100.zip"
    export SQLITE3_LIB_DIR=$TEMP/sqlite
    7z x "$TEMP/sqlite.zip" -o"$SQLITE3_LIB_DIR"
    cmd.exe //C 'C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvarsall.bat' amd64 '&&' lib '/def:%SQLITE3_LIB_DIR%\sqlite3.def' '/out:%SQLITE3_LIB_DIR%\sqlite3.lib'
    export PATH=$PATH:$SQLITE3_LIB_DIR
fi

cd "$TRAVIS_BUILD_DIR"
rustup component add clippy
cargo clippy --color=always --all-targets -- -D warnings

function build() {
    if [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
        cmd.exe //C 'C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvarsall.bat' amd64 '&&' "$@"
    else
        "$@"
    fi
}

if [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
    export BUILD_MODE=--release
else
    export BUILD_MODE=
fi

build cargo test $BUILD_MODE --color=always -- --nocapture
