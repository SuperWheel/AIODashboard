# Spec：任务循环规则（004）

## ADDED 能力：循环规则（Recurrence）

### 数据模型

- `Recurrence`（domain）：
  ```rust
  enum Recurrence {
    Daily,
    Weekly { weekdays: Vec<u8> },  // ISO 1=周一…7=周日；空集 = 锚点日的星期
    Monthly,                        // 日号取区间起始日，短月 clamp 至当月末天
    Yearly,                         // 月日取区间起始日，2/29 平年 clamp 至 2/28
    Once,                           // 每日适用直到完成；完成达标即自动归档
  }
  ```
- 存储：`task_target_periods` 新列 `recurrence TEXT`（JSON 内部标签 `{"kind":"weekly","weekdays":[1,3]}`；NULL = daily）。SCHEMA_V4，加列与 `user_version=4` 同一事务（修正 003 中版本号与建表分步的缺陷）。
- `Task` JSON 增 `recurrence` 字段 = 当日生效规则（core 在 get/list 时回填，来自 `target_on(today)`）。

### 适用性判定（核心不变量）

某任务在某日 `applicable` ⇔ 活动区间覆盖该日 ∧ 存在目标区间覆盖该日（target>0）∧ 该区间 `recurrence.matches(start_day, day)`。

- `matches` 为纯函数；日期串解析失败时防御性返回 true（脏数据不隐藏任务）。
- 不适用日行为：不进 `context today`、`task checkin/decrement` 拒绝（错误信息"任务今天不适用"）、周/月/年热力图与主库热力图该日 `not_applicable`、missed 统计不计。

### 循环修改

`task update --recurrence`（或 GUI 编辑器）：闭合当前开放目标区间、明日起插入新规则区间——与目标修改同一机制，历史日期按当时规则统计。

### 一次性任务

- 适用性同 daily；`task checkin`（及 complete 兼容命令）使当日计数首次达标时，**同一操作内自动归档**：status→archived、活动区间闭合、写 `task.archive` activity、刷新快照；返回视图仍按归档前状态（completed）展示。
- 撤销/减少作用于已归档一次性任务：undo 允许（补偿账本），decrement 因归档被拒；任务保持归档，手动恢复后可继续。

### 任务墙数据源（MODIFIED）

- 新增 core `wall_task_views(conn)`：全部 `active` 任务的当日视图（**含** not_applicable），GUI TasksView 与新 Tauri command `task_wall_views` 使用。
- `context today`（TodayView）不变：仅今日适用任务。

## MODIFIED：AI 协议 v3

- `meta.schema_version`：2 → 3。
- `task create` / `task update` 增参数：`--recurrence <daily|weekly|monthly|yearly|once>`、`--weekdays <n,n,…>`（weekly 时必填可省，元素 1–7，去重升序）；stdin JSON 对应 `recurrence.kind` / `recurrence.weekdays`。
- `Task` JSON 增 `recurrence`（对象，同 domain 序列化）。
- `task show` 的目标区间列表含每段 `recurrence`。

## 兼容性

- V3 存量目标区间行 recurrence=NULL → daily，行为与升级前完全一致。
- 旧客户端忽略 `recurrence` 字段（serde 未知字段容忍）；schema_version 门禁按协议约定处理。
