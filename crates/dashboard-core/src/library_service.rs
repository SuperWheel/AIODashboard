//! Date Library 用例：重要日 CRUD / 归档三选一 / 任务归属。

use dashboard_domain::{Actor, DateLibrary, LibraryKind, LibraryStatus, Task};
use dashboard_storage::{library_repo, period_repo, task_repo};
use rusqlite::Connection;
use serde::Serialize;

use crate::context_service::local_today;
use crate::logical_day as ld;
use crate::{log_activity, snapshot, CoreError, CoreResult};

#[derive(Debug, Clone)]
pub struct CreateLibraryInput {
    pub title: String,
    pub note: String,
    pub icon: String,
    pub color_hex: String,
    pub kind: LibraryKind,
    /// 锚点日（YYYY-MM-DD）
    pub anchor_day: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateLibraryInput {
    pub title: Option<String>,
    pub note: Option<String>,
    pub icon: Option<String>,
    pub color_hex: Option<String>,
    pub anchor_day: Option<String>,
}

fn validate_title(title: &str) -> CoreResult<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(CoreError::Validation("重要日标题不能为空".into()));
    }
    if t.chars().count() > 200 {
        return Err(CoreError::Validation(
            "重要日标题过长（最多 200 字符）".into(),
        ));
    }
    Ok(t.to_string())
}

/// 校验锚点日与类型的匹配：纪念日锚点 ≤ 今天；倒计时日锚点 ≥ 今天。
fn validate_anchor(kind: LibraryKind, anchor_day: &str, today: &str) -> CoreResult<()> {
    ld::parse_day(anchor_day)?;
    match kind {
        LibraryKind::Anniversary if anchor_day > today => {
            Err(CoreError::Validation("纪念日的锚点不能在未来".into()))
        }
        LibraryKind::Countdown if anchor_day < today => {
            Err(CoreError::Validation("倒计时日的锚点不能在过去".into()))
        }
        _ => Ok(()),
    }
}

pub fn create_library(
    conn: &Connection,
    input: &CreateLibraryInput,
    actor: Actor,
) -> CoreResult<DateLibrary> {
    let today = local_today();
    let title = validate_title(&input.title)?;
    validate_anchor(input.kind, &input.anchor_day, &today)?;
    let lib = library_repo::create(
        conn,
        &library_repo::NewLibrary {
            title,
            note: input.note.clone(),
            icon: input.icon.clone(),
            color_hex: input.color_hex.clone(),
            kind: input.kind,
            anchor_day: input.anchor_day.clone(),
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "library.create",
        "date_library",
        Some(&lib.id),
        &serde_json::json!({ "title": lib.title, "kind": lib.kind.as_str() }),
    );
    snapshot::refresh(conn);
    Ok(lib)
}

pub fn get_library(conn: &Connection, id: &str) -> CoreResult<DateLibrary> {
    library_repo::get(conn, id)?.ok_or_else(|| CoreError::NotFound(format!("date_library {id}")))
}

pub fn list_libraries(conn: &Connection, include_archived: bool) -> CoreResult<Vec<DateLibrary>> {
    Ok(library_repo::list(conn, include_archived)?)
}

pub fn update_library(
    conn: &Connection,
    id: &str,
    input: &UpdateLibraryInput,
    actor: Actor,
) -> CoreResult<DateLibrary> {
    let existing = get_library(conn, id)?;
    let today = local_today();
    let mut patch = library_repo::LibraryPatch::default();
    if let Some(title) = &input.title {
        patch.title = Some(validate_title(title)?);
    }
    if let Some(note) = &input.note {
        patch.note = Some(note.clone());
    }
    if let Some(icon) = &input.icon {
        patch.icon = Some(icon.clone());
    }
    if let Some(color) = &input.color_hex {
        patch.color_hex = Some(color.clone());
    }
    if let Some(anchor) = &input.anchor_day {
        validate_anchor(existing.kind, anchor, &today)?;
        patch.anchor_day = Some(anchor.clone());
    }
    library_repo::update(conn, id, &patch)?;
    let updated = get_library(conn, id)?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "library.update",
        "date_library",
        Some(id),
        &serde_json::json!({ "title": updated.title }),
    );
    snapshot::refresh(conn);
    Ok(updated)
}

/// 归档重要日时直属任务的处置方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveTaskMode {
    /// 保留归属（任务随重要日一起不出现在 Today？不——保留归属区间但重要日归档）
    Keep,
    /// 转独立任务（闭合归属区间）
    Detach,
    /// 移动到另一个活动重要日
    MoveTo,
}

/// 归档重要日（三选一处置直属任务，单事务）。
pub fn archive_library(
    conn: &Connection,
    id: &str,
    mode: ArchiveTaskMode,
    move_to: Option<&str>,
    actor: Actor,
) -> CoreResult<DateLibrary> {
    let lib = get_library(conn, id)?;
    if lib.status == LibraryStatus::Archived {
        return Ok(lib); // 幂等
    }
    let today = local_today();
    let task_ids = period_repo::library_current_task_ids(conn, id)?;

    if mode == ArchiveTaskMode::MoveTo {
        let target =
            move_to.ok_or_else(|| CoreError::Validation("move_to 模式需要目标重要日 id".into()))?;
        let target_lib = get_library(conn, target)?;
        if target_lib.status != LibraryStatus::Active {
            return Err(CoreError::Validation("目标重要日已归档".into()));
        }
        if target_lib.id == id {
            return Err(CoreError::Validation("不能移动到自身".into()));
        }
    }

    // 单事务：处置任务 + 归档重要日，失败整体回滚
    conn.execute("SAVEPOINT archive_library", [])?;
    let result = (|| -> CoreResult<()> {
        for tid in &task_ids {
            match mode {
                ArchiveTaskMode::Keep => {}
                ArchiveTaskMode::Detach => {
                    period_repo::close_open_membership(conn, tid, &today)?;
                }
                ArchiveTaskMode::MoveTo => {
                    period_repo::close_open_membership(conn, tid, &today)?;
                    let target = move_to.unwrap_or_default();
                    period_repo::insert_membership(conn, tid, target, &today)?;
                }
            }
        }
        library_repo::update(
            conn,
            id,
            &library_repo::LibraryPatch {
                status: Some(LibraryStatus::Archived),
                ..Default::default()
            },
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            conn.execute("RELEASE archive_library", [])?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK TO archive_library", []);
            let _ = conn.execute("RELEASE archive_library", []);
            return Err(e);
        }
    }

    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "library.archive",
        "date_library",
        Some(id),
        &serde_json::json!({
            "task_mode": match mode {
                ArchiveTaskMode::Keep => "keep",
                ArchiveTaskMode::Detach => "detach",
                ArchiveTaskMode::MoveTo => "move_to",
            },
            "task_count": task_ids.len(),
        }),
    );
    snapshot::refresh(conn);
    get_library(conn, id)
}

pub fn restore_library(conn: &Connection, id: &str, actor: Actor) -> CoreResult<DateLibrary> {
    let lib = get_library(conn, id)?;
    if lib.status == LibraryStatus::Active {
        return Ok(lib); // 幂等
    }
    library_repo::update(
        conn,
        id,
        &library_repo::LibraryPatch {
            status: Some(LibraryStatus::Active),
            ..Default::default()
        },
    )?;
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "library.restore",
        "date_library",
        Some(id),
        &serde_json::json!({ "title": lib.title }),
    );
    snapshot::refresh(conn);
    get_library(conn, id)
}

/// 任务移入重要日（今日起生效；此前归属闭合，历史留原库）。library_id=None 表示移出为独立任务。
pub fn move_task(
    conn: &Connection,
    task_id: &str,
    library_id: Option<&str>,
    actor: Actor,
) -> CoreResult<Task> {
    let task = crate::task_service::get_task(conn, task_id)?;
    let today = local_today();
    let current = period_repo::membership_on(conn, task_id, &today)?;
    let current_lib = current.map(|m| m.library_id);

    if current_lib.as_deref() == library_id {
        return Ok(task); // 幂等：归属未变
    }
    if let Some(lid) = library_id {
        let lib = get_library(conn, lid)?;
        if lib.status != LibraryStatus::Active {
            return Err(CoreError::Validation("目标重要日已归档".into()));
        }
    }

    period_repo::close_open_membership(conn, task_id, &today)?;
    if let Some(lid) = library_id {
        period_repo::insert_membership(conn, task_id, lid, &today)?;
    }
    log_activity(
        conn,
        chrono::Utc::now(),
        actor,
        "task.move_library",
        "task",
        Some(task_id),
        &serde_json::json!({
            "from": current_lib,
            "to": library_id,
            "effective_day": today,
        }),
    );
    snapshot::refresh(conn);
    crate::task_service::get_task(conn, task_id)
}

/// 重要日当前直属任务（启用中的）。
pub fn library_tasks(conn: &Connection, library_id: &str) -> CoreResult<Vec<Task>> {
    get_library(conn, library_id)?;
    let ids = period_repo::library_current_task_ids(conn, library_id)?;
    let mut tasks = Vec::new();
    for id in ids {
        if let Some(t) = task_repo::get(conn, &id)? {
            tasks.push(t);
        }
    }
    Ok(tasks)
}

/// 重要日天数显示数据：纪念日=已过天数；倒计时=剩余/逾期。
#[derive(Debug, Serialize)]
pub struct LibraryDayInfo {
    /// 纪念日：第 N 天（锚点当天为第 1 天）；倒计时：还剩 N 天（锚点当天为 0）
    pub day_count: i64,
    /// anniversary: "day_n"；countdown: "remaining" | "today" | "overdue"
    pub display_kind: String,
}

pub fn day_info(lib: &DateLibrary, today: &str) -> CoreResult<LibraryDayInfo> {
    let diff = ld::days_between(&lib.anchor_day, today)?; // today - anchor
    Ok(match lib.kind {
        LibraryKind::Anniversary => LibraryDayInfo {
            day_count: diff + 1,
            display_kind: "day_n".into(),
        },
        LibraryKind::Countdown => {
            let remaining = -diff; // anchor - today
            LibraryDayInfo {
                day_count: remaining.abs(),
                display_kind: if remaining > 0 {
                    "remaining".into()
                } else if remaining == 0 {
                    "today".into()
                } else {
                    "overdue".into()
                },
            }
        }
    })
}

/// 重要日列表视图（带天数信息）。
#[derive(Debug, Serialize)]
pub struct LibraryListItem {
    #[serde(flatten)]
    pub library: DateLibrary,
    pub day_info: LibraryDayInfo,
    /// 当前直属任务数（启用中）
    pub task_count: i64,
}

pub fn list_library_items(
    conn: &Connection,
    include_archived: bool,
) -> CoreResult<Vec<LibraryListItem>> {
    let today = local_today();
    let mut out = Vec::new();
    for lib in library_repo::list(conn, include_archived)? {
        let task_count = period_repo::library_current_task_ids(conn, &lib.id)?
            .iter()
            .filter(|tid| {
                task_repo::get(conn, tid)
                    .ok()
                    .flatten()
                    .map(|t| t.status == dashboard_domain::TaskStatus::Active)
                    .unwrap_or(false)
            })
            .count() as i64;
        let info = day_info(&lib, &today)?;
        out.push(LibraryListItem {
            library: lib,
            day_info: info,
            task_count,
        });
    }
    Ok(out)
}
