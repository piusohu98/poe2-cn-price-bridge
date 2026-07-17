param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

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
        Title="流放2查价助手 - Cookie 设置"
        Width="840" Height="520"
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
                        <Border x:Name="Bd" CornerRadius="14" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}">
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
                        <Border Width="28" Height="28" CornerRadius="9" Background="#5B3710" BorderBrush="#C78A2B" BorderThickness="1">
                            <TextBlock Text="价" HorizontalAlignment="Center" VerticalAlignment="Center" Foreground="#FFF4D6" FontSize="15" FontWeight="Bold"/>
                        </Border>
                        <TextBlock Text="流放2查价助手" Margin="10,0,0,0" VerticalAlignment="Center" FontSize="13" FontWeight="Bold" Foreground="#E8F1F8"/>
                    </StackPanel>
                    <Button x:Name="CloseButton" Content="×" HorizontalAlignment="Right" Margin="0,0,14,0" VerticalAlignment="Center" Style="{StaticResource ChromeButton}"/>
                </Grid>
            </Border>

            <Grid Grid.Row="1" Margin="28,24,28,22">
                <Grid.RowDefinitions>
                    <RowDefinition Height="72"/>
                    <RowDefinition Height="*"/>
                    <RowDefinition Height="58"/>
                </Grid.RowDefinitions>
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="270"/>
                    <ColumnDefinition Width="24"/>
                    <ColumnDefinition Width="*"/>
                </Grid.ColumnDefinitions>

                <StackPanel Grid.ColumnSpan="3">
                    <TextBlock Text="微信扫码登录并自动配置" FontSize="23" FontWeight="Bold" Foreground="{StaticResource AccentBrush}"/>
                    <TextBlock Text="微信扫码登录会自动取得并验证 POESESSID；手动方式仅供高级用户。" Margin="0,8,0,0" Foreground="{StaticResource MutedBrush}" FontSize="13"/>
                </StackPanel>

                <Border Grid.Row="1" Grid.Column="0" CornerRadius="16" Background="{StaticResource PanelBrush}" BorderBrush="{StaticResource StrokeBrush}" BorderThickness="1" Padding="18">
                    <Grid>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="*"/>
                            <RowDefinition Height="Auto"/>
                        </Grid.RowDefinitions>
                        <TextBlock Text="获取步骤" Foreground="{StaticResource TextBrush}" FontSize="15" FontWeight="Bold"/>
                        <StackPanel Grid.Row="1" Margin="0,18,0,0">
                            <TextBlock Text="1. 点击微信扫码登录并自动配置" Foreground="{StaticResource TextBrush}" FontWeight="SemiBold" Margin="0,0,0,12"/>
                            <TextBlock Text="2. 使用微信完成扫码登录" Foreground="{StaticResource MutedBrush}" TextWrapping="Wrap" Margin="0,0,0,12"/>
                            <TextBlock Text="3. 登录助手自动取得并验证国服 Cookie" Foreground="{StaticResource MutedBrush}" TextWrapping="Wrap" Margin="0,0,0,12"/>
                            <TextBlock Text="4. 成功后自动加密保存，无需浏览器开发者工具" Foreground="{StaticResource MutedBrush}" TextWrapping="Wrap"/>
                        </StackPanel>
                        <StackPanel Grid.Row="2">
                            <Button x:Name="OpenButton" Content="微信扫码登录并自动配置" Style="{StaticResource PrimaryButton}" Margin="0,0,0,10"/>
                            <Button x:Name="CopyHelpButton" Content="高级方式：手动粘贴 Cookie" Style="{StaticResource BaseButton}" Margin="0,0,0,10"/>
                            <Button x:Name="PasteButton" Content="从剪贴板识别" Style="{StaticResource BaseButton}"/>
                        </StackPanel>
                    </Grid>
                </Border>

                <Border x:Name="AdvancedPanel" Visibility="Collapsed" Grid.Row="1" Grid.Column="2" CornerRadius="16" Background="{StaticResource PanelBrush}" BorderBrush="{StaticResource StrokeBrush}" BorderThickness="1" Padding="18">
                    <Grid>
                        <Grid.RowDefinitions>
                            <RowDefinition Height="Auto"/>
                            <RowDefinition Height="*"/>
                            <RowDefinition Height="54"/>
                        </Grid.RowDefinitions>
                        <DockPanel>
                            <Button x:Name="ClearButton" DockPanel.Dock="Right" Content="清空" Width="74" Height="32" Style="{StaticResource BaseButton}"/>
                            <TextBlock Text="高级方式：粘贴 POESESSID 值或完整 Cookie 请求头" Foreground="{StaticResource TextBrush}" FontSize="15" FontWeight="Bold" VerticalAlignment="Center"/>
                        </DockPanel>
                        <TextBox x:Name="CookieBox" Grid.Row="1" Margin="0,14,0,12" AcceptsReturn="True" TextWrapping="Wrap" Style="{StaticResource TextInput}"/>
                        <StackPanel Grid.Row="2" Orientation="Horizontal" HorizontalAlignment="Right" VerticalAlignment="Bottom">
                            <Button x:Name="SaveButton" Content="保存并验证" Width="124" Style="{StaticResource PrimaryButton}"/>
                            <Button x:Name="ValidateButton" Content="验证现有" Width="108" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="StartButton" Content="启动工具" Width="108" Margin="10,0,0,0" Style="{StaticResource PrimaryButton}" IsEnabled="False"/>
                        </StackPanel>
                    </Grid>
                </Border>

                <Border x:Name="StatusShell" Grid.Row="2" Grid.ColumnSpan="3" CornerRadius="14" Background="#111A24" BorderBrush="#263445" BorderThickness="1" Padding="16,0" VerticalAlignment="Bottom" Height="46">
                    <DockPanel LastChildFill="True">
                        <Button x:Name="CloseActionButton" Content="关闭" Width="88" Height="32" DockPanel.Dock="Right" Style="{StaticResource BaseButton}"/>
                        <TextBlock x:Name="StatusText" Text="准备就绪。复制 POESESSID 后保存并验证。" VerticalAlignment="Center" Foreground="{StaticResource GoldBrush}" FontSize="13"/>
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
$copyHelpButton = Find-Control 'CopyHelpButton'
$pasteButton = Find-Control 'PasteButton'
$clearButton = Find-Control 'ClearButton'
$saveButton = Find-Control 'SaveButton'
$validateButton = Find-Control 'ValidateButton'
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

function Update-Ui {
    $window.Dispatcher.Invoke([Action]{}, [System.Windows.Threading.DispatcherPriority]::Render)
}

function Stop-ProcessTree {
    param([System.Diagnostics.Process] $Process)
    if ($null -eq $Process -or $Process.HasExited) { return }
    try { & taskkill.exe /PID $Process.Id /T /F *> $null } catch { try { $Process.Kill() } catch {} }
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
    <# 启动固定发布目录中的微信登录助手，等待有界且不读取或回显 Cookie。 #>
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
function Save-And-ValidateCookie {
    $exeAvailable = $false
    if ($exe) {
        $exeAvailable = Test-Path -LiteralPath $exe
    }
    if (-not $exeAvailable) {
        Set-Status -Text '找不到 POE2PriceHelper.exe，请确认从完整发布包中运行。' -Kind 'error'
        return
    }

    $cookieText = $cookieBox.Text
    $cookieLooksValid = Test-CookieInput $cookieText
    if (-not $cookieLooksValid) {
        Set-Status -Text '没有识别到 POESESSID。请粘贴 POESESSID 值或完整 Cookie 请求头。' -Kind 'error'
        return
    }

    $tmp = $null
    try {
        $tmp = [System.IO.Path]::GetTempFileName()
        [System.IO.File]::WriteAllText($tmp, $cookieText, [System.Text.UTF8Encoding]::new($false))
        Set-Status -Text '正在加密保存 Cookie...'
        Update-Ui
        $process = Invoke-BoundedProcess $exe @('--set-cookie-file', $tmp) 30
        if ($process.TimedOut) {
            Set-Status -Text '保存 Cookie 超时，主程序已终止。' -Kind 'error'
            return
        }
        if ($process.ExitCode -ne 0) {
            Set-Status -Text "保存失败，退出码: $($process.ExitCode)" -Kind 'error'
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
        Set-Status -Text '已保存，但验证失败。请重新登录国服市集并复制新的 POESESSID。' -Kind 'error'
        return
    }

    $startButton.IsEnabled = $true
    Set-Status -Text 'Cookie 已保存并验证通过，可以启动工具。' -Kind 'ok'
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

$openButton.add_Click({
    if (-not (Test-Path -LiteralPath $loginHelper)) {
        Set-Status -Text '找不到 POE2PriceLogin.exe，请确认发布包完整。' -Kind 'error'
        return
    }
    Set-Status -Text '正在打开微信登录助手，请扫码完成登录……'
    Update-Ui
    $result = Invoke-WechatLogin
    switch ($result) {
        'success' { $startButton.IsEnabled = $true; Set-Status -Text '登录验证成功，POESESSID 已加密保存。' -Kind 'ok' }
        'existing' { $startButton.IsEnabled = $true; Set-Status -Text '登录助手已关闭，现有 Cookie 仍验证有效。' -Kind 'ok' }
        'missing' { Set-Status -Text '找不到 POE2PriceLogin.exe，请重新解压完整发布包。' -Kind 'error' }
        'bridge-missing' { Set-Status -Text '找不到主程序，无法完成登录配置。' -Kind 'error' }
        'timeout' { Set-Status -Text '登录超时，已安全终止登录助手；请重试。' -Kind 'error' }
        'cancelled' { Set-Status -Text '登录已取消，未覆盖现有 Cookie。' }
        'runtime-missing' { Set-Status -Text '未检测到 WebView2 Runtime，请安装微软官方运行环境后重试。' -Kind 'error' }
        'validation-failed' { Set-Status -Text '登录完成但国服验证失败，请重试。' -Kind 'error' }
        default { Set-Status -Text '登录助手启动或退出异常；未保存明文 Cookie。' -Kind 'error' }
    }
})

$copyHelpButton.add_Click({
    if ($advancedPanel) { $advancedPanel.Visibility = [System.Windows.Visibility]::Visible }
    Set-Status -Text '高级方式已展开；请手动粘贴 POESESSID 或完整 Cookie 请求头。'
})

$pasteButton.add_Click({
    if ($advancedPanel) { $advancedPanel.Visibility = [System.Windows.Visibility]::Visible }
    $hasClipboardText = [System.Windows.Clipboard]::ContainsText()
    if (-not $hasClipboardText) {
        Set-Status -Text '剪贴板没有文本，请手动粘贴。' -Kind 'error'
        return
    }
    $clip = [System.Windows.Clipboard]::GetText()
    $cookieBox.Text = $clip
    $cookieLooksValid = Test-CookieInput $clip
    if ($cookieLooksValid) {
        Set-Status -Text '已识别到可能的 POESESSID，可以保存并验证。' -Kind 'ok'
        return
    }
    Set-Status -Text '已粘贴，但没有明显识别到 POESESSID。' -Kind 'error'
})
$clearButton.add_Click({
    $cookieBox.Clear()
    $startButton.IsEnabled = $false
    Set-Status -Text '已清空输入。'
})

$saveButton.add_Click({ Save-And-ValidateCookie })

$validateButton.add_Click({
    $exeAvailable = $false
    if ($exe) {
        $exeAvailable = Test-Path -LiteralPath $exe
    }
    if (-not $exeAvailable) {
        Set-Status -Text '找不到 POE2PriceHelper.exe。' -Kind 'error'
        return
    }
    Set-Status -Text '正在验证现有 Cookie...'
    Update-Ui
    $validation = Invoke-BoundedProcess $exe @('--validate-cookie') 30
    if ($validation.TimedOut) {
        Set-Status -Text '验证现有 Cookie 超时，主程序已终止。' -Kind 'error'
        return
    }
    if ($validation.ExitCode -eq 0) {
        $startButton.IsEnabled = $true
        Set-Status -Text '现有 Cookie 验证通过。' -Kind 'ok'
        return
    }
    Set-Status -Text '现有 Cookie 不可用，请重新登录并保存新的 POESESSID。' -Kind 'error'
})

$startButton.add_Click({
    $exeAvailable = $false
    if ($exe) {
        $exeAvailable = Test-Path -LiteralPath $exe
    }
    if (-not $exeAvailable) {
        Set-Status -Text '找不到主程序，请确认发布包完整。' -Kind 'error'
        return
    }
    Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
    Set-Status -Text '工具已启动。进游戏悬停物品按 Ctrl+C 即可查价。' -Kind 'ok'
})

$window.add_ContentRendered({
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
    Set-Status -Text '已自动识别剪贴板里的 POESESSID，可以直接保存并验证。' -Kind 'ok'
})

[void] $window.ShowDialog()
