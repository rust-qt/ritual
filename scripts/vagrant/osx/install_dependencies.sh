#!/usr/bin/env bash

set -e
set -x

SCRIPT_DIR=$(dirname $(readlink -f $0))

source "$SCRIPT_DIR/../install_rust.sh"
