# 番茄钟插件（AI 开发说明）

插件系统的能力示范：KV 持久化、设置、Today 卡片、独立视图、⌘K 命令、宿主 interval。

## 状态设计（可复用的模式）

- **权威状态是 kv 里的时间戳**（`session.ends_at`），不是组件内存。面板重启、关窗到托盘后按本机时钟恢复；不提供跨设备同步。
- 暂停态：`session.paused=true` + `remaining_s`，不写 `ends_at`；继续时用剩余秒数重算 `ends_at`。
- 组件里的 1s interval 只负责重渲染；**到点结算**走宿主 `api.registerInterval`，与 UI 是否打开无关。
- 到点判定：`ends_at` 仍在且已过期 → `completeSession` 删除 session，天然幂等防重复计数。
- 旧版 0.1.x 的裸 `ends_at` 会在首次加载时迁入 `session` 结构。

## 周期与计数

| KV 键 | 含义 |
|---|---|
| `session` | JSON：`mode / ends_at / duration_s / remaining_s / paused` |
| `cycle_position` | 当前块已完成的专注数；达到设置的 `long_break_every` 后建议长休；长休完成归零 |
| `count:YYYY-MM-DD` | 当日完成的专注番茄数 |
| `count:YYYY-MM-DD:min` | 当日专注分钟（按完成时的 duration 累加） |

跳过 / 重置只清 session，不推进 cycle、不计入完成。专注完成才 +1 并累加分钟。

## 能力对照

| 能力 | 用法 | 权限 |
|---|---|---|
| KV | `api.storage.kv.get/set/delete/list` | `storage_quota_bytes` |
| 设置 | `api.ui.registerSettings` + `api.settings.get` | `ui: settings` + contributions.settings |
| Today 卡片 | `api.ui.registerTodayCard`（`size: md`） | `ui: today_card` + contributions.today_cards |
| 独立视图 | `api.ui.registerView` | `ui: view` + contributions.views |
| ⌘K 命令 | 专注/短休/长休/暂停/跳过/重置 共 6 条 | `ui: command` + contributions.commands |
| 定时结算 | `api.registerInterval(tick, 1000)` | SDK 内置，停用自动清理 |

## 视觉

- 主题色复用宿主 CSS 变量：专注 `--accent`、短休 `--info`、长休 `--violet`；软背景用 `color-mix`。
- 圆环进度 = `1 - remaining/duration`；空闲态圆环为空、数字用 `--ink-3`。
- 不引入第二份 React / 外部资源；样式类均为宿主已有语义 token。

## 验证

1. 面板侧边栏出现「番茄钟」视图，Today 页出现卡片（md）
2. ⌘K 输入「番茄」或「专注」出现命令
3. 开始专注 → 圆环走动 → 暂停数字冻结 → 继续恢复
4. 关窗进托盘后再开，剩余时间正确
5. 完成 4 个专注（默认设置）后空闲建议「开始长休」；长休完成后 cycle 归零
6. 插件页可改时长设置并保存
