# 清价 POE2 生产级客户版需求梳理

## 产品定位

清价 POE2 是给 POE2 国服玩家使用的本地查价工具。目标是让普通客户下载、解压、登录 Cookie、进游戏 `Ctrl+C` 后即可看到可用价格，并且在出问题时能导出诊断文件交给维护者排查。

## 当前核心流程

1. 客户下载 `QingPricePOE2-v*-windows-x64.zip`。
2. 解压后运行 `开始使用.bat` 打开控制中心，再按向导完成登录、保存 Cookie、验证 Cookie 和启动。
3. 如需单独重设 Cookie，运行 `设置Cookie.bat`，窗口可识别剪贴板里的 POESESSID，保存后自动验证。
4. 如需调整赛季、抓取数量、自动查价，运行 `常用设置.bat`。
5. 运行 `启动查价.bat` 或 `QingPricePOE2.exe`。
6. 工具常驻系统托盘，无黑色后台窗口。
7. 游戏里悬停物品并按 `Ctrl+C`。
8. 工具自动读取剪贴板、请求国服 trade2、显示价格面板。
9. 需要回看问题时运行 `查询历史.bat` 或托盘右键 `查询历史`。
10. 需要确认版本时运行 `检查更新.bat` 或托盘右键 `检查更新`。
11. 出问题时先运行 `生成支持包.bat` 生成支持包；也可单独运行 `运行自检.bat`、`Diagnostics.bat` 或托盘右键 `导出诊断`。

## 客户版必须满足

- 可安装：发布包内不要求 Rust、Cargo、Python 或浏览器扩展。
- 可引导：第一次使用有向导，不要求客户先读 README。
- 可集中操作：客户有一个控制中心入口，不需要在多个脚本之间猜。
- 可启动：双击 exe 或 `Start.bat` 不出现黑色后台窗口。
- 可退出：托盘右键有明确退出入口。
- 可排障：能导出不含明文 Cookie 的诊断文件。
- 可反馈：能一键生成支持包，包含自检、诊断、更新检查、版本和支持说明。
- 可自检：客户能一键检查发布包文件、Cookie 状态、官网网络和关键设置。
- 可更新判断：客户能手动检查 GitHub Release 最新版本，失败时明确说明网络原因和不需要授权。
- 可追踪崩溃：异常退出时留下不含明文 Cookie 的崩溃报告。
- 可回溯：能查看最近查询历史，定位是解析、认证、请求还是结果为空。
- 可恢复：Cookie 过期时提示用户重新设置，而不是只显示接口错误。
- 可行动：错误页必须给出下一步按钮，例如设置 Cookie、验证 Cookie、打开官网、历史和诊断。
- 可验证：客户能一键验证 Cookie 是否仍可用于国服 trade2。
- 可配置：客户能修改赛季、抓取数量、分页、自动查价和面板停留时间。
- 可重置：客户能一键清除本机配置、Cookie、历史、日志和诊断数据。
- 可卸载：客户能关闭后台、删除快捷方式并清理本机数据。
- 可维护：发布包由脚本生成，GitHub tag 可自动构建。
- 可解释：README 面向普通用户，不只面向开发者。

## 查询能力

- 自动查价：监听游戏 `Ctrl+C` 后的物品文本。
- Cookie 验证：使用国服 trade2 搜索接口确认当前 Cookie 是否可用。
- 手动重查：可在设置里选择 `关闭`、`F6`、`F7`、`F8`、`F9`、`F10`、`Ctrl+Alt+D`。
- 联赛策略：默认 `奥杜尔秘符`，失败后查 `永久`，并允许客户在设置窗口修改。
- 明细抓取：fetch 分批抓取，避免官方接口一次 ID 太多导致明细为空。
- 属性筛选：支持同属性、数值下限、逐条选择属性。
- 结果展示：最低价、挂单数、常见价格、分页表格、官方市集链接。
- 查询历史：保留最近查询摘要、错误信息、价格和官方链接，不保存 Cookie。
- 错误分类：认证、网络、限流、官方接口异常、属性库异常、查询条件不可用、明细读取失败都要分开提示。
- 更新检查：只读 GitHub Release latest，不自动下载、不自动替换文件。

## 生产级后续优先级

### P0

- 发布包一键可用。
- 托盘后台、退出、诊断导出。
- Cookie 设置窗口足够清楚。
- 查价失败时必须有可读错误和日志。
- 查价成功/失败都要写入查询历史，方便客户反馈。

### P1

- 更完整的首次使用向导：登录页、Cookie 获取步骤图、保存后自动验证。
- 查询历史增强：筛选、搜索、复制完整错误上下文。
- 发布包签名；当前至少提供 SHA256 校验。
- 更品牌化的原生设置窗口；当前 WPF 客户窗口已移除 WinForms 系统感。

### P2

- 自动更新检查和下载仍不做静默执行；当前仅提供手动检查更新。
- 更现代的设置 UI 框架，例如 egui/Tauri；保留轻量 overlay。
- 国际化文案分离。
- 自动上传匿名错误摘要需用户明确同意。

## 不做的事

- 不读写游戏内存。
- 不注入游戏进程。
- 不保存明文 Cookie。
- 不把用户 Cookie 打进日志、诊断文件或发布包。

## 验收门槛

- `cargo fmt --check` 通过。
- `cargo build --release` 通过。
- `tools/package_release.ps1` 生成 zip。
- 发布包 exe 可启动，重复启动仍只有一个进程。
- `--diagnostics` 生成诊断文件，且不包含明文 Cookie。
- `--check-update` 生成更新检查报告，网络失败时不泄露 Cookie 且说明不需要授权。
- Cookie 失效、网络失败、筛选过严时，面板有明确错误类型和可点击处理按钮。
- 发布包包含 `QingPricePOE2.exe`、`StartHere.bat`、`开始使用.bat`、`ControlCenter.bat`、`control_center.ps1`、`FirstRun.bat`、`首次向导.bat`、`Start.bat`、`启动查价.bat`、`SetCookie.bat`、`设置Cookie.bat`、`ValidateCookie.bat`、`Settings.bat`、`常用设置.bat`、`History.bat`、`查询历史.bat`、`SelfCheck.bat`、`运行自检.bat`、`CheckUpdate.bat`、`检查更新.bat`、`Diagnostics.bat`、`SupportBundle.bat`、`生成支持包.bat`、`SupportBundle.ps1`、`ResetData.bat`、`ResetData.ps1`、`Uninstall.bat`、`Uninstall.ps1`、`InstallShortcut.bat`、`ClearCookie.bat`、README、CHANGELOG、SUPPORT、VERSION、LICENSE、图标资源。
