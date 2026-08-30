//! Inbox 用例：快速收集 + 后续整理（转 Task / 转 Note）。

use dashboard_domain::{Actor, InboxItem};
use dashboard_storage::{inbox_repo, note_repo, task_repo};
use rusqlite::Connection;
use serde::Serialize;

use crate::{log_activity, snapshot, CoreError, CoreResult};

pub fn add_item(
    conn: &Connection,
    content: &str,
    source: &str,
    actor: Actor,
) -> CoreResult<InboxItem> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(CoreError::Validation("Inbox 内容不能为空".into()));
    }
    let item = inbox_repo::create(conn, &content, source)?;
    log_activity(
        conn,
        item.created_at,
        actor,
        "inbox.add",
        "inbox",
        Some(&item.id),
        &serde_json::json!({ "source": item.source }),
    );
    snapshot::refresh(conn);
    Ok(item)
}

pub fn list_items(conn: &Connection, include_processed: bool) -> CoreResult<Vec<InboxItem>> {
    Ok(inbox_repo::list(conn, include_processed)?)
}

#[derive(Debug, Serialize)]
pub struct ProcessReport {
    pub inbox_id: String,
    pub created_type: &'static str,
    pub created_id: String,
}

/// 将 Inbox 条目转为 Task。
pub fn process_to_task(
    conn: &Connection,
    id: &str,
    due_at: Option<chrono::DateTime<chrono::Utc>>,
    actor: Actor,
) -> CoreResult<ProcessReport> {
    let item =
        inbox_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("inbox {id}")))?;
    if item.status == dashboard_domain::InboxStatus::Processed {
        return Err(CoreError::Conflict(format!("inbox {id} 已处理")));
    }
    let task = task_repo::create(conn, &item.content, due_at, None)?;
    inbox_repo::set_status(conn, id, dashboard_domain::InboxStatus::Processed)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "inbox.process",
        "inbox",
        Some(id),
        &serde_json::json!({ "to": "task", "task_id": task.id }),
    );
    snapshot::refresh(conn);
    Ok(ProcessReport {
        inbox_id: id.to_string(),
        created_type: "task",
        created_id: task.id,
    })
}

/// 将 Inbox 条目转为 Note。
pub fn process_to_note(conn: &Connection, id: &str, actor: Actor) -> CoreResult<ProcessReport> {
    let item =
        inbox_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("inbox {id}")))?;
    if item.status == dashboard_domain::InboxStatus::Processed {
        return Err(CoreError::Conflict(format!("inbox {id} 已处理")));
    }
    let title: String = item.content.chars().take(60).collect();
    let note = note_repo::create(conn, &title, &item.content, None)?;
    inbox_repo::set_status(conn, id, dashboard_domain::InboxStatus::Processed)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "inbox.process",
        "inbox",
        Some(id),
        &serde_json::json!({ "to": "note", "note_id": note.id }),
    );
    Ok(ProcessReport {
        inbox_id: id.to_string(),
        created_type: "note",
        created_id: note.id,
    })
}

pub fn delete_item(conn: &Connection, id: &str, actor: Actor) -> CoreResult<()> {
    inbox_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("inbox {id}")))?;
    inbox_repo::delete(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "inbox.delete",
        "inbox",
        Some(id),
        &serde_json::json!({}),
    );
    snapshot::refresh(conn);
    Ok(())
}
