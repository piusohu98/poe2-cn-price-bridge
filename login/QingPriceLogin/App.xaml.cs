using System;
using System.Linq;
using System.Threading;
using System.Windows;

namespace QingPriceLogin
{
    public partial class App : Application
    {
        private const string SingleInstanceMutexName = @"Global\QingPriceLogin.WebView2Lifecycle";
        private Mutex _singleInstanceMutex;
        private bool _ownsSingleInstanceMutex;

        /// 先执行离线自检或持有全局互斥并回收遗留 UDF，再创建登录窗口。
        protected override async void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);
            if (e.Args.Any(arg => string.Equals(arg, "--self-test", StringComparison.Ordinal)))
            {
                Shutdown(LoginPolicy.RunSelfTest() ? 0 : 1);
                return;
            }

            try
            {
                bool createdNew;
                _singleInstanceMutex = new Mutex(true, SingleInstanceMutexName, out createdNew);
                _ownsSingleInstanceMutex = createdNew;
                if (!createdNew)
                {
                    MessageBox.Show("清价登录助手已在运行。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Information);
                    Shutdown(3);
                    return;
                }
            }
            catch (Exception)
            {
                MessageBox.Show("无法建立登录助手单实例互斥，已安全停止。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Error);
                Shutdown(4);
                return;
            }

            try
            {
                int failures;
                using (var cleanupTimeout = new CancellationTokenSource(LoginPolicy.StaleCleanupTimeout))
                {
                    failures = await LoginPolicy.CleanupStaleUserDataFoldersAsync(
                        12,
                        TimeSpan.FromMilliseconds(250),
                        cleanupTimeout.Token);
                }
                if (failures != 0)
                {
                    MessageBox.Show(
                        "检测到仍被占用的历史 WebView2 临时目录。请关闭残留的 WebView2 进程后重试。",
                        "临时数据清理未完成",
                        MessageBoxButton.OK,
                        MessageBoxImage.Warning);
                    Shutdown(5);
                    return;
                }
            }
            catch (Exception)
            {
                MessageBox.Show("启动前清理历史 WebView2 临时目录失败，已安全停止。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Error);
                Shutdown(8);
                return;
            }

            var window = new MainWindow();

            MainWindow = window;
            window.Show();
        }

        /// 退出时释放全局互斥；不持有互斥的第二实例不得释放它。
        protected override void OnExit(ExitEventArgs e)
        {
            if (_ownsSingleInstanceMutex && _singleInstanceMutex != null)
            {
                try
                {
                    _singleInstanceMutex.ReleaseMutex();
                }
                catch (ApplicationException)
                {
                    // 互斥已不再由当前线程持有时只释放句柄。
                }
            }
            if (_singleInstanceMutex != null)
            {
                _singleInstanceMutex.Dispose();
            }
            base.OnExit(e);
        }
    }
}