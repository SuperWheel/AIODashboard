# Specs 索引

Spec 级任务（新领域模块 / schema 变更 / AI 协议变更 / daemon）**必须**在本目录建档，规则对齐 `AGENTS.md` 流程分级。

## 规则

1. **命名**：`YYYY-MM-DD-<slug>.md`（日期取提案日）。新建文档从 [_TEMPLATE.md](_TEMPLATE.md) 整段复制。
2. **结构固定**：Why → ADDED / MODIFIED / REMOVED → Unknown（提案阶段）→ 决策记录 → 红线核对 → 实施记录 → 已知限制 → Tasks → 测试用例清单。
3. **状态四态**（写死在文档头部状态块）：`提案 → 已确认·进行中 → 已完成（填完成日期）/ 已废弃`。
4. **流程**：提案阶段先对齐确认 → 确认后 TDD（测试用例清单先行确认）→ 实施。
5. **核心纪律：切片提交时同步勾选 Tasks**——代码进了库而复选框没动 = 状态失真，下次会话就会误判进度。
6. 完成当天把状态改「已完成」，dev-log 条目回链本文档；实施偏离如实记「实施记录」。
7. 协议/schema 类 spec 的字段变更必须升 `meta.schema_version`（红线 8）。

## 索引

| 文档 | Spec ID | 状态 | 完成日期 | 摘要 |
|---|---|---|---|---|
| [2026-08-30-plugin-system.md](2026-08-30-plugin-system.md) | plugin-system/v1 | 已完成 | 2026-08-30 | 嵌入式 TS 插件 + 面板即宿主：manifest/宿主/权限/kv/cron/看门狗/脚手架，P1–P3 全落地 |
| [2026-08-30-ui-redesign-bento.md](2026-08-30-ui-redesign-bento.md) | ui-redesign/2026-08 | 已完成 | 2026-08-30 | Bento 总控台 + 语义 token 双主题；遗留 StatCard 环比已立 plan |
