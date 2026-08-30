# 项目背景：AIODashboard

## 一句话简介

Local-First、AI-Friendly、CLI-First 的个人 All-in-One Dashboard（macOS 优先）。
**Rust Core 是唯一业务核心；GUI 与 CLI 是平级客户端；AI 通过稳定 JSON CLI 操作系统；插件是第三类客户端。**

## 详细信息（不在此重复维护）

- 启动方式 / 功能范围 / Roadmap：`README.md`
- 架构红线 / 流程分级 / 六层 Checklist：`AGENTS.md`
- 架构落地说明：`docs/architecture.md`
- Plan 级任务：`docs/plans/`

## 当前状态（2026-08-30）

- MVP（Task / Project / Note / Inbox / Today / Search + GUI + CLI --json + Widget Snapshot v1）✅
- 插件系统 v1（变更包 001）✅
- UI 重设计 Bento 总控台 + 双主题（变更包 002）✅

## 非目标（当前阶段不做）

- dashboardd 守护进程（设计文档 §26 预留，插件宿主采用面板即宿主）
- 跨设备同步（Roadmap Phase 6，确有需求再做）
- 移动端适配（macOS 桌面优先）

## Backlog（下一步候选）

- StatCard 环比昨日接线（plan：docs/plans/2026-08-30-statcard-day-delta.md）
- project create/archive/delete 补刷 Snapshot（plan：docs/plans/2026-08-30-project-snapshot-refresh.md）
- Phase 3 SwiftUI Widget / Phase 2 AI Interface 完善（见 README Roadmap）
