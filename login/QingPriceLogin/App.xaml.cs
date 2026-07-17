using System;
using System.Linq;
using System.Windows;

namespace QingPriceLogin
{
    public partial class App : Application
    {
        /// 处理自检和可选的本地桥接程序路径；参数中永远不包含 Cookie。
        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);
            if (e.Args.Any(arg => string.Equals(arg, "--self-test", StringComparison.Ordinal)))
            {
                Shutdown(LoginPolicy.RunSelfTest() ? 0 : 1);
                return;
            }

            string bridgePath = null;
            for (var index = 0; index < e.Args.Length; index++)
            {
                if (!string.Equals(e.Args[index], "--bridge-exe", StringComparison.Ordinal))
                {
                    continue;
                }
                if (index + 1 >= e.Args.Length)
                {
                    MessageBox.Show("--bridge-exe 缺少路径。", "清价登录助手", MessageBoxButton.OK, MessageBoxImage.Error);
                    Shutdown(2);
                    return;
                }
                bridgePath = e.Args[index + 1];
                index++;
            }

            var window = new MainWindow(bridgePath);
            MainWindow = window;
            window.Show();
        }
    }
}
