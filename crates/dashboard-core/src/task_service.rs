//! Task 用例：创建 / 查询 / 更新 / 归档 / 删除（长期打卡任务）。

use dashboard_domain::{Actor, CardStyle, Recurrence, Task, TaskStatus, TASK_COLOR_PRESETS};
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
    /// 循环规则（默认每日）
    pub recurrence: Recurrence,
    /// 重要性星级 0–5（0=未评级；006）
    pub priority: i64,
    pub project_id: Option<String>,
    /// 创建时直接归入的重要日
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
            recurrence: Recurrence::Daily,
            priority: 0,
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
    /// 修改循环规则（明天起新区间生效，历史口径不变）
    pub recurrence: Option<Recurrence>,
    /// Some(None) 表示移出项目
    pub project_id: Option<Option<String>>,
    /// 重要性星级 0–5（006）
    pub priority: Option<i64>,
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

/// 星级合法域 0–5（0=未评级）。
fn validate_priority(p: i64) -> CoreResult<i64> {
    if !(0..=5).contains(&p) {
        return Err(CoreError::Validation(format!(
            "无效的星级 {p}（应为 0–5，0=未评级）"
        )));
    }
    Ok(p)
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
    let priority = validate_priority(input.priority)?;
    if let Some(pid) = &input.project_id {
        ensure_project_exists(conn, pid)?;
    }
    if let Some(lid) = &input.library_id {
        crate::library_service::get_library(conn, lid)?;
    }
    let today = local_today();
    // 新任务落到其星级档末尾（sort_order = 档内 max + 1024；空档从 1024 起）
    let sort_order = task_repo::max_sort_order_in_band(conn, priority)?
        .map(|m| m + 1024.0)
        .unwrap_or(1024.0);

    let task = task_repo::create(
        conn,
        &task_repo::NewTask {
            title,
            icon: input.icon.trim().to_string(),
            color_hex: color,
            unit: input.unit.trim().to_string(),
            card_style: input.card_style,
            priority,
            sort_order,
            project_id: input.project_id.clone(),
        },
    )?;
    // 开放活动区间 + 首个目标区间（今天起生效；循环锚点 = 今天）
    period_repo::insert_activity(conn, &task.id, &today, None)?;
    period_repo::insert_target(conn, &task.id, target, &input.recurrence, &today, None)?;
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
        &serde_json::json!({
            "title": task.title,
            "daily_target": target,
            "recurrence": serde_json::to_value(&input.recurrence).unwrap_or(serde_json::Value::Null),
        }),
    );
    snapshot::refresh(conn);
    let mut task = task;
    task.recurrence = input.recurrence.clone();
    Ok(task)
}

pub fn get_task(conn: &rusqlite::Connection, id: &str) -> CoreResult<Task> {
    let mut t =
        task_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("task {id}")))?;
    fill_current_recurrence(conn, &mut t)?;
    Ok(t)
}

pub fn list_tasks(conn: &rusqlite::Connection, query: &TaskQuery) -> CoreResult<Vec<Task>> {
    let mut tasks = task_repo::list(conn, query)?;
    for t in &mut tasks {
        fill_current_recurrence(conn, t)?;
    }
    Ok(tasks)
}

/// 归档任务 + 归档日（活动区间最后关闭日 MAX(end_day)），归档页分组/展示用。
#[derive(Debug, Serialize)]
pub struct ArchivedTask {
    #[serde(flatten)]
    pub task: Task,
    pub archived_day: Option<String>,
}

/// 归档任务列表（含归档日）。
pub fn list_archived_tasks(conn: &rusqlite::Connection) -> CoreResult<Vec<ArchivedTask>> {
    let tasks = list_tasks(
        conn,
        &TaskQuery {
            status: Some(TaskStatus::Archived),
            limit: 10_000,
            ..Default::default()
        },
    )?;
    let days = period_repo::archived_days(conn)?;
    Ok(tasks
        .into_iter()
        .map(|task| {
            let archived_day = days.get(&task.id).cloned();
            ArchivedTask { task, archived_day }
        })
        .collect())
}

/// 回填 Task.recurrence = 当日生效目标区间的循环规则（存储无此列，读侧组装）。
fn fill_current_recurrence(conn: &rusqlite::Connection, t: &mut Task) -> CoreResult<()> {
    if let Some(p) = period_repo::target_on(conn, &t.id, &local_today())? {
        t.recurrence = p.recurrence;
    }
    Ok(())
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

/// 解除「今天刚发生的归档」：恢复 Active 并重开今天关闭的活动区间
/// （不新开区间，避免归档-恢复-再归档在同日留下零长度区间）。
/// 仅当任务已归档且确有区间于今天关闭；返回是否执行。
/// 用途：一次性任务完成即自动归档后，当天内撤销完成应能回到待打卡。
pub fn unarchive_if_archived_today(
    conn: &rusqlite::Connection,
    id: &str,
    actor: Actor,
) -> CoreResult<bool> {
    let existing = get_task(conn, id)?;
    if existing.status != TaskStatus::Archived {
        return Ok(false);
    }
    let today = local_today();
    if !period_repo::closed_on(conn, id, &today)? {
        return Ok(false);
    }
    task_repo::update(
        conn,
        id,
        &task_repo::TaskPatch {
            status: Some(TaskStatus::Active),
            ..Default::default()
        },
    )?;
    period_repo::reopen_activity_closed_on(conn, id, &today)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.unarchive",
        "task",
        Some(id),
        &serde_json::json!({ "title": existing.title, "via": "undo_same_day" }),
    );
    snapshot::refresh(conn);
    Ok(true)
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

/// 拖拽落位（006）：同档内重排（new_priority=None）或跨档改级+落位。
/// before_id/after_id = 全局序列中落点的上下邻居（sort_order 取其中点；
/// 缺侧取 ±1024；均缺 = 目标档档末 max+1024）。
pub fn move_task_position(
    conn: &rusqlite::Connection,
    id: &str,
    new_priority: Option<i64>,
    before_id: Option<&str>,
    after_id: Option<&str>,
    actor: Actor,
) -> CoreResult<Task> {
    let existing = get_task(conn, id)?;
    let band = new_priority
        .map(validate_priority)
        .transpose()?
        .unwrap_or(existing.priority);
    if let Some(b) = before_id {
        get_task(conn, b)?;
    }
    if let Some(a) = after_id {
        get_task(conn, a)?;
    }
    let before_so = before_id
        .map(|b| get_task(conn, b).map(|t| t.sort_order))
        .transpose()?;
    let after_so = after_id
        .map(|a| get_task(conn, a).map(|t| t.sort_order))
        .transpose()?;
    let sort_order = match (before_so, after_so) {
        (Some(b), Some(a)) => (b + a) / 2.0,
        (Some(b), None) => b + 1024.0,
        (None, Some(a)) => a - 1024.0,
        (None, None) => task_repo::max_sort_order_in_band(conn, band)?
            .map(|m| m + 1024.0)
            .unwrap_or(1024.0),
    };
    task_repo::update(
        conn,
        id,
        &task_repo::TaskPatch {
            priority: Some(band),
            sort_order: Some(sort_order),
            ..Default::default()
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.move",
        "task",
        Some(id),
        &serde_json::json!({
            "title": existing.title,
            "priority": band,
            "sort_order": sort_order,
        }),
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
    if let Some(p) = input.priority {
        patch.priority = Some(validate_priority(p)?);
    }
    task_repo::update(conn, id, &patch)?;

    // 修改每日目标/循环规则：闭合当前区间，明天起新区间生效（今天仍按旧口径，
    // 避免今日已打卡数据在新旧规则间漂移）。两者只滚动一次区间。
    if input.daily_target.is_some() || input.recurrence.is_some() {
        let today = local_today();
        let current = period_repo::target_on(conn, id, &today)?;
        let new_target = input.daily_target.map(validate_target).transpose()?;
        let target_changed = new_target
            .map(|t| current.as_ref().map(|c| c.target) != Some(t))
            .unwrap_or(false);
        let rec_changed = input
            .recurrence
            .as_ref()
            .map(|r| current.as_ref().map(|c| &c.recurrence) != Some(r))
            .unwrap_or(false);
        if target_changed || rec_changed {
            let target =
                new_target.unwrap_or_else(|| current.as_ref().map(|c| c.target).unwrap_or(1));
            let rec = input.recurrence.clone().unwrap_or_else(|| {
                current
                    .as_ref()
                    .map(|c| c.recurrence.clone())
                    .unwrap_or_default()
            });
            let start = crate::logical_day::add_days(&today, 1)?;
            period_repo::close_open_target(conn, id, &start)?;
            period_repo::insert_target(conn, id, target, &rec, &start, None)?;
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
