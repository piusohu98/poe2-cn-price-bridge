param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$configDir = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
$configPath = Join-Path $configDir 'config.json'
$cookieScript = Join-Path $rootPath 'set_cookie_gui.ps1'
$defaults = @{
    primary_league = '奥杜尔秘符'
    fallback_league = '永久'
    max_fetch_results = 80
    fetch_batch_size = 10
    page_size = 8
    result_timeout_seconds = 16
    auto_clipboard = $true
    manual_hotkey = 'F8'
}

function Get-Config {
    if (Test-Path -LiteralPath $configPath) {
        try {
            return Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
        } catch {
            return [pscustomobject]@{}
        }
    }
    return [pscustomobject]@{}
}

function Ensure-Settings($config) {
    if (-not $config.PSObject.Properties['settings'] -or -not $config.settings) {
        $config | Add-Member -Force -MemberType NoteProperty -Name settings -Value ([pscustomobject]@{})
    }
    foreach ($key in $defaults.Keys) {
        if (-not $config.settings.PSObject.Properties[$key]) {
            $config.settings | Add-Member -Force -MemberType NoteProperty -Name $key -Value $defaults[$key]
        }
    }
    return $config
}

function Save-Config($config) {
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
    $json = $config | ConvertTo-Json -Depth 8
    [System.IO.File]::WriteAllText($configPath, $json, [System.Text.UTF8Encoding]::new($false))
}

$config = Ensure-Settings (Get-Config)

$form = New-Object System.Windows.Forms.Form
$form.Text = '清价 POE2 - 设置'
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(560, 430)
$form.BackColor = [System.Drawing.Color]::FromArgb(18, 20, 24)
$form.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 9)
$iconPath = Join-Path $rootPath 'assets\app.ico'
if (Test-Path -LiteralPath $iconPath) {
    $form.Icon = New-Object System.Drawing.Icon($iconPath)
}

function Add-Label($text, $x, $y, $w = 150) {
    $label = New-Object System.Windows.Forms.Label
    $label.Text = $text
    $label.AutoSize = $false
    $label.Location = New-Object System.Drawing.Point($x, $y)
    $label.Size = New-Object System.Drawing.Size($w, 24)
    $label.ForeColor = [System.Drawing.Color]::FromArgb(148, 163, 184)
    $form.Controls.Add($label)
    return $label
}

function Add-TextBox($text, $x, $y, $w = 250) {
    $box = New-Object System.Windows.Forms.TextBox
    $box.Text = [string]$text
    $box.Location = New-Object System.Drawing.Point($x, $y)
    $box.Size = New-Object System.Drawing.Size($w, 26)
    $box.BackColor = [System.Drawing.Color]::FromArgb(12, 13, 16)
    $box.ForeColor = [System.Drawing.Color]::FromArgb(248, 250, 252)
    $box.BorderStyle = 'FixedSingle'
    $form.Controls.Add($box)
    return $box
}

function Add-Number($value, $x, $y, $min, $max) {
    $num = New-Object System.Windows.Forms.NumericUpDown
    $num.Minimum = $min
    $num.Maximum = $max
    $num.Value = [Math]::Min([Math]::Max([int]$value, $min), $max)
    $num.Location = New-Object System.Drawing.Point($x, $y)
    $num.Size = New-Object System.Drawing.Size(110, 26)
    $num.BackColor = [System.Drawing.Color]::FromArgb(12, 13, 16)
    $num.ForeColor = [System.Drawing.Color]::FromArgb(248, 250, 252)
    $form.Controls.Add($num)
    return $num
}

function Set-Status($text, $kind = 'info') {
    $script:status.Text = $text
    switch ($kind) {
        'ok' { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172) }
        'error' { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(248, 113, 113) }
        default { $script:status.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138) }
    }
}

function Apply-DefaultsToForm {
    $primary.Text = $defaults.primary_league
    $fallback.Text = $defaults.fallback_league
    $maxFetch.Value = $defaults.max_fetch_results
    $batch.Value = $defaults.fetch_batch_size
    $pageSize.Value = $defaults.page_size
    $timeout.Value = $defaults.result_timeout_seconds
    $autoClipboard.Checked = [bool]$defaults.auto_clipboard
    $manualHotkey.SelectedItem = $defaults.manual_hotkey
}

function Save-SettingsFromForm {
    if ([string]::IsNullOrWhiteSpace($primary.Text)) {
        Set-Status '主联赛不能为空，已使用默认赛季。' 'error'
        $primary.Text = $defaults.primary_league
        return $false
    }
    if ([string]::IsNullOrWhiteSpace($fallback.Text)) {
        $fallback.Text = $defaults.fallback_league
    }
    $config.settings.primary_league = $primary.Text.Trim()
    $config.settings.fallback_league = $fallback.Text.Trim()
    $config.settings.max_fetch_results = [int]$maxFetch.Value
    $config.settings.fetch_batch_size = [int]$batch.Value
    $config.settings.page_size = [int]$pageSize.Value
    $config.settings.result_timeout_seconds = [int]$timeout.Value
    $config.settings.auto_clipboard = [bool]$autoClipboard.Checked
    $config.settings.manual_hotkey = [string]$manualHotkey.SelectedItem
    Save-Config $config
    Set-Status '已保存。运行中的工具会自动读取新设置。' 'ok'
    return $true
}

$title = New-Object System.Windows.Forms.Label
$title.Text = '客户常用设置'
$title.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 13, [System.Drawing.FontStyle]::Bold)
$title.AutoSize = $false
$title.Location = New-Object System.Drawing.Point(18, 16)
$title.Size = New-Object System.Drawing.Size(520, 30)
$title.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172)
$form.Controls.Add($title)

Add-Label '主联赛' 22 64 | Out-Null
$primary = Add-TextBox $config.settings.primary_league 160 62

Add-Label '备用联赛' 22 102 | Out-Null
$fallback = Add-TextBox $config.settings.fallback_league 160 100

Add-Label '最多抓取挂单' 22 140 | Out-Null
$maxFetch = Add-Number $config.settings.max_fetch_results 160 138 8 100

Add-Label '每批 fetch 数' 300 140 | Out-Null
$batch = Add-Number $config.settings.fetch_batch_size 410 138 1 10

Add-Label '每页显示条数' 22 178 | Out-Null
$pageSize = Add-Number $config.settings.page_size 160 176 4 12

Add-Label '面板停留秒数' 300 178 | Out-Null
$timeout = Add-Number $config.settings.result_timeout_seconds 410 176 5 90

$autoClipboard = New-Object System.Windows.Forms.CheckBox
$autoClipboard.Text = '启用 Ctrl+C 自动查价'
$autoClipboard.Checked = [bool]$config.settings.auto_clipboard
$autoClipboard.Location = New-Object System.Drawing.Point(160, 218)
$autoClipboard.Size = New-Object System.Drawing.Size(260, 28)
$autoClipboard.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Controls.Add($autoClipboard)

Add-Label '手动查价热键' 22 256 | Out-Null
$manualHotkey = New-Object System.Windows.Forms.ComboBox
$manualHotkey.DropDownStyle = 'DropDownList'
[void] $manualHotkey.Items.AddRange(@('关闭', 'F6', 'F7', 'F8', 'F9', 'F10', 'Ctrl+Alt+D'))
$currentHotkey = [string]$config.settings.manual_hotkey
if (@('off', 'none', 'disabled') -contains $currentHotkey.Trim().ToLowerInvariant()) {
    $currentHotkey = '关闭'
}
if (-not $currentHotkey -or -not $manualHotkey.Items.Contains($currentHotkey)) {
    $currentHotkey = 'F8'
}
$manualHotkey.SelectedItem = $currentHotkey
$manualHotkey.Location = New-Object System.Drawing.Point(160, 254)
$manualHotkey.Size = New-Object System.Drawing.Size(160, 26)
$manualHotkey.BackColor = [System.Drawing.Color]::FromArgb(12, 13, 16)
$manualHotkey.ForeColor = [System.Drawing.Color]::FromArgb(248, 250, 252)
$form.Controls.Add($manualHotkey)

$hint = New-Object System.Windows.Forms.Label
$hint.Text = '设置保存后，运行中的工具会在几秒内自动读取。填错赛季时可点“恢复默认”。'
$hint.AutoSize = $false
$hint.Location = New-Object System.Drawing.Point(22, 296)
$hint.Size = New-Object System.Drawing.Size(516, 34)
$hint.ForeColor = [System.Drawing.Color]::FromArgb(148, 163, 184)
$form.Controls.Add($hint)

$status = New-Object System.Windows.Forms.Label
$status.Text = ''
$status.AutoSize = $false
$status.Location = New-Object System.Drawing.Point(22, 384)
$status.Size = New-Object System.Drawing.Size(516, 24)
$status.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138)
$form.Controls.Add($status)

$cookie = New-Object System.Windows.Forms.Button
$cookie.Text = '设置 Cookie'
$cookie.Location = New-Object System.Drawing.Point(22, 344)
$cookie.Size = New-Object System.Drawing.Size(110, 30)
$cookie.Add_Click({
    if (Test-Path -LiteralPath $cookieScript) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $cookieScript, '-Root', $rootPath)
    } else {
        $status.Text = '找不到 Cookie 设置脚本。'
    }
})
$form.Controls.Add($cookie)

$defaultsButton = New-Object System.Windows.Forms.Button
$defaultsButton.Text = '恢复默认'
$defaultsButton.Location = New-Object System.Drawing.Point(148, 344)
$defaultsButton.Size = New-Object System.Drawing.Size(90, 30)
$defaultsButton.Add_Click({
    Apply-DefaultsToForm
    Set-Status '已恢复推荐默认值，点击“保存”后生效。'
})
$form.Controls.Add($defaultsButton)

$folder = New-Object System.Windows.Forms.Button
$folder.Text = '配置目录'
$folder.Location = New-Object System.Drawing.Point(248, 344)
$folder.Size = New-Object System.Drawing.Size(90, 30)
$folder.Add_Click({
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
    Start-Process $configDir
    Set-Status '已打开配置目录。'
})
$form.Controls.Add($folder)

$save = New-Object System.Windows.Forms.Button
$save.Text = '保存'
$save.Location = New-Object System.Drawing.Point(348, 344)
$save.Size = New-Object System.Drawing.Size(90, 30)
$save.Add_Click({
    [void](Save-SettingsFromForm)
})
$form.Controls.Add($save)

$close = New-Object System.Windows.Forms.Button
$close.Text = '关闭'
$close.Location = New-Object System.Drawing.Point(448, 344)
$close.Size = New-Object System.Drawing.Size(90, 30)
$close.Add_Click({ $form.Close() })
$form.Controls.Add($close)

[void] $form.ShowDialog()
