#!/usr/bin/env bash

cd /vagrant
CARGO_TARGET_DIR=/home/vagrant/cargo_build cargo run -p qt_generator -- "$@"

