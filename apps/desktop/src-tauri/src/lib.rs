//! AIODashboard Desktop —— Tauri Interface 层。
//!
//! 所有 Command 只做参数转换，业务全部委托 dashboard-core。
//! 与 CLI 平级：GUI 创建的任务 = CLI 创建的任务。

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

/// scope: "all" | "open" | "done" | "today" | "overdue"
#[tauri::command]
fn list_tasks(scope: Option<String>) -> R<Vec<Task>> {
    use dashboard_storage::task_repo::TaskQuery;
    let c = conn()?;
    let mut q = TaskQuery {
        limit: 500,
        order_by_due: true,
        ..Default::default()
    };
    match scope.as_deref() {
        Some("open") => q.exclude_done = true,
        Some("done") => q.status = Some(dashboard_domain::TaskStatus::Done),
        Some("today") => {
            let (s, e) = core::context_service::local_today_range(chrono::Utc::now());
            q.due_from = Some(s);
            q.due_to = Some(e);
            q.exclude_done = true;
            q.limit = 200;
        }
        Some("overdue") => {
            let (s, _) = core::context_service::local_today_range(chrono::Utc::now());
            q.due_to = Some(s);
            q.exclude_done = true;
            q.limit = 200;
        }
        _ => {}
    }
    core::task_service::list_tasks(&c, &q).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_task(
    title: String,
    due_at: Option<String>,
    project_id: Option<String>,
    actor: Option<String>,
) -> R<Task> {
    let due = match due_at.as_deref() {
        None | Some("") | Some("null") => None,
        Some(s) => Some(core::parse_due_input(s).map_err(|e| e.to_string())?),
    };
    let c = conn()?;
    let input = core::task_service::CreateTaskInput {
        title,
        due_at: due,
        project_id,
    };
    core::task_service::create_task(&c, &input, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_task_status(id: String, status: String, actor: Option<String>) -> R<Task> {
    let st = dashboard_domain::TaskStatus::parse(&status)
        .ok_or_else(|| format!("无效状态: {status}"))?;
    let c = conn()?;
    core::task_service::set_status(&c, &id, st, actor_from(actor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(id: String, actor: Option<String>) -> R<()> {
    let c = conn()?;
    core::task_service::delete_task(&c, &id, false, actor_from(actor))
        .map(|_| ())
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
    core::inbox_service::process_to_task(&c, &id, None, actor()).map_err(|e| e.to_string())
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
fn plugin_set_enabled(id: String, enabled: bool) -> R<dashboard_domain::PluginRegistration> {
    checked_plugin_id(&id)?;
    let c = conn()?;
    core::plugin_service::set_plugin_enabled(
        &c,
        &core::plugin_manifest::plugins_root(),
        &id,
        enabled,
        Actor::User,
    )
    .map_err(|e| e.to_string())
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

pub fn run() {
    tauri::Builder::default()
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
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }
            builder.build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_today,
            list_tasks,
            create_task,
            set_task_status,
            delete_task,
            list_projects,
            create_project,
            archive_project,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
