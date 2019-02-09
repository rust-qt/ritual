#!/usr/bin/env bash

set -e

SCRIPT_DIR=$(dirname $(readlink -f $0))
REPO_DIR="$SCRIPT_DIR/../"

"$SCRIPT_DIR/build_moqt.sh"
source "$REPO_DIR/scripts/env_moqt.sh"
"$SCRIPT_DIR/run_qt_ritual.sh" $HOME/moqt_workspace "$@"
