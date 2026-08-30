//! 逻辑日（本地 YYYY-MM-DD）纯函数工具。
//!
//! 只处理日期字符串与日历运算，不涉及时区换算——时区边界唯一入口是
//! `context_service::local_today_range / local_today`。

use chrono::{Datelike, NaiveDate};

use crate::{CoreError, CoreResult};

pub fn parse_day(s: &str) -> CoreResult<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| CoreError::Validation(format!("无效的逻辑日 '{s}'（应为 YYYY-MM-DD）")))
}

pub fn fmt_day(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn add_days(day: &str, n: i64) -> CoreResult<String> {
    let d = parse_day(day)?;
    d.checked_add_signed(chrono::Duration::days(n))
        .map(fmt_day)
        .ok_or_else(|| CoreError::Validation(format!("日期溢出: {day} + {n}d")))
}

/// 两个逻辑日相差的自然日数（b - a）。
pub fn days_between(a: &str, b: &str) -> CoreResult<i64> {
    Ok((parse_day(b)? - parse_day(a)?).num_days())
}

/// 周起始为周一（本地习惯，见 design.md）。返回 [周一, 周日]。
pub fn week_range(anchor: &str) -> CoreResult<(String, String)> {
    let d = parse_day(anchor)?;
    let offset = d.weekday().num_days_from_monday() as i64;
    let start = d - chrono::Duration::days(offset);
    let end = start + chrono::Duration::days(6);
    Ok((fmt_day(start), fmt_day(end)))
}

/// 月范围 [1 日, 月末]。
pub fn month_range(anchor: &str) -> CoreResult<(String, String)> {
    let d = parse_day(anchor)?;
    let start = NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
        .ok_or_else(|| CoreError::Validation(format!("无效月份: {anchor}")))?;
    let next = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1)
    }
    .ok_or_else(|| CoreError::Validation(format!("无效月份: {anchor}")))?;
    let end = next - chrono::Duration::days(1);
    Ok((fmt_day(start), fmt_day(end)))
}

/// 年范围 [1月1日, 12月31日]。
pub fn year_range(anchor: &str) -> CoreResult<(String, String)> {
    let d = parse_day(anchor)?;
    let start = NaiveDate::from_ymd_opt(d.year(), 1, 1)
        .ok_or_else(|| CoreError::Validation(format!("无效年份: {anchor}")))?;
    let end = NaiveDate::from_ymd_opt(d.year(), 12, 31)
        .ok_or_else(|| CoreError::Validation(format!("无效年份: {anchor}")))?;
    Ok((fmt_day(start), fmt_day(end)))
}

/// 闭区间逐日迭代（from <= day <= to）。
pub fn days_inclusive(from: &str, to: &str) -> CoreResult<Vec<String>> {
    let mut d = parse_day(from)?;
    let end = parse_day(to)?;
    if d > end {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    while d <= end {
        out.push(fmt_day(d));
        d += chrono::Duration::days(1);
    }
    Ok(out)
}

/// 周几索引：0=周一 … 6=周日。
pub fn weekday_index(day: &str) -> CoreResult<i64> {
    Ok(parse_day(day)?.weekday().num_days_from_monday() as i64)
}

/// 相对周一的年内周列索引：以 1 月 1 日所在周的周一为第 0 列。
pub fn year_week_index(day: &str) -> CoreResult<i64> {
    let d = parse_day(day)?;
    let jan1 = NaiveDate::from_ymd_opt(d.year(), 1, 1)
        .ok_or_else(|| CoreError::Validation(format!("无效年份: {day}")))?;
    let first_monday = jan1 - chrono::Duration::days(jan1.weekday().num_days_from_monday() as i64);
    Ok((d - first_monday).num_days() / 7)
}

/// 1 月 1 日的周几索引（年热力图前置空格数，周一=0）。
pub fn year_leading_empty(year_anchor: &str) -> CoreResult<i64> {
    let d = parse_day(year_anchor)?;
    let jan1 = NaiveDate::from_ymd_opt(d.year(), 1, 1)
        .ok_or_else(|| CoreError::Validation(format!("无效年份: {year_anchor}")))?;
    Ok(jan1.weekday().num_days_from_monday() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week_range_monday_start() {
        // 2026-08-30 是周日
        let (s, e) = week_range("2026-08-30").unwrap();
        assert_eq!((s.as_str(), e.as_str()), ("2026-08-24", "2026-08-30"));
        // 2026-08-31 是周一
        let (s, e) = week_range("2026-08-31").unwrap();
        assert_eq!((s.as_str(), e.as_str()), ("2026-08-31", "2026-09-06"));
    }

    #[test]
    fn month_and_year_range() {
        assert_eq!(
            month_range("2026-02-10").unwrap(),
            ("2026-02-01".to_string(), "2026-02-28".to_string())
        );
        assert_eq!(
            year_range("2026-08-30").unwrap(),
            ("2026-01-01".to_string(), "2026-12-31".to_string())
        );
    }

    #[test]
    fn days_between_natural_days() {
        assert_eq!(days_between("2026-08-01", "2026-08-30").unwrap(), 29);
        assert_eq!(days_between("2026-08-30", "2026-08-30").unwrap(), 0);
        assert_eq!(days_between("2026-08-30", "2026-08-01").unwrap(), -29);
    }
}
