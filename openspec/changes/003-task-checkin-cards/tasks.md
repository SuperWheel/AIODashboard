# Tasks 003: 打卡式任务 + 日期主库 + 四种任务卡片

> 复选框约定：`[x]` 已完成并随切片提交勾选。

## TDD 测试用例清单（先行确认，再实现）

### core 单测

- [x] 五态判定：不适用 / 待完成 / 进行中 / 已完成 / 已错过 / 超额算完成 / 无目标不适用
- [x] 六态判定：future / not_applicable / zero / partial_low(<50%) / partial_high(≥50%) / complete
- [x] 账本幂等：同 operation_id 重放返回原结果不重复计数
- [x] 补偿规则：一条正向只补偿一次；减到 0 以下拒绝；不适用日打卡拒绝
- [x] 目标区间：改目标后历史日期按旧目标判定；重叠区间拒绝；0/负数目标拒绝
- [x] 活动区间：归档-恢复间隔日不适用、不进分母
- [x] 统计口径：完整完成率、当前/最长连续（不适用日打断）、buckets（周/月/年分桶）、今日完成率（超额封顶）
- [x] 主库天数：纪念日第 N 天 / 倒计时还剩·今天·已逾期（自然日，含当天边界）
- [x] 归属：同日唯一归属；移库今日起生效历史留原库；归档三选一事务回滚
- [x] 主库聚合：100%+50% 两任务=75%；无有效任务=不适用；移库前后各归各库
- [x] 迁移 V2→V3：todo/doing→active、done→archived；补默认目标与活动区间；旧数据计数保留

### integration.rs 链路

- [x] CLI 打卡链路：checkin → Core 读计数一致 → snapshot 刷新
- [x] 幂等链路：同 operation_id 重放 → 计数不变
- [x] 主库链路：library create → task move → library show 聚合正确 → archive 三选一
- [x] 协议 v2：信封 `meta.schema_version="2"`；complete/reopen 输出带 deprecated 提示

### 前端

- [ ] 双主题真机走查：四种卡片、热力六态（色阶/虚线/符号）、+/- 同侧圆角矩形
- [ ] 交互走查：打卡/减少/撤销、分组流转、卡片样式切换持久化、编辑器、主库全链路
- [x] 快捷键：Space / Enter / ⌘Z / ⌘1/2/3 / ⌥←/→

## 实施切片

- [x] 切片 0：变更包四件套建档（proposal/spec/design/tasks）
- [x] 切片 1：domain + storage SCHEMA_V3（Task 改造 + 5 新表 + 迁移）
- [x] 切片 2：core（checkin_service + 状态/统计纯函数 + library_service + snapshot 口径）
- [x] 切片 3：protocol v2 + CLI（task 新参数 + checkin/decrement/undo/overview + library 组）
- [x] 切片 4：Tauri commands + 插件桥同步
- [x] 切片 5：前端卡片体系（HeatmapCell / TaskCard×4 / TasksView 卡片墙 / 任务编辑器 / 任务详情 / Today 改造）
- [x] 切片 6：前端日期主库（侧边栏 / 网格 / 详情 / 归档三选一 / 已归档页）
- [x] 切片 7：integration.rs 链路 + check.sh 全绿 + dev-log/README/architecture/PLUGIN_API 同步

## 实施记录

- 切片提交：be589be（建档）→ 53e3c99（后端原子切片：domain+storage+core+CLI+Tauri+integration，因 Task 实体变更编译耦合故合并提交）→ c7708ce（前端卡片+主库视图）。
- 偏离 1：侧边栏未做「每个主库独立入口」，主库经统一 `libraries` 视图 + navParam 进详情（数据量个人级足够；后续需要再加）。
- 偏离 2：⌘Z 撤销限定 Today 页本次会话最近一次打卡（与 PlanningDays「本次运行内」语义一致）；撤销目标收窄为「正向打卡记录」（撤销的撤销不提供）。
- 偏离 3：周起始固定周一（PlanningDays 跟随系统 firstWeekday；本项目个人用，简化）。
- 偏离 4：undo 后发现语义问题修正——只撤销正向打卡记录（防止撤销撤销导致计数漂移）。
- 待办：双主题真机走查两项未勾（需用户独立验收，对齐批次验收惯例）。
- 门禁：check.sh 全绿（Rust 50 项测试 + vitest 22 项 + 前端构建）。
