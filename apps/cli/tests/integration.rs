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
    plugins_dir: PathBuf,
}

impl Env {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let db_path = dir.path().join("dashboard.db");
        let snap_path = dir.path().join("widget-snapshot.json");
        let plugins_dir = dir.path().join("plugins");
        Self {
            _dir: dir,
            db_path,
            snap_path,
            plugins_dir,
        }
    }

    fn cli(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_dashboard"))
            .args(args)
            .env("DASHBOARD_DB_PATH", &self.db_path)
            .env("DASHBOARD_WIDGET_SNAPSHOT_PATH", &self.snap_path)
            .env("DASHBOARD_PLUGINS_DIR", &self.plugins_dir)
            .output()
            .expect("run cli");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).to_string(),
        )
    }

    /// 在测试插件目录里写一个插件（manifest.json + main.js）。
    fn write_plugin(&self, dir_name: &str, manifest_json: &str, entry_js: &str) {
        let p = self.plugins_dir.join(dir_name);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("manifest.json"), manifest_json).unwrap();
        std::fs::write(p.join("main.js"), entry_js).unwrap();
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
            ..Default::default()
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
    assert_eq!(v["meta"]["schema_version"], "3");
    let items = v["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], task.id.as_str());
    assert_eq!(items[0]["title"], "GUI 建的任务");
    // 协议字段稳定性抽查
    assert!(items[0].get("status").is_some());
}

#[test]
fn chain_checkin_then_core_read() {
    let env = Env::new();
    let task = env.core_create_task("打卡任务");

    // CLI 打卡（模拟 AI 通过 CLI 操作），幂等键重放不重复计数
    let (code, _) = env.cli(&["task", "checkin", &task.id, "--operation-id", "op_test_1"]);
    assert_eq!(code, 0);
    let (code, _) = env.cli(&["task", "checkin", &task.id, "--operation-id", "op_test_1"]);
    assert_eq!(code, 0);

    // GUI（Core）视角看到今日 count=1 且已完成（默认目标 1）
    let conn = env.core_conn();
    let v = dashboard_core::checkin_service::task_day_view(&conn, &task.id).unwrap();
    assert_eq!(v.count, 1);
    assert_eq!(v.state, "completed");
}

#[test]
fn chain_complete_compat_deprecated() {
    let env = Env::new();
    let task = env.core_create_task("兼容任务");
    let (code, out) = env.cli(&["task", "complete", &task.id, "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["deprecated"], true);
    assert_eq!(v["data"]["task_day"]["state"], "completed");
}

#[test]
fn chain_library_create_move_archive() {
    let env = Env::new();
    let today = Env::today_str();
    // 创建纪念日重要日
    let (code, out) = env.cli(&[
        "library",
        "create",
        "--title",
        "健身年",
        "--kind",
        "anniversary",
        "--anchor",
        &today,
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let lib_id = v["data"]["id"].as_str().unwrap().to_string();
    assert!(lib_id.starts_with("dlb_"));

    // 建任务并移入
    let (code, out) = env.cli(&[
        "task", "create", "--title", "深蹲", "--target", "3", "--unit", "组", "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let task_id = v["data"]["id"].as_str().unwrap().to_string();
    let (code, out) = env.cli(&["task", "move", &task_id, "--library", &lib_id, "--json"]);
    assert_eq!(code, 0, "{out}");

    // 重要日 show：直属任务 1 个
    let (code, out) = env.cli(&["library", "show", &lib_id, "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"]["day_info"]["day_count"], 1);

    // 归档 detach
    let (code, out) = env.cli(&["library", "archive", &lib_id, "--mode", "detach", "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["status"], "archived");

    // 任务已转独立
    let conn = env.core_conn();
    let view = dashboard_core::checkin_service::task_day_view(&conn, &task_id).unwrap();
    assert!(view.library_id.is_none());
}

#[test]
fn chain_project_update_roundtrip() {
    let env = Env::new();
    let (code, out) = env.cli(&[
        "project",
        "create",
        "--name",
        "旧名字",
        "--description",
        "旧描述",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let id = v["data"]["id"].as_str().unwrap().to_string();

    // 重命名 + 改描述
    let (code, out) = env.cli(&[
        "project",
        "update",
        &id,
        "--name",
        "新名字",
        "--description",
        "新描述",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["name"], "新名字");
    assert_eq!(v["data"]["description"], "新描述");

    // GUI（Core）视角读到新名字，且审计有 project.update
    let conn = env.core_conn();
    let p = dashboard_core::project_service::get_project(&conn, &id).unwrap();
    assert_eq!(p.name, "新名字");
    assert_eq!(p.description, "新描述");

    // 参数校验：空名字 / 不给任何字段 → exit code 2
    let (code, _) = env.cli(&["project", "update", &id, "--name", "  "]);
    assert_eq!(code, 2);
    let (code, _) = env.cli(&["project", "update", &id]);
    assert_eq!(code, 2);
}

#[test]
fn ai_error_protocol_not_found_exit_code_3() {
    let env = Env::new();
    let (code, out) = env.cli(&["task", "show", "tsk_does_not_exist", "--json"]);
    assert_eq!(code, 3);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["error"]["code"], "not_found");
    assert_eq!(v["meta"]["schema_version"], "3");
}

#[test]
fn context_today_envelope_and_snapshot_file() {
    let env = Env::new();
    let today = Env::today_str();
    let _ = today; // 日期边界由 core 单测覆盖，这里只验证链路
    let (code, out) = env.cli(&["task", "create", "--title", "今日任务", "--json"]);
    assert_eq!(code, 0, "{out}");

    let (code, out) = env.cli(&["context", "today", "--json"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    assert!(v["data"]["stats"]["task_total"].as_i64().unwrap() >= 1);

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

// ---------------- 插件链路（plugin-system/v1）----------------

/// T4：list 展示磁盘新发现插件（enabled=null）→ enable 自动登记 → disable → 审计齐全。
#[test]
fn plugin_list_enable_disable_roundtrip() {
    let env = Env::new();
    env.write_plugin(
        "com.test.echo",
        r#"{"id":"com.test.echo","name":"Echo","version":"0.1.0","entry":"main.js"}"#,
        "export function onload() {}",
    );

    // 新发现：已列出但未注册
    let (code, out) = env.cli(&["plugin", "list", "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true);
    let items = v["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "com.test.echo");
    assert!(items[0]["enabled"].is_null());

    // enable：未注册但磁盘合法 → 自动登记并启用
    let (code, out) = env.cli(&["plugin", "enable", "com.test.echo", "--json"]);
    assert_eq!(code, 0, "{out}");

    let (code, out) = env.cli(&["plugin", "list", "--json"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"][0]["enabled"], true);

    // disable
    let (code, out) = env.cli(&["plugin", "disable", "com.test.echo", "--json"]);
    assert_eq!(code, 0, "{out}");
    let conn = env.core_conn();
    let reg = dashboard_core::plugin_service::get_registration(&conn, "com.test.echo").unwrap();
    assert!(!reg.enabled);

    // 审计：register / enable / disable 三条齐全
    let (code, out) = env.cli(&["activity", "--json", "--limit", "50"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("plugin.registered"), "{out}");
    assert!(out.contains("plugin.enable"), "{out}");
    assert!(out.contains("plugin.disable"), "{out}");
}

/// T4：未知插件 id → exit 3 + not_found 信封。
#[test]
fn plugin_unknown_id_exit_code_3() {
    let env = Env::new();
    let (code, out) = env.cli(&["plugin", "disable", "com.missing.thing", "--json"]);
    assert_eq!(code, 3, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["error"]["code"], "not_found");
    assert_eq!(v["meta"]["schema_version"], "3");
}

/// T4：manifest 非法的目录在 list 中以 error 呈现，不影响整体。
#[test]
fn plugin_list_reports_invalid_manifest() {
    let env = Env::new();
    env.write_plugin("com.test.bad", "{{broken", "// noop");

    let (code, out) = env.cli(&["plugin", "list", "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let items = v["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0]["error"].is_string());

    // 非法 manifest 不能通过 enable 混进注册表
    let (code, _) = env.cli(&["plugin", "enable", "com.test.bad", "--json"]);
    assert_eq!(code, 2);
}

/// T4：插件以 Actor::Plugin 调用 core 时审计形态为 plugin:<id>。
#[test]
fn plugin_actor_recorded_in_activity() {
    let env = Env::new();
    let conn = env.core_conn();
    let input = dashboard_core::task_service::CreateTaskInput {
        title: "插件创建的任务".into(),
        ..Default::default()
    };
    let task = dashboard_core::task_service::create_task(
        &conn,
        &input,
        dashboard_domain::Actor::Plugin("com.test.echo".into()),
    )
    .unwrap();
    drop(conn);

    let (code, out) = env.cli(&["activity", "--json", "--limit", "10"]);
    assert_eq!(code, 0);
    assert!(out.contains("plugin:com.test.echo"), "{out}");
    assert_eq!(task.title, "插件创建的任务");
}

/// P3：plugin new 脚手架 → dev 校验 → 重复创建冲突。
#[test]
fn plugin_new_and_dev_roundtrip() {
    let env = Env::new();

    // new：生成脚手架
    let (code, out) = env.cli(&["plugin", "new", "com.test.scaffold", "--json"]);
    assert_eq!(code, 0, "{out}");
    let dir = env.plugins_dir.join("com.test.scaffold");
    assert!(dir.join("manifest.json").is_file());
    assert!(dir.join("main.js").is_file());
    assert!(dir.join("AGENTS.md").is_file());

    // dev：校验通过并输出贡献点统计
    let (code, out) = env.cli(&["plugin", "dev", "com.test.scaffold", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("today_cards"), "{out}");

    // 重复 new → 冲突（exit 5）
    let (code, out) = env.cli(&["plugin", "new", "com.test.scaffold", "--json"]);
    assert_eq!(code, 5, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["code"], "conflict");

    // dev 指向不存在的插件 → exit 3
    let (code, _) = env.cli(&["plugin", "dev", "com.test.missing", "--json"]);
    assert_eq!(code, 3);
}

#[test]
fn chain_task_recurrence() {
    let env = Env::new();
    let today = Env::today_str();
    let wd = {
        use chrono::Datelike;
        let d = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();
        d.weekday().number_from_monday() // ISO 1..7
    };
    let off_day = (wd % 7) + 1; // 必不等于今天

    // 1) weekly 含今天：创建即 JSON 带 recurrence，schema_version=3，打卡成功
    let (code, out) = env.cli(&[
        "task",
        "create",
        "--title",
        "每周打卡",
        "--recurrence",
        "weekly",
        "--weekdays",
        &wd.to_string(),
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["recurrence"]["kind"], "weekly");
    assert_eq!(v["meta"]["schema_version"], "3");
    let id_on = v["data"]["id"].as_str().unwrap().to_string();
    let (code, out) = env.cli(&["task", "checkin", &id_on, "--json"]);
    assert_eq!(code, 0, "{out}");

    // 2) weekly 不含今天：Today 列表排除，checkin 拒绝（Validation → exit 2）
    let (code, out) = env.cli(&[
        "task",
        "create",
        "--title",
        "周外打卡",
        "--recurrence",
        "weekly",
        "--weekdays",
        &off_day.to_string(),
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let id_off = v["data"]["id"].as_str().unwrap().to_string();
    let (code, out) = env.cli(&["task", "checkin", &id_off, "--json"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("今天不适用"));
    let (code, out) = env.cli(&["context", "today", "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let in_today = v["data"]["today_tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["task"]["id"] == id_off.as_str());
    assert!(!in_today);

    // 3) 一次性任务：达标即自动归档，历史保留
    let (code, out) = env.cli(&[
        "task",
        "create",
        "--title",
        "一次性事项",
        "--recurrence",
        "once",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    let id_once = v["data"]["id"].as_str().unwrap().to_string();
    let (code, out) = env.cli(&["task", "checkin", &id_once, "--json"]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = env.cli(&["task", "show", &id_once, "--json"]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["task"]["status"], "archived");
    assert_eq!(v["data"]["count"], 1);

    // 4) 非法循环参数：weekly 缺 --weekdays → exit 2
    let (code, _) = env.cli(&[
        "task",
        "create",
        "--title",
        "坏参数",
        "--recurrence",
        "weekly",
        "--json",
    ]);
    assert_eq!(code, 2);
    let (code, _) = env.cli(&[
        "task",
        "create",
        "--title",
        "坏参数",
        "--recurrence",
        "nope",
        "--json",
    ]);
    assert_eq!(code, 2);
}
