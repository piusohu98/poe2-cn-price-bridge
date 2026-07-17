using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading.Tasks;

namespace QingPriceLogin
{
    internal static class LoginBridge
    {
        /// 构造固定参数的桥接进程；Secret 只会通过标准输入传递。
        internal static ProcessStartInfo CreateStartInfo(string executablePath)
        {
            return new ProcessStartInfo
            {
                FileName = executablePath,
                Arguments = "--set-cookie-stdin",
                WorkingDirectory = Path.GetDirectoryName(Path.GetFullPath(executablePath)),
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = new UTF8Encoding(false),
                StandardErrorEncoding = new UTF8Encoding(false)
            };
        }

        /// 将裸 POESESSID 写入子进程标准输入，并丢弃子进程输出以避免进入助手日志。
        internal static async Task<int> SendPoeSessionAsync(string executablePath, string value)
        {
            using (var process = new Process { StartInfo = CreateStartInfo(executablePath) })
            {
                if (!process.Start())
                {
                    return -1;
                }

                var standardOutput = process.StandardOutput.ReadToEndAsync();
                var standardError = process.StandardError.ReadToEndAsync();
                await process.StandardInput.WriteAsync(value).ConfigureAwait(false);
                process.StandardInput.Close();
                await Task.Run(() => process.WaitForExit()).ConfigureAwait(false);
                await Task.WhenAll(standardOutput, standardError).ConfigureAwait(false);
                return process.ExitCode;
            }
        }
    }
}
