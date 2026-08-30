//! Inbox Repository

use chrono::{DateTime, Utc};
use dashboard_domain::{id_prefix, new_id, InboxItem, InboxStatus};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::read_time;

const COLS: &str = "id, content, source, status, created_at";

fn map_inbox(row: &Row) -> rusqlite::Result<InboxItem> {
    let status_raw: String = row.get(3)?;
    Ok(InboxItem {
        id: row.get(0)?,
        content: row.get(1)?,
        source: row.get(2)?,
        status: match status_raw.as_str() {
            "processed" => InboxStatus::Processed,
            _ => InboxStatus::Open,
        },
        created_at: read_time(row, 4)?,
    })
}

pub fn create(conn: &Connection, content: &str, source: &str) -> rusqlite::Result<InboxItem> {
    let item = InboxItem {
        id: new_id(id_prefix::INBOX),
        content: content.to_string(),
        source: source.to_string(),
        status: InboxStatus::Open,
        created_at: Utc::now(),
    };
    conn.execute(
        "INSERT INTO inbox_items (id, content, source, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            item.id,
            item.content,
            item.source,
            item.status.as_str(),
            read_time_str(item.created_at)
        ],
    )?;
    Ok(item)
}

fn read_time_str(t: DateTime<Utc>) -> String {
    crate::timeutil::write_time(t)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<InboxItem>> {
    conn.query_row(
        &format!("SELECT {COLS} FROM inbox_items WHERE id = ?1"),
        params![id],
        map_inbox,
    )
    .optional()
}

pub fn list(conn: &Connection, include_processed: bool) -> rusqlite::Result<Vec<InboxItem>> {
    let sql = if include_processed {
        format!("SELECT {COLS} FROM inbox_items ORDER BY created_at DESC")
    } else {
        format!("SELECT {COLS} FROM inbox_items WHERE status='open' ORDER BY created_at DESC")
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_inbox)?;
    rows.collect()
}

pub fn set_status(conn: &Connection, id: &str, status: InboxStatus) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE inbox_items SET status=?2 WHERE id=?1",
        params![id, status.as_str()],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM inbox_items WHERE id = ?1", params![id])
}

pub fn count_open(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM inbox_items WHERE status='open'",
        [],
        |r| r.get(0),
    )
}

pub fn search(
    conn: &Connection,
    needle_lower: &str,
    limit: i64,
) -> rusqlite::Result<Vec<InboxItem>> {
    let like = format!("%{needle_lower}%");
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM inbox_items
         WHERE lower(content) LIKE ?1 ORDER BY created_at DESC LIMIT {limit}"
    ))?;
    let rows = stmt.query_map(params![like], map_inbox)?;
    rows.collect()
}
