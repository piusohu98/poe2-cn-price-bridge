using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

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

        private static readonly HashSet<string> AllowedHosts = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "poe.game.qq.com",
            "xui.ptlogin2.qq.com",
            "ssl.ptlogin2.qq.com",
            "ui.ptlogin2.qq.com",
            "ptlogin2.qq.com",
            "graph.qq.com",
            "aq.qq.com",
            "open.weixin.qq.com"
        };

        /// 只允许 HTTPS 的腾讯国服交易站及完成 QQ 登录所需的明确主机。
        internal static bool IsAllowedNavigation(Uri uri)
        {
            return uri != null
                && string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase)
                && AllowedHosts.Contains(uri.IdnHost);
        }

        /// 从适用于目标 URI 的 Cookie 列表中只选取有效的 POESESSID。
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

        /// 为每次登录创建独立且可删除的 WebView2 临时用户数据目录。
        internal static string CreateUserDataFolder()
        {
            return Path.Combine(Path.GetTempPath(), "QingPriceLogin", Guid.NewGuid().ToString("N"));
        }

        /// 删除前确认目录位于 TEMP\QingPriceLogin 下，且末级名称是本次生成的 GUID。
        internal static bool IsSafeUserDataFolder(string path)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                return false;
            }
            var fullPath = Path.GetFullPath(path).TrimEnd(Path.DirectorySeparatorChar);
            var root = Path.GetFullPath(Path.Combine(Path.GetTempPath(), "QingPriceLogin"))
                .TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
            Guid folderId;
            return fullPath.StartsWith(root, StringComparison.OrdinalIgnoreCase)
                && Guid.TryParseExact(Path.GetFileName(fullPath), "N", out folderId);
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
                && string.Equals(currentUri.Scheme, Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase)
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

        /// 无网络自检覆盖导航限制、Cookie 选择、桥接参数和临时目录隔离。
        internal static bool RunSelfTest()
        {
            try
            {
                Assert(IsAllowedNavigation(new Uri(TradeUri)), "国服交易站应被允许");
                Assert(IsAllowedNavigation(new Uri("https://xui.ptlogin2.qq.com/cgi-bin/xlogin")), "QQ 登录主机应被允许");
                Assert(IsAllowedNavigation(new Uri("https://open.weixin.qq.com/connect/qrconnect")), "微信扫码登录主机应被允许");
                Assert(!IsAllowedNavigation(new Uri("http://poe.game.qq.com/trade2")), "HTTP 不应被允许");
                Assert(!IsAllowedNavigation(new Uri("https://poe.game.qq.com.evil.example/trade2")), "伪造主机不应被允许");
                Assert(!IsAllowedNavigation(new Uri("https://www.pathofexile.com/trade2")), "国际服不应被允许");

                var secret = string.Concat("SYNTHETIC_", "LOGIN_", "7f3a91d2");
                var selected = SelectPoeSession(new[]
                {
                    new CookieCandidate("other", secret, ".game.qq.com", "/", DateTime.MinValue),
                    new CookieCandidate("POESESSID", "", ".game.qq.com", "/", DateTime.MinValue),
                    new CookieCandidate("POESESSID", secret, ".game.qq.com", "/", DateTime.MinValue)
                }, DateTime.UtcNow);
                Assert(selected != null && selected.Value == secret, "应只选择有效 POESESSID");

                var startInfo = LoginBridge.CreateStartInfo("QingPricePOE2.exe");
                Assert(startInfo.Arguments == "--set-cookie-stdin", "桥接命令只能使用 stdin 参数");
                Assert(!startInfo.Arguments.Contains(secret), "Secret 不得出现在命令行");

                var fingerprint = CookieFingerprint(secret);
                Assert(ShouldAutoValidate(new Uri(TradeUri), fingerprint, null), "国服页的新 Cookie 应自动验证");
                Assert(!ShouldAutoValidate(new Uri(TradeUri), fingerprint, fingerprint), "同一 Cookie 不应重复自动验证");
                Assert(!ShouldAutoValidate(new Uri("https://open.weixin.qq.com/connect/qrconnect"), fingerprint, null), "登录提供方页面不应自动验证");

                var firstFolder = CreateUserDataFolder();
                var secondFolder = CreateUserDataFolder();
                Assert(!string.Equals(firstFolder, secondFolder, StringComparison.OrdinalIgnoreCase), "临时目录必须唯一");
                Assert(firstFolder.StartsWith(Path.GetTempPath(), StringComparison.OrdinalIgnoreCase), "临时目录必须位于系统 TEMP");
                Assert(IsSafeUserDataFolder(firstFolder), "生成的临时目录必须通过删除边界检查");
                Assert(!IsSafeUserDataFolder(Path.GetTempPath()), "不得把 TEMP 根目录作为删除目标");
                return true;
            }
            catch (Exception)
            {
                return false;
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
