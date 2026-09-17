//! AIODashboard Desktop —— Tauri Interface 层。
//!
//! 所有 Command 只做参数转换，业务全部委托 dashboard-core。
//! 与 CLI 平级：GUI 创建的任务 = CLI 创建的任务。

pub mod plugin_cron;
mod plugin_import;

use dashboard_core as core;
use dashboard_domain::{Actor, Note, Project, Task};
use dashboard_storage as storage;
use rusqlite::Connection;
use tauri::{Manager, State};

type R<T> = Result<T, String>;

fn conn() -> R<Connection> {
    storage::open_default().map_err(|e| e.to_string())
}

fn actor() -> Actor {
    Actor::User
}

/// GUI 只能标记 User；插件身份由独立 token 入口在 Core 中派生。
fn actor_from(actor: Option<String>) -> R<Actor> {
    match actor.as_deref() {
        None => Ok(Actor::User),
        Some("user") => Ok(Actor::User),
        Some(a) => Err(format!(
            "非法 actor '{a}'：插件及其他来源只能通过受控上下文调用"
        )),
    }
}

// ---------------- Today / Tasks ----------------

#[tauri::command]
fn get_today() -> R<core::context_service::TodayContext> {
    let c = conn()?;
    core::context_service::context_today(&c).map_err(|e| e.to_string())
}

/// 任务墙：全部启用任务在指定逻辑日的视图（含当日不适用者）。
/// day=None = 今天；指定日期用于任务页日期翻页（过去日 missed / 未来日 pending）。
#[tauri::command]
fn task_wall_views(day: Option<String>) -> R<Vec<core::checkin_service::TaskDayView>> {
    let c = conn()?;
    match day.as_deref() {
        Some(d) => core::context_service::wall_task_views_on(&c, d).map_err(|e| e.to_string()),
        None => core::context_service::wall_task_views(&c).map_err(|e| e.to_string()),
    }
}

/// scope: "all" | "active" | "archived"
#[tauri::command]
fn list_tasks(scope: Option<String>) -> R<Vec<Task>> {
    use dashboard_storage::task_repo::TaskQuery;
    let c = conn()?;
    let mut q = TaskQuery {
        limit: 500,
        ..Default::default()
    };
    match scope.as_deref() {
        Some("active") => q.status = Some(dashboard_domain::TaskStatus::Active),
        Some("archived") => q.status = Some(dashboard_domain::TaskStatus::Archived),
        _ => {}
    }
    core::task_service::list_tasks(&c, &q).map_err(|e| e.to_string())
}

/// 归档任务列表（含归档日 MAX(end_day)，归档页分组用）。
#[tauri::command]
fn list_archived_tasks() -> R<Vec<core::task_service::ArchivedTask>> {
    let c = conn()?;
    core::task_service::list_archived_tasks(&c).map_err(|e| e.to_string())
}

// Tauri 只对命令顶层参数做 camelCase↔snake_case 转换，嵌套 struct 走纯 serde，
// 必须显式 rename_all，否则前端发的 camelCase 键会被静默丢弃。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskParams {
    title: String,
    target: Option<i64>,
    unit: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    card_style: Option<String>,
    recurrence: Option<String>,
    weekdays: Option<Vec<u8>>,
    project_id: Option<String>,
    library_id: Option<String>,
    priority: Option<i64>,
    actor: Option<String>,
}

/// 解析前端循环参数（kind + weekdays → domain::Recurrence）。
/// 前端 weekly 始终携带非空 weekdays；空集回退为锚点星期。
fn parse_recurrence(kind: &str, weekdays: Option<Vec<u8>>) -> R<dashboard_domain::Recurrence> {
    use dashboard_domain::Recurrence;
    match kind {
        "daily" => Ok(Recurrence::Daily),
        "once" => Ok(Recurrence::Once),
        "monthly" => Ok(Recurrence::Monthly),
        "yearly" => Ok(Recurrence::Yearly),
        "weekly" => {
            let mut ws = weekdays.unwrap_or_default();
            ws.retain(|w| (1..=7).contains(w));
            ws.sort_unstable();
            ws.dedup();
            Ok(Recurrence::Weekly { weekdays: ws })
        }
        other => Err(format!(
            "无效循环类型: {other}（daily|weekly|monthly|yearly|once）"
        )),
    }
}

#[tauri::command]
fn create_task(params: CreateTaskParams) -> R<Task> {
    let style = match params.card_style.as_deref() {
        Some(s) => Some(
            dashboard_domain::CardStyle::parse(s).ok_or_else(|| format!("无效卡片样式: {s}"))?,
        ),
        None => None,
    };
    let c = conn()?;
    let mut input = core::task_service::CreateTaskInput {
        title: params.title,
        icon: params.icon.unwrap_or_default(),
        unit: params.unit.unwrap_or_default(),
        project_id: params.project_id,
        library_id: params.library_id,
        ..Default::default()
    };
    if let Some(t) = params.target {
        input.daily_target = t;
    }
    if let Some(col) = params.color {
        input.color_hex = col;
    }
    if let Some(s) = style {
        input.card_style = s;
    }
    if let Some(kind) = &params.recurrence {
        input.recurrence = parse_recurrence(kind, params.weekdays.clone())?;
    }
    if let Some(p) = params.priority {
        input.priority = p;
    }
    core::task_service::create_task(&c, &input, actor_from(params.actor)?)
        .map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTaskParams {
    id: String,
    title: Option<String>,
    target: Option<i64>,
    unit: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    card_style: Option<String>,
    project_id: Option<String>,
    /// JSON null 无法区分"不变"与"清空"，显式清空项目归属用这个标志。
    clear_project: Option<bool>,
    recurrence: Option<String>,
    weekdays: Option<Vec<u8>>,
    priority: Option<i64>,
    actor: Option<String>,
}

#[tauri::command]
fn update_task(params: UpdateTaskParams) -> R<Task> {
    let style = match params.card_style.as_deref() {
        Some(s) => Some(
            dashboard_domain::CardStyle::parse(s).ok_or_else(|| format!("无效卡片样式: {s}"))?,
        ),
        None => None,
    };
    // projectId 有值 = 设为新项目；clearProject = 移出项目；两者都不传 = 不变。
    let project_id = match (params.clear_project.unwrap_or(false), params.project_id) {
        (true, _) => Some(None),
        (false, Some(p)) => Some(Some(p)),
        (false, None) => None,
    };
    let recurrence = match &params.recurrence {
        Some(kind) => Some(parse_recurrence(kind, params.weekdays.clone())?),
        None => None,
    };
    let c = conn()?;
    let input = core::task_service::UpdateTaskInput {
        title: params.title,
        icon: params.icon,
        color_hex: params.color,
        unit: params.unit,
        daily_target: params.target,
        card_style: style,
        recurrence,
        project_id,
        priority: params.priority,
    };
    core::task_service::update_task(&c, &params.id, &input, actor_from(params.actor)?)
        .map_err(|e| e.to_string())
}

/// 拖拽落位（006）：同档内重排（priority=None）或跨档改级+落位。
/// before_id/after_id = 全局序列中落点的上下邻居。
#[tauri::command]
fn move_task_position(
    id: String,
    priority: Option<i64>,
    before_id: Option<String>,
    after_id: Option<String>,
    actor: Option<String>,
) -> R<Task> {
    let c = conn()?;
    core::task_service::move_task_position(
        &c,
        &id,
        priority,
        before_id.as_deref(),
        after_id.as_deref(),
        actor_from(actor)?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_task(id: String, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::task_service::archive_task(&c, &id, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_task(id: String, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::task_service::restore_task(&c, &id, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(id: String, actor: Option<String>) -> R<()> {
    let c = conn()?;
    core::task_service::delete_task(&c, &id, false, actor_from(actor)?)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ---------------- Check-in ----------------

#[tauri::command]
fn task_checkin(
    id: String,
    operation_id: Option<String>,
    actor: Option<String>,
) -> R<core::checkin_service::TaskDayView> {
    let op = operation_id.unwrap_or_else(|| dashboard_domain::new_id("op"));
    let c = conn()?;
    core::checkin_service::record(&c, &id, &op, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn task_decrement(
    id: String,
    operation_id: Option<String>,
    actor: Option<String>,
) -> R<core::checkin_service::TaskDayView> {
    let op = operation_id.unwrap_or_else(|| dashboard_domain::new_id("op"));
    let c = conn()?;
    core::checkin_service::decrement(&c, &id, &op, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn task_undo(
    id: String,
    operation_id: Option<String>,
    actor: Option<String>,
) -> R<core::checkin_service::TaskDayView> {
    let op = operation_id.unwrap_or_else(|| dashboard_domain::new_id("op"));
    let c = conn()?;
    core::checkin_service::undo(&c, &id, &op, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn task_overview(
    id: String,
    period: String,
    anchor: Option<String>,
) -> R<core::overview_service::PeriodOverview> {
    let c = conn()?;
    core::overview_service::task_period_overview(&c, &id, &period, anchor.as_deref())
        .map_err(|e| e.to_string())
}

// ---------------- Date Libraries ----------------

#[tauri::command]
fn list_libraries(
    include_archived: Option<bool>,
) -> R<Vec<core::library_service::LibraryListItem>> {
    let c = conn()?;
    core::library_service::list_library_items(&c, include_archived.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_library(
    title: String,
    kind: String,
    anchor_day: String,
    note: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    actor: Option<String>,
) -> R<dashboard_domain::DateLibrary> {
    let k = dashboard_domain::LibraryKind::parse(&kind)
        .ok_or_else(|| format!("无效重要日类型: {kind}"))?;
    let c = conn()?;
    let input = core::library_service::CreateLibraryInput {
        title,
        note: note.unwrap_or_default(),
        icon: icon.unwrap_or_default(),
        color_hex: color.unwrap_or_else(|| "#4A90E2".into()),
        kind: k,
        anchor_day,
    };
    core::library_service::create_library(&c, &input, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_library(
    id: String,
    title: Option<String>,
    note: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    anchor_day: Option<String>,
    actor: Option<String>,
) -> R<dashboard_domain::DateLibrary> {
    let c = conn()?;
    let input = core::library_service::UpdateLibraryInput {
        title,
        note,
        icon,
        color_hex: color,
        anchor_day,
    };
    core::library_service::update_library(&c, &id, &input, actor_from(actor)?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_library(
    id: String,
    mode: String,
    move_to: Option<String>,
    actor: Option<String>,
) -> R<dashboard_domain::DateLibrary> {
    let m = match mode.as_str() {
        "keep" => core::library_service::ArchiveTaskMode::Keep,
        "detach" => core::library_service::ArchiveTaskMode::Detach,
        "move_to" | "move-to" => core::library_service::ArchiveTaskMode::MoveTo,
        _ => return Err(format!("无效 mode: {mode}（keep|detach|move_to）")),
    };
    let c = conn()?;
    core::library_service::archive_library(&c, &id, m, move_to.as_deref(), actor_from(actor)?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_library(id: String, actor: Option<String>) -> R<dashboard_domain::DateLibrary> {
    let c = conn()?;
    core::library_service::restore_library(&c, &id, actor_from(actor)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn library_tasks(library_id: String) -> R<Vec<Task>> {
    let c = conn()?;
    core::library_service::library_tasks(&c, &library_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn library_heatmap(
    library_id: String,
    anchor: Option<String>,
) -> R<core::overview_service::LibraryYearHeatmap> {
    let c = conn()?;
    core::overview_service::library_year_heatmap(&c, &library_id, anchor.as_deref())
        .map_err(|e| e.to_string())
}

/// 全局年度综合热力图（所有任务聚合，首页用）。
#[tauri::command]
fn global_year_heatmap(anchor: Option<String>) -> R<core::overview_service::GlobalYearHeatmap> {
    let c = conn()?;
    core::overview_service::global_year_heatmap(&c, anchor.as_deref()).map_err(|e| e.to_string())
}

/// 移动任务到重要日（library_id=None = 移出为独立任务）。
#[tauri::command]
fn move_task_library(id: String, library_id: Option<String>, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::library_service::move_task(&c, &id, library_id.as_deref(), actor_from(actor)?)
        .map_err(|e| e.to_string())
}

// ---------------- Projects ----------------

#[tauri::command]
fn list_projects() -> R<Vec<core::project_service::ProjectWithStats>> {
    let c = conn()?;
    core::project_service::list_projects_with_stats(&c).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_project(name: String, description: Option<String>) -> R<Project> {
    let c = conn()?;
    core::project_service::create_project(&c, &name, description.as_deref().unwrap_or(""), actor())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_project(id: String) -> R<()> {
    let c = conn()?;
    core::project_service::archive_project(&c, &id, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_project(id: String, name: Option<String>, description: Option<String>) -> R<Project> {
    let c = conn()?;
    core::project_service::update_project(&c, &id, name.as_deref(), description.as_deref(), actor())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_project(id: String) -> R<()> {
    let c = conn()?;
    core::project_service::delete_project(&c, &id, false, actor()).map_err(|e| e.to_string())
}

// ---------------- Notes ----------------

#[tauri::command]
fn list_notes(limit: Option<i64>) -> R<Vec<Note>> {
    let c = conn()?;
    core::note_service::recent_notes(&c, limit.unwrap_or(100)).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(title: String, body: String, actor: Option<String>) -> R<Note> {
    let c = conn()?;
    core::note_service::create_note(&c, &title, &body, actor_from(actor)?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_note(id: String, title: String, body: String) -> R<Note> {
    let c = conn()?;
    core::note_service::update_note(&c, &id, &title, &body, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_note(id: String) -> R<()> {
    let c = conn()?;
    core::note_service::delete_note(&c, &id, actor()).map_err(|e| e.to_string())
}

// ---------------- Inbox ----------------

#[tauri::command]
fn list_inbox(include_processed: Option<bool>) -> R<Vec<dashboard_domain::InboxItem>> {
    let c = conn()?;
    core::inbox_service::list_items(&c, include_processed.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_inbox_item(content: String, actor: Option<String>) -> R<dashboard_domain::InboxItem> {
    let c = conn()?;
    core::inbox_service::add_item(&c, &content, "desktop", actor_from(actor)?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn inbox_to_task(id: String) -> R<core::inbox_service::ProcessReport> {
    let c = conn()?;
    core::inbox_service::process_to_task(&c, &id, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn inbox_to_note(id: String) -> R<core::inbox_service::ProcessReport> {
    let c = conn()?;
    core::inbox_service::process_to_note(&c, &id, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_inbox_item(id: String) -> R<()> {
    let c = conn()?;
    core::inbox_service::delete_item(&c, &id, actor()).map_err(|e| e.to_string())
}

// ---------------- Search ----------------

#[tauri::command]
fn search_all(query: String) -> R<core::search_service::SearchResults> {
    let c = conn()?;
    core::search_service::search(&c, &query).map_err(|e| e.to_string())
}

// ---------------- Plugins ----------------

fn checked_plugin_id(id: &str) -> R<()> {
    core::plugin_service::validate_plugin_id(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_list(app: tauri::AppHandle) -> R<Vec<core::plugin_service::PluginInfo>> {
    let c = conn()?;
    let infos = core::plugin_service::list_installed(&c, &core::plugin_manifest::plugins_root())
        .map_err(|e| e.to_string())?;
    app.state::<plugin_cron::CronScheduler>().sync(&app, &infos);
    Ok(infos)
}

#[tauri::command]
fn plugin_read_manifest(id: String) -> R<core::plugin_manifest::PluginManifest> {
    checked_plugin_id(&id)?;
    let dir = core::plugin_manifest::plugins_root().join(&id);
    let manifest = core::plugin_manifest::load_from_dir(&dir).map_err(|e| e.to_string())?;
    if manifest.id != id {
        return Err("插件目录名与 manifest.id 不一致".into());
    }
    Ok(manifest)
}

type PluginResult<T> = Result<T, core::plugin_runtime::PluginFault>;
#[tauri::command]
fn plugin_open_context(
    id: String,
    store: State<'_, core::plugin_runtime::PluginRuntime>,
) -> PluginResult<core::plugin_runtime::PluginSession> {
    let c = storage::open_default().map_err(core::CoreError::from)?;
    Ok(store.open(&c, &core::plugin_manifest::plugins_root(), &id)?)
}
#[tauri::command]
fn plugin_close_context(token: String, store: State<'_, core::plugin_runtime::PluginRuntime>) {
    store.close(&token);
}
#[tauri::command]
fn plugin_load_source(
    token: String,
    store: State<'_, core::plugin_runtime::PluginRuntime>,
) -> PluginResult<String> {
    let c = storage::open_default().map_err(core::CoreError::from)?;
    Ok(store.load_source(&c, &core::plugin_manifest::plugins_root(), &token)?)
}
#[tauri::command]
async fn plugin_call(
    app: tauri::AppHandle,
    token: String,
    call: core::plugin_runtime::PluginCall,
) -> PluginResult<serde_json::Value> {
    tauri::async_runtime::spawn_blocking(move || {
        let c = storage::open_default().map_err(core::CoreError::from)?;
        let result = app.state::<core::plugin_runtime::PluginRuntime>().call(
            &c,
            &core::plugin_manifest::plugins_root(),
            &token,
            call,
        );
        if matches!(&result, Err(core::CoreError::PermissionDenied(_))) {
            use tauri::Emitter;
            let _ = app.emit(
                "plugin-denied",
                serde_json::json!({"code":"permission_denied"}),
            );
        }
        result.map_err(Into::into)
    })
    .await
    .map_err(|e| core::plugin_runtime::PluginFault {
        code: "internal".into(),
        message: e.to_string(),
    })?
}
#[tauri::command]
fn plugin_set_enabled(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
    fingerprint: Option<String>,
) -> R<dashboard_domain::PluginRegistration> {
    checked_plugin_id(&id)?;
    if enabled && fingerprint.is_none() {
        return Err("启用前必须审阅权限".into());
    }
    let c = conn()?;
    let result = core::plugin_service::set_plugin_enabled_checked(
        &c,
        &core::plugin_manifest::plugins_root(),
        &id,
        enabled,
        Actor::User,
        fingerprint.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    app.state::<core::plugin_runtime::PluginRuntime>()
        .revoke_plugin(&id);
    app.state::<plugin_cron::CronScheduler>().rescan(&app);
    Ok(result)
}

#[tauri::command]
fn plugin_disable_all(app: tauri::AppHandle) -> R<()> {
    let c = conn()?;
    core::plugin_service::disable_all(&c, &core::plugin_manifest::plugins_root(), Actor::User)
        .map_err(|e| e.to_string())?;
    app.state::<plugin_cron::CronScheduler>().rescan(&app);
    Ok(())
}

/// 唤出面板：应用整体可能被 macOS 隐藏（⌘H / 隐藏其他），仅 window.show() 不够，
/// 必须先 app.show() 解除应用级隐藏；窗口也可能处于最小化，一并恢复。
fn show_panel(app: &tauri::AppHandle) {
    let _ = app.show();
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(plugin_cron::CronScheduler::default())
        .manage(core::plugin_runtime::PluginRuntime::default())
        .on_window_event(|window, event| {
            // 关窗 = 隐藏到托盘：面板后台留存，插件随面板继续运行（plugin-system/v1 决策）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem},
                tray::{TrayIconBuilder, TrayIconEvent},
            };
            let show = MenuItem::with_id(app, "show", "显示面板", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut builder = TrayIconBuilder::with_id("main-tray")
                .tooltip("AIODashboard")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_panel(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        show_panel(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }
            builder.build(app)?;
            // 启动时扫描启用插件的 cron 声明并调度
            app.state::<plugin_cron::CronScheduler>()
                .rescan(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_today,
            task_wall_views,
            list_tasks,
            list_archived_tasks,
            create_task,
            update_task,
            move_task_position,
            archive_task,
            restore_task,
            delete_task,
            task_checkin,
            task_decrement,
            task_undo,
            task_overview,
            list_libraries,
            create_library,
            update_library,
            archive_library,
            restore_library,
            library_tasks,
            library_heatmap,
            global_year_heatmap,
            move_task_library,
            list_projects,
            create_project,
            archive_project,
            update_project,
            delete_project,
            list_notes,
            create_note,
            update_note,
            delete_note,
            list_inbox,
            add_inbox_item,
            inbox_to_task,
            inbox_to_note,
            delete_inbox_item,
            search_all,
            plugin_list,
            plugin_import::plugin_pick_import_source,
            plugin_import::plugin_preview_import,
            plugin_import::plugin_import,
            plugin_read_manifest,
            plugin_open_context,
            plugin_close_context,
            plugin_load_source,
            plugin_set_enabled,
            plugin_call,
            plugin_disable_all
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 点程序坞图标唤出面板：窗口隐藏到托盘后，macOS 对 Dock 点击发 Reopen
            if let tauri::RunEvent::Reopen { .. } = event {
                show_panel(app);
            }
        });
}

#[cfg(test)]
mod plugin_command_tests {
    #[test]
    fn ordinary_gui_rejects_forged_actor() {
        for actor in [
            "cli",
            "ai",
            "automation",
            "system",
            "plugin:com.test.a",
            "bad",
        ] {
            assert!(super::actor_from(Some(actor.into())).is_err());
        }
        assert!(super::actor_from(None).is_ok());
        assert!(super::actor_from(Some("user".into())).is_ok());
    }
}
