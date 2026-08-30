# 番茄钟插件（AI 开发说明）

插件系统的完整能力示范：KV 持久化、领域事件订阅、Today 卡片、独立视图、⌘K 命令。

## 状态设计（可复用的模式）

- **权威状态是 kv 里的时间戳**（`ends_at`），不是组件内存。面板重启、关窗到托盘、跨设备时钟都一致。
- 组件里的 1s interval 只负责重渲染；正确性 = 时间戳比较。
- 到点判定用「ends_at 值变化」去重，防止重复计数。

## 能力对照

| 能力 | 用法 | 权限 |
|---|---|---|
| KV | `api.storage.kv.get/set/delete/list` | 无需声明（命名空间隔离） |
| 领域事件 | `api.events.on("task.completed", fn)` | `permissions.events` 声明 |
| Today 卡片 | `api.ui.registerTodayCard` | `contributions.today_cards` 声明 |
| 独立视图 | `api.ui.registerView` | `contributions.views` 声明 |
| ⌘K 命令 | `api.ui.registerCommand` | `contributions.commands` 声明 |

## 验证

1. 面板侧边栏出现「番茄钟」视图，Today 页出现倒计时卡片
2. ⌘K 输入「番茄」出现两条命令
3. 完成一个任务（卡片或 CLI `task complete`），空闲状态下自动开始倒计时
4. 开始后关闭窗口（进托盘），2 分钟后重开——倒计时仍在走且剩余时间正确（时间戳权威）
