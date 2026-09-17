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

pub fn update_install_metadata(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    source: &str,
    sha256: &str,
    installed_version: &str,
    previous_version: Option<&str>,
) -> CoreResult<()> {
    plugin_repo::update_install_metadata(
        conn,
        plugin_id,
        source,
        sha256,
        installed_version,
        previous_version,
    )?;
    Ok(())
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

/// 启停插件；未注册但磁盘上存在合法 manifest 的插件先登记（v1 无显式 install 步骤）。
pub fn set_plugin_enabled(
    conn: &rusqlite::Connection,
    root: &std::path::Path,
    plugin_id: &str,
    enabled: bool,
    actor: Actor,
) -> CoreResult<PluginRegistration> {
    set_plugin_enabled_checked(conn, root, plugin_id, enabled, actor, None)
}
pub fn set_plugin_enabled_checked(
    conn: &rusqlite::Connection,
    root: &std::path::Path,
    plugin_id: &str,
    enabled: bool,
    actor: Actor,
    expected: Option<&str>,
) -> CoreResult<PluginRegistration> {
    validate_plugin_id(plugin_id)?;
    let _guard = crate::plugin_package::lock(root)?;
    crate::plugin_package::recover(conn, root)?;
    let bundle = if enabled || plugin_repo::get_registration(conn, plugin_id)?.is_none() {
        if !root.join(plugin_id).exists() {
            return Err(CoreError::NotFound(format!("plugin {plugin_id}")));
        }
        Some(crate::plugin_package::read_bundle(&root.join(plugin_id))?)
    } else {
        None
    };
    if enabled {
        let b = bundle
            .as_ref()
            .ok_or_else(|| CoreError::Validation("插件包缺失".into()))?;
        crate::plugin_manifest::require_current(&b.manifest)?;
        if expected.is_some_and(|v| v != b.hash) {
            return Err(CoreError::PermissionDenied(
                "插件内容变化，请重新审阅权限".into(),
            ));
        }
        if plugin_repo::install_metadata(conn, plugin_id)?
            .and_then(|m| m.content_sha256)
            .is_some_and(|h| h != b.hash)
        {
            return Err(CoreError::PermissionDenied(
                "已安装插件内容变化，请重新安装".into(),
            ));
        }
    }
    plugin_repo::immediate(conn, |c| -> CoreResult<()> {
        ensure_registered(c, plugin_id)?;
        if enabled {
            plugin_repo::approve(
                c,
                plugin_id,
                &bundle
                    .as_ref()
                    .ok_or_else(|| CoreError::Validation("缺少插件包".into()))?
                    .hash,
            )?;
        }
        set_enabled(c, plugin_id, enabled, actor)?;
        Ok(())
    })?;
    get_registration(conn, plugin_id)
}

/// CLI / GUI 列表用读模型：磁盘发现 ∪ 注册表。
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    /// None = 磁盘上存在但尚未注册
    pub enabled: Option<bool>,
    pub dir: String,
    pub source: String,
    pub trust_mode: String,
    pub api_version: String,
    pub sha256: Option<String>,
    pub installed_version: Option<String>,
    pub previous_version: Option<String>,
    /// manifest 缺失 / 非法 / 目录名不一致时的错误信息
    pub error: Option<String>,
    pub fingerprint: String,
    pub integrity: String,
    pub legacy: bool,
    pub revision: i64,
    pub pending_approval: bool,
}

/// 列出全部已知插件：磁盘扫描结果 + 仅存在于注册表的残留项。
pub fn list_installed(
    conn: &rusqlite::Connection,
    root: &std::path::Path,
) -> CoreResult<Vec<PluginInfo>> {
    let _guard = crate::plugin_package::lock(root)?;
    crate::plugin_package::recover(conn, root)?;
    let mut out: Vec<PluginInfo> = Vec::new();
    for d in crate::plugin_manifest::scan_plugins_dir(root) {
        let dir_name = d
            .dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        match d.manifest {
            Some(m) => {
                let meta = plugin_repo::install_metadata(conn, &m.id)?;
                let bundle = crate::plugin_package::read_bundle(&d.dir);
                let fingerprint = bundle.as_ref().map(|b| b.hash.clone()).unwrap_or_default();
                let mismatch = meta
                    .as_ref()
                    .and_then(|m| m.content_sha256.as_ref())
                    .is_some_and(|h| h != &fingerprint);
                let approved =
                    meta.as_ref().and_then(|m| m.approved_sha256.as_ref()) == Some(&fingerprint);
                let registered = plugin_repo::get_registration(conn, &m.id)?;
                let revision = meta.as_ref().map_or(0, |m| m.revision);
                let legacy = m.api_version != "plugin.protocol/v2";
                out.push(PluginInfo {
                    enabled: plugin_repo::get_registration(conn, &m.id)?.map(|r| r.enabled),
                    id: m.id.clone(),
                    name: m.name.clone(),
                    version: m.version.clone(),
                    dir: d.dir.display().to_string(),
                    source: meta
                        .as_ref()
                        .map(|v| v.source.clone())
                        .unwrap_or_else(|| "local".into()),
                    trust_mode: "trusted-webview".into(),
                    api_version: if m.api_version.is_empty() {
                        "plugin.protocol/v1".into()
                    } else {
                        m.api_version.clone()
                    },
                    sha256: meta.as_ref().and_then(|v| v.sha256.clone()),
                    installed_version: meta.as_ref().and_then(|v| v.installed_version.clone()),
                    previous_version: meta.as_ref().and_then(|v| v.previous_version.clone()),
                    error: bundle
                        .err()
                        .map(|e| e.to_string())
                        .or_else(|| mismatch.then(|| "完整性校验失败，请重新安装".into())),
                    fingerprint,
                    integrity: if mismatch {
                        "mismatch"
                    } else if meta
                        .as_ref()
                        .and_then(|m| m.content_sha256.as_ref())
                        .is_some()
                    {
                        "verified"
                    } else {
                        "local-unverified"
                    }
                    .into(),
                    legacy,
                    revision,
                    pending_approval: !legacy
                        && !mismatch
                        && (registered.is_none()
                            || registered.is_some_and(|r| r.enabled && !approved)),
                })
            }
            None => out.push(PluginInfo {
                id: dir_name,
                name: "-".into(),
                version: "-".into(),
                enabled: None,
                dir: d.dir.display().to_string(),
                source: "local".into(),
                trust_mode: "unknown".into(),
                api_version: "-".into(),
                sha256: None,
                installed_version: None,
                previous_version: None,
                error: d.error,
                fingerprint: String::new(),
                integrity: "invalid".into(),
                legacy: true,
                revision: 0,
                pending_approval: false,
            }),
        }
    }
    for r in plugin_repo::list_registrations(conn)? {
        if !out.iter().any(|p| p.id == r.id) {
            out.push(PluginInfo {
                id: r.id.clone(),
                name: "(目录缺失)".into(),
                version: "-".into(),
                enabled: Some(r.enabled),
                dir: root.join(&r.id).display().to_string(),
                source: "missing".into(),
                trust_mode: "unknown".into(),
                api_version: "-".into(),
                sha256: None,
                installed_version: None,
                previous_version: None,
                error: Some("插件目录不存在".into()),
                fingerprint: String::new(),
                integrity: "missing".into(),
                legacy: false,
                revision: 0,
                pending_approval: false,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
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

/// quota 检查和写入在同一 SQLite 写事务内完成。
pub fn kv_set_with_quota(
    conn: &rusqlite::Connection,
    id: &str,
    key: &str,
    value: &str,
    quota: u64,
) -> CoreResult<()> {
    validate_kv_key(key)?;
    plugin_repo::immediate(conn, |c| -> CoreResult<()> {
        let current = plugin_repo::kv_usage_bytes(c, id)?;
        let old = plugin_repo::kv_get(c, id, key)?;
        let used = current.saturating_sub(old.map_or(0, |v| key.len() + v.len()))
            + key.len()
            + value.len();
        if used as u64 > quota {
            return Err(CoreError::PermissionDenied("KV quota 超限".into()));
        }
        kv_set(c, id, key, value)
    })
}

pub fn disable_all(
    conn: &rusqlite::Connection,
    root: &std::path::Path,
    actor: Actor,
) -> CoreResult<()> {
    let _guard = crate::plugin_package::lock(root)?;
    crate::plugin_package::recover(conn, root)?;
    for p in plugin_repo::list_registrations(conn)? {
        set_enabled(conn, &p.id, false, actor.clone())?;
    }
    Ok(())
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
        assert!(!ensure_registered(&conn, "com.a").unwrap().enabled);

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
