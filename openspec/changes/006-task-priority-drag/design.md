# Design 006: 任务星级 + 自由拖拽排序

## 数据模型

- `priority INTEGER NOT NULL DEFAULT 0`（0=未评级，1–5）。
- `sort_order REAL NOT NULL DEFAULT 0`（分数索引）。
- SCHEMA_V5 两列 ALTER，事务内含 PRAGMA user_version=5（沿用 V4 模式）。

## 排序口径

`priority DESC, sort_order ASC, created_at ASC`，在 `wall_task_views` / `wall_task_views_on` 统一
（Today 页不受影响，仍状态排序+会话快照）。新任务 sort_order = 档内 max + 1024 → 档末，
与「新任务固定最末」一致。落位取邻居中点；间隙耗尽需 rebalance 的概率在个人规模下忽略
（最坏 ~50 次相邻插入才逼近 f64 精度，发生后可一键归一化，留作后续）。
  事后补丁：V5 迁移把存量任务 sort_order 全置 0 导致中点塌缩（拖拽看似无效），
  SCHEMA_V6 按档内 created_at 回填 1024 步进（storage 单测覆盖）。

## 拖拽落点 → 服务端调用

前端把落点翻译成 `(before_id, after_id)`（全局序列中的上下邻居；列内子序列与全局序列同序，
故列内落点可直接映射）。目标档 = 落点下方卡片的 priority（落墙尾 = 上方卡片）。
同档 → `move_task_position(id, null, before, after)`；跨档 → confirmDialog 确认后
`move_task_position(id, 目标档, before, after)`，取消不动。

## 服务端

`move_task_position`：校验 priority 域 → 需要时改 priority → 计算新 sort_order
（before/after 邻居中点；缺侧取 ±1024；均缺 = 档内 max+1024）→ 写 activity log → snapshot 刷新。
星级校验在 create/update/move 三处共用（0..=5）。

## 前端

- TaskCard：标题旁 ★×N（>0 时，琥珀色 #F5B942）；⋯ 菜单加「星级」子项（无/1–5）。
- TaskEditor：星级行（点第 N 颗设 N，再点清除为 0）。
- 拖拽：HTML5 DnD（卡片 wrapper draggable），插入指示线（2px accent），
  drop 计算 before/after 后按上节规则调用；拖动中不改变数据，落点确定后一次性写回。
  走查修订：WKWebView 要求 dragstart 必须 setData（否则拖拽不启动）；逻辑抽为共享
  src/dnd.ts（useTaskDnd），首页今日卡同用；首页重建排序 = 完成沉底 + 组内墙口径。
