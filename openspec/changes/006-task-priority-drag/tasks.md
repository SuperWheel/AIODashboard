# Tasks 006: 任务星级 + 自由拖拽排序

## 测试清单（先确认后实现；core 链路 + 迁移）

- [x] create：默认 priority=0、sort_order=档末；--priority 3 生效；priority=6 拒绝
- [x] update：priority 可改；越界拒绝
- [x] move_task_position：档内中点排序生效；档首/档末落位；跨档改 priority + 落位
- [x] 墙排序：priority DESC → sort_order ASC → created_at ASC（含按日翻页）
- [x] 迁移：V4→V5 两列默认值、user_version=5、旧数据保留
- [x] integration.rs：schema_version="5"、CLI --priority 链路

## 实现

- [x] domain：Task + priority/sort_order（serde default）
- [x] storage：SCHEMA_V5 + TASK_COLS/map/insert/TaskPatch + max_sort_order_in_band
- [x] core：create/update priority 校验 + move_task_position + 墙排序切换
- [x] protocol v5 + CLI --priority
- [x] Tauri：update_task 参数 + move_task_position command
- [x] 前端：types/api + TaskEditor 星级行 + TaskCard 星级显示/⋯ 菜单 + 墙 DnD（指示线 + 跨档确认框）
- [x] check.sh 全绿 + dev-log
