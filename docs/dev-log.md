# 开发日志（dev-log）

> 每次对话结束后用 3 分钟记录：做了什么、关键决策、验证结果。
> 模板见文末。按时间倒序排列（最新在上）。

---

## [2026-09-01] 首页面板改造：年度热力图 + 今日双列填充卡 + 主库改名「重要日」

**需求简述**：首页加任务热力图；今日任务列表太长改为双列卡片；主库上首页并改名「重要日」（用户拍板：主库与项目**保留两个实体**——项目=主题分类可挂笔记，重要日=时间战役带日期锚点/归属历史/综合热力图）。

**模式**：Plan（`docs/plans/2026-09-01-home-heatmap-today-cards.md`）

**关键决策**：
- **全局热力图与主库综合热力图同一实现**：`overview_service` 抽出 `TaskDayFacts`（预取）/`task_day_contrib`（单日贡献）/`aggregate_days`（逐日聚合）三件套，主库口径=按当日真实归属过滤，全局口径=全量任务；聚合规则不变（单任务 min(actual/target,1)，日均=贡献和÷有效任务数）。归档任务历史保留（活动区间关闭只影响归档日起）。
- **CLI 不动**：全局热力图只加 Tauri command，避免 AI 协议变更升 schema_version。
- **渲染共享**：`RateHeatmapGrid` 抽进 `Heatmap.tsx`，主库详情与首页共用；首页用 `var(--accent)` 单色（聚合无任务色），点击跳任务页。热力图刷新挂在「打卡信号」（今日次数之和）上，不跟 4s 轮询空转。
- **今日任务卡**：双列网格、圆角矩形、主题色从左按比例填充（完成整卡加深 80%）、末尾圆形勾选框（未完成空心/完成打钩）；点卡 +1、完成态点圈=撤销、hover 浮现 −、标题进详情；三组分组取消，位置快照保留 3s 冻结、重建时未完成在前已完成沉底、新任务追加末尾。
- **改名只动人类可见文案**：UI/CLI help/README/architecture 中 主库→重要日；协议字段（library_id 等）、openspec 历史变更包、dev-log 历史条目不回改。

**变更文件**：core（overview_service 重构+全局函数）、Tauri（+1 command）、前端（types/api/Heatmap/TodayView/新 TodayTaskCard/重要日卡/LibrariesView/LibraryDetailView/TaskEditor 文案）、CLI help 与测试文案、README/architecture、checkin.rs +2 链路测试。

**验证结果**：✅ check.sh 全绿（fmt/clippy/全测/前端 build）；GUI 走查待用户（热力图 hover、填充比例与勾选态、双主题可读性）。

**走查修复**（用户第一轮反馈）：
- 任务卡周/月视图格子无法紧凑填满卡宽 → `HeatmapCell` 新增 fluid 模式（宽 100% + aspect-square + flex 均分），周/月格子按卡宽自适应。
- 年卡/首页热力图最右端应是今天 → 两个热力图组件统一**不渲染未来日**（YTD：1/1→今天，右端即今天），并首屏滚动定位到最右列；纯前端过滤，后端/CLI 年口径不变（不触协议）。
- 热力格内符号标记（圆点/横杠/✓ 等第二通道）全部移除 → 只保留纯色填充 + 虚线不适用框 + 今天粗框；`stateMarker` 删除。
- 首页热力图与今日任务大框对齐 → 热力图卡 col-12→col-8，右栏改 row-span-2 跨两行；grid 自动落位要求右栏 DOM 先于今日任务。
- 今日任务完成后立即沉底 → 改为**会话级位置快照**：会话内绝不重排，完成卡留在原位；视图重进/页面刷新重建快照时才完成沉底（3s 冻结窗口机制废弃）。

**遗留**：今日任务卡数量无上限（任务极多时再考虑收起）；热力图无按日详情页（点击暂跳任务页）。

**走查修复**（用户第二轮反馈，含协议变更）：
- **年热力图窗口改为滚动 53 周（协议 v3→v4）**：`task overview --period year`、重要日/全局综合热力图的年窗口从日历年改为 `[本周一−52周, 本周日]`（恒 371 天、起点周一、`leading_empty_count` 恒 0、`week_index` 以窗口起点为第 0 列）。新 `logical_day::rolling_year_range`，删除 `year_week_index`/`year_leading_empty`。**迁移说明**：CLI 消费者若需日历年口径，按返回的 `start_day`/`end_day` 自行裁剪；字段名与信封结构不变。建档 openspec/changes/005。
- **热力格边框规则**：虚线 = 未来日 / 上一年度（`HeatmapCell.prevYear` + `isPrevYear`）；实线 = 本年已过；粗框 = 今天。不适用日在本年范围内不再虚线。滚动窗口恒 53 列填满卡片。（视觉细节已被第三轮修订取代，见下）
- **快速捕捉建任务默认一次性**（`recurrence=once`，完成即归档，不污染长期打卡统计）；今日任务卡标题点击进任务页、头部加 ＋ 新建按钮（复用 TaskEditor 弹层）。

**走查修复**（用户第三轮反馈）：
- 热力格**取消全部边框**（含今天粗框——今天由位置表达：恒在最右列）；未来日/上一年度改为填充色减淡（×0.45），虚线方案废弃。
- 年热力图取消横向滚动：ResizeObserver 实测容器宽度反推格边长，格子恰好填满（YearHeatmap 移除 cellSize 入参；RateHeatmapGrid 同理）。005 spec/design 已同步修订。
- 走查四轮：格子太小 → 改**自适应列数**：宽度足够显示全部 53 周，不足时只显示最近 N 周（格宽下限 14px，右端恒为本周），仍恰好填满不滚动。宽容器（首页/详情页）接近全年，窄卡片（任务年卡）显示近半年左右。

**同日新增｜任务页日期翻页**（Plan：`docs/plans/2026-09-01-task-wall-day-paging.md`）：任务墙可按日翻看——`DayNavigator`（◀ ▶ 逐日 + 日期弹层跳任意日 + 「回到今天」）；core 新增 `task_day_view_on`/`wall_task_views_on`（过去日未达标自动 missed、未来日 pending，均按真实今天判定）；任务卡周/月/年热力锚定所选日；**只有今天可打卡**，翻页时控件只读（补历史卡留作后续 Spec）；CLI 不动、无协议变更。

---

**需求简述**：任务可指定循环类型——每天、每周（可选星期几）、每月、每年、或一次性；一次性未完成自动顺延、完成即自动归档；每月/每年错位日（31 号遇小月、2/29 遇平年）clamp 至当月最后一天（三项均为用户拍板）。

**模式**：Spec（变更包 openspec/changes/004-task-recurrence，四件套）

**关键决策**（详见 design.md）：
- **循环与卡片样式正交**：样式管统计展示口径，循环管哪天适用；规则存于目标区间行（`task_target_periods.recurrence`，SCHEMA_V4 加列），锚点=区间起始日，改规则=明天起新区间，历史口径不回写（与改目标同一机制）。
- **once = 每日适用 + 完成即自动归档**：不引入"到期日"字段（顺延语义下无信息量）；checkin/complete 达标当次操作内归档，视图按归档前状态返回。
- **适用性统一不变量**：活动区间 ∧ 目标区间 ∧ `Recurrence::matches`；非适用日不进 Today、打卡被拒（"今天不适用"）、热力图记 not_applicable。
- **任务墙与 Today 数据源分离**：新 `wall_task_views`（全部 active 含不适用）供 TasksView，否则每周任务非适用日会整卡消失；Today 仍只看当日适用。
- **V4 迁移单事务**：加列与 user_version=4 同 batch，堵住 003 的崩溃重跑窗口。
- **协议 v3**：Task JSON 增 recurrence（core 读侧回填），CLI `--recurrence/--weekdays`（weekly 必须显式给非空星期集）。

**变更文件**：六层全触——domain（Recurrence+matches+7 单测）、storage（SCHEMA_V4+period_repo）、core（checkin 适用性+自动归档 / task_service 输入与回填 / context wall+missed / overview 两处逐日判定）、protocol v3、CLI、Tauri（params+task_wall_views）、前端（types/api/TaskEditor 循环选择器/TaskCard 不适用态/TasksView 数据源）。

**验证结果**：✅ check.sh 全绿（Rust 全测 + vitest 22 + 前端 build）；GUI 走查待用户（新建每周任务看非适用日行为、一次性完成自动归档）。

**遗留**：插件 bridge `createTask` 暂不暴露循环参数（PLUGIN_API 已注记）；旧 dev-log 提到的"归档一次性任务被 undo 后保持归档态"为有意取舍。

---

## [2026-08-31] 全量 bug 排查批修：账本错账 / Tauri 参数契约 / 插件事件 / 快照缺口

**需求简述**：用户要求排查现有功能 bug。三路并行审查（core / storage / 前端+Tauri）+ CLI 实测确认问题清单后批修。

**模式**：Plan（`docs/plans/2026-08-31-bugfix-batch.md`）

**关键决策**：
- **undo 跨日语义**：补偿记录记入**被撤记录所在 logical_day**（而非今天）——允许跨日撤销且正确改写历史；配合 `day_count`/`counts_between` 的按日聚合口径。
- **reopen 逐条补偿**：不再写无指向的批量负记录（它会让"未补偿"判定失守、账面可被挖负），改为对当日每条未补偿正向记录各写一条带指向的补偿（新增 repo 查询 `uncompensated_positives_on`）。
- **complete_today 用未钳位 `day_sum`**：从真实账面差额算补足量，历史遗留的无指向负记录（旧版 reopen 写法）可被自愈补满。
- **Tauri 嵌套参数加 `rename_all = "camelCase"`**：Tauri 只自动转换命令顶层参数名，嵌套 struct 走纯 serde——此前 GUI 传的 cardStyle/projectId/libraryId 全被静默丢弃。"移出项目"用显式 `clearProject` 标志表达（JSON null 无法表达 `Some(None)`）。
- **插件领域事件接线**：EventBus 改模块单例，App 宿主与 api.ts 共用；写入成功后发射 `task.created/task.completed(达标时)/inbox.added/note.created`，插件经 bridge 写入同样触发；PLUGIN_API.md 补 payload 契约。
- **插件重载先清 owner**：loadPlugin 在 onload 前 unregisterOwner/offOwner——StrictMode 双挂载或重新启用时的二次注册不再因 id 冲突抛错触发自动停用；跨 owner 冲突仍抛错（registry 语义不变）。
- **TaskEditor dayLoaded 守卫**：今日视图查不到该任务（归档/今天不适用/加载竞态）时禁用目标与主库字段并跳过写回，杜绝默认值(1/空)静默重置。

**变更文件**：core（checkin_service 三函数 + project_service/inbox_service 补 snapshot + context_service missed 过滤）、storage（completion_repo +day_sum/+uncompensated_positives_on、project_repo open_task_count 修正）、src-tauri（两个 Params struct）、前端（api.ts 契约+事件、TaskEditor/TodayView/TaskDetailView、events/loader/App）、测试（checkin.rs +3 回归）、文档（PLUGIN_API.md、两份 plan、本 log）。

**验证结果**：✅ check.sh 全绿（Rust 全测 + vitest 22 + 前端 tsc/build）；GUI 真机走查待用户（新建任务带样式/主库、pomodoro 联动、归档项目后 widget 焦点）。

**走查修复**（用户第一轮反馈，前端热更验证）：
- 任务卡右键菜单被下方卡片遮挡 → 菜单打开时整卡提升 `z-30`（卡片互为兄弟节点，菜单自身 z-index 压不过 DOM 靠后的卡片）。
- 任务完成后卡片立即跳到「已完成」组跳来跳去 → TodayView 引入分组快照：打卡操作后 3s 冻结窗口内不重排（数据照常实时刷新、新任务即时插入），之后的轮询刷新再按最新状态归组；重进视图即重建快照。
- 任务卡片墙同行被最高卡拉伸 → 双视图（Plan：`2026-08-31-task-wall-modes.md`）：默认「均衡混排」按估算高度贪心发进最矮列（前缀稳定不跳动），另有「类型分区」视图，页头切换 + localStorage 记忆。
- 走查二轮：①墙序与打卡状态解耦（按 created_at 升序，新任务固定在最末、落在较矮列底部，完成打卡不再引起卡片换位）；②高度档位校准（年卡实测约 670px，原估 560 偏低导致配列失衡）。
- 走查三轮：①发牌档位按实际 DOM 精确校准（76/118/150/670——偏高的估算会让小卡多的列被误判为更高，新卡发错列）；②任务编辑器补「删除」按钮（danger 变体 + confirmDialog 二次确认，走 delete_task 级联删账本/区间），Button 组件新增 danger variant。
- 走查四轮：估算档位仍不准（副标题换行/月卡行数等动态因素），发牌改为**实测高度驱动**——MeasuredCard 包裹每张卡，useLayoutEffect 首帧同步上报 offsetHeight + ResizeObserver 跟踪变化，发牌用实测值（估算仅作首帧兜底）。卡片高度与所在列无关，重排不引起高度变化，收敛无循环。

**遗留**：StatCard 环比（Spec 级，TodayStats 需加字段）；create_task/checkin 事务化；V3 迁移版本号与建表同事务；core/cli 内 SQL 越界（红线 1）——均见 plan 文档结果记录。

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
