//! Plugin 用例：注册 / 启停 / 命名空间 KV。
//!
//! 插件是第三类客户端（与 GUI / CLI 平级）：本模块是插件相关业务规则的唯一实现处。
//! 审计策略：插件注册 / 启停写入 activity log；插件发起的领域变更由调用方传
//! `Actor::Plugin(id)`。plugin_kv 是插件私有数据面，读写不进审计（否则计时类插件会刷屏）。

use chrono::Utc;
use dashboard_domain::{Actor, PluginRegistration};
use dashboard_storage::plugin_repo;
use serde::{Deserialize, Serialize};

use crate::{log_activity, snapshot, CoreError, CoreResult};

/// 校验插件 id 形态：反向域名（≥2 段，段内小写字母/数字/连字符，不以连字符开头结尾）。
pub fn validate_plugin_id(id: &str) -> CoreResult<()> {
    let ok = id.len() <= 200
        && id.contains('.')
        && id.split('.').all(|seg| {
            !seg.is_empty()
                && seg.len() <= 63
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !seg.starts_with('-')
                && !seg.ends_with('-')
        });
    if ok {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "无效插件 id '{id}'（需为反向域名，如 com.example.pomodoro）"
        )))
    }
}

fn validate_kv_key(key: &str) -> CoreResult<()> {
    if key.is_empty() || key.len() > 200 || key.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "无效的 KV key（1–200 字符，不含控制字符）".into(),
        ));
    }
    Ok(())
}

const KV_VALUE_MAX: usize = 1_000_000;

/// 面板首次发现插件时登记（幂等）；仅首次登记写审计。
pub fn ensure_registered(
    conn: &rusqlite::Connection,
    plugin_id: &str,
) -> CoreResult<PluginRegistration> {
    validate_plugin_id(plugin_id)?;
    let existing = plugin_repo::get_registration(conn, plugin_id)?;
    if existing.is_none() {
        plugin_repo::ensure_registered(conn, plugin_id, Utc::now())?;
        log_activity(
            conn,
            Utc::now(),
            Actor::System,
            "plugin.registered",
            "plugin",
            Some(plugin_id),
            &serde_json::json!({}),
        );
    }
    get_registration(conn, plugin_id)
}

pub fn list_plugins(conn: &rusqlite::Connection) -> CoreResult<Vec<PluginRegistration>> {
    Ok(plugin_repo::list_registrations(conn)?)
}

pub fn get_registration(
    conn: &rusqlite::Connection,
    plugin_id: &str,
) -> CoreResult<PluginRegistration> {
    plugin_repo::get_registration(conn, plugin_id)?
        .ok_or_else(|| CoreError::NotFound(format!("plugin {plugin_id}")))
}

/// 启用 / 停用插件。属于业务变更：审计 + 快照刷新。
pub fn set_enabled(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    enabled: bool,
    actor: Actor,
) -> CoreResult<PluginRegistration> {
    let n = plugin_repo::set_enabled(conn, plugin_id, enabled)?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("plugin {plugin_id}")));
    }
    log_activity(
        conn,
        Utc::now(),
        actor,
        if enabled {
            "plugin.enable"
        } else {
            "plugin.disable"
        },
        "plugin",
        Some(plugin_id),
        &serde_json::json!({}),
    );
    snapshot::refresh(conn);
    get_registration(conn, plugin_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginKvEntry {
    pub key: String,
    pub value: String,
}

pub fn kv_get(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    key: &str,
) -> CoreResult<Option<String>> {
    validate_kv_key(key)?;
    Ok(plugin_repo::kv_get(conn, plugin_id, key)?)
}

pub fn kv_set(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    key: &str,
    value: &str,
) -> CoreResult<()> {
    validate_kv_key(key)?;
    if value.len() > KV_VALUE_MAX {
        return Err(CoreError::Validation("KV value 超过 1MB 上限".into()));
    }
    Ok(plugin_repo::kv_set(conn, plugin_id, key, value)?)
}

/// 返回是否确有删除（键不存在返回 false）。
pub fn kv_delete(conn: &rusqlite::Connection, plugin_id: &str, key: &str) -> CoreResult<bool> {
    validate_kv_key(key)?;
    Ok(plugin_repo::kv_delete(conn, plugin_id, key)? > 0)
}

/// 列出插件 KV（可按 key 前缀过滤，通配符语义为纯前缀匹配）。
pub fn kv_list(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    key_prefix: Option<&str>,
) -> CoreResult<Vec<PluginKvEntry>> {
    Ok(plugin_repo::kv_list(conn, plugin_id)?
        .into_iter()
        .filter(|(k, _)| key_prefix.is_none_or(|p| k.starts_with(p)))
        .map(|(k, v)| PluginKvEntry { key: k, value: v })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        dashboard_storage::migrate(&conn).unwrap();
        conn
    }

    /// T1（core 层）：KV 经由 service 仍按插件隔离；前缀过滤生效。
    #[test]
    fn kv_isolated_and_prefix_filtered() {
        let conn = mem_conn();
        kv_set(&conn, "com.a", "pomodo/round", "1").unwrap();
        kv_set(&conn, "com.b", "pomodo/round", "9").unwrap();
        kv_set(&conn, "com.a", "other", "x").unwrap();

        let a = kv_list(&conn, "com.a", Some("pomodo/")).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].value, "1");
        let all = kv_list(&conn, "com.a", None).unwrap();
        assert_eq!(all.len(), 2);
        assert!(kv_get(&conn, "com.a", "missing").unwrap().is_none());
        assert!(kv_delete(&conn, "com.b", "pomodo/round").unwrap());
        assert!(kv_list(&conn, "com.b", None).unwrap().is_empty());
    }

    /// T3（core 层）：注册/启停 + NotFound + 非法 id。
    #[test]
    fn registration_and_toggle() {
        let conn = mem_conn();
        ensure_registered(&conn, "com.a").unwrap();
        assert!(ensure_registered(&conn, "com.a").unwrap().enabled);

        let off = set_enabled(&conn, "com.a", false, Actor::Cli).unwrap();
        assert!(!off.enabled);
        let on = set_enabled(&conn, "com.a", true, Actor::Cli).unwrap();
        assert!(on.enabled);

        assert!(matches!(
            set_enabled(&conn, "com.none", true, Actor::Cli),
            Err(CoreError::NotFound(_))
        ));
        assert!(matches!(
            ensure_registered(&conn, "bad_id"),
            Err(CoreError::Validation(_))
        ));
    }
}
