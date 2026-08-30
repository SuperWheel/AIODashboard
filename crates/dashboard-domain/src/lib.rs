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
    pub const COMPLETION: &str = "cmp";
    pub const TARGET_PERIOD: &str = "tgp";
    pub const ACTIVITY_PERIOD: &str = "tvp";
    pub const DATE_LIBRARY: &str = "dlb";
    pub const MEMBERSHIP_PERIOD: &str = "msp";
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
    Active,
    Archived,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Active => "active",
            TaskStatus::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(TaskStatus::Active),
            "archived" => Some(TaskStatus::Archived),
            _ => None,
        }
    }
}

/// 任务卡片样式偏好（每任务持久化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStyle {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl CardStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            CardStyle::Day => "day",
            CardStyle::Week => "week",
            CardStyle::Month => "month",
            CardStyle::Year => "year",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "day" => Some(CardStyle::Day),
            "week" => Some(CardStyle::Week),
            "month" => Some(CardStyle::Month),
            "year" => Some(CardStyle::Year),
            _ => None,
        }
    }
}

/// 任务主题色预设（hex）。前端用 color-mix 做色阶，双主题下不翻转。
pub const TASK_COLOR_PRESETS: [&str; 6] = [
    "#4A90E2", // 海蓝
    "#38A38A", // 青绿
    "#69A84F", // 草绿
    "#E49345", // 暖橙
    "#D96767", // 珊瑚
    "#8067C8", // 紫罗兰
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    /// 图标或 Emoji（取前 2 字符展示）
    #[serde(default)]
    pub icon: String,
    /// 主题色 hex（预设之一）
    #[serde(default = "default_color_hex")]
    pub color_hex: String,
    /// 计数单位（次/杯/页…）
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub card_style: CardStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_color_hex() -> String {
    TASK_COLOR_PRESETS[0].to_string()
}

/// 打卡账本记录（append-only）。value=+1 为增加；负值为减少/撤销补偿。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRecord {
    pub id: String,
    pub task_id: String,
    /// 客户端生成的幂等键（全局唯一）
    pub operation_id: String,
    pub value: i64,
    pub kind: CompletionKind,
    /// 被补偿的正向记录 id（decrement/undo 时填写）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compensates_record_id: Option<String>,
    /// 逻辑日（本地时区 YYYY-MM-DD）
    pub logical_day: String,
    /// 来源（user/cli/ai/plugin:<id>…）
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionKind {
    Add,
    Decrement,
    Undo,
}

impl CompletionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CompletionKind::Add => "add",
            CompletionKind::Decrement => "decrement",
            CompletionKind::Undo => "undo",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "add" => Some(CompletionKind::Add),
            "decrement" => Some(CompletionKind::Decrement),
            "undo" => Some(CompletionKind::Undo),
            _ => None,
        }
    }
}

/// 历史目标区间 [start_day, end_day)，end_day 为空 = 至今有效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTargetPeriod {
    pub id: String,
    pub task_id: String,
    /// 每日目标次数（1–999）
    pub target: i64,
    pub start_day: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_day: Option<String>,
}

/// 任务活动区间 [start_day, end_day)：任务在哪些自然日有效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskActivityPeriod {
    pub id: String,
    pub task_id: String,
    pub start_day: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_day: Option<String>,
}

/// 日期主库类型：纪念日（锚点在过去/今天）/ 倒计时日（锚点在今天/未来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryKind {
    Anniversary,
    Countdown,
}

impl LibraryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LibraryKind::Anniversary => "anniversary",
            LibraryKind::Countdown => "countdown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "anniversary" => Some(LibraryKind::Anniversary),
            "countdown" => Some(LibraryKind::Countdown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryStatus {
    Active,
    Archived,
}

impl LibraryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LibraryStatus::Active => "active",
            LibraryStatus::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(LibraryStatus::Active),
            "archived" => Some(LibraryStatus::Archived),
            _ => None,
        }
    }
}

/// 日期主库（纪念日 / 倒计时日）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateLibrary {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default = "default_color_hex")]
    pub color_hex: String,
    pub kind: LibraryKind,
    /// 锚点日（本地 YYYY-MM-DD）
    pub anchor_day: String,
    #[serde(default)]
    pub sort_order: i64,
    pub status: LibraryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 任务归属区间 [start_day, end_day)：某段时间属于哪个日期主库。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipPeriod {
    pub id: String,
    pub task_id: String,
    pub library_id: String,
    pub start_day: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_day: Option<String>,
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
