//! Task 用例：创建 / 查询 / 更新 / 完成 / 删除。

use chrono::{DateTime, Utc};
use dashboard_domain::{Actor, Task, TaskStatus};
use dashboard_storage::{project_repo, task_repo, task_repo::TaskQuery};
use serde::Serialize;

use crate::{log_activity, snapshot, CoreResult};

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub title: String,
    pub due_at: Option<DateTime<Utc>>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    /// Some(None) 表示清除期限
    pub due_at: Option<Option<DateTime<Utc>>>,
    pub status: Option<TaskStatus>,
    /// Some(None) 表示移出项目
    pub project_id: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct DeleteReport {
    pub dry_run: bool,
    pub affected_ids: Vec<String>,
}

fn validate_title(title: &str) -> CoreResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(crate::CoreError::Validation("任务标题不能为空".into()));
    }
    if t.chars().count() > 500 {
        return Err(crate::CoreError::Validation(
            "任务标题过长（最多 500 字符）".into(),
        ));
    }
    Ok(t.to_string())
}

fn ensure_project_exists(conn: &rusqlite::Connection, pid: &str) -> CoreResult<()> {
    if project_repo::get(conn, pid)?.is_none() {
        return Err(crate::CoreError::NotFound(format!("project {pid}")));
    }
    Ok(())
}

pub fn create_task(
    conn: &rusqlite::Connection,
    input: &CreateTaskInput,
    actor: Actor,
) -> CoreResult<Task> {
    let title = validate_title(&input.title)?;
    if let Some(pid) = &input.project_id {
        ensure_project_exists(conn, pid)?;
    }
    let task = task_repo::create(conn, &title, input.due_at, input.project_id.as_deref())?;
    log_activity(
        conn,
        task.created_at,
        actor,
        "task.create",
        "task",
        Some(&task.id),
        &serde_json::json!({ "title": task.title }),
    );
    snapshot::refresh(conn);
    Ok(task)
}

pub fn get_task(conn: &rusqlite::Connection, id: &str) -> CoreResult<Task> {
    task_repo::get(conn, id)?.ok_or_else(|| crate::CoreError::NotFound(format!("task {id}")))
}

pub fn list_tasks(conn: &rusqlite::Connection, query: &TaskQuery) -> CoreResult<Vec<Task>> {
    Ok(task_repo::list(conn, query)?)
}

/// 通用状态流转（todo/doing/done）。
pub fn set_status(
    conn: &rusqlite::Connection,
    id: &str,
    status: TaskStatus,
    actor: Actor,
) -> CoreResult<Task> {
    let existing = get_task(conn, id)?;
    if existing.status == status {
        // 幂等：重复完成不报错
        return Ok(existing);
    }
    task_repo::update(
        conn,
        id,
        &task_repo::TaskPatch {
            status: Some(status),
            ..Default::default()
        },
    )?;
    let updated = get_task(conn, id)?;
    log_activity(
        conn,
        Utc::now(),
        actor,
        &format!("task.status.{}", status.as_str()),
        "task",
        Some(id),
        &serde_json::json!({ "before": existing.status.as_str(), "after": status.as_str() }),
    );
    snapshot::refresh(conn);
    Ok(updated)
}

pub fn complete_task(conn: &rusqlite::Connection, id: &str, actor: Actor) -> CoreResult<Task> {
    set_status(conn, id, TaskStatus::Done, actor)
}

pub fn reopen_task(conn: &rusqlite::Connection, id: &str, actor: Actor) -> CoreResult<Task> {
    set_status(conn, id, TaskStatus::Todo, actor)
}

pub fn update_task(
    conn: &rusqlite::Connection,
    id: &str,
    input: &UpdateTaskInput,
    actor: Actor,
) -> CoreResult<Task> {
    let existing = get_task(conn, id)?;

    let mut patch = task_repo::TaskPatch::default();
    if let Some(title) = &input.title {
        patch.title = Some(validate_title(title)?);
    }
    if let Some(due) = input.due_at {
        patch.due_at = Some(due);
    }
    if let Some(status) = input.status {
        patch.status = Some(status);
    }
    if let Some(pid) = &input.project_id {
        if let Some(p) = pid {
            ensure_project_exists(conn, p)?;
        }
        patch.project_id = Some(pid.clone());
    }

    task_repo::update(conn, id, &patch)?;
    let updated = get_task(conn, id)?;
    log_activity(
        conn,
        Utc::now(),
        actor,
        "task.update",
        "task",
        Some(id),
        &serde_json::json!({
            "before_title": existing.title,
            "after_title": updated.title,
        }),
    );
    snapshot::refresh(conn);
    Ok(updated)
}

pub fn delete_task(
    conn: &rusqlite::Connection,
    id: &str,
    dry_run: bool,
    actor: Actor,
) -> CoreResult<DeleteReport> {
    let existing = get_task(conn, id)?;
    if dry_run {
        return Ok(DeleteReport {
            dry_run: true,
            affected_ids: vec![existing.id],
        });
    }
    task_repo::delete(conn, id)?;
    log_activity(
        conn,
        Utc::now(),
        actor,
        "task.delete",
        "task",
        Some(id),
        &serde_json::json!({ "title": existing.title }),
    );
    snapshot::refresh(conn);
    Ok(DeleteReport {
        dry_run: false,
        affected_ids: vec![existing.id],
    })
}

/// 清理所有已完成任务（支持 dry run）。返回受影响的 ID 列表。
pub fn clear_completed_tasks(
    conn: &rusqlite::Connection,
    dry_run: bool,
    actor: Actor,
) -> CoreResult<DeleteReport> {
    let done = task_repo::list(
        conn,
        &TaskQuery {
            status: Some(TaskStatus::Done),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let ids: Vec<String> = done.iter().map(|t| t.id.clone()).collect();
    if dry_run || ids.is_empty() {
        return Ok(DeleteReport {
            dry_run,
            affected_ids: ids,
        });
    }
    for id in &ids {
        task_repo::delete(conn, id)?;
    }
    log_activity(
        conn,
        Utc::now(),
        actor,
        "task.clear_completed",
        "task",
        None,
        &serde_json::json!({ "count": ids.len() }),
    );
    snapshot::refresh(conn);
    Ok(DeleteReport {
        dry_run: false,
        affected_ids: ids,
    })
}
