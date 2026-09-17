# Change Proposal 008: 插件安全与生命周期加固

> 状态：已完成（2026-09-08） · 基于 001-plugin-system/v1 与 007-plugin-platform/v1.1

## Why

当前插件系统已经具备 Trusted WebView 运行能力，但权限主要停留在前端桥，安装、生命周期和 CLI/GUI 同步仍存在越权、残留和状态漂移风险。

## What I Want

1. 在 Rust/Tauri 边界绑定插件上下文与 capability token。
2. 收紧旧插件语义：未声明能力即无权，示例插件迁移到显式权限。
3. 完成 Core/UI/KV/事件权限、网络代理、安装校验和生命周期清理。
4. 修复 CLI/GUI 状态同步、回滚元数据、SDK、脚手架、测试和文档状态。
5. 保留 Trusted WebView；Worker/WASM/独立进程另立变更包。

## What I Know

- v1 插件与宿主共享 React/WebView，权限声明不是进程级沙箱。
- Rust Core 仍是唯一业务实现；公开插件 API 受权限约束，但 Trusted 插件仍可影响共享 WebView。
- 现有 007 变更包仍有未完成的权限、生命周期、设置和文档任务。

## What I Don't Know

- Restricted Plugin 的最终运行时继续留待独立 Spec 验证。
- 本变更不引入公共市场、账号体系或跨设备同步。

## 交付

插件协议 v2、CLI schema 6、数据库 V8；安装/回滚保留 CLI-only。验证证据、残余限制及验收数据路径事件见 [tasks](tasks.md)。

> 后续演进（2026-09-08）：[桌面统一导入 Plan](../../../docs/plans/2026-09-08-plugin-import-entry.md) 在此基础上接入 ZIP / 文件夹预检与安装界面；回滚仍用 CLI。以上 CLI-only 描述保留为 008 当时交付范围，当前操作说明以 PLUGIN_API 为准。
