# 开发日志（dev-log）

> 每次对话结束后用 3 分钟记录：做了什么、关键决策、验证结果。
> 模板见文末。按时间倒序排列（最新在上）。

---

## [2026-09-09] 番茄钟插件升级 0.2.0：周期/暂停/设置 + 圆环 UI

**分级**：Plan 级（仅示例插件，不涉及 Core/schema/CLI 协议）。按 AGENTS.md 本应先确认方案再动手；本会话补对齐后继续。

**交付**：`examples/plugins/com.leeyl.pomodoro` 升到 0.2.0。功能从固定 25 分钟单段，扩为专注/短休/长休完整周期、暂停继续、跳过重置、宿主设置（时长与长休间隔）。UI 重做：SVG 圆环、模式色（`--accent` / `--info` / `--violet`）、循环点、Today 卡片与独立视图统计。

**状态与兼容**：权威仍是 KV 时间戳；会话写入 `session` JSON（`ends_at` / 暂停 `remaining_s`）；旧 0.1.x 裸 `ends_at` 首次加载自动迁移。`cycle_position` 在专注完成时 +1，长休完成归零；跳过不推进。完成时按实际 duration 累加 `count:日:min` 分钟。到点结算用宿主 `registerInterval`，不依赖 UI 是否打开。

**命令**：⌘K 扩为 6 条（开始专注/短休/长休、暂停继续、跳过、重置）。权限增加 `settings`；`version` 0.1.0 → 0.2.0；协议仍为 `plugin.protocol/v2`。

**验证**：根目录 `npm run build && npm test` 7/7 通过（含 pomodoro load/dispose）。未改 Rust/前端；未自动覆盖本机已安装插件（需 GUI 导入或 CLI install 替换后人工走查）。

**安装提示**：`dashboard plugin install examples/plugins/com.leeyl.pomodoro` 或插件页「导入插件」选该目录；替换后需审阅权限再启用。

---

## [2026-09-08] 求职台第二次审阅：右键编辑、全高侧栏与宿主布局适配

**修订**：按用户反馈移除求职列表/事项卡片上的独立编辑操作，统一放入右键菜单，详情顶部和安排条目的更多按钮共用菜单；保留左键查看详情、时间直接修改及键盘菜单操作。右侧详情与编辑改为全高面板，头部和保存底栏固定，内容独立滚动；未保存保护保留。

**现场兼容核对**：实际打开本机 AIODashboard 的今天和任务页，并核对当前源码。真实首页为问候/捕捉/统计、年度热力、双列今日任务与重要日/笔记/项目右栏；`extraCards` 位于这些内容之后，lg 为 6/12 列。任务页是带周期热力图的双列卡片，支持混排/分区及进行中/已归档。修订稿按这些结构重做示意；正式关联应复用 `TodayTaskCard` / `TaskCard`，新增受限来源入口，而不以插件维护第二份任务清单。

**接口边界**：现有 SDK 提供 React、View 和 TodayCard 注册，但不提供宿主托管的全高面板或 ReactDOM/Portal。Spec 009 增加覆盖层的 owner、几何边界、焦点、主题及停用清理要求；来源导航、一次性任务关联、计划日期与标题/完成同步仍需 Core/CLI/SDK 共同实现。本轮仅调整 [设计四件套](../openspec/changes/009-job-tracker-plugin/design.md) 与会话预览，不安装插件、不修改宿主功能代码，也不视为用户已确认正式实施。

**验证**：浏览器实际验证右键编辑城市、更多菜单编辑轮次/时刻、Shift+F10/Escape、窄屏菜单避让、未保存放弃。1024px 与 360px 中面板上下边缘和应用区域完全对齐；从滚动后的今天卡片打开仍对齐，保存底栏可见。关联任务完成后进入已归档，今天保留当日完成态；撤销恢复且招聘阶段不变。双主题、1024/736/360px 布局和预览脚本语法通过，浏览器无错误日志。未运行正式 frontend/full 门禁，不以演示代替真实接入验收。

**工作区**：只读取实际应用数据与现有代码；测试使用预览内存中的虚构岗位/任务。既有混合工作区修改保留，未提交。

---

## [2026-09-08] 求职台审阅修订：编辑入口与今天/任务联动方案

**用户反馈与修订**：原交互稿缺少可用的笔试/面试编辑与明确的详情入口。本轮补齐整行打开详情、显式「编辑」按钮、资料表单与安排表单；笔试/面试可新增或修改类型、轮次、预计/确认/完成/取消、日期时刻和备注，并支持显式同步岗位阶段。未保存修改有放弃确认，窄窗口抽屉内部滚动且保存操作保持可见。

**联动建议（待审阅）**：在「今天」新增求职事项卡片，集中呈现确认安排、计划投递、临近截止和到期跟进；用户可将具体下一步加入一次性任务。交互稿使用同一份内存数据演示改期、取消、投递补录与任务完成的回流。任务完成仅完成该行动，不代表投递成功或面试通过。同步修订 [Spec 009](../openspec/changes/009-job-tracker-plugin/proposal.md) 的四件套，正式实施项继续保持未开始。

**宿主核对**：现有 `registerTodayCard` / `extraCards` 可承载独立卡片，当前卡片属性没有 `onNav`，需复用卡片内详情或增加受限导航契约。SDK `createTask` 默认创建每日任务；Core 虽支持 Once，仍缺少求职行动来源关联与计划日期投影。因此任务联动应作为独立跨层切片，先冻结 Core / CLI / SDK 契约，不通过插件前端监听全局完成事件模拟业务。

**验证**：预览 JavaScript 语法检查通过；浏览器实际完成资料编辑、轮次与时间修改、取消测评、确认安排缺失时间校验、显式同步阶段、未保存放弃、已投递但时间未知补录。今天卡片随源数据同步；下一步重复加入只产生 1 条任务，完成后行动关闭且岗位阶段不变。走查 1024px 浅色主表/详情、736px 深色今天页面、360px 深色安排表单，无根节点横向溢出且操作可见。本轮仅修改设计文档与内存演示，不运行或声称正式 frontend/full 门禁通过。

**交付边界**：新版预览供继续审核，保留用户旧预览中的临时改动；没有连接真实求职数据、安装插件或实施宿主联动。既有其他工作区修改保留，未提交。

---

## [2026-09-08] 求职台插件产品设计与交互稿

**需求与交付**：用户希望以美观表格管理投递公司、岗位、是否投递、时间、状态、期望薪资与面试安排。新增 [Spec 009 提案](../openspec/changes/009-job-tracker-plugin/proposal.md) 及 spec / design / tasks 审阅草案，配套可操作的会话界面稿；本轮没有实施或安装正式插件。

**产品设计**：一行对应公司 + 岗位 + 招聘批次，默认合并展示公司/岗位与投递情况/时间；右侧抽屉承载资料、备注、多轮面试和动态历史。是否投递由投递事实派生，允许已投递但时间未记；面试区分预计/已确认，薪资区分日/月/年、期望/招聘区间/Offer。补充下一步、截止/跟进日期与归档恢复。视觉沿用暖纸白/深夜主题、翡翠绿主色和轻分隔线。

**技术边界**：核对当前 SDK 和 PLUGIN_API 后确认尚无求职领域接口；推荐 Core 求职领域 + 可启停 UI 插件。KV 只用于 UI 偏好，不以无领域审计的 KV 和前端业务逻辑绕过架构。正式开发涉及追加迁移、CLI schema、插件能力/SDK 扩展，按 Spec 流程审阅后实施。现有 createTask 默认每日任务，cron 也不支持退出应用后提醒，因此首版不承诺这些联动。

**验证**：交互稿语法检查、浏览器表格/抽屉/筛选/搜索/阶段切换通过；修正预览环境的原生表单提交限制后，新增校验及「已投递但时间未知」保存实测通过（8 → 9 条，刷新回到 8 条样本）。检查 1024/736/360px 布局及双主题，无根节点横向溢出；窄屏表格在自身容器滚动。`git diff --check` 通过。本轮仅设计文档和演示，不运行或声称正式 frontend/full 门禁通过。

**状态**：设计稿完成，提案及 TDD 清单待审阅，实现项全部未开始。界面稿使用虚构数据、仅在内存中交互；既有混合工作区修改保留，未提交。

**审阅入口**：用户要求打开后，已在 Codex 浏览器中显示并保留求职台交互稿，实际核对主表与 8 条演示记录已加载；尚未收到设计审核结论。

---

## [2026-09-08] 插件统一导入入口实现与验收

**需求与交付**：用户确认 [Plan](plans/2026-09-08-plugin-import-entry.md) 后，插件页页头和空列表接入统一「导入插件」弹窗，支持原生 ZIP / 文件夹选择、只读预检、同 ID 版本替换确认、错误后重新检查、默认停用及「审阅并启用」。同版本重新导入与降级也明确按替换处理。

**实现**：Core 共用来源读取与安装事务，预检返回内容 / ZIP 摘要和目标状态凭据；提交在原有安装锁内复核，内容或目标状态变化即拒绝陈旧确认。安装日志 actor 参数化，GUI 由 Tauri 固定为 User，CLI 保持 Cli。新增 Tauri `plugin_import.rs`，通过官方 Dialog 的 Rust 接口选择来源；文件处理在阻塞任务运行，安装成功撤销旧会话并同步 cron。前端异步状态阻止重复提交与迟到结果覆盖，共用权限展示；成功刷新并接续原有启用流程。

**自动验证**：`cargo check --workspace`、`bash scripts/check.sh full` 通过（104 Rust、45 Vitest、6 SDK/mock/示例测试及所有构建）。桌面走查发现关闭图标受通用按钮 padding 挤压，改用固定宽度图标按钮后，`bash scripts/check.sh frontend` 再次通过；Rust 未再改动，复用 full 结果。`git diff --check` 通过。

**桌面验证**：显式使用 `/tmp/aiodashboard-plugin-import-h2zgj5xq` 的数据库、插件目录与 Snapshot，直接运行 bundle 二进制，避免 LaunchServices 丢失环境。实际完成 ZIP 新安装、文件夹新安装、v1 → v2 替换、权限审阅及命令加载；更新后旧命令移除、重新启用后仅有 v2 命令。预检后用 CLI 修改临时插件状态，GUI 正确拒绝旧确认，重新检查后成功；无效 ZIP、系统选择器取消与预检取消均通过。CLI 只读核对恰有 3 条 GUI 安装活动且 actor=user，版本 / 上一版 / verified / directory 状态正确，status 返回临时库路径及 0 活跃任务 / 项目。证据写入该临时目录的 `acceptance.json`。

**最终交付与范围**：调试应用包 `target/debug/bundle/macos/AIODashboard.app` 已构建并重启复核关闭按钮；验收后停用临时插件并退出实例。同步 README、PLUGIN_API、architecture、计划与 Spec 008 后续演进注记。CLI schema 6、插件 v2、数据库 V8 不变；回滚仍用 CLI。保留既有其他任务的工作区修改，未将混合工作区直接提交。

---

## [2026-09-08] 插件统一导入入口调研与计划

**需求简述**：查看现有插件系统，为用户添加统一的插件导入入口。

**现状核对**：Core / CLI 已支持 ZIP 与本地目录安装，包括清单校验、内容摘要、默认停用、备份和失败恢复；桌面插件页明确使用 CLI 安装，Tauri 与前端 API 均未接通导入。安装内部日志 actor 固定为 CLI，新增 GUI 接入时需按实际来源记录。当前 Spec 008 已标记完成，比早期记忆中的未验收状态更新。

**方案（待确认）**：插件页主按钮与空状态统一打开导入弹窗，支持 ZIP / 文件夹，预检名称、版本、权限和同 ID 替换信息，确认后复用 Core 安装，成功刷新列表并提供「审阅并启用」。实施步骤、预检与提交一致性、测试和桌面验收路径见 [Plan](plans/2026-09-08-plugin-import-entry.md)。

**验证与状态**：本轮 `cargo check --workspace` 通过；仅新增提案、计划索引与本日志，尚未修改功能代码或运行导入操作。按 AGENTS.md 的 Plan 流程，待用户审阅方案后实施，交付时执行 full 门禁及隔离库桌面验证。保留工作区已有其他任务的修改。

---

## [2026-09-05] 任务详情周/月/年视图信息架构与前端重设计

**需求简述**：用户要求在热力图格内不显示文字，并重新调研同类习惯追踪/目标软件，为周、月、年详情视图补充有用信息，提升信息层级与视觉清晰度。

**研究结论**：
- 周视图以每日柱状图为主，突出本周节奏、达标天数、实际次数/目标次数、超额日和环比变化；点击柱体后在图表下方展开该日详情。
- 月视图以无文字日历热力图为主，辅以按周完成率趋势、断点/部分完成/超额完成、工作日与周末对比和选中日详情。
- 年视图以无文字年度热力图为主，辅以按月完成率柱状图、季度对比、最佳/低谷月份、年度覆盖率和长期连续记录。
- 热力图格内不放日期、次数、X/Y 或状态词；日期、图例、提示和详情均放在网格外或交互提示中。
- Habitify 重点展示 Daily Score、Peak Focus Zones、Weekly Rhythm、Missed Habits；Loop 提供 habit score、灵活周期和长期图表；Strides 提供 pace line、rolling average、按周/月/年报告；Streaks 强调连续记录、非每日周期和任务统计。设计只吸收有助于解释数据的部分，不直接堆叠徽章或游戏化组件。

**实现边界**：当前 `PeriodOverview` 已有每日实际次数、目标次数、封顶完成率、超额标记、六态和周期分桶，新增指标均在前端派生；完成时间、备注、情绪等当前协议未提供，未加入页面。

**实现**：`TaskDetailView` 周视图改为每日目标完成率柱状图；月视图增加按周柱状趋势、工作日/周末对比、部分/超额摘要；年视图增加月度趋势和年度状态摘要。周/月/年热力图格内继续不渲染文字，点击/悬停信息移到图表外。

**验证结果**：✅ `bash scripts/check.sh frontend`（32/32 测试、TypeScript、production build 全部通过）。本次为前端展示改动，未重复 Rust 门禁；仍需用户在桌面端进行三种视图的实际观感验收。

**用户走查修订（同日）**：底部摘要由长句 chip 改为图标 + 数值 + 短标签的 `InsightStat`；选中日详情改为日期、实际/目标和进度条；月视图改为左侧日历热力图、右侧按周柱状图的双栏布局；`HeatmapCell` 改用 `outline` 并放开年度热力图容器溢出，修复边缘格选中描边被裁切。重新构建 Tauri debug app，桌面端实际核对周/月/年视图。

**修订验证**：✅ `bash scripts/check.sh frontend`（32/32、TypeScript、production build）；✅ `npx tauri build --debug --bundles app`；✅ 预览包实际走查月视图右侧趋势和月末格选中描边。

**用户走查修订②（同日）**：月视图双栏内容收窄到 `max-w-4xl` 并整体居中；周期导航将「回到本期」移到日期选择器左侧、日期控件保持最右；`DatePickerPanel` 增加 `granularity`，周选择日、月选择月、年选择年，提交值分别归一到日/月首日/年首日。

**修订验证**：✅ `bash scripts/check.sh frontend`（32/32、TypeScript、production build）；✅ `npx tauri build --debug --bundles app`。

---

## [2026-09-05] 任务卡月卡固定每行 11 格

**需求简述**：月卡热力格每行偏少，用户拍板固定每行 11 格。

**模式**：Vibe（纯前端一行逻辑改动）

**关键决策**：
- MonthHeatmapGrid 删掉「格子 <36px 逐列收窄、最少 8 列」的自适应减列逻辑——该逻辑在窄卡片上总会把 11 列目标塌缩到 8 列，是「每行格子少」的根因；改为 `const cols = 11` 固定，格宽随容器自适应（任务卡宽约 25px/格）。
- MonthHeatmapGrid 实际只有任务卡月卡在用了（详情页月视图早已切到 MonthCalendarGrid），注释从「共用」更正为「任务卡月卡专用」。

**变更文件**：
- `apps/desktop/src/components/Heatmap.tsx` — 固定 11 列 + 注释更正

**验证结果**：✅ `bash scripts/check.sh frontend` 全绿（vitest + tsc + production build）。GUI 观感待用户走查。

---

## [2026-09-03] 任务详情页重设计 + 月历热力网格统一

**需求简述**：优化任务卡片点入的详情页四项：①月视图热力图去文字、格子填满、上下月溢出日用更淡格子补位（月卡同样改日历排布）；②周/月/年与日期导航控件对齐任务页设计语言（圆角矩形分段控件、等高）；③返回键改 40×40 圆角正方形只含「←」；④详情页整体布局重排，重点信息突出。

**模式**：Plan（纯前端 UI 优化，无协议/schema 变更）

**关键决策**：
- **MonthHeatmapGrid**（Heatmap.tsx 新增共享件）：7 列按 `weekday_index` 对齐真实星期位置、格内无文字、`minmax(0,1fr)` 流体填满卡宽；首/末周体外补位格用比常规空格更淡的中性色（`--ink 2%`）补成完整矩形；ResizeObserver 实测宽度推算统一圆角。详情页月视图与任务卡月卡共用同一件。
- **月卡**从 11 列流水排改为日历排布（用户拍板学习月视图）；`CARD_HEIGHT_EST.month` 150→460（发牌估算，实测为准）。
- **工具条**：周/月/年换任务页同款分段控件（`bg-hover p-0.5` 容器 + `h-6` 内块，总高 28px）；日期导航复制 DayNavigator 规格（◀ 标签 ▶ 一体控件 + 标签弹 DatePickerPanel 跳任意日 + 非本期「回到本期」），两组件同排同高。
- **返回键**：h-10 w-10 圆角正方形（rounded-xl）与图标块同高，仅「←」。
- **布局重排**：头部身份卡（返回/图标/标题+今日进度+撤销入口/打卡/编辑）→ 工具条 → 5 张统计卡（统一 Card 底）→ 热力图卡（周=柱状图、月=日历网格、年=GitHub 式；选中日详情行固定在卡内，未选中显示提示）→ 趋势卡。删除底部「今日圆环小结」卡（与头部信息重复），撤销打卡入口并入头部副标题。
- 周视图去掉柱状图下方冗余的第二排热力格（柱体本身可点击选中）。

**验证结果**：✅ `bash scripts/check.sh frontend` 全绿（vitest 32/32 + tsc + production build）。GUI 真机走查待用户：月视图/月卡日历对齐与补位格观感、工具条同高、返回键样式、头部卡布局。

**走查修复（同日，格子上限）**：用户反馈月视图/月卡格子太大——7 列均分撑满宽容器时格子被拉到 60–90px+。MonthHeatmapGrid 加 `maxCell` 上限（月卡默认 40px、详情页 48px），容器更宽时格子封顶、网格整体居中，窄容器仍自适应缩小；`CARD_HEIGHT_EST.month` 460→370。重建 debug .app 并重启。

**走查修复②（同日，放弃日历对齐）**：用户拍板月度热力图不要求严格日历排列——改回**行优先多列流水网格**：列数按容器宽度自适应（每行尽可能多放、最少 7 列），格子边长向目标值 20px 收敛、窄容器自动缩小；删除星期表头与上下月补位格（月卡热力图内彻底无文字）。`CARD_HEIGHT_EST.month` 370→150。重建 debug .app 并重启。

**走查修复③（同日，定稿）**：20px 流水版用户嫌小，明确口径「一行 10–11 个格子就差不多，末行不属于本月的空位用更淡的格子补齐、不要空着」。定稿：每行固定目标 11 列（格子 <36px 时逐列收窄、最少 8 列），格子边长封顶 44px、过宽容器中网格居中不再放大（月卡 ~41px、详情页 44px 封顶）；末行空位渲染淡色中性补位格（`--ink 2%`）补成完整矩形。`CARD_HEIGHT_EST.month` 150→240。重建 debug .app 并重启。

**视图差异化重设计（同日）**：用户指出详情页周/月/年内容同质化（同组件换窗口），提出按用户使用目的区分设计；调研 Streaks/Habitify/GitHub 贡献图等同类产品后确认分层（周=行动、月=复盘、年=证据）。提问未作答，按推荐方案实施：
- **周 = 行动台**：本周达标进度环 + 「距完美周还差 N 天/完美周达成」+ 环比上周箭头；7 天 56px 大行动格（状态色填充、今天描边环且**点击直接打卡/撤销**、目标>1 时格内显示 X/Y），格下星期+日期标签；柱状图移除。
- **月 = 日历复盘**：新增 `MonthCalendarGrid`（真实日历排布、28px 封顶居中、淡色补位格、周末列表头 warn 色区分）；右侧分析面板 = 完成率环 + 断点/最长连续/工作日 vs 周末完成率/上月环比。
- **年 = 成就墙**：叙事统计句（最长连续 N 天、覆盖 X%、累计 M 次）+ 里程碑徽章墙（连续 7/30/100/365、累计 100/500 次、完美月 ×N，未达成灰显）+ GitHub 热力图；按月趋势桶仅年视图保留。
- **统计卡按视图定制**（4 卡）：周=本周达标/当前连续/距完美周/上周完成率；月=完成率/断点/最长连续/上月完成率；年=累计打卡/最长连续/完美月/覆盖率。环比需要上一期数据：详情页并发多请求一份 prev overview（周/月/年各按 shiftAnchor 回退一期）。
- 纯前端聚合（断点/周末率/完美月均从 PeriodOverview.days/buckets 前端计算），无协议变更。重建 debug .app 并重启。

**走查回退（同日，减法定稿）**：用户否决成就墙版（太丑 + 违反「热力图里无文字」红线——周格内 X/Y、格下星期标签、年视图叙事句与 emoji 徽章墙全数撤掉）。定稿为极简结构：主视图卡内**只有纯色热力格**（周 = 7×56px 状态格居中、今天描边即打卡入口；月 = 28px 日历网格居中；年 = GitHub 热力图），分析信息全部收进上方 4 张统计卡（周=本周达标/当前连续/距完美周/上周完成率；月=完成率/断点/周末完成率/上月完成率；年=累计打卡/最长连续/完美月/覆盖率）；侧边分析面板、进度环头部、叙事句、徽章墙删除；趋势桶仅年视图保留。重建 debug .app 并重启。

---

## [2026-09-02] 一次性任务完成当天留痕 + 历史翻页可见归档任务

**需求简述**：用户反馈一次性任务做完立刻从主界面消失（应「今天过了再退出」），且归档后日期翻页回看历史完全看不到一次性任务；undo 对已归档任务只写补偿不解除归档，等于完成当天无法撤销。确认三项一起修（Plan：docs/plans/2026-09-02-once-task-linger-history.md）。

**模式**：Plan（修订 004「完成即终态」的可见性语义；无协议字段/schema 变更，协议保持 v5）

**关键决策**：
- **保持「完成即归档」**，改的是可见性：`build_day_view_on` 适用性去掉 `status==Active` 硬条件，改由活动区间覆盖驱动；关闭日当天若达标（count≥target）仍按适用 → completed 留痕，明天起自然退出。
- **候选集**：`today_task_views` / `wall_task_views_on` 改为 `period_repo::task_ids_covering(day)`（活动区间覆盖当日、含关闭日）∩ 全量任务；已归档任务的 not_applicable 卡任何日期都不展示（手动归档=立即退出，不留 NA 卡）。
- **undo 解除当天归档**：Once + 已归档 + 撤前今日 completed + 撤后不再达标 → `task_service::unarchive_if_archived_today` 重开今天关闭的活动区间（不产生零长度区间），回到待打卡可继续打卡；跨天 undo、手动归档不受影响。
- **前端留痕卡**（status=archived）：Today 卡主体不再触发打卡（避免「任务已归档」报错 toast）、隐藏 − 按钮、圆圈撤销可用；任务墙 CheckinControls 对归档卡只显示完成环（点击=撤销），隐藏 +/-。
- 004 spec/design 追加 2026-09-02 修订注记（变更包不搬移，原地标注演进）。

**验证结果**：✅ `bash scripts/check.sh full` 全绿（core 35 测：改写 once_task_auto_archives_on_completion 为「Today 保留完成态」，新增 undo 解除当天归档、多目标撤销回 in_progress、历史翻页可见归档任务（backdate 活动/目标区间）且手动归档当天退出；CLI 链路断言不受影响；前端 32 测 + production build）。GUI 真机走查待用户：完成一次性任务后首页/任务墙留完成卡、圆圈或 ⌘Z 撤销回待打卡、任务页翻回完成日可见。

**提交注意**：TaskCard.tsx / TodayTaskCard.tsx 与 006 拖拽走查修复（memo 化 + dragging prop）的未提交 WIP 同文件纠缠，本次前端改动不单独切片，随该批入库；Rust 与文档按层单独提交。

---

## [2026-09-02] Spec 006：任务星级 + 自由拖拽排序

**需求简述**：任务卡加星级系统（1–5，0=未评级），星级高排前；卡片可自由拖拽手动排序。用户拍板：星级分档优先，跨档拖动弹确认框改成目标档星级；编辑器 + ⋯ 菜单设星。

**模式**：Spec（变更包 openspec/changes/006-task-priority-drag，四件套）

**关键决策**：
- **排序口径**：墙 = priority DESC → sort_order ASC → created_at ASC（core `sort_wall_views` 统一，含按日翻页）；与打卡状态解耦不变；新任务落档末（档内 max+1024）。
- **分数索引**：`sort_order REAL`，落位取邻居中点，不整列重排；`task_repo::create` 参数膨胀触发 clippy，顺手重构为 `NewTask` 参数包。
- **跨档拖拽**：`move_task_position(id, new_priority?, before_id?, after_id?)` 单用例改级+落位（activity log + snapshot）；GUI 在落点确定后判断目标档（下方卡片星级，墙尾取上方），同档直调、跨档先弹 confirmDialog（文案含源/目标星级）。
- **协议 v5**：Task JSON 增 priority/sort_order，CLI `create/update --priority`（stdin JSON 也支持）；拖拽位置写为 GUI 专有。
- **Today 页不受影响**（状态排序+会话快照）。

**验证结果**：✅ check.sh 全绿（core 32 测含星级/落位/跨档、CLI 16 链路含 --priority 与越界 exit code 2、迁移 V5 断言）；GUI 走查待用户（拖动手感、插入指示线、跨档弹窗）。

**走查修复**（同日）：①拖拽完全无反应——根因是 WKWebView 要求 dragstart 必须 `dataTransfer.setData()`（Chrome 不需要故漏写）；②拖拽逻辑抽为共享 `src/dnd.ts`（useTaskDnd hook），首页今日卡同步接入拖拽；③首页排序与既有偏好调和：完成卡仍沉底（会话冻结不变），未完成组内按墙口径（星级→sort_order→原序）——两处排序语义统一，今日卡与任务墙共享同一手动顺序。

**走查修复②（同日）**：拖拽能起拖但落位不生效——根因是 V5 迁移把**存量任务 sort_order 全置 0**，落位中点 (0+0)/2=0 塌缩，写入成功但顺序不变。修复：SCHEMA_V6 数据迁移，按星级档内 created_at 把存量任务回填为 1024 步进（归档任务一并编号防恢复后撞值）；storage 单测覆盖回填顺序与版本号。任务 JSON 不变，协议保持 v5。

**走查修复③（同日，交互重做）**：拖拽升级为实时预览——①拖动中其他卡片实时重排（预览序列）+ FLIP 补间让位动画（快照→translate 反演→0.18s 回放）；②被拖卡渲染为同形状淡色圆角矩形占位块（= 落点标记，用户拍板样式）；③提交点从 onDrop 挪到 onDragEnd，预览序列即落点，不再依赖 drop 事件触发（顺带消除 dropMark 状态不同步的隐患）；④首页提交成功后将新序列写回会话 pin（手动顺序在会话内保持）。任务墙与首页共用 useTaskDnd。

**走查修复④（同日，交互定稿）**：用户指出起拖后原位置不应继续保留占位。改为两阶段序列：① `dragstart` 立即移除被拖卡，原位腾空并触发首轮 FLIP 补位；②进入有效目标后才在目标位置插入等高淡色圆角占位。为避免 source 随双列重排卸载导致 WebKit 丢结束事件，真实拖拽节点移到视口外但保持在原父容器；隔离临时库 GUI 走查又抓到 React 合成 `dragend` 可漏发，追加原始 DOM 节点原生 `dragend` + document `drop` 双兜底，目标 `drop` 也会补算极快释放的落点。所有结束路径共用幂等提交，最后悬停序列同步写入 ref；任务墙与首页均先乐观保留预览顺序再刷新，释放后不再闪回旧序列。新增前端纯函数测试覆盖起拖移除、目标前后插入、重复悬停去重与无效目标。

**验证**：✅ `bash scripts/check.sh` 全绿（前端 26 测，含新增拖拽序列 4 测）；✅ `--bundles app` 调试包构建成功；✅ 使用 4 张临时卡、独立 SQLite/快照/插件目录走查，确认起拖原位立即腾空且未形成有效落点时可靠恢复，正式数据库未参与测试。持续悬停时的动画手感仍以用户真机拖动复核为最终视觉验收。

**走查修复⑤（同日，Pointer Events 重构）**：用户真机反馈“托起瞬间闪一下后不能拖动”。复核确认 v3 的稳定性假设错误：`setPreview(lifted)` 后，两个视图把原卡从 map 子树移除、在另一个条件分支渲染隐藏 source；相同 key 跨子树不能保住同一 DOM，WKWebView 因原生 drag source 被卸载而立即取消，随后 `dragend` 清空无落点预览，形成闪回。v4 移除 HTML5 DnD：移动超过 5px 后先克隆 DOM 为独立 DragOverlay，再移除原卡；pointermove/up/cancel、Esc、失焦全部绑定稳定 document/window，墙外释放恢复；尾随 click 被拦截，避免误打卡。提交邻居同时收紧为目标星级档内查找，杜绝跨档 sort_order 混算。

**验证⑤**：✅ `bash scripts/check.sh` 六步全绿（Rust/CLI/迁移全测 + 前端 vitest 29/29 + production build；前端新增拖动阈值、目标档邻居测试）；✅ 最新 debug `.app` 构建；✅ 4 张临时卡隔离库实拖：均衡混排与类型分区各一次有效释放均只新增一条 `task.move` 且顺序落盘，墙外释放不新增 activity 并恢复卡片；首页拖拽事件链可持续运行。正式数据库未参与验证。

**走查修复⑥（同日，抽动治理）**：抽动来自两层反馈叠加：①命中检测读取正在 FLIP 位移中的视觉矩形，卡片让位后会反向改变下一次落点；②上一轮动画尚未结束时，新 FLIP 又把带 transform 的视觉位置当作布局位置，补间会从错误坐标重启。修复为：命中与 FLIP 终点统一扣除当前 translate、双列先按水平距离选列、上下半区加入 12px 迟滞、跨目标累计移动 6px 后才重新武装；pointermove 只保留最新坐标并每动画帧最多计算一次。连续 FLIP 从当前视觉位置衔接到新布局，并取消过期 rAF，避免正反重排与跳帧。

**门禁降噪**：`scripts/check.sh` 改为必须显式选择 `frontend` / `rust` / `full`。纯前端只跑 vitest + tsc/vite，纯 Rust 跑四项 Rust 检查，跨层/协议/迁移/发布才跑 full；同一批改动相关文件未再变化时复用已通过结果。删除 Spec 006 已完成的“check.sh 全绿”任务项，保留历史验证记录和回归测试本身，避免把“通过过”误解成删除测试保护。

**验证⑥**：✅ `bash -n scripts/check.sh`；✅ `bash scripts/check.sh frontend`（5 个测试文件、31/31 通过 + production build）；✅ debug `.app` 重建并重启；✅ 正式界面墙外释放检查后任务顺序未变化。Rust/Core/协议/迁移未改，按新规则未重复跑 Rust 门禁。

**走查修复⑦（同日，浮层漂移与跟手性）**：用户复测发现首页偶发“托起后飞到鼠标上方”，两页均有跟手延迟。根因确认：FLIP 会把 `transition: transform 180ms` 留在卡片 wrapper，DragOverlay 的 `cloneNode(true)` 连同 inline transform/transition 一起复制；旧浮层又从 fixed `(0,0)` 直接 transition 到鼠标绝对坐标，WebKit 因而先显示在高处再追赶。v4.2 将命中 wrapper 与 FLIP 视觉 inner 分层；克隆浮层显式清除 FLIP 样式，以源卡 fixed 坐标为基准只叠加 `pointer - start` 的小位移，取消 scale 与昂贵 drop-shadow filter。性能侧把首页 DnD 状态下沉到今日任务网格，避免每次预览重渲染 371 天热力图和右栏；两类任务卡 memo 化，预览 setState 改为可中断 transition。

**验证⑦**：✅ `bash scripts/check.sh frontend`（31/31 + production build）；✅ debug `.app` 重建并重启；✅ 首页与任务页分别实拖到墙外，截图核对浮层左上角和原抓取偏移后，抓取点均精确落在指针终点；经历一次 FLIP 后再次托起未再继承旧 transition，释放恢复且未提交顺序。Rust 未改，未重复全量门禁。

**走查修复⑧（同日，非拖动卡闪烁）**：用户复测发现首页其他卡片在随落点让位时仍会闪烁、跳跃。根因有两项：① v4.2 为降低渲染优先级使用 React `startTransition`，但 FLIP 已在调用前拍下旧位置；延迟提交期间新的指针帧可能继续覆盖预览与快照，真正渲染时旧位置已失效；②指针经过让位卡片时仍会触发卡片自身 `hover:-translate-y-px`，与 FLIP 位移叠加。v4.3 恢复快照后的同步 `setPreview`（首页网格已隔离，不会牵连大块内容），并在整个拖拽会话中暂停首页卡与任务墙卡的 hover transform/transition；非拖动卡只保留单一 FLIP 位移源。

**验证⑧**：✅ `bash -n scripts/check.sh`；✅ `bash scripts/check.sh frontend`（5 个测试文件、31/31 通过 + production build）；✅ debug `.app` 重建并重启到首页。Rust/Core/协议/迁移未改，按分层门禁不重复 Rust 检查。

**走查修复⑨（同日，首格闪跳与左列迟钝）**：复测说明 v4.3 只消除了 React 延迟和 hover 叠加，尚未解决 FLIP 轨迹所有权。移到第一格时，多张卡会跨列换位；旧 CSS transition 仍挂在新布局上，尤其“新布局恰好等于当前动画视觉位置”时旧轨迹未被取消，会先朝旧终点继续走、再被下一轮拉回。v4.4 将 FLIP 改为可取消的 Web Animations：快照同时记录视觉/布局坐标，只给真正换位的卡片接续动画；新布局到来先按当前视觉位置算反演量、取消旧动画，再用 240ms ease-out 接到新位置，未换位卡不重启动画。回调 ref 固定复用，避免每轮渲染重复解绑。

**落点修订⑨**：左侧卡片“舍不得让位”不是单纯动画速度，而是旧规则以 50% 垂直中线区分前/后——光标落在卡片中下部会被解释为“放在它后面”，在双列网格里等于占右侧/下一槽。现改为卡片前 68% 均代表直接占据该槽、仅底部代表其后；当前占位外扩 12px 作为稳定区，切换目标累计移动提高到 10px。起拖后指针离开原卡范围才启用落点，继续保证原位置先真正腾空。实际换位通过 `flushSync` 与 FLIP 快照同帧提交。

**验证⑨**：✅ `bash scripts/check.sh frontend`（5 个测试文件、32/32 通过，新增占位稳定区与落点分界测试；TypeScript + production build 通过）；✅ debug `.app` 重建并完整重启，首页正式数据已正常回填。Rust/Core/协议/迁移未改，不重复 Rust 门禁。

---

## [2026-09-02] 任务页切换器规格统一 + 归档页重构（A+B 方案）

**需求简述**：任务页「进行中/已归档」「均衡混排/类型分区」选中色块与日期控件高度不一致，以日期色块为准统一；「已归档」标签去数字；归档页太简陋（任务多了找不到），选定 A+B 方案优化。

**修复**：①三个切换器统一为同一分段控件规格——`bg-hover p-0.5` 圆角容器 + `h-6 rounded-md` 内块，选中 = `bg-surface + shadow-sm`（与快速捕捉/日期控件同规格）；原先的文字按钮（`py-1` + accent/10 底色）下线。②归档页重构（A+B）：搜索框（标题即时过滤）+ 排序分段控件（归档时间/名称/创建时间）+ 按归档月份分组（组内倒序）+ 小卡片（图标色块/标题/归档日期/查看/恢复）。后端新增 `period_repo::archived_days`（GROUP BY 取活动区间最后关闭日 MAX(end_day)）+ `task_service::list_archived_tasks` + Tauri command；CLI 与协议不动。check.sh 全绿（core 29 测含归档日链路）。

**同日追加**：③控件高度立规——同排输入框/选择框必须等高：表单控件一律 `inputCls`(h-9)，工具栏控件一律 28px（新 `inputClsSm`=h-7 与分段控件等高）；归档搜索框已换 inputClsSm。④归档页加「指定日期」筛选：`DatePickerPanel` 支持 `marks`（日→数量）——有归档任务的日子在格子底部渲染色块，颜色深度随当天任务数（相对最大计数 30%→100%）；选中筛选日只显示该日归档任务，可一键清除。
⑤按钮高度立规补记：全局 `Button` 从 py 撑高（约 32px）改为固定 h-9（36px，inline-flex 居中）——页头「＋新建任务」与编辑器底部按钮从此与输入框（inputCls h-9）完全等高。
⑥**高度规则当日再修订（用户定稿单档）**：两档（表单 h-9/工具栏 28px）否决——用户要求按钮、弹窗输入框与任务页工具行一致。全部控件统一 h-7（28px）：Button/inputCls/FieldSelect/EmojiPicker/DateField/搜索框/分段控件（p-0.5+h-6 总高 28px）；inputClsSm 删除；快速捕捉输入框换 inputCls；星期按钮 h-7；日期面板月/年格子与日格子同高 h-8。

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
- 走查修订：①翻页日期过滤不适用任务（如该日前创建的任务不出现）；「今天」的墙仍保留轮空卡（004 决策不变）。②日期导航从独立凸出的一行改为**一体分段控件**（`bg-hover` 圆角容器内 ◀ 日期 ▶，与快速捕捉切换器同规格），收进标签行：左侧 tabs + 日期控件，右侧排列方式切换；非今天时日期高亮主题色 + 「回到今天」。

**同日｜编辑器与设计系统统一**（用户反馈任务编辑器五点）：①图标支持 emoji 选择器（64 个常用表情网格弹层）+ 保留直接打字；②主题色 6 → **15**（taskVisual 与 domain `TASK_COLOR_PRESETS` 同步扩，后端只校验 #RRGGBB 格式无需改动）；③输入框/下拉统一 `inputCls`（h-9 等高）+ 新 `FieldSelect`（appearance-none + 自带 ▾，消除原生下拉与输入框高度差）；④按钮统一加高到 h-9 量级（`px-3.5 py-1.5 text-sm`），圆角矩形不变；⑤`EmojiPicker`/`ColorSwatches`/`FieldSelect` 收进 ui.tsx 共享，重要日编辑器（LibraryEditor）与归档弹窗下拉同步换用。

**同日｜日期选择统一为分层选择器**：原生 `<input type="date">` 全部下线。ui.tsx 新增 `DatePickerPanel`（日/月/年三层：点标题逐级上升、点格子逐级下降；◀ ▶ 按当前层级翻页；今天=主题色文字、选中=主题色填充；页脚「今天」快捷键；周一在前）与 `DateField`（输入框外形、点击弹层）。接入：任务页日期导航弹层、重要日编辑器锚点日期。

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

## 2026-09-05 番茄钟完成任务自动启动修复

**问题**：点击完成任务后，番茄钟自动开始倒计时。

**根因**：`examples/plugins/com.leeyl.pomodoro/main.js` 订阅 `task.completed`，收到事件后主动调用 `start()`。

**修复**：移除完成任务事件订阅及对应权限声明，番茄钟仅由“开始一个番茄钟”命令或按钮启动；同步更新插件说明文案。

**验证**：`bash scripts/check.sh frontend`（vitest 32/32、TypeScript、Vite production build）通过。
## [2026-09-05] 插件平台演进 v1.1：manifest 能力声明切片

- 新增插件 manifest 可选字段：`api_version`、`min_host_version`、`permissions.core`、`permissions.ui`、`permissions.storage_quota_bytes`。
- 保持 plugin.protocol/v1 和旧插件兼容；未知 capability、非法协议版本和超出 1B–10MB 配额的 manifest 会被 Rust/TypeScript 双端拒绝。
- 新增 Rust 与 Vitest 回归测试；当前仅完成协议校验切片，权限执行、SDK、安装回滚和 capability token 仍待后续任务。
- 第二片补充：Tauri 非法 actor 不再回退为 user；HTTP 代理关闭自动重定向并拒绝 localhost/私有 IP；CLI 脚手架同步 capability 字段和语义主题 token。插件 CLI 集成测试通过。
- 第三片补充：`dashboard plugin new <id> --template ts` 生成 `main.ts`、`plugin-sdk.d.ts` 和 esbuild 配置；JS 默认模板继续兼容，脚手架集成测试覆盖两种模板。
- 第四片补充：新增 [packages/plugin-sdk](/Users/leeyl/Project/AIODashboard/packages/plugin-sdk) 与 [packages/plugin-test](/Users/leeyl/Project/AIODashboard/packages/plugin-test)，公共类型和离线测试桩的 TypeScript 构建通过。
- 第五片补充：CLI 新增 `plugin pack/install/rollback`；zip 导入检查绝对路径和 `..` 路径，使用 staging 解压、manifest 校验、旧版本备份和回滚；新增集成测试通过。哈希校验仍待补充。
- 第六片补充：`plugin pack` 输出 SHA-256，`plugin install --sha256` 校验不匹配时拒绝安装；pack/install/rollback 写入 activity log；新增错误哈希回归测试。
- 第七片补充：插件管理页新增来源、协议版本和 `trusted-webview` 信任模式标签；未把 SHA-256 伪装成已持久化状态，持久化展示仍待后续实现。
- 第八片补充：SCHEMA_V7 新增插件安装元数据；zip 安装后持久化来源、SHA-256 和安装版本，插件页显示截断哈希。安装状态操作 UI 仍待实现。
- V7 迁移补充了旧测试数据库缺少 plugin_registry 的兼容分支，storage 全部迁移测试和 workspace cargo check 通过。
- 插件管理页补充安装版本、上一版本标签；页面保持只读状态展示，安装/回滚继续使用 CLI，避免接入未完成的 GUI 操作入口。


## [2026-09-08] Spec 008 插件安全与生命周期加固完成

**范围**：继续执行已批准的六切片计划，保留同进程 Trusted WebView；001/007 历史记录不作为当前安全保证。当前状态与全部证据见 [008 tasks](../openspec/changes/008-plugin-security-hardening/tasks.md)。

**交付**：Core token/actor 绑定、逐次内容和 revision 验证、显式权限、UTF-8 KV quota、领域/自定义事件边界、统一拒绝审计；DNS 固定地址与 HTTP 限额；安全 ZIP/目录导入、journal 恢复与精确 id 回滚；SDK disposer、设置、取消和串行 reload、CLI/GUI 指纹同步；JS/TS vendor 脚手架和 plugin dev --run；管理 UI 完整性/权限/设置/错误提示及停用全部。安装/回滚按计划允许的方案明确为 CLI-only。

**迁移**：plugin.protocol/v2 取代可执行的旧 v1；未声明能力不授权。AI meta.schema_version 从 5 升为 6（插件列表授权/摘要/revision 字段与默认停用语义，permission_denied/exit 4）。新增数据库 V8：content_sha256、approved_sha256、revision、install_operation，并停用既有插件要求重新确认。旧插件必须迁移，不存在宽权限兼容。

**验证**：full 门禁已执行；最终 Rust fmt/clippy/test/build 通过（98 测试），修正新增 IPC 测试中的 owner 参数后 frontend 门禁通过（39 Vitest、6 SDK/mock/示例测试、SDK/test 和前端构建），复用未变更的 Rust 结果。独立 JS/TS 脚手架 build/test 通过；macOS debug bundle 构建与真实窗口验证通过：首次权限确认、停用同步、设置持久化、双主题、重载、计时状态恢复、故障注册清理、回滚元数据、safe-mode。真实验收额外发现并修复 Rust null quota 被 TS 误判的问题，补充回归并重新构建复验。

**数据路径事件**：此前 LaunchServices 启动验收包时没有可靠保留临时环境，窗口曾读取正式库。已立即退出并告知用户；只读核对正式库当前 V8/10 条任务、临时库 0 条任务，无法据此确认迁移发生时点。未操作正式任务或回退迁移。此后改为直接执行 bundle 内二进制并显式设置三项临时路径，剩余验收全部在临时库完成；验收实例已退出。

**边界**：Trusted 权限不是沙箱；同步死循环须 CLI safe-mode 后重启。第三方代码隔离、签名/公证、市场、账号和跨设备同步未纳入。工作区还含其他任务的既有改动，本轮未直接提交混合 diff。


## [2026-09-08] 求职台界面试用插件导入

**需求与范围**：用户审阅界面后要求导入软件，并明确选择「先导入界面试用版，使用演示数据」。完成 [试用 Plan](plans/2026-09-08-job-tracker-ui-trial.md)；Spec 009 的正式求职领域、持久化、CLI 和真实任务关联仍未实施。

**交付**：新增 `examples/plugins/com.leeyl.job-tracker-trial/`。移除模拟宿主外壳，注册真实 View/TodayCard，复用宿主 React/主题，单文件 ESM 与本地 SVG 图标；只申请 UI 权限，试用记录在同一激活期间共享，重载恢复样本。右键/更多编辑、全高详情与安排表单、固定保存底栏、明确标识的任务卡片预览可操作。定时器、监听器、节点和样式随卸载清理。

**原生验收**：旧安装版没有导入入口，保留原安装版及窗口，打开项目新版 `target/debug/bundle/macos/AIODashboard.app`，在真实数据目录通过 GUI 导入/审阅启用。验证城市编辑、面试轮次及日期时刻修改同步到 Today 卡片、滚动后的卡片全高详情、双主题、演示任务完成/撤销、停用移除入口和重新启用恢复样本；修正宿主侧栏的字符图标契约与卡片重绘后的焦点。最终留在「求职台·试用」，主题恢复跟随系统。

**验证**：插件 2 项测试、构建/入口语法、独立 CLI pack/install/enable 通过；最终 frontend 45 Vitest、7 SDK/mock/示例测试及构建通过。新版宿主额外验证 Rust fmt/clippy、104 测试和 workspace build；回环 HTTP 测试的沙箱端口限制在获审批后解除并通过。`git diff --check` 通过。未改动 Rust/协议/数据库迁移代码。

**数据与交付物**：导入前一致性备份和导入后逐条核对确认原有业务数据不变，包括 10 条任务、136 条打卡记录；Today 仍为 6 项/0 完成。ZIP 为 `dist/plugins/job-tracker-trial-0.1.0.zip`，SHA-256 `abf52206298e95b6ff5b82a4cf612a92e6757fc12786f1e8f4e29688d737bb10`。证据和仅本地保留的备份在 Git 忽略目录 `target/job-tracker-trial-20260908/`。本次没有替换 `/Applications` 中的旧版应用；试用需要使用项目新版桌面程序。旧窗口退出被自动审批拒绝后采用保留窗口的安全路径完成导入。

## [2026-09-17] v0.1.0 公开发行准备与项目展示

**需求**：同步本地最新实现至 GitHub，发布 0.1，完善 README、真实预览和宣传页；用户明确授权将仓库公开。

**实现**：版本统一沿用应用 0.1.0，CLI schema 6、插件协议 v2、数据库 V8 与 SDK/test 0.2.0 保持独立。既有 UI、插件平台与导入、求职演示改动按意图分片提交。新增品牌概念封面、中文 README、安装/升级说明、MIT LICENSE（遵循已有 Cargo license）、贡献说明、GitHub Pages 静态宣传页和发行说明。

**用户追加修订**：首页「今日任务」统一主题绿，进度填充 9%、完成填充 16%，保留原色 emoji；其他任务视图继续使用任务自定义颜色。不预装番茄钟，明确示例尚未完成，不作为可用功能宣传。

**验证**：full 门禁通过（104 Rust、45 Vitest、7 SDK/示例），后续仅改首页样式时重跑 frontend 并复用未变更 Rust 结果。使用 Command Line Tools 构建 arm64 release app / DMG / CLI。发行 CLI 在独立数据库验证创建、幂等打卡、context、快照和空插件列表。Gitleaks 8.30.1 对原有 62 个提交及待发布工作树扫描未发现密钥；其下载校验和已核对。

**素材与数据边界**：截图使用临时演示数据库、插件目录、快照路径；直接启动 release 可执行程序的预览副本，副本仅修改应用标识/名称以与正式实例区分，可执行文件字节相同。未将用户正式数据用于展示。此前为临时演示启用的番茄钟已停用并移出临时插件目录。封面使用内置 imagegen，提示词与素材说明见 assets/README.md。

**发行边界**：本次二进制仅 Apple Silicon macOS，ad-hoc 签名、未 Developer ID 分发签名或公证；无内置大模型、自动更新、云同步或原生 WidgetKit。发布与线上核验结果在后续记录中补齐。
