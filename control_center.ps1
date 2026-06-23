param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$exe = Join-Path $rootPath 'QingPricePOE2.exe'
$configPath = Join-Path (Join-Path $env:APPDATA 'poe2_cn_price_bridge') 'config.json'
$tradeHome = 'https://poe.game.qq.com/trade2'

function Resolve-Tool($name) {
    Join-Path $rootPath $name
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

[xml]$xaml = @"
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="清价 POE2 - 控制中心"
        Width="820" Height="610"
        WindowStartupLocation="CenterScreen"
        WindowStyle="None"
        ResizeMode="NoResize"
        AllowsTransparency="True"
        Background="Transparent"
        FontFamily="Microsoft YaHei UI">
    <Window.Resources>
        <SolidColorBrush x:Key="PageBrush" Color="#080B10"/>
        <SolidColorBrush x:Key="PanelBrush" Color="#0F151E"/>
        <SolidColorBrush x:Key="StrokeBrush" Color="#202B3A"/>
        <SolidColorBrush x:Key="TextBrush" Color="#E8F1F8"/>
        <SolidColorBrush x:Key="MutedBrush" Color="#8EA0B6"/>
        <SolidColorBrush x:Key="AccentBrush" Color="#5EEAD4"/>
        <SolidColorBrush x:Key="GoldBrush" Color="#FBBF24"/>

        <Style x:Key="BaseButton" TargetType="{x:Type Button}">
            <Setter Property="Height" Value="46"/>
            <Setter Property="Foreground" Value="#E8F1F8"/>
            <Setter Property="Background" Value="#17202B"/>
            <Setter Property="BorderBrush" Value="#2B3A4D"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="FontWeight" Value="SemiBold"/>
            <Setter Property="Margin" Value="7"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type Button}">
                        <Border x:Name="Bd" CornerRadius="14" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}">
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#223044"/>
                                <Setter TargetName="Bd" Property="BorderBrush" Value="#52677F"/>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#101722"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>
        <Style x:Key="PrimaryButton" TargetType="{x:Type Button}" BasedOn="{StaticResource BaseButton}">
            <Setter Property="Background" Value="#0F766E"/>
            <Setter Property="BorderBrush" Value="#2DD4BF"/>
            <Setter Property="Foreground" Value="#F0FDFA"/>
        </Style>

        <Style x:Key="ChromeButton" TargetType="{x:Type Button}">
            <Setter Property="Width" Value="34"/>
            <Setter Property="Height" Value="30"/>
            <Setter Property="Foreground" Value="#9FB3C8"/>
            <Setter Property="Background" Value="Transparent"/>
            <Setter Property="BorderThickness" Value="0"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="FontSize" Value="18"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type Button}">
                        <Border x:Name="Bd" CornerRadius="9" Background="{TemplateBinding Background}">
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#223044"/>
                                <Setter Property="Foreground" Value="#E8F1F8"/>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#3A141B"/>
                                <Setter Property="Foreground" Value="#FECACA"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>
    </Window.Resources>

    <Border Margin="8" CornerRadius="22" BorderBrush="#223143" BorderThickness="1">
        <Border.Background>
            <LinearGradientBrush StartPoint="0,0" EndPoint="1,1">
                <GradientStop Color="#101824" Offset="0"/>
                <GradientStop Color="#080B10" Offset="0.55"/>
                <GradientStop Color="#05070A" Offset="1"/>
            </LinearGradientBrush>
        </Border.Background>
        <Border.Effect>
            <DropShadowEffect BlurRadius="34" ShadowDepth="0" Opacity="0.50" Color="#000000"/>
        </Border.Effect>
        <Grid>
            <Grid.RowDefinitions>
                <RowDefinition Height="52"/>
                <RowDefinition Height="*"/>
            </Grid.RowDefinitions>

            <Border x:Name="TitleBar" Grid.Row="0" CornerRadius="22,22,0,0" Background="#0B111A">
                <Grid>
                    <StackPanel Orientation="Horizontal" Margin="22,0,0,0" VerticalAlignment="Center">
                        <Border Width="28" Height="28" CornerRadius="9" Background="#0F766E" BorderBrush="#2DD4BF" BorderThickness="1">
                            <TextBlock Text="清" HorizontalAlignment="Center" VerticalAlignment="Center" Foreground="#ECFEFF" FontSize="15" FontWeight="Bold"/>
                        </Border>
                        <TextBlock Text="清价 POE2" Margin="10,0,0,0" VerticalAlignment="Center" FontSize="13" FontWeight="Bold" Foreground="#E8F1F8"/>
                    </StackPanel>
                    <Button x:Name="CloseButton" Content="×" HorizontalAlignment="Right" Margin="0,0,14,0" VerticalAlignment="Center" Style="{StaticResource ChromeButton}"/>
                </Grid>
            </Border>

            <Grid Grid.Row="1" Margin="28,24,28,22">
                <Grid.RowDefinitions>
                    <RowDefinition Height="92"/>
                    <RowDefinition Height="*"/>
                    <RowDefinition Height="58"/>
                </Grid.RowDefinitions>

                <Grid>
                    <StackPanel>
                        <TextBlock Text="控制中心" FontSize="24" FontWeight="Bold" Foreground="{StaticResource AccentBrush}"/>
                        <TextBlock Text="启动工具、设置 Cookie、查看历史、导出诊断和支持包。" Margin="0,8,0,0" Foreground="{StaticResource MutedBrush}" FontSize="13"/>
                    </StackPanel>
                    <Border HorizontalAlignment="Right" VerticalAlignment="Top" CornerRadius="999" Background="#181F2A" BorderBrush="#344457" BorderThickness="1" Padding="16,7">
                        <TextBlock x:Name="CookieState" Text="未配置 Cookie" Foreground="{StaticResource GoldBrush}" FontWeight="SemiBold"/>
                    </Border>
                </Grid>

                <Border Grid.Row="1" CornerRadius="16" Background="{StaticResource PanelBrush}" BorderBrush="{StaticResource StrokeBrush}" BorderThickness="1" Padding="14">
                    <UniformGrid x:Name="ButtonGrid" Columns="3"/>
                </Border>

                <Border x:Name="StatusShell" Grid.Row="2" CornerRadius="14" Background="#111A24" BorderBrush="#263445" BorderThickness="1" Padding="16,0" VerticalAlignment="Bottom" Height="46">
                    <DockPanel LastChildFill="True">
                        <Button x:Name="CloseActionButton" Content="关闭" Width="88" Height="32" DockPanel.Dock="Right" Style="{StaticResource BaseButton}"/>
                        <TextBlock x:Name="StatusText" Text="准备就绪。第一次使用建议先点“首次使用向导”。" VerticalAlignment="Center" Foreground="{StaticResource GoldBrush}" FontSize="13"/>
                    </DockPanel>
                </Border>
            </Grid>
        </Grid>
    </Border>
</Window>
"@

$reader = New-Object System.Xml.XmlNodeReader $xaml
$window = [Windows.Markup.XamlReader]::Load($reader)

function Find-Control($name) {
    return $window.FindName($name)
}

$titleBar = Find-Control 'TitleBar'
$closeButton = Find-Control 'CloseButton'
$closeActionButton = Find-Control 'CloseActionButton'
$buttonGrid = Find-Control 'ButtonGrid'
$cookieState = Find-Control 'CookieState'
$status = Find-Control 'StatusText'
$statusShell = Find-Control 'StatusShell'

$iconPath = Join-Path $rootPath 'assets\app.ico'
if (Test-Path -LiteralPath $iconPath) {
    $window.Icon = [System.Windows.Media.Imaging.BitmapFrame]::Create([Uri]::new($iconPath))
}

function New-Brush($hex) {
    return New-Object System.Windows.Media.SolidColorBrush ([System.Windows.Media.ColorConverter]::ConvertFromString($hex))
}

function Set-Status {
    param(
        [string] $Text,
        [string] $Kind = 'info'
    )
    $status.Text = $Text
    switch ($Kind) {
        'ok' {
            $status.Foreground = New-Brush '#32E6A1'
            $statusShell.BorderBrush = New-Brush '#1C8D68'
        }
        'error' {
            $status.Foreground = New-Brush '#F87171'
            $statusShell.BorderBrush = New-Brush '#8E303A'
        }
        default {
            $status.Foreground = New-Brush '#F4D35E'
            $statusShell.BorderBrush = New-Brush '#384657'
        }
    }
}

function Refresh-CookieState {
    $cookieState.Text = Get-CookieState
}

function Start-HiddenScript($script) {
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $script, '-Root', $rootPath)
        return $true
    }
    Set-Status -Text "找不到 $([IO.Path]::GetFileName($script))" -Kind 'error'
    return $false
}

function Start-VisibleScript($script) {
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File', $script)
        return $true
    }
    Set-Status -Text "找不到 $([IO.Path]::GetFileName($script))" -Kind 'error'
    return $false
}

function Run-ExeCommand($arg, $outputName) {
    if (-not (Test-Path -LiteralPath $exe)) {
        Set-Status -Text '找不到 QingPricePOE2.exe' -Kind 'error'
        return
    }
    $output = Join-Path $rootPath $outputName
    $process = Start-Process -FilePath $exe -ArgumentList @($arg, $output) -WorkingDirectory $rootPath -Wait -PassThru
    if ($process.ExitCode -eq 0 -and (Test-Path -LiteralPath $output)) {
        Start-Process -FilePath $output
        Set-Status -Text "已生成 $outputName" -Kind 'ok'
        return
    }
    Set-Status -Text "$outputName 生成失败，退出码 $($process.ExitCode)" -Kind 'error'
}

function Start-SupportBundle {
    $script = Resolve-Tool 'SupportBundle.ps1'
    if (Test-Path -LiteralPath $script) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $script, '-Root', $rootPath)
        Set-Status -Text '正在生成支持包，完成后会打开文件位置。'
        return
    }
    Set-Status -Text '找不到 SupportBundle.ps1' -Kind 'error'
}

function New-ActionButton {
    param(
        [string] $Text,
        [scriptblock] $Action,
        [bool] $Primary = $false
    )
    $button = New-Object System.Windows.Controls.Button
    $button.Content = $Text
    if ($Primary) {
        $button.Style = $window.Resources['PrimaryButton']
    } else {
        $button.Style = $window.Resources['BaseButton']
    }
    $button.add_Click($Action)
    [void]$buttonGrid.Children.Add($button)
}

$titleBar.add_MouseLeftButtonDown({
    try {
        $window.DragMove()
    } catch {}
})

$closeHandler = {
    $window.Close()
}
$closeButton.add_Click($closeHandler)
$closeActionButton.add_Click($closeHandler)

Refresh-CookieState

New-ActionButton '启动工具' {
    if (Test-Path -LiteralPath $exe) {
        Start-Process -FilePath $exe -WorkingDirectory $rootPath
        Set-Status -Text '已启动，托盘图标会常驻后台。' -Kind 'ok'
        return
    }
    Set-Status -Text '找不到 QingPricePOE2.exe' -Kind 'error'
} $true

New-ActionButton '首次使用向导' {
    if (Start-HiddenScript (Resolve-Tool 'first_run_wizard.ps1')) {
        Set-Status -Text '已打开首次使用向导。'
    }
} $true

New-ActionButton '设置 Cookie' {
    if (Start-HiddenScript (Resolve-Tool 'set_cookie_gui.ps1')) {
        Set-Status -Text '已打开 Cookie 设置窗口。'
    }
}

New-ActionButton '常用设置' {
    if (Start-HiddenScript (Resolve-Tool 'settings_gui.ps1')) {
        Set-Status -Text '已打开设置窗口。'
    }
}

New-ActionButton '查询历史' {
    if (Start-HiddenScript (Resolve-Tool 'history_gui.ps1')) {
        Set-Status -Text '已打开查询历史。'
    }
}

New-ActionButton '打开国服市集' {
    Start-Process $tradeHome
    Set-Status -Text '已打开国服市集。'
}

New-ActionButton '运行自检' {
    Run-ExeCommand '--self-check' 'selfcheck.txt'
}

New-ActionButton '导出诊断' {
    Run-ExeCommand '--diagnostics' 'diagnostics.txt'
}

New-ActionButton '验证 Cookie' {
    if (-not (Test-Path -LiteralPath $exe)) {
        Set-Status -Text '找不到 QingPricePOE2.exe' -Kind 'error'
        return
    }
    Set-Status -Text '正在验证 Cookie...'
    $process = Start-Process -FilePath $exe -ArgumentList '--validate-cookie' -WorkingDirectory $rootPath -Wait -PassThru
    if ($process.ExitCode -eq 0) {
        Refresh-CookieState
        $cookieState.Text = 'Cookie 已验证'
        Set-Status -Text 'Cookie 验证通过。' -Kind 'ok'
        return
    }
    Set-Status -Text 'Cookie 验证失败，请重新设置。' -Kind 'error'
}

New-ActionButton '创建桌面快捷方式' {
    if (Start-VisibleScript (Resolve-Tool 'InstallShortcut.ps1')) {
        Set-Status -Text '已打开快捷方式创建脚本。'
    }
}

New-ActionButton '重置本机数据' {
    if (Start-VisibleScript (Resolve-Tool 'ResetData.ps1')) {
        Set-Status -Text '重置脚本已打开，需要输入 RESET 才会执行。'
    }
}

New-ActionButton '卸载清理' {
    if (Start-VisibleScript (Resolve-Tool 'Uninstall.ps1')) {
        Set-Status -Text '卸载脚本已打开，需要输入 UNINSTALL 才会执行。'
    }
}

New-ActionButton '生成支持包' {
    Start-SupportBundle
}

New-ActionButton '打开说明' {
    $readme = Resolve-Tool 'README.md'
    if (Test-Path -LiteralPath $readme) {
        Start-Process $readme
        Set-Status -Text '已打开 README。'
        return
    }
    Set-Status -Text '找不到 README.md' -Kind 'error'
}

New-ActionButton '打开文件夹' {
    Start-Process $rootPath
    Set-Status -Text '已打开程序文件夹。'
}

New-ActionButton '关闭窗口' {
    $window.Close()
}

[void] $window.ShowDialog()
