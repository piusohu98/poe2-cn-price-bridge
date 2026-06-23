# 发布检查清单

## 构建前

- 更新 `Cargo.toml` 版本号。
- 更新 `CHANGELOG.md`。
- 确认 `README.md` 的使用说明和版本功能一致。
- 确认默认主联赛/备用联赛符合当前国服赛季，或 README 明确提示可用 `Settings.bat` 修改。
- 确认没有把个人 Cookie、日志、诊断文件加入仓库。
- 确认 `assets/app.ico` 和 `assets/app_256.png` 是本项目品牌图标。
- 确认 `build.rs` 会把 `assets/app.ico` 嵌入 exe。
- 确认 `.github/workflows/ci.yml`、issue 模板、`SECURITY.md`、`CONTRIBUTING.md` 存在。

## 本地验证

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
powershell -ExecutionPolicy Bypass -File .\tools\package_release.ps1
powershell -ExecutionPolicy Bypass -File .\tools\verify_release.ps1
powershell -ExecutionPolicy Bypass -File .\tools\verify_release.ps1 -RunLaunchSmoke
```

## 发布包验证

- 解压 `dist\QingPricePOE2-v*-windows-x64.zip` 到一个新目录。
- 运行 `StartHere.bat`，确认控制中心能打开。
- 双击 `QingPricePOE2.exe`，确认无黑色后台窗口。
- 在资源管理器里确认 `QingPricePOE2.exe` 使用清价品牌图标。
- 再双击一次，确认不会出现第二个进程。
- 托盘右键菜单可打开面板、设置 Cookie、导出诊断、退出。
- 托盘右键菜单可运行自检并打开 `selfcheck-*.txt`。
- 托盘右键菜单可打开关于面板并显示版本号。
- 托盘右键菜单可打开首次使用向导。
- 运行 `FirstRun.bat`，确认首次使用向导能打开。
- 检查发布包包含 `VERSION.txt`、`CHANGELOG.md` 和 `SUPPORT.md`。
- 检查发布包包含 `StartHere.bat`、`ControlCenter.bat`、`control_center.ps1` 和 `SupportBundle.bat`。
- 托盘右键菜单可打开设置窗口。
- 托盘右键菜单可打开查询历史窗口。
- 运行 `Settings.bat`，修改设置后确认 `config.json` 的 `settings` 节点更新。
- 运行 `Settings.bat`，确认可恢复默认值、打开配置目录，并且主联赛为空时会阻止保存。
- 在 `Settings.bat` 里把手动查价热键改成 `关闭` 和 `F9` 各测一次，确认运行中的工具能自动刷新。
- 运行 `SelfCheck.bat`，确认生成 `selfcheck.txt`，且不包含明文 Cookie。
- 检查 `selfcheck.txt` 的 package files 区域包含 `StartHere.bat`、`control_center.ps1`、`SupportBundle.bat`、`ResetData.bat`、`Uninstall.bat` 和 `SUPPORT.md`。
- 运行 `Diagnostics.bat`，确认生成 `diagnostics.txt`。
- 运行 `SupportBundle.bat`，确认生成 `support-bundle-*.zip`，且压缩包内含 `selfcheck.txt` 和 `diagnostics.txt`。
- 检查诊断文件只包含 `cookie_saved: true/false`，不包含明文 Cookie。
- 运行 `ResetData.bat`，在未输入确认词时确认会取消，不删除数据。
- 运行 `Uninstall.bat`，在未输入确认词时确认会取消，不删除数据。
- 运行 `History.bat`，确认历史窗口能打开。
- 运行 `SetCookie.bat`，确认 Cookie 设置窗口能打开，且有 `从剪贴板识别`、`保存并验证`、`验证现有 Cookie`。
- 运行 `ValidateCookie.bat`，确认 Cookie 可用时返回通过；过期时有明确失败提示。
- 检查异常退出时 `%APPDATA%\poe2_cn_price_bridge\crashes` 可写入崩溃报告。
- 游戏中悬停物品按 `Ctrl+C`，确认能看到结果面板或明确错误。
- 临时清空 Cookie 或使用无效 Cookie，确认错误页有分类、建议和 `设置Cookie` / `验证Cookie` 按钮。
- 使用过严同属性筛选查询一次，确认空结果文案提示可关闭筛选后重试。
- 确认 `dist\QingPricePOE2-v*-windows-x64.sha256.txt` 已生成。

## GitHub 发布

```powershell
git tag vX.Y.Z
git push origin vX.Y.Z
```

发布后确认 GitHub Release 包含 Windows x64 zip，并下载一次做烟测。
