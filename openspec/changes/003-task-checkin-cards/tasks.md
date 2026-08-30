# Tasks 003: 打卡式任务 + 日期主库 + 四种任务卡片

> 复选框约定：`[x]` 已完成并随切片提交勾选。

## TDD 测试用例清单（先行确认，再实现）

### core 单测

- [ ] 五态判定：不适用 / 待完成 / 进行中 / 已完成 / 已错过 / 超额算完成 / 无目标不适用
- [ ] 六态判定：future / not_applicable / zero / partial_low(<50%) / partial_high(≥50%) / complete
- [ ] 账本幂等：同 operation_id 重放返回原结果不重复计数
- [ ] 补偿规则：一条正向只补偿一次；减到 0 以下拒绝；不适用日打卡拒绝
- [ ] 目标区间：改目标后历史日期按旧目标判定；重叠区间拒绝；0/负数目标拒绝
- [ ] 活动区间：归档-恢复间隔日不适用、不进分母
- [ ] 统计口径：完整完成率、当前/最长连续（不适用日打断）、buckets（周/月/年分桶）、今日完成率（超额封顶）
- [ ] 主库天数：纪念日第 N 天 / 倒计时还剩·今天·已逾期（自然日，含当天边界）
- [ ] 归属：同日唯一归属；移库今日起生效历史留原库；归档三选一事务回滚
- [ ] 主库聚合：100%+50% 两任务=75%；无有效任务=不适用；移库前后各归各库
- [ ] 迁移 V2→V3：todo/doing→active、done→archived；补默认目标与活动区间；旧数据计数保留

### integration.rs 链路

- [ ] CLI 打卡链路：checkin → Core 读计数一致 → snapshot 刷新
- [ ] 幂等链路：同 operation_id 重放 → 计数不变
- [ ] 主库链路：library create → task move → library show 聚合正确 → archive 三选一
- [ ] 协议 v2：信封 `meta.schema_version="2"`；complete/reopen 输出带 deprecated 提示

### 前端

- [ ] 双主题真机走查：四种卡片、热力六态（色阶/虚线/符号）、+/- 同侧圆角矩形
- [ ] 交互走查：打卡/减少/撤销、分组流转、卡片样式切换持久化、编辑器、主库全链路
- [ ] 快捷键：Space / Enter / ⌘Z / ⌘1/2/3 / ⌥←/→

## 实施切片

- [x] 切片 0：变更包四件套建档（proposal/spec/design/tasks）
- [ ] 切片 1：domain + storage SCHEMA_V3（Task 改造 + 5 新表 + 迁移）
- [ ] 切片 2：core（checkin_service + 状态/统计纯函数 + library_service + snapshot 口径）
- [ ] 切片 3：protocol v2 + CLI（task 新参数 + checkin/decrement/undo/overview + library 组）
- [ ] 切片 4：Tauri commands + 插件桥同步
- [ ] 切片 5：前端卡片体系（HeatmapCell / TaskCard×4 / TasksView 卡片墙 / 任务编辑器 / 任务详情 / Today 改造）
- [ ] 切片 6：前端日期主库（侧边栏 / 网格 / 详情 / 归档三选一 / 已归档页）
- [ ] 切片 7：integration.rs 链路 + check.sh 全绿 + dev-log/README/architecture/PLUGIN_API 同步

## 实施记录

（实施中填写：提交哈希、偏离说明）
