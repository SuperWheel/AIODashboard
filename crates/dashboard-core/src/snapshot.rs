//! Widget Snapshot：Rust Core → 简单快照 → 文件（App Group）→ SwiftUI Widget。
//!
//! Widget 只消费本快照，不接触数据库 Schema。

use chrono::{DateTime, Local, TimeZone, Utc};
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
    /// 今日待办总数（今天到期未完成）
    pub task_total: i64,
    /// 今日已完成数
    pub task_completed_today: i64,
    pub overdue_total: i64,
    pub inbox_open: i64,
    /// 未来接入 Calendar；当前恒为 null
    pub next_event: Option<String>,
    /// 当前重点：第一个 doing 任务标题，否则第一个活跃项目名
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

/// 构建今日快照。
pub fn build_today(conn: &Connection) -> rusqlite::Result<WidgetSnapshotV1> {
    use crate::context_service::local_today_range;
    let now = Utc::now();
    let (start, end) = local_today_range(now);

    let total = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            due_from: Some(start),
            due_to: Some(end),
            exclude_done: true,
            limit: 10_000,
            ..Default::default()
        },
    )?
    .len() as i64;
    let overdue = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            due_to: Some(start),
            exclude_done: true,
            limit: 10_000,
            ..Default::default()
        },
    )?
    .len() as i64;
    let completed = ds::task_repo::count_completed_between(conn, start, end)?;
    let inbox_open = ds::inbox_repo::count_open(conn)?;

    // 当前重点：doing > 活跃项目 > 无
    let doing = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            status: Some(dashboard_domain::TaskStatus::Doing),
            limit: 1,
            ..Default::default()
        },
    )?;
    let focus = match doing.first() {
        Some(t) => Some(t.title.clone()),
        None => ds::project_repo::list(conn, false)?
            .first()
            .map(|p| p.name.clone()),
    };

    Ok(WidgetSnapshotV1 {
        schema: SCHEMA,
        generated_at: now,
        today: TodayPart {
            date: Local
                .from_utc_datetime(&now.naive_utc())
                .date_naive()
                .format("%Y-%m-%d")
                .to_string(),
            task_total: total,
            task_completed_today: completed,
            overdue_total: overdue,
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
