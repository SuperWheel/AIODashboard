//! dashboard-storage
//!
//! Infrastructure 层：SQLite（WAL 模式）+ Repository 实现。
//! 所有 SQL 只允许出现在本 crate 内。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub mod activity_repo;
pub mod inbox_repo;
pub mod note_repo;
pub mod project_repo;
pub mod task_repo;
mod timeutil;

/// 解析数据库文件路径：
/// 1. 环境变量 `DASHBOARD_DB_PATH`
/// 2. 默认 `~/Library/Application Support/AIODashboard/dashboard.db`
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("DASHBOARD_DB_PATH") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("AIODashboard")
        .join("dashboard.db")
}

/// 打开数据库并启用 WAL，保证 GUI / CLI / 后台任务并发读写。
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        // 目录创建失败时让 Connection::open 自然报错
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    let _: String = conn.query_row("PRAGMA journal_mode=WAL;", [], |r| r.get(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    migrate(&conn)?;
    Ok(conn)
}

/// 打开默认路径的数据库。
pub fn open_default() -> rusqlite::Result<Connection> {
    open(&default_db_path())
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}

const SCHEMA_V1: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','doing','done')),
    due_at       TEXT,
    project_id   TEXT REFERENCES projects(id) ON DELETE SET NULL,
    completed_at TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_status  ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_due     ON tasks(due_at);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);

CREATE TABLE IF NOT EXISTS notes (
    id         TEXT PRIMARY KEY,
    title      TEXT NOT NULL DEFAULT '',
    body       TEXT NOT NULL DEFAULT '',
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inbox_items (
    id         TEXT PRIMARY KEY,
    content    TEXT NOT NULL,
    source     TEXT NOT NULL DEFAULT 'user',
    status     TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','processed')),
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS activity_log (
    id          TEXT PRIMARY KEY,
    ts          TEXT NOT NULL,
    actor       TEXT NOT NULL CHECK (actor IN ('user','cli','ai','automation','system')),
    action      TEXT NOT NULL,
    object_type TEXT NOT NULL,
    object_id   TEXT,
    detail      TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS widget_snapshots (
    kind         TEXT PRIMARY KEY,
    generated_at TEXT NOT NULL,
    payload      TEXT NOT NULL
);

COMMIT;
"#;
