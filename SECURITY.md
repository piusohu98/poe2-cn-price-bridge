# Security Policy

## Supported Versions

当前只维护最新发布版本。旧版本如果出现 Cookie、安全或接口兼容问题，请升级到最新 Release。

## Sensitive Data

清价 POE2 会保存国服 `POESESSID`，用于访问官方 trade2 接口。Cookie 使用 Windows DPAPI 加密，仅当前 Windows 用户可解密。

以下文件不应包含明文 Cookie：

- `diagnostics.txt`
- `selfcheck.txt`
- 查询历史
- 日志
- 崩溃报告

## Reporting a Vulnerability

如果发现 Cookie 泄露、日志泄露、诊断泄露或不该访问的本地文件行为，请不要在公开 issue 中贴明文 Cookie。可以只描述复现步骤和影响范围，并附上已打码的自检/诊断文件。

## Scope

本工具只调用国服官方 trade2 接口，不修改游戏进程，不注入游戏，不读写游戏内存。
