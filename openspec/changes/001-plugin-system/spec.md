# Spec 001: 插件系统（plugin-system/v1）

> 验收证据与测试函数名见 tasks.md；技术决策见 design.md。

## ADDED

### ADDED-1：插件包格式

- **要求**：插件位于 `<data_dir>/plugins/<plugin_id>/`（macOS：`~/Library/Application Support/AIODashboard/plugins/`，环境变量 `DASHBOARD_PLUGINS_DIR` 覆盖），包含：
  - `manifest.json`：`id`（反向域名，全库唯一）、`name`、`version`、`entry`、`permissions`（network 白名单 host / events 订阅 / cron 表达式，未声明即无权）、`contributions`（views / todayCards / commands）。
  - `main.js`：单文件 ES module，`export onload(api)` / `onunload()`。
  - `AGENTS.md`：目录内面向 AI 的开发说明（脚手架生成）。
- **验收**：合法 manifest 通过校验；缺字段 / 非法 id / entry 文件缺失 / 非法 host / 未知事件 / 错误 cron / 贡献点冲突 → 明确错误。

### ADDED-2：前端插件宿主（apps/desktop）

- **要求**：loader 经 `plugin_load_source(id)` 取源码 → blob URL 动态 `import()` → 调用 `onload(api)`；`onunload()` 时宿主反注册其全部贡献点（视图/卡片/命令/事件/cron）。
- **要求**：ModuleRegistry 统一管理视图 / Today 卡片 / ⌘K 命令；核心五视图走同一注册路径（dogfooding）。
- **要求**：API 桥 `api` 是插件唯一可 touch 的面（禁止拿到原生 invoke）：
  - `api.core`：task/note/inbox/search/context 读写包装，审计 actor=`plugin:<id>`
  - `api.storage.kv`：get/set/delete/list，Rust 侧按插件 id 强制命名空间隔离
  - `api.fetch(url, init)`：仅 manifest 白名单 host，经 Rust 命令代理
  - `api.events.on/off/emit`：领域事件 v1 = activity_log 投影（随 4s 轮询派发）+ 面板事件 `panel.refresh/show/hide`
  - `api.registerCron(expr, handler)` / `api.registerCommand(title, handler)` / `api.registerView(view)` / `api.registerTodayCard(card)`（经宿主共享 React 单实例）/ `api.log(level, msg)`
- **要求**：权限执行——未声明权限的调用拒绝 + `plugin.denied` 事件 + activity log。
- **要求**：安全模式——一键禁用全部插件（坏插件卡 UI 的逃生舱）。
- **验收**：T7–T11（vitest）。

### ADDED-3：存储（SCHEMA_V2，只增不改）

- **要求**：新表 `plugin_registry(id TEXT PK, enabled INTEGER NOT NULL DEFAULT 1, installed_at TEXT NOT NULL)`；`plugin_kv(plugin_id, key, value, updated_at, PRIMARY KEY(plugin_id, key))` + 索引 `idx_plugin_kv_plugin`。
- **验收**：迁移 V1→V2 后 user_version=2、旧表数据完整；kv 命名空间隔离（插件 A 读不到插件 B）。

### ADDED-4：Tauri 命令层与调度

- **要求**：`plugin_list` / `plugin_set_enabled` / `plugin_load_source` / `plugin_kv_get|set|delete|list` / `plugin_http_fetch`（校验调用方插件白名单 → 代理请求 → activity log）。
- **要求**：cron 调度器由 Rust tokio 驱动（croner 解析 5 段表达式），触发 → Tauri 事件 → 前端桥按插件+表达式路由；插件启停后 rescan。
- **要求**：`Actor` 枚举新增 `Plugin(String)`，activity_log.actor 形态 `plugin:<id>`。
- **验收**：白名单命中放行/未命中拒绝；插件写入带 actor 归因。

### ADDED-5：CLI（apps/cli）

- **要求**：`dashboard plugin list [--json]` / `plugin enable <id>` / `plugin disable <id>`，写 `plugin_registry` + activity log（actor=cli）。
- **要求**：JSON 信封字段不变，`schema_version` 保持 "1"（增量能力）。
- **验收**：`plugin list --json` 信封形状正确；enable/disable 落库；未知 id → exit 3 + 错误信封。

### ADDED-6：生命周期与托盘

- **要求**：托盘常驻（tauri `tray-icon` feature）：关窗=隐藏，托盘菜单可退出，重开窗口恢复状态。
- **要求**：加载看门狗——onload/onunload 超时（8s）或抛错 → 插件自动停用，可一键再启用。
- **要求**：首次发现的插件不自动启用，弹权限确认 UI（网络/事件/定时逐项展示）；「暂不启用」登记为停用不再询问。
- **验收**：关窗 → 进程留存 → 托盘/Dock 重开恢复（人工走查）。

### ADDED-7：示例与生态

- **要求**：echo 插件（Today 卡片 + kv 计数，测试锚点）；pomodoro 插件（计时、kv 持久化、task.completed 联动、Today 卡片 + 独立视图）。
- **要求**：`docs/PLUGIN_API.md` 开发者文档；`dashboard plugin new/dev` 脚手架（内嵌 AGENTS.md）。
- **验收**：echo 装载 → 卡片显示 → CLI disable 后卡片消失；pomodoro 计时状态跨刷新存活、托盘后台运行（人工走查）。

## MODIFIED

- `App.tsx`：`ViewName` 硬编码联合类型 → ModuleRegistry 动态渲染（现有五视图行为兼容，⌘K 导航映射保留）。
- `scripts/check.sh` + `.github/workflows/ci.yml`：新增前端 vitest 步骤（门禁六步）。
- `dashboard-domain`：`Actor` 枚举新增 `Plugin(String)`。
- `apps/desktop/src-tauri`：tauri 增 `tray-icon` feature。
- CSP：v1 保持 `null`（blob 动态加载需要）；收紧列入 TODO（后续限定 blob: 来源）。

## REMOVED

- 无。现有功能全部保留。

## 非本变更范围（明确不做）

- dashboardd 守护进程（设计文档 §26 预留，后续独立变更包）。
- 多语言插件（仅 TS/JS）与 CLI 无头调用插件逻辑。
- WASM 沙箱档位（远期）。
- CSP 收紧（后续 TODO）。
