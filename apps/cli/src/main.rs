//! dashboard —— 个人 All-in-One Dashboard CLI
//!
//! 设计要点（面向 AI）：
//! - `--json`：稳定 JSON 信封 `{ success, data, error, meta.schema_version }`
//! - 标准 Exit Code：0 成功 / 1 一般错误 / 2 参数错误 / 3 数据不存在 / 5 冲突
//! - `--stdin`：复杂输入通过 JSON 传入
//! - `--dry-run`：预览删除等危险操作
//! - 环境变量 `DASHBOARD_ACTOR=ai|cli|automation|user`：标记操作来源，写入审计日志

mod output;
mod util;

use std::io::Read;

use chrono::Utc;
use clap::{Parser, Subcommand};
use dashboard_core::{
    context_service, inbox_service, note_service, plugin_manifest, plugin_service, project_service,
    search_service, task_service,
};
use dashboard_core::{CoreError, CoreResult};
use dashboard_domain::{Actor, TaskStatus};
use dashboard_protocol::{Envelope, ExitCode};
use dashboard_storage as ds;
use serde_json::{json, Value};

#[derive(Parser)]
#[command(
    name = "dashboard",
    version,
    about = "个人 All-in-One Dashboard CLI",
    long_about = None
)]
struct Cli {
    /// 输出机器可读的稳定 JSON 协议
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 任务管理
    Task {
        #[command(subcommand)]
        cmd: TaskCmd,
    },
    /// 项目管理
    Project {
        #[command(subcommand)]
        cmd: ProjectCmd,
    },
    /// 笔记管理
    Note {
        #[command(subcommand)]
        cmd: NoteCmd,
    },
    /// 收件箱（快速收集）
    Inbox {
        #[command(subcommand)]
        cmd: InboxCmd,
    },
    /// 插件管理
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// 全局搜索（任务 / 项目 / 笔记 / 收件箱）
    Search { query: String },
    /// AI Context：一次性获取当前状态
    Context {
        #[command(subcommand)]
        cmd: ContextCmd,
    },
    /// 操作审计日志
    Activity {
        #[arg(long)]
        actor: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// 数据库与系统状态
    Status,
}

#[derive(Subcommand)]
enum TaskCmd {
    /// 列出任务
    List {
        #[arg(long)]
        today: bool,
        #[arg(long)]
        overdue: bool,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
    /// 查看单个任务
    Show { id: String },
    /// 创建任务
    Create {
        #[arg(short, long)]
        title: Option<String>,
        /// YYYY-MM-DD 或 RFC3339
        #[arg(long)]
        due: Option<String>,
        #[arg(long)]
        project: Option<String>,
        /// 从 stdin 读取 JSON：{"title":..., "due":..., "project":...}
        #[arg(long)]
        stdin: bool,
    },
    /// 更新任务
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        due: Option<String>,
        #[arg(long = "clear-due")]
        clear_due: bool,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long = "clear-project")]
        clear_project: bool,
    },
    /// 标记完成
    Complete { id: String },
    /// 重新打开
    Reopen { id: String },
    /// 删除任务
    Delete {
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// 清理全部已完成任务
    ClearCompleted {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ProjectCmd {
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(long, default_value_t = String::new())]
        description: String,
    },
    List {
        #[arg(long = "all")]
        include_archived: bool,
    },
    Archive {
        id: String,
    },
    Delete {
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum NoteCmd {
    Create {
        #[arg(short, long, default_value_t = String::new())]
        title: String,
        #[arg(short, long, default_value_t = String::new())]
        body: String,
    },
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    Show {
        id: String,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum InboxCmd {
    Add {
        content: Vec<String>,
    },
    List {
        #[arg(long = "all")]
        include_processed: bool,
    },
    /// 转为任务
    Task {
        id: String,
    },
    /// 转为笔记
    Note {
        id: String,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum ContextCmd {
    Today,
}

#[derive(Subcommand)]
enum PluginCmd {
    /// 列出插件（磁盘发现 ∪ 注册表状态）
    List,
    /// 启用插件（未注册但磁盘合法的插件会先登记）
    Enable { id: String },
    /// 停用插件
    Disable { id: String },
    /// 创建插件脚手架（零工具链 JS 模板，写入插件目录）
    New { id: String },
    /// 校验插件并打印开发信息（manifest / 权限 / 贡献点 / 入口）
    Dev {
        /// 只检查该插件；缺省检查全部已发现插件
        id: Option<String>,
    },
}

const SCAFFOLD_MANIFEST: &str = r#"{
  "id": "{ID}",
  "name": "{ID}",
  "version": "0.1.0",
  "entry": "main.js",
  "description": "TODO: 一句话描述这个插件",
  "permissions": {},
  "contributions": {
    "today_cards": [{ "id": "card" }],
    "commands": [{ "id": "hello", "title": "{ID} · 你好" }]
  }
}
"#;

const SCAFFOLD_MAIN: &str = r#"// TODO: 实现你的插件。API 速查见本目录 AGENTS.md 与 docs/PLUGIN_API.md。
export async function onload(api) {
  const h = api.react.createElement;

  // Today 卡片
  api.ui.registerTodayCard({
    id: "card",
    title: "TODO",
    component: (props) =>
      h(
        "div",
        { className: "rounded-xl border border-white/10 bg-[#161a22] px-4 py-3" },
        h("div", { className: "text-xs font-medium text-emerald-300" }, "{ID}"),
        h("div", { className: "mt-1 text-xs text-slate-400" }, "编辑 main.js 后在「插件」页点「重载」即可热更新。"),
      ),
  });

  // ⌘K 命令
  api.ui.registerCommand({
    id: "hello",
    title: "{ID} · 你好",
    handler: () => api.log.info("hello from {ID}"),
  });

  api.log.info("onload 完成");
}

export async function onunload() {}
"#;

const SCAFFOLD_AGENTS: &str = r#"# {ID}（AI 开发说明）

## 契约
- `manifest.json`：id 为反向域名且必须等于目录名；`permissions` 未声明即无权。
- `main.js`：ESM 入口，导出 `onload(api)` / 可选 `onunload()`。

## Plugin API
- `api.react`：宿主共享单实例 React（createElement / hooks）
- `api.core`：today / listTasks / createTask / setTaskStatus / deleteTask / search / addInboxItem / createNote（写操作 actor=plugin:<id> 入审计）
- `api.storage.kv`：get / set / delete / list（按插件命名空间隔离，跨重启持久）
- `api.fetch(url)`：host 必须在 permissions.network 白名单
- `api.events.on(topic, fn)`：领域事件需 permissions.events 声明；panel.refresh/show/hide 豁免
- `api.registerCron(expr, fn)`：expr 需 permissions.cron 声明（Rust 侧驱动，后台不受定时器节流影响）
- `api.ui`：registerTodayCard / registerView / registerCommand
- `api.log.info|warn|error`

## 推荐模式
权威状态存 kv 时间戳（而非组件内存）：重启、托盘后台均一致。组件内 interval 只管渲染。

## 验证
面板「插件」页 → 重载；或 `dashboard plugin dev {ID}` 校验、`dashboard activity --limit 20` 查审计。
"#;

fn write_scaffold(dir: &std::path::Path, id: &str) -> CoreResult<()> {
    let manifest = SCAFFOLD_MANIFEST.replace("{ID}", id);
    let main_js = SCAFFOLD_MAIN.replace("{ID}", id);
    let agents = SCAFFOLD_AGENTS.replace("{ID}", id);
    std::fs::write(dir.join("manifest.json"), manifest)
        .and_then(|_| std::fs::write(dir.join("main.js"), main_js))
        .and_then(|_| std::fs::write(dir.join("AGENTS.md"), agents))
        .map_err(|e| CoreError::Validation(format!("写入脚手架失败: {e}")))?;
    Ok(())
}

/// 接口层输出：人类文本 + JSON 数据。
struct Out {
    text: String,
    data: Value,
}

fn actor() -> Actor {
    std::env::var("DASHBOARD_ACTOR")
        .ok()
        .and_then(|s| Actor::parse(&s))
        .unwrap_or(Actor::Cli)
}

fn code_str(e: &CoreError) -> &'static str {
    match e {
        CoreError::NotFound(_) => "not_found",
        CoreError::Validation(_) => "validation",
        CoreError::Conflict(_) => "conflict",
        CoreError::Storage(_) => "internal",
    }
}

fn exit_code_of(e: &CoreError) -> i32 {
    let c = match e {
        CoreError::NotFound(_) => ExitCode::NotFound,
        CoreError::Validation(_) => ExitCode::UsageError,
        CoreError::Conflict(_) => ExitCode::Conflict,
        CoreError::Storage(_) => ExitCode::GeneralError,
    };
    i32::from(c)
}

fn main() {
    let cli = Cli::parse();
    let json_mode = cli.json;
    match dispatch(cli.command) {
        Ok(out) => {
            if json_mode {
                println!("{}", Envelope::ok(out.data).to_json());
            } else if !out.text.is_empty() {
                println!("{}", out.text);
            }
        }
        Err(e) => {
            if json_mode {
                println!("{}", Envelope::err(code_str(&e), e.to_string()).to_json());
            } else {
                eprintln!("错误: {e}");
            }
            std::process::exit(exit_code_of(&e));
        }
    }
}

fn dispatch(cmd: Commands) -> CoreResult<Out> {
    match cmd {
        Commands::Task { cmd } => task_cmd(cmd),
        Commands::Project { cmd } => project_cmd(cmd),
        Commands::Note { cmd } => note_cmd(cmd),
        Commands::Inbox { cmd } => inbox_cmd(cmd),
        Commands::Search { query } => {
            let conn = util::open_conn()?;
            let results = search_service::search(&conn, &query)?;
            let mut text = format!("共 {} 条结果\n", results.total);
            for h in &results.hits {
                text.push_str(&format!("{:<8} {}\n      {}\n", h.kind, h.id, h.title));
            }
            Ok(Out {
                text,
                data: serde_json::to_value(&results).unwrap_or(Value::Null),
            })
        }
        Commands::Context { cmd } => context_cmd(cmd),
        Commands::Plugin { cmd } => plugin_cmd(cmd),
        Commands::Activity {
            actor: actor_filter,
            limit,
        } => {
            let conn = util::open_conn()?;
            let a = actor_filter.as_deref().and_then(Actor::parse);
            if actor_filter.is_some() && a.is_none() {
                return Err(CoreError::Validation(
                    "actor 仅支持 user/cli/ai/automation/system".into(),
                ));
            }
            let entries = dashboard_core::list_activity(&conn, limit, a)?;
            let mut text = String::new();
            for e in &entries {
                text.push_str(&format!(
                    "{} [{:^10}] {:<22} {} {}\n",
                    e.ts.with_timezone(&chrono::Local).format("%m-%d %H:%M"),
                    e.actor.as_str(),
                    e.action,
                    e.object_type,
                    e.object_id.clone().unwrap_or_default(),
                ));
            }
            Ok(Out {
                text,
                data: serde_json::to_value(&entries).unwrap_or(Value::Null),
            })
        }
        Commands::Status => status_cmd(),
    }
}

// ---------------- Task ----------------

fn task_table(tasks: &[dashboard_domain::Task]) -> String {
    use output::fmt_due;
    let rows: Vec<Vec<String>> = tasks
        .iter()
        .map(|t| {
            vec![
                t.id.clone(),
                t.status.as_str().to_string(),
                fmt_due(t.due_at),
                output::trunc(&t.title, 40),
            ]
        })
        .collect();
    let mut buf = Vec::new();
    // 简单复用：print_table 直接打印 stdout，这里手动拼装
    let header = ["ID", "STATUS", "DUE", "TITLE"];
    let ncols = 4;
    let mut widths = header.iter().map(|h| h.len()).collect::<Vec<_>>();
    for row in &rows {
        for (i, cell) in row.iter().enumerate().take(ncols) {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let head: String = header
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<width$}  ", h, width = widths[i]))
        .collect();
    buf.push(head.trim_end().to_string());
    for row in &rows {
        let line: String = row
            .iter()
            .enumerate()
            .take(ncols)
            .map(|(i, cell)| format!("{:<width$}  ", cell, width = widths[i]))
            .collect();
        buf.push(line.trim_end().to_string());
    }
    buf.join("\n")
}

fn parse_status(s: &str) -> CoreResult<TaskStatus> {
    TaskStatus::parse(s)
        .ok_or_else(|| CoreError::Validation(format!("无效状态 '{s}'（支持 todo/doing/done）")))
}

fn read_stdin_json() -> CoreResult<Value> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| CoreError::Validation(format!("读取 stdin 失败: {e}")))?;
    serde_json::from_str(&buf)
        .map_err(|e| CoreError::Validation(format!("stdin 不是合法 JSON: {e}")))
}

fn task_cmd(cmd: TaskCmd) -> CoreResult<Out> {
    match cmd {
        TaskCmd::List {
            today,
            overdue,
            status,
            project,
            limit,
        } => {
            let conn = util::open_conn()?;
            let mut q = ds::task_repo::TaskQuery::with_limit(limit);
            if today || overdue {
                let (start, end) = context_service::local_today_range(Utc::now());
                q.exclude_done = true;
                q.order_by_due = true;
                if today {
                    q.due_from = Some(start);
                    q.due_to = Some(end);
                } else {
                    q.due_to = Some(start);
                }
            }
            if let Some(st) = status {
                q.status = Some(parse_status(&st)?);
            }
            q.project_id = project;
            let tasks = task_service::list_tasks(&conn, &q)?;
            Ok(Out {
                text: if tasks.is_empty() {
                    "（无任务）".into()
                } else {
                    task_table(&tasks)
                },
                data: json!(tasks),
            })
        }
        TaskCmd::Show { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let t = task_service::get_task(&conn, &tid)?;
            Ok(Out {
                text: format!("[{}] {} （{}）", t.status.as_str(), t.title, t.id),
                data: json!(t),
            })
        }
        TaskCmd::Create {
            title,
            due,
            project,
            stdin,
        } => {
            let conn = util::open_conn()?;
            let (title, due_s, project) = if stdin {
                let v = read_stdin_json()?;
                (
                    v["title"].as_str().map(|s| s.to_string()),
                    v["due"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or(v["due_at"].as_str().map(|s| s.to_string())),
                    v["project"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or(v["project_id"].as_str().map(|s| s.to_string())),
                )
            } else {
                (title, due, project)
            };
            let title = title.ok_or_else(|| {
                CoreError::Validation("缺少 --title（或使用 --stdin 传入 JSON）".into())
            })?;
            let due_at = match due_s.as_deref() {
                Some(s) => Some(util::parse_due(s).map_err(CoreError::Validation)?),
                None => None,
            };
            let input = task_service::CreateTaskInput {
                title,
                due_at,
                project_id: project,
            };
            let t = task_service::create_task(&conn, &input, actor())?;
            Ok(Out {
                text: format!("已创建任务 {}: {}", t.id, t.title),
                data: json!(t),
            })
        }
        TaskCmd::Update {
            id,
            title,
            due,
            clear_due,
            status,
            project,
            clear_project,
        } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let input = task_service::UpdateTaskInput {
                title,
                due_at: if clear_due {
                    Some(None)
                } else {
                    match due.as_deref() {
                        Some(s) => Some(Some(util::parse_due(s).map_err(CoreError::Validation)?)),
                        None => None,
                    }
                },
                status: match status.as_deref() {
                    Some(s) => Some(parse_status(s)?),
                    None => None,
                },
                project_id: if clear_project {
                    Some(None)
                } else {
                    project.map(Some)
                },
            };
            let t = task_service::update_task(&conn, &tid, &input, actor())?;
            Ok(Out {
                text: format!("已更新任务 {}: {}", t.id, t.title),
                data: json!(t),
            })
        }
        TaskCmd::Complete { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let t = task_service::complete_task(&conn, &tid, actor())?;
            Ok(Out {
                text: format!("已完成: {}", t.title),
                data: json!(t),
            })
        }
        TaskCmd::Reopen { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let t = task_service::reopen_task(&conn, &tid, actor())?;
            Ok(Out {
                text: format!("已重新打开: {}", t.title),
                data: json!(t),
            })
        }
        TaskCmd::Delete { id, dry_run } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let report = task_service::delete_task(&conn, &tid, dry_run, actor())?;
            let n = report.affected_ids.len();
            Ok(Out {
                text: if report.dry_run {
                    format!("[dry-run] 将删除 {n} 个任务")
                } else {
                    format!("已删除 {n} 个任务")
                },
                data: serde_json::to_value(&report).unwrap_or(Value::Null),
            })
        }
        TaskCmd::ClearCompleted { dry_run } => {
            let conn = util::open_conn()?;
            let report = task_service::clear_completed_tasks(&conn, dry_run, actor())?;
            let n = report.affected_ids.len();
            Ok(Out {
                text: if report.dry_run {
                    format!("[dry-run] 预计删除 {n} 个已完成任务")
                } else {
                    format!("已清理 {n} 个已完成任务")
                },
                data: serde_json::to_value(&report).unwrap_or(Value::Null),
            })
        }
    }
}

// ---------------- Project / Note / Inbox ----------------

fn project_cmd(cmd: ProjectCmd) -> CoreResult<Out> {
    match cmd {
        ProjectCmd::Create { name, description } => {
            let conn = util::open_conn()?;
            let p = project_service::create_project(&conn, &name, &description, actor())?;
            Ok(Out {
                text: format!("已创建项目 {}: {}", p.id, p.name),
                data: json!(p),
            })
        }
        ProjectCmd::List { include_archived } => {
            let conn = util::open_conn()?;
            let list = project_service::list_projects_with_stats(&conn)?;
            let filtered: Vec<_> = list
                .into_iter()
                .filter(|p| {
                    include_archived || p.project.status == dashboard_domain::ProjectStatus::Active
                })
                .collect();
            let rows: Vec<Vec<String>> = filtered
                .iter()
                .map(|p| {
                    vec![
                        p.project.id.clone(),
                        p.project.status.as_str().to_string(),
                        format!("{}", p.open_tasks),
                        output::trunc(&p.project.name, 30),
                    ]
                })
                .collect();
            let mut text = String::new();
            if rows.is_empty() {
                text.push_str("（无项目）");
            } else {
                text.push_str("ID                             STATUS   OPEN  NAME\n");
                for r in &rows {
                    text.push_str(&format!(
                        "{:<28}  {:<7}  {:<4}  {}\n",
                        r[0], r[1], r[2], r[3]
                    ));
                }
            }
            Ok(Out {
                text,
                data: json!(filtered),
            })
        }
        ProjectCmd::Archive { id } => {
            let conn = util::open_conn()?;
            project_service::archive_project(&conn, &id, actor())?;
            Ok(Out {
                text: format!("已归档项目 {id}"),
                data: json!({"archived": id}),
            })
        }
        ProjectCmd::Delete { id, dry_run } => {
            let conn = util::open_conn()?;
            if dry_run {
                project_service::get_project(&conn, &id)?;
                return Ok(Out {
                    text: "[dry-run] 将删除该项目（其下任务将移出项目）".into(),
                    data: json!({"dry_run": true}),
                });
            }
            project_service::delete_project(&conn, &id, false, actor())?;
            Ok(Out {
                text: format!("已删除项目 {id}"),
                data: Value::Null,
            })
        }
    }
}

fn note_cmd(cmd: NoteCmd) -> CoreResult<Out> {
    match cmd {
        NoteCmd::Create { title, body } => {
            let conn = util::open_conn()?;
            let n = note_service::create_note(&conn, &title, &body, actor())?;
            Ok(Out {
                text: format!("已创建笔记 {}", n.id),
                data: json!(n),
            })
        }
        NoteCmd::List { limit } => {
            let conn = util::open_conn()?;
            let notes = note_service::recent_notes(&conn, limit)?;
            let rows: Vec<Vec<String>> = notes
                .iter()
                .map(|n| {
                    vec![
                        n.id.clone(),
                        output::trunc(
                            if n.title.is_empty() {
                                "(无标题)"
                            } else {
                                &n.title
                            },
                            40,
                        ),
                        output::trunc(n.body.lines().next().unwrap_or(""), 40),
                    ]
                })
                .collect();
            let mut text = String::from("ID                                   TITLE                                    PREVIEW\n");
            for r in &rows {
                text.push_str(&format!("{:<36}  {:<38}  {}\n", r[0], r[1], r[2]));
            }
            Ok(Out {
                text,
                data: json!(notes),
            })
        }
        NoteCmd::Show { id } => {
            let conn = util::open_conn()?;
            let n = note_service::get_note(&conn, &id)?;
            let text = format!(
                "{}\n\n{}",
                if n.title.is_empty() {
                    "(无标题)"
                } else {
                    &n.title
                },
                n.body
            );
            Ok(Out {
                text,
                data: json!(n),
            })
        }
        NoteCmd::Update { id, title, body } => {
            let conn = util::open_conn()?;
            let existing = note_service::get_note(&conn, &id)?;
            let new_title = title.unwrap_or(existing.title);
            let new_body = body.unwrap_or(existing.body);
            let n = note_service::update_note(&conn, &id, &new_title, &new_body, actor())?;
            Ok(Out {
                text: format!("已更新笔记 {}", n.id),
                data: json!(n),
            })
        }
        NoteCmd::Delete { id } => {
            let conn = util::open_conn()?;
            note_service::delete_note(&conn, &id, actor())?;
            Ok(Out {
                text: format!("已删除笔记 {id}"),
                data: Value::Null,
            })
        }
    }
}

fn inbox_cmd(cmd: InboxCmd) -> CoreResult<Out> {
    match cmd {
        InboxCmd::Add { content } => {
            let content = content.join(" ");
            let conn = util::open_conn()?;
            let item = inbox_service::add_item(&conn, &content, "cli", actor())?;
            Ok(Out {
                text: format!("已加入收件箱 {}: {}", item.id, item.content),
                data: json!(item),
            })
        }
        InboxCmd::List { include_processed } => {
            let conn = util::open_conn()?;
            let items = inbox_service::list_items(&conn, include_processed)?;
            let mut text = String::new();
            for i in &items {
                text.push_str(&format!(
                    "{} [{}] {}\n",
                    i.id,
                    i.status.as_str(),
                    output::trunc(&i.content, 60)
                ));
            }
            if items.is_empty() {
                text.push_str("（收件箱为空）");
            }
            Ok(Out {
                text,
                data: json!(items),
            })
        }
        InboxCmd::Task { id } => {
            let conn = util::open_conn()?;
            let r = inbox_service::process_to_task(&conn, &id, None, actor())?;
            Ok(Out {
                text: format!("已转为任务 {}", r.created_id),
                data: serde_json::to_value(&r).unwrap_or(Value::Null),
            })
        }
        InboxCmd::Note { id } => {
            let conn = util::open_conn()?;
            let r = inbox_service::process_to_note(&conn, &id, actor())?;
            Ok(Out {
                text: format!("已转为笔记 {}", r.created_id),
                data: serde_json::to_value(&r).unwrap_or(Value::Null),
            })
        }
        InboxCmd::Delete { id } => {
            let conn = util::open_conn()?;
            inbox_service::delete_item(&conn, &id, actor())?;
            Ok(Out {
                text: format!("已删除 {id}"),
                data: Value::Null,
            })
        }
    }
}

// ---------------- Plugin ----------------

fn plugin_cmd(cmd: PluginCmd) -> CoreResult<Out> {
    match cmd {
        PluginCmd::List => {
            let conn = util::open_conn()?;
            let plugins = plugin_service::list_installed(&conn, &plugin_manifest::plugins_root())?;
            let mut text = String::new();
            if plugins.is_empty() {
                text.push_str("（未发现插件）");
            } else {
                text.push_str("ID                                   STATE         VERSION  NAME\n");
                for p in &plugins {
                    let state = if p.error.is_some() {
                        "invalid".to_string()
                    } else {
                        match p.enabled {
                            Some(true) => "enabled".to_string(),
                            Some(false) => "disabled".to_string(),
                            None => "new".to_string(),
                        }
                    };
                    text.push_str(&format!(
                        "{:<36}  {:<12}  {:<7}  {}\n",
                        p.id,
                        state,
                        p.version,
                        output::trunc(&p.name, 24)
                    ));
                }
            }
            Ok(Out {
                text,
                data: json!(plugins),
            })
        }
        PluginCmd::Enable { id } => {
            let conn = util::open_conn()?;
            plugin_service::set_plugin_enabled(
                &conn,
                &plugin_manifest::plugins_root(),
                &id,
                true,
                actor(),
            )?;
            Ok(Out {
                text: format!("已启用插件 {id}"),
                data: json!({ "id": id, "enabled": true }),
            })
        }
        PluginCmd::Disable { id } => {
            let conn = util::open_conn()?;
            plugin_service::set_plugin_enabled(
                &conn,
                &plugin_manifest::plugins_root(),
                &id,
                false,
                actor(),
            )?;
            Ok(Out {
                text: format!("已停用插件 {id}"),
                data: json!({ "id": id, "enabled": false }),
            })
        }
        PluginCmd::New { id } => {
            plugin_service::validate_plugin_id(&id)?;
            let dir = plugin_manifest::plugins_root().join(&id);
            if dir.exists() {
                return Err(CoreError::Conflict(format!(
                    "插件目录已存在: {}",
                    dir.display()
                )));
            }
            std::fs::create_dir_all(&dir)
                .map_err(|e| CoreError::Validation(format!("创建目录失败: {e}")))?;
            write_scaffold(&dir, &id)?;
            Ok(Out {
                text: format!(
                    "已创建插件脚手架 {}\n下一步：编辑 main.js 实现功能，然后在面板「插件」页启用并重载。",
                    dir.display()
                ),
                data: json!({ "id": id, "dir": dir.display().to_string() }),
            })
        }
        PluginCmd::Dev { id } => {
            let root = plugin_manifest::plugins_root();
            let targets: Vec<String> = match id {
                Some(one) => vec![one],
                None => plugin_manifest::scan_plugins_dir(&root)
                    .iter()
                    .filter_map(|d| d.manifest.as_ref().map(|m| m.id.clone()))
                    .collect(),
            };
            if targets.is_empty() {
                return Err(CoreError::NotFound("plugin（插件目录为空）".into()));
            }
            let mut text = String::new();
            let mut report: Vec<Value> = Vec::new();
            for t in &targets {
                let dir = root.join(t);
                if !dir.is_dir() {
                    return Err(CoreError::NotFound(format!("plugin {t}")));
                }
                let m = plugin_manifest::load_from_dir(&dir)?;
                let entry_bytes = std::fs::metadata(dir.join(&m.entry))
                    .map_err(|e| CoreError::Validation(format!("读取入口文件失败: {e}")))?
                    .len();
                let perms = &m.permissions;
                let contrib = &m.contributions;
                text.push_str(&format!(
                    "✓ {t}  v{}  entry={} ({} B)\n  权限: network={:?} events={:?} cron={:?}\n  贡献: 卡片 {} · 视图 {} · 命令 {}\n",
                    m.version,
                    m.entry,
                    entry_bytes,
                    perms.network,
                    perms.events,
                    perms.cron,
                    contrib.today_cards.len(),
                    contrib.views.len(),
                    contrib.commands.len(),
                ));
                report.push(json!({
                    "id": m.id, "version": m.version, "entry": m.entry,
                    "entry_bytes": entry_bytes,
                    "permissions": { "network": perms.network, "events": perms.events, "cron": perms.cron },
                    "contributions": {
                        "today_cards": contrib.today_cards.len(),
                        "views": contrib.views.len(),
                        "commands": contrib.commands.len(),
                    },
                }));
            }
            Ok(Out {
                text,
                data: json!(report),
            })
        }
    }
}

// ---------------- Context / Status ----------------

fn context_cmd(cmd: ContextCmd) -> CoreResult<Out> {
    match cmd {
        ContextCmd::Today => {
            let conn = util::open_conn()?;
            let ctx = context_service::context_today(&conn)?;
            use std::fmt::Write as _;
            let mut text = String::new();
            let _ = writeln!(text, "== 今天 {} ==", ctx.date);
            let _ = writeln!(
                text,
                "今日任务 {} · 已完成 {} · 逾期 {} · 收件箱 {}",
                ctx.stats.today_total,
                ctx.stats.completed_today,
                ctx.stats.overdue_total,
                ctx.open_inbox_count
            );
            let _ = writeln!(text, "-- 今日待办");
            for t in &ctx.today_tasks {
                let _ = writeln!(text, "- {} [{}]", t.title, t.status.as_str());
            }
            let _ = writeln!(text, "-- 已逾期");
            for t in &ctx.overdue_tasks {
                let _ = writeln!(
                    text,
                    "- {} [{}] due={}",
                    t.title,
                    t.status.as_str(),
                    output::fmt_due(t.due_at)
                );
            }
            let _ = writeln!(text, "-- 活跃项目");
            let _ = writeln!(
                text,
                "{}",
                ctx.active_projects
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let _ = writeln!(text, "-- 最近笔记");
            for n in &ctx.recent_notes {
                let _ = writeln!(
                    text,
                    "- {}",
                    if n.title.is_empty() {
                        "(无标题)"
                    } else {
                        &n.title
                    }
                );
            }
            Ok(Out {
                text,
                data: serde_json::to_value(&ctx).unwrap_or(Value::Null),
            })
        }
    }
}

fn status_cmd() -> CoreResult<Out> {
    let conn = util::open_conn()?;
    let open_tasks = ds::task_repo::count_open(&conn)?;
    let inbox_open = ds::inbox_repo::count_open(&conn)?;
    let projects = ds::project_repo::list(&conn, false)?.len();
    let info = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "db_path": ds::default_db_path().display().to_string(),
        "widget_snapshot_path": dashboard_core::snapshot::snapshot_path().display().to_string(),
        "open_tasks": open_tasks,
        "inbox_open": inbox_open,
        "active_projects": projects,
    });
    Ok(Out {
        text: format!(
            "Dashboard v{}\nDB: {}\n未完成任务 {} · 收件箱 {} · 活跃项目 {}",
            info["version"].as_str().unwrap_or("?"),
            info["db_path"].as_str().unwrap_or("?"),
            open_tasks,
            inbox_open,
            projects
        ),
        data: info,
    })
}
