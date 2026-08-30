//! 打卡体系 core 链路测试（内存库）。
//!
//! 覆盖 tasks.md TDD 清单中需要数据库的部分：账本幂等/补偿、目标区间、
//! 活动区间、归属与主库聚合、迁移。

use dashboard_core as core;
use dashboard_domain::{Actor, CardStyle, LibraryKind, TaskStatus};
use dashboard_storage as ds;
use rusqlite::Connection;

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    ds::migrate(&c).unwrap();
    c
}

fn today() -> String {
    core::context_service::local_today()
}

fn make_task(conn: &Connection, target: i64) -> dashboard_domain::Task {
    core::task_service::create_task(
        conn,
        &core::task_service::CreateTaskInput {
            title: "喝水".into(),
            unit: "杯".into(),
            daily_target: target,
            ..Default::default()
        },
        Actor::User,
    )
    .unwrap()
}

fn make_lib(conn: &Connection, kind: LibraryKind, anchor: &str) -> dashboard_domain::DateLibrary {
    core::library_service::create_library(
        conn,
        &core::library_service::CreateLibraryInput {
            title: "主库".into(),
            note: String::new(),
            icon: String::new(),
            color_hex: "#4A90E2".into(),
            kind,
            anchor_day: anchor.into(),
        },
        Actor::User,
    )
    .unwrap()
}

// ---------- 账本 ----------

#[test]
fn checkin_idempotent_replay() {
    let c = conn();
    let t = make_task(&c, 2);
    let v1 = core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    assert_eq!(v1.count, 1);
    // 同 operation_id 重放：计数不变
    let v2 = core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    assert_eq!(v2.count, 1);
    // 同一 operation_id 被不同类型使用 → 冲突
    assert!(core::checkin_service::decrement(&c, &t.id, "op-1", Actor::User).is_err());
}

#[test]
fn decrement_compensates_latest_positive_once() {
    let c = conn();
    let t = make_task(&c, 3);
    core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    core::checkin_service::record(&c, &t.id, "op-2", Actor::User).unwrap();
    let v = core::checkin_service::decrement(&c, &t.id, "op-3", Actor::User).unwrap();
    assert_eq!(v.count, 1);
    // 再减一次补偿另一条
    let v = core::checkin_service::decrement(&c, &t.id, "op-4", Actor::User).unwrap();
    assert_eq!(v.count, 0);
    // 减到 0 以下拒绝
    assert!(core::checkin_service::decrement(&c, &t.id, "op-5", Actor::User).is_err());
}

#[test]
fn undo_reverses_latest_operation() {
    let c = conn();
    let t = make_task(&c, 2);
    core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    let v = core::checkin_service::undo(&c, &t.id, "op-2", Actor::User).unwrap();
    assert_eq!(v.count, 0);
    // 已撤销后无可撤销操作
    assert!(core::checkin_service::undo(&c, &t.id, "op-3", Actor::User).is_err());
}

#[test]
fn checkin_rejected_when_not_applicable() {
    let c = conn();
    let t = make_task(&c, 1);
    core::task_service::archive_task(&c, &t.id, Actor::User).unwrap();
    assert!(core::checkin_service::record(&c, &t.id, "op-x", Actor::User).is_err());
}

// ---------- 归档/恢复与活动区间 ----------

#[test]
fn archive_then_restore_gap_not_applicable() {
    let c = conn();
    let t = make_task(&c, 1);
    let today = today();
    // 打卡到完成
    core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    core::task_service::archive_task(&c, &t.id, Actor::User).unwrap();
    let t2 = core::task_service::get_task(&c, &t.id).unwrap();
    assert_eq!(t2.status, TaskStatus::Archived);
    // 归档日不适用（end_day 排他）
    assert!(!ds::period_repo::is_active_on(&c, &t.id, &today).unwrap());
    core::task_service::restore_task(&c, &t.id, Actor::User).unwrap();
    assert!(ds::period_repo::is_active_on(&c, &t.id, &today).unwrap());
}

// ---------- 目标区间 ----------

#[test]
fn target_change_keeps_history() {
    let c = conn();
    let t = make_task(&c, 1);
    let today = today();
    // 今天打卡 1 次（按旧目标 1 应完成）
    core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    // 改目标为 3（明天起生效）
    core::task_service::update_task(
        &c,
        &t.id,
        &core::task_service::UpdateTaskInput {
            daily_target: Some(3),
            ..Default::default()
        },
        Actor::User,
    )
    .unwrap();
    // 今天仍读旧目标 1 → 已完成
    let v = core::checkin_service::task_day_view(&c, &t.id).unwrap();
    assert_eq!(v.target, Some(1));
    assert_eq!(v.state, "completed");
    // 明天起目标是 3
    let tomorrow = core::logical_day::add_days(&today, 1).unwrap();
    let p = ds::period_repo::target_on(&c, &t.id, &tomorrow)
        .unwrap()
        .unwrap();
    assert_eq!(p.target, 3);
    // 非法目标拒绝
    assert!(core::task_service::update_task(
        &c,
        &t.id,
        &core::task_service::UpdateTaskInput {
            daily_target: Some(0),
            ..Default::default()
        },
        Actor::User,
    )
    .is_err());
}

// ---------- 周期总览 ----------

#[test]
fn week_overview_summary() {
    let c = conn();
    let t = make_task(&c, 2);
    core::checkin_service::record(&c, &t.id, "op-1", Actor::User).unwrap();
    let ov = core::overview_service::task_period_overview(&c, &t.id, "week", None).unwrap();
    assert_eq!(ov.days.len(), 7);
    let today_day = ov.days.iter().find(|d| d.is_today).unwrap();
    assert_eq!(today_day.display_state.as_str(), "partial_high");
    assert_eq!(today_day.actual_count, 1);
    assert_eq!(ov.summary.applicable_day_count, 1); // 只有今天已发生
    assert_eq!(ov.summary.complete_day_count, 0);
}

#[test]
fn year_overview_days_count() {
    let c = conn();
    let t = make_task(&c, 1);
    let ov = core::overview_service::task_period_overview(&c, &t.id, "year", None).unwrap();
    assert!(ov.days.len() == 365 || ov.days.len() == 366);
}

// ---------- 日期主库 ----------

#[test]
fn library_day_calculation() {
    let c = conn();
    let today = today();
    // 纪念日：锚点=今天 → 第 1 天
    let lib = make_lib(&c, LibraryKind::Anniversary, &today);
    let info = core::library_service::day_info(&lib, &today).unwrap();
    assert_eq!(info.day_count, 1);
    assert_eq!(info.display_kind, "day_n");
    // 纪念日锚点不能在未来
    let tomorrow = core::logical_day::add_days(&today, 1).unwrap();
    assert!(core::library_service::create_library(
        &c,
        &core::library_service::CreateLibraryInput {
            title: "x".into(),
            note: String::new(),
            icon: String::new(),
            color_hex: "#4A90E2".into(),
            kind: LibraryKind::Anniversary,
            anchor_day: tomorrow.clone(),
        },
        Actor::User,
    )
    .is_err());
    // 倒计时：明天 → 还剩 1 天
    let lib2 = make_lib(&c, LibraryKind::Countdown, &tomorrow);
    let info2 = core::library_service::day_info(&lib2, &today).unwrap();
    assert_eq!(info2.day_count, 1);
    assert_eq!(info2.display_kind, "remaining");
    // 倒计时：今天 → 就是今天
    let lib3 = make_lib(&c, LibraryKind::Countdown, &today);
    assert_eq!(
        core::library_service::day_info(&lib3, &today)
            .unwrap()
            .display_kind,
        "today"
    );
}

#[test]
fn membership_move_effective_today() {
    let c = conn();
    let today = today();
    let lib_a = make_lib(&c, LibraryKind::Anniversary, &today);
    let lib_b = make_lib(&c, LibraryKind::Anniversary, &today);
    let t = make_task(&c, 1);
    core::library_service::move_task(&c, &t.id, Some(&lib_a.id), Actor::User).unwrap();
    core::library_service::move_task(&c, &t.id, Some(&lib_b.id), Actor::User).unwrap();
    // 今天在 B
    let m = ds::period_repo::membership_on(&c, &t.id, &today)
        .unwrap()
        .unwrap();
    assert_eq!(m.library_id, lib_b.id);
    // A 的当前直属为空
    assert!(ds::period_repo::library_current_task_ids(&c, &lib_a.id)
        .unwrap()
        .is_empty());
    // 幂等：再移到 B 不报错
    core::library_service::move_task(&c, &t.id, Some(&lib_b.id), Actor::User).unwrap();
}

#[test]
fn archive_library_modes() {
    let c = conn();
    let today = today();
    let lib = make_lib(&c, LibraryKind::Anniversary, &today);
    let t = make_task(&c, 1);
    core::library_service::move_task(&c, &t.id, Some(&lib.id), Actor::User).unwrap();
    // detach 模式
    let lib2 = core::library_service::archive_library(
        &c,
        &lib.id,
        core::library_service::ArchiveTaskMode::Detach,
        None,
        Actor::User,
    )
    .unwrap();
    assert_eq!(lib2.status, dashboard_domain::LibraryStatus::Archived);
    assert!(ds::period_repo::membership_on(&c, &t.id, &today)
        .unwrap()
        .is_none());
    // 归档幂等
    core::library_service::archive_library(
        &c,
        &lib.id,
        core::library_service::ArchiveTaskMode::Detach,
        None,
        Actor::User,
    )
    .unwrap();
    // 恢复
    let lib3 = core::library_service::restore_library(&c, &lib.id, Actor::User).unwrap();
    assert_eq!(lib3.status, dashboard_domain::LibraryStatus::Active);
}

#[test]
fn archive_library_move_to_rolls_back_on_bad_target() {
    let c = conn();
    let today = today();
    let lib = make_lib(&c, LibraryKind::Anniversary, &today);
    let t = make_task(&c, 1);
    core::library_service::move_task(&c, &t.id, Some(&lib.id), Actor::User).unwrap();
    // move_to 目标不存在 → 报错且归属不受影响
    assert!(core::library_service::archive_library(
        &c,
        &lib.id,
        core::library_service::ArchiveTaskMode::MoveTo,
        Some("dlb_nonexistent"),
        Actor::User,
    )
    .is_err());
    assert_eq!(
        ds::period_repo::membership_on(&c, &t.id, &today)
            .unwrap()
            .unwrap()
            .library_id,
        lib.id
    );
}

// ---------- 主库综合热力图 ----------

#[test]
fn library_heatmap_aggregate() {
    let c = conn();
    let today = today();
    let lib = make_lib(&c, LibraryKind::Anniversary, &today);
    // 任务 A：目标 2 打 2 → 100%；任务 B：目标 2 打 1 → 50%；主库 = 75%
    let a = make_task(&c, 2);
    let b = make_task(&c, 2);
    core::library_service::move_task(&c, &a.id, Some(&lib.id), Actor::User).unwrap();
    core::library_service::move_task(&c, &b.id, Some(&lib.id), Actor::User).unwrap();
    core::checkin_service::record(&c, &a.id, "op-a1", Actor::User).unwrap();
    core::checkin_service::record(&c, &a.id, "op-a2", Actor::User).unwrap();
    core::checkin_service::record(&c, &b.id, "op-b1", Actor::User).unwrap();

    let hm = core::overview_service::library_year_heatmap(&c, &lib.id, None).unwrap();
    let today_cell = hm.days.iter().find(|d| d.is_today).unwrap();
    assert_eq!(today_cell.display_state, "rate");
    assert!((today_cell.rate.unwrap() - 0.75).abs() < 1e-9);
    assert_eq!(today_cell.active_task_count, 2);
}

// ---------- 今日上下文 ----------

#[test]
fn today_context_checkin_stats() {
    let c = conn();
    let _a = make_task(&c, 2);
    let b = make_task(&c, 1);
    core::checkin_service::record(&c, &b.id, "op-1", Actor::User).unwrap();
    let ctx = core::context_service::context_today(&c).unwrap();
    assert_eq!(ctx.stats.task_total, 2);
    assert_eq!(ctx.stats.completed_today, 1);
    // 完成率 = (0 + 1) / (2 + 1) = 1/3
    assert!((ctx.stats.completion_rate - 1.0 / 3.0).abs() < 1e-9);
}

// ---------- 迁移 ----------

#[test]
fn migration_v3_maps_old_status_and_backfills_periods() {
    let c = Connection::open_in_memory().unwrap();
    // 手工建 V1 结构与旧数据
    c.execute_batch(
        "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'active', created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'todo', due_at TEXT, project_id TEXT, completed_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE notes (id TEXT PRIMARY KEY, title TEXT NOT NULL DEFAULT '', body TEXT NOT NULL DEFAULT '', project_id TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE inbox_items (id TEXT PRIMARY KEY, content TEXT NOT NULL, source TEXT NOT NULL DEFAULT 'user', status TEXT NOT NULL DEFAULT 'open', created_at TEXT NOT NULL);
         CREATE TABLE activity_log (id TEXT PRIMARY KEY, ts TEXT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL, object_type TEXT NOT NULL, object_id TEXT, detail TEXT NOT NULL DEFAULT '{}');
         CREATE TABLE widget_snapshots (kind TEXT PRIMARY KEY, generated_at TEXT NOT NULL, payload TEXT NOT NULL);
         INSERT INTO tasks VALUES ('tsk_a','旧待办','todo',NULL,NULL,NULL,'2026-08-01T00:00:00Z','2026-08-01T00:00:00Z');
         INSERT INTO tasks VALUES ('tsk_b','旧完成','done',NULL,NULL,'2026-08-02T00:00:00Z','2026-08-01T00:00:00Z','2026-08-02T00:00:00Z');
         PRAGMA user_version = 2;",
    )
    .unwrap();
    ds::migrate(&c).unwrap();
    let a = ds::task_repo::get(&c, "tsk_a").unwrap().unwrap();
    assert_eq!(a.status, TaskStatus::Active);
    let b = ds::task_repo::get(&c, "tsk_b").unwrap().unwrap();
    assert_eq!(b.status, TaskStatus::Archived);
    // 补默认目标与活动区间
    let targets = ds::period_repo::list_targets(&c, "tsk_a").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].target, 1);
    let acts = ds::period_repo::list_activity(&c, "tsk_a").unwrap();
    assert_eq!(acts.len(), 1);
    assert!(acts[0].end_day.is_none());
    // 归档任务的活动区间已闭合
    let acts_b = ds::period_repo::list_activity(&c, "tsk_b").unwrap();
    assert!(acts_b[0].end_day.is_some());
}

// ---------- 兼容命令 ----------

#[test]
fn complete_today_compat_fills_target() {
    let c = conn();
    let t = make_task(&c, 3);
    core::checkin_service::record(&c, &t.id, "op-1", Actor::Cli).unwrap();
    let v = core::checkin_service::complete_today(&c, &t.id, Actor::Cli).unwrap();
    assert_eq!(v.count, 3);
    assert_eq!(v.state, "completed");
    // reopen 兼容：今日清零
    let v = core::checkin_service::reopen_today(&c, &t.id, Actor::Cli).unwrap();
    assert_eq!(v.count, 0);
}

#[test]
fn card_style_persisted() {
    let c = conn();
    let t = make_task(&c, 1);
    core::task_service::update_task(
        &c,
        &t.id,
        &core::task_service::UpdateTaskInput {
            card_style: Some(CardStyle::Year),
            ..Default::default()
        },
        Actor::User,
    )
    .unwrap();
    assert_eq!(
        core::task_service::get_task(&c, &t.id).unwrap().card_style,
        CardStyle::Year
    );
}
