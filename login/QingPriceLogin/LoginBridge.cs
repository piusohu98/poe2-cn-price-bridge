using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace QingPriceLogin
{
    internal enum BridgeResult
    {
        Accepted,
        Rejected,
        TimedOut,
        Cancelled,
        StartFailed
    }

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

        /// 将裸 POESESSID 写入后立即关闭 stdin，并在超时或取消时终止整个子进程树。
        internal static async Task<BridgeResult> SendPoeSessionAsync(
            string executablePath,
            string value,
            TimeSpan timeout,
            CancellationToken cancellationToken)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                return BridgeResult.Cancelled;
            }

            using (var process = new Process { StartInfo = CreateStartInfo(executablePath) })
            {
                try
                {
                    if (!process.Start())
                    {
                        return BridgeResult.StartFailed;
                    }
                    var standardOutput = process.StandardOutput.ReadToEndAsync();
                    var standardError = process.StandardError.ReadToEndAsync();
                    try
                    {
                        await process.StandardInput.WriteAsync(value).ConfigureAwait(false);
                    }
                    finally
                    {
                        process.StandardInput.Close();
                    }

                    var result = await WaitForExitOrStopAsync(process, timeout, cancellationToken).ConfigureAwait(false);
                    await WaitForDrainAsync(standardOutput, standardError).ConfigureAwait(false);
                    return result;
                }
                catch (Exception)
                {
                    TerminateProcessTree(process);
                    return cancellationToken.IsCancellationRequested
                        ? BridgeResult.Cancelled
                        : BridgeResult.StartFailed;
                }
            }
        }

        /// 等待进程有界退出；超时或取消时先调用 taskkill /T，再用 Process.Kill 兜底。
        internal static async Task<BridgeResult> WaitForExitOrStopAsync(
            Process process,
            TimeSpan timeout,
            CancellationToken cancellationToken)
        {
            var exitTask = Task.Run(() => process.WaitForExit());
            var timeoutTask = Task.Delay(timeout);
            var cancellationTask = Task.Delay(Timeout.Infinite, cancellationToken);
            var completed = await Task.WhenAny(exitTask, timeoutTask, cancellationTask).ConfigureAwait(false);
            if (completed == exitTask)
            {
                await exitTask.ConfigureAwait(false);
                return process.ExitCode == 0 ? BridgeResult.Accepted : BridgeResult.Rejected;
            }

            var cancelled = cancellationToken.IsCancellationRequested;
            TerminateProcessTree(process);
            await Task.WhenAny(exitTask, Task.Delay(TimeSpan.FromSeconds(3))).ConfigureAwait(false);
            return cancelled ? BridgeResult.Cancelled : BridgeResult.TimedOut;
        }

        /// 离线启动本地长运行进程，验证超时和取消都会有界终止。
        internal static bool RunOfflineSelfTest()
        {
            try
            {
                using (var timeoutProcess = StartLongRunningTestProcess())
                {
                    var result = WaitForExitOrStopAsync(
                        timeoutProcess,
                        TimeSpan.FromMilliseconds(150),
                        CancellationToken.None).GetAwaiter().GetResult();
                    if (result != BridgeResult.TimedOut)
                    {
                        return false;
                    }
                }

                using (var cancelProcess = StartLongRunningTestProcess())
                using (var cancellation = new CancellationTokenSource())
                {
                    cancellation.CancelAfter(150);
                    var result = WaitForExitOrStopAsync(
                        cancelProcess,
                        TimeSpan.FromSeconds(10),
                        cancellation.Token).GetAwaiter().GetResult();
                    return result == BridgeResult.Cancelled;
                }
            }
            catch (Exception)
            {
                return false;
            }
        }

        private static Process StartLongRunningTestProcess()
        {
            var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = "ping.exe",
                    Arguments = "-n 30 127.0.0.1",
                    UseShellExecute = false,
                    CreateNoWindow = true
                }
            };
            if (!process.Start())
            {
                process.Dispose();
                throw new InvalidOperationException("无法启动桥接离线测试进程");
            }
            return process;
        }

        private static async Task WaitForDrainAsync(Task<string> standardOutput, Task<string> standardError)
        {
            var drain = Task.WhenAll(standardOutput, standardError);
            await Task.WhenAny(drain, Task.Delay(TimeSpan.FromSeconds(2))).ConfigureAwait(false);
        }

        private static void TerminateProcessTree(Process process)
        {
            try
            {
                if (process == null || process.HasExited)
                {
                    return;
                }
                var processId = process.Id;
                using (var taskKill = Process.Start(new ProcessStartInfo
                {
                    FileName = "taskkill.exe",
                    Arguments = "/PID " + processId + " /T /F",
                    UseShellExecute = false,
                    CreateNoWindow = true
                }))
                {
                    if (taskKill != null && !taskKill.WaitForExit(3000))
                    {
                        taskKill.Kill();
                    }
                }
            }
            catch (Exception)
            {
                // taskkill 失败后继续使用当前进程句柄兜底。
            }

            try
            {
                if (process != null && !process.HasExited)
                {
                    process.Kill();
                }
            }
            catch (Exception)
            {
                // 进程可能已在检查与终止之间退出。
            }
        }
    }
}
