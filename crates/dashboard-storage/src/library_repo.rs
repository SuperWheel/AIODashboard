//! Date Library Repository：重要日（纪念日 / 倒计时日）。

use chrono::Utc;
use dashboard_domain::{id_prefix, new_id, DateLibrary, LibraryKind, LibraryStatus};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::{read_time, write_time};

const COLS: &str =
    "id, title, note, icon, color_hex, kind, anchor_day, sort_order, status, created_at, updated_at";

fn invalid(col: usize, what: &str, raw: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        col,
        rusqlite::types::Type::Text,
        format!("invalid {what}: {raw}").into(),
    )
}

fn map_library(row: &Row) -> rusqlite::Result<DateLibrary> {
    let kind_raw: String = row.get(5)?;
    let status_raw: String = row.get(8)?;
    Ok(DateLibrary {
        id: row.get(0)?,
        title: row.get(1)?,
        note: row.get(2)?,
        icon: row.get(3)?,
        color_hex: row.get(4)?,
        kind: LibraryKind::parse(&kind_raw).ok_or_else(|| invalid(5, "library kind", &kind_raw))?,
        anchor_day: row.get(6)?,
        sort_order: row.get(7)?,
        status: LibraryStatus::parse(&status_raw)
            .ok_or_else(|| invalid(8, "library status", &status_raw))?,
        created_at: read_time(row, 9)?,
        updated_at: read_time(row, 10)?,
    })
}

#[derive(Debug)]
pub struct NewLibrary {
    pub title: String,
    pub note: String,
    pub icon: String,
    pub color_hex: String,
    pub kind: LibraryKind,
    pub anchor_day: String,
}

pub fn create(conn: &Connection, input: &NewLibrary) -> rusqlite::Result<DateLibrary> {
    let now = Utc::now();
    let max_sort: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM date_libraries",
        [],
        |r| r.get(0),
    )?;
    let lib = DateLibrary {
        id: new_id(id_prefix::DATE_LIBRARY),
        title: input.title.clone(),
        note: input.note.clone(),
        icon: input.icon.clone(),
        color_hex: input.color_hex.clone(),
        kind: input.kind,
        anchor_day: input.anchor_day.clone(),
        sort_order: max_sort + 1,
        status: LibraryStatus::Active,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO date_libraries
         (id, title, note, icon, color_hex, kind, anchor_day, sort_order, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            lib.id,
            lib.title,
            lib.note,
            lib.icon,
            lib.color_hex,
            lib.kind.as_str(),
            lib.anchor_day,
            lib.sort_order,
            lib.status.as_str(),
            write_time(lib.created_at),
            write_time(lib.updated_at),
        ],
    )?;
    Ok(lib)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<DateLibrary>> {
    conn.query_row(
        &format!("SELECT {COLS} FROM date_libraries WHERE id = ?1"),
        params![id],
        map_library,
    )
    .optional()
}

pub fn list(conn: &Connection, include_archived: bool) -> rusqlite::Result<Vec<DateLibrary>> {
    let sql = if include_archived {
        format!("SELECT {COLS} FROM date_libraries ORDER BY sort_order ASC, created_at ASC")
    } else {
        format!(
            "SELECT {COLS} FROM date_libraries WHERE status = 'active'
             ORDER BY sort_order ASC, created_at ASC"
        )
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_library)?;
    rows.collect()
}

#[derive(Debug, Default, Clone)]
pub struct LibraryPatch {
    pub title: Option<String>,
    pub note: Option<String>,
    pub icon: Option<String>,
    pub color_hex: Option<String>,
    pub anchor_day: Option<String>,
    pub status: Option<LibraryStatus>,
}

pub fn update(conn: &Connection, id: &str, patch: &LibraryPatch) -> rusqlite::Result<()> {
    let mut sets: Vec<String> = Vec::new();
    let mut args: Vec<rusqlite::types::Value> = Vec::new();

    macro_rules! set_text {
        ($field:expr, $val:expr) => {{
            args.push(rusqlite::types::Value::Text(($val).to_string()));
            sets.push(format!("{} = ?{}", $field, args.len()));
        }};
    }

    if let Some(title) = &patch.title {
        set_text!("title", title);
    }
    if let Some(note) = &patch.note {
        set_text!("note", note);
    }
    if let Some(icon) = &patch.icon {
        set_text!("icon", icon);
    }
    if let Some(color) = &patch.color_hex {
        set_text!("color_hex", color);
    }
    if let Some(anchor) = &patch.anchor_day {
        set_text!("anchor_day", anchor);
    }
    if let Some(status) = patch.status {
        set_text!("status", status.as_str());
    }

    if sets.is_empty() {
        return Ok(());
    }
    args.push(rusqlite::types::Value::from(write_time(Utc::now())));
    sets.push(format!("updated_at = ?{}", args.len()));

    let sql = format!("UPDATE date_libraries SET {} WHERE id = ?", sets.join(", "));
    let mut all_args = args;
    all_args.push(rusqlite::types::Value::Text(id.to_string()));
    conn.execute(&sql, rusqlite::params_from_iter(all_args.iter()))?;
    Ok(())
}
