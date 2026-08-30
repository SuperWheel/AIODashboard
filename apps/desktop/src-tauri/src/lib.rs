//! AIODashboard Desktop —— Tauri Interface 层。
//!
//! 所有 Command 只做参数转换，业务全部委托 dashboard-core。
//! 与 CLI 平级：GUI 创建的任务 = CLI 创建的任务。

use dashboard_core as core;
use dashboard_domain::{Actor, Note, Project, Task};
use dashboard_storage as storage;
use rusqlite::Connection;

type R<T> = Result<T, String>;

fn conn() -> R<Connection> {
    storage::open_default().map_err(|e| e.to_string())
}

fn actor() -> Actor {
    Actor::User
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
fn create_task(title: String, due_at: Option<String>, project_id: Option<String>) -> R<Task> {
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
    core::task_service::create_task(&c, &input, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_task_status(id: String, status: String) -> R<Task> {
    let st = dashboard_domain::TaskStatus::parse(&status)
        .ok_or_else(|| format!("无效状态: {status}"))?;
    let c = conn()?;
    core::task_service::set_status(&c, &id, st, actor()).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(id: String) -> R<()> {
    let c = conn()?;
    core::task_service::delete_task(&c, &id, false, actor())
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
fn create_note(title: String, body: String) -> R<Note> {
    let c = conn()?;
    core::note_service::create_note(&c, &title, &body, actor()).map_err(|e| e.to_string())
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
fn add_inbox_item(content: String) -> R<dashboard_domain::InboxItem> {
    let c = conn()?;
    core::inbox_service::add_item(&c, &content, "desktop", actor()).map_err(|e| e.to_string())
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

pub fn run() {
    tauri::Builder::default()
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
            search_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
