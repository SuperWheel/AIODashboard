//! CLI 工具函数。

use dashboard_core::CoreResult;
use dashboard_storage::{library_repo, task_repo};
use rusqlite::Connection;

/// 打开默认数据库（WAL）。
pub fn open_conn() -> CoreResult<Connection> {
    Ok(dashboard_storage::open_default()?)
}

/// 支持完整 ID 或唯一前缀匹配。
pub fn resolve_task_id(conn: &Connection, input: &str) -> CoreResult<String> {
    if let Some(t) = task_repo::get(conn, input)? {
        return Ok(t.id);
    }
    let mut stmt = conn.prepare("SELECT id FROM tasks WHERE id LIKE ?1 || '%'")?;
    let ids: Vec<String> = stmt
        .query_map([input], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?;
    match ids.len() {
        0 => Err(dashboard_core::CoreError::NotFound(format!("task {input}"))),
        1 => Ok(ids.into_iter().next().unwrap_or_default()),
        _ => Err(dashboard_core::CoreError::Validation(format!(
            "ID 前缀 '{input}' 匹配到多个任务，请使用更长前缀"
        ))),
    }
}

/// 主库 ID 完整或唯一前缀匹配。
pub fn resolve_library_id(conn: &Connection, input: &str) -> CoreResult<String> {
    if let Some(l) = library_repo::get(conn, input)? {
        return Ok(l.id);
    }
    let mut stmt = conn.prepare("SELECT id FROM date_libraries WHERE id LIKE ?1 || '%'")?;
    let ids: Vec<String> = stmt
        .query_map([input], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?;
    match ids.len() {
        0 => Err(dashboard_core::CoreError::NotFound(format!(
            "date_library {input}"
        ))),
        1 => Ok(ids.into_iter().next().unwrap_or_default()),
        _ => Err(dashboard_core::CoreError::Validation(format!(
            "ID 前缀 '{input}' 匹配到多个主库，请使用更长前缀"
        ))),
    }
}
