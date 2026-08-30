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

/// 操作来源。写入 Activity Log，用于区分 USER / CLI / AI / AUTOMATION / SYSTEM。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    User,
    Cli,
    Ai,
    Automation,
    System,
}

impl Actor {
    pub fn as_str(self) -> &'static str {
        match self {
            Actor::User => "user",
            Actor::Cli => "cli",
            Actor::Ai => "ai",
            Actor::Automation => "automation",
            Actor::System => "system",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Actor::User),
            "cli" => Some(Actor::Cli),
            "ai" => Some(Actor::Ai),
            "automation" => Some(Actor::Automation),
            "system" => Some(Actor::System),
            _ => None,
        }
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
