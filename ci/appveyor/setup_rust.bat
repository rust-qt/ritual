echo on

set PATH=%USERPROFILE%\.cargo\bin;%PATH%

rustup show || goto :install_rust
echo "Rustup is already installed."
rustup self update || goto :error

:rust_ok
where rustc cargo || goto :error
rustc -vV || goto :error
cargo -vV || goto :error
rustup component add clippy
rustup component add rustfmt

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%

:install_rust
echo "Installing rustup"
curl -sSf -o "%TEMP%\rustup-init.exe" https://win.rustup.rs/ || goto :error
"%TEMP%\rustup-init.exe" -y || goto :error
rustup show || goto :error
goto :rust_ok
