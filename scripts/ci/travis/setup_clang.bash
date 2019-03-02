#!/bin/bash

# This script installs libclang on Travis.

set -o errexit

if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
    export DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH:/Library/Developer/CommandLineTools/usr/lib
else
    export LLVM_CONFIG_PATH=/usr/lib/llvm-3.8/bin/llvm-config
    export CLANG_SYSTEM_INCLUDE_PATH=/usr/lib/llvm-3.8/lib/clang/3.8.0/include
fi
