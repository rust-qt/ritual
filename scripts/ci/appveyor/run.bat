echo on

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

call "%APPVEYOR_BUILD_FOLDER%\scripts\ci\appveyor\setup_rust.bat" || goto :error
call "%APPVEYOR_BUILD_FOLDER%\scripts\ci\appveyor\setup_sqlite.bat" || goto :error

set PATH=%PATH%;C:\Program Files\LLVM\bin

set RUST_BACKTRACE=1

set RITUAL_TEMP_TEST_DIR=%USERPROFILE%\ritual_temp_test_dir

cd "%APPVEYOR_BUILD_FOLDER%"
cargo clippy --all-targets -- -D warnings || goto :error
cargo test -v -- --nocapture || goto :error
cargo fmt -- --check || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%
