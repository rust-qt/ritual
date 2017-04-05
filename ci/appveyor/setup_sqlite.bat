curl -o "%TEMP%\sqlite.zip" "https://www.sqlite.org/2016/sqlite-dll-win64-x64-3150100.zip" || goto :error
7z x "%TEMP%\sqlite.zip" -o"%TEMP%\sqlite" || goto :error
set SQLITE3_LIB_DIR=%TEMP%\sqlite
lib /def:%SQLITE3_LIB_DIR%\sqlite3.def /out:%SQLITE3_LIB_DIR%\sqlite3.lib || goto :error
