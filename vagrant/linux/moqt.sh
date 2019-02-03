#!/usr/bin/env bash

set -e

cd ~
mkdir -p build_moqt/build_moqt_core
cd build_moqt/build_moqt_core
cmake -DCMAKE_INSTALL_PREFIX=$HOME/moqt/moqt_core /vagrant/moqt/moqt_core
make install


