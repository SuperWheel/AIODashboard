# Tasks 002: UI 重设计 · Bento 总控台

> 复选框约定：`[x]` 已完成（2026-08-30 切片 1–5 全部完成）。

## 实施切片

- [x] 切片 1 主题基建：styles.css token 化 + `@custom-variant` + 主题切换逻辑；全量替换写死色值（46a5e5d）
- [x] 切片 2 组件库：ui.tsx 重构 + ProgressRing/PageHeader/QuickCapture 新组件（0f619d6）
- [x] 切片 3 Today 页 Bento 化：网格布局 + 逾期折叠 + 插件槽位 size 协议（bdb90f2）
- [x] 切片 4 子页打磨：PageHeader 统一、空态、行交互、Sidebar 图标与毛玻璃（52bd7ed）
- [x] 切片 5 验收：check.sh 全绿 + 双主题真机走查（两轮）+ dev-log

## 测试与验证清单

- [x] bridge.test.ts 补 Today 卡片 size 透传/非法 size 拒绝用例（vitest 22 通过）
- [x] `bash scripts/check.sh` 六步全绿（fmt/clippy/Rust 测试/构建 + vitest + 前端生产构建）
- [x] 双主题人工走查：浅色/深色下各视图、弹窗、插件卡片观感（两轮真机走查完成，顺带修复插件热加载/卡片交互/输入法 Enter 等问题）

## 已知遗留

- [ ] StatCard「环比昨日」箭头未接线——`context today` 无昨日数据，属 core 层新增字段 → 已立 plan：`docs/plans/2026-08-30-statcard-day-delta.md`（待确认后实施）

## 实施记录

- 切片提交：46a5e5d / 0f619d6 / bdb90f2 / 52bd7ed；dev-log 2026-08-30「UI 重设计」条。
- 真机走查修复（后续切片）：插件启停热加载 + 卡片可交互 + 输入法 Enter 守卫（b972abb）；confirm 失效替换 DialogHost + 导航带参数（c9785c4）。
- 偏离：无实质偏离；`size` 协议为实施中与插件系统 001 协同新增（已四方同步）。
