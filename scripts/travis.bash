#!/bin/bash

# This script is used to run tests in Travis's Linux and Mac OS environments,
# but it's possible to run it locally:
#
# - set NOT_TRAVIS_FILES to the directory where to write files
#   (defaults to $HOME);
# - set BUILD_TYPE to "debug" if you want to disable release mode;
# - current directory should be cpp_to_rust repositry.
#
# The script will skip some parts if certain files  are present.
# Remove these files to force execution.
# Travis runs the script on a clean VM, so it will run everything.


set -o errexit

QT_REPOS_BRANCH="-b master"


if [ "$TRAVIS" = "true" ]; then
  echo "Travis detected. Forcing quiet mode."
  export CPP_TO_RUST_QUIET=1
fi

if [ "$TRAVIS_BUILD_DIR" = "" ]; then
  echo "TRAVIS_BUILD_DIR is not present!"
  TRAVIS_BUILD_DIR=`pwd`
  echo "TRAVIS_BUILD_DIR is set to \"$TRAVIS_BUILD_DIR\""
fi
if [ "$NO_TRAVIS_FILES" = "" ]; then
  FILES=$HOME
else
  FILES=$NO_TRAVIS_FILES
fi
echo "Files are stored in \"$FILES\""



if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
  OS_NAME=osx
elif [[ "$TRAVIS_OS_NAME" == "linux" ]]; then
  OS_NAME=linux
else
  case "$OSTYPE" in
    linux*)   OS_NAME=linux ;;
    darwin*)  OS_NAME=osx ;;
    win*)     OS_NAME=windows ;;
    cygwin*)  OS_NAME=cygwin ;;
    bsd*)     OS_NAME=bsd ;;
    solaris*) OS_NAME=solaris ;;
    *)        OS_NAME=$OSTYPE ;;
  esac
fi
echo "Detected OS: $OS_NAME"

if [[ "$OS_NAME" == "osx" ]]; then
  cd $FILES
  CLANG_DIR=$FILES/clang-3.8
  if [ -d "$CLANG_DIR" ]; then
    echo "$CLANG_DIR already exists"
  else
    echo "Downloading libclang"
    wget http://llvm.org/releases/3.8.0/clang+llvm-3.8.0-x86_64-apple-darwin.tar.xz -O - | tar -xJ
    mv clang+llvm-3.8.0-x86_64-apple-darwin "$CLANG_DIR"
  fi
  set -x
  export LLVM_CONFIG_PATH=$CLANG_DIR/bin/llvm-config
  export CLANG_SYSTEM_INCLUDE_PATH=$CLANG_DIR/lib/clang/3.8.0/include
  set +x
  QT_DIR=$FILES/Qt5.7.0
  if [ -d "$QT_DIR" ]; then
    echo "$QT_DIR already exists"
  else
    echo "Downloading Qt"
    wget https://github.com/rust-qt/extra_files/releases/download/v0.0.1/Qt5.7.0.tar.gz -O - | tar -xJ
  fi
  set -x
  export PATH=$QT_DIR/bin:$PATH
  export QT_PLUGIN_PATH=$QT_DIR/plugins
  set +x
  XVFB_RUN=""

elif [[ "$OS_NAME" == "linux" ]]; then
  set -x
  export LLVM_CONFIG_PATH=/usr/lib/llvm-3.8/bin/llvm-config
  export CLANG_SYSTEM_INCLUDE_PATH=/usr/lib/llvm-3.8/lib/clang/3.8.0/include
  set +x
  XVFB_RUN="xvfb-run -a"

else
  echo "TRAVIS_OS_NAME env var must be either 'osx' or 'linux'."
  exit 1
fi

if [[ "$BUILD_TYPE" == "debug" ]]; then
  echo "Building in debug mode."
  CARGO_ARGS=""
  export RUST_BACKTRACE=1
else
  echo "Building in release mode."
  CARGO_ARGS="--release"
fi



if [ -f "$FILES/tests_ok" ]; then
  echo "Skipped compiling and testing cpp_to_rust because $FILES/tests_ok already exists"
else
  echo "Compiling and testing cpp_to_rust"
  cd "$TRAVIS_BUILD_DIR"
  cargo test $CARGO_ARGS --verbose
  touch $FILES/tests_ok
fi

# cargo build $CARGO_ARGS
# exit

cd $FILES
REPOS=$FILES/repos
OUT=$FILES/output
if [ -d "$REPOS" ]; then
  echo "Skipped cloning Qt library repos because $REPOS already exists"
else
  echo "Cloning Qt library repos"
  mkdir "$REPOS"
  cd "$REPOS"
  git clone $QT_REPOS_BRANCH https://github.com/rust-qt/qt_core.git
  if [ "$TRAVIS" = "true" ]; then
    echo "Quick mode: only qt_core"
  else
    git clone $QT_REPOS_BRANCH https://github.com/rust-qt/qt_gui.git
    git clone $QT_REPOS_BRANCH https://github.com/rust-qt/qt_widgets.git
  fi
fi

cd "$TRAVIS_BUILD_DIR"

echo "Running cpp_to_rust on Qt libraries"

function build_one {
  local NAME=$1
  local DEPS=$2
  local PREFIX=$3
  local COMPLETED="$OUT/${NAME}_out/completed"
  if [ -f "$COMPLETED" ]; then
    echo "Skipped building and testing $NAME because $COMPLETED already exists"
  else
    echo "Building and testing $NAME"
    $PREFIX cargo run $CARGO_ARGS -- -s $REPOS/$NAME -o $OUT/${NAME}_out $DEPS
    touch "$COMPLETED"
  fi
}

build_one qt_core
if [ "$TRAVIS" = "true" ]; then
  echo "Quick mode: only qt_core"
else
  build_one qt_gui "-d $OUT/qt_core_out" "$XVFB_RUN"
  build_one qt_widgets "-d $OUT/qt_core_out $OUT/qt_gui_out" "$XVFB_RUN"
fi

