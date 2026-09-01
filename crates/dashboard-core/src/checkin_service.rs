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
    /// 当前生效的重要日 id（无 = 独立任务）
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
    let period = period_repo::target_on(conn, &task.id, today)?
        .ok_or_else(|| CoreError::Validation("任务今天没有有效目标，不能打卡".into()))?;
    if !period.recurrence.matches(&period.start_day, today) {
        return Err(CoreError::Validation("任务今天不适用，不能打卡".into()));
    }
    if period.target <= 0 {
        return Err(CoreError::Validation(
            "任务今天没有有效目标，不能打卡".into(),
        ));
    }
    Ok(period.target)
}

/// 按任意逻辑日构建任务视图：day=评价日，today=真实今天（missed/pending 分界）。
fn build_day_view_on(
    conn: &Connection,
    task: Task,
    day: &str,
    today: &str,
) -> CoreResult<TaskDayView> {
    let period = period_repo::target_on(conn, &task.id, day)?;
    let target = period.as_ref().map(|p| p.target);
    let applicable = task.status == dashboard_domain::TaskStatus::Active
        && period_repo::is_active_on(conn, &task.id, day)?
        && period
            .as_ref()
            .map(|p| p.recurrence.matches(&p.start_day, day))
            .unwrap_or(false);
    let count = completion_repo::day_count(conn, &task.id, day)?;
    let state = crate::day_state::eval_day_state(applicable, target, count, day, today);
    let library_id = period_repo::membership_on(conn, &task.id, day)?.map(|m| m.library_id);
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

fn build_day_view(conn: &Connection, task: Task, today: &str) -> CoreResult<TaskDayView> {
    build_day_view_on(conn, task, today, today)
}

/// 一次性任务打卡达标 → 自动归档（004 决策：完成即终态，历史保留）。
/// 归档后返回的视图仍按归档前状态展示（调用方先 build 再归档）。
fn auto_archive_once_on_completion(
    conn: &Connection,
    task: &Task,
    view: &TaskDayView,
    actor: Actor,
) -> CoreResult<()> {
    if task.recurrence == dashboard_domain::Recurrence::Once
        && view.state == crate::day_state::TaskDayState::Completed.as_str()
    {
        crate::task_service::archive_task(conn, &task.id, actor)?;
    }
    Ok(())
}

pub fn task_day_view(conn: &Connection, task_id: &str) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    build_day_view(conn, task, &today)
}

/// 指定逻辑日的任务视图（任务页日期翻页用）。过去日未达标=missed，未来日=pending。
pub fn task_day_view_on(conn: &Connection, task_id: &str, day: &str) -> CoreResult<TaskDayView> {
    crate::logical_day::parse_day(day)?;
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    build_day_view_on(conn, task, day, &today)
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
        actor.clone(),
        "task.checkin",
        "task",
        Some(task_id),
        &serde_json::json!({ "logical_day": today }),
    );
    snapshot::refresh(conn);
    let v = build_day_view(conn, task.clone(), &today)?;
    auto_archive_once_on_completion(conn, &task, &v, actor)?;
    Ok(v)
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
    // 补偿必须记在被撤记录的逻辑日：日聚合全部按 logical_day 分组，
    // 记到"今天"会既改不动历史日、又污染今天的账面。
    completion_repo::append(
        conn,
        &completion_repo::NewCompletion {
            task_id: task_id.to_string(),
            operation_id: operation_id.to_string(),
            value: -target_record.value,
            kind: CompletionKind::Undo,
            compensates_record_id: Some(target_record.id.clone()),
            logical_day: target_record.logical_day.clone(),
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
        &serde_json::json!({
            "reversed": target_record.id,
            "logical_day": target_record.logical_day,
        }),
    );
    snapshot::refresh(conn);
    build_day_view(conn, task, &today)
}

/// 兼容旧协议：`task complete` → 补满今日目标；`task reopen` → 今日清零。
pub fn complete_today(conn: &Connection, task_id: &str, actor: Actor) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    let target = require_applicable_today(conn, &task, &today)?;
    // 用未钳位的账面和算差额：day_count 的下限 0 会把历史负差额藏起来，
    // 导致补录量不足、补满后仍显示未完成。
    let sum = completion_repo::day_sum(conn, task_id, &today)?;
    let missing = target - sum;
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
            actor.clone(),
            "task.checkin",
            "task",
            Some(task_id),
            &serde_json::json!({ "logical_day": today, "via": "complete_compat", "value": missing }),
        );
        snapshot::refresh(conn);
    }
    let v = build_day_view(conn, task.clone(), &today)?;
    auto_archive_once_on_completion(conn, &task, &v, actor)?;
    Ok(v)
}

pub fn reopen_today(conn: &Connection, task_id: &str, actor: Actor) -> CoreResult<TaskDayView> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    require_applicable_today(conn, &task, &today)?;
    // 逐条补偿当日所有未补偿正向记录。若只追加一条无指向的批量负记录，
    // 原正向记录仍算"未补偿"，decrement/undo 会继续放行把账面挖成负数。
    let records = completion_repo::uncompensated_positives_on(conn, task_id, &today)?;
    for rec in &records {
        completion_repo::append(
            conn,
            &completion_repo::NewCompletion {
                task_id: task_id.to_string(),
                operation_id: dashboard_domain::new_id("op"),
                value: -rec.value,
                kind: CompletionKind::Undo,
                compensates_record_id: Some(rec.id.clone()),
                logical_day: rec.logical_day.clone(),
                source: actor.as_str(),
            },
        )?;
    }
    if !records.is_empty() {
        log_activity(
            conn,
            chrono::Utc::now(),
            actor,
            "task.undo",
            "task",
            Some(task_id),
            &serde_json::json!({ "logical_day": today, "via": "reopen_compat", "records": records.len() }),
        );
        snapshot::refresh(conn);
    }
    build_day_view(conn, task, &today)
}
