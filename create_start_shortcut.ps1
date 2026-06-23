param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
$rootPath = (Resolve-Path -LiteralPath $Root).Path
$desktop = [Environment]::GetFolderPath('Desktop')
$shortcut = Join-Path $desktop '清价 POE2.lnk'
$exe = Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe'
$fallbackBat = Join-Path $rootPath 'start_bridge.bat'

$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
if (Test-Path -LiteralPath $exe) {
    $link.TargetPath = $exe
    $link.IconLocation = "$exe,0"
} else {
    $link.TargetPath = $fallbackBat
}
$link.WorkingDirectory = $rootPath
$link.Description = '清价 POE2 国服查价'
$link.Hotkey = 'F7'
$link.Save()

Write-Host "Shortcut: $shortcut"
