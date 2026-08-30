//! Note 用例。

use dashboard_domain::{Actor, Note};
use dashboard_storage::note_repo;
use rusqlite::Connection;

use crate::{log_activity, CoreError, CoreResult};

fn validate_title(title: &str) -> CoreResult<String> {
    let t = title.trim().to_string();
    if t.chars().count() > 300 {
        return Err(CoreError::Validation(
            "笔记标题过长（最多 300 字符）".into(),
        ));
    }
    Ok(t)
}

pub fn create_note(conn: &Connection, title: &str, body: &str, actor: Actor) -> CoreResult<Note> {
    let title = validate_title(title)?;
    if title.is_empty() && body.trim().is_empty() {
        return Err(CoreError::Validation("笔记标题与内容不能同时为空".into()));
    }
    let note = note_repo::create(conn, &title, body, None)?;
    log_activity(
        conn,
        note.created_at,
        actor,
        "note.create",
        "note",
        Some(&note.id),
        &serde_json::json!({ "title": note.title }),
    );
    Ok(note)
}

pub fn get_note(conn: &Connection, id: &str) -> CoreResult<Note> {
    note_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("note {id}")))
}

pub fn update_note(
    conn: &Connection,
    id: &str,
    title: &str,
    body: &str,
    actor: Actor,
) -> CoreResult<Note> {
    get_note(conn, id)?;
    let title = validate_title(title)?;
    note_repo::update(conn, id, &title, body)?;
    let updated = get_note(conn, id)?;
    log_activity(
        conn,
        updated.updated_at,
        actor,
        "note.update",
        "note",
        Some(id),
        &serde_json::json!({}),
    );
    Ok(updated)
}

pub fn delete_note(conn: &Connection, id: &str, actor: Actor) -> CoreResult<()> {
    get_note(conn, id)?;
    note_repo::delete(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "note.delete",
        "note",
        Some(id),
        &serde_json::json!({}),
    );
    Ok(())
}

pub fn recent_notes(conn: &Connection, limit: i64) -> CoreResult<Vec<Note>> {
    Ok(note_repo::recent(conn, limit)?)
}
