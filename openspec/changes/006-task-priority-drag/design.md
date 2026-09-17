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

前端把落点翻译成 `(before_id, after_id)`。目标档 = 落点下方卡片的 priority
（落墙尾 = 上方卡片）；提交前只在目标档内向两侧查找邻居，禁止拿其他星级档的
`sort_order` 参与中点计算。列内子序列与全局序列同序，故列内落点可直接映射。
同档 → `move_task_position(id, null, before, after)`；跨档 → confirmDialog 确认后
`move_task_position(id, 目标档, before, after)`，取消不动。

## 服务端

`move_task_position`：校验 priority 域 → 需要时改 priority → 计算新 sort_order
（before/after 邻居中点；缺侧取 ±1024；均缺 = 档内 max+1024）→ 写 activity log → snapshot 刷新。
星级校验在 create/update/move 三处共用（0..=5）。

## 前端

- TaskCard：标题旁 ★×N（>0 时，琥珀色 #F5B942）；⋯ 菜单加「星级」子项（无/1–5）。
- TaskEditor：星级行（点第 N 颗设 N，再点清除为 0）。
- 拖拽：使用 Pointer Events，不再使用 HTML5 `draggable`。`pointerdown` 只建立候选会话，
  移动达到 5px 阈值后，先把当前卡片 DOM 克隆到 body 形成 `pointer-events:none` 的 fixed
  DragOverlay，再从布局序列移除原卡并用 FLIP 让其他卡片补位。移动、释放、取消均监听
  document/window，生命周期与会被 React 重排卸载的卡片节点完全解耦。
- 悬停：只有进入有效目标后，才把 drag id 插回预览序列并渲染为等高、同圆角的淡色占位；
  `pointermove` 只更新最新坐标，预览命中在下一动画帧合并计算；仍基于“先移除自身，再插
  目标前/后”的同一纯函数计算。指针落在卡片间隙时，先选水平方向最近的列，再从该列选
  纵向最近卡片，其他卡片 FLIP 让位。
- 防抽动：命中矩形会扣除卡片当前 FLIP translate，始终使用新布局坐标；卡片前 68% 区域
  表示占据其位置、底部才表示放在其后，边界保留 8px 迟滞；当前占位周围有 12px 稳定区，
  切换到其他目标还需累计移动 10px。这样左列/首格会及时让位，指针擦过边缘又不会反复换位。
- 动画接续：FLIP 同时保存“当前视觉位置”和“无 transform 布局位置”。新布局到来时，只处理
  真正换位的卡片；先读取旧动画中的当前位置，再取消旧动画并用 Web Animations 从该位置接到
  新位置（240ms ease-out）。新布局恰好等于当前视觉位置时只取消旧轨迹，不能让它继续朝旧终点走。
- 防漂移：卡片命中节点与 FLIP 视觉节点分为外/内两层；DragOverlay 克隆后清除所有 FLIP
  inline transform/transition，不从页面 `(0,0)` 做绝对大位移，而以源卡片 fixed 坐标为
  基准仅叠加指针增量。浮层不缩放、不使用昂贵 filter，只做 compositor transform。
- 跟手性能：首页把 `useTaskDnd` 收进今日任务网格，预览变化不再重渲染年度热力图、重要日
  和插件卡；TaskCard/TodayTaskCard memo 化。FLIP 快照后的实际换位用 `flushSync` 保证同帧提交，
  不能使用 React transition 延迟布局，否则后续指针帧会覆盖快照、让卡片从过期坐标起跳；
  拖拽期间同时暂停普通卡片的 hover 位移，避免 hover transform 与 FLIP transform 叠加闪烁。
- 释放：document `pointerup` 提交 ref 中的最后预览；服务端成功后先把相同序列乐观写回
  任务墙/首页 pin，再撤预览并刷新，避免闪回。未进入有效落点、墙外释放、Esc、
  `pointercancel` 或窗口失焦均取消并恢复原序；拖拽尾随 click 在 capture 阶段拦截，避免误打卡。
- 共享：逻辑集中在 `src/dnd.ts`（`useTaskDnd`），任务墙和首页今日卡共用；首页重建排序 =
  完成沉底 + 组内墙口径。
