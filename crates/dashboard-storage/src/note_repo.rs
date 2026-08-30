//! Note Repository

use chrono::Utc;

use dashboard_domain::{id_prefix, new_id, Note};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::{read_time, write_time};

const COLS: &str = "id, title, body, project_id, created_at, updated_at";

fn map_note(row: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        project_id: row.get(3)?,
        created_at: read_time(row, 4)?,
        updated_at: read_time(row, 5)?,
    })
}

pub fn create(
    conn: &Connection,
    title: &str,
    body: &str,
    project_id: Option<&str>,
) -> rusqlite::Result<Note> {
    let now = Utc::now();
    let note = Note {
        id: new_id(id_prefix::NOTE),
        title: title.to_string(),
        body: body.to_string(),
        project_id: project_id.map(|s| s.to_string()),
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO notes (id, title, body, project_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            note.id,
            note.title,
            note.body,
            note.project_id,
            write_time(note.created_at),
            write_time(note.updated_at),
        ],
    )?;
    Ok(note)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<Note>> {
    conn.query_row(
        &format!("SELECT {COLS} FROM notes WHERE id = ?1"),
        params![id],
        map_note,
    )
    .optional()
}

pub fn update(conn: &Connection, id: &str, title: &str, body: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notes SET title=?2, body=?3, updated_at=?4 WHERE id=?1",
        params![id, title, body, write_time(Utc::now())],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])
}

/// 最近更新的笔记。
pub fn recent(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<Note>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM notes ORDER BY updated_at DESC LIMIT {limit}"
    ))?;
    let rows = stmt.query_map([], map_note)?;
    rows.collect()
}

pub fn search(conn: &Connection, needle_lower: &str, limit: i64) -> rusqlite::Result<Vec<Note>> {
    let like = format!("%{needle_lower}%");
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM notes
         WHERE lower(title) LIKE ?1 OR lower(body) LIKE ?1
         ORDER BY updated_at DESC LIMIT {limit}"
    ))?;
    let rows = stmt.query_map(params![like], map_note)?;
    rows.collect()
}
