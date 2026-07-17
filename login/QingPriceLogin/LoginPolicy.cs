using Microsoft.Web.WebView2.Core;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace QingPriceLogin
{
    internal sealed class CookieCandidate
    {
        internal CookieCandidate(string name, string value, string domain, string path, DateTime expires)
        {
            Name = name;
            Value = value;
            Domain = domain;
            Path = path;
            Expires = expires;
        }

        internal string Name { get; }
        internal string Value { get; }
        internal string Domain { get; }
        internal string Path { get; }
        internal DateTime Expires { get; }
    }

    internal static class LoginPolicy
    {
        internal const string TradeUri = "https://poe.game.qq.com/trade2";
        internal const string CookieUri = "https://poe.game.qq.com/";
        internal const string RuntimeDownloadUri = "https://developer.microsoft.com/microsoft-edge/webview2/consumer/";
        internal static readonly TimeSpan BridgeTimeout = TimeSpan.FromSeconds(50);
        internal static readonly TimeSpan BrowsingDataCleanupTimeout = TimeSpan.FromSeconds(4);
        internal static readonly TimeSpan StaleCleanupTimeout = TimeSpan.FromSeconds(10);

        private static readonly HashSet<string> AllowedHosts = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            // 国服官方交易站；2026-07-17 已人工验证打开、回跳和 Cookie 验证。
            "poe.game.qq.com",
            // 微信开放平台扫码页；2026-07-17 已人工观察并完成扫码登录。
            "open.weixin.qq.com"
        };

        /// 只允许已验证主机的 HTTPS 443 导航，其他协议、主机和端口默认拒绝。
        internal static bool IsAllowedNavigation(Uri uri)
        {
            return uri != null
                && string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase)
                && uri.IsDefaultPort
                && uri.Port == 443
                && AllowedHosts.Contains(uri.IdnHost);
        }

        /// 所有无法验证的服务器证书都必须取消，禁止进入可绕过的 TLS 错误页。
        internal static CoreWebView2ServerCertificateErrorAction CertificateErrorAction
        {
            get { return CoreWebView2ServerCertificateErrorAction.Cancel; }
        }

        /// 从已由 CookieManager 按目标 URI 筛选的列表中只选取有效 POESESSID。
        internal static CookieCandidate SelectPoeSession(IEnumerable<CookieCandidate> cookies, DateTime now)
        {
            return cookies
                .Where(cookie => string.Equals(cookie.Name, "POESESSID", StringComparison.Ordinal))
                .Where(cookie => !string.IsNullOrWhiteSpace(cookie.Value))
                .Where(cookie => cookie.Expires == DateTime.MinValue || cookie.Expires.ToUniversalTime() > now.ToUniversalTime())
                .OrderByDescending(cookie => DomainScore(cookie.Domain))
                .ThenByDescending(cookie => cookie.Path == null ? 0 : cookie.Path.Length)
                .FirstOrDefault();
        }

        /// 返回生产环境唯一允许存放 WebView2 临时用户数据的根目录。
        internal static string GetUserDataRoot()
        {
            return NormalizeDirectoryPath(Path.Combine(Path.GetTempPath(), "QingPriceLogin"));
        }

        /// 为本次登录创建位于固定根目录下的 GUID 路径，但不提前创建目录。
        internal static string CreateUserDataFolder()
        {
            return Path.Combine(GetUserDataRoot(), Guid.NewGuid().ToString("N"));
        }

        /// 确认候选路径是指定根目录的直接 GUID 子目录，拒绝路径穿越和根目录本身。
        internal static bool IsSafeDirectGuidDirectory(string root, string candidate)
        {
            if (string.IsNullOrWhiteSpace(root) || string.IsNullOrWhiteSpace(candidate))
            {
                return false;
            }

            string normalizedRoot;
            string normalizedCandidate;
            try
            {
                normalizedRoot = NormalizeDirectoryPath(root);
                normalizedCandidate = NormalizeDirectoryPath(candidate);
            }
            catch (Exception)
            {
                return false;
            }

            var parent = Directory.GetParent(normalizedCandidate);
            Guid folderId;
            return parent != null
                && string.Equals(NormalizeDirectoryPath(parent.FullName), normalizedRoot, StringComparison.OrdinalIgnoreCase)
                && Guid.TryParseExact(Path.GetFileName(normalizedCandidate), "N", out folderId);
        }

        /// 在固定次数内删除一个安全的直接 GUID 子目录；重解析点一律拒绝递归处理。
        internal static Task<bool> DeleteUserDataDirectoryWithRetryAsync(
            string candidate,
            int attempts,
            TimeSpan retryDelay,
            CancellationToken cancellationToken)
        {
            return DeleteDirectGuidDirectoryWithRetryAsync(
                GetUserDataRoot(),
                candidate,
                attempts,
                retryDelay,
                cancellationToken);
        }

        /// 测试辅助入口允许指定隔离根目录；生产代码不得调用此方法扩大清理边界。
        private static async Task<bool> DeleteDirectGuidDirectoryWithRetryAsync(
            string root,
            string candidate,
            int attempts,
            TimeSpan retryDelay,
            CancellationToken cancellationToken)
        {
            if (attempts <= 0 || !IsSafeRootDirectory(root) || !IsSafeDirectGuidDirectory(root, candidate))
            {
                return false;
            }

            for (var attempt = 0; attempt < attempts; attempt++)
            {
                cancellationToken.ThrowIfCancellationRequested();
                try
                {
                    if (!IsSafeRootDirectory(root))
                    {
                        return false;
                    }
                    if (!Directory.Exists(candidate))
                    {
                        return true;
                    }
                    var attributes = File.GetAttributes(candidate);
                    if ((attributes & FileAttributes.ReparsePoint) != 0 || ContainsReparsePoint(candidate))
                    {
                        return false;
                    }
                    Directory.Delete(candidate, true);
                    return true;
                }
                catch (IOException)
                {
                    // WebView2 子进程可能仍在释放文件，进入有界重试。
                }
                catch (UnauthorizedAccessException)
                {
                    // 文件句柄或 ACL 暂时阻止删除，进入有界重试。
                }

                if (attempt + 1 < attempts)
                {
                    await Task.Delay(retryDelay, cancellationToken).ConfigureAwait(false);
                }
            }
            return !Directory.Exists(candidate);
        }

        /// 只枚举根目录直接子目录并回收 GUID 遗留目录，返回未能删除的数量。
        internal static Task<int> CleanupStaleUserDataFoldersAsync(
            int attempts,
            TimeSpan retryDelay,
            CancellationToken cancellationToken)
        {
            return CleanupRootAsync(GetUserDataRoot(), attempts, retryDelay, cancellationToken);
        }

        /// 测试辅助入口只扫描调用方提供的隔离根目录的直接子目录。
        private static async Task<int> CleanupRootAsync(
            string root,
            int attempts,
            TimeSpan retryDelay,
            CancellationToken cancellationToken)
        {
            var normalizedRoot = NormalizeDirectoryPath(root);
            if (Directory.Exists(normalizedRoot) && !IsSafeRootDirectory(normalizedRoot))
            {
                throw new IOException("UDF 根目录是重解析点，拒绝清理");
            }
            Directory.CreateDirectory(normalizedRoot);
            if (!IsSafeRootDirectory(normalizedRoot))
            {
                throw new IOException("UDF 根目录边界验证失败");
            }
            var failures = 0;
            foreach (var directory in Directory.GetDirectories(normalizedRoot, "*", SearchOption.TopDirectoryOnly))
            {
                cancellationToken.ThrowIfCancellationRequested();
                if (!IsSafeDirectGuidDirectory(normalizedRoot, directory))
                {
                    continue;
                }
                if (!await DeleteDirectGuidDirectoryWithRetryAsync(
                    normalizedRoot,
                    directory,
                    attempts,
                    retryDelay,
                    cancellationToken).ConfigureAwait(false))
                {
                    failures++;
                }
            }
            return failures;
        }

        /// 用不可逆摘要标识已尝试的 Cookie，避免在内存中额外长期保存 Secret。
        internal static string CookieFingerprint(string value)
        {
            using (var sha256 = SHA256.Create())
            {
                return Convert.ToBase64String(sha256.ComputeHash(Encoding.ASCII.GetBytes(value)));
            }
        }

        /// 仅在返回国服站且 Cookie 已变化时执行自动验证。
        internal static bool ShouldAutoValidate(Uri currentUri, string fingerprint, string lastFingerprint)
        {
            return currentUri != null
                && IsAllowedNavigation(currentUri)
                && string.Equals(currentUri.IdnHost, "poe.game.qq.com", StringComparison.OrdinalIgnoreCase)
                && !string.IsNullOrEmpty(fingerprint)
                && !string.Equals(fingerprint, lastFingerprint, StringComparison.Ordinal);
        }

        private static int DomainScore(string domain)
        {
            var normalized = (domain ?? string.Empty).TrimStart('.');
            if (string.Equals(normalized, "poe.game.qq.com", StringComparison.OrdinalIgnoreCase))
            {
                return 3;
            }
            if (string.Equals(normalized, "game.qq.com", StringComparison.OrdinalIgnoreCase))
            {
                return 2;
            }
            return 1;
        }

/// 固定清理根目录必须真实存在且不能是重解析点，避免通过 junction 越过临时目录边界。
        private static bool IsSafeRootDirectory(string root)
        {
            try
            {
                return Directory.Exists(root)
                    && (File.GetAttributes(root) & FileAttributes.ReparsePoint) == 0;
            }
            catch (Exception)
            {
                return false;
            }
        }

        /// 递归删除前拒绝候选树中的任何重解析点，避免清理过程跟随外部路径。
        private static bool ContainsReparsePoint(string root)
        {
            var pending = new Stack<string>();
            pending.Push(root);
            while (pending.Count != 0)
            {
                var current = pending.Pop();
                foreach (var entry in Directory.EnumerateFileSystemEntries(current))
                {
                    var attributes = File.GetAttributes(entry);
                    if ((attributes & FileAttributes.ReparsePoint) != 0)
                    {
                        return true;
                    }
                    if ((attributes & FileAttributes.Directory) != 0)
                    {
                        pending.Push(entry);
                    }
                }
            }
            return false;
        }
        private static string NormalizeDirectoryPath(string path)
        {
            return Path.GetFullPath(path).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        }

        /// 无网络自检覆盖导航、证书、Cookie、桥接参数和 UDF 生命周期策略。
        internal static bool RunSelfTest()
        {
            try
            {
                RunNavigationAndCookieTests();
                RunUserDataFolderTests();
                Assert(LoginBridge.RunOfflineSelfTest(), "桥接超时和取消策略测试失败");
                return true;
            }
            catch (Exception)
            {
                return false;
            }
        }

        private static void RunNavigationAndCookieTests()
        {
            Assert(IsAllowedNavigation(new Uri(TradeUri)), "国服交易站应被允许");
            Assert(IsAllowedNavigation(new Uri("https://open.weixin.qq.com/connect/qrconnect")), "微信扫码主机应被允许");
            Assert(!IsAllowedNavigation(new Uri("http://poe.game.qq.com/trade2")), "HTTP 不应被允许");
            Assert(!IsAllowedNavigation(new Uri("https://poe.game.qq.com:4443/trade2")), "非 443 端口不应被允许");
            Assert(!IsAllowedNavigation(new Uri("https://poe.game.qq.com.evil.example/trade2")), "伪造主机不应被允许");
            Assert(!IsAllowedNavigation(new Uri("https://www.pathofexile.com/trade2")), "国际服不应被允许");
            Assert(!IsAllowedNavigation(new Uri("file:///C:/Windows/System32/drivers/etc/hosts")), "file 协议不应被允许");
            Assert(!IsAllowedNavigation(new Uri("data:text/html,blocked")), "data 协议不应被允许");
            Assert(!IsAllowedNavigation(new Uri("javascript:alert(1)")), "javascript 协议不应被允许");
            Assert(!IsAllowedNavigation(new Uri("qingprice://login")), "自定义协议不应被允许");
            Assert(CertificateErrorAction == CoreWebView2ServerCertificateErrorAction.Cancel, "证书错误必须取消");

            var secret = string.Concat("SYNTHETIC_", "LOGIN_", "7f3a91d2");
            var selected = SelectPoeSession(new[]
            {
                new CookieCandidate("other", "not-read-by-production", ".game.qq.com", "/", DateTime.MinValue),
                new CookieCandidate("POESESSID", "", ".game.qq.com", "/", DateTime.MinValue),
                new CookieCandidate("POESESSID", secret, ".game.qq.com", "/", DateTime.MinValue)
            }, DateTime.UtcNow);
            Assert(selected != null && selected.Value == secret, "应只选择有效 POESESSID");

            var startInfo = LoginBridge.CreateStartInfo("POE2PriceHelper.exe");
            Assert(startInfo.Arguments == "--set-cookie-stdin", "桥接命令只能使用 stdin 参数");
            Assert(!startInfo.Arguments.Contains(secret), "Secret 不得出现在命令行");

            var fingerprint = CookieFingerprint(secret);
            Assert(ShouldAutoValidate(new Uri(TradeUri), fingerprint, null), "国服页的新 Cookie 应自动验证");
            Assert(!ShouldAutoValidate(new Uri(TradeUri), fingerprint, fingerprint), "同一 Cookie 不应重复自动验证");
            Assert(!ShouldAutoValidate(new Uri("https://open.weixin.qq.com/connect/qrconnect"), fingerprint, null), "登录提供方页面不应自动验证");
        }

        private static void RunUserDataFolderTests()
        {
            var testRoot = Path.Combine(Path.GetTempPath(), "QingPriceLoginPolicyTests", Guid.NewGuid().ToString("N"));
            var outside = Path.Combine(Path.GetDirectoryName(testRoot), "outside-" + Guid.NewGuid().ToString("N"));
            try
            {
                Directory.CreateDirectory(testRoot);
                Directory.CreateDirectory(outside);
                var stale = Path.Combine(testRoot, Guid.NewGuid().ToString("N"));
                var nonGuid = Path.Combine(testRoot, "keep-me");
                Directory.CreateDirectory(stale);
                Directory.CreateDirectory(nonGuid);
                File.WriteAllText(Path.Combine(stale, "cache.bin"), "test");

                var failures = CleanupRootAsync(
                    testRoot,
                    3,
                    TimeSpan.FromMilliseconds(20),
                    CancellationToken.None).GetAwaiter().GetResult();
                Assert(failures == 0 && !Directory.Exists(stale), "崩溃遗留 GUID 目录应被清理");
                Assert(Directory.Exists(nonGuid), "非 GUID 目录不得清理");
                Assert(Directory.Exists(outside), "根目录外路径不得清理");

                var traversal = Path.Combine(testRoot, Guid.NewGuid().ToString("N"), "..", "..", Path.GetFileName(outside));
                Assert(!IsSafeDirectGuidDirectory(testRoot, traversal), "路径穿越不得通过边界检查");
                Assert(!IsSafeDirectGuidDirectory(testRoot, testRoot), "根目录本身不得作为删除目标");

                var occupied = Path.Combine(testRoot, Guid.NewGuid().ToString("N"));
                Directory.CreateDirectory(occupied);
                var occupiedFile = Path.Combine(occupied, "locked.bin");
                File.WriteAllText(occupiedFile, "locked");
                using (var stream = new FileStream(occupiedFile, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                {
                    var deletedWhileLocked = DeleteDirectGuidDirectoryWithRetryAsync(
                        testRoot,
                        occupied,
                        2,
                        TimeSpan.FromMilliseconds(20),
                        CancellationToken.None).GetAwaiter().GetResult();
                    Assert(!deletedWhileLocked && Directory.Exists(occupied), "文件占用时应在有界重试后失败");
                }
                var deletedAfterRelease = DeleteDirectGuidDirectoryWithRetryAsync(
                    testRoot,
                    occupied,
                    3,
                    TimeSpan.FromMilliseconds(20),
                    CancellationToken.None).GetAwaiter().GetResult();
                Assert(deletedAfterRelease && !Directory.Exists(occupied), "文件释放后应能清理目录");
            }
            finally
            {
                if (Directory.Exists(testRoot))
                {
                    Directory.Delete(testRoot, true);
                }
                if (Directory.Exists(outside))
                {
                    Directory.Delete(outside, true);
                }
            }
        }

        private static void Assert(bool condition, string message)
        {
            if (!condition)
            {
                throw new InvalidOperationException(message);
            }
        }
    }
}