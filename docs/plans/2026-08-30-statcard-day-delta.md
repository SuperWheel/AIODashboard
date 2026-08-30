# Plan：StatCard 接线「环比昨日」箭头

> 级别：Plan · 状态：提案
> 提出日期：2026-08-30
> 关联：docs/specs/2026-08-30-ui-redesign-bento.md（M3）· dev-log 2026-08-30「UI 重设计」条

## 需求

- 现状：首页统计 2×2 迷你卡（StatCard）只有数值，环比昨日箭头「留了 delta 口径但未接线」——`context today` 没有昨日数据，属 core 层新增字段（dev-log 原话）。
- 预期：StatCard 显示数值 + 环比昨日箭头（↑ 上升 / ↓ 下降 / — 持平，含差值），数据口径与今日统计一致。

## 分步计划

- [ ] 1. 核对现有今日统计查询的数据链路（core → context command → api.ts → StatCard），确认 delta 口径需要哪些字段。
- [ ] 2. core：昨日统计查询——复用 `local_today_range` 的唯一入口逻辑取昨日区间（不在别处重写日期边界，红线 6）；今日+昨日合并暴露，补 core 单测。
- [ ] 3. Tauri command + `api.ts` 透出；涉及 `context today` 输出字段变化时按红线 8 评估是否属协议变更（新增字段且向后兼容，dev-log 记录即可）。
- [ ] 4. StatCard 渲染 delta：除零/零基数显示「—」，颜色用 info/danger 语义 token，`tabular-nums`。
- [ ] 5. 记 dev-log；双主题真机走查。

## 验证

- [ ] `bash scripts/check.sh` 全绿
- [ ] 人工验证：造昨日与今日数据各若干，核对箭头方向与差值；跨午夜边界场景目测正常。

## 结果记录

（完成后填写）
