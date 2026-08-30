//! dashboard-storage
//!
//! Infrastructure 层：SQLite（WAL 模式）+ Repository 实现。
//! 所有 SQL 只允许出现在本 crate 内。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub mod activity_repo;
pub mod completion_repo;
pub mod inbox_repo;
pub mod library_repo;
pub mod note_repo;
pub mod period_repo;
pub mod plugin_repo;
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

/// 对已打开的连接执行迁移（公开给内存库 / 测试场景使用）。
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        conn.execute_batch(SCHEMA_V2)?;
        conn.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        conn.execute_batch(SCHEMA_V3)?;
        conn.pragma_update(None, "user_version", 3)?;
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

/// V2（plugin-system/v1）：插件注册表与插件命名空间 KV；
/// 同时重建 activity_log 放开 actor CHECK（允许 `plugin:<id>`，旧数据原样保留）。
const SCHEMA_V2: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS plugin_registry (
    id           TEXT PRIMARY KEY,
    enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0,1)),
    installed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_kv (
    plugin_id  TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (plugin_id, key)
);
CREATE INDEX IF NOT EXISTS idx_plugin_kv_plugin ON plugin_kv(plugin_id);

CREATE TABLE IF NOT EXISTS activity_log_v2 (
    id          TEXT PRIMARY KEY,
    ts          TEXT NOT NULL,
    actor       TEXT NOT NULL CHECK (actor IN ('user','cli','ai','automation','system') OR actor LIKE 'plugin:%'),
    action      TEXT NOT NULL,
    object_type TEXT NOT NULL,
    object_id   TEXT,
    detail      TEXT NOT NULL DEFAULT '{}'
);
INSERT INTO activity_log_v2 (id, ts, actor, action, object_type, object_id, detail)
    SELECT id, ts, actor, action, object_type, object_id, detail FROM activity_log;
DROP TABLE activity_log;
ALTER TABLE activity_log_v2 RENAME TO activity_log;

COMMIT;
"#;

/// V3（task-checkin-cards）：任务从 todo 改为长期打卡对象。
/// - tasks 表重建：status todo/doing→active、done→archived；删除 due_at/completed_at；
///   新增 icon/color_hex/unit/card_style。旧 due_at/completed_at 数据不回填打卡账本，
///   避免污染迁移日热力图。
/// - 新增打卡账本、目标区间、活动区间、日期主库、归属区间五张表。
/// - 为每个存量任务补默认目标区间（target=1）与活动区间（起点=创建日本地日期）。
const SCHEMA_V3: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS tasks_v3 (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived')),
    icon        TEXT NOT NULL DEFAULT '',
    color_hex   TEXT NOT NULL DEFAULT '#4A90E2',
    unit        TEXT NOT NULL DEFAULT '',
    card_style  TEXT NOT NULL DEFAULT 'day' CHECK (card_style IN ('day','week','month','year')),
    project_id  TEXT REFERENCES projects(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
INSERT INTO tasks_v3 (id, title, status, icon, color_hex, unit, card_style, project_id, created_at, updated_at)
    SELECT id, title,
           CASE WHEN status = 'done' THEN 'archived' ELSE 'active' END,
           '', '#4A90E2', '', 'day', project_id, created_at, updated_at
    FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_v3 RENAME TO tasks;
CREATE INDEX IF NOT EXISTS idx_tasks_status  ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);

CREATE TABLE IF NOT EXISTS completion_records (
    id                     TEXT PRIMARY KEY,
    task_id                TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    operation_id           TEXT NOT NULL UNIQUE,
    value                  INTEGER NOT NULL,
    kind                   TEXT NOT NULL CHECK (kind IN ('add','decrement','undo')),
    compensates_record_id  TEXT REFERENCES completion_records(id),
    logical_day            TEXT NOT NULL,
    source                 TEXT NOT NULL DEFAULT 'user',
    created_at             TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_completion_task_day ON completion_records(task_id, logical_day);

CREATE TABLE IF NOT EXISTS task_target_periods (
    id         TEXT PRIMARY KEY,
    task_id    TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    target     INTEGER NOT NULL CHECK (target BETWEEN 1 AND 999),
    start_day  TEXT NOT NULL,
    end_day    TEXT
);
CREATE INDEX IF NOT EXISTS idx_target_periods_task ON task_target_periods(task_id);

CREATE TABLE IF NOT EXISTS task_activity_periods (
    id         TEXT PRIMARY KEY,
    task_id    TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    start_day  TEXT NOT NULL,
    end_day    TEXT
);
CREATE INDEX IF NOT EXISTS idx_activity_periods_task ON task_activity_periods(task_id);

CREATE TABLE IF NOT EXISTS date_libraries (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    note        TEXT NOT NULL DEFAULT '',
    icon        TEXT NOT NULL DEFAULT '',
    color_hex   TEXT NOT NULL DEFAULT '#4A90E2',
    kind        TEXT NOT NULL CHECK (kind IN ('anniversary','countdown')),
    anchor_day  TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    status      TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_libraries_status ON date_libraries(status);

CREATE TABLE IF NOT EXISTS task_library_membership_periods (
    id          TEXT PRIMARY KEY,
    task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    library_id  TEXT NOT NULL REFERENCES date_libraries(id) ON DELETE CASCADE,
    start_day   TEXT NOT NULL,
    end_day     TEXT
);
CREATE INDEX IF NOT EXISTS idx_membership_task   ON task_library_membership_periods(task_id);
CREATE INDEX IF NOT EXISTS idx_membership_library ON task_library_membership_periods(library_id);

-- 存量任务：补默认目标区间与开放活动区间（起点=创建日的本地日期）
INSERT INTO task_target_periods (id, task_id, target, start_day, end_day)
    SELECT 'tgp_' || lower(hex(randomblob(16))), id, 1, date(created_at, 'localtime'), NULL
    FROM tasks;
INSERT INTO task_activity_periods (id, task_id, start_day, end_day)
    SELECT 'tvp_' || lower(hex(randomblob(16))), id, date(created_at, 'localtime'),
           CASE WHEN status = 'archived' THEN date(updated_at, 'localtime') ELSE NULL END
    FROM tasks;

COMMIT;
"#;
