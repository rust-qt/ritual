echo on
setlocal EnableDelayedExpansion

"%APPVEYOR_BUILD_FOLDER%\ci\appveyor\setup_rust.bash" || goto :error

set CPP_TO_RUST_QUIET=1
set RUST_BACKTRACE=1

echo "Setting VS environment"
call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64 || goto :error

set PATH=C:\Qt\5.7\msvc2015_64\bin;%PATH%

echo "Compiling and testing cpp_to_rust"
cargo test || goto :error

goto :eof

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%

