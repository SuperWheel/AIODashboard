# OpenSpec · 变更包规范

本目录管理 AIODashboard 的全部 Spec 级变更。**每个 Spec = 一个变更包目录**，内含固定四件套：

```text
openspec/
├── README.md                  # 本文件：规则 + 索引
├── project.md                 # 项目背景速览（细节指向 README/AGENTS，不重复维护）
└── changes/
    └── <NNN>-<slug>/          # 变更包，编号三位递增
        ├── proposal.md        # 提案：Why / What I Want / What I Know / What I Don't Know
        ├── spec.md            # 需求：ADDED-N / MODIFIED / REMOVED，每条带「要求 + 验收」；末尾「非本变更范围」
        ├── design.md          # 设计：关键决策（选择/理由/后果）+ 红线核对 + 数据模型/风险
        └── tasks.md           # 任务：分阶段勾选清单（含 TDD 测试清单）；允许补充文档（如 algorithm.md）
```

## 流程（对齐 AGENTS.md Spec 级）

1. **提案**：建 `NNN-slug/` 目录，写 proposal.md，状态 = 提案。先对齐确认，不动代码。
2. **确认**：proposal 拍板后，补全 spec.md（需求与验收）与 design.md（决策记录）；**TDD：tasks.md 里先列测试用例清单，确认后再实现**。
3. **实施**：按 tasks.md 切片推进，**每片提交前同步勾选**（代码进了库而复选框没动 = 状态失真）。
4. **完成**：proposal.md 头部标注「已完成（日期）」；实施偏离如实记入 tasks.md「实施记录」；记 dev-log 并回链变更包。
5. 跨变更包的规划用 tasks.md 互链（参照 001 的 Phase 索引写法），不复制内容。

## 约定

- 命名：`NNN-kebab-slug`（NNN 三位递增，不复用）；文件名固定 `proposal.md / spec.md / design.md / tasks.md`。
- spec.md 写法：`### ADDED-N：名称`，每条用 `- **要求**：…` / `- **验收**：…`；变更写 `## MODIFIED`，删除写 `## REMOVED`；明确不做写 `## 非本变更范围`。
- design.md 写法：`### 决策 N：标题`，每条给出 **选择 / 理由 / 后果（或否决理由）**；必须含「红线核对」表（七条架构红线逐条对照）。
- Plan 级任务不进本目录，走 `docs/plans/`（轻量模板）；Plan 影响面超界 → 升级为变更包。
- 状态标记：进行中的变更包在 proposal.md 头部标「进行中」；本项目无归档移动操作，历史变更包永久保留在 changes/ 下。

## 索引

| 变更包 | 标题 | 状态 | 完成日期 |
|---|---|---|---|
| [001-plugin-system](changes/001-plugin-system/proposal.md) | 插件系统（plugin-system/v1） | 已完成 | 2026-08-30 |
| [002-ui-redesign-bento](changes/002-ui-redesign-bento/proposal.md) | UI 重设计 · Bento 总控台 | 已完成 | 2026-08-30 |
| [003-task-checkin-cards](changes/003-task-checkin-cards/proposal.md) | 打卡式任务 + 日期主库 + 四种任务卡片 | 进行中 | — |
