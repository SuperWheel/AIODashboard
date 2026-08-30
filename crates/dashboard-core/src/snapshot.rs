//! Widget Snapshot：Rust Core → 简单快照 → 文件（App Group）→ SwiftUI Widget。
//!
//! Widget 只消费本快照，不接触数据库 Schema。

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;

use dashboard_storage as ds;

pub const SNAPSHOT_KIND_TODAY: &str = "today";
const SCHEMA: &str = "widget.snapshot/v1";

#[derive(Debug, Serialize)]
pub struct WidgetSnapshotV1 {
    pub schema: &'static str,
    pub generated_at: DateTime<Utc>,
    pub today: TodayPart,
}

#[derive(Debug, Serialize)]
pub struct TodayPart {
    /// 本地日期 "2026-08-22"
    pub date: String,
    /// 今日有效任务数
    pub task_total: i64,
    /// 今日已达标任务数
    pub task_completed_today: i64,
    /// 今日完成率（Σmin(count,target)/Σtarget）
    pub completion_rate: f64,
    /// 近 7 天已错过天数
    pub missed_last_7d: i64,
    pub inbox_open: i64,
    /// 未来接入 Calendar；当前恒为 null
    pub next_event: Option<String>,
    /// 当前重点：第一个进行中任务标题，否则第一个活跃项目名
    pub current_focus: Option<String>,
}

/// 快照文件写入路径：
/// 1. `DASHBOARD_WIDGET_SNAPSHOT_PATH` 环境变量
/// 2. macOS App Group 容器 `~/Library/Group Containers/group.com.aiodashboard.shared/widget-snapshot.json`
/// 3. 兜底：数据库同目录下 `widget/widget-snapshot.json`
pub fn snapshot_path() -> PathBuf {
    if let Ok(p) = std::env::var("DASHBOARD_WIDGET_SNAPSHOT_PATH") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let group = PathBuf::from(&home).join("Library/Group Containers/group.com.aiodashboard.shared");
    if group.is_dir() {
        return group.join("widget-snapshot.json");
    }
    let db_path = ds::default_db_path();
    db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("widget")
        .join("widget-snapshot.json")
}

/// 构建今日快照（打卡口径）。
pub fn build_today(conn: &Connection) -> rusqlite::Result<WidgetSnapshotV1> {
    let now = Utc::now();
    let views = crate::context_service::today_task_views(conn)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let completed = views.iter().filter(|v| v.state == "completed").count() as i64;
    let rate = crate::day_state::today_rate(
        &views
            .iter()
            .filter_map(|v| v.target.map(|t| (v.count, t)))
            .collect::<Vec<_>>(),
    );
    let missed = crate::context_service::missed_days_last_7d(conn)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let inbox_open = ds::inbox_repo::count_open(conn)?;

    // 当前重点：进行中任务 > 活跃项目 > 无
    let focus = match views.iter().find(|v| v.state == "in_progress") {
        Some(v) => Some(v.task.title.clone()),
        None => ds::project_repo::list(conn, false)?
            .first()
            .map(|p| p.name.clone()),
    };

    Ok(WidgetSnapshotV1 {
        schema: SCHEMA,
        generated_at: now,
        today: TodayPart {
            date: crate::context_service::local_today(),
            task_total: views.len() as i64,
            task_completed_today: completed,
            completion_rate: rate,
            missed_last_7d: missed,
            inbox_open,
            next_event: None,
            current_focus: focus,
        },
    })
}

fn write_atomic(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 业务变更后调用：刷新内存中的最新快照（存 DB），并尽力写入快照文件。
/// 失败不影响主流程。
pub fn refresh(conn: &Connection) {
    match build_today(conn) {
        Ok(snap) => {
            let payload = serde_json::to_string_pretty(&snap).unwrap_or_else(|_| "{}".into());
            let _ = conn.execute(
                "INSERT INTO widget_snapshots (kind, generated_at, payload) VALUES (?1,?2,?3)
                 ON CONFLICT(kind) DO UPDATE SET generated_at=excluded.generated_at, payload=excluded.payload",
                rusqlite::params![SNAPSHOT_KIND_TODAY, snap.generated_at.to_rfc3339(), payload],
            );
            if let Err(e) = write_atomic(&snapshot_path(), &payload) {
                eprintln!("[dashboard] widget snapshot write failed: {e}");
            }
        }
        Err(e) => eprintln!("[dashboard] widget snapshot build failed: {e}"),
    }
}
