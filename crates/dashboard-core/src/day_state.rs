//! 任务日状态与热力状态判定（纯函数，无 IO）。
//!
//! 口径照搬 PlanningDays 已验证模型（见 openspec/changes/003-task-checkin-cards/design.md 决策 2）：
//! - 五态：not_applicable / pending / in_progress / completed / missed
//! - 六态：not_applicable / future / zero / partial_low / partial_high / complete

use serde::Serialize;

/// 当日五态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDayState {
    NotApplicable,
    Pending,
    InProgress,
    Completed,
    Missed,
}

impl TaskDayState {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskDayState::NotApplicable => "not_applicable",
            TaskDayState::Pending => "pending",
            TaskDayState::InProgress => "in_progress",
            TaskDayState::Completed => "completed",
            TaskDayState::Missed => "missed",
        }
    }
}

/// 热力图六态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeatmapState {
    NotApplicable,
    Future,
    Zero,
    PartialLow,
    PartialHigh,
    Complete,
}

impl HeatmapState {
    pub fn as_str(self) -> &'static str {
        match self {
            HeatmapState::NotApplicable => "not_applicable",
            HeatmapState::Future => "future",
            HeatmapState::Zero => "zero",
            HeatmapState::PartialLow => "partial_low",
            HeatmapState::PartialHigh => "partial_high",
            HeatmapState::Complete => "complete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "not_applicable" => Some(HeatmapState::NotApplicable),
            "future" => Some(HeatmapState::Future),
            "zero" => Some(HeatmapState::Zero),
            "partial_low" => Some(HeatmapState::PartialLow),
            "partial_high" => Some(HeatmapState::PartialHigh),
            "complete" => Some(HeatmapState::Complete),
            _ => None,
        }
    }
}

/// 五态判定。
///
/// - applicable：当天在活动区间内且有有效目标（target > 0）
/// - actual：当日净次数
/// - day / today：YYYY-MM-DD 逻辑日（字典序即时间序）
pub fn eval_day_state(
    applicable: bool,
    target: Option<i64>,
    actual: i64,
    day: &str,
    today: &str,
) -> TaskDayState {
    if !applicable || target.map(|t| t <= 0).unwrap_or(true) {
        return TaskDayState::NotApplicable;
    }
    let target = target.unwrap_or(0);
    if actual >= target {
        return TaskDayState::Completed;
    }
    if day < today {
        return TaskDayState::Missed;
    }
    if actual > 0 {
        return TaskDayState::InProgress;
    }
    TaskDayState::Pending
}

/// 六态判定，返回 (state, capped_rate, is_overachieved)。
pub fn eval_heatmap_state(
    applicable: bool,
    target: Option<i64>,
    actual: i64,
    day: &str,
    today: &str,
) -> (HeatmapState, Option<f64>, bool) {
    if day > today {
        return (HeatmapState::Future, None, false);
    }
    if !applicable || target.map(|t| t <= 0).unwrap_or(true) {
        return (HeatmapState::NotApplicable, None, false);
    }
    let target = target.unwrap_or(0);
    let overachieved = actual > target;
    let rate = (actual as f64 / target as f64).min(1.0);
    if actual >= target {
        return (HeatmapState::Complete, Some(1.0), overachieved);
    }
    if actual <= 0 {
        return (HeatmapState::Zero, Some(0.0), false);
    }
    if rate < 0.5 {
        (HeatmapState::PartialLow, Some(rate), false)
    } else {
        (HeatmapState::PartialHigh, Some(rate), false)
    }
}

/// 热力图单日数据（统计输入）。
#[derive(Debug, Clone, Copy)]
pub struct DayInput {
    pub state: HeatmapState,
    pub actual: i64,
    pub target: Option<i64>,
}

/// 周期统计口径。
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct PeriodSummary {
    /// 适用日净次数之和
    pub actual_count: i64,
    /// 已发生且应做的天数（非 not_applicable/future）
    pub applicable_day_count: i64,
    /// 其中 100% 完成的天数
    pub complete_day_count: i64,
    /// 完整完成率 = complete / applicable（适用日为 0 时 = 0）
    pub complete_day_rate: f64,
    /// 当前连续完整完成天数（不适用日也打断）
    pub current_streak: i64,
    /// 周期内最长连续完整完成天数
    pub longest_streak: i64,
}

pub fn summarize(days: &[DayInput]) -> PeriodSummary {
    let mut s = PeriodSummary::default();
    let mut current = 0i64;
    for d in days {
        match d.state {
            HeatmapState::NotApplicable | HeatmapState::Future => {
                // 不适用/未来：不进分母；不适用日打断连续
                if d.state == HeatmapState::NotApplicable {
                    current = 0;
                }
            }
            HeatmapState::Complete => {
                s.applicable_day_count += 1;
                s.complete_day_count += 1;
                s.actual_count += d.actual;
                current += 1;
                s.longest_streak = s.longest_streak.max(current);
            }
            HeatmapState::Zero | HeatmapState::PartialLow | HeatmapState::PartialHigh => {
                s.applicable_day_count += 1;
                s.actual_count += d.actual;
                current = 0;
            }
        }
    }
    s.current_streak = current;
    s.complete_day_rate = if s.applicable_day_count > 0 {
        s.complete_day_count as f64 / s.applicable_day_count as f64
    } else {
        0.0
    };
    s
}

/// 今日总完成率：Σmin(count, target) / Σtarget（超额封顶）。
pub fn today_rate(items: &[(i64, i64)]) -> f64 {
    let mut got = 0i64;
    let mut want = 0i64;
    for (count, target) in items {
        if *target > 0 {
            got += (*count).min(*target);
            want += *target;
        }
    }
    if want > 0 {
        got as f64 / want as f64
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-08-30";

    #[test]
    fn day_state_five_states() {
        // 不适用：不在活动区间或无有效目标
        assert_eq!(
            eval_day_state(false, Some(1), 0, TODAY, TODAY),
            TaskDayState::NotApplicable
        );
        assert_eq!(
            eval_day_state(true, None, 0, TODAY, TODAY),
            TaskDayState::NotApplicable
        );
        assert_eq!(
            eval_day_state(true, Some(0), 0, TODAY, TODAY),
            TaskDayState::NotApplicable
        );
        // 待完成：今天有效、0 次
        assert_eq!(
            eval_day_state(true, Some(2), 0, TODAY, TODAY),
            TaskDayState::Pending
        );
        // 进行中：今天有效、有记录未达标
        assert_eq!(
            eval_day_state(true, Some(2), 1, TODAY, TODAY),
            TaskDayState::InProgress
        );
        // 已完成：达标与超额
        assert_eq!(
            eval_day_state(true, Some(2), 2, TODAY, TODAY),
            TaskDayState::Completed
        );
        assert_eq!(
            eval_day_state(true, Some(2), 5, TODAY, TODAY),
            TaskDayState::Completed
        );
        // 已错过：昨天未达标
        assert_eq!(
            eval_day_state(true, Some(2), 1, "2026-08-29", TODAY),
            TaskDayState::Missed
        );
        // 过去已达标仍是 completed（completed 优先于时间判断）
        assert_eq!(
            eval_day_state(true, Some(2), 2, "2026-08-29", TODAY),
            TaskDayState::Completed
        );
    }

    #[test]
    fn heatmap_six_states() {
        // 未来一律 future（即使适用）
        assert_eq!(
            eval_heatmap_state(true, Some(1), 0, "2026-09-01", TODAY).0,
            HeatmapState::Future
        );
        assert_eq!(
            eval_heatmap_state(false, Some(1), 0, "2026-08-29", TODAY).0,
            HeatmapState::NotApplicable
        );
        assert_eq!(
            eval_heatmap_state(true, Some(3), 0, "2026-08-29", TODAY).0,
            HeatmapState::Zero
        );
        let (s, rate, over) = eval_heatmap_state(true, Some(4), 1, "2026-08-29", TODAY);
        assert_eq!(s, HeatmapState::PartialLow);
        assert_eq!(rate, Some(0.25));
        assert!(!over);
        assert_eq!(
            eval_heatmap_state(true, Some(4), 2, "2026-08-29", TODAY).0,
            HeatmapState::PartialHigh
        );
        let (s, rate, over) = eval_heatmap_state(true, Some(2), 5, "2026-08-29", TODAY);
        assert_eq!(s, HeatmapState::Complete);
        assert_eq!(rate, Some(1.0));
        assert!(over);
    }

    #[test]
    fn summary_rates_and_streaks() {
        let days = vec![
            DayInput {
                state: HeatmapState::Complete,
                actual: 2,
                target: Some(2),
            },
            DayInput {
                state: HeatmapState::Complete,
                actual: 3,
                target: Some(2),
            },
            DayInput {
                state: HeatmapState::PartialHigh,
                actual: 1,
                target: Some(2),
            },
            DayInput {
                state: HeatmapState::NotApplicable,
                actual: 0,
                target: None,
            },
            DayInput {
                state: HeatmapState::Complete,
                actual: 2,
                target: Some(2),
            },
            DayInput {
                state: HeatmapState::Future,
                actual: 0,
                target: Some(2),
            },
        ];
        let s = summarize(&days);
        assert_eq!(s.applicable_day_count, 4);
        assert_eq!(s.complete_day_count, 3);
        assert!((s.complete_day_rate - 0.75).abs() < 1e-9);
        assert_eq!(s.actual_count, 8);
        // 不适用日打断连续：2 连胜被 partial 打断，不适用日再清零，最后 1
        assert_eq!(s.longest_streak, 2);
        assert_eq!(s.current_streak, 1);
    }

    #[test]
    fn today_rate_caps_overachievement() {
        assert!((today_rate(&[(2, 2), (1, 2)]) - 0.75).abs() < 1e-9);
        // 超额封顶：5/2 只计 2
        assert!((today_rate(&[(5, 2), (0, 2)]) - 0.5).abs() < 1e-9);
        assert_eq!(today_rate(&[]), 0.0);
    }
}
