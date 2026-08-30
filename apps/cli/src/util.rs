//! CLI 工具函数。

use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use dashboard_core::CoreResult;
use dashboard_storage::task_repo;
use rusqlite::Connection;

/// 打开默认数据库（WAL）。
pub fn open_conn() -> CoreResult<Connection> {
    Ok(dashboard_storage::open_default()?)
}

/// 解析期限：
/// - `YYYY-MM-DD`：按本地时区当天 23:59:59
/// - RFC3339 完整时间戳
pub fn parse_due(s: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = d.and_hms_opt(23, 59, 59).unwrap();
        let local = Local
            .from_local_datetime(&naive)
            .earliest()
            .ok_or_else(|| format!("无效的本地日期: {s}"))?;
        return Ok(local.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| format!("无法解析日期 '{s}'（支持 YYYY-MM-DD 或 RFC3339）"))
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
        1 => Ok(ids.into_iter().next().unwrap()),
        _ => Err(dashboard_core::CoreError::Validation(format!(
            "ID 前缀 '{input}' 匹配到多个任务，请使用更长前缀"
        ))),
    }
}
