//! dashboard-domain
//!
//! 纯领域层：实体、枚举与 ID 生成。
//! 不依赖任何基础设施（SQLite / Tauri / CLI）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID 前缀，形如 `tsk_01h...`，便于人类与 AI 快速识别对象类型。
pub mod id_prefix {
    pub const TASK: &str = "tsk";
    pub const PROJECT: &str = "prj";
    pub const NOTE: &str = "not";
    pub const INBOX: &str = "inb";
    pub const ACTIVITY: &str = "act";
}

/// 生成带前缀的 UUIDv7（时间有序）。
pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::now_v7())
}

/// 操作来源。写入 Activity Log，用于区分 USER / CLI / AI / AUTOMATION / SYSTEM 与插件。
/// 序列化为字符串：固定来源用小写单词，插件来源形如 `plugin:<id>`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    User,
    Cli,
    Ai,
    Automation,
    System,
    /// 插件来源，携带插件 id（如 `com.leeyl.pomodoro`）
    Plugin(String),
}

impl Actor {
    pub fn as_str(&self) -> String {
        match self {
            Actor::User => "user".to_string(),
            Actor::Cli => "cli".to_string(),
            Actor::Ai => "ai".to_string(),
            Actor::Automation => "automation".to_string(),
            Actor::System => "system".to_string(),
            Actor::Plugin(id) => format!("plugin:{id}"),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Actor::User),
            "cli" => Some(Actor::Cli),
            "ai" => Some(Actor::Ai),
            "automation" => Some(Actor::Automation),
            "system" => Some(Actor::System),
            _ => s
                .strip_prefix("plugin:")
                .filter(|id| !id.is_empty())
                .map(|id| Actor::Plugin(id.to_string())),
        }
    }
}

impl Serialize for Actor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for Actor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Actor::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid actor: {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::Doing => "doing",
            TaskStatus::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "todo" => Some(TaskStatus::Todo),
            "doing" => Some(TaskStatus::Doing),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InboxStatus {
    Open,
    Processed,
}

impl InboxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InboxStatus::Open => "open",
            InboxStatus::Processed => "processed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    pub id: String,
    pub content: String,
    pub source: String,
    pub status: InboxStatus,
    pub created_at: DateTime<Utc>,
}

/// 操作审计日志条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    pub ts: DateTime<Utc>,
    pub actor: Actor,
    pub action: String,
    pub object_type: String,
    pub object_id: Option<String>,
    pub detail: serde_json::Value,
}

/// 插件注册状态（plugin_registry 表）。插件本体是磁盘上的插件目录，这里只记启停状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistration {
    pub id: String,
    pub enabled: bool,
    pub installed_at: DateTime<Utc>,
}
