//! 时间存取工具：统一 RFC3339 (UTC) 文本格式。

use chrono::{DateTime, Utc};
use rusqlite::Row;

pub fn write_time(t: DateTime<Utc>) -> String {
    t.to_rfc3339()
}

fn parse_or_err(idx: usize, s: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                idx,
                rusqlite::types::Type::Text,
                format!("invalid timestamp '{s}': {e}").into(),
            )
        })
}

pub fn read_time(row: &Row, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let s: String = row.get(idx)?;
    parse_or_err(idx, s)
}

#[allow(dead_code)]
pub fn read_time_opt(row: &Row, idx: usize) -> rusqlite::Result<Option<DateTime<Utc>>> {
    let s: Option<String> = row.get(idx)?;
    match s {
        None => Ok(None),
        Some(s) => parse_or_err(idx, s).map(Some),
    }
}
