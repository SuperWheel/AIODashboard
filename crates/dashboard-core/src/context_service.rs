//! Context 系统：`dashboard context today`
//!
//! AI 协作体验的核心：一次调用即可了解用户当前状态。

use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use dashboard_domain::{Note, Task};
use rusqlite::Connection;
use serde::Serialize;

use crate::CoreResult;
use dashboard_storage as ds;

#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TodayStats {
    /// 今天到期（未完成）
    pub today_total: i64,
    pub overdue_total: i64,
    pub completed_today: i64,
}

#[derive(Debug, Serialize)]
pub struct TodayContext {
    pub generated_at: DateTime<Utc>,
    /// 本地日期，例如 "2026-08-22"
    pub date: String,
    pub stats: TodayStats,
    pub today_tasks: Vec<Task>,
    pub overdue_tasks: Vec<Task>,
    pub active_projects: Vec<ProjectSummary>,
    pub open_inbox_count: i64,
    pub recent_notes: Vec<Note>,
}

/// 本地时区的“今天”区间 [start, end)，返回 UTC 时间。
pub fn local_today_range(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let now_local = Local.from_utc_datetime(&now.naive_utc());
    let start_local = now_local.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let start = Local
        .from_local_datetime(&start_local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(now);
    let end = start + Duration::days(1);
    (start, end)
}

pub fn context_today(conn: &Connection) -> CoreResult<TodayContext> {
    let now = Utc::now();
    let (start, end) = local_today_range(now);

    let today_tasks = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            due_from: Some(start),
            due_to: Some(end),
            exclude_done: true,
            order_by_due: true,
            limit: 200,
            ..Default::default()
        },
    )?;
    let overdue_tasks = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            due_from: None,
            due_to: Some(start),
            exclude_done: true,
            order_by_due: true,
            limit: 200,
            ..Default::default()
        },
    )?;
    let completed_today = ds::task_repo::count_completed_between(conn, start, end)?;
    let projects = ds::project_repo::list(conn, false)?;
    let inbox_open = ds::inbox_repo::count_open(conn)?;
    let recent_notes = ds::note_repo::recent(conn, 3)?;

    Ok(TodayContext {
        generated_at: now,
        date: Local
            .from_utc_datetime(&now.naive_utc())
            .date_naive()
            .format("%Y-%m-%d")
            .to_string(),
        stats: TodayStats {
            today_total: today_tasks.len() as i64,
            overdue_total: overdue_tasks.len() as i64,
            completed_today,
        },
        today_tasks,
        overdue_tasks,
        active_projects: projects
            .into_iter()
            .take(5)
            .map(|p| ProjectSummary {
                id: p.id,
                name: p.name,
            })
            .collect(),
        open_inbox_count: inbox_open,
        recent_notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_range_is_24h_starting_midnight_local() {
        let now = Utc::now();
        let (s, e) = local_today_range(now);
        assert_eq!((e - s), Duration::days(1));
        // 起点应为本地 00:00
        let s_local = Local.from_utc_datetime(&s.naive_utc());
        assert_eq!(
            s_local.time(),
            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        );
    }
}
