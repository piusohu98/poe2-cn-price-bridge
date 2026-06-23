param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$appDir = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
$historyPath = Join-Path $appDir 'history.jsonl'
$exeCandidates = @(
    (Join-Path $rootPath 'QingPricePOE2.exe'),
    (Join-Path $rootPath 'poe2_cn_price_bridge.exe'),
    (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

function Read-History {
    if (-not (Test-Path -LiteralPath $historyPath)) { return @() }
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($line in [System.IO.File]::ReadLines($historyPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $rows.Add(($line | ConvertFrom-Json)) } catch {}
    }
    return @($rows | Select-Object -Last 80)
}

function UnixToLocal($value) {
    try {
        return [DateTimeOffset]::FromUnixTimeSeconds([int64]$value).LocalDateTime.ToString('yyyy-MM-dd HH:mm:ss')
    } catch {
        return ''
    }
}

$form = New-Object System.Windows.Forms.Form
$form.Text = '清价 POE2 - 查询历史'
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'Sizable'
$form.MinimumSize = New-Object System.Drawing.Size(860, 460)
$form.ClientSize = New-Object System.Drawing.Size(960, 540)
$form.BackColor = [System.Drawing.Color]::FromArgb(14, 17, 22)
$form.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 9)

$title = New-Object System.Windows.Forms.Label
$title.Text = '最近查询'
$title.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 13, [System.Drawing.FontStyle]::Bold)
$title.AutoSize = $false
$title.Location = New-Object System.Drawing.Point(18, 14)
$title.Size = New-Object System.Drawing.Size(400, 30)
$title.ForeColor = [System.Drawing.Color]::FromArgb(134, 239, 172)
$form.Controls.Add($title)

$grid = New-Object System.Windows.Forms.DataGridView
$grid.Location = New-Object System.Drawing.Point(18, 54)
$grid.Size = New-Object System.Drawing.Size(922, 390)
$grid.Anchor = 'Top,Bottom,Left,Right'
$grid.BackgroundColor = [System.Drawing.Color]::FromArgb(9, 12, 18)
$grid.BorderStyle = 'FixedSingle'
$grid.GridColor = [System.Drawing.Color]::FromArgb(36, 50, 68)
$grid.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$grid.ColumnHeadersDefaultCellStyle.BackColor = [System.Drawing.Color]::FromArgb(28, 32, 39)
$grid.ColumnHeadersDefaultCellStyle.ForeColor = [System.Drawing.Color]::FromArgb(203, 213, 225)
$grid.EnableHeadersVisualStyles = $false
$grid.RowHeadersVisible = $false
$grid.AllowUserToAddRows = $false
$grid.ReadOnly = $true
$grid.SelectionMode = 'FullRowSelect'
$grid.AutoSizeColumnsMode = 'Fill'
$form.Controls.Add($grid)

$status = New-Object System.Windows.Forms.Label
$status.Text = ''
$status.AutoSize = $false
$status.Location = New-Object System.Drawing.Point(18, 494)
$status.Size = New-Object System.Drawing.Size(650, 24)
$status.Anchor = 'Bottom,Left,Right'
$status.ForeColor = [System.Drawing.Color]::FromArgb(253, 230, 138)
$form.Controls.Add($status)

function Load-Grid {
    $rows = Read-History | Sort-Object ts -Descending
    $table = New-Object System.Data.DataTable
    foreach ($name in @('时间','状态','物品','稀有度','联赛','总数','价格','筛选','信息','链接')) {
        [void]$table.Columns.Add($name)
    }
    foreach ($row in $rows) {
        $filter = @()
        if ($row.used_mods) { $filter += '同属性' }
        if ($row.used_values) { $filter += '数值' }
        $item = if ($row.item) { $row.item } else { $row.base_type }
        [void]$table.Rows.Add(
            (UnixToLocal $row.ts),
            [string]$row.status,
            [string]$item,
            [string]$row.rarity,
            [string]$row.league,
            [string]$row.total,
            [string]$row.priced,
            ($filter -join '+'),
            [string]$row.message,
            [string]$row.url
        )
    }
    $grid.DataSource = $table
    if ($grid.Columns['链接']) { $grid.Columns['链接'].Visible = $false }
    $status.Text = "历史文件: $historyPath"
}

function Selected-Link {
    if (-not $grid.CurrentRow) { return $null }
    $value = $grid.CurrentRow.Cells['链接'].Value
    if ([string]::IsNullOrWhiteSpace([string]$value)) { return $null }
    return [string]$value
}

$refresh = New-Object System.Windows.Forms.Button
$refresh.Text = '刷新'
$refresh.Location = New-Object System.Drawing.Point(18, 454)
$refresh.Size = New-Object System.Drawing.Size(88, 30)
$refresh.Anchor = 'Bottom,Left'
$refresh.Add_Click({ Load-Grid })
$form.Controls.Add($refresh)

$open = New-Object System.Windows.Forms.Button
$open.Text = '打开链接'
$open.Location = New-Object System.Drawing.Point(118, 454)
$open.Size = New-Object System.Drawing.Size(96, 30)
$open.Anchor = 'Bottom,Left'
$open.Add_Click({
    $link = Selected-Link
    if ($link) { Start-Process $link } else { $status.Text = '当前记录没有市集链接。' }
})
$form.Controls.Add($open)

$copy = New-Object System.Windows.Forms.Button
$copy.Text = '复制链接'
$copy.Location = New-Object System.Drawing.Point(226, 454)
$copy.Size = New-Object System.Drawing.Size(96, 30)
$copy.Anchor = 'Bottom,Left'
$copy.Add_Click({
    $link = Selected-Link
    if ($link) {
        [System.Windows.Forms.Clipboard]::SetText($link)
        $status.Text = '已复制链接。'
    } else {
        $status.Text = '当前记录没有市集链接。'
    }
})
$form.Controls.Add($copy)

$diagnostics = New-Object System.Windows.Forms.Button
$diagnostics.Text = '导出诊断'
$diagnostics.Location = New-Object System.Drawing.Point(334, 454)
$diagnostics.Size = New-Object System.Drawing.Size(96, 30)
$diagnostics.Anchor = 'Bottom,Left'
$diagnostics.Add_Click({
    if (-not $exe) {
        $status.Text = '找不到主程序。'
        return
    }
    $target = Join-Path $rootPath 'diagnostics.txt'
    $process = Start-Process -FilePath $exe -ArgumentList @('--diagnostics', $target) -Wait -PassThru
    if ($process.ExitCode -eq 0) {
        $status.Text = "诊断已导出: $target"
        Start-Process $target
    } else {
        $status.Text = "导出诊断失败，退出码: $($process.ExitCode)"
    }
})
$form.Controls.Add($diagnostics)

$close = New-Object System.Windows.Forms.Button
$close.Text = '关闭'
$close.Location = New-Object System.Drawing.Point(850, 454)
$close.Size = New-Object System.Drawing.Size(90, 30)
$close.Anchor = 'Bottom,Right'
$close.Add_Click({ $form.Close() })
$form.Controls.Add($close)

Load-Grid
[void] $form.ShowDialog()
