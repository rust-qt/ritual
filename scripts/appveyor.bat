echo on
setlocal EnableDelayedExpansion

set QT_REPOS_BRANCH=-b master

set FILES=%USERPROFILE%\files
if not exist "%FILES%" mkdir "%FILES%"

if "%BUILD_TYPE%"=="debug" (
  echo "Building in debug mode."
  set CARGO_ARGS=
  set RUST_BACKTRACE=1
) else (
  echo "Building in release mode."
  set CARGO_ARGS="--release"
)

if "%APPVEYOR_BUILD_FOLDER%"=="" (
  set BUILD_DIR=%cd%
) else (
  set BUILD_DIR=%APPVEYOR_BUILD_FOLDER%
)


if "%APPVEYOR%"=="True" (
  echo "Appveyor detected. Forcing quiet mode."
  set CPP_TO_RUST_QUIET=1
)

set PATH=%USERPROFILE%\.cargo\bin;%PATH%

rustup show
if "%ERRORLEVEL%" == "0" (
  echo "Rustup is already installed."
) else (
  echo "Installing rustup"
  cd "%FILES%"
  curl -sSf -o rustup-init.exe https://win.rustup.rs/ || goto :error
  rustup-init.exe -y || goto :error
  rustup show || goto :error
)

rustup toolchain list || goto :error
where rustc cargo || goto :error
rustc -vV || goto :error
cargo -vV || goto :error


echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

set PATH=C:\Qt\5.7\msvc2015_64\bin;%PATH%

cd "%BUILD_DIR%"
if exist "%FILES%\tests_ok" (
  echo "Skipped compiling and testing cpp_to_rust because %FILES%/tests_ok already exists"
) else (
  echo "Compiling and testing cpp_to_rust"
  cargo test %CARGO_ARGS% || goto :error
  type nul > "%FILES%\tests_ok" || goto :error
)

set REPOS=%FILES%\repos
set OUT=%FILES%\output

if exist "%REPOS%" (
  echo "Skipped cloning Qt library repos because %REPOS% already exists"
) else (
  echo "Cloning Qt library repos"
  mkdir "%REPOS%" || goto :error
  cd "%REPOS%" || goto :error
  git clone %QT_REPOS_BRANCH% https://github.com/rust-qt/qt_core.git || goto :error
  if "%APPVEYOR%" == "True" (
    echo "Quick mode: only qt_core"
  ) else (
    git clone %QT_REPOS_BRANCH% https://github.com/rust-qt/qt_gui.git || goto :error
    git clone %QT_REPOS_BRANCH% https://github.com/rust-qt/qt_widgets.git || goto :error
  )
)

echo "Running cpp_to_rust on Qt libraries"
cd "%BUILD_DIR%"
call :build_one qt_core || goto :error
if "%APPVEYOR%" == "True" (
  echo "Quick mode: only qt_core"
) else (
  call :build_one qt_gui "-d %OUT%\qt_core_out" || goto :error
  call :build_one qt_widgets "-d %OUT%\qt_core_out %OUT%\qt_gui_out" || goto :error
)

goto :eof

:build_one
  set NAME=%~1
  set DEPS=%~2
  set COMPLETED="%OUT%\%NAME%_out\completed"
  if exist "%COMPLETED%" (
    echo "Skipped building and testing %NAME% because %COMPLETED% already exists"
  ) else (
    echo "Building and testing %NAME%"
    cargo run %CARGO_ARGS% -- -s %REPOS%\%NAME% -o %OUT%\%NAME%_out %DEPS% || goto :error
    type nul > "%COMPLETED%" || goto :error
  )

goto :eof

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%

