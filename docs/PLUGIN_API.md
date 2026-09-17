# AIODashboard 插件开发手册 · plugin.protocol/v2

> 2026-09-08。当前是**可信来源、同进程 WebView** 插件平台。CLI JSON schema 为 6，数据库 schema 为 V8；三个版本分别管理不同契约。

## 先了解怎样使用

插件是嵌入面板的 JS 单文件 ESM，入口导出 `onload(api)` 和可选 `onunload()`。宿主提供 React、Core 用例、插件 KV、事件、cron、卡片、视图、命令和设置。所有领域业务仍在 Rust Core。

```bash
dashboard plugin new com.example.hello --template ts
# 在命令返回的插件目录执行：
npm install
npm run build
npm test
dashboard plugin dev com.example.hello --run
# 打开插件页，审阅来源、权限和内容后启用
```

- `plugin new` 默认生成 JS；`--template ts` 生成 TS 和 esbuild 配置。两种模板均包含 README、MIT LICENSE、AGENTS、构建配置和 smoke test。
- 模板内 `vendor/plugin-sdk`、`vendor/plugin-test` 是随 CLI 附带的本地依赖，不依赖尚未发布的 npm 包。
- `plugin dev` 默认只校验磁盘内容；`--run` 执行当前可信插件的 `npm run build` 与 `npm test`。JSON 的 `build_and_tests_run` 明确标记是否执行。
- JS/TS 都须最终生成根目录 `main.js`。React 从 `api.react` 取得；不要打包另一个 React 或 Tauri API。
- 生命周期依赖桌面应用进程；退出应用后插件和 cron 停止。CLI 不提供无头运行插件业务。

## 安装、升级、回滚与逃生

桌面入口：进入「插件」页，点击右上角「导入插件」（空列表也有入口），选择 ZIP 或文件夹。预检展示插件名称、ID、版本与请求权限；同 ID 插件显示「当前版本 → 导入版本」，同版本重新导入或降级也需要确认替换。确认导入后，选择「完成」保留停用，或「审阅并启用」继续授权。

文件夹应为包含 `manifest.json` 和入口文件的插件根目录，目录名与 manifest.id 一致。ZIP 须包含这个单一根目录，可用以下 pack 命令生成。导入失败会保留来源和具体错误，点击「重新检查」后再确认；文件选择器取消或关闭预检弹窗不会安装插件。

```bash
dashboard plugin pack com.example.hello --output ./hello.zip
dashboard plugin install ./hello.zip --sha256 <发布者提供的ZIP摘要>
dashboard plugin install /absolute/path/com.example.hello
dashboard plugin enable com.example.hello
dashboard plugin disable com.example.hello
dashboard plugin rollback com.example.hello
dashboard plugin safe-mode
```

- GUI 与 CLI 共用 Core 安装用例；回滚继续使用 CLI。桌面系统选择器采用 [Tauri 官方 Dialog](https://v2.tauri.app/plugin/dialog/) 的 Rust 接口，前端通过宿主命令调用。
- 安装、升级、回滚后均停用；确认权限后再启用。`plugin enable` 是 CLI 使用者对当前本地内容的显式授权。
- GUI 确认绑定当时显示的内容摘要；确认期间文件变化则拒绝，需重新审阅。
- 导入预检只读取来源和已安装状态；提交时在安装锁内复核来源内容 / ZIP 摘要及目标插件状态。并发安装、启停或文件修改使确认过期时，必须重新检查。GUI 安装 activity 记录为 `user`，CLI 安装仍记录为 `cli`；桌面新增命令不改变 CLI schema 6、插件 v2 或数据库 V8。
- 管理页约每 2 秒发现 CLI 的启停和版本变化并重载；每次 Rust 插件调用即时核验修订号，旧 token 不会在停用后再启用时复活。
- 安装保留上一个版本及来源元数据，回滚后仍保留 KV。**文件回滚不回滚插件私有数据格式**：插件作者需保持自己的 KV 数据向后兼容。
- 若插件同步死循环卡住窗口，使用 `plugin safe-mode` 后退出并重启应用；UI 看门狗不能中断同步 JS。
- 默认目录为 `~/Library/Application Support/AIODashboard/plugins/`；`DASHBOARD_PLUGINS_DIR` 可覆盖。测试时同时设置 DB 与快照环境变量。

## Manifest 与版本迁移

```json
{
  "id": "com.example.hello",
  "name": "示例插件",
  "version": "0.1.0",
  "api_version": "plugin.protocol/v2",
  "min_host_version": "0.1.0",
  "entry": "main.js",
  "author": "作者",
  "license": "MIT",
  "description": "一个示例",
  "permissions": {
    "core": ["task.read"],
    "ui": ["command", "settings"],
    "storage_quota_bytes": 1048576,
    "network": ["api.github.com"],
    "events": ["task.created"],
    "cron": ["*/5 * * * *"]
  },
  "contributions": {
    "commands": [{"id": "hello", "title": "问候"}],
    "settings": [{"id": "preferences", "title": "偏好"}]
  }
}
```

目录名必须等于 id。入口为根目录单个 `.js` 文件；manifest 上限 64KB。version/min_host_version 使用 `x.y.z` 稳定版本，不接受预发布或 build metadata；宿主不足最低版本时拒绝加载。未知字段忽略。

**从 v1 迁移：**首次打开 V8 数据库会停用已有插件并保留全部 KV。v1/缺少 api_version 的清单仍可在列表诊断，但不能启用；需改为 v2，并逐项声明实际能力、UI 贡献点和 KV 配额。没有旧版宽权限回退。建议先备份数据库与插件目录再升级宿主。

## API 与权限

| API | 必要能力与行为 |
|---|---|
| `core.today()` | `context.read`；返回 Today 聚合上下文 |
| `core.listTasks(scope?)` | `task.read`；all/active/archived |
| `core.createTask(title,target?)` | `task.write`；默认每日任务 |
| `core.checkinTask(id,operationId?)` | `task.write`；提供稳定 operationId 可安全重试 |
| `core.archiveTask(id)` / `deleteTask(id)` | `task.write` |
| `core.search(query)` | `search.read`；全局搜索本身授予跨实体读取 |
| `core.createNote(title,body)` | `note.write` |
| `core.addInboxItem(content)` | `inbox.write` |
| `storage.kv.get/set/delete/list` | 必须声明 `storage_quota_bytes`；键与值 UTF-8 总字节数计入配额 |
| `fetch(url)` | 精确网络 host 白名单；仅 GET；返回 `{status,text,json}` |
| `events.on(topic,handler)` | 领域订阅须声明；返回 disposer |
| `events.emit(topic,payload)` | 仅可发射 `plugin.<完整id>:<主题>` |
| `registerCron(expr,handler)` | 表达式须在 cron 声明中，返回 disposer |
| `registerInterval(handler,ms)` | 宿主托管定时器，最小 100ms，停用自动清理 |
| `ui.registerTodayCard/registerView/registerCommand` | 同时满足 ui 能力和 contributions 中的 id；返回 disposer |
| `ui.registerSettings(definition)` | `ui.settings`、settings 贡献 id、KV 配额；返回 disposer |
| `settings.get(id)` / `settings.set(id,values)` | 对已注册设置读写；按字段类型与范围校验 |
| `log.info/warn/error(message)` | 带插件前缀的调试输出 |

`note.read`、`inbox.read` 为能力保留名称，目前没有独立 listNotes/listInbox 插件方法。所有未声明的实际调用默认拒绝，错误包含 `code=permission_denied`，并记录 `plugin.denied`。正常领域写入由 Core 归因为 `plugin:<id>`。

存储配额可为 1–10,000,000 字节；单值最多 1,000,000 字节，key 为 1–200 UTF-8 字节且无控制字符。配额检查与写入共用 SQLite 立即事务，覆盖并发竞争；覆盖键先扣除旧值。KV 不作为高频 activity log 写入，领域变更、权限拒绝、网络与安装操作可审计。

## 事件和调度

领域事件：`task.created`、`task.completed`、`note.created`、`inbox.added`。写入成功后由桌面 API 层发出，至少含对应 id 和 title/content；插件不得伪造领域事件。当前没有 CLI activity log → 插件领域事件的投影，CLI 写入仍会刷新面板数据。

`panel.refresh/show/hide` 是宿主公开通知，无需事件权限。自定义事件使用冒号隔开完整插件 id，避免 `com.a` 与 `com.a.child` 的前缀混淆；不能监听其他插件的自定义主题。

Cron 为五段表达式，最多 16 条，由 Rust 调度后投递到 WebView；时区为 UTC。它不是持久任务队列：应用退出期间不补发，同一回调耗时很长时作者应自行防重入。面板隐藏时可以继续收到投递，但系统休眠/挂起仍会影响执行时间。

## UI、设置与生命周期

组件从 `api.react` 使用 React，卡片参数为 `{api,onChanged,today}`，视图参数为 `{api,onChanged,onNav,refreshKey,today}`。只有 `context.read` 插件收到非空 today，避免 UI props 绕过读取权限。

```js
api.ui.registerSettings({
  id: "preferences", title: "偏好",
  fields: [{key:"enabled",label:"启用提示",type:"boolean",default:true}]
});
const values = await api.settings.get("preferences");
```

设置支持 text、number（min/max）、boolean、select（options），最多 30 字段；宿主渲染，持久化在 `@settings:<id>` KV。

使用 `bg-surface/text-ink/text-ink2/border-line/text-accent` 等语义主题类；卡片 size 为 sm/md/lg。样式类须已在宿主样式表中，不支持运行时任意 Tailwind 编译。

加载和卸载各有 8 秒异步看门狗，失败、取消与停用都会清理经 SDK 注册的事件、cron、定时器和贡献点。迟到的异步回调不能继续调用已关闭 API。插件自行创建的全局监听器、原生定时器和模块顶层副作用仍需自行清理；建议统一使用 SDK 的 disposer 与 `registerInterval`。

## 网络和包完整性

HTTP 使用标准 URL 解析，不允许 URL 凭据。DNS 最多等待 3 秒，结果逐个检查，并固定连接地址以防止第二次 DNS 解析改变目标；拒绝回环、私网、链路本地、未指定和特殊地址（含 IPv4-mapped IPv6）。HTTP 总超时 15 秒，连接超时 5 秒，响应最多 2MB（包括解压后的文本），不自动跟随重定向，不使用环境代理。成功、失败都记录网络审计；日志只记 host，不记录可能含密钥的 query。

ZIP 本体最多 16MB、展开后最多 32MB、单文件最多 8MB、文件/目录最多 1024、目录深度最多 16。拒绝绝对路径、`..`、反斜杠、重复/大小写冲突路径、符号链接、硬链接及特殊文件。目录导入执行同样的普通文件检查。`.git` 与 `node_modules` 不打包，不作为运行载荷。

安装先校验并复制到 staging，在跨进程文件锁下替换；恢复日志与 SQLite install operation 标志配合，遇到未提交的替换会恢复旧目录。备份按精确 id 存放。

ZIP SHA-256 表示输入包；内容 SHA-256 表示实际安装文件集合。加载与每次插件请求都会重新校验内容及授权摘要。摘要证明字节一致，**不证明发布者身份**；当前没有签名、市场或自动更新。

## 测试与交付

仓库执行 `bash scripts/check.sh full`，包含 Rust、CLI、Tauri 单测、SDK 构建、测试桩与三个示例、Vitest 和前端生产构建。plugin-test 共用宿主 SDK 权限与生命周期代码，支持注入网络响应和 Core fixture；它不替代真实 Tauri/CSP/托盘验收。

交付前检查：构建、权限拒绝、审计、双主题、重载、停用、CLI 联动、KV 隔离和配额、损坏包、摘要不符、失败恢复、回滚。真实窗口验收必须使用独立 DB/插件目录/快照。直接以三项 DASHBOARD_* 环境变量运行 bundle 内可执行文件，并核对进程环境和空任务数；不要假设 LaunchServices/工具重新启动应用会保留环境。

Trusted WebView 与宿主共享 DOM/JS。token 和权限只能约束公开 API 的身份及行为，不能阻止恶意同进程代码劫持宿主。Worker/WASM/独立进程隔离属于后续独立 Spec。
