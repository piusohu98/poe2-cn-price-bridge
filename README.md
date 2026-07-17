# 流放2查价助手 国服查价

一个 Rust 编写的 POE2 国服游戏内查价工具。悬停物品后按 `Ctrl+C`，工具会自动读取游戏复制的物品文本，请求国服官方 `poe.game.qq.com/api/trade2`，并在游戏上方显示价格面板。

![icon](assets/app_256.png)

## 特性

- 无黑色后台窗口，后台常驻到系统托盘。
- 托盘右键菜单支持打开面板、立即查价、设置 Cookie、运行自检、导出诊断、检查更新、关于和退出。
- 游戏内 `Ctrl+C` 自动查价，手动重查热键可在设置里选择或关闭。
- 支持国服 `POESESSID`，Cookie 用 Windows DPAPI 加密保存到当前用户。
- 支持同属性查价、按数值下限筛选、逐条选择属性。
- 一次抓取更多挂单并分页显示。
- 查价失败时显示错误类型、原因、建议操作和快捷按钮。
- 自带发布包脚本，可以打成别人解压就能用的 zip。
- 自带本地日志、诊断导出、支持包和手动检查更新，方便客户反馈问题。
- 开源协作文件完整：CI、Issue 模板、CHANGELOG、CONTRIBUTING、SECURITY。

## 下载后怎么用

1. 解压发布包 `POE2PriceHelper-v*-windows-x64.zip`。
2. 先运行 `开始使用.bat`，在控制中心里点 `首次使用向导`。
3. 按向导完成登录、保存 Cookie、验证 Cookie 和启动工具。
4. 以后可以继续用 `开始使用.bat`，也可以直接运行 `启动查价.bat` 或双击 `POE2PriceHelper.exe`。
5. 进游戏，鼠标悬停物品，按 `Ctrl+C`。
6. 查看右上角价格面板；关闭面板后工具仍在托盘后台运行。

如果直接运行主程序且还没有保存 Cookie，工具会自动打开首次使用向导。

如果只想单独设置 Cookie，可以运行 `设置Cookie.bat`。窗口会自动识别剪贴板里的 POESESSID，保存后会自动验证 Cookie 是否可用。

以上手动复制仅是“高级方式”备用流程；普通用户请使用首次向导中的微信扫码登录，无需打开 F12。若确需手动获取，可在浏览器打开 `https://poe.game.qq.com/trade2` 后按 `F12`，到 `Application/应用 -> Cookies -> https://poe.game.qq.com`，复制 `POESESSID`，也可以复制完整 `Cookie: ...` 请求头。

需要手动确认 Cookie 是否仍可用时，运行 `ValidateCookie.bat`，或托盘右键选择 `验证 Cookie`。

需要查看最近查询时，运行 `查询历史.bat`，或托盘右键选择 `查询历史`。历史记录不会保存物品全文或 Cookie，只保存查询摘要、结果、错误信息和市集链接。

需要确认是不是最新版时，运行 `检查更新.bat`，或托盘右键选择 `检查更新`。它只读取 GitHub Release 信息，不会自动下载或替换本机文件；GitHub API 偶尔网络失败时不需要授权，稍后重试即可。

发布包内的 `VERSION.txt` 记录版本、构建时间和常用入口，`CHANGELOG.md` 记录版本变化。

## 控制中心

运行 `开始使用.bat`、`StartHere.bat` 或 `ControlCenter.bat` 会打开控制中心，集中提供启动工具、首次使用向导、设置 Cookie、常用设置、查询历史、运行自检、导出诊断、检查更新、重置和卸载。发布包保留英文入口用于脚本兼容，客户日常优先点中文入口。

## 重置和卸载

- `ResetData.bat` 会关闭后台，并清除本机配置、Cookie、历史、日志、自检、诊断和崩溃报告。程序文件夹不会被删除。
- `Uninstall.bat` 会关闭后台、删除桌面快捷方式并清除本机数据。完成后可以手动删除解压出来的程序文件夹。
- 两个脚本都会要求输入确认词，避免误操作。

## 客户排障

- 客户先双击 `SelfCheck.bat`，会生成 `selfcheck.txt`，用于检查文件完整度、配置、Cookie 验证和官网网络访问。
- 托盘右键选择 `导出诊断`，会在本机生成诊断文件。
- 发布包里也可以双击 `Diagnostics.bat`，生成 `diagnostics.txt`。
- 双击 `检查更新.bat` 会生成 `update-check.txt`，用于确认本机版本和 GitHub Release 最新版本。
- 需要一次性反馈给维护者时，运行 `SupportBundle.bat` 或控制中心的 `生成支持包`，会生成 `support-bundle-*.zip`，里面包含自检、诊断、更新检查、版本和支持说明。
- 诊断文件包含版本、配置路径、是否已保存 Cookie、最近日志，不包含明文 Cookie。
- 诊断文件也包含最近查询历史摘要，方便排查“为什么查不到”。
- 查价失败弹窗会区分 `Cookie 未设置`、`Cookie 已过期`、`网络连接失败`、`请求过于频繁`、`查询条件不可用`、`挂单明细读取失败` 等类型。
- 错误弹窗底部提供 `向导`、`设置Cookie`、`验证Cookie`、`打开官网`、`历史`、`诊断` 快捷操作。
- 客户反馈问题时可参考 `SUPPORT.md`，不要发送明文 Cookie 或个人账号信息。

## 设置

运行 `Settings.bat`，或托盘右键选择 `设置`，可以修改：

- 主联赛、备用联赛。
- 最多抓取挂单数量。
- 每批 fetch 数量。
- 每页显示条数。
- 面板停留秒数。
- 是否启用 `Ctrl+C` 自动查价。
- 手动查价热键：`关闭`、`F6`、`F7`、`F8`、`F9`、`F10`、`Ctrl+Alt+D`。
- 设置窗口可一键恢复推荐默认值，也可以打开配置目录排查本机配置。

## 面板功能

面板采用纵向布局，从顶部到底部依次显示：

- **标题栏**：显示物品稀有度颜色、名称、基础类型，右侧显示价值评级徽章。
- **物品详情区**：展示品质、需求等级、物品等级、武器伤害（含物理/元素 DPS 计算）、护甲/闪避/能量护盾、插槽等信息。
- **词缀筛选区**：列出识别的物品词缀，点击词缀可选择/取消筛选，支持单独切换每条词缀。
- **搜索控制栏**：显示"同属性"和"数值"筛选开关状态、已选属性数量、匹配数量和当前页码。
- **价格指标卡**：三张卡片分别显示最低价、挂单数和常见价格分布。
- **挂单表格**：以表格形式展示挂单列表，支持按等级、价格、上架时间排序，每行带私聊按钮。
- **快捷键**：`←→` 翻页、`M` 切换同属性筛选、`V` 切换数值筛选、`P` 固定面板、`C` 复制市集链接、`O` 打开市集、`Esc` 关闭面板、`1-4` 切换词缀选择。
- **鼠标滚轮**：在面板上滚动可翻页。
- **鼠标悬停**：鼠标悬停在面板上暂停自动隐藏倒计时，按钮悬停有高亮反馈。
- **固定**：点击"固定"按钮让面板不自动消失。

## 托盘菜单

- `运行自检` 会生成并打开 `selfcheck-*.txt`。
- `导出诊断` 会生成并打开 `diagnostics-*.txt`。
- `检查更新` 会生成并打开 `update-check-*.txt`。
- `生成支持包` 会打包自检、诊断、更新检查、版本和支持说明，方便发给维护者。
- `关于` 会显示版本、热键、隐私说明和排障入口。
- `退出` 会真正关闭后台常驻进程。

## 查询历史

查询历史保存位置：

```text
%APPDATA%\poe2_cn_price_bridge\history.jsonl
```

历史窗口支持刷新、打开市集链接、复制链接和导出诊断。

## 从源码构建

需要 Windows、Rust stable、MSVC Build Tools。仓库包含 `rust-toolchain.toml`，会自动启用 `rustfmt` 和 `clippy` 组件。

```powershell
cargo build --release
```

如果本机 MSVC 环境没有自动加载，可以先运行：

```bat
setup_msvc_env.bat
```

生成图标和发布包：

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\package_release.ps1
powershell -ExecutionPolicy Bypass -File .\tools\verify_release.ps1
```

输出文件在：

```text
dist\POE2PriceHelper-v*-windows-x64.zip
dist\POE2PriceHelper-v*-windows-x64.sha256.txt
```

## 第三方组件

独立登录 PoC `POE2PriceLogin.exe` 使用固定版本 `Microsoft.Web.WebView2 1.0.4078.44`。NuGet 依赖由 `login/QingPriceLogin/packages.lock.json` 锁定，构建时通过 `tools/build_login.ps1` 从 `Cargo.toml` 注入统一版本号。

发布包在 `licenses` 目录中包含对应版本的 Microsoft WebView2 `LICENSE.txt` 和 `NOTICE.txt`。程序不捆绑 WebView2 Runtime；缺失时只引导到微软官方下载页面。

## GitHub 发布

仓库包含 `.github/workflows/release.yml`。推送 tag 后会自动构建 Windows x64 zip 并挂到 GitHub Release：

```powershell
git tag v0.2.0
git push origin v0.2.0
```

也可以在 GitHub Actions 页面手动运行 `build-release`。

开源前建议先看 `docs/GITHUB_OPEN_SOURCE.md`，确认没有提交 `dist`、`target`、日志、诊断文件或个人 Cookie。

Pull Request 会通过 `.github/workflows/ci.yml` 检查 `cargo fmt --check` 和 `cargo build --release`。

## 配置和隐私

Cookie 保存位置：

```text
%APPDATA%\poe2_cn_price_bridge\config.json
```

Cookie 使用 Windows DPAPI 加密，只能由当前 Windows 用户解密。开源仓库和发布包不会包含你的 Cookie。

如果程序异常退出，会在下面目录留下崩溃报告，报告不包含明文 Cookie：

```text
%APPDATA%\poe2_cn_price_bridge\crashes
```

日志位置：

```text
%APPDATA%\poe2_cn_price_bridge\logs\app.log
```

## 联赛

默认先查 `奥杜尔秘符`，失败再查 `永久`。如果国服赛季名变化，在 `Settings.bat` 里修改主联赛和备用联赛。

## 说明

本项目只调用国服官方 trade2 接口，不修改游戏进程，也不读写游戏内存。使用前请自行确认符合游戏和平台规则。

## WebView2 Runtime

登录助手不捆绑 WebView2 Runtime。若首次向导提示 Runtime 缺失，请仅使用微软官方安装入口：
https://developer.microsoft.com/microsoft-edge/webview2/
