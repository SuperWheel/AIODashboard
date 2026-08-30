# AIODashboard

Local-First、AI-Friendly、CLI-First 的个人 All-in-One Dashboard（MVP / macOS 优先）。

> 架构原则：**Rust Core 是唯一业务核心**；GUI 与 CLI 是平级客户端；AI 通过稳定 JSON CLI 操作系统；SQLite 是第一阶段唯一事实来源。

## 技术栈

| 层级 | 技术 |
|---|---|
| Desktop | Tauri 2 |
| GUI | React 18 + TypeScript + Tailwind CSS 4 |
| Core | Rust（Cargo Workspace） |
| 数据库 | SQLite（WAL 模式） |
| CLI | Rust + clap |

## 项目结构

```text
crates/
├── dashboard-domain      # 领域层：Task / Project / Note / InboxItem / ActivityEntry
├── dashboard-storage     # Infrastructure：SQLite + Repository（唯一允许写 SQL 的地方）
├── dashboard-core        # Application：所有业务用例（GUI/CLI 共用）
└── dashboard-protocol    # AI 输出协议：JSON 信封 { success, data, error, meta }
apps/
├── cli                   # `dashboard` 命令行（一等公民，面向 AI）
└── desktop               # Tauri 2 桌面端（src/ 前端 + src-tauri/ 后端）
```

数据流（两条链完全一致）：

```text
React ──Tauri Command──┐
                       ├──► dashboard-core ──► SQLite (WAL)
dashboard CLI ─────────┘
```

CLI 与 GUI 通过共享数据库文件 + WAL 并发 + 前端轮询实现实时同步。

## 环境要求

- Rust stable（`rustup` 安装）
- Node.js ≥ 20
- macOS（Xcode CLT）

## 快速开始

### 1. 运行桌面端

```bash
cd apps/desktop
npm install
npm run tauri dev     # 开发模式
# 或打包： npm run tauri build
```

### 2. 使用 CLI

```bash
cargo build -p dashboard-cli
alias dashboard="$PWD/target/debug/dashboard"

dashboard status
dashboard task create --title "喝水" --target 8 --unit 杯 --icon 🥤
dashboard task checkin tsk_xxx   # 打卡 +1（幂等）
dashboard task overview tsk_xxx --period week
dashboard library create --title "考研" --kind countdown --anchor 2026-12-21
dashboard context today          # AI 一键获取当前状态
```

### 3. AI / Agent 接入

所有命令支持 `--json`，返回稳定信封：

```bash
$ dashboard task list --json
{
  "success": true,
  "data": [ { "id": "tsk_…", "title": "…", "status": "active", "card_style": "day", "color_hex": "#4A90E2" } ],
  "error": null,
  "meta": { "schema_version": "2" }
}
```

- **Exit Code**：`0` 成功 · `1` 一般错误 · `2` 参数错误 · `3` 数据不存在 · `5` 冲突
- **stdin**：`echo '{"title":"...","target":8,"unit":"杯"}' | dashboard task create --stdin`
- **Dry Run**：`dashboard task delete <id> --dry-run`
- **幂等打卡**：`dashboard task checkin <id> --operation-id <key>`（重放不重复计数）
- **操作来源标记**：`DASHBOARD_ACTOR=ai dashboard task checkin tsk_xxx`（写入审计日志）

> 协议 v2（2026-08-30）：任务从 todo 改为长期打卡对象。`complete/reopen` 保留为重映射别名
> （补满今日目标 / 今日清零），输出带 `deprecated` 提示；`--due`、`--today/--overdue` 已移除。

推荐 Agent 工作流：

```bash
dashboard context today --json   # 1. 了解现状
# …AI 分析规划…
DASHBOARD_ACTOR=ai dashboard task create --title "…"   # 2. 执行修改
dashboard activity list --actor ai                     # 3. 可审计
```

GUI 每 4 秒自动轮询刷新，CLI/AI 的修改会自动出现在界面上。

## 数据位置

- 数据库：`~/Library/Application Support/AIODashboard/dashboard.db`
- Widget 快照：`~/Library/Group Containers/group.com.aiodashboard.shared/widget-snapshot.json`（不存在则写入数据库同目录 `widget/`）
- 覆盖：环境变量 `DASHBOARD_DB_PATH`、`DASHBOARD_WIDGET_SNAPSHOT_PATH`

Widget Snapshot 协议（`widget.snapshot/v1`）：

```json
{
  "schema": "widget.snapshot/v1",
  "generated_at": "2026-08-22T08:49:08Z",
  "today": {
    "date": "2026-08-22",
    "task_total": 5,
    "task_completed_today": 3,
    "completion_rate": 0.62,
    "missed_last_7d": 2,
    "inbox_open": 4,
    "next_event": null,
    "current_focus": "Dashboard MVP"
  }
}
```

每次任务/收件箱变更后自动重新生成，供未来 SwiftUI WidgetKit 直接读取展示。

## MVP 功能范围

- ✅ Task（打卡式）：创建(目标/单位/图标/主题色) / 打卡+1 / 减少 / 撤销 / 周月年总览 / 归档恢复 / 四种卡片
- ✅ DateLibrary：纪念日 / 倒计时日主库、任务归属（历史留痕）、综合热力图、归档三选一
- ✅ Project：创建 / 归档 / 删除 / 进行中任务数统计
- ✅ Note：创建 / 编辑 / 删除 / 最近列表
- ✅ Inbox：快速收集 / 转 Task / 转 Note / 删除
- ✅ Today 页：今日待办、逾期提醒、快速添加、统计卡片
- ✅ Unified Search：⌘K 全局搜索（任务/项目/笔记/收件箱）
- ✅ Context 系统：`dashboard context today`
- ✅ Activity Log：全量审计（区分 user/cli/ai/automation/system）
- ✅ Widget Snapshot：V1 协议 + 文件输出

## 开发

```bash
bash scripts/check.sh   # 机械门禁：fmt → clippy → test → build → 前端 build（交付前必须全绿）
cargo test              # 单元测试 + 端到端链路集成测试（§31 四条链）
cargo build -p dashboard-cli   # 只构建 CLI
```

- 架构细节：`docs/architecture.md`
- AI 协作约定与开发流程分级：`AGENTS.md`
- 开发日志：`docs/dev-log.md`

## Roadmap

按设计文档阶段推进：

- Phase 2：AI Interface 完善（--request-id 幂等、权限策略、JSON Schema）
- Phase 3：macOS Native —— SwiftUI + WidgetKit 小组件（消费现有 snapshot 协议）
- Phase 4：Automation（Trigger → Condition → Action）
- Phase 5：第三方集成（Calendar / GitHub / Email）
- Phase 6：跨设备同步（确有需求再做）
