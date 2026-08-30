//! Period Repository：目标区间 / 活动区间 / 主库归属区间。

use dashboard_domain::{id_prefix, new_id, MembershipPeriod, TaskActivityPeriod, TaskTargetPeriod};
use rusqlite::{params, Connection, Row};

fn map_target(row: &Row) -> rusqlite::Result<TaskTargetPeriod> {
    Ok(TaskTargetPeriod {
        id: row.get(0)?,
        task_id: row.get(1)?,
        target: row.get(2)?,
        start_day: row.get(3)?,
        end_day: row.get(4)?,
    })
}

fn map_activity(row: &Row) -> rusqlite::Result<TaskActivityPeriod> {
    Ok(TaskActivityPeriod {
        id: row.get(0)?,
        task_id: row.get(1)?,
        start_day: row.get(2)?,
        end_day: row.get(3)?,
    })
}

fn map_membership(row: &Row) -> rusqlite::Result<MembershipPeriod> {
    Ok(MembershipPeriod {
        id: row.get(0)?,
        task_id: row.get(1)?,
        library_id: row.get(2)?,
        start_day: row.get(3)?,
        end_day: row.get(4)?,
    })
}

const TARGET_COLS: &str = "id, task_id, target, start_day, end_day";
const ACTIVITY_COLS: &str = "id, task_id, start_day, end_day";
const MEMBERSHIP_COLS: &str = "id, task_id, library_id, start_day, end_day";

// ---------- 目标区间 ----------

pub fn list_targets(conn: &Connection, task_id: &str) -> rusqlite::Result<Vec<TaskTargetPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TARGET_COLS} FROM task_target_periods WHERE task_id = ?1
         ORDER BY start_day ASC"
    ))?;
    let rows = stmt.query_map(params![task_id], map_target)?;
    rows.collect()
}

/// 某日生效的目标区间（start_day <= day < end_day 或 end_day 为空）。
pub fn target_on(
    conn: &Connection,
    task_id: &str,
    day: &str,
) -> rusqlite::Result<Option<TaskTargetPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TARGET_COLS} FROM task_target_periods
         WHERE task_id = ?1 AND start_day <= ?2 AND (end_day IS NULL OR end_day > ?2)
         ORDER BY start_day DESC LIMIT 1"
    ))?;
    let mut rows = stmt.query_map(params![task_id, day], map_target)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

pub fn insert_target(
    conn: &Connection,
    task_id: &str,
    target: i64,
    start_day: &str,
    end_day: Option<&str>,
) -> rusqlite::Result<TaskTargetPeriod> {
    let p = TaskTargetPeriod {
        id: new_id(id_prefix::TARGET_PERIOD),
        task_id: task_id.to_string(),
        target,
        start_day: start_day.to_string(),
        end_day: end_day.map(|s| s.to_string()),
    };
    conn.execute(
        "INSERT INTO task_target_periods (id, task_id, target, start_day, end_day)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![p.id, p.task_id, p.target, p.start_day, p.end_day],
    )?;
    Ok(p)
}

/// 闭合当前开放的目标区间（end_day = day）。
pub fn close_open_target(conn: &Connection, task_id: &str, day: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE task_target_periods SET end_day = ?2
         WHERE task_id = ?1 AND end_day IS NULL",
        params![task_id, day],
    )
}

// ---------- 活动区间 ----------

pub fn list_activity(
    conn: &Connection,
    task_id: &str,
) -> rusqlite::Result<Vec<TaskActivityPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ACTIVITY_COLS} FROM task_activity_periods WHERE task_id = ?1
         ORDER BY start_day ASC"
    ))?;
    let rows = stmt.query_map(params![task_id], map_activity)?;
    rows.collect()
}

/// 某日是否在活动区间内。
pub fn is_active_on(conn: &Connection, task_id: &str, day: &str) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM task_activity_periods
         WHERE task_id = ?1 AND start_day <= ?2 AND (end_day IS NULL OR end_day > ?2)",
        params![task_id, day],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn insert_activity(
    conn: &Connection,
    task_id: &str,
    start_day: &str,
    end_day: Option<&str>,
) -> rusqlite::Result<TaskActivityPeriod> {
    let p = TaskActivityPeriod {
        id: new_id(id_prefix::ACTIVITY_PERIOD),
        task_id: task_id.to_string(),
        start_day: start_day.to_string(),
        end_day: end_day.map(|s| s.to_string()),
    };
    conn.execute(
        "INSERT INTO task_activity_periods (id, task_id, start_day, end_day)
         VALUES (?1, ?2, ?3, ?4)",
        params![p.id, p.task_id, p.start_day, p.end_day],
    )?;
    Ok(p)
}

/// 闭合当前开放的活动区间（end_day = day）。
pub fn close_open_activity(conn: &Connection, task_id: &str, day: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE task_activity_periods SET end_day = ?2
         WHERE task_id = ?1 AND end_day IS NULL",
        params![task_id, day],
    )
}

// ---------- 归属区间 ----------

pub fn list_memberships(
    conn: &Connection,
    task_id: &str,
) -> rusqlite::Result<Vec<MembershipPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEMBERSHIP_COLS} FROM task_library_membership_periods WHERE task_id = ?1
         ORDER BY start_day ASC"
    ))?;
    let rows = stmt.query_map(params![task_id], map_membership)?;
    rows.collect()
}

/// 某日生效的归属（一个任务同日最多一个主库）。
pub fn membership_on(
    conn: &Connection,
    task_id: &str,
    day: &str,
) -> rusqlite::Result<Option<MembershipPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEMBERSHIP_COLS} FROM task_library_membership_periods
         WHERE task_id = ?1 AND start_day <= ?2 AND (end_day IS NULL OR end_day > ?2)
         ORDER BY start_day DESC LIMIT 1"
    ))?;
    let mut rows = stmt.query_map(params![task_id, day], map_membership)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

pub fn insert_membership(
    conn: &Connection,
    task_id: &str,
    library_id: &str,
    start_day: &str,
) -> rusqlite::Result<MembershipPeriod> {
    let p = MembershipPeriod {
        id: new_id(id_prefix::MEMBERSHIP_PERIOD),
        task_id: task_id.to_string(),
        library_id: library_id.to_string(),
        start_day: start_day.to_string(),
        end_day: None,
    };
    conn.execute(
        "INSERT INTO task_library_membership_periods (id, task_id, library_id, start_day, end_day)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![p.id, p.task_id, p.library_id, p.start_day],
    )?;
    Ok(p)
}

/// 闭合当前开放的归属区间（end_day = day）。
pub fn close_open_membership(
    conn: &Connection,
    task_id: &str,
    day: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE task_library_membership_periods SET end_day = ?2
         WHERE task_id = ?1 AND end_day IS NULL",
        params![task_id, day],
    )
}

/// 某日主库的全部有效直属任务 id。
pub fn library_task_ids_on(
    conn: &Connection,
    library_id: &str,
    day: &str,
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT task_id FROM task_library_membership_periods
         WHERE library_id = ?1 AND start_day <= ?2 AND (end_day IS NULL OR end_day > ?2)",
    )?;
    let rows = stmt.query_map(params![library_id, day], |r| r.get::<_, String>(0))?;
    rows.collect()
}

/// 主库当前（开放区间）直属任务 id。
pub fn library_current_task_ids(
    conn: &Connection,
    library_id: &str,
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT task_id FROM task_library_membership_periods
         WHERE library_id = ?1 AND end_day IS NULL",
    )?;
    let rows = stmt.query_map(params![library_id], |r| r.get::<_, String>(0))?;
    rows.collect()
}

/// 某日主库归属记录（供主库热力图逐日聚合）。
pub fn memberships_covering(
    conn: &Connection,
    library_id: &str,
    day: &str,
) -> rusqlite::Result<Vec<MembershipPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEMBERSHIP_COLS} FROM task_library_membership_periods
         WHERE library_id = ?1 AND start_day <= ?2 AND (end_day IS NULL OR end_day > ?2)"
    ))?;
    let rows = stmt.query_map(params![library_id, day], map_membership)?;
    rows.collect()
}

/// 列出某任务在 [from_day, to_day] 范围内与任一主库相关的归属区间（供批量计算）。
pub fn memberships_in_range(
    conn: &Connection,
    task_id: &str,
    from_day: &str,
    to_day: &str,
) -> rusqlite::Result<Vec<MembershipPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEMBERSHIP_COLS} FROM task_library_membership_periods
         WHERE task_id = ?1 AND start_day <= ?3 AND (end_day IS NULL OR end_day > ?2)
         ORDER BY start_day ASC"
    ))?;
    let rows = stmt.query_map(params![task_id, from_day, to_day], map_membership)?;
    rows.collect()
}

/// 列出主库在 [from_day, to_day] 范围内出现过的归属区间（供主库热力图批量计算）。
pub fn library_memberships_in_range(
    conn: &Connection,
    library_id: &str,
    from_day: &str,
    to_day: &str,
) -> rusqlite::Result<Vec<MembershipPeriod>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEMBERSHIP_COLS} FROM task_library_membership_periods
         WHERE library_id = ?1 AND start_day <= ?3 AND (end_day IS NULL OR end_day > ?2)
         ORDER BY start_day ASC"
    ))?;
    let rows = stmt.query_map(params![library_id, from_day, to_day], map_membership)?;
    rows.collect()
}
