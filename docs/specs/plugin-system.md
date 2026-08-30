# Spec：插件系统 plugin-system/v1

> 状态：提案已确认（2026-08-30）· 模式：Spec · TDD：测试用例清单确认后实施
> 关联：docs/architecture.md · 设计文档 §26（daemon 预留）

## 决策记录（已拍板）

- **形态**：前端嵌入式 TS 插件系统（Obsidian 同构），插件生命周期 = 面板生命周期
- **宿主**：面板即宿主，不做 daemon；托盘常驻（close-to-tray）为配套必做
- **核心四实体**（Task/Project/Note/Inbox）保持 host 服务，不插件化；仅视图经 ModuleRegistry 模块化
- **首个插件**：番茄钟（P2）
- **节奏**：P1+P2 切片连推，每片独立提交并通过门禁
- 排除项及理由见会话提案记录：Rust dylib（无稳定 ABI/崩溃连坐）、外部进程（不满足嵌入语义）、WASM（工具链门槛，留作远期沙箱档位）

## Why

个人面板走向 All-in-One 统一面板：功能以统一模块契约接入，开发者（尤其 vibe coding / AI Agent）可自助开发插件。现有 CLI JSON 信封、activity log、权限 actor 机制与架构红线恰好构成插件系统的地基——插件是**第三类客户端**（与 GUI/CLI 平级）。

## ADDED

### A1 插件包格式

- 位置：`<data_dir>/plugins/<plugin_id>/`（macOS：`~/Library/Application Support/AIODashboard/plugins/`；环境变量 `DASHBOARD_PLUGINS_DIR` 覆盖）
- `manifest.json`：
  ```json
  {
    "id": "com.leeyl.pomodoro",
    "name": "番茄钟",
    "version": "0.1.0",
    "entry": "main.js",
    "permissions": {
      "network": [],
      "events": ["task.completed"],
      "cron": ["*/1 * * * *"]
    },
    "contributions": {
      "views": [{ "id": "pomodoro", "title": "番茄钟" }],
      "todayCards": [{ "id": "pomodoro-card" }],
      "commands": [{ "id": "pomodoro.start", "title": "开始专注" }]
    }
  }
  ```
  - `id`：反向域名，全库唯一；`permissions` 声明式权限（未声明即无权）
- `main.js`：单文件 ES module（开发模板 esbuild 打包，TS strict）
- `AGENTS.md`：插件目录内面向 AI 的开发说明（P3 脚手架生成）

### A2 插件宿主（前端 TS，apps/desktop）

- **loader**：`plugin_load_source(id)` 命令取源码 → blob URL 动态 `import()` → 调用插件 `export onload(api)`；`onunload()` 时由宿主反注册其全部贡献点（视图/卡片/命令/事件/cron）
- **ModuleRegistry**：视图 / Today 卡片 / ⌘K 命令的统一注册表；核心五视图改走同一注册路径（dogfooding）
- **API 桥 `api`**（插件唯一可touch的面，禁止拿到原生 invoke）：
  - `api.core`：task/note/inbox/search/context 的读写包装 → Tauri command → core；审计 actor=`plugin:<id>`
  - `api.storage.kv`：get/set/delete/list（Rust 侧按插件 id 强制命名空间隔离）
  - `api.fetch(url, init)`：仅 manifest 白名单 host，经 Rust 命令代理（可审计）
  - `api.events.on/off/emit`：领域事件 v1 = activity_log 投影（随 4s 轮询派发）+ 面板事件 `panel.refresh/show/hide`
  - `api.registerCron(expr, handler)`：Rust tokio 驱动触发 → 事件派发（后台窗口定时器节流的精度保障）
  - `api.registerCommand(title, handler)`：进 ⌘K 面板
  - `api.registerView(view)` / `api.registerTodayCard(card)`：React 组件经 `api.react`（宿主共享单实例：createElement/hooks）
  - `api.log(level, msg)`：写插件日志文件
- **权限执行**：未声明权限的调用 → 拒绝 + `plugin.denied` 事件 + activity log
- **安全模式**：设置页一键禁用全部插件（坏插件卡 UI 的逃生舱）

### A3 Rust / Storage（src-tauri + dashboard-storage）

- **SCHEMA_V2**（只增不改）：
  - `plugin_registry(id TEXT PK, enabled INTEGER NOT NULL DEFAULT 1, installed_at TEXT NOT NULL)`
  - `plugin_kv(plugin_id TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY(plugin_id, key))` + 索引 `idx_plugin_kv_plugin`
- **Tauri commands**：`plugin_list` / `plugin_set_enabled` / `plugin_load_source` / `plugin_kv_get|set|delete|list` / `plugin_http_fetch`
  - `plugin_http_fetch`：校验调用方插件的 manifest 白名单 → 代理请求 → activity log（网络行为全审计）
- **cron 调度器**：启动时收集启用插件的 cron 声明，tokio interval 触发 → Tauri event → 前端桥派发给插件
- **托盘常驻**：tauri `tray-icon` feature；关窗=隐藏，托盘菜单可退出；重开窗口恢复状态

### A4 CLI（apps/cli）

- `dashboard plugin list [--json]`：列出已装插件（id/名称/版本/启用状态）
- `dashboard plugin enable <id>` / `disable <id>`：写 `plugin_registry` + activity log（actor=cli）
- exit code 沿用现有语义（not found → 3）；**JSON 信封字段不变，schema_version 保持 "1"**（增量能力，dev-log 记录）

### A5 示例与文档

- `echo` 插件（P1，测试锚点：注册一张 Today 卡片显示文本 + kv 计数）
- `pomodoro` 插件（P2：计时、kv 持久化、task.completed 联动、Today 卡片 + 独立视图）
- `docs/PLUGIN_API.md` + `dashboard plugin new/dev` 脚手架（P3）

## MODIFIED

- `App.tsx`：`ViewName` 硬编码联合类型 → ModuleRegistry 动态渲染（现有五视图行为兼容，⌘K 导航映射保留）
- `scripts/check.sh` + `.github/workflows/ci.yml`：新增前端 vitest 步骤（门禁六步）
- `dashboard-domain`：`Actor` 枚举新增 `Plugin(String)`（activity_log.actor 形态 `plugin:<id>`）
- `apps/desktop/src-tauri`：tauri 增 `tray-icon` feature
- CSP：v1 保持 `null`（blob 动态加载需要）；收紧列入 TODO（后续限定 blob: 来源）

## REMOVED

- 无。现有功能全部保留。

## 红线核对

| 红线 | 落实 |
|---|---|
| SQL 只在 storage | 插件经 command → core → repo，插件无 SQL |
| 业务只在 core | 插件是客户端，新增领域逻辑仍进 core |
| 前端不碰 DB | 插件与前端同界，只经 invoke |
| 变更留 activity log | actor=`plugin:<id>` 全量审计 |
| 迁移只增不改 | V1→V2 新表，不改历史分支 |
| 时间边界唯一入口 | 不新增边界逻辑，cron 由 Rust chrono 处理 |
| 库代码禁 unwrap/expect | 新代码遵守 |

## 已知限制（v1 如实记录）

- 插件仅 TS/JS 一种语言；CLI 不能无头调用插件逻辑（插件驻留 UI）
- 坏插件可卡 UI 线程（缓解：安全模式；看门狗 P3）
- 领域事件延迟 ≤ 轮询周期（4s，activity_log 投影）
- 信任模型 = Obsidian 同款：文档明示「只装信任来源的插件」

## Tasks

### P1 协议与宿主（切片 1–3）
- [ ] T-S1 storage：SCHEMA_V2（plugin_registry + plugin_kv）+ 两个 repo + 迁移/隔离测试
- [ ] T-S2 manifest 结构 + Rust 校验器 + 单测
- [ ] T-S3 tauri commands（plugin_list/set_enabled/load_source/kv_*/http_fetch 白名单）+ Actor::Plugin 贯通
- [ ] T-S4 CLI `plugin list/enable/disable` + 集成测试
- [ ] T-S5 前端 vitest 基建 + check.sh/CI 增步
- [ ] T-S6 前端插件宿主：loader + ModuleRegistry + API 桥 + 权限执行 + 单测
- [ ] T-S7 echo 示例插件 + 端到端人工验证
- [ ] T-S8 dev-log + check.sh 全绿 + 切片提交

### P2 UI 与生命周期（切片 4–6）
- [ ] T-S9 槽位渲染：插件视图页签 / Today 卡片 / ⌘K 命令接入
- [ ] T-S10 托盘常驻 + 关窗隐藏 + 重开恢复
- [ ] T-S11 插件管理最小 UI（列表/启用开关/权限展示）
- [ ] T-S12 pomodoro 插件（kv 持久化 + 事件联动 + 卡片 + 视图）
- [ ] T-S13 dev-log + 门禁 + 切片提交

### P3 生态（后续，可另开 spec 细化）
- [ ] `plugin new/dev` 脚手架（esbuild 模板内嵌 AGENTS.md）+ docs/PLUGIN_API.md
- [ ] cron 注册 UI 化 + 权限确认 UI（安装时）
- [ ] 插件加载看门狗（加载超时自动禁用）

## 测试用例清单（TDD：确认后按此先行写测试）

**Rust（cargo，现有基建）**
- [ ] T1 plugin_kv repo：写入/覆盖/删除/列出；**命名空间隔离**（插件 A 读不到插件 B 的键）
- [ ] T2 迁移 V1→V2：user_version=2；旧表数据完整；新表存在
- [ ] T3 plugin_registry repo：默认启用；enable/disable 持久化往返
- [ ] T4 CLI 集成：`plugin list --json` 信封形状；enable/disable 落库 + activity log（actor=cli）；未知 id → exit 3 + 错误信封
- [ ] T5 http 白名单：命中放行、未命中拒绝（错误信封 + 审计记录）
- [ ] T6 manifest 校验：缺字段 / 非法 id / entry 文件缺失 → 明确错误码

**前端（vitest，新基建）**
- [ ] T7 manifest 校验（loader 侧）与 Rust 规则一致
- [ ] T8 loader：onload 被调用；onunload 后视图/命令/事件/cron 全部反注册
- [ ] T9 ModuleRegistry：注册→查询→注销；id 冲突拒绝
- [ ] T10 权限桥：未声明 network 调 api.fetch → 拒绝；kv 越权命名空间 → 拒绝
- [ ] T11 事件总线：emit→handler 收到；handler 抛错不炸宿主

**端到端 / 人工**
- [ ] T12 echo 插件：装载 → Today 卡片显示 → CLI disable 后卡片消失
- [ ] T13 pomodoro：计时状态跨刷新存活（kv）；task.completed 联动；关窗到托盘后台仍计时
- [ ] T14 托盘：关窗 → 进程留存 → 托盘重开窗口状态恢复
- [ ] T15 门禁：check.sh 六步全绿；CI 同步通过
