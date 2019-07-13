#!/bin/bash

set -o errexit
set -x

export RUST_BACKTRACE=1
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir

if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
    export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/Library/Developer/CommandLineTools/usr/lib
elif [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
    sudo apt-get update
    sudo apt-get install llvm-3.8 libclang-3.8-dev --yes --force-yes
    export LLVM_CONFIG_PATH=/usr/lib/llvm-3.8/bin/llvm-config
    export CLANG_SYSTEM_INCLUDE_PATH=/usr/lib/llvm-3.8/lib/clang/3.8.0/include
elif [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
    curl -o "$TEMP/sqlite.zip" "https://www.sqlite.org/2016/sqlite-dll-win64-x64-3150100.zip"
    export SQLITE3_LIB_DIR=$TEMP/sqlite
    7z x "$TEMP/sqlite.zip" -o"$SQLITE3_LIB_DIR"
    cmd.exe /C '"C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" amd64 && lib /def:%SQLITE3_LIB_DIR%\sqlite3.def /out:%SQLITE3_LIB_DIR%\sqlite3.lib'
    export PATH=$PATH:$SQLITE3_LIB_DIR
fi

cd "$TRAVIS_BUILD_DIR"
rustup component add clippy
rustup component add rustfmt

cargo fmt -- --check
cargo clippy --color=always --all-targets -- -D warnings

if [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
cmd.exe /C '"C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" amd64 && cargo test --color=always --release -- --nocapture'
else
    cargo test --color=always -- --nocapture
fi
