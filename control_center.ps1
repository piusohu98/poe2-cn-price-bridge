param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$exe = Join-Path $rootPath 'QingPricePOE2.exe'
$configPath = Join-Path (Join-Path $env:APPDATA 'poe2_cn_price_bridge') 'config.json'
$tradeHome = 'https://poe.game.qq.com/trade2'

function Resolve-Tool($name) {
    Join-Path $rootPath $name
}

function Start-HiddenScript($script) {
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $script, '-Root', $rootPath)
    } else {
        Set-Status "找不到 $([IO.Path]::GetFileName($script))"
    }
}

function Start-VisibleScript($script) {
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File', $script)
    } else {
        Set-Status "找不到 $([IO.Path]::GetFileName($script))"
    }
}

function Start-SupportBundle {
    $script = Resolve-Tool 'SupportBundle.ps1'
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $script, '-Root', $rootPath)
        Set-Status '正在生成支持包，完成后会打开文件位置。'
    } else {
        Set-Status '找不到 SupportBundle.ps1'
    }
}

function Run-ExeCommand($args, $outputName) {
    if (-not (Test-Path -LiteralPath $exe)) {
        Set-Status '找不到 QingPricePOE2.exe'
        return
    }
    $output = Join-Path $rootPath $outputName
    $process = Start-Process -FilePath $exe -ArgumentList @($args, $output) -WorkingDirectory $rootPath -Wait -PassThru
    if ($process.ExitCode -eq 0 -and (Test-Path -LiteralPath $output)) {
        Start-Process -FilePath $output
        Set-Status "已生成 $outputName"
    } else {
        Set-Status "$outputName 生成失败，退出码 $($process.ExitCode)"
    }
}

function Get-CookieState {
    if (-not (Test-Path -LiteralPath $configPath)) {
        return '未配置 Cookie'
    }
    try {
        $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
        if ($config.PSObject.Properties['cookie_dpapi'] -and $config.cookie_dpapi) {
            return 'Cookie 已保存'
        }
    } catch {
        return '配置文件读取失败'
    }
    return '未配置 Cookie'
}

function Set-Status($text) {
    $script:status.Text = $text
}

$form = New-Object System.Windows.Forms.Form
$form.Text = '清价 POE2 - 控制中心'
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.ClientSize = New-Object System.Drawing.Size(760, 580)
$form.BackColor = [System.Drawing.Color]::FromArgb(13, 16, 21)
$form.ForeColor = [System.Drawing.Color]::FromArgb(226, 232, 240)
$form.Font = New-Object System.Drawing.Font('Microsoft YaHei UI', 9)
$iconPath = Join-Path $rootPath 'assets\app.ico'
if (Test-Path -LiteralPath $iconPath) {
    $form.Icon = New-Object System.Drawing.Icon($iconPath)
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

function New-Button($text, $x, $y, $w, $h, $primary = $false) {
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

New-Label '清价 POE2' 28 22 260 34 17 $true ([System.Drawing.Color]::FromArgb(134, 239, 172)) | Out-Null
New-Label '国服 trade2 游戏内查价工具' 30 58 420 24 9 $false ([System.Drawing.Color]::FromArgb(148, 163, 184)) | Out-Null
$versionText = if (Test-Path -LiteralPath (Join-Path $rootPath 'VERSION.txt')) {
    (Get-Content -LiteralPath (Join-Path $rootPath 'VERSION.txt') -Raw -ErrorAction SilentlyContinue) -split "`r?`n" | Select-Object -First 2
} else {
    @('version: unknown')
}
New-Label ($versionText -join '    ') 30 84 690 24 9 $false ([System.Drawing.Color]::FromArgb(148, 163, 184)) | Out-Null

$cookieState = New-Label (Get-CookieState) 590 28 130 28 9 $true ([System.Drawing.Color]::FromArgb(253, 230, 138))
$status = New-Label '准备就绪。第一次使用建议先点“首次使用向导”。' 30 532 700 28 9 $false ([System.Drawing.Color]::FromArgb(253, 230, 138))

$buttons = @(
    @{ Text='启动工具'; X=30;  Y=130; Primary=$true;  Action={ if(Test-Path $exe){ Start-Process -FilePath $exe -WorkingDirectory $rootPath; Set-Status '已启动，托盘图标会常驻后台。' } else { Set-Status '找不到 QingPricePOE2.exe' } } },
    @{ Text='首次使用向导'; X=270; Y=130; Primary=$true;  Action={ Start-HiddenScript (Resolve-Tool 'first_run_wizard.ps1'); Set-Status '已打开首次使用向导。' } },
    @{ Text='设置 Cookie'; X=510; Y=130; Primary=$false; Action={ Start-HiddenScript (Resolve-Tool 'set_cookie_gui.ps1'); Set-Status '已打开 Cookie 设置窗口。' } },
    @{ Text='常用设置'; X=30;  Y=190; Primary=$false; Action={ Start-HiddenScript (Resolve-Tool 'settings_gui.ps1'); Set-Status '已打开设置窗口。' } },
    @{ Text='查询历史'; X=270; Y=190; Primary=$false; Action={ Start-HiddenScript (Resolve-Tool 'history_gui.ps1'); Set-Status '已打开查询历史。' } },
    @{ Text='打开国服市集'; X=510; Y=190; Primary=$false; Action={ Start-Process $tradeHome; Set-Status '已打开国服市集。' } },
    @{ Text='运行自检'; X=30;  Y=250; Primary=$false; Action={ Run-ExeCommand '--self-check' 'selfcheck.txt' } },
    @{ Text='导出诊断'; X=270; Y=250; Primary=$false; Action={ Run-ExeCommand '--diagnostics' 'diagnostics.txt' } },
    @{ Text='验证 Cookie'; X=510; Y=250; Primary=$false; Action={ if(Test-Path $exe){ $p=Start-Process -FilePath $exe -ArgumentList '--validate-cookie' -WorkingDirectory $rootPath -Wait -PassThru; if($p.ExitCode -eq 0){ Set-Status 'Cookie 验证通过。'; $cookieState.Text='Cookie 已验证' } else { Set-Status 'Cookie 验证失败，请重新设置。' } } } },
    @{ Text='创建桌面快捷方式'; X=30;  Y=310; Primary=$false; Action={ Start-VisibleScript (Resolve-Tool 'InstallShortcut.ps1'); Set-Status '已打开快捷方式创建脚本。' } },
    @{ Text='重置本机数据'; X=270; Y=310; Primary=$false; Action={ Start-VisibleScript (Resolve-Tool 'ResetData.ps1'); Set-Status '重置脚本已打开，需要输入 RESET 才会执行。' } },
    @{ Text='卸载清理'; X=510; Y=310; Primary=$false; Action={ Start-VisibleScript (Resolve-Tool 'Uninstall.ps1'); Set-Status '卸载脚本已打开，需要输入 UNINSTALL 才会执行。' } },
    @{ Text='生成支持包'; X=30;  Y=370; Primary=$false; Action={ Start-SupportBundle } },
    @{ Text='打开说明'; X=270; Y=370; Primary=$false; Action={ $readme=Resolve-Tool 'README.md'; if(Test-Path $readme){ Start-Process $readme; Set-Status '已打开 README。' } } },
    @{ Text='打开文件夹'; X=510; Y=370; Primary=$false; Action={ Start-Process $rootPath; Set-Status '已打开程序文件夹。' } },
    @{ Text='关闭窗口'; X=270; Y=430; Primary=$false; Action={ $form.Close() } }
)

foreach ($item in $buttons) {
    $button = New-Button $item.Text $item.X $item.Y 200 42 $item.Primary
    $action = $item.Action
    $button.Add_Click($action)
}

[void] $form.ShowDialog()
