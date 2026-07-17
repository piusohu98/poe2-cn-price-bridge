param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'
# WebView2 Runtime 不会随应用捆绑；构建阶段只记录官方安装入口，避免发布说明指向第三方下载站。
$webView2RuntimeDownloadUrl = 'https://developer.microsoft.com/microsoft-edge/webview2/'
if ($webView2RuntimeDownloadUrl -notmatch '^https://developer\.microsoft\.com/microsoft-edge/webview2/$') {
    throw 'WebView2 Runtime download URL must remain the Microsoft official page'
}
$rootPath = (Resolve-Path -LiteralPath $Root).Path
$cargoTomlPath = Join-Path $rootPath 'Cargo.toml'
$projectPath = Join-Path $rootPath 'login\QingPriceLogin\QingPriceLogin.csproj'
$cargoToml = Get-Content -LiteralPath $cargoTomlPath -Raw -Encoding UTF8
if ($cargoToml -notmatch '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$') {
    throw 'Cannot read a three-part release version from Cargo.toml'
}
$version = $Matches[1]

# 统一从 Cargo.toml 传入版本，并强制使用已提交的 NuGet 锁文件。
& dotnet restore $projectPath --locked-mode "-p:AppVersion=$version"
if ($LASTEXITCODE -ne 0) {
    throw "POE2PriceLogin locked restore failed with exit code $LASTEXITCODE"
}
& dotnet build $projectPath -c $Configuration --no-restore -warnaserror "-p:AppVersion=$version"
if ($LASTEXITCODE -ne 0) {
    throw "POE2PriceLogin build failed with exit code $LASTEXITCODE"
}

Write-Host "POE2PriceLogin $version $Configuration build passed"
Write-Host "WebView2 Runtime official install: $webView2RuntimeDownloadUrl"
