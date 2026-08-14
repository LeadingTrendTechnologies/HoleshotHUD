@echo off
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\pack.ps1"
if errorlevel 1 exit /b 1
echo.
echo Zip is in dist\
