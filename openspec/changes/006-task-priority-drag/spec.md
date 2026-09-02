# Spec 006: 任务星级 + 自由拖拽排序（task-priority-drag）

> 协议影响：`meta.schema_version` "4" → "5"（Task JSON 增 priority/sort_order 字段）。
> Schema 影响：SCHEMA_V5（tasks 增两列，只增不改）。

### ADDED-1：任务星级（priority）

- **要求**：`tasks.priority INTEGER NOT NULL DEFAULT 0`；合法域 0–5（0=未评级）；create/update 校验，越界报参数错误（exit code 2）。
- **要求**：编辑器与卡片 ⋯ 菜单可设置；卡片标题旁显示（>0 时显示 ★×N）。
- **验收**：core 测试覆盖默认值、设置、越界拒绝。

### ADDED-2：手动排序（sort_order）

- **要求**：`tasks.sort_order REAL NOT NULL DEFAULT 0`；新任务 = 其星级档内 max+1024（档末）；拖动落位 = 邻居中点（档首 = 首元素−1024，档末 = 末元素+1024）。
- **要求**：任务墙排序 = priority DESC, sort_order ASC, created_at ASC（core 统一，含按日翻页视图）。
- **要求**：`task_service::move_task_position(id, new_priority?, before_id?, after_id?)` 单用例完成改档+落位，写 activity log + 刷新 snapshot。
- **验收**：core 测试覆盖档内中点、档首/档末、新任务档末、跨档改级+落位。

### ADDED-3：拖拽交互（GUI）

- **要求**：均衡混排与类型分区两模式卡片均可拖拽；拖动中显示插入位置指示；落点在另一星级档时弹确认框（文案含源/目标星级），确认才生效。
- **要求**：拖拽不改变列分配算法（均衡发牌按序列重算）。

### MODIFIED-1：AI 协议 v5

- **要求**：Task JSON 增 `priority`/`sort_order`；`meta.schema_version="5"`；CLI `task create/update` 支持 `--priority 0..5`；integration.rs 断言升级。
- **迁移说明**：旧消费者忽略新字段即可；CLI 暂不提供位置移动命令（GUI 专有）。
