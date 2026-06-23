@echo off
setlocal
cd /d "%~dp0"

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0create_start_shortcut.ps1" -Root "%ROOT%"
if errorlevel 1 exit /b 1

echo Created desktop shortcut: POE2-CN-Price
echo 默认启动/唤出热键: F7
echo 查价: 游戏里 Ctrl+C 自动查价
echo 手动重查热键可在 Settings.bat 或托盘设置里修改/关闭
if /i "%~1"=="--no-pause" exit /b 0
pause
