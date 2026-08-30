//! 周期总览：周/月/年热力图数据构建（单任务 + 主库聚合）。

use chrono::Datelike;
use dashboard_domain::Task;
use dashboard_storage::{completion_repo, period_repo};
use rusqlite::Connection;
use serde::Serialize;

use crate::context_service::local_today;
use crate::day_state::{eval_day_state, eval_heatmap_state, summarize, DayInput, PeriodSummary};
use crate::logical_day as ld;
use crate::CoreResult;

/// 热力图单日。
#[derive(Debug, Serialize)]
pub struct HeatmapDay {
    pub logical_day: String,
    /// 六态（not_applicable/future/zero/partial_low/partial_high/complete）
    pub display_state: crate::day_state::HeatmapState,
    /// 五态（仅已发生且适用的日子有意义；不适用/未来为 null）
    pub day_state: Option<String>,
    pub actual_count: i64,
    pub target_count: Option<i64>,
    /// 封顶完成率 0–1（not_applicable/future 为 null）
    pub capped_rate: Option<f64>,
    pub is_overachieved: bool,
    pub is_today: bool,
    /// 0=周一 … 6=周日
    pub weekday_index: i64,
    /// 年视图周列索引（以 1 月 1 日所在周周一为第 0 列）
    pub week_index: i64,
    pub month: i64,
}

#[derive(Debug, Serialize)]
pub struct PeriodBucket {
    pub label: String,
    pub start_day: String,
    pub end_day: String,
    pub actual_count: i64,
    pub applicable_day_count: i64,
    pub complete_day_count: i64,
    pub complete_day_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct PeriodOverview {
    /// week | month | year
    pub kind: String,
    pub anchor_day: String,
    pub start_day: String,
    pub end_day: String,
    /// 年热力图前置空格数（1 月 1 日的周几索引，周一=0）
    pub leading_empty_count: i64,
    pub days: Vec<HeatmapDay>,
    pub summary: PeriodSummary,
    pub buckets: Vec<PeriodBucket>,
}

fn period_range(kind: &str, anchor: &str) -> CoreResult<(String, String)> {
    match kind {
        "week" => ld::week_range(anchor),
        "month" => ld::month_range(anchor),
        "year" => ld::year_range(anchor),
        _ => Err(crate::CoreError::Validation(format!(
            "无效的 period: {kind}（week|month|year）"
        ))),
    }
}

fn build_days(
    conn: &Connection,
    task: &Task,
    from: &str,
    to: &str,
    today: &str,
) -> CoreResult<Vec<HeatmapDay>> {
    let counts: std::collections::HashMap<String, i64> =
        completion_repo::counts_between(conn, &task.id, from, to)?
            .into_iter()
            .collect();
    let targets = period_repo::list_targets(conn, &task.id)?;
    let activities = period_repo::list_activity(conn, &task.id)?;

    let mut days = Vec::new();
    for day in ld::days_inclusive(from, to)? {
        let actual = counts.get(&day).copied().unwrap_or(0).max(0);
        let target = targets
            .iter()
            .find(|p| {
                p.start_day <= day
                    && p.end_day
                        .as_deref()
                        .map(|e| e > day.as_str())
                        .unwrap_or(true)
            })
            .map(|p| p.target);
        let applicable = activities.iter().any(|p| {
            p.start_day <= day
                && p.end_day
                    .as_deref()
                    .map(|e| e > day.as_str())
                    .unwrap_or(true)
        });
        let (display, rate, over) = eval_heatmap_state(applicable, target, actual, &day, today);
        let day_state = match display {
            crate::day_state::HeatmapState::Future
            | crate::day_state::HeatmapState::NotApplicable => None,
            _ => Some(
                eval_day_state(applicable, target, actual, &day, today)
                    .as_str()
                    .to_string(),
            ),
        };
        days.push(HeatmapDay {
            weekday_index: ld::weekday_index(&day)?,
            week_index: ld::year_week_index(&day)?,
            month: ld::parse_day(&day)?.month() as i64,
            is_today: day == *today,
            logical_day: day,
            display_state: display,
            day_state,
            actual_count: actual,
            target_count: target,
            capped_rate: rate,
            is_overachieved: over,
        });
    }
    Ok(days)
}

fn build_buckets(kind: &str, days: &[HeatmapDay]) -> Vec<PeriodBucket> {
    // 分桶：周=每日一桶；月=按 ISO 周（weekday 0 起 7 天）；年=按月
    let mut buckets: Vec<(String, Vec<&HeatmapDay>)> = Vec::new();
    match kind {
        "week" => {
            for d in days {
                buckets.push((d.logical_day.clone(), vec![d]));
            }
        }
        "month" => {
            for chunk in days.chunks(7) {
                if chunk.is_empty() {
                    continue;
                }
                let label = format!("第{}周", buckets.len() + 1);
                buckets.push((label, chunk.iter().collect()));
            }
        }
        "year" => {
            let mut by_month: std::collections::BTreeMap<i64, Vec<&HeatmapDay>> =
                std::collections::BTreeMap::new();
            for d in days {
                by_month.entry(d.month).or_default().push(d);
            }
            for (m, group) in by_month {
                buckets.push((format!("{m}月"), group));
            }
        }
        _ => {}
    }
    buckets
        .into_iter()
        .map(|(label, group)| {
            let real_inputs: Vec<DayInput> = group
                .iter()
                .map(|d| DayInput {
                    state: d.display_state,
                    actual: d.actual_count,
                    target: d.target_count,
                })
                .collect();
            let s = summarize(&real_inputs);
            PeriodBucket {
                label,
                start_day: group
                    .first()
                    .map(|d| d.logical_day.clone())
                    .unwrap_or_default(),
                end_day: group
                    .last()
                    .map(|d| d.logical_day.clone())
                    .unwrap_or_default(),
                actual_count: s.actual_count,
                applicable_day_count: s.applicable_day_count,
                complete_day_count: s.complete_day_count,
                complete_day_rate: s.complete_day_rate,
            }
        })
        .collect()
}

/// 单任务周期总览。
pub fn task_period_overview(
    conn: &Connection,
    task_id: &str,
    kind: &str,
    anchor_day: Option<&str>,
) -> CoreResult<PeriodOverview> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    let anchor = anchor_day
        .map(|s| s.to_string())
        .unwrap_or_else(|| today.clone());
    // 校验 anchor 合法
    let _ = ld::parse_day(&anchor)?;
    let (start, end) = period_range(kind, &anchor)?;
    let days = build_days(conn, &task, &start, &end, &today)?;
    let inputs: Vec<DayInput> = days
        .iter()
        .map(|d| DayInput {
            state: d.display_state,
            actual: d.actual_count,
            target: d.target_count,
        })
        .collect();
    let summary = summarize(&inputs);
    let buckets = build_buckets(kind, &days);
    let leading = if kind == "year" {
        ld::year_leading_empty(&anchor)?
    } else {
        0
    };
    Ok(PeriodOverview {
        kind: kind.to_string(),
        anchor_day: anchor,
        start_day: start,
        end_day: end,
        leading_empty_count: leading,
        days,
        summary,
        buckets,
    })
}

/// 主库年度综合热力图单日。
#[derive(Debug, Serialize)]
pub struct LibraryHeatmapDay {
    pub logical_day: String,
    /// not_applicable / future / rate（0–1）
    pub display_state: String,
    /// 主库当日完成率（不适用/未来为 null）
    pub rate: Option<f64>,
    pub active_task_count: i64,
    pub is_today: bool,
    pub weekday_index: i64,
    pub week_index: i64,
    pub month: i64,
}

#[derive(Debug, Serialize)]
pub struct LibraryYearHeatmap {
    pub library_id: String,
    pub year: i64,
    pub start_day: String,
    pub end_day: String,
    pub leading_empty_count: i64,
    pub days: Vec<LibraryHeatmapDay>,
}

/// 主库综合热力图（按年）。逐日读取当时真实生效的活动/目标/归属区间。
pub fn library_year_heatmap(
    conn: &Connection,
    library_id: &str,
    anchor_day: Option<&str>,
) -> CoreResult<LibraryYearHeatmap> {
    let lib = crate::library_service::get_library(conn, library_id)?;
    let today = local_today();
    let anchor = anchor_day
        .map(|s| s.to_string())
        .unwrap_or_else(|| today.clone());
    let anchor_date = ld::parse_day(&anchor)?;
    let (start, end) = ld::year_range(&anchor)?;

    // 预取：年内出现过的归属区间
    let memberships = period_repo::library_memberships_in_range(conn, &lib.id, &start, &end)?;
    // 涉及任务的 id 去重
    let task_ids: Vec<String> = {
        let mut v: Vec<String> = memberships.iter().map(|m| m.task_id.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    // 预取每个任务的打卡计数、目标区间、活动区间
    let mut counts: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    let mut targets: std::collections::HashMap<String, Vec<dashboard_domain::TaskTargetPeriod>> =
        std::collections::HashMap::new();
    let mut activities: std::collections::HashMap<
        String,
        Vec<dashboard_domain::TaskActivityPeriod>,
    > = std::collections::HashMap::new();
    for tid in &task_ids {
        for (day, n) in completion_repo::counts_between(conn, tid, &start, &end)? {
            counts.insert((tid.clone(), day), n);
        }
        targets.insert(tid.clone(), period_repo::list_targets(conn, tid)?);
        activities.insert(tid.clone(), period_repo::list_activity(conn, tid)?);
    }

    // 按日聚合归属：day -> Vec<task_id>
    let mut day_tasks: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for m in &memberships {
        let from = m.start_day.clone().max(start.clone());
        let to = m
            .end_day
            .clone()
            .unwrap_or_else(|| end.clone())
            .min(end.clone());
        for day in ld::days_inclusive(&from, &to)? {
            day_tasks.entry(day).or_default().push(m.task_id.clone());
        }
    }

    let mut days = Vec::new();
    for day in ld::days_inclusive(&start, &end)? {
        let is_today = day == today;
        let weekday_index = ld::weekday_index(&day)?;
        let week_index = ld::year_week_index(&day)?;
        let month = ld::parse_day(&day)?.month() as i64;

        if day > today {
            days.push(LibraryHeatmapDay {
                logical_day: day,
                display_state: "future".into(),
                rate: None,
                active_task_count: 0,
                is_today,
                weekday_index,
                week_index,
                month,
            });
            continue;
        }

        let tids = day_tasks.get(&day).cloned().unwrap_or_default();
        let mut contrib_sum = 0f64;
        let mut active_n = 0i64;
        for tid in &tids {
            let target = targets
                .get(tid)
                .and_then(|ps| {
                    ps.iter().find(|p| {
                        p.start_day <= day
                            && p.end_day
                                .as_deref()
                                .map(|e| e > day.as_str())
                                .unwrap_or(true)
                    })
                })
                .map(|p| p.target);
            let applicable = activities.get(tid).map(|ps| {
                ps.iter().any(|p| {
                    p.start_day <= day
                        && p.end_day
                            .as_deref()
                            .map(|e| e > day.as_str())
                            .unwrap_or(true)
                })
            }) == Some(true);
            if !applicable || target.map(|t| t <= 0).unwrap_or(true) {
                continue;
            }
            let target = target.unwrap_or(0) as f64;
            let actual = counts
                .get(&(tid.clone(), day.clone()))
                .copied()
                .unwrap_or(0)
                .max(0) as f64;
            contrib_sum += (actual / target).min(1.0);
            active_n += 1;
        }

        let (state, rate) = if active_n == 0 {
            ("not_applicable", None)
        } else {
            ("rate", Some(contrib_sum / active_n as f64))
        };
        days.push(LibraryHeatmapDay {
            logical_day: day,
            display_state: state.into(),
            rate,
            active_task_count: active_n,
            is_today,
            weekday_index,
            week_index,
            month,
        });
    }

    Ok(LibraryYearHeatmap {
        library_id: lib.id,
        year: anchor_date.year() as i64,
        leading_empty_count: ld::year_leading_empty(&anchor)?,
        start_day: start,
        end_day: end,
        days,
    })
}
