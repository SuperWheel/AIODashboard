# 开发日志（dev-log）

> 每次对话结束后用 3 分钟记录：做了什么、关键决策、验证结果。
> 模板见文末。按时间倒序排列（最新在上）。

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
