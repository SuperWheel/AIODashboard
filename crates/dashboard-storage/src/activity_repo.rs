//! Activity Log Repository

use chrono::{DateTime, Utc};
use dashboard_domain::{id_prefix, new_id, ActivityEntry, Actor};
use rusqlite::{params, Connection, Row};

use crate::timeutil::{read_time, write_time};

const COLS: &str = "id, ts, actor, action, object_type, object_id, detail";

fn map_entry(row: &Row) -> rusqlite::Result<ActivityEntry> {
    let actor_raw: String = row.get(2)?;
    Ok(ActivityEntry {
        id: row.get(0)?,
        ts: read_time(row, 1)?,
        actor: Actor::parse(&actor_raw).unwrap_or(Actor::System),
        action: row.get(3)?,
        object_type: row.get(4)?,
        object_id: row.get(5)?,
        detail: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(serde_json::Value::Null),
    })
}

pub fn record(
    conn: &Connection,
    ts: DateTime<Utc>,
    actor: Actor,
    action: &str,
    object_type: &str,
    object_id: Option<&str>,
    detail: &serde_json::Value,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO activity_log (id, ts, actor, action, object_type, object_id, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new_id(id_prefix::ACTIVITY),
            write_time(ts),
            actor.as_str(),
            action,
            object_type,
            object_id,
            detail.to_string(),
        ],
    )?;
    Ok(())
}

pub fn list(
    conn: &Connection,
    limit: i64,
    actor_filter: Option<Actor>,
) -> rusqlite::Result<Vec<ActivityEntry>> {
    let (sql, args): (String, Vec<Box<dyn rusqlite::ToSql>>) = match actor_filter {
        Some(actor) => (
            format!(
                "SELECT {COLS} FROM activity_log WHERE actor=?1 ORDER BY ts DESC LIMIT {limit}"
            ),
            vec![Box::new(actor.as_str())],
        ),
        None => (
            format!("SELECT {COLS} FROM activity_log ORDER BY ts DESC LIMIT {limit}"),
            vec![],
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_entry)?;
    rows.collect()
}
