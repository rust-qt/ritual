echo on

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

call "%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_rust.bat" || goto :error
call "%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_sqlite.bat" || goto :error

set RUST_BACKTRACE=1

set CPP_TO_RUST_TEMP_TEST_DIR=%USERPROFILE%\cpp_to_rust_temp_test_dir
if not exist "%CPP_TO_RUST_TEMP_TEST_DIR%" mkdir "%CPP_TO_RUST_TEMP_TEST_DIR%"

cd "%APPVEYOR_BUILD_FOLDER%"
cargo clippy --all-targets || goto :error
cargo test -v || goto :error
cargo fmt -- --check || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%
