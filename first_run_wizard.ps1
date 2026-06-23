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
$settingsScript = Join-Path $rootPath 'settings_gui.ps1'

$form = New-Object System.Windows.Forms.Form
$form.Text = '清价 POE2 - 首次使用向导'
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(720, 520)
$form.BackColor = [System.Drawing.Color]::FromArgb(14, 17, 22)
$form.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 9)

function New-Button($text, $x, $y, $w, $primary = $false) {
    $button = New-Object System.Windows.Forms.Button
    $button.Text = $text
    $button.Location = New-Object System.Drawing.Point($x, $y)
    $button.Size = New-Object System.Drawing.Size($w, 32)
    $button.FlatStyle = 'Flat'
    $button.FlatAppearance.BorderSize = 1
    if ($primary) {
        $button.BackColor = [System.Drawing.Color]::FromArgb(31, 91, 72)
        $button.FlatAppearance.BorderColor = [System.Drawing.Color]::FromArgb(52, 211, 153)
        $button.ForeColor = [System.Drawing.Color]::FromArgb(220, 252, 231)
    } else {
        $button.BackColor = [System.Drawing.Color]::FromArgb(26, 30, 38)
        $button.FlatAppearance.BorderColor = [System.Drawing.Color]::FromArgb(58, 65, 78)
        $button.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
    }
    $form.Controls.Add($button)
    return $button
}

function New-Label($text, $x, $y, $w, $h = 24, $color = $null, $bold = $false) {
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $text
    $label.AutoSize = $false
    $label.Location = New-Object System.Drawing.Point($x, $y)
    $label.Size = New-Object System.Drawing.Size($w, $h)
    if ($color) { $label.ForeColor = $color } else { $label.ForeColor = [System.Drawing.Color]::FromArgb(203, 213, 225) }
    if ($bold) { $label.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 10, [System.Drawing.FontStyle]::Bold) }
    $form.Controls.Add($label)
    return $label
}

function Set-StepState($label, $state) {
    switch ($state) {
        'done' {
            $label.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172)
            $label.Text = $label.Text -replace '^[○●✓!]\s*', '✓ '
        }
        'active' {
            $label.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138)
            $label.Text = $label.Text -replace '^[○●✓!]\s*', '● '
        }
        'error' {
            $label.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
            $label.Text = $label.Text -replace '^[○●✓!]\s*', '! '
        }
        default {
            $label.ForeColor = [System.Drawing.Color]::FromArgb(148, 163, 184)
            $label.Text = $label.Text -replace '^[○●✓!]\s*', '○ '
        }
    }
}

$title = New-Label '清价 POE2 首次使用向导' 24 18 650 32 ([System.Drawing.Color]::FromArgb(134, 239, 172)) $true
$title.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 15, [System.Drawing.FontStyle]::Bold)

New-Label '按顺序完成登录、保存 Cookie 和验证。完成后直接启动工具，进游戏悬停物品按 Ctrl+C。' 24 54 660 36 ([System.Drawing.Color]::FromArgb(148, 163, 184)) | Out-Null

$step1 = New-Label '○ 1. 打开国服市集并登录' 30 108 260 28 $null $true
$step2 = New-Label '○ 2. 粘贴 POESESSID 或 Cookie' 30 146 260 28 $null $true
$step3 = New-Label '○ 3. 保存并验证 Cookie' 30 184 260 28 $null $true
$step4 = New-Label '○ 4. 启动查价工具' 30 222 260 28 $null $true
Set-StepState $step1 'active'

New-Label 'Cookie 内容' 330 104 320 22 ([System.Drawing.Color]::FromArgb(148, 163, 184)) | Out-Null
$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.ScrollBars = 'Vertical'
$box.Location = New-Object System.Drawing.Point(330, 130)
$box.Size = New-Object System.Drawing.Size(360, 160)
$box.BackColor = [System.Drawing.Color]::FromArgb(9, 12, 18)
$box.ForeColor = [System.Drawing.Color]::FromArgb(248, 250, 252)
$box.BorderStyle = 'FixedSingle'
$form.Controls.Add($box)

$help = New-Label '浏览器登录后按 F12 -> Application/应用 -> Cookies -> https://poe.game.qq.com，复制 POESESSID 的值。也可以粘贴完整 Cookie: 请求头。' 330 300 360 62 ([System.Drawing.Color]::FromArgb(148, 163, 184))

$status = New-Label '' 24 458 666 42 ([System.Drawing.Color]::FromArgb(253, 230, 138))

$open = New-Button '打开登录页' 30 278 130 $true
$open.Add_Click({
    Start-Process 'https://poe.game.qq.com/trade2'
    Set-StepState $step1 'done'
    Set-StepState $step2 'active'
    $status.Text = '登录完成后复制 POESESSID，再回到这里粘贴。'
})

$paste = New-Button '从剪贴板粘贴' 170 278 130
$paste.Add_Click({
    if ([System.Windows.Forms.Clipboard]::ContainsText()) {
        $box.Text = [System.Windows.Forms.Clipboard]::GetText()
        Set-StepState $step2 'done'
        Set-StepState $step3 'active'
        $status.Text = '已粘贴。点击“保存并验证”。'
    } else {
        $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
        $status.Text = '剪贴板没有文本。'
    }
})

$save = New-Button '保存并验证' 330 376 130 $true
$settings = New-Button '打开设置' 470 376 100
$start = New-Button '启动工具' 580 376 110 $true
$start.Enabled = $false

$save.Add_Click({
    if (-not $exe -or -not (Test-Path -LiteralPath $exe)) {
        Set-StepState $step3 'error'
        $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
        $status.Text = '找不到主程序，请确认向导和 exe 在同一个发布包里。'
        return
    }
    if ([string]::IsNullOrWhiteSpace($box.Text)) {
        Set-StepState $step2 'error'
        $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
        $status.Text = '先粘贴 POESESSID 或 Cookie。'
        return
    }
    $tmp = [System.IO.Path]::GetTempFileName()
    try {
        [System.IO.File]::WriteAllText($tmp, $box.Text, [System.Text.UTF8Encoding]::new($false))
        $status.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138)
        $status.Text = '正在保存 Cookie...'
        $form.Refresh()
        $saveProcess = Start-Process -FilePath $exe -ArgumentList @('--set-cookie-file', $tmp) -Wait -PassThru
        if ($saveProcess.ExitCode -ne 0) {
            Set-StepState $step3 'error'
            $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
            $status.Text = "保存失败，退出码: $($saveProcess.ExitCode)"
            return
        }
        $status.Text = '已保存，正在验证...'
        $form.Refresh()
        $validation = Start-Process -FilePath $exe -ArgumentList @('--validate-cookie') -Wait -PassThru
        if ($validation.ExitCode -eq 0) {
            Set-StepState $step3 'done'
            Set-StepState $step4 'active'
            $start.Enabled = $true
            $status.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172)
            $status.Text = 'Cookie 验证通过。现在可以启动工具。'
        } else {
            Set-StepState $step3 'error'
            $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
            $status.Text = '已保存，但验证失败。请确认官网已登录，并重新复制 POESESSID。'
        }
    } finally {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
    }
})

$settings.Add_Click({
    if (Test-Path -LiteralPath $settingsScript) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $settingsScript, '-Root', $rootPath)
    } else {
        $status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113)
        $status.Text = '找不到设置脚本。'
    }
})

$start.Add_Click({
    if ($exe -and (Test-Path -LiteralPath $exe)) {
        Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
        Set-StepState $step4 'done'
        $status.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172)
        $status.Text = '工具已启动。进游戏悬停物品按 Ctrl+C 即可查价。'
    }
})

$close = New-Button '关闭' 600 424 90
$close.Add_Click({ $form.Close() })

[void] $form.ShowDialog()
