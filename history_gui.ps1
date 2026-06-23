param(
    [Parameter(Mandatory = $true)]
    [string] $Root
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$rootPath = (Resolve-Path -LiteralPath $Root).Path
$appDir = Join-Path $env:APPDATA 'poe2_cn_price_bridge'
$historyPath = Join-Path $appDir 'history.jsonl'
$exeCandidates = @(
    (Join-Path $rootPath 'QingPricePOE2.exe'),
    (Join-Path $rootPath 'poe2_cn_price_bridge.exe'),
    (Join-Path $rootPath 'target\release\poe2_cn_price_bridge.exe')
)
$exe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

function UnixToLocal($value) {
    try {
        return [DateTimeOffset]::FromUnixTimeSeconds([int64]$value).LocalDateTime.ToString('yyyy-MM-dd HH:mm:ss')
    } catch {
        return ''
    }
}

function Read-History {
    if (-not (Test-Path -LiteralPath $historyPath)) {
        return @()
    }

    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($line in [System.IO.File]::ReadLines($historyPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        try {
            $row = $line | ConvertFrom-Json
            $filter = @()
            if ($row.used_mods) { $filter += '同属性' }
            if ($row.used_values) { $filter += '数值' }
            $itemName = if ($row.item) { [string]$row.item } else { [string]$row.base_type }
            $rows.Add([pscustomobject]@{
                Time = UnixToLocal $row.ts
                Status = [string]$row.status
                Item = $itemName
                Rarity = [string]$row.rarity
                League = [string]$row.league
                Total = [string]$row.total
                Price = [string]$row.priced
                Filter = ($filter -join '+')
                Message = [string]$row.message
                Url = [string]$row.url
            })
        } catch {}
    }

    return @($rows | Sort-Object Time -Descending | Select-Object -First 100)
}

[xml]$xaml = @"
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="清价 POE2 - 查询历史"
        Width="1040" Height="620"
        WindowStartupLocation="CenterScreen"
        WindowStyle="None"
        ResizeMode="CanResize"
        AllowsTransparency="True"
        Background="Transparent"
        FontFamily="Microsoft YaHei UI">
    <Window.Resources>
        <SolidColorBrush x:Key="PageBrush" Color="#090D14"/>
        <SolidColorBrush x:Key="PanelBrush" Color="#101720"/>
        <SolidColorBrush x:Key="StrokeBrush" Color="#263445"/>
        <SolidColorBrush x:Key="TextBrush" Color="#E8F1F8"/>
        <SolidColorBrush x:Key="MutedBrush" Color="#91A3B8"/>
        <SolidColorBrush x:Key="AccentBrush" Color="#32E6A1"/>
        <SolidColorBrush x:Key="GoldBrush" Color="#F4D35E"/>

        <Style x:Key="BaseButton" TargetType="{x:Type Button}">
            <Setter Property="Height" Value="38"/>
            <Setter Property="Padding" Value="16,0"/>
            <Setter Property="Foreground" Value="#E8F1F8"/>
            <Setter Property="Background" Value="#172230"/>
            <Setter Property="BorderBrush" Value="#314154"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="FontSize" Value="13"/>
            <Setter Property="FontWeight" Value="SemiBold"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="{x:Type Button}">
                        <Border x:Name="Bd" CornerRadius="10" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}">
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#223145"/>
                                <Setter TargetName="Bd" Property="BorderBrush" Value="#4B6078"/>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Setter TargetName="Bd" Property="Background" Value="#0F1824"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>

        <Style x:Key="PrimaryButton" TargetType="{x:Type Button}" BasedOn="{StaticResource BaseButton}">
            <Setter Property="Background" Value="#137A5B"/>
            <Setter Property="BorderBrush" Value="#32E6A1"/>
            <Setter Property="Foreground" Value="#F0FFF8"/>
        </Style>

        <Style TargetType="{x:Type GridViewColumnHeader}">
            <Setter Property="Background" Value="#172230"/>
            <Setter Property="Foreground" Value="#BFD0E0"/>
            <Setter Property="BorderBrush" Value="#263445"/>
            <Setter Property="BorderThickness" Value="0,0,1,1"/>
            <Setter Property="Padding" Value="10,8"/>
            <Setter Property="FontWeight" Value="SemiBold"/>
        </Style>

        <Style TargetType="{x:Type ListViewItem}">
            <Setter Property="Foreground" Value="#E8F1F8"/>
            <Setter Property="Background" Value="#101720"/>
            <Setter Property="Padding" Value="6,4"/>
            <Setter Property="HorizontalContentAlignment" Value="Stretch"/>
            <Style.Triggers>
                <Trigger Property="IsMouseOver" Value="True">
                    <Setter Property="Background" Value="#172230"/>
                </Trigger>
                <Trigger Property="IsSelected" Value="True">
                    <Setter Property="Background" Value="#113A31"/>
                    <Setter Property="Foreground" Value="#F0FFF8"/>
                </Trigger>
            </Style.Triggers>
        </Style>
    </Window.Resources>

    <Border Margin="8" CornerRadius="18" Background="{StaticResource PageBrush}" BorderBrush="#23C7E8" BorderThickness="1">
        <Border.Effect>
            <DropShadowEffect BlurRadius="28" ShadowDepth="0" Opacity="0.42" Color="#000000"/>
        </Border.Effect>
        <Grid>
            <Grid.RowDefinitions>
                <RowDefinition Height="52"/>
                <RowDefinition Height="*"/>
            </Grid.RowDefinitions>

            <Border x:Name="TitleBar" Grid.Row="0" CornerRadius="18,18,0,0" Background="#0B111A">
                <Grid>
                    <TextBlock Text="清价 POE2" Margin="22,0,0,0" VerticalAlignment="Center" FontSize="13" FontWeight="Bold" Foreground="#E8F1F8"/>
                    <Button x:Name="CloseButton" Content="×" Width="38" Height="30" HorizontalAlignment="Right" Margin="0,0,14,0" VerticalAlignment="Center" Background="Transparent" BorderThickness="0" Foreground="#9FB3C8" FontSize="18" Cursor="Hand"/>
                </Grid>
            </Border>

            <Grid Grid.Row="1" Margin="28,24,28,22">
                <Grid.RowDefinitions>
                    <RowDefinition Height="78"/>
                    <RowDefinition Height="*"/>
                    <RowDefinition Height="58"/>
                </Grid.RowDefinitions>

                <Grid>
                    <StackPanel>
                        <TextBlock Text="查询历史" FontSize="24" FontWeight="Bold" Foreground="{StaticResource AccentBrush}"/>
                        <TextBlock x:Name="SubtitleText" Text="最近 100 条查价记录，可打开市集链接或复制链接。" Margin="0,8,0,0" Foreground="{StaticResource MutedBrush}" FontSize="13"/>
                    </StackPanel>
                    <Border HorizontalAlignment="Right" VerticalAlignment="Top" CornerRadius="999" Background="#181F2A" BorderBrush="#344457" BorderThickness="1" Padding="16,7">
                        <TextBlock x:Name="CountText" Text="0 条" Foreground="{StaticResource GoldBrush}" FontWeight="SemiBold"/>
                    </Border>
                </Grid>

                <Border Grid.Row="1" CornerRadius="16" Background="{StaticResource PanelBrush}" BorderBrush="{StaticResource StrokeBrush}" BorderThickness="1" Padding="12">
                    <ListView x:Name="HistoryList" Background="#101720" BorderThickness="0" Foreground="{StaticResource TextBrush}" ScrollViewer.HorizontalScrollBarVisibility="Auto" ScrollViewer.VerticalScrollBarVisibility="Auto">
                        <ListView.View>
                            <GridView>
                                <GridViewColumn Header="时间" Width="150" DisplayMemberBinding="{Binding Time}"/>
                                <GridViewColumn Header="状态" Width="80" DisplayMemberBinding="{Binding Status}"/>
                                <GridViewColumn Header="物品" Width="230" DisplayMemberBinding="{Binding Item}"/>
                                <GridViewColumn Header="稀有度" Width="80" DisplayMemberBinding="{Binding Rarity}"/>
                                <GridViewColumn Header="联赛" Width="110" DisplayMemberBinding="{Binding League}"/>
                                <GridViewColumn Header="总数" Width="70" DisplayMemberBinding="{Binding Total}"/>
                                <GridViewColumn Header="价格" Width="120" DisplayMemberBinding="{Binding Price}"/>
                                <GridViewColumn Header="筛选" Width="90" DisplayMemberBinding="{Binding Filter}"/>
                                <GridViewColumn Header="信息" Width="240" DisplayMemberBinding="{Binding Message}"/>
                            </GridView>
                        </ListView.View>
                    </ListView>
                </Border>

                <Border x:Name="StatusShell" Grid.Row="2" CornerRadius="14" Background="#111A24" BorderBrush="#263445" BorderThickness="1" Padding="16,0" VerticalAlignment="Bottom" Height="46">
                    <DockPanel LastChildFill="True">
                        <StackPanel DockPanel.Dock="Right" Orientation="Horizontal">
                            <Button x:Name="RefreshButton" Content="刷新" Width="80" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="OpenButton" Content="打开链接" Width="96" Margin="10,0,0,0" Style="{StaticResource PrimaryButton}"/>
                            <Button x:Name="CopyButton" Content="复制链接" Width="96" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="DiagnosticsButton" Content="导出诊断" Width="96" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                            <Button x:Name="CloseActionButton" Content="关闭" Width="80" Margin="10,0,0,0" Style="{StaticResource BaseButton}"/>
                        </StackPanel>
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
$historyList = Find-Control 'HistoryList'
$countText = Find-Control 'CountText'
$subtitleText = Find-Control 'SubtitleText'
$status = Find-Control 'StatusText'
$statusShell = Find-Control 'StatusShell'
$refresh = Find-Control 'RefreshButton'
$open = Find-Control 'OpenButton'
$copy = Find-Control 'CopyButton'
$diagnostics = Find-Control 'DiagnosticsButton'

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

function Load-Grid {
    $rows = @(Read-History)
    $historyList.ItemsSource = $rows
    $countText.Text = "$($rows.Count) 条"
    if ($rows.Count -eq 0) {
        Set-Status -Text "暂无历史记录。历史文件: $historyPath"
        return
    }
    Set-Status -Text "历史文件: $historyPath" -Kind 'ok'
}

function Selected-Link {
    $selected = $historyList.SelectedItem
    if ($null -eq $selected) {
        return $null
    }
    $value = [string]$selected.Url
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $null
    }
    return $value
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

$refresh.add_Click({
    Load-Grid
})

$open.add_Click({
    $link = Selected-Link
    if ($link) {
        Start-Process $link
        Set-Status -Text '已打开市集链接。' -Kind 'ok'
        return
    }
    Set-Status -Text '当前记录没有市集链接。' -Kind 'error'
})

$copy.add_Click({
    $link = Selected-Link
    if ($link) {
        [System.Windows.Clipboard]::SetText($link)
        Set-Status -Text '已复制链接。' -Kind 'ok'
        return
    }
    Set-Status -Text '当前记录没有市集链接。' -Kind 'error'
})

$diagnostics.add_Click({
    if (-not $exe) {
        Set-Status -Text '找不到主程序。' -Kind 'error'
        return
    }
    $target = Join-Path $rootPath 'diagnostics.txt'
    $process = Start-Process -FilePath $exe -ArgumentList @('--diagnostics', $target) -Wait -PassThru
    if ($process.ExitCode -eq 0) {
        Start-Process $target
        Set-Status -Text "诊断已导出: $target" -Kind 'ok'
        return
    }
    Set-Status -Text "导出诊断失败，退出码: $($process.ExitCode)" -Kind 'error'
})

$historyList.add_MouseDoubleClick({
    $link = Selected-Link
    if ($link) {
        Start-Process $link
    }
})

Load-Grid
[void] $window.ShowDialog()
