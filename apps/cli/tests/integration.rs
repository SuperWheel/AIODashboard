//! 端到端链路验证（对应设计文档 §31）：
//!
//! 链路一：CLI 创建 Task → SQLite → Core 能读取（= GUI 视角）
//! 链路二：Core/GUI 创建 Task → SQLite → CLI --json 能读取
//! 链路三：AI 视角：--json 信封稳定、exit code 正确
//!
//! 运行方式：`cargo test -p dashboard-cli`

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

struct Env {
    _dir: TempDir,
    db_path: PathBuf,
    snap_path: PathBuf,
}

impl Env {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let db_path = dir.path().join("dashboard.db");
        let snap_path = dir.path().join("widget-snapshot.json");
        Self {
            _dir: dir,
            db_path,
            snap_path,
        }
    }

    fn cli(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_dashboard"))
            .args(args)
            .env("DASHBOARD_DB_PATH", &self.db_path)
            .env("DASHBOARD_WIDGET_SNAPSHOT_PATH", &self.snap_path)
            .output()
            .expect("run cli");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
        )
    }

    /// 模拟 GUI / AI 直接调用 Core（与 Tauri Command 完全相同的代码路径）。
    /// 注意使用显式路径，避免依赖进程级环境变量（测试并行安全）。
    fn core_conn(&self) -> rusqlite::Connection {
        dashboard_storage::open(&self.db_path).unwrap()
    }

    fn core_create_task(&self, title: &str) -> dashboard_domain::Task {
        let conn = self.core_conn();
        let input = dashboard_core::task_service::CreateTaskInput {
            title: title.to_string(),
            due_at: None,
            project_id: None,
        };
        dashboard_core::task_service::create_task(&conn, &input, dashboard_domain::Actor::User)
            .unwrap()
    }

    fn today_str() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
}

#[test]
fn chain_cli_create_then_core_read() {
    let env = Env::new();
    let (code, out) = env.cli(&["task", "create", "--title", "CLI 建的任务", "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    let id = v["data"]["id"].as_str().unwrap().to_string();

    // GUI（Core）视角能读到
    let conn = env.core_conn();
    let q = dashboard_storage::task_repo::TaskQuery::with_limit(100);
    let tasks = dashboard_core::task_service::list_tasks(&conn, &q).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].title, "CLI 建的任务");
}

#[test]
fn chain_core_create_then_cli_read_json() {
    let env = Env::new();
    let task = env.core_create_task("GUI 建的任务");

    let (code, out) = env.cli(&["task", "list", "--json"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["meta"]["schema_version"], "1");
    let items = v["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], task.id.as_str());
    assert_eq!(items[0]["title"], "GUI 建的任务");
    // 协议字段稳定性抽查
    assert!(items[0].get("status").is_some());
}

#[test]
fn chain_complete_and_status_sync() {
    let env = Env::new();
    let task = env.core_create_task("待完成任务");

    // CLI 完成（模拟 AI 通过 CLI 操作）
    let (code, _) = env.cli(&["task", "complete", &task.id]);
    assert_eq!(code, 0);

    // GUI（Core）视角看到 done
    let conn = env.core_conn();
    let t = dashboard_core::task_service::get_task(&conn, &task.id).unwrap();
    assert_eq!(t.status, dashboard_domain::TaskStatus::Done);
    assert!(t.completed_at.is_some());
}

#[test]
fn ai_error_protocol_not_found_exit_code_3() {
    let env = Env::new();
    let (code, out) = env.cli(&["task", "show", "tsk_does_not_exist", "--json"]);
    assert_eq!(code, 3);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["error"]["code"], "not_found");
    assert_eq!(v["meta"]["schema_version"], "1");
}

#[test]
fn context_today_envelope_and_snapshot_file() {
    let env = Env::new();
    let today = Env::today_str();
    let (code, out) = env.cli(&["task", "create", "--title", "今日任务", "--due", &today]);
    assert_eq!(code, 0, "{out}");

    let (code, out) = env.cli(&["context", "today", "--json"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    assert!(v["data"]["stats"]["today_total"].as_i64().unwrap() >= 1);

    // 任务变更后 Widget Snapshot 文件已生成
    let raw = std::fs::read_to_string(&env.snap_path).unwrap();
    let s: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(s["schema"], "widget.snapshot/v1");
    assert!(s["today"]["task_total"].as_i64().unwrap() >= 1);
}

#[test]
fn dry_run_reports_without_deleting() {
    let env = Env::new();
    let task = env.core_create_task("不要真的删我");

    let (code, out) = env.cli(&["task", "delete", &task.id, "--dry-run"]);
    assert_eq!(code, 0);
    assert!(out.contains("dry-run") || out.contains("将删除"));

    let conn = env.core_conn();
    assert!(dashboard_core::task_service::get_task(&conn, &task.id).is_ok());
}
