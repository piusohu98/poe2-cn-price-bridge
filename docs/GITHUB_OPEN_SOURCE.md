# GitHub 开源发布说明

## 开源前检查

- 不提交 `target/`、`dist/`、`.vs/`、日志、临时文件、诊断文件。
- 不提交 `%APPDATA%\poe2_cn_price_bridge\config.json`。
- 不提交任何 `POESESSID`、完整 Cookie 请求头或个人账号截图。
- 保留 `Cargo.lock`，方便 Windows 客户和 GitHub Actions 复现构建。
- 保留 `rust-toolchain.toml`、`LICENSE`、`README.md`、`SUPPORT.md`、`.github/workflows/release.yml`、`.github/ISSUE_TEMPLATE/`、`docs/` 和 `assets/`。

## 首次推到 GitHub

```powershell
git init
git add .
git commit -m "Initial open source release"
git branch -M main
git remote add origin https://github.com/<你的账号>/<仓库名>.git
git push -u origin main
```

## 发布客户包

本地生成：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\package_release.ps1
```

输出：

```text
dist\QingPricePOE2-v*-windows-x64.zip
```

GitHub Release 自动生成：

```powershell
git tag v0.2.0
git push origin v0.2.0
```

## Issue 反馈模板建议

用户反馈问题时，让对方提供：

- Windows 版本。
- 工具版本。
- 游戏内复制的物品类型，例如通货、暗金、稀有装备。
- 错误页上的错误类型。
- `SelfCheck.bat` 生成的 `selfcheck.txt`。
- `Diagnostics.bat` 生成的 `diagnostics.txt`。
- 或直接提供 `SupportBundle.bat` 生成的 `support-bundle-*.zip`。

提醒用户不要发送明文 Cookie、POESESSID、手机号、QQ 号或支付信息。

## 维护者发布习惯

- 每次改功能后先跑 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` 和 `cargo build --release`。
- 发包前跑 `tools/package_release.ps1`、`tools/verify_release.ps1` 和 `tools/verify_release.ps1 -RunLaunchSmoke`，并从 zip 解压目录启动一次。
- 客户包只从 `dist/*.zip` 分发，不直接发 `target/release` 里的 exe。
- GitHub Release 描述里写清楚新增功能、修复问题和是否需要重新设置 Cookie。
