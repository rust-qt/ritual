#!/usr/bin/env bash

set -e
set -x

SCRIPT_DIR=$(dirname $(readlink -f $0))


# add llvm repository
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
sudo add-apt-repository "deb http://apt.llvm.org/bionic/ llvm-toolchain-bionic-6.0 main"

sudo apt-get update
sudo apt-get install -y build-essential cmake libsqlite3-dev libclang-6.0-dev

source "$SCRIPT_DIR/../install_rust.sh"
