# Echo 插件（AI 开发说明）

这是 AIODashboard 插件系统的最小示例。AI Agent 开发新插件时，复制本目录并修改。

## 目录结构

- `manifest.json`：清单。`id` 为反向域名且必须与目录名一致；`permissions` 声明式权限（未声明即无权）；`contributions` 声明 UI 贡献点。
- `main.js`：入口（`manifest.entry` 指向）。ESM 单文件，导出 `onload(api)` / 可选 `onunload()`。

## Plugin API 速查

- `api.pluginId` / `api.react`（createElement、hooks —— 宿主共享单实例 React）
- `api.core`：`today()` `listTasks(scope)` `createTask(title, dueAt?)` `setTaskStatus(id, status)`
  `deleteTask(id)` `search(query)` `addInboxItem(content)` `createNote(title, body)`
  —— 全部以 `actor=plugin:<id>` 写审计日志
- `api.storage.kv`：`get/set/delete/list`（服务端按插件 id 强制命名空间）
- `api.fetch(url)`：host 必须在 `manifest.permissions.network` 白名单内
- `api.events.on(topic, handler)`：领域事件须在 `permissions.events` 声明；
  `panel.refresh/show/hide` 无需声明。handler 抛错不会影响宿主
- `api.ui`：`registerTodayCard({ id, title, component })` / `registerView(...)` / `registerCommand(...)`
- `api.log.info|warn|error`

## 验证流程（CLI）

```bash
dashboard plugin list                # 新插件显示为 new，面板加载后自动登记
dashboard plugin disable <id>        # 停用后卡片/视图消失
dashboard activity --limit 20        # 检查 actor=plugin:<id> 审计
```
