//! AIODashboard Desktop —— Tauri Interface 层。
//!
//! 所有 Command 只做参数转换，业务全部委托 dashboard-core。
//! 与 CLI 平级：GUI 创建的任务 = CLI 创建的任务。

pub mod plugin_cron;

use dashboard_core as core;
use dashboard_domain::{Actor, Note, Project, Task};
use dashboard_storage as storage;
use rusqlite::Connection;
use tauri::Manager;

type R<T> = Result<T, String>;

fn conn() -> R<Connection> {
    storage::open_default().map_err(|e| e.to_string())
}

fn actor() -> Actor {
    Actor::User
}

/// 可选 actor 参数：普通前端调用不传（= User）；插件桥传 `plugin:<id>`（审计归因）。
fn actor_from(actor: Option<String>) -> Actor {
    match actor {
        Some(a) => Actor::parse(&a).unwrap_or(Actor::User),
        None => Actor::User,
    }
}

// ---------------- Today / Tasks ----------------

#[tauri::command]
fn get_today() -> R<core::context_service::TodayContext> {
    let c = conn()?;
    core::context_service::context_today(&c).map_err(|e| e.to_string())
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

#[derive(Debug, serde::Deserialize)]
struct CreateTaskParams {
    title: String,
    target: Option<i64>,
    unit: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    card_style: Option<String>,
    project_id: Option<String>,
    library_id: Option<String>,
    actor: Option<String>,
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
    core::task_service::create_task(&c, &input, actor_from(params.actor)).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
struct UpdateTaskParams {
    id: String,
    title: Option<String>,
    target: Option<i64>,
    unit: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    card_style: Option<String>,
    project_id: Option<Option<String>>,
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
    let c = conn()?;
    let input = core::task_service::UpdateTaskInput {
        title: params.title,
        icon: params.icon,
        color_hex: params.color,
        unit: params.unit,
        daily_target: params.target,
        card_style: style,
        project_id: params.project_id,
    };
    core::task_service::update_task(&c, &params.id, &input, actor_from(params.actor))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_task(id: String, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::task_service::archive_task(&c, &id, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_task(id: String, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::task_service::restore_task(&c, &id, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(id: String, actor: Option<String>) -> R<()> {
    let c = conn()?;
    core::task_service::delete_task(&c, &id, false, actor_from(actor))
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
    core::checkin_service::record(&c, &id, &op, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn task_decrement(
    id: String,
    operation_id: Option<String>,
    actor: Option<String>,
) -> R<core::checkin_service::TaskDayView> {
    let op = operation_id.unwrap_or_else(|| dashboard_domain::new_id("op"));
    let c = conn()?;
    core::checkin_service::decrement(&c, &id, &op, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn task_undo(
    id: String,
    operation_id: Option<String>,
    actor: Option<String>,
) -> R<core::checkin_service::TaskDayView> {
    let op = operation_id.unwrap_or_else(|| dashboard_domain::new_id("op"));
    let c = conn()?;
    core::checkin_service::undo(&c, &id, &op, actor_from(actor)).map_err(|e| e.to_string())
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
        .ok_or_else(|| format!("无效主库类型: {kind}"))?;
    let c = conn()?;
    let input = core::library_service::CreateLibraryInput {
        title,
        note: note.unwrap_or_default(),
        icon: icon.unwrap_or_default(),
        color_hex: color.unwrap_or_else(|| "#4A90E2".into()),
        kind: k,
        anchor_day,
    };
    core::library_service::create_library(&c, &input, actor_from(actor)).map_err(|e| e.to_string())
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
    core::library_service::update_library(&c, &id, &input, actor_from(actor))
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
    core::library_service::archive_library(&c, &id, m, move_to.as_deref(), actor_from(actor))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_library(id: String, actor: Option<String>) -> R<dashboard_domain::DateLibrary> {
    let c = conn()?;
    core::library_service::restore_library(&c, &id, actor_from(actor)).map_err(|e| e.to_string())
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

/// 移动任务到主库（library_id=None = 移出为独立任务）。
#[tauri::command]
fn move_task_library(id: String, library_id: Option<String>, actor: Option<String>) -> R<Task> {
    let c = conn()?;
    core::library_service::move_task(&c, &id, library_id.as_deref(), actor_from(actor))
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
    core::note_service::create_note(&c, &title, &body, actor_from(actor)).map_err(|e| e.to_string())
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
    core::inbox_service::add_item(&c, &content, "desktop", actor_from(actor))
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
fn plugin_list() -> R<Vec<core::plugin_service::PluginInfo>> {
    let c = conn()?;
    core::plugin_service::list_installed(&c, &core::plugin_manifest::plugins_root())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_read_manifest(id: String) -> R<core::plugin_manifest::PluginManifest> {
    checked_plugin_id(&id)?;
    let dir = core::plugin_manifest::plugins_root().join(&id);
    core::plugin_manifest::load_from_dir(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_set_enabled(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> R<dashboard_domain::PluginRegistration> {
    checked_plugin_id(&id)?;
    let c = conn()?;
    let reg = core::plugin_service::set_plugin_enabled(
        &c,
        &core::plugin_manifest::plugins_root(),
        &id,
        enabled,
        Actor::User,
    )
    .map_err(|e| e.to_string())?;
    drop(c);
    // 启停可能改变 cron 声明集，重排调度
    app.state::<plugin_cron::CronScheduler>().rescan(&app);
    Ok(reg)
}

/// 返回插件入口 JS 源码（前端 loader 拿去 blob import）。停用 / 未注册的插件拒绝加载。
#[tauri::command]
fn plugin_load_source(id: String) -> R<String> {
    checked_plugin_id(&id)?;
    let c = conn()?;
    let reg = core::plugin_service::get_registration(&c, &id).map_err(|e| e.to_string())?;
    if !reg.enabled {
        return Err(format!("插件 {id} 已停用"));
    }
    drop(c);
    let dir = core::plugin_manifest::plugins_root().join(&id);
    let m = core::plugin_manifest::load_from_dir(&dir).map_err(|e| e.to_string())?;
    std::fs::read_to_string(dir.join(&m.entry)).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_kv_get(plugin_id: String, key: String) -> R<Option<String>> {
    checked_plugin_id(&plugin_id)?;
    let c = conn()?;
    core::plugin_service::kv_get(&c, &plugin_id, &key).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_kv_set(plugin_id: String, key: String, value: String) -> R<()> {
    checked_plugin_id(&plugin_id)?;
    let c = conn()?;
    core::plugin_service::kv_set(&c, &plugin_id, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_kv_delete(plugin_id: String, key: String) -> R<bool> {
    checked_plugin_id(&plugin_id)?;
    let c = conn()?;
    core::plugin_service::kv_delete(&c, &plugin_id, &key).map_err(|e| e.to_string())
}

#[tauri::command]
fn plugin_kv_list(
    plugin_id: String,
    key_prefix: Option<String>,
) -> R<Vec<core::plugin_service::PluginKvEntry>> {
    checked_plugin_id(&plugin_id)?;
    let c = conn()?;
    core::plugin_service::kv_list(&c, &plugin_id, key_prefix.as_deref()).map_err(|e| e.to_string())
}

/// 插件 HTTP 代理：校验 manifest 网络白名单 → 后台线程请求 → 全量审计。
/// v1 仅 GET；返回 { status, text, json }（json 为可解析时的结构化结果）。
#[tauri::command]
async fn plugin_http_fetch(plugin_id: String, url: String) -> R<serde_json::Value> {
    checked_plugin_id(&plugin_id)?;
    let dir = core::plugin_manifest::plugins_root().join(&plugin_id);
    let manifest = core::plugin_manifest::load_from_dir(&dir).map_err(|e| e.to_string())?;
    core::plugin_manifest::check_network_allowed(&manifest.permissions.network, &url)
        .map_err(|e| e.to_string())?;

    let url_owned = url.clone();
    let (status, text) =
        tauri::async_runtime::spawn_blocking(move || -> Result<(u16, String), String> {
            let resp = ureq::get(&url_owned)
                .timeout(std::time::Duration::from_secs(15))
                .call()
                .map_err(|e| e.to_string())?;
            let status = resp.status();
            let text = resp.into_string().map_err(|e| e.to_string())?;
            Ok((status, text))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let json = serde_json::from_str::<serde_json::Value>(&text).ok();
    {
        let c = conn()?;
        core::log_activity(
            &c,
            chrono::Utc::now(),
            Actor::Plugin(plugin_id.clone()),
            "plugin.http_fetch",
            "plugin",
            Some(&plugin_id),
            &serde_json::json!({ "url": url, "status": status }),
        );
    }
    Ok(serde_json::json!({ "status": status, "text": text, "json": json }))
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
        .manage(plugin_cron::CronScheduler::default())
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
            list_tasks,
            create_task,
            update_task,
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
            plugin_read_manifest,
            plugin_set_enabled,
            plugin_load_source,
            plugin_kv_get,
            plugin_kv_set,
            plugin_kv_delete,
            plugin_kv_list,
            plugin_http_fetch
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
