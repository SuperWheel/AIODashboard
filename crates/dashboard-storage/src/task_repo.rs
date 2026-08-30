//! Task Repository

use chrono::{DateTime, Utc};
use dashboard_domain::{id_prefix, new_id, Task, TaskStatus};
use rusqlite::{
    params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension, Row,
};

use crate::timeutil::{read_time, read_time_opt, write_time};

pub const TASK_COLS: &str =
    "id, title, status, due_at, project_id, completed_at, created_at, updated_at";

fn map_task(row: &Row) -> rusqlite::Result<Task> {
    let status_raw: String = row.get(2)?;
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: TaskStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                format!("invalid task status: {status_raw}").into(),
            )
        })?,
        due_at: read_time_opt(row, 3)?,
        project_id: row.get(4)?,
        completed_at: read_time_opt(row, 5)?,
        created_at: read_time(row, 6)?,
        updated_at: read_time(row, 7)?,
    })
}

#[derive(Debug, Default, Clone)]
pub struct TaskQuery {
    pub status: Option<TaskStatus>,
    pub exclude_done: bool,
    pub project_id: Option<String>,
    /// due_at >= due_from
    pub due_from: Option<DateTime<Utc>>,
    /// due_at <= due_to
    pub due_to: Option<DateTime<Utc>>,
    /// 排序：按到期时间升序（无期限靠后）
    pub order_by_due: bool,
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
    if q.exclude_done {
        sql.push_str(" AND status != 'done'");
    }
    if let Some(pid) = &q.project_id {
        args.push(SqlValue::Text(pid.as_str().to_string()));
        sql.push_str(&format!(" AND project_id = ?{}", args.len()));
    }
    if let Some(from) = q.due_from {
        args.push(SqlValue::from(write_time(from)));
        sql.push_str(" AND due_at IS NOT NULL");
        sql.push_str(&format!(" AND due_at >= ?{}", args.len()));
    }
    if let Some(to) = q.due_to {
        args.push(SqlValue::from(write_time(to)));
        sql.push_str(" AND due_at IS NOT NULL");
        sql.push_str(&format!(" AND due_at <= ?{}", args.len()));
    }

    if q.order_by_due {
        sql.push_str(" ORDER BY COALESCE(due_at, '9999-12-31T00:00:00Z') ASC, created_at ASC");
    } else {
        sql.push_str(" ORDER BY created_at DESC");
    }
    sql.push_str(&format!(" LIMIT {}", q.limit.max(1)));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(args.iter()), map_task)?;
    rows.collect()
}

pub fn insert(conn: &Connection, task: &Task) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tasks (id, title, status, due_at, project_id, completed_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            task.id,
            task.title,
            task.status.as_str(),
            task.due_at.map(write_time),
            task.project_id,
            task.completed_at.map(write_time),
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

/// 新建任务（内部构造完整实体）。
pub fn create(
    conn: &Connection,
    title: &str,
    due_at: Option<DateTime<Utc>>,
    project_id: Option<&str>,
) -> rusqlite::Result<Task> {
    let now = Utc::now();
    let task = Task {
        id: new_id(id_prefix::TASK),
        title: title.to_string(),
        status: TaskStatus::Todo,
        due_at,
        project_id: project_id.map(|s| s.to_string()),
        completed_at: None,
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
    pub due_at: Option<Option<DateTime<Utc>>>, // Some(None) = 清除
    pub project_id: Option<Option<String>>,    // Some(None) = 清除
}

pub fn update(conn: &Connection, id: &str, patch: &TaskPatch) -> rusqlite::Result<()> {
    let mut sets: Vec<String> = Vec::new();
    let mut args: Vec<SqlValue> = Vec::new();

    if let Some(title) = &patch.title {
        args.push(SqlValue::Text(title.as_str().to_string()));
        sets.push(format!("title = ?{}", args.len()));
    }
    if let Some(status) = patch.status {
        args.push(SqlValue::Text(status.as_str().to_string()));
        sets.push(format!("status = ?{}", args.len()));
        if status == TaskStatus::Done {
            args.push(SqlValue::from(write_time(Utc::now())));
            sets.push(format!("completed_at = ?{}", args.len()));
        } else {
            sets.push("completed_at = NULL".to_string());
        }
    }
    if let Some(due) = patch.due_at {
        match due {
            Some(dt) => {
                args.push(SqlValue::from(write_time(dt)));
                sets.push(format!("due_at = ?{}", args.len()));
            }
            None => sets.push("due_at = NULL".to_string()),
        }
    }
    if let Some(pid) = &patch.project_id {
        match pid {
            Some(p) => {
                args.push(SqlValue::Text(p.as_str().to_string()));
                sets.push(format!("project_id = ?{}", args.len()));
            }
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

pub fn count_open(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status != 'done'",
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

/// 统计某时间段内完成的任务数。
pub fn count_completed_between(
    conn: &Connection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status='done' AND completed_at IS NOT NULL
         AND completed_at >= ?1 AND completed_at < ?2",
        params![write_time(from), write_time(to)],
        |r| r.get(0),
    )
}
