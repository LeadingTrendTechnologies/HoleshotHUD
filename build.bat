@echo off
setlocal
cd /d "%~dp0"

python tools\gen_font.py
if errorlevel 1 exit /b 1

set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if not exist "%VCVARS%" (
  echo Visual Studio 2022 vcvars64.bat not found.
  exit /b 1
)

call "%VCVARS%" >nul
msbuild mxbo.vcxproj /p:Configuration=Release /p:Platform=x64 /m
if errorlevel 1 exit /b 1

where cargo >nul 2>nul
if not errorlevel 1 (
  cargo build --release --manifest-path overlay\Cargo.toml
  if errorlevel 1 exit /b 1
)

set "MXDIR=D:\Steam\steamapps\common\MX Bikes"
if not exist "%MXDIR%\plugins" (
  echo MX Bikes plugins folder not found at "%MXDIR%\plugins"
  echo Built plugin is in out\Release\mxbo.dlo — copy it and assets\ yourself.
  exit /b 0
)

copy /Y "out\Release\mxbo.dlo" "%MXDIR%\plugins\mxbo.dlo"
if errorlevel 1 (
  echo WARNING: Could not copy mxbo.dlo - fully quit MX Bikes and run build.bat again.
)
mkdir "%MXDIR%\plugins\mxbo_data\fonts" 2>nul
xcopy /E /Y /Q "assets\*" "%MXDIR%\plugins\mxbo_data\" >nul
echo Installed mxbo.dlo and mxbo_data to "%MXDIR%\plugins"
endlocal
