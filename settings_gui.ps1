param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

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

[xml]$xaml = @"
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="清价 POE2 - 设置"
        Width="720" Height="540"
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
            <Setter Property="Height" Value="38"/>
            <Setter Property="Padding" Value="16,0"/>
            <Setter Property="Foreground" Value="#E8F1F8"/>
            <Setter Property="Background" Value="#17202B"/>
            <Setter Property="BorderBrush" Value="#2B3A4D"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="FontWeight" Value="SemiBold"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type Button}">
                        <Border x:Name="Bd" CornerRadius="13" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}">
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
        <Style x:Key="Input" TargetType="{x:Type TextBox}">
            <Setter Property="Height" Value="34"/>
            <Setter Property="Background" Value="#0A111A"/>
            <Setter Property="Foreground" Value="#F8FAFC"/>
            <Setter Property="BorderBrush" Value="#344457"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="CaretBrush" Value="#32E6A1"/>
            <Setter Property="Padding" Value="10,6"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type TextBox}">
                        <Border x:Name="Bd" CornerRadius="12" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}">
                            <ScrollViewer x:Name="PART_ContentHost" Margin="0"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsKeyboardFocused" Value="True">
                                <Setter TargetName="Bd" Property="BorderBrush" Value="#2DD4BF"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>
        <Style x:Key="Combo" TargetType="{x:Type ComboBox}">
            <Setter Property="Height" Value="34"/>
            <Setter Property="Background" Value="#0A111A"/>
            <Setter Property="Foreground" Value="#E8F1F8"/>
            <Setter Property="BorderBrush" Value="#344457"/>
            <Setter Property="Padding" Value="8,4"/>
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
                    <RowDefinition Height="74"/>
                    <RowDefinition Height="*"/>
                    <RowDefinition Height="58"/>
                </Grid.RowDefinitions>

                <StackPanel>
                    <TextBlock Text="客户常用设置" FontSize="23" FontWeight="Bold" Foreground="{StaticResource AccentBrush}"/>
                    <TextBlock Text="调整联赛、抓取数量、页面显示和热键。保存后运行中的工具会自动读取。" Margin="0,8,0,0" Foreground="{StaticResource MutedBrush}" FontSize="13"/>
                </StackPanel>

                <Border Grid.Row="1" CornerRadius="16" Background="{StaticResource PanelBrush}" BorderBrush="{StaticResource StrokeBrush}" BorderThickness="1" Padding="18">
                    <Grid>
                        <Grid.ColumnDefinitions>
                            <ColumnDefinition Width="150"/>
                            <ColumnDefinition Width="*"/>
                            <ColumnDefinition Width="28"/>
                            <ColumnDefinition Width="150"/>
                            <ColumnDefinition Width="*"/>
                        </Grid.ColumnDefinitions>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="46"/>
                            <RowDefinition Height="*"/>
                            <RowDefinition Height="44"/>
                        </Grid.RowDefinitions>

                        <TextBlock Text="主联赛" Grid.Row="0" Grid.Column="0" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="PrimaryLeague" Grid.Row="0" Grid.Column="1" Grid.ColumnSpan="4" Style="{StaticResource Input}"/>

                        <TextBlock Text="备用联赛" Grid.Row="1" Grid.Column="0" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="FallbackLeague" Grid.Row="1" Grid.Column="1" Grid.ColumnSpan="4" Style="{StaticResource Input}"/>

                        <TextBlock Text="最多抓取挂单" Grid.Row="2" Grid.Column="0" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="MaxFetch" Grid.Row="2" Grid.Column="1" Style="{StaticResource Input}"/>
                        <TextBlock Text="每批 fetch 数" Grid.Row="2" Grid.Column="3" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="BatchSize" Grid.Row="2" Grid.Column="4" Style="{StaticResource Input}"/>

                        <TextBlock Text="每页显示条数" Grid.Row="3" Grid.Column="0" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="PageSize" Grid.Row="3" Grid.Column="1" Style="{StaticResource Input}"/>
                        <TextBlock Text="面板停留秒数" Grid.Row="3" Grid.Column="3" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <TextBox x:Name="TimeoutSeconds" Grid.Row="3" Grid.Column="4" Style="{StaticResource Input}"/>

                        <TextBlock Text="手动查价热键" Grid.Row="4" Grid.Column="0" Foreground="{StaticResource MutedBrush}" VerticalAlignment="Center"/>
                        <ComboBox x:Name="ManualHotkey" Grid.Row="4" Grid.Column="1" Width="170" HorizontalAlignment="Left" Style="{StaticResource Combo}"/>
                        <CheckBox x:Name="AutoClipboard" Grid.Row="4" Grid.Column="3" Grid.ColumnSpan="2" Content="启用 Ctrl+C 自动查价" Foreground="{StaticResource TextBrush}" VerticalAlignment="Center"/>

                        <TextBlock Grid.Row="5" Grid.ColumnSpan="5" Text="数值建议：最多抓取 80，每批 10，每页 8，面板停留 16 秒。热键可关闭，Ctrl+C 自动查价仍可单独启用。" Foreground="{StaticResource MutedBrush}" TextWrapping="Wrap" VerticalAlignment="Center"/>

                        <StackPanel Grid.Row="7" Grid.ColumnSpan="5" Orientation="Horizontal" HorizontalAlignment="Right">
                            <Button x:Name="CookieButton" Content="设置 Cookie" Width="112" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="DefaultsButton" Content="恢复默认" Width="100" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="FolderButton" Content="配置目录" Width="100" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="SaveButton" Content="保存" Width="92" Margin="10,0,0,0" Style="{StaticResource PrimaryButton}"/>
                        </StackPanel>
                    </Grid>
                </Border>

                <Border x:Name="StatusShell" Grid.Row="2" CornerRadius="14" Background="#111A24" BorderBrush="#263445" BorderThickness="1" Padding="16,0" VerticalAlignment="Bottom" Height="46">
                    <DockPanel LastChildFill="True">
                        <Button x:Name="CloseActionButton" Content="关闭" Width="88" Height="32" DockPanel.Dock="Right" Style="{StaticResource BaseButton}"/>
                        <TextBlock x:Name="StatusText" Text="准备就绪。" VerticalAlignment="Center" Foreground="{StaticResource GoldBrush}" FontSize="13"/>
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
$primary = Find-Control 'PrimaryLeague'
$fallback = Find-Control 'FallbackLeague'
$maxFetch = Find-Control 'MaxFetch'
$batch = Find-Control 'BatchSize'
$pageSize = Find-Control 'PageSize'
$timeout = Find-Control 'TimeoutSeconds'
$manualHotkey = Find-Control 'ManualHotkey'
$autoClipboard = Find-Control 'AutoClipboard'
$cookie = Find-Control 'CookieButton'
$defaultsButton = Find-Control 'DefaultsButton'
$folder = Find-Control 'FolderButton'
$save = Find-Control 'SaveButton'
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

function Set-ComboSelection($combo, $value) {
    foreach ($item in $combo.Items) {
        if ([string]$item.Content -eq [string]$value) {
            $combo.SelectedItem = $item
            return
        }
    }
}

foreach ($hotkey in @('关闭', 'F6', 'F7', 'F8', 'F9', 'F10', 'Ctrl+Alt+D')) {
    $item = New-Object System.Windows.Controls.ComboBoxItem
    $item.Content = $hotkey
    [void]$manualHotkey.Items.Add($item)
}

function Apply-DefaultsToForm {
    $primary.Text = $defaults.primary_league
    $fallback.Text = $defaults.fallback_league
    $maxFetch.Text = [string]$defaults.max_fetch_results
    $batch.Text = [string]$defaults.fetch_batch_size
    $pageSize.Text = [string]$defaults.page_size
    $timeout.Text = [string]$defaults.result_timeout_seconds
    $autoClipboard.IsChecked = [bool]$defaults.auto_clipboard
    Set-ComboSelection $manualHotkey $defaults.manual_hotkey
}

function Get-IntInRange {
    param(
        [string] $Text,
        [int] $Default,
        [int] $Min,
        [int] $Max
    )
    $value = 0
    if (-not [int]::TryParse($Text, [ref]$value)) {
        return $Default
    }
    if ($value -lt $Min) {
        return $Min
    }
    if ($value -gt $Max) {
        return $Max
    }
    return $value
}

function Save-SettingsFromForm {
    if ([string]::IsNullOrWhiteSpace($primary.Text)) {
        Set-Status -Text '主联赛不能为空，已使用默认赛季。' -Kind 'error'
        $primary.Text = $defaults.primary_league
        return $false
    }
    if ([string]::IsNullOrWhiteSpace($fallback.Text)) {
        $fallback.Text = $defaults.fallback_league
    }

    $selectedHotkey = if ($manualHotkey.SelectedItem) { [string]$manualHotkey.SelectedItem.Content } else { $defaults.manual_hotkey }
    $config.settings.primary_league = $primary.Text.Trim()
    $config.settings.fallback_league = $fallback.Text.Trim()
    $config.settings.max_fetch_results = Get-IntInRange $maxFetch.Text $defaults.max_fetch_results 8 100
    $config.settings.fetch_batch_size = Get-IntInRange $batch.Text $defaults.fetch_batch_size 1 10
    $config.settings.page_size = Get-IntInRange $pageSize.Text $defaults.page_size 4 12
    $config.settings.result_timeout_seconds = Get-IntInRange $timeout.Text $defaults.result_timeout_seconds 5 90
    $config.settings.auto_clipboard = [bool]$autoClipboard.IsChecked
    $config.settings.manual_hotkey = $selectedHotkey
    Save-Config $config
    Set-Status -Text '已保存。运行中的工具会自动读取新设置。' -Kind 'ok'
    return $true
}

$primary.Text = [string]$config.settings.primary_league
$fallback.Text = [string]$config.settings.fallback_league
$maxFetch.Text = [string]$config.settings.max_fetch_results
$batch.Text = [string]$config.settings.fetch_batch_size
$pageSize.Text = [string]$config.settings.page_size
$timeout.Text = [string]$config.settings.result_timeout_seconds
$autoClipboard.IsChecked = [bool]$config.settings.auto_clipboard
$currentHotkey = [string]$config.settings.manual_hotkey
if (@('off', 'none', 'disabled') -contains $currentHotkey.Trim().ToLowerInvariant()) {
    $currentHotkey = '关闭'
}
if (-not $currentHotkey) {
    $currentHotkey = $defaults.manual_hotkey
}
Set-ComboSelection $manualHotkey $currentHotkey
if (-not $manualHotkey.SelectedItem) {
    Set-ComboSelection $manualHotkey $defaults.manual_hotkey
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

$cookie.add_Click({
    if (Test-Path -LiteralPath $cookieScript) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $cookieScript, '-Root', $rootPath)
        Set-Status -Text '已打开 Cookie 设置窗口。'
        return
    }
    Set-Status -Text '找不到 Cookie 设置脚本。' -Kind 'error'
})

$defaultsButton.add_Click({
    Apply-DefaultsToForm
    Set-Status -Text '已恢复推荐默认值，点击“保存”后生效。'
})

$folder.add_Click({
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
    Start-Process $configDir
    Set-Status -Text '已打开配置目录。'
})

$save.add_Click({
    [void](Save-SettingsFromForm)
})

[void] $window.ShowDialog()
