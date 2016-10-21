echo on

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

call "%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_rust.bat" || goto :error

set CPP_TO_RUST_QUIET=1
set RUST_BACKTRACE=1

echo "Compiling and testing cpp_to_rust"
cargo test || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%
