//! Plugin Repository：插件注册表与插件命名空间 KV。

use chrono::{DateTime, Utc};
use dashboard_domain::PluginRegistration;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::timeutil::{read_time, write_time};

const REG_COLS: &str = "id, enabled, installed_at";

fn map_registration(row: &Row) -> rusqlite::Result<PluginRegistration> {
    Ok(PluginRegistration {
        id: row.get(0)?,
        enabled: row.get::<_, i64>(1)? != 0,
        installed_at: read_time(row, 2)?,
    })
}

/// 首次发现插件时登记（幂等，已存在则原样保留启停状态）。
pub fn ensure_registered(
    conn: &Connection,
    plugin_id: &str,
    at: DateTime<Utc>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO plugin_registry (id, enabled, installed_at) VALUES (?1, 1, ?2)
         ON CONFLICT(id) DO NOTHING",
        params![plugin_id, write_time(at)],
    )?;
    Ok(())
}

pub fn get_registration(
    conn: &Connection,
    plugin_id: &str,
) -> rusqlite::Result<Option<PluginRegistration>> {
    conn.query_row(
        &format!("SELECT {REG_COLS} FROM plugin_registry WHERE id = ?1"),
        params![plugin_id],
        map_registration,
    )
    .optional()
}

/// 返回受影响行数：0 表示插件未注册（NotFound 判定在 core 层做）。
pub fn set_enabled(conn: &Connection, plugin_id: &str, enabled: bool) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE plugin_registry SET enabled = ?2 WHERE id = ?1",
        params![plugin_id, enabled as i64],
    )
}

pub fn list_registrations(conn: &Connection) -> rusqlite::Result<Vec<PluginRegistration>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {REG_COLS} FROM plugin_registry ORDER BY id ASC"
    ))?;
    let rows = stmt.query_map([], map_registration)?;
    rows.collect()
}

pub fn kv_get(conn: &Connection, plugin_id: &str, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2",
        params![plugin_id, key],
        |r| r.get(0),
    )
    .optional()
}

pub fn kv_set(conn: &Connection, plugin_id: &str, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO plugin_kv (plugin_id, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(plugin_id, key)
         DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![plugin_id, key, value, write_time(Utc::now())],
    )?;
    Ok(())
}

/// 返回是否确有删除（键不存在返回 false）。
pub fn kv_delete(conn: &Connection, plugin_id: &str, key: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM plugin_kv WHERE plugin_id = ?1 AND key = ?2",
        params![plugin_id, key],
    )
}

/// 列出插件全部 KV（按 key 升序）。前缀过滤在 core 层做，避免 SQL LIKE 通配符转义问题。
pub fn kv_list(conn: &Connection, plugin_id: &str) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt =
        conn.prepare("SELECT key, value FROM plugin_kv WHERE plugin_id = ?1 ORDER BY key ASC")?;
    let rows = stmt.query_map(params![plugin_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_db_path(tag: &str) -> std::path::PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "dashboard_plugin_repo_test_{}_{}_{}.db",
            std::process::id(),
            tag,
            n
        ))
    }

    /// T1：plugin_kv 按插件命名空间隔离；覆盖写；删除。
    #[test]
    fn kv_namespaced_by_plugin() {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate(&conn).unwrap();

        kv_set(&conn, "com.a", "counter", "1").unwrap();
        kv_set(&conn, "com.b", "counter", "9").unwrap();
        assert_eq!(
            kv_get(&conn, "com.a", "counter").unwrap().as_deref(),
            Some("1")
        );
        assert_eq!(
            kv_get(&conn, "com.b", "counter").unwrap().as_deref(),
            Some("9")
        );

        // 同插件覆盖写
        kv_set(&conn, "com.a", "counter", "2").unwrap();
        assert_eq!(
            kv_get(&conn, "com.a", "counter").unwrap().as_deref(),
            Some("2")
        );
        // 隔离：改 com.a 不影响 com.b
        assert_eq!(
            kv_get(&conn, "com.b", "counter").unwrap().as_deref(),
            Some("9")
        );

        assert_eq!(kv_delete(&conn, "com.a", "counter").unwrap(), 1);
        assert_eq!(kv_get(&conn, "com.a", "counter").unwrap(), None);
        // 删不存在的键返回 0
        assert_eq!(kv_delete(&conn, "com.a", "counter").unwrap(), 0);
    }

    /// T2：V1 库升级到最新版本不破坏旧数据，新表可用。
    #[test]
    fn migration_v1_to_v2_preserves_data() {
        let path = temp_db_path("mig");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(crate::SCHEMA_V1).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO tasks (id, title, status, created_at, updated_at)
                 VALUES ('tsk_old', '迁移前任务', 'todo', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }
        let conn = crate::open(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 6);

        // 旧数据完整
        let old = crate::task_repo::get(&conn, "tsk_old").unwrap();
        assert_eq!(old.unwrap().title, "迁移前任务");
        // 新表可写
        kv_set(&conn, "com.a", "k", "v").unwrap();
        assert!(kv_get(&conn, "com.a", "k").unwrap().is_some());
        let _ = std::fs::remove_file(&path);
    }

    /// T3：注册表幂等登记、启停持久化、列表有序。
    #[test]
    fn registry_idempotent_and_persisted() {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate(&conn).unwrap();
        let now = Utc::now();

        ensure_registered(&conn, "com.b", now).unwrap();
        // 幂等：重复登记不新增、不重置状态
        set_enabled(&conn, "com.b", false).unwrap();
        ensure_registered(&conn, "com.b", now).unwrap();

        ensure_registered(&conn, "com.a", now).unwrap();
        let list = list_registrations(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "com.a");
        assert!(list[0].enabled);
        assert!(!list[1].enabled);

        set_enabled(&conn, "com.b", true).unwrap();
        assert!(get_registration(&conn, "com.b").unwrap().unwrap().enabled);
        // 未注册插件更新返回 0 行
        assert_eq!(set_enabled(&conn, "com.none", true).unwrap(), 0);
    }
}
