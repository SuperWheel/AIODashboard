//! Completion Repository：打卡账本（append-only）。

use chrono::{DateTime, Utc};
use dashboard_domain::{id_prefix, new_id, CompletionKind, CompletionRecord};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::{read_time, write_time};

const COLS: &str =
    "id, task_id, operation_id, value, kind, compensates_record_id, logical_day, source, created_at";

fn map_record(row: &Row) -> rusqlite::Result<CompletionRecord> {
    let kind_raw: String = row.get(4)?;
    Ok(CompletionRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        operation_id: row.get(2)?,
        value: row.get(3)?,
        kind: CompletionKind::parse(&kind_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                format!("invalid completion kind: {kind_raw}").into(),
            )
        })?,
        compensates_record_id: row.get(5)?,
        logical_day: row.get(6)?,
        source: row.get(7)?,
        created_at: read_time(row, 8)?,
    })
}

#[derive(Debug)]
pub struct NewCompletion {
    pub task_id: String,
    pub operation_id: String,
    pub value: i64,
    pub kind: CompletionKind,
    pub compensates_record_id: Option<String>,
    pub logical_day: String,
    pub source: String,
}

/// 追加一条账本记录。operation_id 冲突时返回 Ok(None)（幂等重放，调用方读原记录）。
pub fn append(
    conn: &Connection,
    input: &NewCompletion,
) -> rusqlite::Result<Option<CompletionRecord>> {
    let record = CompletionRecord {
        id: new_id(id_prefix::COMPLETION),
        task_id: input.task_id.clone(),
        operation_id: input.operation_id.clone(),
        value: input.value,
        kind: input.kind,
        compensates_record_id: input.compensates_record_id.clone(),
        logical_day: input.logical_day.clone(),
        source: input.source.clone(),
        created_at: Utc::now(),
    };
    let affected = conn.execute(
        "INSERT OR IGNORE INTO completion_records
         (id, task_id, operation_id, value, kind, compensates_record_id, logical_day, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record.id,
            record.task_id,
            record.operation_id,
            record.value,
            record.kind.as_str(),
            record.compensates_record_id,
            record.logical_day,
            record.source,
            write_time(record.created_at),
        ],
    )?;
    if affected == 0 {
        return Ok(None);
    }
    Ok(Some(record))
}

pub fn get_by_operation(
    conn: &Connection,
    operation_id: &str,
) -> rusqlite::Result<Option<CompletionRecord>> {
    conn.query_row(
        &format!("SELECT {COLS} FROM completion_records WHERE operation_id = ?1"),
        params![operation_id],
        map_record,
    )
    .optional()
}

/// 当日净计数（Σvalue，下限 0）。
pub fn day_count(conn: &Connection, task_id: &str, logical_day: &str) -> rusqlite::Result<i64> {
    let sum: i64 = conn.query_row(
        "SELECT COALESCE(SUM(value), 0) FROM completion_records
         WHERE task_id = ?1 AND logical_day = ?2",
        params![task_id, logical_day],
        |r| r.get(0),
    )?;
    Ok(sum.max(0))
}

/// 某任务在 [from_day, to_day] 范围内的每日净计数（logical_day 字典序即时间序）。
pub fn counts_between(
    conn: &Connection,
    task_id: &str,
    from_day: &str,
    to_day: &str,
) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT logical_day, COALESCE(SUM(value), 0) FROM completion_records
         WHERE task_id = ?1 AND logical_day >= ?2 AND logical_day <= ?3
         GROUP BY logical_day",
    )?;
    let rows = stmt.query_map(params![task_id, from_day, to_day], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

/// 当日净计数（Σvalue，不钳位；需要真实账面差额时使用，如补满目标）。
pub fn day_sum(conn: &Connection, task_id: &str, logical_day: &str) -> rusqlite::Result<i64> {
    let sum: i64 = conn.query_row(
        "SELECT COALESCE(SUM(value), 0) FROM completion_records
         WHERE task_id = ?1 AND logical_day = ?2",
        params![task_id, logical_day],
        |r| r.get(0),
    )?;
    Ok(sum)
}

/// 某任务当日最近一条未被补偿的正向记录（decrement 的补偿对象）。
pub fn latest_uncompensated_positive(
    conn: &Connection,
    task_id: &str,
    logical_day: &str,
) -> rusqlite::Result<Option<CompletionRecord>> {
    conn.query_row(
        &format!(
            "SELECT {COLS} FROM completion_records c
             WHERE c.task_id = ?1 AND c.logical_day = ?2 AND c.value > 0
               AND NOT EXISTS (
                   SELECT 1 FROM completion_records x
                   WHERE x.compensates_record_id = c.id
               )
             ORDER BY c.created_at DESC, c.id DESC LIMIT 1"
        ),
        params![task_id, logical_day],
        map_record,
    )
    .optional()
}

/// 某任务当日全部未被补偿的正向记录（reopen 清零的补偿对象，新→旧）。
pub fn uncompensated_positives_on(
    conn: &Connection,
    task_id: &str,
    logical_day: &str,
) -> rusqlite::Result<Vec<CompletionRecord>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM completion_records c
         WHERE c.task_id = ?1 AND c.logical_day = ?2 AND c.value > 0
           AND NOT EXISTS (
               SELECT 1 FROM completion_records x
               WHERE x.compensates_record_id = c.id
           )
         ORDER BY c.created_at DESC, c.id DESC"
    ))?;
    let rows = stmt.query_map(params![task_id, logical_day], map_record)?;
    rows.collect()
}

/// 某任务最近一条未被补偿的正向记录（undo 的目标；不限定当日）。
pub fn latest_uncompensated_positive_any_day(
    conn: &Connection,
    task_id: &str,
) -> rusqlite::Result<Option<CompletionRecord>> {
    conn.query_row(
        &format!(
            "SELECT {COLS} FROM completion_records c
             WHERE c.task_id = ?1 AND c.value > 0
               AND NOT EXISTS (
                   SELECT 1 FROM completion_records x
                   WHERE x.compensates_record_id = c.id
               )
             ORDER BY c.created_at DESC, c.id DESC LIMIT 1"
        ),
        params![task_id],
        map_record,
    )
    .optional()
}

/// 某任务指定记录是否已被补偿。
pub fn is_compensated(conn: &Connection, record_id: &str) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM completion_records WHERE compensates_record_id = ?1",
        params![record_id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// 一组任务当日的净计数（今日总览批量取数）。
pub fn day_counts_for_tasks(
    conn: &Connection,
    logical_day: &str,
) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, COALESCE(SUM(value), 0) FROM completion_records
         WHERE logical_day = ?1 GROUP BY task_id",
    )?;
    let rows = stmt.query_map(params![logical_day], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

/// 近 N 个逻辑日内，每日「完成打卡任务数」（用于统计/错过提示等聚合）。
pub fn completed_task_counts_between(
    conn: &Connection,
    from_day: &str,
    to_day: &str,
) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT logical_day, COUNT(DISTINCT task_id) FROM completion_records
         WHERE logical_day >= ?1 AND logical_day <= ?2 AND value > 0
         GROUP BY logical_day",
    )?;
    let rows = stmt.query_map(params![from_day, to_day], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

/// 时间段内产生的正向打卡总数（兼容旧「今日完成」统计语义）。
pub fn count_adds_between(
    conn: &Connection,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM completion_records
         WHERE value > 0 AND created_at >= ?1 AND created_at < ?2",
        params![write_time(from), write_time(to)],
        |r| r.get(0),
    )
}
