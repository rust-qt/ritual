#!/bin/bash

# This script installs libclang on Travis.

set -o errexit

if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
  CLANG_DIR=$HOME/clang-3.8
  cd "$HOME"
  echo "Downloading libclang"
  wget http://llvm.org/releases/3.8.0/clang+llvm-3.8.0-x86_64-apple-darwin.tar.xz -O - | tar -xJ
  mv clang+llvm-3.8.0-x86_64-apple-darwin "$CLANG_DIR"

  set -x
  export LLVM_CONFIG_PATH=$CLANG_DIR/bin/llvm-config
  export CLANG_SYSTEM_INCLUDE_PATH=$CLANG_DIR/lib/clang/3.8.0/include
  export LD_LIBRARY_PATH=$CLANG_DIR/lib
  set +x
else
  set -x
  export LLVM_CONFIG_PATH=/usr/lib/llvm-3.8/bin/llvm-config
  export CLANG_SYSTEM_INCLUDE_PATH=/usr/lib/llvm-3.8/lib/clang/3.8.0/include
  set +x
fi
