param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'
$rootPath = (Resolve-Path -LiteralPath $Root).Path
$cargoTomlPath = Join-Path $rootPath 'Cargo.toml'
$projectPath = Join-Path $rootPath 'login\QingPriceLogin\QingPriceLogin.csproj'
$cargoToml = Get-Content -LiteralPath $cargoTomlPath -Raw -Encoding UTF8
if ($cargoToml -notmatch '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$') {
    throw 'Cannot read a three-part release version from Cargo.toml'
}
$version = $Matches[1]

# 统一从 Cargo.toml 传入版本，并强制使用已提交的 NuGet 锁文件。
& dotnet restore $projectPath --locked-mode "-p:QingPriceVersion=$version"
if ($LASTEXITCODE -ne 0) {
    throw "QingPriceLogin locked restore failed with exit code $LASTEXITCODE"
}
& dotnet build $projectPath -c $Configuration --no-restore "-p:QingPriceVersion=$version"
if ($LASTEXITCODE -ne 0) {
    throw "QingPriceLogin build failed with exit code $LASTEXITCODE"
}

Write-Host "QingPriceLogin $version $Configuration build passed"
