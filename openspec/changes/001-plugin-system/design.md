# Design 001: 插件系统 · 关键设计决策

## 决策 1：运行时形态 = 前端嵌入式 TS（Obsidian 同构）

- **选择**：插件是前端 TS/JS 模块，生命周期 = 面板生命周期。
- **理由**：满足「嵌入面板」语义；复用宿主 React 实例与既有轮询；工具链零负担（单文件 main.js）。
- **否决项**：
  - Rust dylib——无稳定 ABI、崩溃连坐、开发门槛高。
  - 外部进程 + stdio JSON-RPC——不满足嵌入语义（无法直接贡献视图/卡片组件）。
  - WASM——工具链重，留作远期沙箱档位。

## 决策 2：宿主 = 面板即宿主，不做 daemon

- **选择**：插件驻留 GUI；托盘常驻（close-to-tray）为配套必做，保证后台运行语义。
- **理由**：个人面板常驻桌面，无 7×24 服务需求；daemon 属过度设计。
- **后果**：CLI 不能无头调用插件逻辑（已知限制）；cron 触发归 Rust 以规避窗口节流。

## 决策 3：核心四实体不插件化，仅视图模块化

- **选择**：Task/Project/Note/Inbox 保持 host 服务；ModuleRegistry 只模块化视图/卡片/命令贡献点。
- **理由**：四实体是其他插件与功能的依赖地基，拆掉会动摇 core 唯一业务实现的红线。

## 决策 4：权限模型 = manifest 声明式 + 首次发现确认

- **选择**：未声明即无权；拒绝调用 + `plugin.denied` 事件 + activity log；安装时刻弹权限确认 UI，逐项展示网络/事件/定时。
- **理由**：与 AI 协议的 actor 审计体系同构；信任模型 = Obsidian 同款（文档明示只装信任来源）。
- **后果**：修正了 P1 的自动启用行为（首次发现不再默认启用）。

## 决策 5：cron 触发归 Rust

- **选择**：croner 解析 5 段表达式，tokio 驱动 `CronScheduler`，500ms 切片睡眠响应重排；到点 emit `plugin-cron` 事件 → 前端 CronRegistry 路由。
- **理由**：后台窗口定时器被节流，精度无保障；Rust 侧驱动与语言无关且可测试。
- **实现细节**：锁毒化用 `into_inner()` 恢复，库代码零 unwrap/expect。

## 决策 6：脚手架 = 零工具链纯 JS 模板（实施偏离）

- **选择**：`plugin new` 生成纯 JS 单文件模板，内嵌 AGENTS.md。
- **理由**：与示例插件一致，AI 生成即可运行，无构建依赖。
- **偏离说明**：spec 原案为 esbuild 打包 TS 模板；TS 用户可自行预编译为单文件 main.js，已写入文档。

## 红线核对

| 红线 | 落实 |
|---|---|
| SQL 只在 storage | 插件经 command → core → repo，插件无 SQL |
| 业务只在 core | 插件是客户端，新增领域逻辑仍进 core |
| 前端不碰 DB | 插件与前端同界，只经 invoke |
| 变更留 activity log | actor=`plugin:<id>` 全量审计 |
| 迁移只增不改 | V1→V2 新表，不改历史分支 |
| 时间边界唯一入口 local_today_range | 不新增边界逻辑，cron 由 Rust chrono 处理 |
| 库代码禁 unwrap/expect | 新代码遵守（cron 锁毒化 into_inner 恢复） |

## 数据模型（SCHEMA_V2）

- `plugin_registry(id TEXT PK, enabled INTEGER NOT NULL DEFAULT 1, installed_at TEXT NOT NULL)`
- `plugin_kv(plugin_id TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY(plugin_id, key))` + `idx_plugin_kv_plugin`

## 风险与已知限制

- 坏插件可卡 UI 线程（缓解：安全模式 + 8s 看门狗 + PluginErrorBoundary 渲染降级）。
- 领域事件延迟 ≤ 轮询周期（4s，activity_log 投影）。
- CSP 保持 null，收紧（限定 blob: 来源）留 TODO。
