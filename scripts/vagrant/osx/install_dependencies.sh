#!/usr/bin/env bash

set -e
set -x

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

SCRIPT_DIR=$(realpath $(dirname $0))

source "$SCRIPT_DIR/../install_rust.sh"

if brew --version ; then
    echo "brew is already installed"
else
    echo "installing brew"
    /usr/bin/ruby -e "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install)"
fi

brew install cmake

export DYLD_LIBRARY_PATH=/Library/Developer/CommandLineTools/usr/lib
export RITUAL_TEMP_TEST_DIR=$HOME/ritual_temp_test_dir
