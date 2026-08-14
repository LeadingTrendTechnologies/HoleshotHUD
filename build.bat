@echo off
setlocal
cd /d "%~dp0"

set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VCVARS%" set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VCVARS%" set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS%" (
  call "%VCVARS%" >nul
)

where msbuild >nul 2>nul
if errorlevel 1 (
  echo MSBuild not found. Install Visual Studio 2022 with C++ desktop tools.
  exit /b 1
)

msbuild mxbo.vcxproj /p:Configuration=Release /p:Platform=x64 /m
if errorlevel 1 exit /b 1

where cargo >nul 2>nul
if errorlevel 1 (
  echo cargo not found. Install Rust from https://rustup.rs/
  exit /b 1
)

set "CARGO_TARGET_DIR="
cargo build --release --manifest-path overlay\Cargo.toml
if errorlevel 1 exit /b 1

echo.
echo Built:
echo   out\Release\mxbo.dlo
echo   overlay\target\release\mxbo-overlay.exe
echo Run pack.bat to make a zip for other PCs.
endlocal
