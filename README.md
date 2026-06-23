# 清价 POE2 国服查价

一个 Rust 编写的 POE2 国服游戏内查价工具。悬停物品后按 `Ctrl+C`，工具会自动读取游戏复制的物品文本，请求国服官方 `poe.game.qq.com/api/trade2`，并在游戏上方显示价格面板。

![icon](assets/app_256.png)

## 特性

- 无黑色后台窗口，后台常驻到系统托盘。
- 托盘右键菜单支持打开面板、立即查价、设置 Cookie、运行自检、导出诊断、关于和退出。
- 游戏内 `Ctrl+C` 自动查价，手动重查热键可在设置里选择或关闭。
- 支持国服 `POESESSID`，Cookie 用 Windows DPAPI 加密保存到当前用户。
- 支持同属性查价、按数值下限筛选、逐条选择属性。
- 一次抓取更多挂单并分页显示。
- 查价失败时显示错误类型、原因、建议操作和快捷按钮。
- 自带发布包脚本，可以打成别人解压就能用的 zip。
- 自带本地日志和诊断导出，方便客户反馈问题。
- 开源协作文件完整：CI、Issue 模板、CHANGELOG、CONTRIBUTING、SECURITY。

## 下载后怎么用

1. 解压发布包 `QingPricePOE2-v*-windows-x64.zip`。
2. 先运行 `开始使用.bat`，在控制中心里点 `首次使用向导`。
3. 按向导完成登录、保存 Cookie、验证 Cookie 和启动工具。
4. 以后可以继续用 `开始使用.bat`，也可以直接运行 `启动查价.bat` 或双击 `QingPricePOE2.exe`。
5. 进游戏，鼠标悬停物品，按 `Ctrl+C`。
6. 查看右上角价格面板；关闭面板后工具仍在托盘后台运行。

如果直接运行主程序且还没有保存 Cookie，工具会自动打开首次使用向导。

如果只想单独设置 Cookie，可以运行 `设置Cookie.bat`。窗口会自动识别剪贴板里的 POESESSID，保存后会自动验证 Cookie 是否可用。

如果不知道在哪里复制 Cookie，可以在浏览器打开 `https://poe.game.qq.com/trade2` 后按 `F12`，到 `Application/应用 -> Cookies -> https://poe.game.qq.com`，复制 `POESESSID`。也可以复制完整 `Cookie: ...` 请求头。

需要手动确认 Cookie 是否仍可用时，运行 `ValidateCookie.bat`，或托盘右键选择 `验证 Cookie`。

需要查看最近查询时，运行 `查询历史.bat`，或托盘右键选择 `查询历史`。历史记录不会保存物品全文或 Cookie，只保存查询摘要、结果、错误信息和市集链接。

发布包内的 `VERSION.txt` 记录版本、构建时间和常用入口，`CHANGELOG.md` 记录版本变化。

## 控制中心

运行 `开始使用.bat`、`StartHere.bat` 或 `ControlCenter.bat` 会打开控制中心，集中提供启动工具、首次使用向导、设置 Cookie、常用设置、查询历史、运行自检、导出诊断、重置和卸载。发布包保留英文入口用于脚本兼容，客户日常优先点中文入口。

## 重置和卸载

- `ResetData.bat` 会关闭后台，并清除本机配置、Cookie、历史、日志、自检、诊断和崩溃报告。程序文件夹不会被删除。
- `Uninstall.bat` 会关闭后台、删除桌面快捷方式并清除本机数据。完成后可以手动删除解压出来的程序文件夹。
- 两个脚本都会要求输入确认词，避免误操作。

## 客户排障

- 客户先双击 `SelfCheck.bat`，会生成 `selfcheck.txt`，用于检查文件完整度、配置、Cookie 验证和官网网络访问。
- 托盘右键选择 `导出诊断`，会在本机生成诊断文件。
- 发布包里也可以双击 `Diagnostics.bat`，生成 `diagnostics.txt`。
- 需要一次性反馈给维护者时，运行 `SupportBundle.bat` 或控制中心的 `生成支持包`，会生成 `support-bundle-*.zip`。
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

- `最低价`、`挂单数`、`常见价格` 显示核心价格信息。
- `同属性` 按复制文本里识别到的装备属性重新查价。
- `数值` 在同属性查询时带上当前属性数值下限。
- `属性1`、`属性2` 等小按钮可以逐条选择哪些属性参与查询。
- `上一页` / `下一页` 翻看更多挂单。
- `打开市集` 打开本次官方市集查询页。
- `复制链接` 复制本次查询链接。
- `固定` 让面板不自动消失。

## 托盘菜单

- `运行自检` 会生成并打开 `selfcheck-*.txt`。
- `导出诊断` 会生成并打开 `diagnostics-*.txt`。
- `生成支持包` 会打包自检、诊断、版本和支持说明，方便发给维护者。
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
dist\QingPricePOE2-v*-windows-x64.zip
dist\QingPricePOE2-v*-windows-x64.sha256.txt
```

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
