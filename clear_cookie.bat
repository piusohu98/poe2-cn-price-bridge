@echo off
setlocal
cd /d "%~dp0"
if not exist "target\release\poe2_cn_price_bridge.exe" (
  call "%~dp0setup_msvc_env.bat"
  cargo build --release
  if errorlevel 1 pause & exit /b 1
)
"%~dp0target\release\poe2_cn_price_bridge.exe" --clear-cookie
if errorlevel 1 (
  echo 清除失败。
  pause
  exit /b 1
)
echo 已清除保存的 Cookie。
pause
