using Microsoft.Web.WebView2.Core;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Threading;

namespace QingPriceLogin
{
    public partial class MainWindow : Window
    {
        private readonly string _bridgePath;
        private readonly string _userDataFolder;
        private readonly DispatcherTimer _cookieTimer;
        private bool _checkingCookie;
        private string _lastAttemptedCookieFingerprint;
        private bool _cleanupStarted;
        private bool _allowClose;

        internal MainWindow(string bridgePath)
        {
            InitializeComponent();
            _bridgePath = string.IsNullOrWhiteSpace(bridgePath)
                ? Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "QingPricePOE2.exe")
                : Path.GetFullPath(bridgePath);
            _userDataFolder = LoginPolicy.CreateUserDataFolder();
            _cookieTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2) };
            _cookieTimer.Tick += CookieTimer_Tick;
            Loaded += MainWindow_Loaded;
        }

        /// 初始化独立 WebView2 环境；不复用系统浏览器用户数据。
        private async void MainWindow_Loaded(object sender, RoutedEventArgs e)
        {
            await InitializeBrowserAsync();
        }

        private async Task InitializeBrowserAsync()
        {
            try
            {
                CoreWebView2Environment.GetAvailableBrowserVersionString();
                Directory.CreateDirectory(_userDataFolder);
                var environment = await CoreWebView2Environment.CreateAsync(null, _userDataFolder);
                await Browser.EnsureCoreWebView2Async(environment);
                ConfigureBrowser();
                CheckButton.IsEnabled = true;
                StatusText.Text = "请完成 QQ 登录；登录成功后助手会自动检查。";
                _cookieTimer.Start();
                Browser.Source = new Uri(LoginPolicy.TradeUri);
            }
            catch (WebView2RuntimeNotFoundException)
            {
                StatusText.Text = "未检测到 Microsoft Edge WebView2 Runtime。";
                var result = MessageBox.Show(
                    "此登录 PoC 需要 Microsoft Edge WebView2 Evergreen Runtime。是否打开微软官方下载页面？",
                    "缺少 WebView2 Runtime",
                    MessageBoxButton.YesNo,
                    MessageBoxImage.Warning);
                if (result == MessageBoxResult.Yes)
                {
                    Process.Start(new ProcessStartInfo(LoginPolicy.RuntimeDownloadUri) { UseShellExecute = true });
                }
                Close();
            }
            catch (Exception)
            {
                StatusText.Text = "WebView2 初始化失败。";
                MessageBox.Show("登录窗口初始化失败；未记录异常详情或任何 Cookie。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Error);
                Close();
            }
        }

        /// 关闭开发工具、下载和权限请求，并对顶层、框架及弹窗导航使用同一白名单。
        private void ConfigureBrowser()
        {
            var core = Browser.CoreWebView2;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.NavigationStarting += Core_NavigationStarting;
            core.FrameNavigationStarting += Core_FrameNavigationStarting;
            core.NewWindowRequested += Core_NewWindowRequested;
            core.DownloadStarting += (sender, args) => args.Cancel = true;
            core.PermissionRequested += (sender, args) => args.State = CoreWebView2PermissionState.Deny;
            core.NavigationCompleted += Core_NavigationCompleted;
        }

        private void Core_NavigationStarting(object sender, CoreWebView2NavigationStartingEventArgs e)
        {
            if (!IsAllowedUri(e.Uri))
            {
                e.Cancel = true;
                ShowBlockedNavigation(e.Uri);
            }
        }

        private void Core_FrameNavigationStarting(object sender, CoreWebView2NavigationStartingEventArgs e)
        {
            if (!IsAllowedUri(e.Uri))
            {
                e.Cancel = true;
                ShowBlockedNavigation(e.Uri);
            }
        }

        private void Core_NewWindowRequested(object sender, CoreWebView2NewWindowRequestedEventArgs e)
        {
            e.Handled = true;
            if (IsAllowedUri(e.Uri))
            {
                Browser.CoreWebView2.Navigate(e.Uri);
            }
            else
            {
                ShowBlockedNavigation(e.Uri);
            }
        }

        private async void Core_NavigationCompleted(object sender, CoreWebView2NavigationCompletedEventArgs e)
        {
            await CheckLoginAsync(false);
        }

        private async void CookieTimer_Tick(object sender, EventArgs e)
        {
            await CheckLoginAsync(false);
        }

        private async void CheckButton_Click(object sender, RoutedEventArgs e)
        {
            await CheckLoginAsync(true);
        }

        private void CloseButton_Click(object sender, RoutedEventArgs e)
        {
            Close();
        }

        private static bool IsAllowedUri(string rawUri)
        {
            Uri uri;
            return Uri.TryCreate(rawUri, UriKind.Absolute, out uri) && LoginPolicy.IsAllowedNavigation(uri);
        }

        /// 被拦截时只显示主机名，不回显完整查询参数。
        private void ShowBlockedNavigation(string rawUri)
        {
            Uri uri;
            var host = Uri.TryCreate(rawUri, UriKind.Absolute, out uri) ? uri.IdnHost : "未知地址";
            StatusText.Text = "已阻止不在白名单中的导航：" + host;
        }

        /// 获取适用于国服目标 URI 的 Cookie，只向 Rust 传递 POESESSID 的值。
        private async Task CheckLoginAsync(bool userInitiated)
        {
            if (_checkingCookie || Browser.CoreWebView2 == null)
            {
                return;
            }

            _checkingCookie = true;
            try
            {
                var cookies = await Browser.CoreWebView2.CookieManager.GetCookiesAsync(LoginPolicy.CookieUri);
                var candidates = cookies.Select(cookie => new CookieCandidate(
                    cookie.Name,
                    cookie.Value,
                    cookie.Domain,
                    cookie.Path,
                    cookie.IsSession ? DateTime.MinValue : cookie.Expires));
                var selected = LoginPolicy.SelectPoeSession(candidates, DateTime.UtcNow);
                if (selected == null)
                {
                    StatusText.Text = "尚未取得 POESESSID，请继续完成登录。";
                    return;
                }

                var fingerprint = LoginPolicy.CookieFingerprint(selected.Value);
                if (!userInitiated && !LoginPolicy.ShouldAutoValidate(Browser.Source, fingerprint, _lastAttemptedCookieFingerprint))
                {
                    if (Browser.Source == null || !string.Equals(Browser.Source.IdnHost, "poe.game.qq.com", StringComparison.OrdinalIgnoreCase))
                    {
                        StatusText.Text = "请先完成 QQ 或微信登录，返回国服交易站后会自动验证。";
                    }
                    return;
                }
                _lastAttemptedCookieFingerprint = fingerprint;

                if (!File.Exists(_bridgePath))
                {
                    StatusText.Text = "未找到 QingPricePOE2.exe，无法验证登录结果。";
                    MessageBox.Show("请将登录助手与 QingPricePOE2.exe 放在同一目录，或使用 --bridge-exe 指定路径。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Error);
                    return;
                }

                StatusText.Text = "已取得登录状态，正在通过国服接口验证……";
                CheckButton.IsEnabled = false;
                var exitCode = await LoginBridge.SendPoeSessionAsync(_bridgePath, selected.Value);
                if (exitCode == 0)
                {
                    StatusText.Text = "登录验证成功，POESESSID 已由主程序加密保存。";
                    MessageBox.Show("登录验证成功。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Information);
                    Close();
                    return;
                }

                StatusText.Text = "登录状态未通过国服接口验证，请重新登录后再试。";
                if (userInitiated)
                {
                    MessageBox.Show("验证失败。登录助手未保存该 Cookie，也不会显示验证详情。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
            }
            catch (Exception)
            {
                StatusText.Text = "检查登录状态失败；未记录异常详情或任何 Cookie。";
            }
            finally
            {
                _checkingCookie = false;
                if (!_cleanupStarted)
                {
                    CheckButton.IsEnabled = true;
                }
            }
        }

        /// 退出前先清空 WebView2 数据，再重试删除独立的临时用户数据目录。
        protected override async void OnClosing(CancelEventArgs e)
        {
            if (_allowClose)
            {
                base.OnClosing(e);
                return;
            }

            e.Cancel = true;
            if (_cleanupStarted)
            {
                return;
            }

            _cleanupStarted = true;
            CheckButton.IsEnabled = false;
            _cookieTimer.Stop();
            StatusText.Text = "正在清理临时登录数据……";
            var cleaned = await CleanupAsync();
            if (!cleaned)
            {
                MessageBox.Show(
                    "WebView2 已关闭，但临时目录未能完全删除：\n" + _userDataFolder + "\n请关闭相关 WebView2 进程后手动删除。",
                    "临时数据清理未完成",
                    MessageBoxButton.OK,
                    MessageBoxImage.Warning);
            }
            _allowClose = true;
            Close();
        }

        private async Task<bool> CleanupAsync()
        {
            try
            {
                if (Browser.CoreWebView2 != null)
                {
                    Browser.CoreWebView2.CookieManager.DeleteAllCookies();
                    await Browser.CoreWebView2.Profile.ClearBrowsingDataAsync();
                }
            }
            catch (Exception)
            {
                // 即使 WebView2 清理 API 失败，也继续释放控件并删除隔离目录。
            }

            try
            {
                Browser.Dispose();
            }
            catch (Exception)
            {
                // 释放失败仍继续尝试目录清理。
            }

            if (!LoginPolicy.IsSafeUserDataFolder(_userDataFolder))
            {
                return false;
            }

            for (var attempt = 0; attempt < 8; attempt++)
            {
                try
                {
                    if (Directory.Exists(_userDataFolder))
                    {
                        Directory.Delete(_userDataFolder, true);
                    }
                    return true;
                }
                catch (IOException)
                {
                    await Task.Delay(250);
                }
                catch (UnauthorizedAccessException)
                {
                    await Task.Delay(250);
                }
            }
            return !Directory.Exists(_userDataFolder);
        }
    }
}
