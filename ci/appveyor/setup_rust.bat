echo on
setlocal EnableDelayedExpansion

set PATH=%USERPROFILE%\.cargo\bin;%PATH%

rustup show
if "%ERRORLEVEL%" == "0" (
  echo "Rustup is already installed."
  rustup update stable
) else (
  echo "Installing rustup"
  curl -sSf -o "%TEMP%\rustup-init.exe" https://win.rustup.rs/ || goto :error
  "%TEMP%\rustup-init.exe" -y || goto :error
  rustup show || goto :error
)

rustup toolchain list || goto :error
where rustc cargo || goto :error
rustc -vV || goto :error
cargo -vV || goto :error

exit /b 0

:error
echo "Failed (errorlevel = %errorlevel%)"
exit /b %errorlevel%

