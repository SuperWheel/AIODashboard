# Change Proposal 001: 插件系统（plugin-system/v1）

> 状态：**已完成（2026-08-30，P1–P3 全部落地）** · 模式：Spec · 关联：docs/architecture.md · docs/PLUGIN_API.md · 设计文档 §26（daemon 预留）

## Why（为什么做）

个人面板走向 All-in-One 统一面板：功能以统一模块契约接入，开发者（尤其 vibe coding / AI Agent）可自助开发插件。现有 CLI JSON 信封、activity log、权限 actor 机制与架构红线恰好构成插件系统的地基——插件是**第三类客户端**（与 GUI/CLI 平级）。

## What I Want（要什么）

1. **插件包格式**：目录 + manifest（声明式权限）+ 单文件入口 + 面向 AI 的 AGENTS.md。
2. **前端插件宿主**：loader / ModuleRegistry / API 桥（core 读写、kv、代理 fetch、事件、cron、贡献点注册）/ 权限执行 / 安全模式。
3. **Rust 与存储**：SCHEMA_V2（plugin_registry + plugin_kv）、Tauri 命令层（含网络白名单代理与审计）、cron 调度器、托盘常驻。
4. **CLI**：`plugin list / enable / disable`，沿用 JSON 信封与 exit code 语义。
5. **示例与生态**：echo + pomodoro 示例插件、PLUGIN_API.md、`plugin new/dev` 脚手架、加载看门狗、权限确认 UI。

## What I Know（已知）

- 核心四实体（Task/Project/Note/Inbox）是其他功能的地基，保持 host 服务不插件化，仅视图经 ModuleRegistry 模块化。
- 前端轮询（4s）+ WAL 并发是现有同步机制，领域事件可由 activity_log 投影派发。
- blob URL 动态 `import()` 要求 CSP 保持 `null`。
- cron 需要后台精度，面板窗口定时器会被节流 → 触发归 Rust。

## What I Don't Know（提案时待定 → 已拍板，见 design.md）

- 插件运行时形态：外部进程 / dylib / WASM / 嵌入脚本？→ **前端嵌入式 TS（Obsidian 同构）**，决策 1。
- v1 宿主归属：独立 daemon 还是面板内嵌？→ **面板即宿主 + 托盘常驻**，决策 2。
- 首个示范插件：番茄钟 / GitHub 统计 / 日历？→ **番茄钟**，决策 3。
- 节奏：先 P1 还是 P1+P2 连推？→ **P1+P2 切片连推，每片独立提交过门禁**，决策 4。
