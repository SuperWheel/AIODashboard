# 开发日志（dev-log）

> 每次对话结束后用 3 分钟记录：做了什么、关键决策、验证结果。
> 模板见文末。按时间倒序排列（最新在上）。

---

## [2026-08-30] Spec 003：打卡式任务 + 日期主库 + 四种任务卡片（任务模块整体替换）

**需求简述**：把 PlanningDays 的任务卡片体系（日/周/月/年卡 + 打卡账本 + 日期主库）完整移植进 AIODashboard，替换 todo 式任务模块；设计语言以圆角矩形为主（+/- 同侧并排），贴合双主题 Bento。

**模式**：Spec（变更包 openspec/changes/003-task-checkin-cards，四件套）

**关键决策**（详见 design.md）：
- **append-only 打卡账本**：operation_id 唯一约束保证幂等；减少/撤销都是补偿记录；一条正向只补偿一次；撤销只针对正向打卡（不提供撤销的撤销）。
- **统计口径照搬 PlanningDays 已验证公式**：五态/六态判定、完整完成率、连续天数（不适用日打断）、今日完成率 Σmin(count,target)/Σtarget、主库逐日聚合 min(count/target,1) 求均值。
- **Task 重构**：status→active/archived；+icon/color_hex/unit/card_style；删 due_at/completed_at；旧数据 todo/doing→active、done→archived，补默认目标区间与活动区间。
- **日期主库**：纪念日/倒计时日（自然日计算）；任务归属按逻辑日区间，移库今日起生效、历史留原库；归档三选一（保留/转独立/移动）单事务。
- **协议 v2**：meta.schema_version 1→2；`task complete/reopen` 重映射为「补满今日/今日清零」并带 deprecated 提示；插件桥 setTaskStatus 移除，改 checkinTask/archiveTask（PLUGIN_API.md 三方同步）。
- **色阶用 color-mix + 任务主题色 hex**：双主题自动成立，不新增语义 token；六态带非颜色通道（虚线/粗框/点/横杠/双点/✓）。

**变更文件**：
- 后端：`crates/dashboard-domain`（Task 重构 + 5 新实体）、`dashboard-storage`（SCHEMA_V3 + completion/period/library 三 repo）、`dashboard-core`（checkin/day_state/overview/library/logical_day 五模块）
- 接口：`dashboard-protocol`（v2）、`apps/cli`（checkin/decrement/undo/overview/move + library 命令组）、`apps/desktop/src-tauri`（12 个新 command）
- 前端：taskVisual.ts、Heatmap/TaskCard/CheckinRow/TaskEditor/TaskDetailView/LibrariesView/LibraryDetailView 七新组件；TasksView 卡片墙化、TodayView 打卡式、TaskRow 删除
- 测试：core 17 项打卡链路 + integration.rs 打卡/主库/协议 v2 链路

**验证结果**：✅ check.sh 全绿（Rust 50 测试 + vitest 22 + 前端构建）；双主题真机走查待用户验收（tasks.md 保留两项未勾）。

**下一步**：真机走查验收；已知遗留（StatCard 环比、项目 snapshot 刷新 plan）不变。

---


## [2026-08-30] Spec 管理升级：采用 OpenSpec 四件套变更包

**需求简述**：用户指定以自身财务分析项目（Web_Financial_Analyse/openspec）的 OpenSpec 规范为准——今后 Spec 一律按「变更包四件套」生成，当天早些时候建立的单文件 spec 模式随即废弃。

**模式**：Plan（文档体系调整）

**关键决策**：
- **四件套结构**（`openspec/changes/<NNN-slug>/`）：proposal.md（Why / What I Want / What I Know / What I Don't Know）→ spec.md（ADDED-N 每条带要求+验收 / MODIFIED / REMOVED / 非本变更范围）→ design.md（决策 N：选择/理由/后果 + 红线核对表）→ tasks.md（分阶段勾选 + TDD 测试清单 + 实施记录）。
- **迁移而非并存**：docs/specs/ 两份单文件 spec 拆解为变更包 001-plugin-system、002-ui-redesign-bento，信息无损重组（决策记录→design、需求→spec、任务+测试→tasks），原目录删除——避免两套 spec 之家并存。
- 新增 `openspec/README.md`（规则+索引）与 `openspec/project.md`（项目速览，细节指向 README/AGENTS 不重复维护）。
- AGENTS.md「文档落位」段、关键参考、目录结构同步更新；plans/README 的升级路径改为指向变更包。
- Plan 级（docs/plans/）不受影响，维持轻量模板。

**变更文件**：
- `openspec/` — README.md、project.md、changes/001-plugin-system/、changes/002-ui-redesign-bento/（各四件套）
- 删除 `docs/specs/`；`AGENTS.md`、`docs/plans/README.md` 指针更新

**验证结果**：✅ check.sh 全绿（纯文档改动）

**下一步**：两份待确认 plan（StatCard 环比 / project snapshot 刷新）仍待拍板；下一个 Spec 级任务直接走变更包流程。

---

## [2026-08-30] 文档管理体系：specs/ 与 plans/ 分目录 + 存量规格状态同步

**需求简述**：用户指出 spec/plan 管理不规范——文档散落、状态失真（插件 spec 已全量实施但 Tasks 全部未勾选）、Plan 级任务无落点。要求 plans 归 `docs/plans/`、specs 归 `docs/specs/`，spec 文档严格按 spec 规则建立。

**模式**：Plan

**关键决策**：
- **两目录分离**：`docs/specs/`（Spec 级）与 `docs/plans/`（Plan 级）；命名统一 `YYYY-MM-DD-<slug>.md`；各自内置 `_TEMPLATE.md`（结构固定）+ `README.md`（规则 + 索引表）。
- **spec 结构固化**（对齐 AGENTS.md Spec 级规则）：Why → ADDED/MODIFIED/REMOVED → Unknown（提案阶段）→ 决策记录 → 红线核对 → 实施记录 → 已知限制 → Tasks → 测试用例清单。
- **核心纪律**：切片提交时同步勾选 Tasks（防状态失真再次发生）；完成后当天改状态「已完成」；dev-log 回链文档；Plan 影响面超界 → 升级 Spec。
- **存量同步按证据勾选，不凭印象**：逐项对照 git 提交、Rust/vitest/集成测试函数名、dev-log 真机走查记录后勾选——plugin-system/v1 状态改「已完成」（T-S1~T-S13、T1~T15 全勾，并补记 `plugin new` 脚手架偏离 esbuild 模板的事实）；ui-redesign 重构为标准 spec 结构并改「已完成」。
- **已知遗留立 plan 案**（状态=提案，待确认后实施）：①StatCard 环比昨日（core 缺昨日数据）②project create/archive/delete 未刷 snapshot（红线 4 缺口）。

**变更文件**：
- `docs/specs/` — README.md（规则+索引）、_TEMPLATE.md、plugin-system/ui-redesign 两文档迁入并规范命名
- `docs/plans/` — README.md、_TEMPLATE.md、statcard-day-delta、project-snapshot-refresh 两份提案
- `AGENTS.md` — 流程分级新增「文档落位」段 + 关键参考加两行指针 + 目录结构补 specs/plans

**验证结果**：✅ check.sh 全绿（纯文档改动，门禁确认无副作用）

**下一步**：拍板两份待确认 plan（StatCard 环比 / project snapshot 刷新），按 Plan 流程实施。

---

## [2026-08-30] 修复走查三问题②：confirm 失效 / 跳转不精确 / 项目可重命名

**需求简述**：真机再反馈——①任务页新建任务首页不显示；②首页卡片跳转到错误的 tab；③项目无法删除、无法重命名。

**模式**：Plan

**根因与修复**：
- **Tauri(WKWebView) 不支持 `window.confirm/alert`**：confirm 静默返回 false → 所有删除确认失效（③的"无法删除"）；alert 不显示 → 全部错误提示被吞。新增 `DialogHost`（Promise 化 confirmDialog + 右下角 toastError），全组件替换原生调用。这是比前几次都隐蔽的平台坑，已设为硬约束。
- **导航无参数**（②）：`onNav` 只带视图 key，任务页永远落在默认 tab「未完成」。扩展为 `onNav(v, param?)`（插件协议向后兼容），统计卡分别跳 today/done/overdue tab，最近笔记跳转并选中该条。
- ①任务页快速添加此前日期留空 = 不排期 → 永远不进 Today。改为默认今天到期（可清空），并加提示文案。
- **项目重命名走完整六层**：storage `project_repo::update` → core `update_project`（project.update 审计 + snapshot 刷新，因 current_focus 回退到首个活跃项目名）→ CLI `project update` → Tauri command → 项目卡片内联编辑 → 集成测试 `chain_project_update_roundtrip`。
- 发现既有缺口：project create/archive/delete 均未刷新 snapshot（current_focus 可能陈旧），本次只在 update_project 落实红线，其余留作后续小修复。

**验证结果**：✅ check.sh 全绿（含新增链路测试；Rust 26 + vitest 22 + 集成 12）。

---

**需求简述**：用户真机反馈——①新建任务首页不显示；②启用番茄钟后侧栏和首页都不出现；③希望首页所有卡片可交互。

**模式**：Plan（bug 修复 + 小特性）

**根因与修复**：
- **插件启停只写库不加载**（②的根因）：`PluginsView.toggle` 只调 `pluginSetEnabled`，
  运行中的面板只在挂载时 loadAllPlugins——新启用的插件永远不进注册表。修复：toggle 后
  dispatch `reload-plugins`（宿主既有 dispose-all + 重载链路），并补上 PLUGIN_API.md §6
  承诺但漏做的「重载」按钮。
- ①经 CLI 全链路验证数据层无恙（`task create --due 今天` → `context today` 即时可见），
  GUI 有 4s 轮询 + bump 即时刷新；疑似用户操作发生在番茄钟白屏打死的旧窗口里。另发现一个
  真实隐患：**中文输入法组合期按 Enter 会误提交半成品文本**，已在全部输入框加
  `isComposing` 守卫。
- ③Card 组件支持 `onClick`（hover 上浮 + 键盘 Enter/Space + role=button），StatCard 透出；
  Today 页四张统计卡跳对应视图，最近笔记逐条、活跃项目徽标可点击。

**验证结果**：✅ check.sh 全绿（vitest 22 + Rust 25 + 集成 11）。

---

**需求简述**：真机走查发现关窗进托盘后，点托盘图标无法唤出面板。

**根因**：`window.hide()` 收进托盘后，若应用整体被 macOS 隐藏（⌘H 等），仅 `window.show()`
不解除应用级隐藏；窗口若处于最小化也未恢复。

**修复**：新增 `show_panel()`（`app.show()` + `unminimize` + `show` + `set_focus`），
托盘左键 / 托盘菜单「显示面板」/ Dock 图标 `RunEvent::Reopen` 三条路径统一走它；
`.run(context)` 改为 `.build` + `app.run` 以挂 Reopen。

**验证结果**：✅ check.sh 全绿；真机关窗→点托盘/Dock 均可唤出。

---

**需求简述**：用户真机走查发现点进番茄钟视图全屏空白。

**模式**：Plan（bug 修复）

**根因**：bridge 暴露的 `api.storage` 是扁平结构，而 PLUGIN_API.md / 示例插件 / 插件 AGENTS.md
全部约定 `api.storage.kv.*`。番茄钟视图的 useEffect 同步调用 `api.storage.kv.list()` 抛
TypeError；宿主没有错误边界，React 整树卸载 → 白屏。Today 卡片未崩只是因为 kv 调用都在
async 函数里（错误沦为静默 rejection）。教训：**协议文档、测试、示例三方必须互相锁死**——
扁平结构曾被 T10 测试断言固定，文档和示例却写着 kv，两条线各自"通过"。

**修复**：
- bridge 改为 `storage.kv.{get,set,delete,list}`（文档即协议），T10 改为固定公开协议形状
- 新增 `PluginErrorBoundary` 包住插件视图与 Today 卡片：渲染/effect 抛错只降级该块
- 已安装插件目录同步了 token 化后的示例代码

**验证结果**：✅ vitest 22 通过 + 前端构建通过；真机点击番茄钟视图恢复正常（vite 热重载后）。
注：echo 插件此前因 onload 顶层调 kv 被看门狗自动停用，修复后需在插件页手动重新启用。

---

## [2026-08-30] UI 重设计：Bento 总控台 + 跟随系统双主题（切片 1–5 全部完成）

**需求简述**：按用户拍板的定稿（`docs/ui-redesign-2026-08.md`）重做面板 UI——风格 C（Bento 总控台）、布局（首页网格 + 子页单列）、跟随系统双主题、首页四项重点（今日任务为主 / 插件卡片舞台 / 快速捕捉 / 统计可视化）。

**模式**：Plan（设计定稿经用户四问确认后按 5 切片实施）

**关键决策**：
- **语义 token 而非 dark: 变体**：`@theme inline` 把 `--color-bg/surface/ink/accent…` 映射到运行时 CSS 变量，双主题 = 变量翻转（浅色 `:root` 默认 / `[data-theme="dark"]` / 系统深色媒体查询三选择器）。组件零 `dark:` 前缀，插件 UI 天然跟着换主题——这是「插件只用语义类名」规范成立的基础。
- **主题三态**（system/light/dark）存 localStorage，index.html 内联脚本首帧前还原；system 态不设属性、交给媒体查询，原生控件配色由根 `color-scheme` 统一驱动（替掉日期输入框上的 `[color-scheme:dark]` 补丁）。
- **插件协议新增 `registerTodayCard` 的 `size`**（sm=3列/md=4列/lg=6列，缺省 md）：bridge 运行时校验、registry 透传、App 包网格占位；向后兼容（可选字段），示例插件 echo/pomodoro 已声明 sm。PLUGIN_API.md 同步「UI 与双主题」一节。
- **今日任务为主卡**（span 8，逾期任务红色横带置顶分组）；QuickCapture 独立成卡（今天任务/收件箱双去向，Enter 即走）；统计降为 2×2 迷你卡；问候卡带 SVG 完成率环（ProgressRing）。
- 环比昨日箭头：StatCard 留了 delta 口径但**未接线**——`context today` 没有昨日数据，属于 core 层新增字段，后续单独切片。
- 新增依赖 lucide-react（仅 Sidebar 核心视图用；插件视图仍用 manifest 字符图标，协议不变）。
- 顺修：QuickCapture/番茄钟的"今天"改用本地时区（原 `toISOString().slice(0,10)` 跨午夜偏一天），新增 `hooks.localToday()`。

**变更文件**：
- `apps/desktop/src/styles.css` — token 体系 + 双主题 + focus-visible + reduced-motion
- `apps/desktop/src/theme.ts`、`index.html` — 主题三态 hook + 防闪烁
- `apps/desktop/src/components/` — ui.tsx 重构（Button/PageHeader/ProgressRing/Empty/Card hoverable）、QuickCapture 新组件、TodayView Bento 重写、五个子页 + Sidebar/SearchPalette/审批弹窗全量 token 化
- `apps/desktop/src/plugins/{registry,bridge}.ts` — CardSize + size 透传校验；bridge.test.ts 补 size 用例
- `examples/plugins/` — echo/pomodoro token 化 + size: "sm"
- `docs/PLUGIN_API.md` — size + 双主题规范；`docs/ui-redesign-2026-08.md` — 设计定稿

**验证结果**：
- ✅ `bash scripts/check.sh` 全绿（fmt/clippy/Rust 测试/构建 + vitest 22 通过 + 前端生产构建）
- ⚠️ 双主题人工走查（浅色下各视图/弹窗/插件卡片的实际观感）待用户在真机确认

**下一步**：真机走查双主题；`context today` 补昨日数据后接 StatCard delta；三栏式任务详情（范围外，单独立项）。

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
