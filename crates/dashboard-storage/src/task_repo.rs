//! Task Repository

use chrono::Utc;
use dashboard_domain::{id_prefix, new_id, CardStyle, Task, TaskStatus};
use rusqlite::{
    params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension, Row,
};

use crate::timeutil::{read_time, write_time};

pub const TASK_COLS: &str =
    "id, title, status, icon, color_hex, unit, card_style, project_id, created_at, updated_at";

fn invalid(col: usize, what: &str, raw: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        col,
        rusqlite::types::Type::Text,
        format!("invalid {what}: {raw}").into(),
    )
}

fn map_task(row: &Row) -> rusqlite::Result<Task> {
    let status_raw: String = row.get(2)?;
    let card_raw: String = row.get(6)?;
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: TaskStatus::parse(&status_raw)
            .ok_or_else(|| invalid(2, "task status", &status_raw))?,
        icon: row.get(3)?,
        color_hex: row.get(4)?,
        unit: row.get(5)?,
        card_style: CardStyle::parse(&card_raw)
            .ok_or_else(|| invalid(6, "card style", &card_raw))?,
        // 当日循环规则由 core 读侧回填（存储无此列）
        recurrence: Default::default(),
        project_id: row.get(7)?,
        created_at: read_time(row, 8)?,
        updated_at: read_time(row, 9)?,
    })
}

#[derive(Debug, Default, Clone)]
pub struct TaskQuery {
    pub status: Option<TaskStatus>,
    /// 只列启用中（排除归档）
    pub active_only: bool,
    pub project_id: Option<String>,
    pub limit: i64,
}

impl TaskQuery {
    pub fn with_limit(limit: i64) -> Self {
        Self {
            limit,
            ..Default::default()
        }
    }
}

fn query_tasks(conn: &Connection, q: &TaskQuery) -> rusqlite::Result<Vec<Task>> {
    let mut sql = format!("SELECT {TASK_COLS} FROM tasks WHERE 1=1");
    let mut args: Vec<SqlValue> = Vec::new();

    if let Some(status) = q.status {
        args.push(SqlValue::Text(status.as_str().to_string()));
        sql.push_str(&format!(" AND status = ?{}", args.len()));
    }
    if q.active_only {
        sql.push_str(" AND status = 'active'");
    }
    if let Some(pid) = &q.project_id {
        args.push(SqlValue::Text(pid.as_str().to_string()));
        sql.push_str(&format!(" AND project_id = ?{}", args.len()));
    }

    sql.push_str(" ORDER BY created_at DESC");
    sql.push_str(&format!(" LIMIT {}", q.limit.max(1)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(args.iter()), map_task)?;
    rows.collect()
}

pub fn insert(conn: &Connection, task: &Task) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tasks (id, title, status, icon, color_hex, unit, card_style, project_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            task.id,
            task.title,
            task.status.as_str(),
            task.icon,
            task.color_hex,
            task.unit,
            task.card_style.as_str(),
            task.project_id,
            write_time(task.created_at),
            write_time(task.updated_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<Task>> {
    conn.query_row(
        &format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"),
        params![id],
        map_task,
    )
    .optional()
}

/// 新建任务（内部构造完整实体；目标/活动区间由 core 用例负责）。
pub fn create(
    conn: &Connection,
    title: &str,
    icon: &str,
    color_hex: &str,
    unit: &str,
    card_style: CardStyle,
    project_id: Option<&str>,
) -> rusqlite::Result<Task> {
    let now = Utc::now();
    let task = Task {
        id: new_id(id_prefix::TASK),
        title: title.to_string(),
        status: TaskStatus::Active,
        icon: icon.to_string(),
        color_hex: color_hex.to_string(),
        unit: unit.to_string(),
        card_style,
        recurrence: Default::default(),
        project_id: project_id.map(|s| s.to_string()),
        created_at: now,
        updated_at: now,
    };
    insert(conn, &task)?;
    Ok(task)
}

/// 更新任务的可变字段；None 表示不修改。
#[derive(Debug, Default, Clone)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub icon: Option<String>,
    pub color_hex: Option<String>,
    pub unit: Option<String>,
    pub card_style: Option<CardStyle>,
    pub project_id: Option<Option<String>>, // Some(None) = 清除
}

pub fn update(conn: &Connection, id: &str, patch: &TaskPatch) -> rusqlite::Result<()> {
    let mut sets: Vec<String> = Vec::new();
    let mut args: Vec<SqlValue> = Vec::new();

    macro_rules! set_text {
        ($field:expr, $val:expr) => {{
            args.push(SqlValue::Text(($val).to_string()));
            sets.push(format!("{} = ?{}", $field, args.len()));
        }};
    }

    if let Some(title) = &patch.title {
        set_text!("title", title);
    }
    if let Some(status) = patch.status {
        set_text!("status", status.as_str());
    }
    if let Some(icon) = &patch.icon {
        set_text!("icon", icon);
    }
    if let Some(color) = &patch.color_hex {
        set_text!("color_hex", color);
    }
    if let Some(unit) = &patch.unit {
        set_text!("unit", unit);
    }
    if let Some(style) = patch.card_style {
        set_text!("card_style", style.as_str());
    }
    if let Some(pid) = &patch.project_id {
        match pid {
            Some(p) => set_text!("project_id", p),
            None => sets.push("project_id = NULL".to_string()),
        }
    }

    if sets.is_empty() {
        return Ok(());
    }
    args.push(SqlValue::from(write_time(Utc::now())));
    sets.push(format!("updated_at = ?{}", args.len()));

    let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
    let mut all_args = args;
    all_args.push(SqlValue::Text(id.to_string()));
    conn.execute(&sql, params_from_iter(all_args.iter()))?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
}

pub fn count_active(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status = 'active'",
        [],
        |r| r.get(0),
    )
}

pub fn list(conn: &Connection, q: &TaskQuery) -> rusqlite::Result<Vec<Task>> {
    query_tasks(conn, q)
}

pub fn search(conn: &Connection, needle_lower: &str, limit: i64) -> rusqlite::Result<Vec<Task>> {
    let like = format!("%{needle_lower}%");
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLS} FROM tasks
         WHERE lower(title) LIKE ?1
         ORDER BY updated_at DESC LIMIT ?2"
    ))?;
    let rows = stmt.query_map(params![like, limit], map_task)?;
    rows.collect()
}
