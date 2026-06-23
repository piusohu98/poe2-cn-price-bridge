@echo off
setlocal
cd /d "%~dp0"
if not exist "target\release\poe2_cn_price_bridge.exe" (
  call "%~dp0setup_msvc_env.bat"
  cargo build --release
  if errorlevel 1 pause & exit /b 1
)
set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0set_cookie_gui.ps1" -Root "%ROOT%"
exit /b 0
