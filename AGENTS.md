# AIODashboard · AGENTS.md

> 本文件是 AI 助手的"地图式导航"：只写本项目特有的约定与指针，不堆实现细节。
> 详细内容见「关键参考」。保持 ≤ 300 行，避免注意力稀释。

## 一句话简介

Local-First、AI-Friendly、CLI-First 的个人 All-in-One Dashboard（macOS 优先）。
**Rust Core 是唯一业务核心；GUI 与 CLI 是平级客户端；AI 通过稳定 JSON CLI 操作系统。**

## 技术栈

- Core：Rust（Cargo Workspace，edition 2021）
- 数据库：SQLite + WAL（rusqlite bundled）
- Desktop：Tauri 2 + React 18 + TypeScript(strict) + Tailwind CSS 4
- CLI：clap 4
- Node.js ≥ 20（前端构建必需）

## 目录结构

```text
crates/
├── dashboard-domain      # 领域实体（零基础设施依赖）
├── dashboard-storage     # SQLite + Repository —— 全仓唯一允许写 SQL 的地方
├── dashboard-core        # Application 用例 —— 全部业务规则唯一实现处
├── dashboard-protocol    # AI 输出协议（JSON 信封 / exit code 常量）
apps/
├── cli/                  # `dashboard` 二进制（面向人类+AI 的入口）
│   └── tests/integration.rs   # §31 四条链路的端到端测试
└── desktop/              # Tauri 2：src/(React) + src-tauri/(Interface 层)
openspec/                 # Spec 变更包：changes/<NNN-slug>/ 四件套（proposal/spec/design/tasks）
docs/                     # architecture.md / dev-log.md / plans/ / PLUGIN_API.md
scripts/check.sh          # 机械门禁（与 CI 同构）
.github/workflows/ci.yml  # CI（push/PR 自动跑同一套检查）
```

## 架构红线（不可绕过）

1. **SQL 只出现在 `dashboard-storage`**；其他 crate 出现 `rusqlite` SQL 字符串即违规。
2. **业务逻辑只写在 `dashboard-core`**；Tauri command 与 clap handler 只做参数转换和输出。
3. **前端禁止直接读写 SQLite**，只能 `invoke` Tauri command。
4. 所有业务变更必须：写 activity log（`log_activity`）→ 刷新 Widget Snapshot（`snapshot::refresh`）。
5. Domain 层零依赖基础设施（不得引入 rusqlite/clap/tauri）。
6. 时间一律 UTC 存储（RFC3339 文本）；"今天/逾期"边界用本地时区换算
   （唯一入口 `context_service::local_today_range`），不要在别处重写这段逻辑。
7. 数据库迁移只增不改：新变更 = 新的 `SCHEMA_V{n}` 分支 + `user_version` 递增，
   禁止修改已发布过的历史迁移。
8. AI 协议是公共 API：JSON 信封字段、CLI 输出字段、exit code 语义变更 = Spec 级变更，
   必须升 `meta.schema_version` 并在 dev-log 记录迁移说明。
9. 库代码路径禁止 `unwrap()/expect()`（启动入口与测试除外）；错误统一 `CoreError`。

## 新增领域实体的固定清单（Checklist）

按序走完全部六层，缺一不可：

```text
1. domain    实体 + id 前缀（crates/dashboard-domain）
2. storage   表(SCHEMA_Vn) + repository（含索引）
3. core      service 用例 + activity log + snapshot::refresh
4. cli       子命令，支持 --json / 错误映射到 exit code
5. tauri     command 包装 core service
6. frontend  types.ts + api.ts + 视图组件（snake_case 与 serde 对齐）
7. tests     集成测试进 apps/cli/tests/integration.rs
```

## 常用命令

```bash
bash scripts/check.sh          # 机械门禁：fmt → clippy(-D warnings) → test → build → 前端 build
cargo test --workspace         # 只跑 Rust 测试
cd apps/desktop && npm run tauri dev    # 启动桌面端
cargo build -p dashboard-cli && target/debug/dashboard --help   # CLI
```

环境注意：需要 `rustup component add clippy rustfmt`（本机已装）；Node 必须 ≥ 20。

## 开发流程分级

| 任务举例 | 模式 | 要求 |
|---|---|---|
| 改文案 / Tailwind 微调 / 小 bug | Vibe | 说清现状与预期；完成后 AI 自述改动；跑 check.sh |
| 新增 CLI 子命令 / 新增视图 / 单个 command | Plan | 先复述需求+分步计划确认；补对应测试；记 dev-log |
| 新领域模块(Habit/Calendar…) / schema 变更 / AI 协议变更 / daemon | Spec | Why→What→Unknown 提案确认后：spec(ADDED/MODIFIED/REMOVED)→tasks 逐项勾选；TDD：先测试用例确认再实现 |

判断三问：①新功能还是修改？②影响其他模块吗？③一个月后自己还看得懂吗？
任一"新功能/跨模块/不确定" → 至少 Plan；涉及协议/schema/新模块 → Spec。

对话流程：说需求 → AI 复述确认 → 审阅计划 → 执行 → 跑门禁验证 → 记 dev-log。
反模式：不看计划直接执行、一次生成全部代码、AI 报错后人肉改代码（应把报错喂回）、
攒大 diff 不切片提交、只跑 type-check 就宣称交付（必须完整 build）。

**文档落位**：Spec 级任务在 `openspec/changes/<NNN-slug>/` 建变更包，固定四件套
`proposal / spec / design / tasks`（规则与索引见 `openspec/README.md`，格式对齐 OpenSpec）；
Plan 级任务在 `docs/plans/` 建档（命名 `YYYY-MM-DD-<slug>.md`，从 `_TEMPLATE.md` 复制）。
核心纪律：**切片提交时同步勾选 tasks.md 复选框**；完成后当天在 proposal.md 标注「已完成」；
实施中发现影响面超出 Plan → 升级为 Spec 变更包。

## 测试规范

- Plan 级：新增功能必须有对应测试（core 单测 或 integration.rs 链路测试）+ 人工验证核心路径。
- Spec 级：TDD——先让 AI 生成测试用例清单并确认，再实现；测试不过自动修复（≤3 轮）后继续。
- 纯逻辑优先可测：日期边界、过滤排序等抽成纯函数放 core/context_service 这类无 UI 模块。
- 门禁定义交付：`bash scripts/check.sh` 全绿才算完成；口头"应该没问题"无效。

## Git 纪律

- 大改动按可验证切片提交，建议顺序：修复 → core 功能 → CLI → 前端 → 工具链/CI → 文档。
- 一个 commit 不混多个意图；拆分重构 PR 禁止夹带新功能。
- 当前仓库尚未 `git init`：首次初始化后建议接 GitHub 并启用 `.github/workflows/ci.yml`
  （workflow 文件推送需要 token 带 `workflow` scope）。

## 当前状态

MVP（Phase 0 + Phase 1）已完成并验证四条核心链路：
Task / Project / Note / Inbox / Today / Search + GUI + CLI --json + Widget Snapshot v1。
下一步见 README Roadmap（Phase 3 SwiftUI Widget / Phase 2 AI Interface 完善）。

## 关键参考

- 总体设计文档 V0.1（用户提供的 PDF/MD，架构权威来源）
- 架构落地说明：`docs/architecture.md`
- 开发日志（每次对话后更新）：`docs/dev-log.md`
- Spec 变更包（Spec 级必须建档，四件套）：`openspec/changes/`（规则与索引见 `openspec/README.md`）
- Plan 文档（Plan 级建档）：`docs/plans/`（README 含规则与索引）
- 使用说明 & Roadmap：`README.md`
- 链路测试范例：`apps/cli/tests/integration.rs`

---

## 给 AI 的总约束（每次会话可引用）

```text
约束：
1. 先对齐再动手：先复述需求和分步计划，确认后再执行。
2. 交付前必须跑 bash scripts/check.sh 全绿（fmt/clippy/test/build/前端 build 缺一不可）。
3. 架构红线不可绕过：SQL 只在 storage、业务只在 core、前端不碰 DB、变更必留 activity log。
4. 新实体走六层 Checklist，并在 apps/cli/tests/integration.rs 补链路测试。
5. CLI --json 输出是 AI 公共 API：改字段属 Spec 级变更，需升 schema_version。
6. 迁移只增不改；时间边界只用 local_today_range；库代码禁 unwrap/expect。
7. 大改动切片提交；禁止顺手全仓 format 或无关重构。
8. 结束后更新 docs/dev-log.md，必要时同步 README / docs/architecture.md。
```
