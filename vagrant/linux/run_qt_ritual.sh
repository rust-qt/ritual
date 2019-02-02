#!/usr/bin/env bash

cd /vagrant

export LIBCLANG_PATH=/usr/lib/llvm-6.0/lib/
export CARGO_TARGET_DIR=/home/vagrant/cargo_build
export RUST_BACKTRACE=1

cargo run -p qt_ritual -- "$@"

