param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$assets = Join-Path $Root 'assets'
New-Item -ItemType Directory -Force -Path $assets | Out-Null
$sourceImagePath = Join-Path $assets 'app_source.png'
if (-not (Test-Path -LiteralPath $sourceImagePath)) {
    throw "Icon source image is missing: $sourceImagePath"
}

function New-IconPngBytes {
    param([int] $Size)

    # 从统一源图高质量缩放，确保预览图和嵌入 EXE 的各尺寸图标一致。
    $source = [System.Drawing.Image]::FromFile($sourceImagePath)
    $bitmap = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.DrawImage($source, 0, 0, $Size, $Size)

    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    [byte[]] $bytes = $stream.ToArray()

    $graphics.Dispose()
    $bitmap.Dispose()
    $source.Dispose()
    $stream.Dispose()

    return ,$bytes
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$images = foreach ($size in $sizes) {
    [pscustomobject]@{
        Size = $size
        Bytes = [byte[]](New-IconPngBytes -Size $size)
    }
}

$ico = Join-Path $assets 'app.ico'
$writer = New-Object System.IO.BinaryWriter ([System.IO.File]::Open($ico, [System.IO.FileMode]::Create))
try {
    $writer.Write([UInt16]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]$images.Count)
    $offset = 6 + ($images.Count * 16)
    foreach ($image in $images) {
        $sizeByte = if ($image.Size -ge 256) { 0 } else { [byte]$image.Size }
        $writer.Write([byte]$sizeByte)
        $writer.Write([byte]$sizeByte)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$image.Bytes.Length)
        $writer.Write([UInt32]$offset)
        $offset += $image.Bytes.Length
    }
    foreach ($image in $images) {
        [byte[]] $bytes = $image.Bytes
        $writer.Write($bytes)
    }
} finally {
    $writer.Dispose()
}

[byte[]] $previewBytes = ($images | Where-Object Size -eq 256 | Select-Object -First 1).Bytes
[System.IO.File]::WriteAllBytes((Join-Path $assets 'app_256.png'), $previewBytes)
Write-Host "Generated $ico"
