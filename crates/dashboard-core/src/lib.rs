//! dashboard-core
//!
//! Application 层：所有业务用例的唯一实现。
//! GUI（Tauri Command）、CLI、AI 都只能通过本 crate 操作数据。

pub mod context_service;
pub mod inbox_service;
pub mod note_service;
pub mod plugin_manifest;
pub mod plugin_service;
pub mod project_service;
pub mod search_service;
pub mod snapshot;
pub mod task_service;

use chrono::{DateTime, Utc};
use dashboard_domain::{ActivityEntry, Actor};
use dashboard_storage::activity_repo;
use rusqlite::Connection;

/// 核心错误类型。接口层负责映射为 Exit Code / HTTP / Tauri 错误。
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;

/// 写入审计日志。所有业务变更都应调用。
pub fn log_activity(
    conn: &Connection,
    ts: DateTime<Utc>,
    actor: Actor,
    action: &str,
    object_type: &str,
    object_id: Option<&str>,
    detail: &serde_json::Value,
) {
    // 审计日志失败不应阻断主流程，但也不静默吞掉：打印到 stderr。
    if let Err(e) = activity_repo::record(conn, ts, actor, action, object_type, object_id, detail) {
        eprintln!("[dashboard] activity log failed: {e}");
    }
}

/// 列出操作日志。
pub fn list_activity(
    conn: &Connection,
    limit: i64,
    actor_filter: Option<Actor>,
) -> CoreResult<Vec<ActivityEntry>> {
    Ok(activity_repo::list(conn, limit, actor_filter)?)
}

/// 解析期限输入：
/// - `YYYY-MM-DD`：按本地时区当天 23:59:59
/// - RFC3339 完整时间戳
pub fn parse_due_input(s: &str) -> CoreResult<DateTime<Utc>> {
    use chrono::{Local, NaiveDate, TimeZone};
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive = d.and_hms_opt(23, 59, 59).unwrap();
        let local = Local
            .from_local_datetime(&naive)
            .earliest()
            .ok_or_else(|| CoreError::Validation(format!("无效的本地日期: {s}")))?;
        return Ok(local.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| {
            CoreError::Validation(format!("无法解析日期 '{s}'（支持 YYYY-MM-DD 或 RFC3339）"))
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn actor_roundtrip() {
        use dashboard_domain::Actor;
        for a in [
            Actor::User,
            Actor::Cli,
            Actor::Ai,
            Actor::Automation,
            Actor::System,
        ] {
            assert_eq!(Actor::parse(&a.as_str()), Some(a));
        }
        let p = Actor::Plugin("com.leeyl.pomodoro".to_string());
        assert_eq!(p.as_str(), "plugin:com.leeyl.pomodoro");
        assert_eq!(Actor::parse(&p.as_str()), Some(p.clone()));
        // 非法形态不解析为 Plugin
        assert_eq!(Actor::parse("plugin:"), None);
        // serde 往返：actor 序列化始终是字符串
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"plugin:com.leeyl.pomodoro\"");
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
