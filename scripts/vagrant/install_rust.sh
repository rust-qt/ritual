#!/usr/bin/env bash

set -e
set -x

# install Rust
if rustup -V ; then
    echo "rustup is already installed"
    rustup update
else
    echo "installing rustup"
    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source $HOME/.cargo/env
fi

cd /vagrant
rustup toolchain install `cat rust-toolchain`
rustup component add clippy
rustup component add rustfmt

