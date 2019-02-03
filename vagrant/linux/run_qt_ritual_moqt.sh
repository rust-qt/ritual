#!/usr/bin/env bash

set -e

dir=$(dirname $(readlink -f $0))

$dir/moqt.sh

export MOQT_PATH=$HOME/moqt

export CPLUS_INCLUDE_PATH=$MOQT_PATH/moqt_core/include
export LIBRARY_PATH=$MOQT_PATH/moqt_core/lib

$dir/run_qt_ritual.sh $HOME/moqt_workspace "$@"
