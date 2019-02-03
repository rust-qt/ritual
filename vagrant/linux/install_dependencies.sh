#!/usr/bin/env bash

set -e
set -x

# add llvm repository
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key|sudo apt-key add -
sudo add-apt-repository "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-6.0 main"

sudo apt-get update
sudo apt-get install -y build-essential cmake libsqlite3-dev libclang-6.0-dev


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
