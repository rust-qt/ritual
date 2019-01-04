#!/usr/bin/env bash

export MOQT_PATH=/home/vagrant/moqt

export CPLUS_INCLUDE_PATH=/home/vagrant/moqt/moqt_core/include
export LIBRARY_PATH=/home/vagrant/moqt/moqt_core/lib

/vagrant/vagrant/linux/run_qt_generator.sh -w /home/vagrant/moqt_workspace "$@"
