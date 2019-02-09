#!/usr/bin/env bash

set -e

SCRIPT_DIR=$(dirname $(readlink -f $0))
REPO_DIR="$SCRIPT_DIR/../"

export MOQT_BUILD_DIR=$HOME/build_moqt
export MOQT_INSTALL_DIR=$HOME/moqt
"$REPO_DIR/scripts/build_moqt.sh"


