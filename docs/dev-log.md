# 开发日志（dev-log）

> 每次对话结束后用 3 分钟记录：做了什么、关键决策、验证结果。
> 模板见文末。按时间倒序排列（最新在上）。

---

## [2026-08-30] 插件系统 P3（plugin-system/v1 切片 8–9）——P1~P3 全部完成

**需求简述**：cron 全链路（Rust 驱动）、首次发现权限确认 UI、加载看门狗、插件重载、`plugin new/dev` 脚手架、PLUGIN_API.md 开发者文档。

**模式**：Spec（延续既有 spec）

**关键决策**：
- **cron 精度归 Rust**（croner 解析 5 段表达式）：`CronScheduler` 托管 state，启动与插件启停后 rescan；到点 emit `plugin-cron` 事件 → 前端 CronRegistry 按插件+表达式路由 → handler。循环用 500ms 切片睡眠以响应重排，锁毒化用 into_inner 恢复（库代码零 unwrap/expect）。
- **"安装时刻"语义落地**：首次发现的插件不再自动启用，弹权限确认（网络/事件/定时逐项展示）；「暂不启用」登记为停用不再询问——修正了 P1 的自动启用行为。
- **看门狗**：onload/onunload 超 8s 或抛错 → 插件自动停用；任何加载失败也自动停用，可一键再启用。
- `plugin new` 脚手架改为**零工具链纯 JS 模板**（偏离 spec 里"esbuild 模板"：与示例插件一致、AI 生成即可运行；TS 用户可自行预编译为单文件 main.js，已写进文档）。
- 热更新入口 = 插件页「重载」按钮（dispose 全部 → 重新 loadAllPlugins），不经 daemon。
- 教训：用 sed 改多行原始字符串定界符吃掉了模板开头的 `{`，集成测试当场抓住（"脚手架生成的 manifest 不是合法 JSON"）——再次证明链路测试的价值。

**变更文件**：
- `apps/desktop/src-tauri/` — plugin_cron.rs（CronScheduler + 单测）、plugin_set_enabled 增 AppHandle 重排、croner 依赖
- `apps/desktop/src/plugins/` — crons.ts、桥增 registerCron（manifest 权限校验）、loader 增看门狗/待确认队列、宿主六模块配套
- `apps/desktop/src/` — PluginApprovalModal、App cron 监听 + reload 事件
- `apps/cli/` — plugin new / dev 子命令 + 脚手架模板 + 集成测试
- `docs/PLUGIN_API.md` — 开发者完整参考（心智模型/manifest/API 表/推荐模式/工作流/安全）

**验证结果**：
- ✅ `bash scripts/check.sh` 六步全绿（Rust 25 测试 + vitest 21 测试 + 集成 11 条）
- ✅ CLI 实测：`plugin new` 生成可用脚手架 → `plugin dev` 校验通过（贡献点统计正确）→ 重复创建 exit 5
- ⚠️ 待人工 GUI 验证：cron 触发、新插件确认弹窗、看门狗、重载按钮

**里程碑**：plugin-system/v1 的 P1~P3 全部落地——第三方开发者（含 AI Agent）已可按 `docs/PLUGIN_API.md` 自助开发插件：`plugin new` → 编辑 → `plugin dev` → 面板确认启用。**下一步**：真实插件实践打磨 API（如日历集成），评估 Plugin Registry / 市场、跨设备同步（Phase 6）。

---

## [2026-08-30] 插件系统 P2（plugin-system/v1 切片 5–7）

**需求简述**：⌘K 插件命令接线、panel 事件、托盘常驻、插件管理页、番茄钟插件。

**模式**：Spec（延续 P1 的 spec 与测试清单）

**关键决策**：
- 托盘语义 = 关窗即隐藏（`CloseRequested` → `prevent_close` + hide），面板后台留存时插件随 webview 继续运行；托盘菜单/单击恢复窗口。
- panel.refresh/show/hide 三事件经 EventBus 派发（visibilitychange）；领域事件订阅仍需 manifest 声明，panel.* 豁免。
- ⌘K 面板：空查询展示全部插件命令（可发现性），输入按标题过滤；命令执行后统一 bump 刷新。
- 番茄钟的状态权威是 kv 里的 `ends_at` 时间戳而非组件内存——重启/托盘后台不走时；到点判定用「ends_at 值变化」去重防重复计数。这是给插件开发者的推荐模式，写进了插件 AGENTS.md。
- 插件管理页本身经 registry 注册（owner="core"），继续 dogfooding。

**变更文件**：
- `apps/desktop/src-tauri/` — tray-icon feature、关窗隐藏、托盘菜单（Manager trait 导入）
- `apps/desktop/src/` — SearchPalette 命令区、App panel 事件、PluginsView 新视图
- `examples/plugins/com.leeyl.pomodoro/` — 完整能力示范插件（含 AGENTS.md 模式说明）

**验证结果**：
- ✅ `bash scripts/check.sh` 六步全绿
- ✅ CLI：pomodoro 发现→启用→审计齐全；echo/pomodoro 均已装入数据目录并启用
- ⚠️ 待人工 GUI 验证（T13/T14）：番茄钟卡片与视图、完成任务自动开始、关窗进托盘后台倒计时正确、⌘K 命令

**下一步**：P3——`plugin new/dev` 脚手架 + PLUGIN_API.md、cron 注册（Rust 驱动）、安装权限确认 UI、插件加载看门狗。

---

## [2026-08-30] 插件系统 P1（plugin-system/v1 切片 1–4）

**需求简述**：按已确认的 spec 实施 P1——存储/领域基础、manifest 校验、CLI 与 Tauri 命令层、前端插件宿主与 echo 示例插件。

**模式**：Spec（TDD：测试清单先确认，实现随后）

**关键决策**：
- `Actor` 新增 `Plugin(String)`（去 Copy），serde 手工实现保持 JSON 字符串形态 `plugin:<id>`。
- SCHEMA_V2 除两新表外**重建 activity_log 放开 actor CHECK**（测试抓出：旧 CHECK 拒绝插件审计），旧数据原样迁移，符合"只增不改"。
- 插件审计策略：注册/启停进 activity log；`plugin_kv` 是数据面不审计（否则计时插件刷屏）。
- Tauri 写命令增可选 `actor` 参数：普通前端不传（=user），插件桥传 `plugin:<id>`。
- 前端宿主依赖注入（coreApi/registry/events 可替换），vitest 直测桥权限逻辑；loader 为平台粘合层不进单测。
- 核心五视图改走 ModuleRegistry（owner="core"），与插件同路径 dogfooding。
- 教训：`check.sh | tail` 管道吞退出码导致一次"假绿"提交（App.tsx 变量遮蔽 TS 错误），已 amend 修复；门禁判定必须看退出码。

**变更文件**：
- `crates/dashboard-{domain,storage,core}/` — Actor/PluginRegistration、SCHEMA_V2、plugin_repo、plugin_manifest、plugin_service
- `apps/cli/` — plugin list/enable/disable + 4 条集成测试
- `apps/desktop/` — src/plugins/*（宿主六模块）、App/Sidebar/TodayView 模块化改造、命令层 8 个 plugin_* command、17 个 vitest
- `scripts/check.sh` + `.github/workflows/ci.yml` — 六步门禁（新增 vitest）
- `examples/plugins/com.leeyl.echo/` — 最小示例（含面向 AI 的 AGENTS.md）

**验证结果**：
- ✅ `bash scripts/check.sh` 六步全绿（Rust 22 测试 + vitest 17 测试）
- ⚠️ 待人工 GUI 验证（T12/T14）：echo 卡片显示、`plugin disable` 后消失、CLI enable/disable 与面板联动

**下一步**：P2——插件视图页签/⌘K 命令接线、托盘常驻、插件管理页、pomodoro 插件。

---

## [2026-08-30] 接入 GitHub 远端

**需求简述**：创建与项目同名的私人 GitHub 仓库并推送现有提交。

**模式**：Vibe

**关键决策**：
- 仓库：`SuperWheel/AIODashboard`（private，默认分支 main），`gh repo create --source=. --remote=origin --push` 一步完成。
- 推送前用 filter-branch 把三笔提交的作者从占位 `leeyl@local` 重写为 GitHub noreply 身份 `67827727+SuperWheel@users.noreply.github.com`，使提交正确归属账号（哈希因此变为 c66c58c/731651e/ce1a28f）。
- token 已含 `workflow` scope，`.github/workflows/ci.yml` 可直接推送（推送即触发 CI）。

**变更文件**：
- 远端仓库 + `origin` remote 配置 + 本地 git user.email
- `docs/dev-log.md` — 本条记录

**验证结果**：
- ✅ 远端 visibility=PRIVATE，default branch=main，HEAD 与本地一致
- ✅ ci.yml 已存在于远端；push 触发的 CI 运行结果见 Actions 页

**下一步**：进入 Phase 2（AI Interface：--request-id 幂等 / 权限策略 / JSON Schema）或 Phase 3（SwiftUI Widget）。

---

## [2026-08-30] 仓库初始化与切片提交

**需求简述**：按上次规划的下一步，`git init` 并将现有 MVP 代码按可验证切片提交。

**模式**：Vibe

**关键决策**：
- 三个初始切片：① Rust 全量（workspace manifests + 四 crate + CLI + src-tauri Interface 层）② React 前端 ③ 门禁/CI/文档。src-tauri 是 cargo workspace member，必须与根 manifest 同片，否则首个提交检出后 cargo 无法解析。
- 中间提交不单独跑 cargo（根 manifest 引用全部 member，切片检出不可独立构建），以"提交前对工作树跑门禁 + 按意图切片"为准，不伪造历史 manifest。
- 仓库本地 git 身份暂为占位 `Leeyl <leeyl@local>`，接 GitHub 前需改为真实邮箱。

**变更文件**：
- `.git/` — `git init -b main` + 本地 user.name/email
- `docs/dev-log.md` — 本条记录

**验证结果**：
- 切片 ① 提交前：cargo fmt --check / clippy -D warnings / test --workspace（8 通过）/ build 全绿
- 切片 ② 提交前：`npm run build`（tsc + vite）通过
- 切片 ③ 提交前：`bash scripts/check.sh` 全绿（完整门禁）

**下一步**：接 GitHub 远端（推送 ci.yml 需 token 带 `workflow` scope）→ 进入 Phase 2/3。

---

## [2026-08-22] 开发流程规范落地

**需求简述**：参考《AI Coding 开发规范》为项目建立可执行的开发流程（门禁、分级模式、日志）。

**模式**：Plan

**关键决策**：
- 门禁定义为一条命令 `bash scripts/check.sh`：fmt → clippy(-D warnings) → test → build → 前端 tsc+vite build；CI 与其同构，杜绝"门禁假绿"。
- clippy 直接开 `-D warnings`（当前基线 0 警告，起步即严格；若未来第三方噪音增多再渐进放宽）。
- 分级模式（Vibe/Plan/Spec）+ 架构红线 + 新实体六层 Checklist 写进 `AGENTS.md` 作为每次会话的总约束来源。
- Vue/前端专项规则不适用本项目，未采纳；保留"type-check ≠ build"原则（npm run build 同时含两者）。

**变更文件**：
- `AGENTS.md` — 新增：AI 导航地图 + 流程分级 + 总约束
- `scripts/check.sh` — 新增：机械门禁
- `.github/workflows/ci.yml` — 新增：与门禁同构的 CI（仓库 git init 后生效）
- `docs/dev-log.md` — 新增：本日志

**验证结果**：✅ `bash scripts/check.sh` 全绿（5 步全过）

**下一步**：git init 并按切片提交现有代码；之后进入 Phase 2/3。

---

## [2026-08-22] MVP 搭建（Phase 0 + Phase 1 一次完成）

**需求简述**：按《总体设计文档 V0.1》实现最小可行产品并验证 §31 四条核心链路。

**模式**：Spec

**关键决策**：
- 六层架构落位：domain(零依赖) / storage(唯一 SQL) / core(唯一业务) / protocol(JSON 信封) / cli / desktop(Tauri Interface 层)；GUI 与 CLI 平级调用同一 core。
- SQLite WAL 支持多进程并发，GUI↔CLI 同步采用"共享库文件 + 前端 4s 轮询 + focus 刷新"，MVP 不做 daemon。
- AI 协议：`--json` 统一信封 `{success,data,error,meta.schema_version="1"}`；exit code 0/1/2/3/5；`--stdin`、`--dry-run`、`DASHBOARD_ACTOR` 审计标记。
- Widget Snapshot v1：业务变更后原子写入快照文件（App Group 目录优先，兜底 DB 同目录），SwiftUI Widget 未来只读文件不碰 DB schema。
- ID 采用 `前缀_uuidv7`（时间有序）；时间 UTC RFC3339 存储，"今天"边界唯一入口 `local_today_range`。

**变更文件**：
- `crates/dashboard-{domain,storage,core,protocol}/` — 四个核心 crate
- `apps/cli/` — `dashboard` 二进制 + integration.rs 链路测试
- `apps/desktop/` — React 前端五视图 + src-tauri commands
- `docs/architecture.md` / `README.md`

**验证结果**：
✅ cargo test --workspace 全绿（2 单测 + 6 链路集成）
✅ 四条链路实测通过（GUI↔CLI 双向同步、AI JSON 协议、Snapshot 文件生成）
✅ 桌面端真实启动验证

**下一步**：Phase 3 SwiftUI Widget 或 Phase 2 AI 权限/幂等。

---

## 模板

```markdown
## [YYYY-MM-DD] 功能名称

**需求简述**：一句话描述
**模式**：Vibe | Plan | Spec
**关键决策**：
- 用了 XX 方案而非 YY，因为...
**变更文件**：
- `path` — 说明
**验证结果**：✅ check.sh 全绿 / ⚠️ 已知问题：...
**下一步**：...
```
