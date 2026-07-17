# Changelog

## Unreleased

### Added

- 新增独立的 `QingPriceLogin.exe` 登录 PoC，使用 .NET Framework 4.8 WPF 和 `Microsoft.Web.WebView2` 1.0.4078.44。
- 新增 Rust `--set-cookie-stdin` 桥接入口，仅接受裸 POESESSID，并在国服 trade2 验证通过后使用 DPAPI 保存。
- 登录助手增加腾讯国服交易站、QQ 登录和微信扫码登录所需的精确 HTTPS 导航白名单。
- CI 和发布验证增加登录助手构建、离线策略自检、stdin 合成 Secret 泄露测试及验证失败不保存测试。

### Changed

- Windows CI 固定到 `windows-2022`，发布包增加登录助手所需的最小 WebView2 托管程序集和 x64 Loader。
- 自动登录检测仅在返回国服交易站且 Cookie 发生变化后执行；自动失败不再弹窗打断扫码。

### Security

- WebView2 使用独立的 `%TEMP%\QingPriceLogin\<GUID>` 用户数据目录，退出前清除 Cookie 和浏览数据，并校验删除边界后清理目录。
- 登录助手不读取浏览器用户数据、不注入脚本、不读取账号、密码、QQ 号或二维码内容；Cookie 不进入命令行、日志、剪贴板或临时文件。
- 候选 Cookie 验证失败不会覆盖现有 DPAPI 存储；错误返回会按运行时已知 Secret 再次脱敏。

### Validation

- 2026-07-17 人工验证：国服交易站可在 WebView2 打开，微信扫码登录成功，POESESSID 可提取并通过国服 `poe.game.qq.com/api/trade2` 验证，助手退出后临时目录无残留。
- 当前仍为独立 PoC，尚未接入现有首次使用向导；手动粘贴 Cookie 方式保持不变。

## 0.2.0

### Added

- Rust 原生托盘常驻和游戏内覆盖面板。
- 国服 `poe.game.qq.com/api/trade2` 查价。
- `Ctrl+C` 自动查价，手动热键可在设置里选择或关闭。
- 首次使用向导、Cookie 设置、Cookie 验证。
- 同属性查价、数值下限筛选、逐条属性选择。
- 结果分页、官方市集链接复制和打开。
- 查询历史、诊断导出、一键自检。
- 手动检查 GitHub Release 更新，生成 `update-check.txt`，不自动下载或替换文件。
- 崩溃报告写入 `%APPDATA%\poe2_cn_price_bridge\crashes`。
- 发布包 zip 和 SHA256 校验文件。
- EXE 内嵌品牌图标和 Windows 版本资源。
- 托盘菜单增加 `运行自检` 和 `关于`。
- 发布包增加 `ResetData.bat` 和 `Uninstall.bat`。
- 发布包增加 `StartHere.bat` / `ControlCenter.bat` 客户控制中心。
- 自检报告覆盖完整客户交付入口，便于远程排查缺文件问题。
- 发布包增加中文客户入口，包括 `开始使用.bat`、`启动查价.bat`、`检查更新.bat` 等。
- 增加 `SUPPORT.md`、PR 模板、功能请求模板和 clippy CI 门禁。
- 移除早期浏览器扩展遗留代码，避免开源后误导用户。
- 发布包增加 `SupportBundle.bat`，一键生成可发给维护者的支持包。
- 重做 Cookie 设置窗口，增加剪贴板自动识别、复制获取步骤、验证现有 Cookie 和启动工具。
- 设置窗口增加恢复默认、配置目录入口和保存前校验。
- 增加 `rust-toolchain.toml` 和 Cargo 元数据，提升开源构建复现性。

### Changed

- 错误页改为可读分类，并提供向导、设置 Cookie、验证 Cookie、历史和诊断快捷按钮。
- 图标改为“清”字品牌图标。
- 客户窗口统一为更现代的 WPF 深色 UI，减少系统默认控件感。
- 设置窗口增加手动查价热键选择。
- GitHub Release 流程增加格式检查和测试门禁。
- 增加 `tools/verify_release.ps1` 自动验证发布包。

### Security

- Cookie 使用 Windows DPAPI 加密保存到当前 Windows 用户。
- 诊断、自检、历史和崩溃报告不输出明文 Cookie。
