# 支持与反馈

## 客户反馈问题时

请优先让客户提供这些信息：

- 工具版本，查看发布包里的 `VERSION.txt`。
- Windows 版本。
- 问题类型：启动、Cookie、查价结果、游戏内面板、热键、发布包。
- 复现步骤：做了什么、看到什么、期望什么。
- 优先提供 `SupportBundle.bat` 生成的 `support-bundle-*.zip`。
- 如不方便打包，再分别提供 `SelfCheck.bat` 生成的 `selfcheck.txt` 和 `Diagnostics.bat` 生成的 `diagnostics.txt`。

不要让客户发送明文 `POESESSID`、完整 Cookie 请求头、手机号、QQ 号、支付信息或账号截图。

## 常见处理顺序

1. 先运行 `StartHere.bat`，确认控制中心能打开。
2. 运行 `SetCookie.bat` 重新保存 Cookie。
3. 运行 `ValidateCookie.bat` 验证 Cookie 是否仍可用。
4. 运行 `SelfCheck.bat` 检查发布包完整性、网络和配置。
5. 运行 `SupportBundle.bat` 生成支持包，再根据错误类型判断。

## 常见问题

- `Cookie 未设置`：运行 `SetCookie.bat`。
- `Cookie 已过期`：重新登录国服市集，再保存新的 `POESESSID`。
- `请求过于频繁`：暂停一会儿再查，减少连续重试。
- `没有在线挂单`：降低筛选强度，关闭同属性或数值筛选后重试。
- `查询条件不可用`：复制文本里的部分中文属性可能暂时无法匹配官方属性库。

## 开源维护者

合并 PR 前至少运行：

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\package_release.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\verify_release.ps1
```
