echo on

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

call "%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_rust.bat" || goto :error
call "%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_sqlite.bat" || goto :error

set RUST_BACKTRACE=1

set CPP_TO_RUST_TEMP_TEST_DIR=%USERPROFILE%\cpp_to_rust_temp_test_dir
if not exist "%CPP_TO_RUST_TEMP_TEST_DIR%" mkdir "%CPP_TO_RUST_TEMP_TEST_DIR%"

cd "%APPVEYOR_BUILD_FOLDER%\cpp_to_rust\cpp_utils"
cargo update || goto :error
cargo test || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\cpp_to_rust\cpp_to_rust_common"
cargo update || goto :error
cargo test -v || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\cpp_to_rust\cpp_to_rust_build_tools"
cargo update || goto :error
cargo test -v || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\cpp_to_rust\cpp_to_rust_generator"
cargo update || goto :error
cargo test -v -- --nocapture || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\qt_generator\qt_generator_common"
cargo update || goto :error
cargo test -v || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\qt_generator\qt_build_tools"
cargo update || goto :error
cargo test -v || goto :error

cd "%APPVEYOR_BUILD_FOLDER%\qt_generator\qt_generator"
cargo update || goto :error
cargo test -v || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%
