//! Project Repository

use chrono::Utc;

use dashboard_domain::{id_prefix, new_id, Project, ProjectStatus};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::{read_time, write_time};

const COLS: &str = "id, name, description, status, created_at, updated_at";

fn map_project(row: &Row) -> rusqlite::Result<Project> {
    let status_raw: String = row.get(3)?;
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: match status_raw.as_str() {
            "active" => ProjectStatus::Active,
            _ => ProjectStatus::Archived,
        },
        created_at: read_time(row, 4)?,
        updated_at: read_time(row, 5)?,
    })
}

pub fn create(conn: &Connection, name: &str, description: &str) -> rusqlite::Result<Project> {
    let now = Utc::now();
    let project = Project {
        id: new_id(id_prefix::PROJECT),
        name: name.to_string(),
        description: description.to_string(),
        status: ProjectStatus::Active,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO projects (id, name, description, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            project.id,
            project.name,
            project.description,
            project.status.as_str(),
            write_time(project.created_at),
            write_time(project.updated_at),
        ],
    )?;
    Ok(project)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<Project>> {
    conn.query_row(
        &format!("SELECT {COLS} FROM projects WHERE id = ?1"),
        params![id],
        map_project,
    )
    .optional()
}

pub fn list(conn: &Connection, include_archived: bool) -> rusqlite::Result<Vec<Project>> {
    let sql = if include_archived {
        format!("SELECT {COLS} FROM projects ORDER BY created_at ASC")
    } else {
        format!("SELECT {COLS} FROM projects WHERE status='active' ORDER BY created_at ASC")
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_project)?;
    rows.collect()
}

pub fn set_status(conn: &Connection, id: &str, status: ProjectStatus) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE projects SET status=?2, updated_at=?3 WHERE id=?1",
        params![id, status.as_str(), write_time(Utc::now())],
    )?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, name: &str, description: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE projects SET name=?2, description=?3, updated_at=?4 WHERE id=?1",
        params![id, name, description, write_time(Utc::now())],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
}

pub fn open_task_count(conn: &Connection, project_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE project_id=?1 AND status != 'done'",
        params![project_id],
        |r| r.get(0),
    )
}
