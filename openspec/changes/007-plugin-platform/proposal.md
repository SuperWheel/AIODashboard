# Change Proposal 007: 插件平台演进（plugin-platform/v1.1）

> 状态：历史切片保留，未完成项由 008 接续（2026-09-08） · 基于 001-plugin-system/v1 增量演进

## Why

AIODashboard 已有可运行的插件 v1，但用户要通过 Vibe Coding 长期开发、导入和分发自定义功能，还需要稳定的公共 SDK、明确的权限能力、可重复的测试流程、安装回滚和更清晰的信任边界。

## What I Want

1. 提供 TypeScript SDK、类型定义和插件测试工具。
2. 为 manifest 增加 API/宿主兼容性、细粒度权限、存储配额和完整元数据。
3. 保持 Rust Core 为唯一业务核心，插件只能通过公开桥接 API 工作。
4. 支持本地开发、打包导入、校验、启停、重载和回滚。
5. 将当前同进程插件明确标记为受信任模式，并为未来 Worker/WASM/外部进程隔离预留协议。
6. 补充安全审计：actor 绑定、网络重定向/内网防护、CSP、生命周期清理。

## What I Know

- 001-plugin-system/v1 已实现插件目录、manifest、loader、ModuleRegistry、KV、事件、cron、CLI 和示例插件。
- 插件是 GUI/CLI 之外的第三类客户端；所有领域写入仍须进入 dashboard-core 并留下 activity log。
- Obsidian 的可借鉴模式是 manifest + lifecycle + extension points + local development + release policy。

## What I Don't Know

- Restricted Plugin 的最终运行时选择（Worker、WASM 或外部进程）需要单独验证，暂不纳入本变更的强制实现。
- 公共 Registry 的托管位置、签名服务和跨设备同步策略暂不确定。
