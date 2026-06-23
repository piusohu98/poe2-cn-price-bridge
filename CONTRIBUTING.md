# Contributing

感谢参与清价 POE2。这个项目优先保证客户能稳定使用，所以改动请尽量小而可验证。

## 本地验证

```powershell
cargo fmt --check
cargo test
cargo build --release
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\package_release.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\verify_release.ps1
```

发布包验证请参考 `docs/RELEASE_CHECKLIST.md`。

## 提交建议

- 不提交 `target/`、`dist/`、日志、诊断、自检、崩溃报告。
- 不提交任何明文 `POESESSID` 或完整 Cookie 请求头。
- 改查价请求、Cookie、诊断、历史记录时，要确认不会泄露用户隐私。
- 改 UI 时先保证游戏内面板文字不重叠，按钮仍可点击。

## Issue 反馈

反馈问题时优先附上：

- Windows 版本。
- 工具版本。
- 错误页上的错误类型。
- `SelfCheck.bat` 生成的 `selfcheck.txt`。
- 必要时再附 `Diagnostics.bat` 生成的 `diagnostics.txt`。

不要发送明文 Cookie、POESESSID、手机号、QQ 号或支付信息。
