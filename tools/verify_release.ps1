param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
    [switch] $RunLaunchSmoke
)

$ErrorActionPreference = 'Stop'
$rootPath = (Resolve-Path -LiteralPath $Root).Path

function Assert-Ok {
    param(
        [bool] $Condition,
        [string] $Message
    )
    if (-not $Condition) {
        throw $Message
    }
    Write-Host "[ok] $Message"
}

$cargoToml = Get-Content -LiteralPath (Join-Path $rootPath 'Cargo.toml') -Raw
if ($cargoToml -notmatch 'version\s*=\s*"([^"]+)"') {
    throw 'Cannot read version from Cargo.toml'
}
$version = $Matches[1]

$packageName = "QingPricePOE2-v$version-windows-x64"
$distRoot = Join-Path $rootPath 'dist'
$packageDir = Join-Path $distRoot $packageName
$zipPath = Join-Path $distRoot "$packageName.zip"
$checksumPath = Join-Path $distRoot "$packageName.sha256.txt"
$exePath = Join-Path $packageDir 'QingPricePOE2.exe'

Assert-Ok (Test-Path -LiteralPath $packageDir) "package directory exists"
Assert-Ok (Test-Path -LiteralPath $zipPath) "zip exists"
Assert-Ok (Test-Path -LiteralPath $checksumPath) "sha256 file exists"
Assert-Ok (Test-Path -LiteralPath $exePath) "exe exists"

$requiredFiles = @(
    'QingPricePOE2.exe',
    'StartHere.bat',
    'ControlCenter.bat',
    'Start.bat',
    'FirstRun.bat',
    'SetCookie.bat',
    'ValidateCookie.bat',
    'Settings.bat',
    'History.bat',
    'SelfCheck.bat',
    'Diagnostics.bat',
    'SupportBundle.bat',
    'SupportBundle.ps1',
    'ResetData.bat',
    'ResetData.ps1',
    'Uninstall.bat',
    'Uninstall.ps1',
    'InstallShortcut.bat',
    'InstallShortcut.ps1',
    'ClearCookie.bat',
    'README.md',
    'CHANGELOG.md',
    'SUPPORT.md',
    'VERSION.txt',
    'LICENSE',
    'first_run_wizard.ps1',
    'set_cookie_gui.ps1',
    'settings_gui.ps1',
    'history_gui.ps1',
    'control_center.ps1',
    'assets\app.ico',
    'assets\app_256.png'
)

foreach ($relative in $requiredFiles) {
    Assert-Ok (Test-Path -LiteralPath (Join-Path $packageDir $relative)) "package contains $relative"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $zipEntries = @{}
    foreach ($entry in $zip.Entries) {
        $zipEntries[$entry.FullName.Replace('/', '\')] = $true
    }
    foreach ($relative in $requiredFiles) {
        Assert-Ok $zipEntries.ContainsKey($relative) "zip contains $relative"
    }
} finally {
    $zip.Dispose()
}

$expectedHash = (Get-Content -LiteralPath $checksumPath -Raw).Trim().Split()[0]
$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath).Hash
Assert-Ok ($actualHash -eq $expectedHash) "sha256 matches zip"

$versionText = Get-Content -LiteralPath (Join-Path $packageDir 'VERSION.txt') -Raw
Assert-Ok ($versionText -match "version:\s*$([regex]::Escape($version))") "VERSION.txt has package version"
Assert-Ok ($versionText -match 'start_here:\s*StartHere\.bat') "VERSION.txt lists start here"
Assert-Ok ($versionText -match 'control_center:\s*ControlCenter\.bat') "VERSION.txt lists control center"
Assert-Ok ($versionText -match 'self_check:\s*SelfCheck\.bat') "VERSION.txt lists self-check"
Assert-Ok ($versionText -match 'support_bundle:\s*SupportBundle\.bat') "VERSION.txt lists support bundle"
Assert-Ok ($versionText -match 'reset_data:\s*ResetData\.bat') "VERSION.txt lists reset"
Assert-Ok ($versionText -match 'uninstall:\s*Uninstall\.bat') "VERSION.txt lists uninstall"

$versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($exePath)
Assert-Ok ($versionInfo.ProductName -eq 'QingPrice POE2') "exe product name is set"
Assert-Ok ($versionInfo.FileDescription -eq 'QingPrice POE2 CN price checker') "exe file description is set"
Assert-Ok ($versionInfo.OriginalFilename -eq 'QingPricePOE2.exe') "exe original filename is set"
Assert-Ok ($versionInfo.FileVersion -eq $version) "exe file version matches Cargo.toml"

Add-Type -AssemblyName System.Drawing
$icon = [System.Drawing.Icon]::ExtractAssociatedIcon($exePath)
try {
    Assert-Ok ($null -ne $icon) "exe associated icon can be extracted"
} finally {
    if ($icon) {
        $icon.Dispose()
    }
}

$parseErrorsFound = New-Object System.Collections.Generic.List[string]
foreach ($file in Get-ChildItem -LiteralPath $packageDir -Filter *.ps1 -File) {
    $tokens = $null
    $parseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($file.FullName, [ref]$tokens, [ref]$parseErrors) | Out-Null
    if ($null -ne $parseErrors -and $parseErrors.Count -gt 0) {
        foreach ($parseError in $parseErrors) {
            $parseErrorsFound.Add("$($file.Name): $($parseError.Extent.StartLineNumber):$($parseError.Extent.StartColumnNumber) $($parseError.Message)")
        }
    }
}
if ($parseErrorsFound.Count -gt 0) {
    $parseErrorsFound | ForEach-Object { Write-Host "[error] $_" }
}
Assert-Ok ($parseErrorsFound.Count -eq 0) "package PowerShell scripts parse"

$customerUiScripts = @(
    'control_center.ps1',
    'first_run_wizard.ps1',
    'set_cookie_gui.ps1',
    'settings_gui.ps1',
    'history_gui.ps1'
)
foreach ($uiScript in $customerUiScripts) {
    $uiText = Get-Content -LiteralPath (Join-Path $packageDir $uiScript) -Raw
    Assert-Ok ($uiText -match 'PresentationFramework') "$uiScript uses WPF"
    Assert-Ok (-not ($uiText -match 'System\.Windows\.Forms|DataGridView|FormBorderStyle|ClientSize')) "$uiScript does not use legacy WinForms UI"
}

$cookieGui = Get-Content -LiteralPath (Join-Path $packageDir 'set_cookie_gui.ps1') -Raw
Assert-Ok ($cookieGui -match 'Test-CookieInput') "cookie setup has clipboard detection"
Assert-Ok ($cookieGui -match 'Add_Shown|add_ContentRendered|Add_ContentRendered') "cookie setup checks clipboard on open"
Assert-Ok ($cookieGui -match '--validate-cookie') "cookie setup can validate existing cookie"
Assert-Ok ($cookieGui -match 'Save-And-ValidateCookie') "cookie setup saves and validates"

$settingsGui = Get-Content -LiteralPath (Join-Path $packageDir 'settings_gui.ps1') -Raw
Assert-Ok ($settingsGui -match 'Apply-DefaultsToForm') "settings can restore defaults"
Assert-Ok ($settingsGui -match 'Save-SettingsFromForm') "settings validates before saving"
Assert-Ok ($settingsGui -match 'Start-Process \$configDir') "settings can open config directory"

foreach ($generated in @('selfcheck.txt', 'diagnostics.txt')) {
    Assert-Ok (-not (Test-Path -LiteralPath (Join-Path $packageDir $generated))) "package has no generated $generated"
}
Assert-Ok (-not (Get-ChildItem -LiteralPath $packageDir -Filter 'support-bundle-*.zip' -File -ErrorAction SilentlyContinue)) "package has no generated support bundle"

$tempRoot = Join-Path $env:TEMP ("qingprice-release-verify-" + [guid]::NewGuid().ToString('N'))
$oldAppData = $env:APPDATA
try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    $env:APPDATA = Join-Path $tempRoot 'appdata'
    New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
    $selfCheckPath = Join-Path $tempRoot 'selfcheck.txt'
    & $exePath --self-check $selfCheckPath | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "--self-check failed with exit code $LASTEXITCODE"
    }
    Assert-Ok (Test-Path -LiteralPath $selfCheckPath) "self-check report is generated"
    $selfCheck = Get-Content -LiteralPath $selfCheckPath -Raw
    Assert-Ok ($selfCheck -match 'cookie_saved:\s*false') "self-check uses isolated APPDATA"
    Assert-Ok (-not ($selfCheck -match 'POESESSID\s*=\s*[A-Za-z0-9_%\-]{8,}|Cookie:\s*[^\r\n]*POESESSID\s*=')) "self-check does not expose plain cookie"
    Assert-Ok ($selfCheck -match 'StartHere\.bat:\s*ok') "self-check verifies StartHere"
    Assert-Ok ($selfCheck -match 'control_center\.ps1:\s*ok') "self-check verifies control center"
    Assert-Ok ($selfCheck -match 'SupportBundle\.bat:\s*ok') "self-check verifies support bundle"
    Assert-Ok ($selfCheck -match 'ResetData\.bat:\s*ok') "self-check verifies reset"
    Assert-Ok ($selfCheck -match 'Uninstall\.bat:\s*ok') "self-check verifies uninstall"
    Assert-Ok ($selfCheck -match 'SUPPORT\.md:\s*ok') "self-check verifies support guide"

    $supportScript = Join-Path $packageDir 'SupportBundle.ps1'
    $supportParseTokens = $null
    $supportParseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($supportScript, [ref]$supportParseTokens, [ref]$supportParseErrors) | Out-Null
    Assert-Ok (($null -eq $supportParseErrors) -or ($supportParseErrors.Count -eq 0)) "support bundle script parses"

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $supportScript -Root $packageDir -NoExplorer | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "SupportBundle.ps1 failed with exit code $LASTEXITCODE"
    }
    $supportZip = Get-ChildItem -LiteralPath $packageDir -Filter 'support-bundle-*.zip' -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    Assert-Ok ($null -ne $supportZip) "support bundle zip is generated"
    $supportExtract = Join-Path $tempRoot 'support-extract'
    Expand-Archive -LiteralPath $supportZip.FullName -DestinationPath $supportExtract -Force
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'selfcheck.txt')) "support bundle contains self-check"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'diagnostics.txt')) "support bundle contains diagnostics"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'VERSION.txt')) "support bundle contains version"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'SUPPORT.md')) "support bundle contains support guide"
    $supportLeak = @(Get-ChildItem -LiteralPath $supportExtract -Recurse -File |
        Select-String -Pattern 'POESESSID\s*=\s*[A-Za-z0-9_%\-]{8,}|Cookie:\s*[^\r\n]*POESESSID\s*=' -CaseSensitive:$false)
    if ($supportLeak.Count -gt 0) {
        $supportLeak | ForEach-Object { Write-Host "[error] possible secret in $($_.Path):$($_.LineNumber)" }
    }
    Assert-Ok ($supportLeak.Count -eq 0) "support bundle does not expose plain cookie"
} finally {
    $env:APPDATA = $oldAppData
    Get-ChildItem -LiteralPath $packageDir -Filter 'support-bundle-*.zip' -File -ErrorAction SilentlyContinue |
        Remove-Item -Force
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

if ($RunLaunchSmoke) {
    Get-Process QingPricePOE2,poe2_cn_price_bridge -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Process -FilePath $exePath -WorkingDirectory $packageDir
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exePath -WorkingDirectory $packageDir
    Start-Sleep -Milliseconds 900
    $procs = @(Get-Process QingPricePOE2 -ErrorAction SilentlyContinue)
    try {
        Assert-Ok ($procs.Count -eq 1) "launch smoke keeps a single process"
    } finally {
        $procs | Stop-Process -Force
    }
}

Write-Host "Release verification passed: $packageName"
