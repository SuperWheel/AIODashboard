//! Project 用例。

use dashboard_domain::{Actor, Project};
use dashboard_storage::{project_repo, task_repo};
use rusqlite::Connection;
use serde::Serialize;

use crate::{log_activity, CoreError, CoreResult};

#[derive(Debug, Serialize)]
pub struct ProjectWithStats {
    #[serde(flatten)]
    pub project: Project,
    pub open_tasks: i64,
}

fn validate_name(name: &str) -> CoreResult<String> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return Err(CoreError::Validation("项目名称不能为空".into()));
    }
    if n.chars().count() > 100 {
        return Err(CoreError::Validation(
            "项目名称过长（最多 100 字符）".into(),
        ));
    }
    Ok(n)
}

pub fn create_project(
    conn: &Connection,
    name: &str,
    description: &str,
    actor: Actor,
) -> CoreResult<Project> {
    let name = validate_name(name)?;
    let p = project_repo::create(conn, &name, description.trim())?;
    log_activity(
        conn,
        p.created_at,
        actor,
        "project.create",
        "project",
        Some(&p.id),
        &serde_json::json!({ "name": p.name }),
    );
    crate::snapshot::refresh(conn);
    Ok(p)
}

pub fn get_project(conn: &Connection, id: &str) -> CoreResult<Project> {
    project_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("project {id}")))
}

pub fn list_projects(conn: &Connection, include_archived: bool) -> CoreResult<Vec<Project>> {
    Ok(project_repo::list(conn, include_archived)?)
}

pub fn list_projects_with_stats(conn: &Connection) -> CoreResult<Vec<ProjectWithStats>> {
    let projects = project_repo::list(conn, true)?;
    let mut out = Vec::with_capacity(projects.len());
    for p in projects {
        let open = task_repo::list(
            conn,
            &task_repo::TaskQuery {
                project_id: Some(p.id.clone()),
                active_only: true,
                limit: 1_000,
                ..Default::default()
            },
        )?
        .len() as i64;
        out.push(ProjectWithStats {
            project: p,
            open_tasks: open,
        });
    }
    Ok(out)
}

/// 更新项目名称/描述（None = 保持不变）。
pub fn update_project(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    actor: Actor,
) -> CoreResult<Project> {
    let existing = get_project(conn, id)?;
    let new_name = match name {
        Some(n) => validate_name(n)?,
        None => existing.name.clone(),
    };
    let new_desc = match description {
        Some(d) => d.trim().to_string(),
        None => existing.description.clone(),
    };
    project_repo::update(conn, id, &new_name, &new_desc)?;
    let updated = get_project(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "project.update",
        "project",
        Some(id),
        &serde_json::json!({
            "before": { "name": existing.name, "description": existing.description },
            "after": { "name": updated.name, "description": updated.description },
        }),
    );
    crate::snapshot::refresh(conn);
    Ok(updated)
}

/// 归档项目（不删除数据）。
pub fn archive_project(conn: &Connection, id: &str, actor: Actor) -> CoreResult<()> {
    get_project(conn, id)?;
    project_repo::set_status(conn, id, dashboard_domain::ProjectStatus::Archived)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "project.archive",
        "project",
        Some(id),
        &serde_json::json!({}),
    );
    crate::snapshot::refresh(conn);
    Ok(())
}

pub fn delete_project(conn: &Connection, id: &str, dry_run: bool, actor: Actor) -> CoreResult<()> {
    get_project(conn, id)?;
    if dry_run {
        return Ok(());
    }
    // tasks.project_id 由 ON DELETE SET NULL 处理
    project_repo::delete(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "project.delete",
        "project",
        Some(id),
        &serde_json::json!({}),
    );
    crate::snapshot::refresh(conn);
    Ok(())
}
