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

use clap::{Parser, Subcommand};
use dashboard_core::{
    checkin_service, context_service, inbox_service, library_service, note_service,
    overview_service, plugin_manifest, plugin_service, project_service, search_service,
    task_service,
};
use dashboard_core::{CoreError, CoreResult};
use dashboard_domain::{Actor, CardStyle, LibraryKind, Recurrence, TaskStatus};
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
    /// 重要日（纪念日 / 倒计时日）
    Library {
        #[command(subcommand)]
        cmd: LibraryCmd,
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
        /// 只看启用中 / 已归档
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
    /// 查看单个任务（含今日打卡状态）
    Show { id: String },
    /// 创建任务（长期打卡对象）
    Create {
        #[arg(short, long)]
        title: Option<String>,
        /// 每日目标次数（1–999）
        #[arg(long)]
        target: Option<i64>,
        /// 计数单位（次/杯/页…）
        #[arg(long)]
        unit: Option<String>,
        /// 图标或 Emoji
        #[arg(long)]
        icon: Option<String>,
        /// 主题色 #RRGGBB（预设 6 色之一或合法 hex）
        #[arg(long)]
        color: Option<String>,
        /// 卡片样式 day|week|month|year
        #[arg(long = "card-style")]
        card_style: Option<String>,
        /// 循环类型 daily|weekly|monthly|yearly|once
        #[arg(long)]
        recurrence: Option<String>,
        /// weekly 循环的星期（1=周一…7=周日，逗号分隔）
        #[arg(long, value_delimiter = ',')]
        weekdays: Option<Vec<u8>>,
        /// 重要性星级（0–5，0=未评级）
        #[arg(long)]
        priority: Option<i64>,
        #[arg(long)]
        project: Option<String>,
        /// 创建时归入的重要日
        #[arg(long)]
        library: Option<String>,
        /// 从 stdin 读取 JSON：{"title":..., "target":..., "unit":...}
        #[arg(long)]
        stdin: bool,
    },
    /// 更新任务（目标/循环修改从明天起生效）
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        target: Option<i64>,
        #[arg(long)]
        unit: Option<String>,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long = "card-style")]
        card_style: Option<String>,
        /// 循环类型 daily|weekly|monthly|yearly|once
        #[arg(long)]
        recurrence: Option<String>,
        /// weekly 循环的星期（1=周一…7=周日，逗号分隔）
        #[arg(long, value_delimiter = ',')]
        weekdays: Option<Vec<u8>>,
        /// 重要性星级（0–5，0=未评级）
        #[arg(long)]
        priority: Option<i64>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long = "clear-project")]
        clear_project: bool,
    },
    /// 打卡 +1（幂等：--operation-id 重放不重复计数）
    Checkin {
        id: String,
        /// 幂等键；缺省自动生成
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
    },
    /// 减少一次（补偿当日最近一次打卡）
    Decrement {
        id: String,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
    },
    /// 撤销最近一次打卡
    Undo {
        id: String,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
    },
    /// 周期总览（周/月/年热力图数据）
    Overview {
        id: String,
        #[arg(long, default_value = "week")]
        period: String,
        /// 锚点逻辑日 YYYY-MM-DD（缺省今天）
        #[arg(long)]
        anchor: Option<String>,
    },
    /// 归档任务（停止打卡，历史保留）
    Archive { id: String },
    /// 恢复已归档任务
    Restore { id: String },
    /// [deprecated] 等价于补满今日目标，请改用 checkin
    Complete { id: String },
    /// [deprecated] 等价于今日清零，请改用 decrement/undo
    Reopen { id: String },
    /// 移动任务到重要日（今日起生效；--clear 移出为独立任务）
    Move {
        id: String,
        #[arg(long)]
        library: Option<String>,
        #[arg(long)]
        clear: bool,
    },
    /// 删除任务
    Delete {
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// 清理全部已归档任务
    ClearArchived {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum LibraryCmd {
    /// 创建重要日
    Create {
        #[arg(short, long)]
        title: Option<String>,
        /// anniversary（纪念日，锚点 ≤ 今天）| countdown（倒计时日，锚点 ≥ 今天）
        #[arg(long)]
        kind: String,
        /// 锚点日 YYYY-MM-DD
        #[arg(long)]
        anchor: String,
        #[arg(long, default_value_t = String::new())]
        note: String,
        #[arg(long, default_value_t = String::new())]
        icon: String,
        #[arg(long)]
        color: Option<String>,
        /// 从 stdin 读取 JSON
        #[arg(long)]
        stdin: bool,
    },
    /// 列出重要日
    List {
        #[arg(long = "all")]
        include_archived: bool,
    },
    /// 查看重要日（含天数、直属任务、年度热力图）
    Show { id: String },
    /// 更新重要日
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        icon: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        anchor: Option<String>,
    },
    /// 归档重要日（必须选择直属任务处置方式）
    Archive {
        id: String,
        /// keep=保留归属 / detach=转独立 / move-to=移到其他重要日
        #[arg(long)]
        mode: String,
        /// mode=move-to 时的目标重要日 id
        #[arg(long)]
        to: Option<String>,
    },
    /// 恢复已归档重要日
    Restore { id: String },
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
    /// 更新项目名称/描述（至少给一项）
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
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
    /// 停用全部插件；用于故障逃生，重启应用后生效
    SafeMode,
    /// 列出插件（磁盘发现 ∪ 注册表状态）
    List,
    /// 启用插件（未注册但磁盘合法的插件会先登记）
    Enable { id: String },
    /// 停用插件
    Disable { id: String },
    /// 创建插件脚手架（默认 JS；可选 TS）
    New {
        id: String,
        /// 模板类型：js 或 ts
        #[arg(long, default_value = "js")]
        template: String,
    },
    /// 校验插件并打印开发信息（manifest / 权限 / 贡献点 / 入口）
    Dev {
        /// 只检查该插件；缺省检查全部已发现插件
        id: Option<String>,
        /// 执行当前可信插件的 npm build/test（先在插件目录 npm install）
        #[arg(long)]
        run: bool,
    },
    /// 将插件目录打包为 zip
    Pack {
        id: String,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// 从本地目录或 zip 安全导入插件（默认停用）
    Install {
        archive: std::path::PathBuf,
        #[arg(long)]
        sha256: Option<String>,
    },
    /// 恢复最近一次安装前的插件版本
    Rollback { id: String },
}

const SCAFFOLD_MANIFEST: &str = r#"{
  "id": "{ID}",
  "name": "{ID}",
  "version": "0.1.0",
  "api_version": "plugin.protocol/v2",
  "min_host_version": "0.1.0",
  "entry": "main.js",
  "description": "TODO: 一句话描述这个插件",
  "permissions": {
    "core": ["task.read"],
    "ui": ["today_card", "command"],
    "storage_quota_bytes": 1048576
  },
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
        { className: "rounded-2xl border border-line bg-surface px-4 py-3" },
        h("div", { className: "text-xs font-medium text-accent" }, "{ID}"),
        h("div", { className: "mt-1 text-xs text-ink2" }, "编辑 main.js 后在「插件」页点「重载」即可热更新。"),
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
- `api.core`：today / listTasks / createTask / checkinTask / archiveTask / deleteTask / search / addInboxItem / createNote（按 `permissions.core` 声明能力，写操作 actor=plugin:<id> 入审计）
- `api.storage.kv`：get / set / delete / list（需声明 storage_quota_bytes，按插件命名空间隔离）
- `api.fetch(url)`：host 必须在 permissions.network 白名单
- `api.events.on(topic, fn)`：领域事件需 permissions.events 声明；panel.refresh/show/hide 豁免
- `api.registerCron(expr, fn)`：expr 需 permissions.cron 声明（Rust 侧驱动，后台不受定时器节流影响）
- `api.ui`：registerTodayCard / registerView / registerCommand / registerSettings（注册返回 disposer）
- `api.log.info|warn|error`
- 新 manifest 应声明 `api_version: plugin.protocol/v2`、`permissions.core` 和 `permissions.ui`。

## 推荐模式
权威状态存 kv 时间戳（而非组件内存）：重启、托盘后台均一致。组件内 interval 只管渲染。

## 验证
先 npm install，再 `dashboard plugin dev {ID} --run` 构建并测试；面板「插件」页审阅权限后启用；`dashboard activity --limit 20` 查审计。
"#;

fn write_scaffold(dir: &std::path::Path, id: &str, template: &str) -> CoreResult<()> {
    if !matches!(template, "js" | "ts") {
        return Err(CoreError::Validation("template 必须是 js 或 ts".into()));
    }
    let write = |name: &str, content: &str| -> CoreResult<()> {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::Validation(e.to_string()))?;
        }
        std::fs::write(path, content).map_err(|e| CoreError::Validation(e.to_string()))
    };
    write("manifest.json", &SCAFFOLD_MANIFEST.replace("{ID}", id))?;
    write("main.js", &SCAFFOLD_MAIN.replace("{ID}", id))?;
    if template == "ts" {
        write(
            "main.ts",
            &format!(
                "import type {{ PluginApi }} from '@aiodashboard/plugin-sdk';\n{}",
                SCAFFOLD_MAIN
                    .replace("{ID}", id)
                    .replace("onload(api)", "onload(api: PluginApi)")
            ),
        )?;
    }
    write("AGENTS.md", &SCAFFOLD_AGENTS.replace("{ID}", id))?;
    write("README.md","# 本地可信插件\n\n先 `npm install`，再 `npm run build` 与 `npm test`。可使用 `dashboard plugin dev <id> --run`。\n未声明能力一律拒绝；修改后在插件页审阅权限再启用。插件与宿主共享 WebView，只运行可信代码。\n")?;
    write("LICENSE", "MIT License\n\nCopyright (c) Plugin author\n\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the Software), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED AS IS, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.\n")?;
    write(
        "vendor/plugin-sdk/src/index.ts",
        include_str!("../../../packages/plugin-sdk/src/index.ts"),
    )?;
    write(
        "vendor/plugin-sdk/src/data.ts",
        include_str!("../../../packages/plugin-sdk/src/data.ts"),
    )?;
    write(
        "vendor/plugin-sdk/package.json",
        include_str!("../../../packages/plugin-sdk/package.json"),
    )?;
    write(
        "vendor/plugin-sdk/tsconfig.json",
        include_str!("../../../packages/plugin-sdk/tsconfig.json"),
    )?;
    write(
        "vendor/plugin-test/src/index.ts",
        include_str!("../../../packages/plugin-test/src/index.ts"),
    )?;
    write(
        "vendor/plugin-test/package.json",
        include_str!("../../../packages/plugin-test/package.json"),
    )?;
    write(
        "vendor/plugin-test/tsconfig.json",
        include_str!("../../../packages/plugin-test/tsconfig.json"),
    )?;
    let entry_build = if template == "ts" {
        "tsc --noEmit -p tsconfig.json && esbuild main.ts --bundle --format=esm --external:react --external:@tauri-apps/* --outfile=main.js"
    } else {
        "node --check main.js"
    };
    write("package.json",&json!({"private":true,"type":"module","scripts":{"build":format!("tsc -p vendor/plugin-sdk && tsc -p vendor/plugin-test && {entry_build}"),"test":"node --test smoke.test.mjs"},"dependencies":{"@aiodashboard/plugin-sdk":"file:vendor/plugin-sdk","@aiodashboard/plugin-test":"file:vendor/plugin-test","react":"^18.3.1"},"devDependencies":{"typescript":"^5.6.2","@types/react":"^18.3.3","esbuild":"^0.25.0"}}).to_string())?;
    write(
        "tsconfig.json",
        r#"{"compilerOptions":{"target":"ES2022","module":"NodeNext","moduleResolution":"NodeNext","strict":true,"skipLibCheck":true,"noEmit":true},"include":["main.ts"]}"#,
    )?;
    write(
        "smoke.test.mjs",
        r#"import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createPluginTestContext} from '@aiodashboard/plugin-test';
import {onload,onunload} from './main.js';
test('load, contributions and disposal',async()=>{
 const manifest=JSON.parse(await readFile(new URL('./manifest.json',import.meta.url),'utf8'));
 const ctx=createPluginTestContext(manifest);
 try {await onload(ctx.api);assert.ok(ctx.commands.size+ctx.cards.size+ctx.views.size>0);}
 finally {ctx.dispose();await onunload?.();}
 assert.equal(ctx.commands.size+ctx.cards.size+ctx.views.size,0);
});
"#,
    )?;
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
        CoreError::PermissionDenied(_) => "permission_denied",
    }
}

fn exit_code_of(e: &CoreError) -> i32 {
    let c = match e {
        CoreError::NotFound(_) => ExitCode::NotFound,
        CoreError::Validation(_) => ExitCode::UsageError,
        CoreError::Conflict(_) => ExitCode::Conflict,
        CoreError::Storage(_) => ExitCode::GeneralError,
        CoreError::PermissionDenied(_) => ExitCode::PermissionDenied,
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
        Commands::Library { cmd } => library_cmd(cmd),
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

// ---------------- Task / Library ----------------

fn task_table(tasks: &[dashboard_domain::Task]) -> String {
    let header = ["ID", "STATUS", "CARD", "TITLE"];
    let rows: Vec<Vec<String>> = tasks
        .iter()
        .map(|t| {
            vec![
                t.id.clone(),
                t.status.as_str().to_string(),
                t.card_style.as_str().to_string(),
                output::trunc(&t.title, 40),
            ]
        })
        .collect();
    let mut widths = header.iter().map(|h| h.len()).collect::<Vec<_>>();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut buf = Vec::new();
    buf.push(
        header
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:<width$}  ", h, width = widths[i]))
            .collect::<String>()
            .trim_end()
            .to_string(),
    );
    for row in &rows {
        buf.push(
            row.iter()
                .enumerate()
                .map(|(i, cell)| format!("{:<width$}  ", cell, width = widths[i]))
                .collect::<String>()
                .trim_end()
                .to_string(),
        );
    }
    buf.join("\n")
}

fn parse_task_status(s: &str) -> CoreResult<TaskStatus> {
    TaskStatus::parse(s)
        .ok_or_else(|| CoreError::Validation(format!("无效状态 '{s}'（支持 active/archived）")))
}

fn parse_card_style(s: &str) -> CoreResult<CardStyle> {
    CardStyle::parse(s)
        .ok_or_else(|| CoreError::Validation(format!("无效卡片样式 '{s}'（day|week|month|year）")))
}

/// 解析循环类型与星期集（004）。CLI 层要求 weekly 显式给出非空 --weekdays。
fn parse_recurrence(kind: &str, weekdays: Option<Vec<u8>>) -> CoreResult<Recurrence> {
    match kind {
        "daily" => Ok(Recurrence::Daily),
        "once" => Ok(Recurrence::Once),
        "monthly" => Ok(Recurrence::Monthly),
        "yearly" => Ok(Recurrence::Yearly),
        "weekly" => {
            let mut ws = weekdays.unwrap_or_default();
            ws.retain(|w| (1..=7).contains(w));
            if ws.is_empty() {
                return Err(CoreError::Validation(
                    "weekly 循环需要 --weekdays（1=周一…7=周日，逗号分隔，如 --weekdays 1,3,5）"
                        .into(),
                ));
            }
            ws.sort_unstable();
            ws.dedup();
            Ok(Recurrence::Weekly { weekdays: ws })
        }
        other => Err(CoreError::Validation(format!(
            "无效循环类型 '{other}'（daily|weekly|monthly|yearly|once）"
        ))),
    }
}

fn parse_library_kind(s: &str) -> CoreResult<LibraryKind> {
    LibraryKind::parse(s).ok_or_else(|| {
        CoreError::Validation(format!("无效重要日类型 '{s}'（anniversary|countdown）"))
    })
}

fn read_stdin_json() -> CoreResult<Value> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| CoreError::Validation(format!("读取 stdin 失败: {e}")))?;
    serde_json::from_str(&buf)
        .map_err(|e| CoreError::Validation(format!("stdin 不是合法 JSON: {e}")))
}

fn gen_op_id() -> String {
    dashboard_domain::new_id("op")
}

fn day_view_text(v: &checkin_service::TaskDayView) -> String {
    let target = v.target.map(|t| t.to_string()).unwrap_or("-".into());
    format!(
        "{} [{}] 今日 {}/{} {}",
        v.task.title, v.state, v.count, target, v.task.unit
    )
}

fn task_cmd(cmd: TaskCmd) -> CoreResult<Out> {
    match cmd {
        TaskCmd::List {
            status,
            project,
            limit,
        } => {
            let conn = util::open_conn()?;
            let mut q = ds::task_repo::TaskQuery::with_limit(limit);
            if let Some(st) = status {
                q.status = Some(parse_task_status(&st)?);
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
            let v = checkin_service::task_day_view(&conn, &tid)?;
            Ok(Out {
                text: day_view_text(&v),
                data: serde_json::to_value(&v).unwrap_or(Value::Null),
            })
        }
        TaskCmd::Create {
            title,
            target,
            unit,
            icon,
            color,
            card_style,
            recurrence,
            weekdays,
            priority,
            project,
            library,
            stdin,
        } => {
            let conn = util::open_conn()?;
            let mut input = task_service::CreateTaskInput::default();
            if stdin {
                let v = read_stdin_json()?;
                input.title = v["title"].as_str().unwrap_or("").to_string();
                if let Some(t) = v["target"].as_i64().or(v["daily_target"].as_i64()) {
                    input.daily_target = t;
                }
                if let Some(s) = v["unit"].as_str() {
                    input.unit = s.to_string();
                }
                if let Some(s) = v["icon"].as_str() {
                    input.icon = s.to_string();
                }
                if let Some(s) = v["color"].as_str().or(v["color_hex"].as_str()) {
                    input.color_hex = s.to_string();
                }
                if let Some(s) = v["card_style"].as_str() {
                    input.card_style = parse_card_style(s)?;
                }
                if let Some(rec) = v["recurrence"].as_object() {
                    let kind = rec.get("kind").and_then(|k| k.as_str()).unwrap_or("daily");
                    let ws = rec.get("weekdays").and_then(|w| w.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_u64().map(|n| n as u8))
                            .collect()
                    });
                    input.recurrence = parse_recurrence(kind, ws)?;
                }
                input.project_id = v["project"]
                    .as_str()
                    .or(v["project_id"].as_str())
                    .map(|s| s.to_string());
                input.library_id = v["library"]
                    .as_str()
                    .or(v["library_id"].as_str())
                    .map(|s| s.to_string());
                if let Some(p) = v["priority"].as_i64() {
                    input.priority = p;
                }
            } else {
                input.title = title.ok_or_else(|| {
                    CoreError::Validation("缺少 --title（或使用 --stdin 传入 JSON）".into())
                })?;
                if let Some(t) = target {
                    input.daily_target = t;
                }
                input.unit = unit.unwrap_or_default();
                input.icon = icon.unwrap_or_default();
                if let Some(c) = color {
                    input.color_hex = c;
                }
                if let Some(s) = card_style {
                    input.card_style = parse_card_style(&s)?;
                }
                if let Some(kind) = &recurrence {
                    input.recurrence = parse_recurrence(kind, weekdays.clone())?;
                }
                if let Some(p) = priority {
                    input.priority = p;
                }
                input.project_id = project;
                input.library_id = library;
            }
            let t = task_service::create_task(&conn, &input, actor())?;
            Ok(Out {
                text: format!("已创建任务 {}: {}", t.id, t.title),
                data: json!(t),
            })
        }
        TaskCmd::Update {
            id,
            title,
            target,
            unit,
            icon,
            color,
            card_style,
            recurrence,
            weekdays,
            priority,
            project,
            clear_project,
        } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let input = task_service::UpdateTaskInput {
                title,
                icon,
                color_hex: color,
                unit,
                daily_target: target,
                card_style: match card_style.as_deref() {
                    Some(s) => Some(parse_card_style(s)?),
                    None => None,
                },
                recurrence: match &recurrence {
                    Some(kind) => Some(parse_recurrence(kind, weekdays.clone())?),
                    None => {
                        if weekdays.is_some() {
                            return Err(CoreError::Validation(
                                "--weekdays 需要与 --recurrence weekly 一起使用".into(),
                            ));
                        }
                        None
                    }
                },
                project_id: if clear_project {
                    Some(None)
                } else {
                    project.map(Some)
                },
                priority,
            };
            let t = task_service::update_task(&conn, &tid, &input, actor())?;
            Ok(Out {
                text: format!("已更新任务 {}: {}", t.id, t.title),
                data: json!(t),
            })
        }
        TaskCmd::Checkin { id, operation_id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let op = operation_id.unwrap_or_else(gen_op_id);
            let v = checkin_service::record(&conn, &tid, &op, actor())?;
            Ok(Out {
                text: day_view_text(&v),
                data: serde_json::to_value(&v).unwrap_or(Value::Null),
            })
        }
        TaskCmd::Decrement { id, operation_id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let op = operation_id.unwrap_or_else(gen_op_id);
            let v = checkin_service::decrement(&conn, &tid, &op, actor())?;
            Ok(Out {
                text: day_view_text(&v),
                data: serde_json::to_value(&v).unwrap_or(Value::Null),
            })
        }
        TaskCmd::Undo { id, operation_id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let op = operation_id.unwrap_or_else(gen_op_id);
            let v = checkin_service::undo(&conn, &tid, &op, actor())?;
            Ok(Out {
                text: day_view_text(&v),
                data: serde_json::to_value(&v).unwrap_or(Value::Null),
            })
        }
        TaskCmd::Overview { id, period, anchor } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let ov =
                overview_service::task_period_overview(&conn, &tid, &period, anchor.as_deref())?;
            let s = &ov.summary;
            Ok(Out {
                text: format!(
                    "{}（{} ~ {}）：完成 {}/{} 天 · 完成率 {:.0}% · 当前连续 {} 天 · 最长连续 {} 天",
                    ov.kind,
                    ov.start_day,
                    ov.end_day,
                    s.complete_day_count,
                    s.applicable_day_count,
                    s.complete_day_rate * 100.0,
                    s.current_streak,
                    s.longest_streak,
                ),
                data: serde_json::to_value(&ov).unwrap_or(Value::Null),
            })
        }
        TaskCmd::Archive { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let t = task_service::archive_task(&conn, &tid, actor())?;
            Ok(Out {
                text: format!("已归档: {}", t.title),
                data: json!(t),
            })
        }
        TaskCmd::Restore { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let t = task_service::restore_task(&conn, &tid, actor())?;
            Ok(Out {
                text: format!("已恢复: {}", t.title),
                data: json!(t),
            })
        }
        TaskCmd::Complete { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let v = checkin_service::complete_today(&conn, &tid, actor())?;
            Ok(Out {
                text: format!(
                    "已补满今日目标: {}（complete 已废弃，请改用 checkin）",
                    v.task.title
                ),
                data: json!({
                    "deprecated": true,
                    "replacement": "task checkin",
                    "task_day": v,
                }),
            })
        }
        TaskCmd::Reopen { id } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let v = checkin_service::reopen_today(&conn, &tid, actor())?;
            Ok(Out {
                text: format!(
                    "已清零今日打卡: {}（reopen 已废弃，请改用 decrement/undo）",
                    v.task.title
                ),
                data: json!({
                    "deprecated": true,
                    "replacement": "task decrement / task undo",
                    "task_day": v,
                }),
            })
        }
        TaskCmd::Move { id, library, clear } => {
            let conn = util::open_conn()?;
            let tid = util::resolve_task_id(&conn, &id)?;
            let target = if clear {
                None
            } else {
                Some(util::resolve_library_id(
                    &conn,
                    library.as_deref().ok_or_else(|| {
                        CoreError::Validation("需要 --library <id> 或 --clear".into())
                    })?,
                )?)
            };
            let t = library_service::move_task(&conn, &tid, target.as_deref(), actor())?;
            Ok(Out {
                text: format!(
                    "已移动任务 {} → {}",
                    t.title,
                    target.as_deref().unwrap_or("（独立）")
                ),
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
        TaskCmd::ClearArchived { dry_run } => {
            let conn = util::open_conn()?;
            let report = task_service::clear_archived_tasks(&conn, dry_run, actor())?;
            let n = report.affected_ids.len();
            Ok(Out {
                text: if report.dry_run {
                    format!("[dry-run] 预计删除 {n} 个已归档任务")
                } else {
                    format!("已清理 {n} 个已归档任务")
                },
                data: serde_json::to_value(&report).unwrap_or(Value::Null),
            })
        }
    }
}

fn library_cmd(cmd: LibraryCmd) -> CoreResult<Out> {
    match cmd {
        LibraryCmd::Create {
            title,
            kind,
            anchor,
            note,
            icon,
            color,
            stdin,
        } => {
            let conn = util::open_conn()?;
            let mut input = library_service::CreateLibraryInput {
                title: String::new(),
                note,
                icon,
                color_hex: color.unwrap_or_else(|| "#4A90E2".into()),
                kind: parse_library_kind(&kind)?,
                anchor_day: anchor,
            };
            if stdin {
                let v = read_stdin_json()?;
                input.title = v["title"].as_str().unwrap_or("").to_string();
                input.kind = parse_library_kind(v["kind"].as_str().unwrap_or(""))?;
                input.anchor_day = v["anchor"]
                    .as_str()
                    .or(v["anchor_day"].as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(s) = v["note"].as_str() {
                    input.note = s.to_string();
                }
                if let Some(s) = v["icon"].as_str() {
                    input.icon = s.to_string();
                }
                if let Some(s) = v["color"].as_str().or(v["color_hex"].as_str()) {
                    input.color_hex = s.to_string();
                }
            } else {
                input.title = title.ok_or_else(|| {
                    CoreError::Validation("缺少 --title（或使用 --stdin 传入 JSON）".into())
                })?;
            }
            let lib = library_service::create_library(&conn, &input, actor())?;
            Ok(Out {
                text: format!("已创建重要日 {}: {}", lib.id, lib.title),
                data: json!(lib),
            })
        }
        LibraryCmd::List { include_archived } => {
            let conn = util::open_conn()?;
            let items = library_service::list_library_items(&conn, include_archived)?;
            let mut text = String::new();
            if items.is_empty() {
                text.push_str("（无重要日）");
            } else {
                for it in &items {
                    let days = match it.day_info.display_kind.as_str() {
                        "day_n" => format!("第 {} 天", it.day_info.day_count),
                        "remaining" => format!("还剩 {} 天", it.day_info.day_count),
                        "today" => "就是今天".to_string(),
                        _ => format!("已逾期 {} 天", it.day_info.day_count),
                    };
                    text.push_str(&format!(
                        "{} [{}|{}] {} · {} · {} 个任务\n",
                        it.library.id,
                        it.library.kind.as_str(),
                        it.library.status.as_str(),
                        it.library.title,
                        days,
                        it.task_count,
                    ));
                }
            }
            Ok(Out {
                text: text.trim_end().to_string(),
                data: json!(items),
            })
        }
        LibraryCmd::Show { id } => {
            let conn = util::open_conn()?;
            let lid = util::resolve_library_id(&conn, &id)?;
            let lib = library_service::get_library(&conn, &lid)?;
            let today = context_service::local_today();
            let info = library_service::day_info(&lib, &today)?;
            let tasks = library_service::library_tasks(&conn, &lid)?;
            let heatmap = overview_service::library_year_heatmap(&conn, &lid, None)?;
            let text = format!(
                "{} [{}]\n锚点 {} · 任务 {} 个 · 热力图 {} 天",
                lib.title,
                info.display_kind,
                lib.anchor_day,
                tasks.len(),
                heatmap.days.len(),
            );
            Ok(Out {
                text,
                data: json!({
                    "library": lib,
                    "day_info": info,
                    "tasks": tasks,
                    "year_heatmap": heatmap,
                }),
            })
        }
        LibraryCmd::Update {
            id,
            title,
            note,
            icon,
            color,
            anchor,
        } => {
            let conn = util::open_conn()?;
            let lid = util::resolve_library_id(&conn, &id)?;
            let input = library_service::UpdateLibraryInput {
                title,
                note,
                icon,
                color_hex: color,
                anchor_day: anchor,
            };
            let lib = library_service::update_library(&conn, &lid, &input, actor())?;
            Ok(Out {
                text: format!("已更新重要日 {}: {}", lib.id, lib.title),
                data: json!(lib),
            })
        }
        LibraryCmd::Archive { id, mode, to } => {
            let conn = util::open_conn()?;
            let lid = util::resolve_library_id(&conn, &id)?;
            let m = match mode.as_str() {
                "keep" => library_service::ArchiveTaskMode::Keep,
                "detach" => library_service::ArchiveTaskMode::Detach,
                "move-to" | "move_to" => library_service::ArchiveTaskMode::MoveTo,
                _ => {
                    return Err(CoreError::Validation(
                        "无效 mode（keep|detach|move-to）".into(),
                    ))
                }
            };
            let to_id = match to.as_deref() {
                Some(t) => Some(util::resolve_library_id(&conn, t)?),
                None => None,
            };
            let lib = library_service::archive_library(&conn, &lid, m, to_id.as_deref(), actor())?;
            Ok(Out {
                text: format!("已归档重要日 {}: {}", lib.id, lib.title),
                data: json!(lib),
            })
        }
        LibraryCmd::Restore { id } => {
            let conn = util::open_conn()?;
            let lid = util::resolve_library_id(&conn, &id)?;
            let lib = library_service::restore_library(&conn, &lid, actor())?;
            Ok(Out {
                text: format!("已恢复重要日 {}: {}", lib.id, lib.title),
                data: json!(lib),
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
        ProjectCmd::Update {
            id,
            name,
            description,
        } => {
            if name.is_none() && description.is_none() {
                return Err(dashboard_core::CoreError::Validation(
                    "至少提供 --name 或 --description 之一".into(),
                ));
            }
            let conn = util::open_conn()?;
            let p = project_service::update_project(
                &conn,
                &id,
                name.as_deref(),
                description.as_deref(),
                actor(),
            )?;
            Ok(Out {
                text: format!("已更新项目 {}: {}", p.id, p.name),
                data: json!(p),
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
            let r = inbox_service::process_to_task(&conn, &id, actor())?;
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
        PluginCmd::New { id, template } => {
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
            write_scaffold(&dir, &id, &template)?;
            Ok(Out {
                text: format!(
                    "已创建插件脚手架 {}\n下一步：编辑 main.js 实现功能，然后在面板「插件」页启用并重载。",
                    dir.display()
                ),
                data: json!({ "id": id, "dir": dir.display().to_string() }),
            })
        }
        PluginCmd::Dev { id, run } => {
            let root = plugin_manifest::plugins_root();
            let targets: Vec<String> = match id {
                Some(one) => vec![one],
                None => {
                    let scanned = plugin_manifest::scan_plugins_dir(&root);
                    if let Some(d) = scanned.iter().find(|d| d.error.is_some()) {
                        return Err(CoreError::Validation(d.error.clone().unwrap_or_default()));
                    }
                    scanned
                        .into_iter()
                        .filter_map(|d| d.manifest.map(|m| m.id))
                        .collect()
                }
            };
            if targets.is_empty() {
                return Err(CoreError::NotFound("plugin（插件目录为空）".into()));
            }
            let mut text = String::new();
            let mut report: Vec<Value> = Vec::new();
            for t in &targets {
                plugin_service::validate_plugin_id(t)?;
                let dir = root.join(t);
                if !dir.is_dir() {
                    return Err(CoreError::NotFound(format!("plugin {t}")));
                }
                if run {
                    for script in ["build", "test"] {
                        let out = std::process::Command::new("npm")
                            .args(["run", script])
                            .current_dir(&dir)
                            .output()
                            .map_err(|e| CoreError::Validation(e.to_string()))?;
                        if !out.status.success() {
                            return Err(CoreError::Validation(format!(
                                "npm {script} 失败: {} {}",
                                String::from_utf8_lossy(&out.stdout),
                                String::from_utf8_lossy(&out.stderr)
                            )));
                        }
                    }
                }
                let m = plugin_manifest::load_from_dir(&dir)?;
                plugin_manifest::require_current(&m)?;
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
                    "permissions": perms, "build_and_tests_run":run,
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
        PluginCmd::Pack { id, output } => {
            let conn = util::open_conn()?;
            let root = plugin_manifest::plugins_root();
            let out = output.unwrap_or_else(|| std::path::PathBuf::from(format!("{id}.zip")));
            let data = dashboard_core::plugin_package::pack(&conn, &root, &id, &out)?;
            Ok(Out {
                text: format!("已打包 {}", data["archive"]),
                data,
            })
        }
        PluginCmd::Install { archive, sha256 } => {
            let conn = util::open_conn()?;
            let result = dashboard_core::plugin_package::install(
                &conn,
                &plugin_manifest::plugins_root(),
                &archive,
                sha256.as_deref(),
            )?;
            Ok(Out {
                text: format!("已安装 {}（停用）；审阅权限后使用 plugin enable", result.id),
                data: json!(result),
            })
        }
        PluginCmd::Rollback { id } => {
            let conn = util::open_conn()?;
            let result = dashboard_core::plugin_package::rollback(
                &conn,
                &plugin_manifest::plugins_root(),
                &id,
            )?;
            Ok(Out {
                text: format!("已回滚 {id}（停用）；审阅权限后再启用"),
                data: json!(result),
            })
        }
        PluginCmd::SafeMode => {
            let conn = util::open_conn()?;
            plugin_service::disable_all(&conn, &plugin_manifest::plugins_root(), actor())?;
            Ok(Out {
                text: "已停用全部插件。若窗口被同步死循环阻塞，退出并重启应用。".into(),
                data: json!({"safe_mode":true}),
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
                "今日任务 {} · 已达标 {} · 完成率 {:.0}% · 近7天错过 {} · 收件箱 {}",
                ctx.stats.task_total,
                ctx.stats.completed_today,
                ctx.stats.completion_rate * 100.0,
                ctx.stats.missed_last_7d,
                ctx.open_inbox_count
            );
            let _ = writeln!(text, "-- 今日任务");
            for v in &ctx.today_tasks {
                let target = v.target.map(|t| t.to_string()).unwrap_or("-".into());
                let _ = writeln!(
                    text,
                    "- {} [{}/{} {}] {}",
                    v.task.title, v.count, target, v.task.unit, v.state
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
    let open_tasks = ds::task_repo::count_active(&conn)?;
    let inbox_open = ds::inbox_repo::count_open(&conn)?;
    let projects = ds::project_repo::list(&conn, false)?.len();
    let info = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "db_path": ds::default_db_path().display().to_string(),
        "widget_snapshot_path": dashboard_core::snapshot::snapshot_path().display().to_string(),
        "active_tasks": open_tasks,
        "inbox_open": inbox_open,
        "active_projects": projects,
    });
    Ok(Out {
        text: format!(
            "Dashboard v{}\nDB: {}\n启用任务 {} · 收件箱 {} · 活跃项目 {}",
            info["version"].as_str().unwrap_or("?"),
            info["db_path"].as_str().unwrap_or("?"),
            open_tasks,
            inbox_open,
            projects
        ),
        data: info,
    })
}
