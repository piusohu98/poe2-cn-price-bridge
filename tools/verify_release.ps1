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
    '开始使用.bat',
    'ControlCenter.bat',
    'Start.bat',
    '启动查价.bat',
    'FirstRun.bat',
    '首次向导.bat',
    'SetCookie.bat',
    '设置Cookie.bat',
    'ValidateCookie.bat',
    'Settings.bat',
    '常用设置.bat',
    'History.bat',
    '查询历史.bat',
    'SelfCheck.bat',
    '运行自检.bat',
    'CheckUpdate.bat',
    '检查更新.bat',
    'Diagnostics.bat',
    'SupportBundle.bat',
    '生成支持包.bat',
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
Assert-Ok ($versionText -match 'start_here_zh:\s*开始使用\.bat') "VERSION.txt lists Chinese start here"
Assert-Ok ($versionText -match 'control_center:\s*ControlCenter\.bat') "VERSION.txt lists control center"
Assert-Ok ($versionText -match 'cookie_zh:\s*设置Cookie\.bat') "VERSION.txt lists Chinese cookie setup"
Assert-Ok ($versionText -match 'history_zh:\s*查询历史\.bat') "VERSION.txt lists Chinese history"
Assert-Ok ($versionText -match 'self_check:\s*SelfCheck\.bat') "VERSION.txt lists self-check"
Assert-Ok ($versionText -match 'update_check:\s*CheckUpdate\.bat') "VERSION.txt lists update check"
Assert-Ok ($versionText -match 'update_check_zh:\s*检查更新\.bat') "VERSION.txt lists Chinese update check"
Assert-Ok ($versionText -match 'support_bundle:\s*SupportBundle\.bat') "VERSION.txt lists support bundle"
Assert-Ok ($versionText -match 'support_bundle_zh:\s*生成支持包\.bat') "VERSION.txt lists Chinese support bundle"
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
    $uiPath = Join-Path $packageDir $uiScript
    $uiBytes = [System.IO.File]::ReadAllBytes($uiPath)
    Assert-Ok ($uiBytes.Length -gt 3 -and $uiBytes[0] -eq 0xEF -and $uiBytes[1] -eq 0xBB -and $uiBytes[2] -eq 0xBF) "$uiScript is UTF-8 BOM for Windows PowerShell"
    $uiText = Get-Content -LiteralPath $uiPath -Raw
    Assert-Ok ($uiText -match 'PresentationFramework') "$uiScript uses WPF"
    Assert-Ok (-not ($uiText -match 'System\.Windows\.Forms|DataGridView|FormBorderStyle|ClientSize')) "$uiScript does not use legacy WinForms UI"
}

$windowsPowerShellParser = Join-Path $env:TEMP ("qingprice-ps5-parse-" + [guid]::NewGuid().ToString('N') + '.ps1')
[System.IO.File]::WriteAllText($windowsPowerShellParser, @'
param(
    [Parameter(Mandatory = $true)]
    [string] $Path
)

$tokens = $null
$parseErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile($Path, [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($null -ne $parseErrors -and $parseErrors.Count -gt 0) {
    foreach ($parseError in $parseErrors) {
        Write-Host "$($parseError.Extent.StartLineNumber):$($parseError.Extent.StartColumnNumber) $($parseError.Message)"
    }
    exit 1
}
'@, [System.Text.UTF8Encoding]::new($true))
try {
    foreach ($uiScript in $customerUiScripts) {
        $uiPath = Join-Path $packageDir $uiScript
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $windowsPowerShellParser -Path $uiPath | Out-Host
        Assert-Ok ($LASTEXITCODE -eq 0) "$uiScript parses in Windows PowerShell"
    }
} finally {
    Remove-Item -LiteralPath $windowsPowerShellParser -Force -ErrorAction SilentlyContinue
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

$controlCenter = Get-Content -LiteralPath (Join-Path $packageDir 'control_center.ps1') -Raw
Assert-Ok ($controlCenter -match '--check-update') "control center can run update check"

foreach ($generated in @('selfcheck.txt', 'diagnostics.txt', 'update-check.txt')) {
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
    $selfCheck = Get-Content -LiteralPath $selfCheckPath -Raw -Encoding UTF8
    Assert-Ok ($selfCheck -match 'cookie_saved:\s*false') "self-check uses isolated APPDATA"
    Assert-Ok (-not ($selfCheck -match 'POESESSID\s*=\s*[A-Za-z0-9_%\-]{8,}|Cookie:\s*[^\r\n]*POESESSID\s*=')) "self-check does not expose plain cookie"
    Assert-Ok ($selfCheck -match 'StartHere\.bat:\s*ok') "self-check verifies StartHere"
    Assert-Ok ($selfCheck -match '开始使用\.bat:\s*ok') "self-check verifies Chinese StartHere"
    Assert-Ok ($selfCheck -match 'control_center\.ps1:\s*ok') "self-check verifies control center"
    Assert-Ok ($selfCheck -match '查询历史\.bat:\s*ok') "self-check verifies Chinese history"
    Assert-Ok ($selfCheck -match 'CheckUpdate\.bat:\s*ok') "self-check verifies update check"
    Assert-Ok ($selfCheck -match '检查更新\.bat:\s*ok') "self-check verifies Chinese update check"
    Assert-Ok ($selfCheck -match 'SupportBundle\.bat:\s*ok') "self-check verifies support bundle"
    Assert-Ok ($selfCheck -match '生成支持包\.bat:\s*ok') "self-check verifies Chinese support bundle"
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
    foreach ($uiScript in $customerUiScripts) {
        $uiPath = Join-Path $packageDir $uiScript
        $process = Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File', $uiPath, '-Root', $packageDir) -PassThru
        Start-Sleep -Milliseconds 1400
        try {
            Assert-Ok (-not $process.HasExited) "$uiScript WPF window launches"
        } finally {
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            }
        }
    }

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
