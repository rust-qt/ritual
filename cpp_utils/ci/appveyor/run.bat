echo on

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

curl -o "%TEMP%\setup_rust.bat" https://raw.githubusercontent.com/rust-qt/cpp_to_rust/master/ci/appveyor/setup_rust.bat || goto :error
call "%TEMP%\setup_rust.bat" || goto :error

set RUST_BACKTRACE=1
cargo test || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%
