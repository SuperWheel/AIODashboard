//! Context 系统：`dashboard context today`
//!
//! AI 协作体验的核心：一次调用即可了解用户当前状态。

use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use dashboard_domain::{Note, TaskStatus};
use rusqlite::Connection;
use serde::Serialize;

use crate::checkin_service::TaskDayView;
use crate::CoreResult;
use dashboard_storage as ds;

#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct TodayStats {
    /// 今日有效任务数（活动中且今天适用）
    pub task_total: i64,
    /// 今日已达标任务数
    pub completed_today: i64,
    /// 今日完成率（Σmin(count,target) / Σtarget，超额封顶）
    pub completion_rate: f64,
    /// 近 7 天「已错过」天数（全部启用任务合计）
    pub missed_last_7d: i64,
}

#[derive(Debug, Serialize)]
pub struct TodayContext {
    pub generated_at: DateTime<Utc>,
    /// 本地日期（逻辑日），例如 "2026-08-22"
    pub date: String,
    pub stats: TodayStats,
    /// 今日有效任务（含打卡状态）
    pub today_tasks: Vec<TaskDayView>,
    pub active_projects: Vec<ProjectSummary>,
    pub open_inbox_count: i64,
    pub recent_notes: Vec<Note>,
}

/// 本地时区的“今天”区间 [start, end)，返回 UTC 时间。
pub fn local_today_range(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let now_local = Local.from_utc_datetime(&now.naive_utc());
    let start_local = now_local
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or(now.naive_utc());
    let start = Local
        .from_local_datetime(&start_local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(now);
    let end = start + Duration::days(1);
    (start, end)
}

/// 本地时区的今天（逻辑日，YYYY-MM-DD）。打卡体系的「今天」唯一入口。
pub fn local_today() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// 今日任务视图列表：全部启用且今天适用的任务，按状态分组排序（进行中 > 待完成 > 已完成）。
pub fn today_task_views(conn: &Connection) -> CoreResult<Vec<TaskDayView>> {
    let tasks = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            status: Some(TaskStatus::Active),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let mut views = Vec::new();
    for t in tasks {
        let view = crate::checkin_service::task_day_view(conn, &t.id)?;
        // 今天不适用的任务（无有效目标等）不进今日列表
        if view.state == crate::day_state::TaskDayState::NotApplicable.as_str() {
            continue;
        }
        views.push(view);
    }
    let rank = |s: &str| match s {
        "in_progress" => 0,
        "pending" => 1,
        "completed" => 2,
        _ => 3,
    };
    views.sort_by_key(|v| rank(&v.state));
    Ok(views)
}

/// 任务墙视图：全部启用任务的当日视图（**含**今天不适用者）。
/// Today 只看今日适用；任务墙要展示"存在但今天轮空"的卡（否则每周任务
/// 在非适用日会从任务页整卡消失）。顺序按创建时间升序。
pub fn wall_task_views(conn: &Connection) -> CoreResult<Vec<TaskDayView>> {
    let today = local_today();
    wall_task_views_on(conn, &today)
}

/// 任务墙视图（指定逻辑日）：全部启用任务在该日的视图，含当日不适用者。
/// 过去日未达标 = missed、未来日 = pending（day_state 按真实今天判定）。
/// 日期翻页专用；顺序按创建时间升序。
pub fn wall_task_views_on(conn: &Connection, day: &str) -> CoreResult<Vec<TaskDayView>> {
    crate::logical_day::parse_day(day)?;
    let tasks = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            status: Some(TaskStatus::Active),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let mut views = Vec::with_capacity(tasks.len());
    for t in tasks {
        views.push(crate::checkin_service::task_day_view_on(conn, &t.id, day)?);
    }
    views.sort_by_key(|v| v.task.created_at);
    Ok(views)
}

/// 近 7 天已错过天数（所有启用任务合计）。
pub fn missed_days_last_7d(conn: &Connection) -> CoreResult<i64> {
    let today = local_today();
    let from = crate::logical_day::add_days(&today, -6)?;
    let tasks = ds::task_repo::list(
        conn,
        &ds::task_repo::TaskQuery {
            status: Some(TaskStatus::Active),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let mut missed = 0i64;
    for t in &tasks {
        let counts: std::collections::HashMap<String, i64> =
            ds::completion_repo::counts_between(conn, &t.id, &from, &today)?
                .into_iter()
                .collect();
        for day in crate::logical_day::days_inclusive(&from, &today)? {
            let period = ds::period_repo::target_on(conn, &t.id, &day)?;
            let target = period.as_ref().map(|p| p.target);
            let applicable = ds::period_repo::is_active_on(conn, &t.id, &day)?
                && period
                    .as_ref()
                    .map(|p| p.recurrence.matches(&p.start_day, &day))
                    .unwrap_or(false);
            let actual = counts.get(&day).copied().unwrap_or(0).max(0);
            let state = crate::day_state::eval_day_state(applicable, target, actual, &day, &today);
            if state == crate::day_state::TaskDayState::Missed {
                missed += 1;
            }
        }
    }
    Ok(missed)
}

pub fn context_today(conn: &Connection) -> CoreResult<TodayContext> {
    let now = Utc::now();
    let today = local_today();

    let views = today_task_views(conn)?;
    let completed = views
        .iter()
        .filter(|v| v.state == crate::day_state::TaskDayState::Completed.as_str())
        .count() as i64;
    let rate = crate::day_state::today_rate(
        &views
            .iter()
            .filter_map(|v| v.target.map(|t| (v.count, t)))
            .collect::<Vec<_>>(),
    );
    let missed = missed_days_last_7d(conn)?;

    let projects = ds::project_repo::list(conn, false)?;
    let inbox_open = ds::inbox_repo::count_open(conn)?;
    let recent_notes = ds::note_repo::recent(conn, 3)?;

    Ok(TodayContext {
        generated_at: now,
        date: today,
        stats: TodayStats {
            task_total: views.len() as i64,
            completed_today: completed,
            completion_rate: rate,
            missed_last_7d: missed,
        },
        today_tasks: views,
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

    #[test]
    fn local_today_format() {
        let t = local_today();
        assert_eq!(t.len(), 10);
        assert!(crate::logical_day::parse_day(&t).is_ok());
    }
}
