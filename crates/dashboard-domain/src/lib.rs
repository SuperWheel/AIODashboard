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
/// 与前端 `taskVisual.TASK_COLORS` 同步；后端校验只要求合法 #RRGGBB，不限于预设。
pub const TASK_COLOR_PRESETS: [&str; 15] = [
    "#4A90E2", // 海蓝
    "#38BDF8", // 天青
    "#6366F1", // 靛蓝
    "#8067C8", // 紫罗兰
    "#A855F7", // 亮紫
    "#EC4899", // 玫粉
    "#D96767", // 珊瑚
    "#EF4444", // 朱红
    "#E49345", // 暖橙
    "#F5B942", // 杏黄
    "#69A84F", // 草绿
    "#10B981", // 翠绿
    "#38A38A", // 青绿
    "#92400E", // 咖啡
    "#64748B", // 岩灰
];

/// 循环规则：任务在哪些自然日适用。与卡片样式正交——样式管统计展示口径。
/// 存于目标区间行上（锚点 = 区间起始日），改规则 = 新区间，历史口径不回写。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Recurrence {
    /// 每天适用（默认，也是 V3 存量行为）
    #[default]
    Daily,
    /// 每周指定星期（ISO 1=周一…7=周日）；空集 = 锚点日的星期
    Weekly { weekdays: Vec<u8> },
    /// 每月：日号取锚点日号，短月 clamp 至当月最后一天
    Monthly,
    /// 每年：月日取锚点，2/29 平年 clamp 至 2/28
    Yearly,
    /// 一次性：每日适用直到完成，打卡达标即自动归档（未完成自动顺延）
    Once,
}

impl Recurrence {
    /// 规则是否命中某日。anchor = 区间起始日（YYYY-MM-DD）。
    /// 日期串解析失败时防御性返回 true：脏数据不隐藏任务。
    pub fn matches(&self, anchor: &str, day: &str) -> bool {
        use chrono::{Datelike, NaiveDate};
        let parse = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
        let (Some(a), Some(d)) = (parse(anchor), parse(day)) else {
            return true;
        };
        match self {
            Recurrence::Daily | Recurrence::Once => true,
            Recurrence::Weekly { weekdays } => {
                let ws: Vec<u32> = if weekdays.is_empty() {
                    vec![a.weekday().number_from_monday()]
                } else {
                    weekdays.iter().map(|w| u32::from(*w)).collect()
                };
                ws.contains(&d.weekday().number_from_monday())
            }
            Recurrence::Monthly => {
                let last = last_day_of_month(d.year(), d.month());
                let dom = a.day().min(last);
                d.day() == dom
            }
            Recurrence::Yearly => {
                if d.month() != a.month() {
                    return false;
                }
                let last = last_day_of_month(d.year(), d.month());
                let dom = a.day().min(last);
                d.day() == dom
            }
        }
    }
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    use chrono::{Datelike, NaiveDate};
    // 下月 1 号前一天
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(31)
}

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
    /// 当前生效循环规则（core 在读取时按当日目标区间回填；存储无此列）
    #[serde(default)]
    pub recurrence: Recurrence,
    /// 重要性星级：0=未评级，1–5 星（006；协议 v5）
    #[serde(default)]
    pub priority: i64,
    /// 手动排序分数索引：档内升序，新任务取档末 max+1024（006；协议 v5）
    #[serde(default)]
    pub sort_order: f64,
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
    /// 循环规则（锚点 = start_day）；V3 存量行无此列时反序列化为 daily
    #[serde(default)]
    pub recurrence: Recurrence,
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

/// 重要日类型：纪念日（锚点在过去/今天）/ 倒计时日（锚点在今天/未来）。
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

/// 重要日（纪念日 / 倒计时日）。
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

/// 任务归属区间 [start_day, end_day)：某段时间属于哪个重要日。
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

#[cfg(test)]
mod recurrence_tests {
    use super::Recurrence;

    #[test]
    fn daily_and_once_match_every_day() {
        let anchor = "2026-09-01";
        for day in ["2026-09-01", "2026-09-15", "2027-01-01"] {
            assert!(Recurrence::Daily.matches(anchor, day));
            assert!(Recurrence::Once.matches(anchor, day));
        }
    }

    #[test]
    fn weekly_matches_weekday_set() {
        // 2026-09-01 是周二（ISO 2）；2026-09-02 周三；2026-09-05 周六
        let rec = Recurrence::Weekly {
            weekdays: vec![2, 3],
        };
        assert!(rec.matches("2026-09-01", "2026-09-01")); // 周二
        assert!(rec.matches("2026-09-01", "2026-09-02")); // 周三
        assert!(!rec.matches("2026-09-01", "2026-09-05")); // 周六
        assert!(rec.matches("2026-09-01", "2026-09-08")); // 次周二
    }

    #[test]
    fn weekly_empty_set_falls_back_to_anchor_weekday() {
        let rec = Recurrence::Weekly { weekdays: vec![] };
        assert!(rec.matches("2026-09-02", "2026-09-02")); // 锚点周三
        assert!(!rec.matches("2026-09-02", "2026-09-03")); // 周四
    }

    #[test]
    fn monthly_clamps_to_month_end() {
        let rec = Recurrence::Monthly;
        assert!(rec.matches("2026-01-31", "2026-01-31"));
        assert!(rec.matches("2026-01-31", "2026-02-28")); // 2 月 clamp
        assert!(!rec.matches("2026-01-31", "2026-02-27"));
        assert!(rec.matches("2026-01-31", "2026-04-30"));
        assert!(rec.matches("2024-01-31", "2024-02-29")); // 闰年 2 月
    }

    #[test]
    fn yearly_requires_month_and_clamps_leap_day() {
        let rec = Recurrence::Yearly;
        assert!(rec.matches("2024-02-29", "2024-02-29")); // 闰年命中
        assert!(rec.matches("2024-02-29", "2025-02-28")); // 平年 clamp
        assert!(!rec.matches("2024-02-29", "2025-03-01"));
        assert!(!rec.matches("2024-02-29", "2025-02-27"));
        assert!(rec.matches("2026-09-01", "2027-09-01"));
        assert!(!rec.matches("2026-09-01", "2027-09-02"));
    }

    #[test]
    fn malformed_dates_match_defensively() {
        assert!(Recurrence::Weekly { weekdays: vec![1] }.matches("bad", "2026-09-01"));
        assert!(Recurrence::Monthly.matches("2026-09-01", "bad"));
    }

    #[test]
    fn serde_roundtrip_and_default() {
        let rec = Recurrence::Weekly {
            weekdays: vec![1, 3, 5],
        };
        let s = serde_json::to_string(&rec).unwrap();
        assert_eq!(s, r#"{"kind":"weekly","weekdays":[1,3,5]}"#);
        let back: Recurrence = serde_json::from_str(&s).unwrap();
        assert_eq!(rec, back);
        assert_eq!(
            serde_json::to_string(&Recurrence::Daily).unwrap(),
            r#"{"kind":"daily"}"#
        );
        assert_eq!(Recurrence::default(), Recurrence::Daily);
    }
}
