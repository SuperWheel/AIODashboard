//! 插件 manifest（manifest.json）：结构定义、校验、磁盘扫描与网络白名单规则。
//!
//! 这是插件契约的 Rust 侧单一事实来源；前端 loader 有对应的 TS 镜像实现，
//! 两边规则由测试对齐（T6/T7）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

/// 领域事件白名单：manifest 中 `permissions.events` 只允许订阅这些。
pub const KNOWN_EVENTS: &[&str] = &[
    "task.completed",
    "task.created",
    "inbox.added",
    "note.created",
];

/// 插件清单。未知字段忽略（向前兼容）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    /// 入口 JS 文件名（相对插件目录，不允许路径分隔符）
    pub entry: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub contributions: PluginContributions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// 允许访问的 host 白名单（精确域名，如 "api.github.com"）
    #[serde(default)]
    pub network: Vec<String>,
    /// 订阅的领域事件（见 KNOWN_EVENTS）
    #[serde(default)]
    pub events: Vec<String>,
    /// cron 表达式（5 段），由 Rust 侧调度驱动
    #[serde(default)]
    pub cron: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginContributions {
    #[serde(default)]
    pub views: Vec<PluginViewContribution>,
    #[serde(default)]
    pub today_cards: Vec<PluginCardContribution>,
    #[serde(default)]
    pub commands: Vec<PluginCommandContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginViewContribution {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCardContribution {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandContribution {
    pub id: String,
    pub title: String,
}

impl PluginManifest {
    pub fn validate(&self) -> CoreResult<()> {
        crate::plugin_service::validate_plugin_id(&self.id)?;
        if self.name.trim().is_empty() || self.name.chars().count() > 100 {
            return Err(CoreError::Validation(
                "插件 name 不能为空且 ≤100 字符".into(),
            ));
        }
        if self.version.trim().is_empty() || self.version.len() > 32 {
            return Err(CoreError::Validation(
                "插件 version 不能为空且 ≤32 字符".into(),
            ));
        }
        let entry = &self.entry;
        if entry.is_empty()
            || !entry.ends_with(".js")
            || entry.contains('/')
            || entry.contains('\\')
            || entry.contains("..")
        {
            return Err(CoreError::Validation(format!(
                "无效 entry '{entry}'（应为插件目录内的单个 .js 文件名）"
            )));
        }
        for h in &self.permissions.network {
            validate_host(h)?;
        }
        for ev in &self.permissions.events {
            if !KNOWN_EVENTS.contains(&ev.as_str()) {
                return Err(CoreError::Validation(format!(
                    "未知事件 '{ev}'（支持 {}）",
                    KNOWN_EVENTS.join(", ")
                )));
            }
        }
        for c in &self.permissions.cron {
            if c.split_whitespace().count() != 5 {
                return Err(CoreError::Validation(format!(
                    "无效 cron '{c}'（需 5 段表达式，如 */1 * * * *）"
                )));
            }
        }
        check_ids_unique(
            "view",
            self.contributions.views.iter().map(|v| (&v.id, &v.title)),
        )?;
        check_ids_unique(
            "today_card",
            self.contributions
                .today_cards
                .iter()
                .map(|c| (&c.id, &c.id)),
        )?;
        check_ids_unique(
            "command",
            self.contributions
                .commands
                .iter()
                .map(|c| (&c.id, &c.title)),
        )?;
        Ok(())
    }
}

fn check_ids_unique<'a, I: Iterator<Item = (&'a String, &'a String)>>(
    kind: &str,
    items: I,
) -> CoreResult<()> {
    let mut seen = std::collections::HashSet::new();
    for (id, title) in items {
        if id.is_empty() {
            return Err(CoreError::Validation(format!("{kind} 贡献点 id 不能为空")));
        }
        if title.trim().is_empty() {
            return Err(CoreError::Validation(format!(
                "{kind} '{id}' 的标题不能为空"
            )));
        }
        if !seen.insert(id.as_str()) {
            return Err(CoreError::Validation(format!("{kind} id '{id}' 重复")));
        }
    }
    Ok(())
}

/// 校验 host 形态：小写域名（≥2 段），不含 scheme / 路径 / 端口 / 通配符。
pub fn validate_host(host: &str) -> CoreResult<()> {
    let ok = host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        });
    if ok {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "无效 host '{host}'（应为小写域名，如 api.github.com）"
        )))
    }
}

/// 从 URL 提取小写 host（仅支持 http/https，端口会被剥掉）。
pub fn extract_host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#', ':']).unwrap_or(rest.len());
    let host = &rest[..end];
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// 网络白名单判定（T5 的纯逻辑半边，代理命令在接口层）。
pub fn check_network_allowed(allowlist: &[String], url: &str) -> CoreResult<()> {
    let host = extract_host(url)
        .ok_or_else(|| CoreError::Validation(format!("非法 URL '{url}'（仅支持 http/https）")))?;
    if allowlist.iter().any(|h| h == &host) {
        Ok(())
    } else {
        Err(CoreError::Validation(format!(
            "network 权限未包含 host '{host}'"
        )))
    }
}

/// 插件根目录：`DASHBOARD_PLUGINS_DIR` 或数据库同目录下 `plugins/`。
pub fn plugins_root() -> PathBuf {
    if let Ok(p) = std::env::var("DASHBOARD_PLUGINS_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    dashboard_storage::default_db_path()
        .parent()
        .map(|p| p.join("plugins"))
        .unwrap_or_else(|| PathBuf::from("plugins"))
}

/// 读取并校验一个插件目录（manifest 合法且 entry 文件存在）。
pub fn load_from_dir(dir: &Path) -> CoreResult<PluginManifest> {
    let path = dir.join("manifest.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| CoreError::Validation(format!("读取 {} 失败: {e}", path.display())))?;
    let m: PluginManifest = serde_json::from_str(&raw).map_err(|e| {
        CoreError::Validation(format!("{} 不是合法的插件清单: {e}", path.display()))
    })?;
    m.validate()?;
    if !dir.join(&m.entry).is_file() {
        return Err(CoreError::Validation(format!(
            "插件 {} 的入口文件 '{}' 不存在",
            m.id, m.entry
        )));
    }
    Ok(m)
}

/// 磁盘扫描结果：manifest 非法的目录以 error 形式呈现，不中断整体扫描。
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub dir: PathBuf,
    pub manifest: Option<PluginManifest>,
    pub error: Option<String>,
}

/// 扫描插件根目录（一级子目录，含 manifest.json 的视为插件）。
/// 约定：目录名必须等于 manifest.id，否则视为非法。
pub fn scan_plugins_dir(root: &Path) -> Vec<DiscoveredPlugin> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for e in entries.flatten() {
        let dir = e.path();
        if !dir.is_dir() {
            continue;
        }
        match load_from_dir(&dir) {
            Ok(m) => {
                let name_ok = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n == m.id);
                if name_ok {
                    out.push(DiscoveredPlugin {
                        dir,
                        manifest: Some(m),
                        error: None,
                    });
                } else {
                    out.push(DiscoveredPlugin {
                        error: Some("目录名与 manifest.id 不一致".into()),
                        dir,
                        manifest: None,
                    });
                }
            }
            Err(e) => out.push(DiscoveredPlugin {
                dir,
                manifest: None,
                error: Some(e.to_string()),
            }),
        }
    }
    out.sort_by(|a, b| a.dir.cmp(&b.dir));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_plugin(dir: &Path, dir_name: &str, manifest: &str, entry: Option<&str>) -> PathBuf {
        let p = dir.join(dir_name);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("manifest.json"), manifest).unwrap();
        if let Some(js) = entry {
            std::fs::write(p.join("main.js"), js).unwrap();
        }
        p
    }

    fn valid_manifest(id: &str) -> String {
        format!(r#"{{"id":"{id}","name":"Echo","version":"0.1.0","entry":"main.js"}}"#)
    }

    /// T6：最小合法 manifest 通过，默认权限/贡献点为空。
    #[test]
    fn manifest_validate_accepts_minimal() {
        let m: PluginManifest = serde_json::from_str(&valid_manifest("com.test.echo")).unwrap();
        m.validate().unwrap();
        assert!(m.permissions.network.is_empty());
        assert!(m.contributions.views.is_empty());
    }

    /// T6：非法字段逐一拒绝。
    #[test]
    fn manifest_validate_rejects_bad_fields() {
        for id in ["no_dot", "-a.b.c", "a..b", "com.X.y", ""] {
            let m: PluginManifest = serde_json::from_str(&valid_manifest(id)).unwrap();
            assert!(m.validate().is_err(), "id '{id}' 应被拒绝");
        }
        for entry in ["sub/main.js", "main.txt", "../main.js", ""] {
            let json =
                format!(r#"{{"id":"com.ok.id","name":"N","version":"0.1.0","entry":"{entry}"}}"#);
            let m: PluginManifest = serde_json::from_str(&json).unwrap();
            assert!(m.validate().is_err(), "entry '{entry}' 应被拒绝");
        }
        // 非法 permissions（构造合法外层 + 非法 permissions）
        let mk = |perm: &str| {
            format!(
                r#"{{"id":"com.ok.id","name":"N","version":"0.1.0","entry":"main.js","permissions":{perm}}}"#
            )
        };
        for perm in [
            r#"{"network":["https://api.example.com"]}"#,
            r#"{"network":["api.example.com:8443"]}"#,
            r#"{"events":["task.deleted"]}"#,
            r#"{"cron":["* * * *"]}"#,
        ] {
            let m: PluginManifest = serde_json::from_str(&mk(perm)).unwrap();
            assert!(m.validate().is_err(), "permissions {perm} 应被拒绝");
        }
    }

    /// T6：load_from_dir 要求 entry 文件真实存在。
    #[test]
    fn load_from_dir_checks_entry_exists() {
        let dir = TempDir::new().unwrap();
        let p = write_plugin(
            dir.path(),
            "com.test.echo",
            &valid_manifest("com.test.echo"),
            None,
        );
        assert!(load_from_dir(&p).is_err());
        std::fs::write(p.join("main.js"), "// ok").unwrap();
        assert!(load_from_dir(&p).is_ok());
    }

    /// T6：扫描不因个别非法插件中断；目录名必须与 id 一致。
    #[test]
    fn scan_reports_errors_but_continues() {
        let dir = TempDir::new().unwrap();
        write_plugin(
            dir.path(),
            "com.test.echo",
            &valid_manifest("com.test.echo"),
            Some("// ok"),
        );
        write_plugin(dir.path(), "com.test.bad", "{{broken", Some("// ok"));
        write_plugin(
            dir.path(),
            "com.test.mismatch",
            &valid_manifest("com.test.other"),
            Some("// ok"),
        );
        std::fs::write(dir.path().join("notes.txt"), "not a plugin").unwrap();

        let found = scan_plugins_dir(dir.path());
        assert_eq!(found.len(), 3);
        let ok = found
            .iter()
            .find(|d| d.dir.ends_with("com.test.echo"))
            .unwrap();
        assert_eq!(ok.manifest.as_ref().unwrap().id, "com.test.echo");
        assert!(found.iter().filter(|d| d.error.is_some()).count() == 2);
    }

    /// T5（纯逻辑半边）：网络白名单判定。
    #[test]
    fn network_allowlist_check() {
        let allow = vec!["api.github.com".to_string()];
        assert!(check_network_allowed(&allow, "https://api.github.com/repos/x").is_ok());
        assert!(check_network_allowed(&allow, "https://API.GitHub.com/x").is_ok());
        assert!(check_network_allowed(&allow, "https://evil.com/x").is_err());
        assert!(check_network_allowed(&allow, "ftp://api.github.com").is_err());
        assert!(check_network_allowed(&allow, "not a url").is_err());
    }
}
