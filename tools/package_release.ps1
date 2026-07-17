param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
    [switch] $SkipBuild
)

$ErrorActionPreference = 'Stop'
$rootPath = (Resolve-Path -LiteralPath $Root).Path
$loginProject = Join-Path $rootPath 'login\QingPriceLogin\QingPriceLogin.csproj'
$loginOutput = Join-Path $rootPath 'login\QingPriceLogin\bin\Release\net48'

& (Join-Path $PSScriptRoot 'make_icon.ps1') -Root $rootPath

if (-not $SkipBuild) {
    Push-Location $rootPath
    try {
        $setup = Join-Path $rootPath 'setup_msvc_env.bat'
        if (Test-Path -LiteralPath $setup) {
            cmd.exe /c "`"$setup`" && cargo build --release"
        } else {
            cargo build --release
        }
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}


if (-not $SkipBuild) {
    dotnet build $loginProject -c Release
    if ($LASTEXITCODE -ne 0) {
        throw "QingPriceLogin build failed with exit code $LASTEXITCODE"
    }
}

$cargoToml = Get-Content -LiteralPath (Join-Path $rootPath 'Cargo.toml') -Raw
if ($cargoToml -notmatch 'version\s*=\s*"([^"]+)"') {
    throw 'Cannot read version from Cargo.toml'
}
$version = $Matches[1]

$distRoot = Join-Path $rootPath 'dist'
$packageName = "QingPricePOE2-v$version-windows-x64"
$packageDir = Join-Path $distRoot $packageName
$zipPath = Join-Path $distRoot "$packageName.zip"
$checksumPath = Join-Path $distRoot "$packageName.sha256.txt"

if (Test-Path -LiteralPath $packageDir) {
    Remove-Item -LiteralPath $packageDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $packageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $packageDir 'assets') | Out-Null

Copy-Item -LiteralPath (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe') -Destination (Join-Path $packageDir 'QingPricePOE2.exe') -Force
$loginFiles = @(
    'QingPriceLogin.exe',
    'QingPriceLogin.exe.config',
    'Microsoft.Web.WebView2.Core.dll',
    'Microsoft.Web.WebView2.Wpf.dll',
    'WebView2Loader.dll'
)
foreach ($loginFile in $loginFiles) {
    Copy-Item -LiteralPath (Join-Path $loginOutput $loginFile) -Destination (Join-Path $packageDir $loginFile) -Force
}
Copy-Item -LiteralPath (Join-Path $rootPath 'first_run_wizard.ps1') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'control_center.ps1') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'history_gui.ps1') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'set_cookie_gui.ps1') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'settings_gui.ps1') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'support_bundle.ps1') -Destination (Join-Path $packageDir 'SupportBundle.ps1') -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'README.md') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'CHANGELOG.md') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'SUPPORT.md') -Destination $packageDir -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'LICENSE') -Destination $packageDir -Force -ErrorAction SilentlyContinue
Copy-Item -LiteralPath (Join-Path $rootPath 'assets\app.ico') -Destination (Join-Path $packageDir 'assets\app.ico') -Force
Copy-Item -LiteralPath (Join-Path $rootPath 'assets\app_256.png') -Destination (Join-Path $packageDir 'assets\app_256.png') -Force

$buildTime = [DateTimeOffset]::UtcNow.ToString('yyyy-MM-dd HH:mm:ss UTC')
Set-Content -LiteralPath (Join-Path $packageDir 'VERSION.txt') -Encoding UTF8 -Value @"
清价 POE2 国服查价
version: $version
build_time: $buildTime

start_here: StartHere.bat
start_here_zh: 开始使用.bat
control_center: ControlCenter.bat
start: Start.bat / 启动查价.bat / QingPricePOE2.exe
first_run: FirstRun.bat
login_poc: QingPriceLogin.exe
first_run_zh: 首次向导.bat
settings: Settings.bat
settings_zh: 常用设置.bat
cookie_zh: 设置Cookie.bat
history_zh: 查询历史.bat
self_check: SelfCheck.bat
update_check: CheckUpdate.bat
update_check_zh: 检查更新.bat
diagnostics: Diagnostics.bat
support_bundle: SupportBundle.bat
support_bundle_zh: 生成支持包.bat
reset_data: ResetData.bat
uninstall: Uninstall.bat

隐私说明:
- Cookie 使用 Windows DPAPI 加密保存到当前 Windows 用户。
- selfcheck.txt / diagnostics.txt / 崩溃报告不应包含明文 Cookie。
"@

Set-Content -LiteralPath (Join-Path $packageDir 'Start.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
start "" "%~dp0QingPricePOE2.exe"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'ControlCenter.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0control_center.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'StartHere.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0control_center.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'FirstRun.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0first_run_wizard.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'SetCookie.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0set_cookie_gui.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'ValidateCookie.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
"%~dp0QingPricePOE2.exe" --validate-cookie
if errorlevel 1 (
  echo Cookie validation failed. Please run SetCookie.bat again.
) else (
  echo Cookie validation passed.
)
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'Settings.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0settings_gui.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'ClearCookie.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
"%~dp0QingPricePOE2.exe" --clear-cookie
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'Diagnostics.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
"%~dp0QingPricePOE2.exe" --diagnostics "%~dp0diagnostics.txt"
echo.
echo Diagnostics exported to "%~dp0diagnostics.txt"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'SupportBundle.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0SupportBundle.ps1" -Root "%~dp0"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'SelfCheck.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
"%~dp0QingPricePOE2.exe" --self-check "%~dp0selfcheck.txt"
echo.
echo Self-check exported to "%~dp0selfcheck.txt"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'CheckUpdate.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
"%~dp0QingPricePOE2.exe" --check-update "%~dp0update-check.txt"
echo.
echo Update check exported to "%~dp0update-check.txt"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'History.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0history_gui.ps1" -Root "%~dp0"
'@

Set-Content -LiteralPath (Join-Path $packageDir '开始使用.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0StartHere.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '启动查价.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0Start.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '首次向导.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0FirstRun.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '设置Cookie.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0SetCookie.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '常用设置.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0Settings.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '查询历史.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0History.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '运行自检.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0SelfCheck.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '检查更新.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0CheckUpdate.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir '生成支持包.bat') -Encoding ASCII -Value @'
@echo off
cd /d "%~dp0"
call "%~dp0SupportBundle.bat"
'@

Set-Content -LiteralPath (Join-Path $packageDir 'InstallShortcut.ps1') -Encoding UTF8 -Value @'
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$shell = New-Object -ComObject WScript.Shell
$desktop = [Environment]::GetFolderPath('Desktop')
$shortcut = $shell.CreateShortcut((Join-Path $desktop '清价 POE2.lnk'))
$shortcut.TargetPath = Join-Path $root 'QingPricePOE2.exe'
$shortcut.WorkingDirectory = $root
$shortcut.IconLocation = (Join-Path $root 'assets\app.ico')
$shortcut.Hotkey = 'F7'
$shortcut.Description = '清价 POE2 国服查价'
$shortcut.Save()
Write-Host 'Desktop shortcut created: 清价 POE2.lnk'
'@

Set-Content -LiteralPath (Join-Path $packageDir 'InstallShortcut.bat') -Encoding ASCII -Value @'
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0InstallShortcut.ps1"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'ResetData.ps1') -Encoding UTF8 -Value @'
$ErrorActionPreference = 'Stop'
$appDir = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
Write-Host 'This will close QingPrice POE2 and remove local config, cookie, history, logs, self-check, diagnostics and crash reports.'
Write-Host 'It will NOT delete this program folder.'
$confirm = Read-Host 'Type RESET to continue'
if ($confirm -ne 'RESET') {
    Write-Host 'Cancelled.'
    exit 0
}
Get-Process QingPricePOE2,poe2_cn_price_bridge -ErrorAction SilentlyContinue | Stop-Process -Force
if (Test-Path -LiteralPath $appDir) {
    Remove-Item -LiteralPath $appDir -Recurse -Force
    Write-Host "Removed local data: $appDir"
} else {
    Write-Host "Local data folder does not exist: $appDir"
}
Write-Host 'Reset complete. Run FirstRun.bat before using the tool again.'
'@

Set-Content -LiteralPath (Join-Path $packageDir 'ResetData.bat') -Encoding ASCII -Value @'
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0ResetData.ps1"
pause
'@

Set-Content -LiteralPath (Join-Path $packageDir 'Uninstall.ps1') -Encoding UTF8 -Value @'
$ErrorActionPreference = 'Stop'
$appDir = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
$desktop = [Environment]::GetFolderPath('Desktop')
$shortcuts = @(
    (Join-Path $desktop '清价 POE2.lnk'),
    (Join-Path $desktop 'POE2-CN-Price.lnk')
)
Write-Host 'This will close QingPrice POE2, remove desktop shortcuts and remove local app data.'
Write-Host 'After this finishes, you can delete this extracted program folder manually.'
$confirm = Read-Host 'Type UNINSTALL to continue'
if ($confirm -ne 'UNINSTALL') {
    Write-Host 'Cancelled.'
    exit 0
}
Get-Process QingPricePOE2,poe2_cn_price_bridge -ErrorAction SilentlyContinue | Stop-Process -Force
foreach ($shortcut in $shortcuts) {
    if (Test-Path -LiteralPath $shortcut) {
        Remove-Item -LiteralPath $shortcut -Force
        Write-Host "Removed shortcut: $shortcut"
    }
}
if (Test-Path -LiteralPath $appDir) {
    Remove-Item -LiteralPath $appDir -Recurse -Force
    Write-Host "Removed local data: $appDir"
}
Write-Host 'Uninstall cleanup complete.'
'@

Set-Content -LiteralPath (Join-Path $packageDir 'Uninstall.bat') -Encoding ASCII -Value @'
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Uninstall.ps1"
pause
'@

foreach ($script in Get-ChildItem -LiteralPath $packageDir -Filter *.ps1 -File) {
    $text = [System.IO.File]::ReadAllText($script.FullName)
    [System.IO.File]::WriteAllText($script.FullName, $text, [System.Text.UTF8Encoding]::new($true))
}

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
if (Test-Path -LiteralPath $checksumPath) {
    Remove-Item -LiteralPath $checksumPath -Force
}
$items = Get-ChildItem -LiteralPath $packageDir -Force
Compress-Archive -LiteralPath $items.FullName -DestinationPath $zipPath -Force
$hash = Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath
Set-Content -LiteralPath $checksumPath -Encoding ASCII -Value "$($hash.Hash)  $([IO.Path]::GetFileName($zipPath))"

Write-Host "Package: $packageDir"
Write-Host "Zip: $zipPath"
Write-Host "SHA256: $checksumPath"
