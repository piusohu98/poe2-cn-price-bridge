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

# 仅通过本机 WebView2 API 检测 Runtime；验证过程不访问网络。
function Test-WebView2Runtime {
    param(
        [string] $PackageDirectory
    )
    $coreAssemblyPath = Join-Path $PackageDirectory 'Microsoft.Web.WebView2.Core.dll'
    try {
        Add-Type -Path $coreAssemblyPath -ErrorAction Stop
        $runtimeVersion = [Microsoft.Web.WebView2.Core.CoreWebView2Environment]::GetAvailableBrowserVersionString()
        return -not [string]::IsNullOrWhiteSpace($runtimeVersion)
    } catch {
        return $false
    }
}

# 等待 Windows GUI 子系统程序完成，避免 PowerShell 5.1 在产物落盘前继续执行。
function Invoke-BridgeCommand {
    param(
        [string] $FilePath,
        [string[]] $Arguments
    )
    $quotedArguments = @($Arguments | ForEach-Object { '"' + $_.Replace('"', '\"') + '"' })
    $process = Start-Process -FilePath $FilePath -ArgumentList $quotedArguments -Wait -PassThru
    return $process.ExitCode
}

# 通过标准输入调用 Rust 桥接入口，捕获但不打印子进程输出。
function Invoke-BridgeStdinCommand {
    param(
        [string] $FilePath,
        [string] $InputValue
    )
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $FilePath
    $startInfo.Arguments = '--set-cookie-stdin'
    $startInfo.WorkingDirectory = Split-Path -Parent $FilePath
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    try {
        if (-not $process.Start()) {
            throw 'failed to start stdin bridge process'
        }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.StandardInput.Write($InputValue)
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(60000)) {
            & taskkill.exe /PID $process.Id /T /F | Out-Null
            throw 'stdin bridge process timed out'
        }
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            Stdout = $stdoutTask.Result
            Stderr = $stderrTask.Result
        }
    } finally {
        $process.Dispose()
    }
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
$loginExePath = Join-Path $packageDir 'QingPriceLogin.exe'

Assert-Ok (Test-Path -LiteralPath $packageDir) "package directory exists"
Assert-Ok (Test-Path -LiteralPath $zipPath) "zip exists"
Assert-Ok (Test-Path -LiteralPath $checksumPath) "sha256 file exists"
Assert-Ok (Test-Path -LiteralPath $exePath) "exe exists"

$requiredFiles = @(
    'QingPricePOE2.exe',
    'QingPriceLogin.exe',
    'QingPriceLogin.exe.config',
    'Microsoft.Web.WebView2.Core.dll',
    'Microsoft.Web.WebView2.Wpf.dll',
    'WebView2Loader.dll',
    'licenses\Microsoft.Web.WebView2-LICENSE.txt',
    'licenses\Microsoft.Web.WebView2-NOTICE.txt',
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

$webView2Version = '1.0.4078.44'
$webView2RuntimeUrl = 'https://developer.microsoft.com/microsoft-edge/webview2/'
$webView2LicenseHash = '0AF8F1B807512AAE39C2AC1AA4D0CAE65CABECB6FD554B8439A5162A0D6ECA55'
$webView2NoticeHash = '106423785C5B7EBA0A8E61D1837F2132E9C828E20AD530F565D981C1DF60DD90'
$webView2LicensePath = Join-Path $packageDir 'licenses\Microsoft.Web.WebView2-LICENSE.txt'
$webView2NoticePath = Join-Path $packageDir 'licenses\Microsoft.Web.WebView2-NOTICE.txt'
Assert-Ok ((Get-Item -LiteralPath $webView2LicensePath).Length -gt 0) "WebView2 license is non-empty"
Assert-Ok ((Get-Item -LiteralPath $webView2NoticePath).Length -gt 0) "WebView2 notice is non-empty"
Assert-Ok ((Get-FileHash -LiteralPath $webView2LicensePath -Algorithm SHA256).Hash -eq $webView2LicenseHash) "WebView2 license matches locked package"
Assert-Ok ((Get-FileHash -LiteralPath $webView2NoticePath -Algorithm SHA256).Hash -eq $webView2NoticeHash) "WebView2 notice matches locked package"
$loginProjectText = Get-Content -LiteralPath (Join-Path $rootPath 'login\QingPriceLogin\QingPriceLogin.csproj') -Raw -Encoding UTF8
Assert-Ok ($loginProjectText -match ('Microsoft\.Web\.WebView2" Version="\[' + [regex]::Escape($webView2Version) + '\]"')) "WebView2 PackageReference is exact"
$loginLock = Get-Content -LiteralPath (Join-Path $rootPath 'login\QingPriceLogin\packages.lock.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$lockedVersions = @($loginLock.dependencies.PSObject.Properties | ForEach-Object { $_.Value.'Microsoft.Web.WebView2'.resolved } | Select-Object -Unique)
Assert-Ok ($lockedVersions.Count -eq 1 -and $lockedVersions[0] -eq $webView2Version) "WebView2 lock file matches packaged licenses"
if (Test-WebView2Runtime -PackageDirectory $packageDir) {
    Write-Host '[ok] WebView2 Runtime is available on this verification host'
} else {
    Write-Host '[info] WebView2 Runtime is not installed on this verification host; package keeps the official install URL'
}

# 从 PE 头读取 Machine 字段，确保发布包没有混入 x86/ARM64 Loader。
$loaderPath = Join-Path $packageDir 'WebView2Loader.dll'
$loaderStream = [IO.File]::OpenRead($loaderPath)
$loaderReader = New-Object IO.BinaryReader($loaderStream)
try {
    $loaderStream.Position = 0x3c
    $peOffset = $loaderReader.ReadInt32()
    $loaderStream.Position = $peOffset
    $peSignature = $loaderReader.ReadUInt32()
    $peMachine = $loaderReader.ReadUInt16()
} finally {
    $loaderReader.Dispose()
    $loaderStream.Dispose()
}
Assert-Ok ($peSignature -eq 0x00004550) "WebView2Loader.dll has a valid PE signature"
Assert-Ok ($peMachine -eq 0x8664) "WebView2Loader.dll is x64"
$loginSelfTestExit = Invoke-BridgeCommand -FilePath $loginExePath -Arguments @('--self-test')
Assert-Ok ($loginSelfTestExit -eq 0) "login helper offline self-test passes"

$versionText = Get-Content -LiteralPath (Join-Path $packageDir 'VERSION.txt') -Raw
Assert-Ok ($versionText -match "version:\s*$([regex]::Escape($version))") "VERSION.txt has package version"
Assert-Ok ($versionText -match 'start_here:\s*StartHere\.bat') "VERSION.txt lists start here"
Assert-Ok ($versionText -match 'start_here_zh:\s*开始使用\.bat') "VERSION.txt lists Chinese start here"
Assert-Ok ($versionText -match 'login_poc:\s*QingPriceLogin\.exe') "VERSION.txt lists login PoC"
Assert-Ok ($versionText -match ('webview2_runtime:\s*' + [regex]::Escape($webView2RuntimeUrl))) "VERSION.txt lists Microsoft WebView2 Runtime official URL"
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
Assert-Ok ($versionText -match 'third_party_webview2:\s*licenses\\Microsoft\.Web\.WebView2-LICENSE\.txt') "VERSION.txt lists WebView2 license"
Assert-Ok ($versionText -match 'third_party_webview2_notice:\s*licenses\\Microsoft\.Web\.WebView2-NOTICE\.txt') "VERSION.txt lists WebView2 notice"

$versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($exePath)
Assert-Ok ($versionInfo.ProductName -eq 'QingPrice POE2') "exe product name is set"
Assert-Ok ($versionInfo.FileDescription -eq 'QingPrice POE2 CN price checker') "exe file description is set"
Assert-Ok ($versionInfo.OriginalFilename -eq 'QingPricePOE2.exe') "exe original filename is set"
Assert-Ok ($versionInfo.FileVersion -eq $version) "exe file version matches Cargo.toml"
$loginVersionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($loginExePath)
Assert-Ok ($loginVersionInfo.FileVersion -eq "$version.0") "login exe file version matches Cargo.toml"
Assert-Ok ($loginVersionInfo.ProductVersion -match ('^' + [regex]::Escape($version))) "login exe product version matches Cargo.toml"
$loginAssemblyVersion = [Reflection.AssemblyName]::GetAssemblyName($loginExePath).Version.ToString()
Assert-Ok ($loginAssemblyVersion -eq "$version.0") "login exe assembly version matches Cargo.toml"

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
Assert-Ok ($cookieGui -match 'finally\s*\{[\s\S]*Remove-Item -LiteralPath \$tmp') "cookie setup always removes plaintext temp file"

$firstRunWizard = Get-Content -LiteralPath (Join-Path $packageDir 'first_run_wizard.ps1') -Raw
Assert-Ok ($firstRunWizard -match 'finally\s*\{[\s\S]*Remove-Item -LiteralPath \$tmp') "first-run wizard always removes plaintext temp file"
Assert-Ok ($firstRunWizard -match 'QingPriceLogin\.exe') "first-run wizard invokes the login helper"
Assert-Ok ($firstRunWizard -match '微信[\s\S]{0,24}扫码|扫码[\s\S]{0,24}微信') "first-run wizard presents verified WeChat login"
Assert-Ok ($firstRunWizard -match '高级') "first-run wizard keeps manual Cookie as an advanced option"
Assert-Ok ($firstRunWizard -match 'Runtime|运行环境|WebView2') "first-run wizard handles missing WebView2 Runtime"
Assert-Ok ($firstRunWizard -notmatch '--bridge-exe|\$tradeHome|按 F12') "first-run wizard does not use an unbounded or F12 default flow"

$setCookieGui = Get-Content -LiteralPath (Join-Path $packageDir 'set_cookie_gui.ps1') -Raw
Assert-Ok ($setCookieGui -match 'QingPriceLogin\.exe') "Cookie GUI invokes the login helper"
Assert-Ok ($setCookieGui -match '微信[\s\S]{0,24}扫码|扫码[\s\S]{0,24}微信') "Cookie GUI presents verified WeChat login"
Assert-Ok ($setCookieGui -match '高级') "Cookie GUI keeps manual Cookie as an advanced option"
Assert-Ok ($setCookieGui -notmatch '--bridge-exe|\$tradeHome|按 F12') "Cookie GUI does not use an unbounded or F12 default flow"
$loginAppText = Get-Content -LiteralPath (Join-Path $rootPath 'login\QingPriceLogin\App.xaml.cs') -Raw -Encoding UTF8
$loginWindowText = Get-Content -LiteralPath (Join-Path $rootPath 'login\QingPriceLogin\MainWindow.xaml.cs') -Raw -Encoding UTF8
Assert-Ok ($loginAppText -notmatch '--bridge-exe') "login helper does not accept arbitrary bridge paths"
Assert-Ok ($loginWindowText -match 'AppDomain\.CurrentDomain\.BaseDirectory[\s\S]{0,120}QingPricePOE2\.exe') "login helper uses same-directory bridge executable"
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
    $exitCode = Invoke-BridgeCommand -FilePath $exePath -Arguments @('--self-check', $selfCheckPath)
    if ($exitCode -ne 0) {
        throw "--self-check failed with exit code $exitCode"
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

    $syntheticSecret = ('SYNTHETIC_' + 'POESESSID_' + '7f3a91d2')
    $stdinResult = Invoke-BridgeStdinCommand -FilePath $exePath -InputValue $syntheticSecret
    Assert-Ok ($stdinResult.ExitCode -ne 0) "synthetic stdin Cookie is not accepted"
    Assert-Ok (-not $stdinResult.Stdout.Contains($syntheticSecret)) "stdin bridge stdout does not expose synthetic secret"
    Assert-Ok (-not $stdinResult.Stderr.Contains($syntheticSecret)) "stdin bridge stderr does not expose synthetic secret"
    $isolatedConfig = Join-Path $env:APPDATA 'poe2_cn_price_bridge\config.json'
    Assert-Ok (-not (Test-Path -LiteralPath $isolatedConfig)) "failed stdin validation does not save Cookie"

    $secretInput = Join-Path $tempRoot 'synthetic-cookie.txt'
    try {
        [System.IO.File]::WriteAllText($secretInput, $syntheticSecret, [System.Text.UTF8Encoding]::new($false))
        $exitCode = Invoke-BridgeCommand -FilePath $exePath -Arguments @('--set-cookie-file', $secretInput)
        if ($exitCode -ne 0) {
            throw "--set-cookie-file failed with exit code $exitCode"
        }
    } finally {
        Remove-Item -LiteralPath $secretInput -Force -ErrorAction SilentlyContinue
    }

    $appRoot = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
    $logsDir = Join-Path $appRoot 'logs'
    $crashesDir = Join-Path $appRoot 'crashes'
    New-Item -ItemType Directory -Force -Path $logsDir, $crashesDir | Out-Null
    $syntheticPayload = "POESESSID=$syntheticSecret`r`nCookie: foo=bar; POESESSID=$syntheticSecret`r`n{`"POESESSID`":`"$syntheticSecret`"}`r`nknown=$syntheticSecret"
    [System.IO.File]::WriteAllText((Join-Path $logsDir 'app.log'), $syntheticPayload, [System.Text.UTF8Encoding]::new($false))
    $historyLine = "{`"ts`":1,`"status`":`"error`",`"item`":`"test`",`"base_type`":`"test`",`"rarity`":`"rare`",`"league`":null,`"total`":null,`"priced`":null,`"message`":`"Cookie: POESESSID=$syntheticSecret`",`"url`":null,`"used_mods`":false,`"used_values`":false}"
    [System.IO.File]::WriteAllText((Join-Path $appRoot 'history.jsonl'), $historyLine, [System.Text.UTF8Encoding]::new($false))
    [System.IO.File]::WriteAllText((Join-Path $crashesDir 'crash-synthetic.txt'), $syntheticPayload, [System.Text.UTF8Encoding]::new($false))

    $redactionProbe = Join-Path $tempRoot 'redaction-probe.txt'
    [System.IO.File]::WriteAllText($redactionProbe, $syntheticPayload, [System.Text.UTF8Encoding]::new($false))
    $exitCode = Invoke-BridgeCommand -FilePath $exePath -Arguments @('--redact-file', $redactionProbe)
    if ($exitCode -ne 0) {
        throw "--redact-file failed with exit code $exitCode"
    }
    Assert-Ok (-not ((Get-Content -LiteralPath $redactionProbe -Raw -Encoding UTF8).Contains($syntheticSecret))) "redaction removes header, JSON, keyed and known secret formats"

    $secretSelfCheckPath = Join-Path $tempRoot 'selfcheck-with-secret.txt'
    $exitCode = Invoke-BridgeCommand -FilePath $exePath -Arguments @('--self-check', $secretSelfCheckPath)
    if ($exitCode -ne 0) {
        throw "secret --self-check failed with exit code $exitCode"
    }
    Assert-Ok (-not ((Get-Content -LiteralPath $secretSelfCheckPath -Raw -Encoding UTF8).Contains($syntheticSecret))) "self-check redacts synthetic secret"

    $diagnosticsPath = Join-Path $tempRoot 'diagnostics-with-secret.txt'
    $exitCode = Invoke-BridgeCommand -FilePath $exePath -Arguments @('--diagnostics', $diagnosticsPath)
    if ($exitCode -ne 0) {
        throw "--diagnostics failed with exit code $exitCode"
    }
    Assert-Ok (-not ((Get-Content -LiteralPath $diagnosticsPath -Raw -Encoding UTF8).Contains($syntheticSecret))) "diagnostics redacts synthetic secret"

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
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'update-check.txt')) "support bundle contains update check"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'VERSION.txt')) "support bundle contains version"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'SUPPORT.md')) "support bundle contains support guide"
    Assert-Ok (Test-Path -LiteralPath (Join-Path $supportExtract 'crashes\crash-synthetic.txt')) "support bundle contains synthetic crash report"
    $supportLeak = @(Get-ChildItem -LiteralPath $supportExtract -Recurse -File |
        Select-String -Pattern 'POESESSID\s*=\s*[A-Za-z0-9_%\-]{8,}|Cookie:\s*[^\r\n]*POESESSID\s*=' -CaseSensitive:$false)
    if ($supportLeak.Count -gt 0) {
        $supportLeak | ForEach-Object { Write-Host "[error] possible secret in $($_.Path):$($_.LineNumber)" }
    }
    Assert-Ok ($supportLeak.Count -eq 0) "support bundle does not expose plain cookie"
    $syntheticLeaks = @(Get-ChildItem -LiteralPath $supportExtract -Recurse -File |
        Select-String -SimpleMatch $syntheticSecret)
    Assert-Ok ($syntheticLeaks.Count -eq 0) "support bundle contains no synthetic secret"
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
