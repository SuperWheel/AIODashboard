# Change Proposal 006: 任务星级 + 自由拖拽排序（task-priority-drag）

> 状态：已完成（2026-09-02） · 提出日期：2026-09-02
> 关联：004（循环规则）、005（滚动年热力图）、task-wall-layout-preferences

## Why

用户希望任务有重要性区分（星级 1–5）且能手动拖拽排序。任务墙目前只按 created_at 升序，无法表达重要性与个人排序偏好。

## What（关键决策均已由用户拍板 2026-09-02）

1. **星级**：Task 加 `priority`（0=未评级，1–5 星）；在任务编辑器与卡片 ⋯ 菜单设置；卡片标题旁显示星级。
2. **排序**：任务墙 = priority 降序分档 → sort_order 升序 → created_at 升序。新任务追加到其星级档末尾（保留「新任务固定最末、与打卡状态解耦」规则）。
3. **拖拽**：卡片可拖动改 `sort_order`（分数索引取邻居中点）。同星级档内拖动直接生效；**跨档拖动先弹确认框**，确认后任务改为目标档星级并落位，取消则不动。
4. 均衡混排的列分配算法不变——拖的是发牌序列，不是列。
5. **协议**：Task JSON 加 `priority` / `sort_order`，`meta.schema_version` 4→5；CLI `task create/update` 加 `--priority`；拖拽位置写为 GUI 专有（CLI 暂不提供位置移动）。
6. Today 页不受星级影响（保持状态排序 + 会话级位置快照）。

## Unknown / 风险

- 跨档确认框复用 confirmDialog，文案含源/目标星级。
- `sort_order` 稀疏索引长期使用后理论上需 rebalance（个人规模可忽略，design 注明）。
