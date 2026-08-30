# PLUGIN_API — AIODashboard 插件开发指南（plugin.protocol/v1）

> 面向开发者与 AI Agent 的插件开发完整参考。示例见 `examples/plugins/`
> （echo 最小示例、pomodoro 全能力示范，两者都内嵌面向 AI 的 AGENTS.md）。

## 1. 心智模型

AIODashboard 插件 = **加载进面板内部的 TS/JS 模块**（Obsidian 同构）：

- 插件生命周期 = 面板生命周期：面板后台留存（关窗进托盘）时插件继续运行
- 插件与核心五视图走同一 ModuleRegistry——内置功能与插件平级
- 插件是**第三类客户端**（与 GUI / CLI 平级）：所有领域写入以 `actor=plugin:<id>` 记入审计日志
- 权限是声明式的：manifest 里没写的能力，调用即被拒绝

## 2. 插件包结构

```text
<plugins_dir>/<plugin_id>/
├── manifest.json    # 清单：身份、权限、贡献点
├── main.js          # 入口（ESM 单文件；可由 TS+esbuild 预编译）
└── AGENTS.md        # 面向 AI 的开发说明（强烈建议提供）
```

插件目录位置：`~/Library/Application Support/AIODashboard/plugins/`（环境变量 `DASHBOARD_PLUGINS_DIR` 覆盖）。
**目录名必须等于 manifest.id。**

### manifest.json

```json
{
  "id": "com.example.myplugin",
  "name": "我的插件",
  "version": "0.1.0",
  "entry": "main.js",
  "description": "一句话描述",
  "permissions": {
    "network": ["api.github.com"],
    "events": ["task.completed"],
    "cron": ["*/30 * * * *"]
  },
  "contributions": {
    "today_cards": [{ "id": "card" }],
    "views": [{ "id": "view", "title": "我的视图" }],
    "commands": [{ "id": "do", "title": "做一件事" }]
  }
}
```

- `id`：反向域名（小写字母/数字/连字符，≥2 段），全局唯一
- `permissions.network`：允许 `api.fetch` 访问的精确 host 列表
- `permissions.events`：允许订阅的领域事件；当前支持 `task.completed` `task.created` `inbox.added` `note.created`（`panel.*` 无需声明）
- `permissions.cron`：允许注册的 cron 表达式（5 段）；由 Rust 侧驱动，后台不受 webview 定时器节流影响
- 未知字段忽略（向前兼容）；两边校验（CLI 与面板）规则一致

## 3. 入口与生命周期

```js
export async function onload(api) {
  // 注册 UI、订阅事件、初始化状态
}
export async function onunload() {
  // 可选。注册项与订阅由宿主自动清理
}
```

- 启用（首次确认 / 插件页开关）→ `onload`；停用或重载 → `onunload` + 宿主反注册
- **看门狗**：`onload` 超过 8 秒未完成或抛错 → 插件自动停用（安全模式思路）
- 新插件首次发现会弹权限确认；「暂不启用」登记为停用、不再询问

## 4. Plugin API

| 面 | 方法 | 说明 |
|---|---|---|
| `api.pluginId` | — | 本插件 id |
| `api.react` | — | 宿主共享单实例 React：`createElement` `useState` `useEffect` … |
| `api.core.today()` | 读 | `context today` 同源数据 |
| `api.core.listTasks(scope?)` | 读 | `"all"/"active"/"archived"`（v2 起；旧 open/done/today/overdue 已移除） |
| `api.core.createTask(title, target?)` | 写 | 创建打卡任务；target=每日目标（默认 1） |
| `api.core.checkinTask(id)` | 写 | 打卡 +1（幂等账本） |
| `api.core.archiveTask(id)` | 写 | 归档任务（停止打卡，历史保留） |
| `api.core.deleteTask(id)` | 写 | |
| `api.core.search(query)` | 读 | 全局搜索 |
| `api.core.addInboxItem(content)` | 写 | |
| `api.core.createNote(title, body)` | 写 | |
| `api.storage.kv.get(key)` | 数据 | `string \| null` |
| `api.storage.kv.set(key, value)` | 数据 | 值 ≤1MB；建议存 JSON 字符串 |
| `api.storage.kv.delete(key)` | 数据 | 返回是否确有删除 |
| `api.storage.kv.list(prefix?)` | 数据 | 本插件命名空间内 |
| `api.fetch(url)` | 网络 | host 必须在白名单；返回 `{status,text,json}`；仅 http/https GET |
| `api.events.on(topic, fn)` | 事件 | 返回取消函数；handler 抛错不影响宿主 |
| `api.events.emit(topic, payload)` | 事件 | 插件间/自身广播（自定义 topic 无需声明） |
| `api.registerCron(expr, fn)` | 调度 | expr 需在 manifest 声明；到点由 Rust 事件驱动 |
| `api.ui.registerTodayCard({id,title,size?,component})` | UI | Today 页 Bento 卡片；`size` = `"sm"`(3列) / `"md"`(默认, 4列) / `"lg"`(6列) |
| `api.ui.registerView({id,title,icon?,component})` | UI | 侧边栏独立视图 |
| `api.ui.registerCommand({id,title,handler})` | UI | ⌘K 面板命令 |
| `api.log.info/warn/error(msg)` | 日志 | 前缀 `[<plugin_id>]` |

写操作自动以 `actor=plugin:<id>` 记审计，`dashboard activity --limit 50` 可查。

**UI 组件签名**：卡片 `({ api, onChanged, today }) => ReactNode`；视图 `({ api, onChanged, onNav, refreshKey, today }) => ReactNode`。

**UI 与双主题（必须遵守）**：面板是浅色/深色双主题，插件 UI 只允许使用语义 token 类名，
禁止写死色值（如 `bg-[#161a22]`、`text-slate-400`、`text-white`、内联 hex）：

- 容器：`rounded-2xl border border-line bg-surface shadow-card`（卡片根节点加 `h-full` 填满 Bento 格位）
- 文字：`text-ink`（主）/ `text-ink2`（次）/ `text-ink3`（弱、占位）
- 强调：`text-accent`、`bg-accent text-onaccent`（主按钮）、`bg-hover`（hover/弱底色）
- 状态：`text-danger` / `text-warn` / `text-info` / `text-violet`

写死的色值在另一个主题下会不可读；宿主不兜底。示例见 `examples/plugins/`。

## 5. 推荐模式（重要）

**权威状态存 kv 时间戳，不存组件内存。**

```js
// 开始：记下结束时间点
await api.storage.kv.set("ends_at", new Date(Date.now() + 25 * 60_000).toISOString());
// 渲染：interval 只负责重绘，正确性 = 时间戳比较
const ends = await api.storage.kv.get("ends_at");
const remaining = Math.max(0, new Date(ends).getTime() - Date.now());
```

这样面板重启、关窗进托盘、后台节流都不会让状态走样。参考 pomodoro 示例（含到点去重）。

## 6. 开发工作流

```bash
# 1. 脚手架（零工具链 JS 模板）
dashboard plugin new com.example.myplugin

# 2. 编辑 main.js 实现功能（AI 可直接读本目录 AGENTS.md + 本文档）

# 3. 校验
dashboard plugin dev com.example.myplugin   # manifest/权限/贡献点/入口检查
dashboard plugin dev                        # 检查全部插件

# 4. 面板加载：首次发现会弹权限确认；改完代码在「插件」页点「重载」热更新
# 5. 审计与启停
dashboard activity --limit 20
dashboard plugin disable com.example.myplugin
```

TS 开发：模板即纯 JS ESM，无需构建；想用 TS 就 `tsc/esbuild` 预编译成单文件 `main.js`（React 以宿主实例为准，不要打包 react 进插件）。

## 7. 安全与信任（务必了解）

- 插件代码运行在面板 webview 内，拥有其声明权限对应的能力；**权限清单是"守门"，不是"沙箱"**
- 只安装你信任来源的插件；分发走目录拷贝 / Git（中心市场在 Roadmap 远期）
- 网络：`api.fetch` 是唯一出口（白名单 + 全量审计）；不要绕过
- 异常插件的三道闸：manifest 双端校验 → onload 看门狗（8s）→ 插件页一键停用

## 8. 版本与兼容

- 本协议版本：`plugin.protocol/v1`（随 `meta.schema_version` 体系记录于 dev-log）
- 新增能力保持向后兼容；破坏性变更会升协议版本并迁移
- 领域事件、UI 模板库会持续扩充——欢迎在审计日志之外，用 `api.log` 观察行为
