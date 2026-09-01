//! 周期总览：周/月/年热力图数据构建（单任务 + 重要日聚合）。

use chrono::Datelike;
use dashboard_domain::Task;
use dashboard_storage::{completion_repo, period_repo, task_repo};
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
    /// 周列索引：窗口起点（周一）为第 0 列
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
    /// 滚动年窗口起点为周一，前置空格恒 0（保留字段兼容）
    pub leading_empty_count: i64,
    pub days: Vec<HeatmapDay>,
    pub summary: PeriodSummary,
    pub buckets: Vec<PeriodBucket>,
}

fn period_range(kind: &str, anchor: &str) -> CoreResult<(String, String)> {
    match kind {
        "week" => ld::week_range(anchor),
        "month" => ld::month_range(anchor),
        // 年 = 滚动 53 周窗口（右端为本周），协议 v4 起不再是日历年
        "year" => ld::rolling_year_range(anchor),
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
        let period = targets.iter().find(|p| {
            p.start_day <= day
                && p.end_day
                    .as_deref()
                    .map(|e| e > day.as_str())
                    .unwrap_or(true)
        });
        let target = period.map(|p| p.target);
        let applicable = activities.iter().any(|p| {
            p.start_day <= day
                && p.end_day
                    .as_deref()
                    .map(|e| e > day.as_str())
                    .unwrap_or(true)
        }) && period
            .map(|p| p.recurrence.matches(&p.start_day, &day))
            .unwrap_or(false);
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
            week_index: ld::rolling_week_index(from, &day)?,
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
    Ok(PeriodOverview {
        kind: kind.to_string(),
        anchor_day: anchor,
        start_day: start,
        end_day: end,
        leading_empty_count: 0,
        days,
        summary,
        buckets,
    })
}

/// 聚合热力图单日（重要日综合 / 全局共用形状）。
/// display_state: not_applicable / future / rate；rate 时 rate 字段为 0–1。
#[derive(Debug, Serialize)]
pub struct AggregateHeatmapDay {
    pub logical_day: String,
    pub display_state: String,
    /// 当日完成率（不适用/未来为 null）
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
    pub days: Vec<AggregateHeatmapDay>,
}

#[derive(Debug, Serialize)]
pub struct GlobalYearHeatmap {
    pub year: i64,
    pub start_day: String,
    pub end_day: String,
    pub leading_empty_count: i64,
    pub days: Vec<AggregateHeatmapDay>,
}

/// 聚合预取：计数按 (task_id, day)，目标/活动区间按 task_id。
struct TaskDayFacts {
    counts: std::collections::HashMap<(String, String), i64>,
    targets: std::collections::HashMap<String, Vec<dashboard_domain::TaskTargetPeriod>>,
    activities: std::collections::HashMap<String, Vec<dashboard_domain::TaskActivityPeriod>>,
}

fn prefetch_task_facts(
    conn: &Connection,
    task_ids: &[String],
    start: &str,
    end: &str,
) -> CoreResult<TaskDayFacts> {
    let mut counts = std::collections::HashMap::new();
    let mut targets = std::collections::HashMap::new();
    let mut activities = std::collections::HashMap::new();
    for tid in task_ids {
        for (day, n) in completion_repo::counts_between(conn, tid, start, end)? {
            counts.insert((tid.clone(), day), n);
        }
        targets.insert(tid.clone(), period_repo::list_targets(conn, tid)?);
        activities.insert(tid.clone(), period_repo::list_activity(conn, tid)?);
    }
    Ok(TaskDayFacts {
        counts,
        targets,
        activities,
    })
}

/// 单任务单日贡献：活动区间不覆盖 / 循环不命中 / 无目标或目标<=0 → None；
/// 否则 min(actual/target, 1)（超额封顶）。
fn task_day_contrib(facts: &TaskDayFacts, task_id: &str, day: &str) -> Option<f64> {
    let period = facts.targets.get(task_id).and_then(|ps| {
        ps.iter().find(|p| {
            p.start_day.as_str() <= day && p.end_day.as_deref().map(|e| e > day).unwrap_or(true)
        })
    })?;
    if period.target <= 0 || !period.recurrence.matches(&period.start_day, day) {
        return None;
    }
    let active = facts
        .activities
        .get(task_id)
        .map(|ps| {
            ps.iter().any(|p| {
                p.start_day.as_str() <= day && p.end_day.as_deref().map(|e| e > day).unwrap_or(true)
            })
        })
        .unwrap_or(false);
    if !active {
        return None;
    }
    let actual = facts
        .counts
        .get(&(task_id.to_string(), day.to_string()))
        .copied()
        .unwrap_or(0)
        .max(0) as f64;
    Some((actual / period.target as f64).min(1.0))
}

/// 逐日聚合：day_tasks 为 None 时每日取 all_tasks（全局口径），
/// 否则按当日真实生效归属（重要日口径）。未来日 → future；无有效任务 → not_applicable。
fn aggregate_days(
    facts: &TaskDayFacts,
    start: &str,
    end: &str,
    today: &str,
    day_tasks: Option<&std::collections::HashMap<String, Vec<String>>>,
    all_tasks: &[String],
) -> CoreResult<Vec<AggregateHeatmapDay>> {
    let mut days = Vec::new();
    for day in ld::days_inclusive(start, end)? {
        let is_today = day == *today;
        let weekday_index = ld::weekday_index(&day)?;
        let week_index = ld::rolling_week_index(start, &day)?;
        let month = ld::parse_day(&day)?.month() as i64;

        if day.as_str() > today {
            days.push(AggregateHeatmapDay {
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

        let tids: &[String] = match day_tasks {
            Some(m) => m.get(&day).map(Vec::as_slice).unwrap_or(&[]),
            None => all_tasks,
        };
        let mut contrib_sum = 0f64;
        let mut active_n = 0i64;
        for tid in tids {
            if let Some(c) = task_day_contrib(facts, tid, &day) {
                contrib_sum += c;
                active_n += 1;
            }
        }
        let (state, rate) = if active_n == 0 {
            ("not_applicable", None)
        } else {
            ("rate", Some(contrib_sum / active_n as f64))
        };
        days.push(AggregateHeatmapDay {
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
    Ok(days)
}

/// 重要日综合热力图（滚动年窗口）。逐日读取当时真实生效的活动/目标/归属区间。
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
    let (start, end) = ld::rolling_year_range(&anchor)?;

    // 预取：年内出现过的归属区间
    let memberships = period_repo::library_memberships_in_range(conn, &lib.id, &start, &end)?;
    // 涉及任务的 id 去重
    let task_ids: Vec<String> = {
        let mut v: Vec<String> = memberships.iter().map(|m| m.task_id.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    let facts = prefetch_task_facts(conn, &task_ids, &start, &end)?;

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

    let days = aggregate_days(&facts, &start, &end, &today, Some(&day_tasks), &[])?;
    Ok(LibraryYearHeatmap {
        library_id: lib.id,
        year: anchor_date.year() as i64,
        leading_empty_count: 0,
        start_day: start,
        end_day: end,
        days,
    })
}

/// 全局年度综合热力图：聚合所有任务（含已归档——归档只关闭活动区间，
/// 历史日口径由区间决定，不回写）。聚合口径与重要日综合热力图一致。
/// 窗口 = 滚动 53 周（右端为本周，跨年含上一年尾部）。
pub fn global_year_heatmap(
    conn: &Connection,
    anchor_day: Option<&str>,
) -> CoreResult<GlobalYearHeatmap> {
    let today = local_today();
    let anchor = anchor_day
        .map(|s| s.to_string())
        .unwrap_or_else(|| today.clone());
    let anchor_date = ld::parse_day(&anchor)?;
    let (start, end) = ld::rolling_year_range(&anchor)?;

    let tasks = task_repo::list(
        conn,
        &task_repo::TaskQuery {
            limit: i64::MAX,
            ..Default::default()
        },
    )?;
    let task_ids: Vec<String> = tasks.into_iter().map(|t| t.id).collect();
    let facts = prefetch_task_facts(conn, &task_ids, &start, &end)?;
    let days = aggregate_days(&facts, &start, &end, &today, None, &task_ids)?;

    Ok(GlobalYearHeatmap {
        year: anchor_date.year() as i64,
        leading_empty_count: 0,
        start_day: start,
        end_day: end,
        days,
    })
}
