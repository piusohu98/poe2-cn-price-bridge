param(
    [string] $Root = (Split-Path -Parent $MyInvocation.MyCommand.Path),
    [switch] $NoExplorer
)

$ErrorActionPreference = 'Stop'

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$exeCandidates = @(
    (Join-Path $rootPath 'POE2PriceHelper.exe'),
    (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $exe) {
    throw '找不到 POE2PriceHelper.exe。请先运行 StartHere.bat，或从完整发布包中执行。'
}

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$bundleDir = Join-Path $rootPath "support-bundle-$stamp"
$zipPath = Join-Path $rootPath "support-bundle-$stamp.zip"
$bundleCompleted = $false
try {
    New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null

    Set-Content -LiteralPath (Join-Path $bundleDir 'README.txt') -Encoding UTF8 -Value @"
流放2查价助手客户支持包
生成时间: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')

可发送给维护者的文件:
- selfcheck.txt
- diagnostics.txt
- update-check.txt
- VERSION.txt
- SUPPORT.md

隐私提醒:
- 请不要额外发送明文 POESESSID、完整 Cookie 请求头、手机号、QQ 号或支付信息。
- 本脚本在压缩前会统一脱敏并扫描 POESESSID 和完整 Cookie 请求头。
"@

    $selfCheckPath = Join-Path $bundleDir 'selfcheck.txt'
    $process = Start-Process -FilePath $exe -ArgumentList @('--self-check', "`"$selfCheckPath`"") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "self-check 失败，退出码 $($process.ExitCode)"
    }

    $diagnosticsPath = Join-Path $bundleDir 'diagnostics.txt'
    $process = Start-Process -FilePath $exe -ArgumentList @('--diagnostics', "`"$diagnosticsPath`"") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "diagnostics 失败，退出码 $($process.ExitCode)"
    }

    $updateCheckPath = Join-Path $bundleDir 'update-check.txt'
    $process = Start-Process -FilePath $exe -ArgumentList @('--check-update', "`"$updateCheckPath`"") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "check-update 失败，退出码 $($process.ExitCode)"
    }

    foreach ($name in @('VERSION.txt', 'SUPPORT.md', 'CHANGELOG.md')) {
        $path = Join-Path $rootPath $name
        if (Test-Path -LiteralPath $path) {
            Copy-Item -LiteralPath $path -Destination $bundleDir -Force
        }
    }

    $crashDir = Join-Path (Join-Path $env:APPDATA 'poe2_cn_price_bridge') 'crashes'
    if (Test-Path -LiteralPath $crashDir) {
        $latestCrashes = Get-ChildItem -LiteralPath $crashDir -Filter 'crash-*.txt' -File |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 5
        if ($latestCrashes) {
            $targetCrashDir = Join-Path $bundleDir 'crashes'
            New-Item -ItemType Directory -Force -Path $targetCrashDir | Out-Null
            foreach ($crash in $latestCrashes) {
                Copy-Item -LiteralPath $crash.FullName -Destination $targetCrashDir -Force
            }
        }
    }

    foreach ($file in Get-ChildItem -LiteralPath $bundleDir -Recurse -File) {
        $process = Start-Process -FilePath $exe -ArgumentList @('--redact-file', "`"$($file.FullName)`"") -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            throw "脱敏失败: $($file.FullName)，退出码 $($process.ExitCode)"
        }
    }

    $secretPatterns = @(
        '(?i)POESESSID["'']?\s*[:=](?!\s*["'']?\[REDACTED\])\s*["'']?[^\s;,"''}\r\n]{8,}',
        '(?i)Cookie\s*:(?!\s*\[REDACTED\])\s*[^\r\n]+'
    )
    $leakMatches = Get-ChildItem -LiteralPath $bundleDir -Recurse -File |
        Select-String -Pattern $secretPatterns
    if ($leakMatches) {
        $leakMatches | ForEach-Object {
            Write-Host "[error] 疑似敏感信息: $($_.Path):$($_.LineNumber)"
        }
        throw '支持包生成已取消：检测到疑似明文 Cookie。'
    }

    if (Test-Path -LiteralPath $zipPath) {
        Remove-Item -LiteralPath $zipPath -Force
    }
    $bundleItems = Get-ChildItem -LiteralPath $bundleDir -Force
    Compress-Archive -LiteralPath $bundleItems.FullName -DestinationPath $zipPath -Force
    $bundleCompleted = $true
} finally {
    if (Test-Path -LiteralPath $bundleDir) {
        Remove-Item -LiteralPath $bundleDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $bundleCompleted -and (Test-Path -LiteralPath $zipPath)) {
        Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Support bundle: $zipPath"
if (-not $NoExplorer) {
    Start-Process explorer.exe -ArgumentList "/select,`"$zipPath`""
}
