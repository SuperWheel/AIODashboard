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
- [x] 拖拽交互 v3：起拖原位腾空 + 目标占位 + FLIP 让位 + 预览序列原位提交；纯函数测试覆盖索引逻辑（已由 v4 取代：隐藏 source 实际被 React 重挂载，WKWebView 会取消拖拽）
- [x] 拖拽交互 v4：Pointer Events + document 稳定事件链 + DOM DragOverlay；墙外/Esc/失焦取消；目标档内邻居提交；隔离数据库实拖覆盖首页、均衡混排、类型分区和墙外释放
- [x] 拖拽稳定 v4.1：稳定布局命中 + 双列优先 + 中线迟滞 + 6px 重排再武装 + 每帧一次预览；连续 FLIP 从当前视觉位置衔接
- [x] 拖拽稳定 v4.2：浮层清除继承动画并改为相对位移；命中/FLIP 分层；首页网格隔离渲染；任务卡 memo + 可中断预览更新
- [x] 拖拽稳定 v4.3：FLIP 快照后同步提交预览；拖拽期间暂停非拖动卡的 hover 位移，消除让位闪烁/跳跃
- [x] 拖拽稳定 v4.4：可取消 WAAPI 动画从当前视觉位置接续；仅重启动过的卡片；首格优先占位 + 当前槽稳定区 + 10px 目标再武装
