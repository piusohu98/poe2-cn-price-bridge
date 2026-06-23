param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$exeCandidates = @(
    (Join-Path $rootPath 'QingPricePOE2.exe'),
    (Join-Path $rootPath 'poe2_cn_price_bridge.exe'),
    (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
$tradeHome = 'https://poe.game.qq.com/trade2'
$cookieHelp = @"
1. 打开 https://poe.game.qq.com/trade2 并登录。
2. 按 F12 打开开发者工具。
3. 进入 Application/应用 -> Cookies -> https://poe.game.qq.com。
4. 找到 POESESSID，复制 Value 的值。
5. 回到清价窗口，点击“从剪贴板识别”或直接粘贴后保存。
"@

function Set-Status($text, $kind = 'info') {
    $script:status.Text = $text
    switch ($kind) {
        'ok' { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172) }
        'error' { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113) }
        default { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138) }
    }
}

function Test-CookieInput($text) {
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    if ($text -match 'POESESSID\s*=') {
        return $true
    }
    if ($text.Trim() -match '^[A-Za-z0-9_%\-]{12,}$') {
        return $true
    }
    return $false
}

function New-Label($text, $x, $y, $w, $h, $size = 9, $bold = $false, $color = $null) {
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $text
    $label.AutoSize = $false
    $label.Location = New-Object System.Drawing.Point($x, $y)
    $label.Size = New-Object System.Drawing.Size($w, $h)
    $style = if ($bold) { [System.Drawing.FontStyle]::Bold } else { [System.Drawing.FontStyle]::Regular }
    $label.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', $size, $style)
    $label.ForeColor = if ($color) { $color } else { [System.Drawing.Color]::FromArgb(203, 213, 225) }
    $form.Controls.Add($label)
    return $label
}

function New-Button($text, $x, $y, $w, $h = 34, $primary = $false) {
    $button = New-Object System.Windows.Forms.Button
    $button.Text = $text
    $button.Location = New-Object System.Drawing.Point($x, $y)
    $button.Size = New-Object System.Drawing.Size($w, $h)
    $button.FlatStyle = 'Flat'
    $button.FlatAppearance.BorderSize = 1
    if ($primary) {
        $button.BackColor = [System.Drawing.Color]::FromArgb(31, 91, 72)
        $button.FlatAppearance.BorderColor = [System.Drawing.Color]::FromArgb(52, 211, 153)
        $button.ForeColor = [System.Drawing.Color]::FromArgb(220, 252, 231)
    } else {
        $button.BackColor = [System.Drawing.Color]::FromArgb(24, 28, 36)
        $button.FlatAppearance.BorderColor = [System.Drawing.Color]::FromArgb(58, 65, 78)
        $button.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
    }
    $form.Controls.Add($button)
    return $button
}

function Save-And-ValidateCookie {
    if (-not $exe -or -not (Test-Path -LiteralPath $exe)) {
        Set-Status '找不到 QingPricePOE2.exe，请确认从完整发布包中运行。' 'error'
        return
    }
    if (-not (Test-CookieInput $box.Text)) {
        Set-Status '没有识别到 POESESSID。请粘贴 POESESSID 值或完整 Cookie 请求头。' 'error'
        return
    }

    $tmp = [System.IO.Path]::GetTempFileName()
    try {
        [System.IO.File]::WriteAllText($tmp, $box.Text, [System.Text.UTF8Encoding]::new($false))
        Set-Status '正在保存 Cookie...'
        $form.Refresh()
        $process = Start-Process -FilePath $exe -ArgumentList @('--set-cookie-file', $tmp) -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Set-Status "保存失败，退出码: $($process.ExitCode)" 'error'
            return
        }

        Set-Status '已保存，正在请求国服 trade2 验证...'
        $form.Refresh()
        $validation = Start-Process -FilePath $exe -ArgumentList @('--validate-cookie') -Wait -PassThru
        if ($validation.ExitCode -eq 0) {
            $start.Enabled = $true
            Set-Status 'Cookie 已保存并验证通过，可以启动工具。' 'ok'
        } else {
            Set-Status '已保存，但验证失败。请重新登录国服市集并复制新的 POESESSID。' 'error'
        }
    } finally {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
}

$form = New-Object System.Windows.Forms.Form
$form.Text = '清价 POE2 - Cookie 设置'
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(720, 470)
$form.BackColor = [System.Drawing.Color]::FromArgb(13, 16, 21)
$form.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 9)
$iconPath = Join-Path $rootPath 'assets\app.ico'
if (Test-Path -LiteralPath $iconPath) {
    $form.Icon = New-Object System.Drawing.Icon($iconPath)
}

New-Label '保存国服 POESESSID' 24 18 420 34 16 $true ([System.Drawing.Color]::FromArgb(134, 239, 172)) | Out-Null
New-Label 'Cookie 只会用 Windows DPAPI 加密保存到当前 Windows 用户，本窗口不会把明文写入日志。' 26 54 660 24 9 $false ([System.Drawing.Color]::FromArgb(148, 163, 184)) | Out-Null

New-Label '获取步骤' 26 98 250 24 10 $true ([System.Drawing.Color]::FromArgb(226, 232, 240)) | Out-Null
New-Label "1. 打开国服市集并登录`r`n2. F12 -> Application/应用 -> Cookies`r`n3. 选择 https://poe.game.qq.com`r`n4. 复制 POESESSID 的 Value" 28 130 265 118 9 $false ([System.Drawing.Color]::FromArgb(203, 213, 225)) | Out-Null

$open = New-Button '打开登录页' 28 268 120 34 $true
$copyHelp = New-Button '复制步骤' 158 268 100
$paste = New-Button '从剪贴板识别' 28 314 230 34
$clear = New-Button '清空输入' 28 360 100

New-Label '粘贴 POESESSID 值或完整 Cookie 请求头' 320 98 360 24 10 $true ([System.Drawing.Color]::FromArgb(226, 232, 240)) | Out-Null
$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.ScrollBars = 'Vertical'
$box.Location = New-Object System.Drawing.Point(320, 128)
$box.Size = New-Object System.Drawing.Size(370, 190)
$box.BackColor = [System.Drawing.Color]::FromArgb(9, 12, 18)
$box.ForeColor = [System.Drawing.Color]::FromArgb(248, 250, 252)
$box.BorderStyle = 'FixedSingle'
$form.Controls.Add($box)

$save = New-Button '保存并验证' 320 336 130 36 $true
$validate = New-Button '验证现有 Cookie' 462 336 130 36
$start = New-Button '启动工具' 604 336 86 36 $true
$start.Enabled = $false
$close = New-Button '关闭' 604 384 86 34

$status = New-Label '准备就绪。可以先打开登录页，复制 POESESSID 后回到这里保存。' 24 426 666 28 9 $false ([System.Drawing.Color]::FromArgb(253, 230, 138))

$open.Add_Click({
    Start-Process $tradeHome
    Set-Status '登录完成后复制 POESESSID 的 Value，再回到这里点击“从剪贴板识别”。'
})

$copyHelp.Add_Click({
    [System.Windows.Forms.Clipboard]::SetText($cookieHelp)
    Set-Status '已复制 Cookie 获取步骤。'
})

$paste.Add_Click({
    if (-not [System.Windows.Forms.Clipboard]::ContainsText()) {
        Set-Status '剪贴板没有文本。' 'error'
        return
    }
    $clip = [System.Windows.Forms.Clipboard]::GetText()
    $box.Text = $clip
    if (Test-CookieInput $clip) {
        Set-Status '已识别到可能的 POESESSID，可以保存并验证。' 'ok'
    } else {
        Set-Status '已粘贴，但没有明显识别到 POESESSID。请确认复制的是 Value 或完整 Cookie。' 'error'
    }
})

$clear.Add_Click({
    $box.Clear()
    $start.Enabled = $false
    Set-Status '已清空输入。'
})

$save.Add_Click({ Save-And-ValidateCookie })

$validate.Add_Click({
    if (-not $exe -or -not (Test-Path -LiteralPath $exe)) {
        Set-Status '找不到 QingPricePOE2.exe。' 'error'
        return
    }
    Set-Status '正在验证现有 Cookie...'
    $form.Refresh()
    $validation = Start-Process -FilePath $exe -ArgumentList @('--validate-cookie') -Wait -PassThru
    if ($validation.ExitCode -eq 0) {
        $start.Enabled = $true
        Set-Status '现有 Cookie 验证通过。' 'ok'
    } else {
        Set-Status '现有 Cookie 不可用，请重新登录并保存新的 POESESSID。' 'error'
    }
})

$start.Add_Click({
    if ($exe -and (Test-Path -LiteralPath $exe)) {
        Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
        Set-Status '工具已启动。进游戏悬停物品按 Ctrl+C 即可查价。' 'ok'
    }
})

$close.Add_Click({ $form.Close() })

$form.Add_Shown({
    if ([System.Windows.Forms.Clipboard]::ContainsText()) {
        $clip = [System.Windows.Forms.Clipboard]::GetText()
        if (Test-CookieInput $clip) {
            $box.Text = $clip
            Set-Status '已自动识别剪贴板里的 POESESSID，可以直接保存并验证。' 'ok'
        }
    }
})

[void] $form.ShowDialog()
