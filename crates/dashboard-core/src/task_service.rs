//! Task 用例：创建 / 查询 / 更新 / 归档 / 删除（长期打卡任务）。

use dashboard_domain::{Actor, CardStyle, Task, TaskStatus, TASK_COLOR_PRESETS};
use dashboard_storage::{period_repo, project_repo, task_repo, task_repo::TaskQuery};
use serde::Serialize;

use crate::context_service::local_today;
use crate::{log_activity, snapshot, CoreError, CoreResult};

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub title: String,
    pub icon: String,
    pub color_hex: String,
    pub unit: String,
    /// 每日目标次数（1–999）
    pub daily_target: i64,
    pub card_style: CardStyle,
    pub project_id: Option<String>,
    /// 创建时直接归入的主库
    pub library_id: Option<String>,
}

impl Default for CreateTaskInput {
    fn default() -> Self {
        Self {
            title: String::new(),
            icon: String::new(),
            color_hex: TASK_COLOR_PRESETS[0].to_string(),
            unit: String::new(),
            daily_target: 1,
            card_style: CardStyle::Day,
            project_id: None,
            library_id: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub icon: Option<String>,
    pub color_hex: Option<String>,
    pub unit: Option<String>,
    /// 修改每日目标（从当前逻辑日起生效）
    pub daily_target: Option<i64>,
    pub card_style: Option<CardStyle>,
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
        return Err(CoreError::Validation("任务标题不能为空".into()));
    }
    if t.chars().count() > 500 {
        return Err(CoreError::Validation(
            "任务标题过长（最多 500 字符）".into(),
        ));
    }
    Ok(t.to_string())
}

fn validate_target(target: i64) -> CoreResult<i64> {
    if !(1..=999).contains(&target) {
        return Err(CoreError::Validation("每日目标必须在 1–999 之间".into()));
    }
    Ok(target)
}

fn validate_color(hex: &str) -> CoreResult<String> {
    let h = hex.trim();
    // 只接受预设色或合法 hex，防止注入任意 CSS
    let ok = TASK_COLOR_PRESETS.contains(&h)
        || (h.len() == 7 && h.starts_with('#') && h[1..].chars().all(|c| c.is_ascii_hexdigit()));
    if !ok {
        return Err(CoreError::Validation(format!(
            "无效的主题色 '{hex}'（应为 #RRGGBB）"
        )));
    }
    Ok(h.to_string())
}

fn ensure_project_exists(conn: &rusqlite::Connection, pid: &str) -> CoreResult<()> {
    if project_repo::get(conn, pid)?.is_none() {
        return Err(CoreError::NotFound(format!("project {pid}")));
    }
    Ok(())
}

pub fn create_task(
    conn: &rusqlite::Connection,
    input: &CreateTaskInput,
    actor: Actor,
) -> CoreResult<Task> {
    let title = validate_title(&input.title)?;
    let target = validate_target(input.daily_target)?;
    let color = validate_color(&input.color_hex)?;
    if let Some(pid) = &input.project_id {
        ensure_project_exists(conn, pid)?;
    }
    if let Some(lid) = &input.library_id {
        crate::library_service::get_library(conn, lid)?;
    }
    let today = local_today();

    let task = task_repo::create(
        conn,
        &title,
        input.icon.trim(),
        &color,
        input.unit.trim(),
        input.card_style,
        input.project_id.as_deref(),
    )?;
    // 开放活动区间 + 首个目标区间（今天起生效）
    period_repo::insert_activity(conn, &task.id, &today, None)?;
    period_repo::insert_target(conn, &task.id, target, &today, None)?;
    if let Some(lid) = &input.library_id {
        period_repo::insert_membership(conn, &task.id, lid, &today)?;
    }

    log_activity(
        conn,
        task.created_at,
        actor,
        "task.create",
        "task",
        Some(&task.id),
        &serde_json::json!({ "title": task.title, "daily_target": target }),
    );
    snapshot::refresh(conn);
    Ok(task)
}

pub fn get_task(conn: &rusqlite::Connection, id: &str) -> CoreResult<Task> {
    task_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("task {id}")))
}

pub fn list_tasks(conn: &rusqlite::Connection, query: &TaskQuery) -> CoreResult<Vec<Task>> {
    Ok(task_repo::list(conn, query)?)
}

/// 归档任务：关闭当前活动区间（今日为最后一个有效日）。
pub fn archive_task(conn: &rusqlite::Connection, id: &str, actor: Actor) -> CoreResult<Task> {
    let existing = get_task(conn, id)?;
    if existing.status == TaskStatus::Archived {
        return Ok(existing); // 幂等
    }
    let today = local_today();
    task_repo::update(
        conn,
        id,
        &task_repo::TaskPatch {
            status: Some(TaskStatus::Archived),
            ..Default::default()
        },
    )?;
    period_repo::close_open_activity(conn, id, &today)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.archive",
        "task",
        Some(id),
        &serde_json::json!({ "title": existing.title }),
    );
    snapshot::refresh(conn);
    get_task(conn, id)
}

/// 恢复任务：重新启用并开启新活动区间（归档与恢复之间显示为不适用）。
pub fn restore_task(conn: &rusqlite::Connection, id: &str, actor: Actor) -> CoreResult<Task> {
    let existing = get_task(conn, id)?;
    if existing.status == TaskStatus::Active {
        return Ok(existing); // 幂等
    }
    let today = local_today();
    task_repo::update(
        conn,
        id,
        &task_repo::TaskPatch {
            status: Some(TaskStatus::Active),
            ..Default::default()
        },
    )?;
    // 归档闭合区间时 end_day=今天（今天已不适用），恢复直接从今天开新区间
    period_repo::insert_activity(conn, id, &today, None)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.restore",
        "task",
        Some(id),
        &serde_json::json!({ "title": existing.title }),
    );
    snapshot::refresh(conn);
    get_task(conn, id)
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
    if let Some(icon) = &input.icon {
        patch.icon = Some(icon.trim().to_string());
    }
    if let Some(color) = &input.color_hex {
        patch.color_hex = Some(validate_color(color)?);
    }
    if let Some(unit) = &input.unit {
        patch.unit = Some(unit.trim().to_string());
    }
    if let Some(style) = input.card_style {
        patch.card_style = Some(style);
    }
    if let Some(pid) = &input.project_id {
        if let Some(p) = pid {
            ensure_project_exists(conn, p)?;
        }
        patch.project_id = Some(pid.clone());
    }
    task_repo::update(conn, id, &patch)?;

    // 修改每日目标：闭合当前区间，明天起生效新区间（今天仍按旧目标，避免今日已打卡数据口径漂移）
    if let Some(target) = input.daily_target {
        let target = validate_target(target)?;
        let today = local_today();
        let current = period_repo::target_on(conn, id, &today)?.map(|p| p.target);
        if current != Some(target) {
            let start = crate::logical_day::add_days(&today, 1)?;
            period_repo::close_open_target(conn, id, &start)?;
            period_repo::insert_target(conn, id, target, &start, None)?;
        }
    }

    let updated = get_task(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
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
        chrono::Utc::now(),
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

/// 清理所有已归档任务（支持 dry run）。
pub fn clear_archived_tasks(
    conn: &rusqlite::Connection,
    dry_run: bool,
    actor: Actor,
) -> CoreResult<DeleteReport> {
    let archived = task_repo::list(
        conn,
        &TaskQuery {
            status: Some(TaskStatus::Archived),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let ids: Vec<String> = archived.iter().map(|t| t.id.clone()).collect();
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
        chrono::Utc::now(),
        actor,
        "task.clear_archived",
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
