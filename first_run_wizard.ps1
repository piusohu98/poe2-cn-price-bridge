param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

Add-Type -Name Win32 -Namespace System -MemberDefinition @'
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetConsoleWindow();
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
'@
$hwnd = [System.Win32]::GetConsoleWindow()
[System.Win32]::ShowWindow($hwnd, 0)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$exeCandidates = @(
    (Join-Path $rootPath 'POE2PriceHelper.exe'),
    (Join-Path $rootPath 'poe2_cn_price_bridge.exe'),
    (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
$bridgeExe = Join-Path $rootPath 'POE2PriceHelper.exe'
$loginHelper = Join-Path $rootPath 'POE2PriceLogin.exe'
$settingsScript = Join-Path $rootPath 'settings_gui.ps1'
$loginTimeoutSeconds = 180

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

[xml]$xaml = @"
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="流放2查价助手 - 首次使用向导"
        Width="880" Height="600"
        WindowStartupLocation="CenterScreen"
        WindowStyle="None"
        ResizeMode="NoResize"
        AllowsTransparency="True"
        Background="Transparent"
        FontFamily="Microsoft YaHei UI">
    <Window.Resources>
        <SolidColorBrush x:Key="PageBrush" Color="#080B10"/>
        <SolidColorBrush x:Key="PanelBrush" Color="#0F151E"/>
        <SolidColorBrush x:Key="PanelAltBrush" Color="#131C28"/>
        <SolidColorBrush x:Key="StrokeBrush" Color="#202B3A"/>
        <SolidColorBrush x:Key="TextBrush" Color="#E8F1F8"/>
        <SolidColorBrush x:Key="MutedBrush" Color="#8EA0B6"/>
        <SolidColorBrush x:Key="AccentBrush" Color="#5EEAD4"/>
        <SolidColorBrush x:Key="CyanBrush" Color="#38BDF8"/>
        <SolidColorBrush x:Key="GoldBrush" Color="#FBBF24"/>
        <SolidColorBrush x:Key="DangerBrush" Color="#F87171"/>

        <Style x:Key="BaseButton" TargetType="{x:Type Button}">
            <Setter Property="Height" Value="38"/>
            <Setter Property="Padding" Value="18,0"/>
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
                        <Border x:Name="Bd"
                                CornerRadius="13"
                                Background="{TemplateBinding Background}"
                                BorderBrush="{TemplateBinding BorderBrush}"
                                BorderThickness="{TemplateBinding BorderThickness}">
                            <ContentPresenter HorizontalAlignment="Center"
                                              VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#223044"/>
                                <Setter TargetName="Bd" Property="BorderBrush" Value="#52677F"/>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#101722"/>
                            </Trigger>
                            <Trigger Property="IsEnabled" Value="False">
                                <Setter TargetName="Bd" Property="Opacity" Value="0.42"/>
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

        <Style x:Key="TextInput" TargetType="{x:Type TextBox}">
            <Setter Property="Background" Value="#0A111A"/>
            <Setter Property="Foreground" Value="#F8FAFC"/>
            <Setter Property="BorderBrush" Value="#344457"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="CaretBrush" Value="#32E6A1"/>
            <Setter Property="Padding" Value="12"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="VerticalScrollBarVisibility" Value="Auto"/>
            <Setter Property="HorizontalScrollBarVisibility" Value="Disabled"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type TextBox}">
                        <Border x:Name="Bd"
                                CornerRadius="14"
                                Background="{TemplateBinding Background}"
                                BorderBrush="{TemplateBinding BorderBrush}"
                                BorderThickness="{TemplateBinding BorderThickness}">
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
    </Window.Resources>

    <Border Margin="8"
            CornerRadius="22"
            BorderBrush="#223143"
            BorderThickness="1">
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

            <Border x:Name="TitleBar"
                    Grid.Row="0"
                    CornerRadius="22,22,0,0"
                    Background="#0B111A">
                <Grid>
                    <StackPanel Orientation="Horizontal" Margin="22,0,0,0" VerticalAlignment="Center">
                        <Border Width="28" Height="28" CornerRadius="9" Background="#5B3710" BorderBrush="#C78A2B" BorderThickness="1">
                            <TextBlock Text="价" HorizontalAlignment="Center" VerticalAlignment="Center" Foreground="#FFF4D6" FontSize="15" FontWeight="Bold"/>
                        </Border>
                        <TextBlock Text="流放2查价助手"
                                   Margin="10,0,0,0"
                                   VerticalAlignment="Center"
                                   FontSize="13"
                                   FontWeight="Bold"
                                   Foreground="#E8F1F8"/>
                    </StackPanel>
                    <Button x:Name="CloseButton"
                            Content="×"
                            HorizontalAlignment="Right"
                            Margin="0,0,14,0"
                            VerticalAlignment="Center"
                            Style="{StaticResource ChromeButton}"/>
                </Grid>
            </Border>

            <Grid Grid.Row="1" Margin="30,24,30,24">
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="300"/>
                    <ColumnDefinition Width="24"/>
                    <ColumnDefinition Width="*"/>
                </Grid.ColumnDefinitions>
                <Grid.RowDefinitions>
                    <RowDefinition Height="76"/>
                    <RowDefinition Height="*"/>
                    <RowDefinition Height="64"/>
                </Grid.RowDefinitions>

                <StackPanel Grid.ColumnSpan="3">
                    <TextBlock Text="首次使用向导"
                               FontSize="24"
                               FontWeight="Bold"
                               Foreground="{StaticResource AccentBrush}"/>
                    <TextBlock Text="完成登录、保存 Cookie、验证，然后启动工具。进游戏悬停物品按 Ctrl+C 自动查价。"
                               Margin="0,8,0,0"
                               Foreground="{StaticResource MutedBrush}"
                               FontSize="13"/>
                </StackPanel>

                <Border Grid.Row="1"
                        Grid.Column="0"
                        CornerRadius="16"
                        Background="{StaticResource PanelBrush}"
                        BorderBrush="{StaticResource StrokeBrush}"
                        BorderThickness="1"
                        Padding="18">
                    <Grid>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="*"/>
                            <RowDefinition Height="Auto"/>
                        </Grid.RowDefinitions>
                        <TextBlock Text="设置流程"
                                   Foreground="{StaticResource TextBrush}"
                                   FontSize="15"
                                   FontWeight="Bold"/>
                        <StackPanel Grid.Row="1" Margin="0,18,0,0">
                            <Border x:Name="Step1Shell" CornerRadius="12" Padding="12" Margin="0,0,0,10" Background="#151F2A" BorderBrush="#344457" BorderThickness="1">
                                <DockPanel>
                                    <TextBlock x:Name="Step1Mark" Text="1" Width="24" Foreground="{StaticResource GoldBrush}" FontWeight="Bold"/>
                                    <TextBlock x:Name="Step1Text" Text="微信扫码登录并自动配置" Foreground="{StaticResource TextBrush}" FontWeight="SemiBold"/>
                                </DockPanel>
                            </Border>
                            <Border x:Name="Step2Shell" CornerRadius="12" Padding="12" Margin="0,0,0,10" Background="#101821" BorderBrush="#263445" BorderThickness="1">
                                <DockPanel>
                                    <TextBlock x:Name="Step2Mark" Text="2" Width="24" Foreground="{StaticResource MutedBrush}" FontWeight="Bold"/>
                                    <TextBlock x:Name="Step2Text" Text="自动取得 POESESSID" Foreground="{StaticResource MutedBrush}" FontWeight="SemiBold"/>
                                </DockPanel>
                            </Border>
                            <Border x:Name="Step3Shell" CornerRadius="12" Padding="12" Margin="0,0,0,10" Background="#101821" BorderBrush="#263445" BorderThickness="1">
                                <DockPanel>
                                    <TextBlock x:Name="Step3Mark" Text="3" Width="24" Foreground="{StaticResource MutedBrush}" FontWeight="Bold"/>
                                    <TextBlock x:Name="Step3Text" Text="国服接口验证" Foreground="{StaticResource MutedBrush}" FontWeight="SemiBold"/>
                                </DockPanel>
                            </Border>
                            <Border x:Name="Step4Shell" CornerRadius="12" Padding="12" Background="#101821" BorderBrush="#263445" BorderThickness="1">
                                <DockPanel>
                                    <TextBlock x:Name="Step4Mark" Text="4" Width="24" Foreground="{StaticResource MutedBrush}" FontWeight="Bold"/>
                                    <TextBlock x:Name="Step4Text" Text="启动查价工具" Foreground="{StaticResource MutedBrush}" FontWeight="SemiBold"/>
                                </DockPanel>
                            </Border>
                        </StackPanel>
                        <StackPanel Grid.Row="2" Orientation="Horizontal" Margin="0,18,0,0">
                            <Button x:Name="OpenButton" Content="微信扫码登录并自动配置" Width="132" Style="{StaticResource PrimaryButton}"/>
                            <Button x:Name="PasteButton" Content="高级方式：手动粘贴 Cookie" Width="132" Margin="12,0,0,0" Style="{StaticResource BaseButton}"/>
                        </StackPanel>
                    </Grid>
                </Border>

                <Border x:Name="AdvancedPanel" Visibility="Collapsed" Grid.Row="1"
                        Grid.Column="2"
                        CornerRadius="16"
                        Background="{StaticResource PanelBrush}"
                        BorderBrush="{StaticResource StrokeBrush}"
                        BorderThickness="1"
                        Padding="18">
                    <Grid>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="*"/>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="54"/>
                        </Grid.RowDefinitions>
                        <TextBlock Text="Cookie 内容"
                                   Foreground="{StaticResource TextBrush}"
                                   FontSize="15"
                                   FontWeight="Bold"/>
                        <TextBox x:Name="CookieBox"
                                 Grid.Row="1"
                                 Margin="0,14,0,12"
                                 AcceptsReturn="True"
                                 TextWrapping="Wrap"
                                 Style="{StaticResource TextInput}"/>
                        <TextBlock Grid.Row="2"
                                   Text="默认使用已验证的微信扫码登录。手动粘贴 POESESSID 仅作为高级方式。"
                                   TextWrapping="Wrap"
                                   LineHeight="20"
                                   Foreground="{StaticResource MutedBrush}"
                                   FontSize="12.5"/>
                        <StackPanel Grid.Row="3" Orientation="Horizontal" HorizontalAlignment="Right" VerticalAlignment="Bottom">
                            <Button x:Name="SaveButton" Content="保存并验证" Width="128" Style="{StaticResource PrimaryButton}"/>
                            <Button x:Name="SettingsButton" Content="打开设置" Width="112" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="StartButton" Content="启动工具" Width="112" Margin="10,0,0,0" Style="{StaticResource PrimaryButton}" IsEnabled="False"/>
                        </StackPanel>
                    </Grid>
                </Border>

                <Border x:Name="StatusShell"
                        Grid.Row="2"
                        Grid.ColumnSpan="3"
                        CornerRadius="14"
                        Background="#111A24"
                        BorderBrush="#263445"
                        BorderThickness="1"
                        Padding="16,0"
                        VerticalAlignment="Bottom"
                        Height="48">
                    <DockPanel LastChildFill="True">
                        <Button x:Name="CloseActionButton"
                                Content="关闭"
                                Width="88"
                                Height="32"
                                DockPanel.Dock="Right"
                                Style="{StaticResource BaseButton}"/>
                        <TextBlock x:Name="StatusText"
                                   Text="准备就绪。点击“微信扫码登录并自动配置”开始。"
                                   VerticalAlignment="Center"
                                   Foreground="{StaticResource GoldBrush}"
                                   FontSize="13"/>
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
$openButton = Find-Control 'OpenButton'
$pasteButton = Find-Control 'PasteButton'
$saveButton = Find-Control 'SaveButton'
$settingsButton = Find-Control 'SettingsButton'
$startButton = Find-Control 'StartButton'
$cookieBox = Find-Control 'CookieBox'
$statusText = Find-Control 'StatusText'
$statusShell = Find-Control 'StatusShell'
$advancedPanel = Find-Control 'AdvancedPanel'

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
    $statusText.Text = $Text
    switch ($Kind) {
        'ok' {
            $statusText.Foreground = New-Brush '#32E6A1'
            $statusShell.BorderBrush = New-Brush '#1C8D68'
        }
        'error' {
            $statusText.Foreground = New-Brush '#F87171'
            $statusShell.BorderBrush = New-Brush '#8E303A'
        }
        default {
            $statusText.Foreground = New-Brush '#F4D35E'
            $statusShell.BorderBrush = New-Brush '#384657'
        }
    }
}

function Set-StepState($index, $state) {
    $shell = Find-Control "Step$($index)Shell"
    $mark = Find-Control "Step$($index)Mark"
    $text = Find-Control "Step$($index)Text"
    switch ($state) {
        'done' {
            $shell.Background = New-Brush '#102A24'
            $shell.BorderBrush = New-Brush '#32E6A1'
            $mark.Text = '✓'
            $mark.Foreground = New-Brush '#32E6A1'
            $text.Foreground = New-Brush '#E8F1F8'
        }
        'active' {
            $shell.Background = New-Brush '#2A2615'
            $shell.BorderBrush = New-Brush '#F4D35E'
            $mark.Text = '●'
            $mark.Foreground = New-Brush '#F4D35E'
            $text.Foreground = New-Brush '#F8FAFC'
        }
        'error' {
            $shell.Background = New-Brush '#2B1519'
            $shell.BorderBrush = New-Brush '#F87171'
            $mark.Text = '!'
            $mark.Foreground = New-Brush '#F87171'
            $text.Foreground = New-Brush '#F8FAFC'
        }
        default {
            $shell.Background = New-Brush '#101821'
            $shell.BorderBrush = New-Brush '#263445'
            $mark.Text = [string]$index
            $mark.Foreground = New-Brush '#91A3B8'
            $text.Foreground = New-Brush '#91A3B8'
        }
    }
}

function Update-Ui {
    $window.Dispatcher.Invoke([Action]{}, [System.Windows.Threading.DispatcherPriority]::Render)
}

function Stop-ProcessTree {
    param([System.Diagnostics.Process] $Process)
    if ($null -eq $Process -or $Process.HasExited) { return }
    try {
        & taskkill.exe /PID $Process.Id /T /F *> $null
    } catch {
        try { $Process.Kill() } catch {}
    }
}
# 启动主程序并限制等待时间；超时会终止进程树且不回显 Cookie。
function Invoke-BoundedProcess {
    param(
        [string] $FilePath,
        [string[]] $Arguments,
        [int] $TimeoutSeconds = 30
    )
    try {
        $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -WorkingDirectory $rootPath -PassThru -WindowStyle Hidden
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            Stop-ProcessTree $process
            return [pscustomobject]@{ ExitCode = -1; TimedOut = $true }
        }
        return [pscustomobject]@{ ExitCode = $process.ExitCode; TimedOut = $false }
    } catch {
        return [pscustomobject]@{ ExitCode = -1; TimedOut = $false }
    }
}
function Invoke-MainValidation {
    if (-not $exe -or -not (Test-Path -LiteralPath $exe)) { return $false }
    $validation = Invoke-BoundedProcess $exe @('--validate-cookie') 30
    return (-not $validation.TimedOut) -and $validation.ExitCode -eq 0
}

function Invoke-WechatLogin {
    <# 启动独立微信登录助手，使用固定发布目录并限制等待时长，不读取或回显 Cookie。 #>
    if (-not (Test-Path -LiteralPath $loginHelper)) { return 'missing' }
    if (-not (Test-Path -LiteralPath $bridgeExe)) { return 'bridge-missing' }
    $hadValidCookie = Invoke-MainValidation
    $process = $null
    try {
        $process = Start-Process -FilePath $loginHelper -WorkingDirectory $rootPath -PassThru
        $deadline = [DateTime]::UtcNow.AddSeconds($loginTimeoutSeconds)
        while (-not $process.HasExited -and [DateTime]::UtcNow -lt $deadline) {
            Update-Ui
            Start-Sleep -Milliseconds 200
        }
        if (-not $process.HasExited) { Stop-ProcessTree $process; return 'timeout' }
        if ($process.ExitCode -eq 2) { return 'cancelled' }
        if ($process.ExitCode -eq 6) { return 'runtime-missing' }
        if ($process.ExitCode -ne 0) { return 'failed' }
    } catch { return 'failed' }
    if (Invoke-MainValidation) {
        if ($hadValidCookie) { return 'existing' }
        return 'success'
    }
    return 'validation-failed'
}
$titleBar.Add_MouseLeftButtonDown({
    try {
        $window.DragMove()
    } catch {}
})

$closeHandler = {
    $window.Close()
}
$closeButton.add_Click($closeHandler)
$closeActionButton.add_Click($closeHandler)

$openHandler = {
    if (-not (Test-Path -LiteralPath $loginHelper)) {
        Set-StepState 1 'error'
        Set-Status -Text '找不到 POE2PriceLogin.exe，请确认发布包完整。' -Kind 'error'
        return
    }
    Set-StepState 1 'active'
    Set-Status -Text '正在打开微信登录助手，请扫码完成登录……'
    Update-Ui
    $result = Invoke-WechatLogin
    switch ($result) {
        'success' {
            Set-StepState 1 'done'; Set-StepState 2 'done'; Set-StepState 3 'done'; Set-StepState 4 'active'
            $startButton.IsEnabled = $true
            Set-Status -Text '登录验证成功，可以启动工具。' -Kind 'ok'
        }
        'existing' {
            Set-StepState 1 'done'; Set-StepState 2 'done'; Set-StepState 3 'done'; Set-StepState 4 'active'
            $startButton.IsEnabled = $true
            Set-Status -Text '登录助手已关闭，现有 Cookie 仍验证有效。' -Kind 'ok'
        }
        'missing' {
            Set-StepState 1 'error'
            Set-Status -Text '找不到 POE2PriceLogin.exe，请重新解压完整发布包。' -Kind 'error'
        }
        'bridge-missing' {
            Set-StepState 1 'error'
            Set-Status -Text '找不到主程序，无法完成登录配置。' -Kind 'error'
        }
        'timeout' {
            Set-StepState 1 'error'
            Set-Status -Text '登录超时，已安全终止登录助手；请重试。' -Kind 'error'
        }
        'cancelled' {
            Set-StepState 1 'active'
            Set-Status -Text '登录已取消，未覆盖现有 Cookie。'
        }
        'runtime-missing' {
            Set-StepState 1 'error'
            Set-Status -Text '未检测到 WebView2 Runtime，请安装微软官方运行环境后重试。' -Kind 'error'
        }
        'validation-failed' {
            Set-StepState 3 'error'
            Set-Status -Text '登录完成但国服验证失败，请重试。' -Kind 'error'
        }
        default {
            Set-StepState 1 'error'
            Set-Status -Text '登录助手启动或退出异常；未保存明文 Cookie。' -Kind 'error'
        }
    }
}
$openButton.add_Click($openHandler)

$pasteHandler = {
    if ($advancedPanel) {
        $advancedPanel.Visibility = [System.Windows.Visibility]::Visible
    }
    $hasClipboardText = [System.Windows.Clipboard]::ContainsText()
    if (-not $hasClipboardText) {
        Set-Status -Text '高级方式已展开；剪贴板没有文本，请手动粘贴。'
        return
    }
    $clip = [System.Windows.Clipboard]::GetText()
    $cookieBox.Text = $clip
    if ((Test-CookieInput $clip)) {
        Set-StepState 2 'done'; Set-StepState 3 'active'
        Set-Status -Text '已识别到可能的 POESESSID，点击“保存并验证”。' -Kind 'ok'
        return
    }
    Set-StepState 2 'error'
    Set-Status -Text '已粘贴，但没有明显识别到 POESESSID。' -Kind 'error'
}
$pasteButton.add_Click($pasteHandler)
$saveHandler = {
    $exeAvailable = $false
    if ($exe) {
        $exeAvailable = Test-Path -LiteralPath $exe
    }
    if (-not $exeAvailable) {
        Set-StepState 3 'error'
        Set-Status -Text '找不到主程序，请确认向导和 exe 在同一个发布包里。' -Kind 'error'
        return
    }

    $cookieText = $cookieBox.Text
    $cookieLooksValid = Test-CookieInput $cookieText
    if (-not $cookieLooksValid) {
        Set-StepState 2 'error'
        Set-Status -Text '先粘贴 POESESSID 值或完整 Cookie 请求头。' -Kind 'error'
        return
    }

    $tmp = $null
    try {
        $tmp = [System.IO.Path]::GetTempFileName()
        [System.IO.File]::WriteAllText($tmp, $cookieText, [System.Text.UTF8Encoding]::new($false))
        Set-Status -Text '正在加密保存 Cookie...'
        Update-Ui
        $saveProcess = Invoke-BoundedProcess $exe @('--set-cookie-file', $tmp) 30
        if ($saveProcess.TimedOut) {
            Set-Status -Text '保存 Cookie 超时，主程序已终止。' -Kind 'error'
            return
        }
        if ($saveProcess.ExitCode -ne 0) {
            Set-StepState 3 'error'
            Set-Status -Text "保存失败，退出码: $($saveProcess.ExitCode)" -Kind 'error'
            return
        }

        Set-Status -Text '已保存，正在请求国服 trade2 验证...'
        Update-Ui
        $validation = Invoke-BoundedProcess $exe @('--validate-cookie') 30
    } finally {
        if ($tmp -and (Test-Path -LiteralPath $tmp)) {
            Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
        }
        $cookieText = $null
    }
    if ($validation.TimedOut) {
        Set-Status -Text '国服验证超时，主程序已终止。' -Kind 'error'
        return
    }
    if ($validation.ExitCode -ne 0) {
        Set-StepState 3 'error'
        Set-Status -Text '已保存，但验证失败。请重新登录国服市集并复制新的 POESESSID。' -Kind 'error'
        return
    }

    Set-StepState 3 'done'
    Set-StepState 4 'active'
    $startButton.IsEnabled = $true
    Set-Status -Text 'Cookie 验证通过。现在可以启动工具。' -Kind 'ok'
}
$saveButton.add_Click($saveHandler)

$settingsHandler = {
    $settingsAvailable = Test-Path -LiteralPath $settingsScript
    if ($settingsAvailable) {
        Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File', $settingsScript, '-Root', $rootPath)
        Set-Status -Text '已打开设置窗口。'
        return
    }

    Set-Status -Text '找不到设置脚本。' -Kind 'error'
}
$settingsButton.add_Click($settingsHandler)

$startHandler = {
    $exeAvailable = $false
    if ($exe) {
        $exeAvailable = Test-Path -LiteralPath $exe
    }
    if (-not $exeAvailable) {
        Set-Status -Text '找不到主程序，请确认发布包完整。' -Kind 'error'
        return
    }

    Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
    Set-StepState 4 'done'
    Set-Status -Text '工具已启动。进游戏悬停物品按 Ctrl+C 即可查价。' -Kind 'ok'
}
$startButton.add_Click($startHandler)

$contentRenderedHandler = {
    $hasClipboardText = [System.Windows.Clipboard]::ContainsText()
    if (-not $hasClipboardText) {
        return
    }
    $clip = [System.Windows.Clipboard]::GetText()
    $cookieLooksValid = Test-CookieInput $clip
    if (-not $cookieLooksValid) {
        return
    }
    $cookieBox.Text = $clip
    Set-StepState 2 'done'
    Set-StepState 3 'active'
    Set-Status -Text '已自动识别剪贴板里的 POESESSID，可以直接保存并验证。' -Kind 'ok'
}
$window.add_ContentRendered($contentRenderedHandler)

[void] $window.ShowDialog()
