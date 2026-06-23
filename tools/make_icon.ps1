param(
    [string] $Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$assets = Join-Path $Root 'assets'
New-Item -ItemType Directory -Force -Path $assets | Out-Null

function New-IconPngBytes {
    param([int] $Size)

    $bitmap = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $graphics.Clear([System.Drawing.Color]::Transparent)

    $scale = $Size / 256.0
    function S([float] $Value) { [int][Math]::Round($Value * $scale) }

    $rect = New-Object System.Drawing.Rectangle (S 12), (S 12), (S 232), (S 232)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $radius = S 46
    $path.AddArc($rect.X, $rect.Y, $radius, $radius, 180, 90)
    $path.AddArc($rect.Right - $radius, $rect.Y, $radius, $radius, 270, 90)
    $path.AddArc($rect.Right - $radius, $rect.Bottom - $radius, $radius, $radius, 0, 90)
    $path.AddArc($rect.X, $rect.Bottom - $radius, $radius, $radius, 90, 90)
    $path.CloseFigure()

    $bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, ([System.Drawing.Color]::FromArgb(20,25,34)), ([System.Drawing.Color]::FromArgb(6,8,12)), 45
    $graphics.FillPath($bg, $path)
    $outline = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(36,50,68)), (S 5)
    $graphics.DrawPath($outline, $path)

    $coin = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(216,168,52)), (S 18)
    $coin.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $coin.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $graphics.DrawEllipse($coin, (S 48), (S 48), (S 160), (S 160))

    $jade = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(52,211,153)), (S 18)
    $jade.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $jade.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $jade.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $points = @(
        (New-Object System.Drawing.Point (S 71), (S 145)),
        (New-Object System.Drawing.Point (S 105), (S 111)),
        (New-Object System.Drawing.Point (S 130), (S 136)),
        (New-Object System.Drawing.Point (S 181), (S 85))
    )
    $graphics.DrawLines($jade, $points)
    $graphics.DrawLine($jade, (S 183), (S 85), (S 183), (S 128))
    $graphics.DrawLine($jade, (S 183), (S 128), (S 141), (S 128))

    $font = New-Object System.Drawing.Font 'Microsoft YaHei UI', (S 64), ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(232,255,247))
    $graphics.DrawString(([string][char]0x6E05), $font, $brush, (New-Object System.Drawing.RectangleF 0, (S 146), $Size, (S 72)), $format)

    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    [byte[]] $bytes = $stream.ToArray()

    $graphics.Dispose()
    $bitmap.Dispose()
    $path.Dispose()
    $bg.Dispose()
    $outline.Dispose()
    $coin.Dispose()
    $jade.Dispose()
    $font.Dispose()
    $format.Dispose()
    $brush.Dispose()
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
