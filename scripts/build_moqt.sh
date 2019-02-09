#!/usr/bin/env bash

set -e

SCRIPT_DIR=$(dirname $(readlink -f $0))
REPO_DIR="$SCRIPT_DIR/../"

mkdir -p "$MOQT_BUILD_DIR/build_moqt_core"
cd "$MOQT_BUILD_DIR/build_moqt_core"
cmake "-DCMAKE_INSTALL_PREFIX=$MOQT_INSTALL_DIR/moqt_core" "$REPO_DIR/qt_ritual/test_assets/moqt/moqt_core"
make install


