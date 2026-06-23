@echo off
setlocal
cd /d "%~dp0"
if not exist "target\release\poe2_cn_price_bridge.exe" (
  call "%~dp0setup_msvc_env.bat"
  cargo build --release
  if errorlevel 1 pause & exit /b 1
)
start "" "%~dp0target\release\poe2_cn_price_bridge.exe" %*
exit /b 0
