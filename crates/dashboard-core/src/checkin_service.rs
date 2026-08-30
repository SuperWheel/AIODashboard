//! Check-in 用例：打卡 / 减少 / 撤销（append-only 账本 + 幂等）。

use dashboard_domain::{Actor, CompletionKind, Task};
use dashboard_storage::{completion_repo, period_repo};
use rusqlite::Connection;
use serde::Serialize;

use crate::context_service::local_today;
use crate::{log_activity, snapshot, CoreError, CoreResult};

/// 任务当日视图（打卡操作的返回，也是 Today 页的数据单元）。
#[derive(Debug, Serialize)]
pub struct TaskDayView {
    pub task: Task,
    /// 当日生效目标（None = 不适用）
    pub target: Option<i64>,
    /// 当日净次数
    pub count: i64,
    pub state: String,
    /// 当前生效的主库 id（无 = 独立任务）
    pub library_id: Option<String>,
    /// 是否存在可撤销的最近一次操作
    pub can_undo: bool,
}

fn require_applicable_today(conn: &Connection, task: &Task, today: &str) -> CoreResult<i64> {
    if task.status != dashboard_domain::TaskStatus::Active {
        return Err(CoreError::Validation("任务已归档，不能打卡".into()));
    }
    if !period_repo::is_active_on(conn, &task.id, today)? {
        return Err(CoreError::Validation("任务今天不适用，不能打卡".into()));
    }
    let target = period_repo::target_on(conn, &task.id, today)?.map(|p| p.target);
    match target {
        Some(t) if t > 0 => Ok(t),
        _ => Err(CoreError::Validation(
            "任务今天没有有效目标，不能打卡".into(),
        )),
    }
}

fn build_day_view(conn: &Connection, task: Task, today: &str) -> CoreResult<TaskDayView> {
    let target = period_repo::target_on(conn, &task.id, today)?.map(|p| p.target);
    let applicable = task.status == dashboard_domain::TaskStatus::Active
        && period_repo::is_active_on(conn, &task.id, today)?;
    let count = completion_repo::day_count(conn, &task.id, today)?;
    let state = crate::day_state::eval_day_state(applicable, target, count, today, today);
    let library_id = period_repo::membership_on(conn, &task.id, today)?.map(|m| m.library_id);
    let can_undo =
        completion_repo::latest_uncompensated_positive_any_day(conn, &task.id)?.is_some();
    Ok(TaskDayView {
        task,
        target,
        count,
        state: state.as_str().to_string(),
        library_id,
        can_undo,
    })
}

pub fn task_day_view(conn: &Connection, task_id: &str) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    build_day_view(conn, task, &today)
}

/// 打卡 +1。幂等：同 operation_id 重放返回当前视图，不重复计数。
pub fn record(
    conn: &Connection,
    task_id: &str,
    operation_id: &str,
    actor: Actor,
) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();

    if let Some(existing) = completion_repo::get_by_operation(conn, operation_id)? {
        // 幂等重放：校验同一意图后返回现状
        if existing.task_id != task_id || existing.kind != CompletionKind::Add {
            return Err(CoreError::Conflict(format!(
                "operation_id 已被其他操作使用: {operation_id}"
            )));
        }
        return build_day_view(conn, task, &today);
    }

    require_applicable_today(conn, &task, &today)?;
    completion_repo::append(
        conn,
        &completion_repo::NewCompletion {
            task_id: task_id.to_string(),
            operation_id: operation_id.to_string(),
            value: 1,
            kind: CompletionKind::Add,
            compensates_record_id: None,
            logical_day: today.clone(),
            source: actor.as_str(),
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.checkin",
        "task",
        Some(task_id),
        &serde_json::json!({ "logical_day": today }),
    );
    snapshot::refresh(conn);
    build_day_view(conn, task, &today)
}

/// 减少一次（补偿当日最近一条未被补偿的正向记录）。幂等同上。
pub fn decrement(
    conn: &Connection,
    task_id: &str,
    operation_id: &str,
    actor: Actor,
) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();

    if let Some(existing) = completion_repo::get_by_operation(conn, operation_id)? {
        if existing.task_id != task_id || existing.kind != CompletionKind::Decrement {
            return Err(CoreError::Conflict(format!(
                "operation_id 已被其他操作使用: {operation_id}"
            )));
        }
        return build_day_view(conn, task, &today);
    }

    require_applicable_today(conn, &task, &today)?;
    let target_record = completion_repo::latest_uncompensated_positive(conn, task_id, &today)?
        .ok_or_else(|| CoreError::Validation("今天没有可减少的打卡记录".into()))?;
    completion_repo::append(
        conn,
        &completion_repo::NewCompletion {
            task_id: task_id.to_string(),
            operation_id: operation_id.to_string(),
            value: -target_record.value,
            kind: CompletionKind::Decrement,
            compensates_record_id: Some(target_record.id),
            logical_day: today.clone(),
            source: actor.as_str(),
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.decrement",
        "task",
        Some(task_id),
        &serde_json::json!({ "logical_day": today }),
    );
    snapshot::refresh(conn);
    build_day_view(conn, task, &today)
}

/// 撤销最近一条未被补偿的正向打卡（写入等值负补偿，kind=undo）。
/// 只针对 add 记录：撤销的撤销（redo）不提供，避免语义混乱。幂等同上。
pub fn undo(
    conn: &Connection,
    task_id: &str,
    operation_id: &str,
    actor: Actor,
) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();

    if let Some(existing) = completion_repo::get_by_operation(conn, operation_id)? {
        if existing.task_id != task_id || existing.kind != CompletionKind::Undo {
            return Err(CoreError::Conflict(format!(
                "operation_id 已被其他操作使用: {operation_id}"
            )));
        }
        return build_day_view(conn, task, &today);
    }

    let target_record = completion_repo::latest_uncompensated_positive_any_day(conn, task_id)?
        .ok_or_else(|| CoreError::Validation("没有可撤销的打卡记录".into()))?;
    completion_repo::append(
        conn,
        &completion_repo::NewCompletion {
            task_id: task_id.to_string(),
            operation_id: operation_id.to_string(),
            value: -target_record.value,
            kind: CompletionKind::Undo,
            compensates_record_id: Some(target_record.id.clone()),
            logical_day: today.clone(),
            source: actor.as_str(),
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.undo",
        "task",
        Some(task_id),
        &serde_json::json!({ "reversed": target_record.id }),
    );
    snapshot::refresh(conn);
    build_day_view(conn, task, &today)
}

/// 兼容旧协议：`task complete` → 补满今日目标；`task reopen` → 今日清零。
pub fn complete_today(conn: &Connection, task_id: &str, actor: Actor) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    let target = require_applicable_today(conn, &task, &today)?;
    let count = completion_repo::day_count(conn, task_id, &today)?;
    let missing = target - count;
    if missing > 0 {
        completion_repo::append(
            conn,
            &completion_repo::NewCompletion {
                task_id: task_id.to_string(),
                operation_id: dashboard_domain::new_id("op"),
                value: missing,
                kind: CompletionKind::Add,
                compensates_record_id: None,
                logical_day: today.clone(),
                source: actor.as_str(),
            },
        )?;
        log_activity(
            conn,
            chrono::Utc::now(),
            actor,
            "task.checkin",
            "task",
            Some(task_id),
            &serde_json::json!({ "logical_day": today, "via": "complete_compat", "value": missing }),
        );
        snapshot::refresh(conn);
    }
    build_day_view(conn, task, &today)
}

pub fn reopen_today(conn: &Connection, task_id: &str, actor: Actor) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    require_applicable_today(conn, &task, &today)?;
    let count = completion_repo::day_count(conn, task_id, &today)?;
    if count > 0 {
        completion_repo::append(
            conn,
            &completion_repo::NewCompletion {
                task_id: task_id.to_string(),
                operation_id: dashboard_domain::new_id("op"),
                value: -count,
                kind: CompletionKind::Undo,
                compensates_record_id: None,
                logical_day: today.clone(),
                source: actor.as_str(),
            },
        )?;
        log_activity(
            conn,
            chrono::Utc::now(),
            actor,
            "task.undo",
            "task",
            Some(task_id),
            &serde_json::json!({ "logical_day": today, "via": "reopen_compat", "value": -count }),
        );
        snapshot::refresh(conn);
    }
    build_day_view(conn, task, &today)
}
