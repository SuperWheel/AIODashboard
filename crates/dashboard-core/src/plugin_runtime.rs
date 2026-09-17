//! 插件用例入口；身份、权限、审计都在 Core，Tauri 仅传输。
use crate::{
    plugin_manifest::{self, PluginManifest},
    plugin_package, plugin_service, CoreError, CoreResult,
};
use dashboard_domain::Actor;
use dashboard_storage::plugin_repo;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, sync::Mutex};

#[derive(Clone, Serialize)]
pub struct PluginSession {
    pub token: String,
    pub plugin_id: String,
    pub manifest: PluginManifest,
    pub fingerprint: String,
    pub revision: i64,
}
#[derive(Default)]
pub struct PluginRuntime {
    sessions: Mutex<HashMap<String, PluginSession>>,
}
#[derive(Debug, Serialize)]
pub struct PluginFault {
    pub code: String,
    pub message: String,
}
impl From<CoreError> for PluginFault {
    fn from(e: CoreError) -> Self {
        Self {
            code: match &e {
                CoreError::PermissionDenied(_) => "permission_denied",
                CoreError::NotFound(_) => "not_found",
                CoreError::Conflict(_) => "conflict",
                CoreError::Validation(_) => "validation",
                _ => "internal",
            }
            .into(),
            message: e.to_string(),
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginCall {
    Today,
    ListTasks {
        scope: Option<String>,
    },
    Search {
        query: String,
    },
    CreateTask {
        title: String,
        target: Option<i64>,
    },
    CheckinTask {
        id: String,
        operation_id: Option<String>,
    },
    ArchiveTask {
        id: String,
    },
    DeleteTask {
        id: String,
    },
    AddInboxItem {
        content: String,
    },
    CreateNote {
        title: String,
        body: String,
    },
    KvGet {
        key: String,
    },
    KvSet {
        key: String,
        value: String,
    },
    KvDelete {
        key: String,
    },
    KvList {
        prefix: Option<String>,
    },
    Fetch {
        url: String,
    },
    Authorize {
        action: String,
    },
    Denied {
        action: String,
    },
}
impl PluginCall {
    fn action(&self) -> String {
        match self {
            Self::Today => "core.context.read",
            Self::ListTasks { .. } => "core.task.read",
            Self::Search { .. } => "core.search.read",
            Self::CreateTask { .. }
            | Self::CheckinTask { .. }
            | Self::ArchiveTask { .. }
            | Self::DeleteTask { .. } => "core.task.write",
            Self::AddInboxItem { .. } => "core.inbox.write",
            Self::CreateNote { .. } => "core.note.write",
            Self::KvGet { .. }
            | Self::KvSet { .. }
            | Self::KvDelete { .. }
            | Self::KvList { .. } => "storage",
            Self::Fetch { .. } => "network",
            Self::Authorize { action } | Self::Denied { action } => return action.clone(),
        }
        .into()
    }
}
fn permission(message: &str) -> CoreError {
    CoreError::PermissionDenied(message.into())
}
fn audit_denial(c: &Connection, id: Option<&str>, action: &str, message: &str) {
    crate::log_activity(
        c,
        chrono::Utc::now(),
        id.map_or(Actor::System, |id| Actor::Plugin(id.into())),
        "plugin.denied",
        "plugin",
        id,
        &json!({"action":action,"reason":message}),
    );
}
pub fn authorize(m: &PluginManifest, action: &str) -> CoreResult<()> {
    let p = &m.permissions;
    let ok = if let Some(cap) = action.strip_prefix("core.") {
        p.core.iter().any(|v| v == cap)
    } else if action == "storage" {
        p.storage_quota_bytes.is_some()
    } else if action == "network" {
        !p.network.is_empty()
    } else if let Some(rest) = action.strip_prefix("ui.") {
        let (kind, id) = rest.split_once(':').unwrap_or((rest, ""));
        p.ui.iter().any(|v| v == kind)
            && match kind {
                "view" => m.contributions.views.iter().any(|v| v.id == id),
                "today_card" => m.contributions.today_cards.iter().any(|v| v.id == id),
                "command" => m.contributions.commands.iter().any(|v| v.id == id),
                "settings" => m.contributions.settings.iter().any(|v| v.id == id),
                _ => false,
            }
    } else if let Some(expr) = action.strip_prefix("cron:") {
        p.cron.iter().any(|v| v == expr)
    } else if let Some(topic) = action.strip_prefix("event.on:") {
        ["panel.refresh", "panel.show", "panel.hide"].contains(&topic)
            || p.events.iter().any(|v| v == topic)
            || own_topic(&m.id, topic)
    } else if let Some(topic) = action.strip_prefix("event.emit:") {
        own_topic(&m.id, topic)
    } else {
        false
    };
    if ok {
        Ok(())
    } else {
        Err(permission(&format!("未声明能力 {action}")))
    }
}
fn own_topic(id: &str, topic: &str) -> bool {
    topic
        .strip_prefix(&format!("plugin.{id}:"))
        .is_some_and(|s| !s.is_empty() && s.len() <= 100)
}
impl PluginRuntime {
    pub fn open(&self, c: &Connection, root: &Path, id: &str) -> CoreResult<PluginSession> {
        plugin_service::validate_plugin_id(id)?;
        let _guard = plugin_package::lock(root)?;
        plugin_package::recover(c, root)?;
        let result = (|| {
            if !plugin_service::get_registration(c, id)?.enabled {
                return Err(permission("插件已停用"));
            }
            let b = plugin_package::read_bundle(&root.join(id))?;
            plugin_manifest::require_current(&b.manifest)?;
            let meta =
                plugin_repo::install_metadata(c, id)?.ok_or_else(|| permission("插件未注册"))?;
            if meta.approved_sha256.as_ref() != Some(&b.hash)
                || meta.content_sha256.as_ref().is_some_and(|h| h != &b.hash)
            {
                return Err(permission("插件内容变化或尚未确认，请重新安装/确认权限"));
            }
            let s = PluginSession {
                token: uuid::Uuid::new_v4().to_string(),
                plugin_id: id.into(),
                manifest: b.manifest,
                fingerprint: b.hash,
                revision: meta.revision,
            };
            let mut sessions = self.sessions.lock().unwrap_or_else(|p| p.into_inner());
            if sessions.len() >= 128 {
                return Err(CoreError::Validation("插件会话数量超限".into()));
            }
            sessions.insert(s.token.clone(), s.clone());
            Ok(s)
        })();
        if let Err(ref e) = result {
            audit_denial(c, Some(id), "open", &e.to_string());
        }
        result
    }
    pub fn close(&self, token: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(token);
    }
    pub fn revoke_plugin(&self, id: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|_, s| s.plugin_id != id);
    }
    fn session(&self, c: &Connection, root: &Path, token: &str) -> CoreResult<PluginSession> {
        let s = self
            .sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(token)
            .cloned()
            .ok_or_else(|| permission("上下文无效或已撤销"))?;
        let meta = plugin_repo::install_metadata(c, &s.plugin_id)?
            .ok_or_else(|| permission("插件未注册"))?;
        if !plugin_service::get_registration(c, &s.plugin_id)?.enabled
            || meta.revision != s.revision
        {
            return Err(permission("插件会话已过期"));
        }
        let b = plugin_package::read_bundle(&root.join(&s.plugin_id))?;
        if b.hash != s.fingerprint || meta.approved_sha256.as_ref() != Some(&b.hash) {
            return Err(permission("插件内容已变化"));
        }
        Ok(s)
    }
    pub fn load_source(&self, c: &Connection, root: &Path, token: &str) -> CoreResult<String> {
        let _guard = plugin_package::lock(root)?;
        plugin_package::recover(c, root)?;
        let result = (|| {
            let s = self.session(c, root, token)?;
            let b = plugin_package::read_bundle(&root.join(&s.plugin_id))?;
            if b.hash != s.fingerprint {
                return Err(permission("插件内容变化"));
            }
            String::from_utf8(
                b.files
                    .get(&b.manifest.entry)
                    .cloned()
                    .ok_or_else(|| permission("缺少入口"))?,
            )
            .map_err(|_| CoreError::Validation("JS 入口不是 UTF-8".into()))
        })();
        if let Err(ref e) = result {
            let id = self
                .sessions
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .get(token)
                .map(|s| s.plugin_id.clone());
            audit_denial(c, id.as_deref(), "load_source", &e.to_string());
        }
        result
    }

    pub fn call(
        &self,
        c: &Connection,
        root: &Path,
        token: &str,
        call: PluginCall,
    ) -> CoreResult<Value> {
        let action = call.action();
        let guard = plugin_package::lock(root)?;
        plugin_package::recover(c, root)?;
        let s = match self.session(c, root, token) {
            Ok(s) => s,
            Err(e) => {
                let id = self
                    .sessions
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .get(token)
                    .map(|s| s.plugin_id.clone());
                audit_denial(c, id.as_deref(), &action, &e.to_string());
                return Err(e);
            }
        };
        let result = (|| {
            if matches!(&call, PluginCall::Denied { .. }) {
                audit_denial(c, Some(&s.plugin_id), &action, "前端拒绝");
                return Ok(Value::Null);
            }
            authorize(&s.manifest, &action)?;
            let actor = Actor::Plugin(s.plugin_id.clone());
            let id = &s.plugin_id;
            if let PluginCall::Fetch { url } = &call {
                drop(guard);
                let result = crate::plugin_network::fetch(&s.manifest.permissions.network, url);
                let host = crate::plugin_network::parse_url(url)
                    .ok()
                    .and_then(|u| u.host_str().map(str::to_owned));
                crate::log_activity(
                    c,
                    chrono::Utc::now(),
                    actor,
                    "plugin.http_fetch",
                    "plugin",
                    Some(id),
                    &json!({"host":host,"success":result.is_ok(),"error":result.as_ref().err().map(|e|e.to_string())}),
                );
                let _guard = plugin_package::lock(root)?;
                self.session(c, root, token)?; // 不向已撤销会话交付响应。
                return result;
            }
            use PluginCall::*;
            match call {
                Today => serde_json::to_value(crate::context_service::context_today(c)?),
                ListTasks { scope } => {
                    let mut q = dashboard_storage::task_repo::TaskQuery {
                        limit: 500,
                        ..Default::default()
                    };
                    q.status = match scope.as_deref() {
                        Some("active") => Some(dashboard_domain::TaskStatus::Active),
                        Some("archived") => Some(dashboard_domain::TaskStatus::Archived),
                        Some("all") | None => None,
                        _ => return Err(CoreError::Validation("非法任务范围".into())),
                    };
                    serde_json::to_value(crate::task_service::list_tasks(c, &q)?)
                }
                Search { query } => serde_json::to_value(crate::search_service::search(c, &query)?),
                CreateTask { title, target } => {
                    serde_json::to_value(crate::task_service::create_task(
                        c,
                        &crate::task_service::CreateTaskInput {
                            title,
                            daily_target: target.unwrap_or(1),
                            ..Default::default()
                        },
                        actor,
                    )?)
                }
                CheckinTask { id, operation_id } => {
                    serde_json::to_value(crate::checkin_service::record(
                        c,
                        &id,
                        &operation_id.unwrap_or_else(|| dashboard_domain::new_id("op")),
                        actor,
                    )?)
                }
                ArchiveTask { id } => {
                    serde_json::to_value(crate::task_service::archive_task(c, &id, actor)?)
                }
                DeleteTask { id } => {
                    crate::task_service::delete_task(c, &id, false, actor)?;
                    Ok(Value::Null)
                }
                AddInboxItem { content } => serde_json::to_value(crate::inbox_service::add_item(
                    c, &content, "plugin", actor,
                )?),
                CreateNote { title, body } => {
                    serde_json::to_value(crate::note_service::create_note(c, &title, &body, actor)?)
                }
                KvGet { key } => serde_json::to_value(plugin_service::kv_get(c, id, &key)?),
                KvSet { key, value } => {
                    plugin_service::kv_set_with_quota(
                        c,
                        id,
                        &key,
                        &value,
                        s.manifest.permissions.storage_quota_bytes.unwrap_or(0),
                    )?;
                    Ok(Value::Null)
                }
                KvDelete { key } => serde_json::to_value(plugin_service::kv_delete(c, id, &key)?),
                KvList { prefix } => {
                    serde_json::to_value(plugin_service::kv_list(c, id, prefix.as_deref())?)
                }
                Authorize { .. } | Denied { .. } => Ok(Value::Null),
                Fetch { .. } => Ok(Value::Null),
            }
            .map_err(|e| CoreError::Validation(e.to_string()))
        })();
        if let Err(ref e) = result {
            if matches!(e, CoreError::PermissionDenied(_)) {
                audit_denial(c, Some(&s.plugin_id), &action, &e.to_string());
            }
        }
        result
    }
}
