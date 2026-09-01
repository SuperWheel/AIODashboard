//! dashboard-protocol
//!
//! 面向 AI 的稳定输出协议：
//! - 统一 JSON 信封 `{ success, data, error, meta.schema_version }`
//! - 标准 Exit Code（0 成功 / 1 一般错误 / 2 参数错误 / 3 数据不存在 / 5 冲突）
//!
//! ## schema_version 历史
//! - "1"：todo 式任务（due_at / todo·doing·done）
//! - "2"：打卡式任务（task-checkin-cards，变更包 003）——Task 字段重构、
//!   新增 checkin/library 命令组、complete/reopen 语义重映射（deprecated）
//! - "3"：任务循环规则（task-recurrence，变更包 004）——Task 增 recurrence 字段、
//!   task create/update 增 --recurrence/--weekdays、SCHEMA_V4 目标区间加循环列
//! - "4"：年总览改滚动窗口（rolling-year-heatmap，变更包 005）——`task overview --period year`
//!   与 library 热力图从日历年改为滚动 53 周（右端为本周），week_index 以窗口起点为第 0 列，
//!   leading_empty_count 恒 0

use serde::Serialize;

pub const SCHEMA_VERSION: &str = "4";

/// 标准 Exit Code。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    GeneralError = 1,
    UsageError = 2,
    NotFound = 3,
    PermissionDenied = 4,
    Conflict = 5,
}

impl From<ExitCode> for i32 {
    fn from(c: ExitCode) -> i32 {
        c as i32
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub schema_version: &'static str,
}

/// 统一 JSON 信封。机器输出格式不随 UI 调整而变化。
#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorBody>,
    pub meta: Meta,
}

impl<T> Envelope<T>
where
    T: Serialize,
{
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: Meta {
                schema_version: SCHEMA_VERSION,
            },
        }
    }
}

impl Envelope<()> {
    pub fn err(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiErrorBody {
                code,
                message: message.into(),
            }),
            meta: Meta {
                schema_version: SCHEMA_VERSION,
            },
        }
    }
}

impl<T: Serialize> Envelope<T> {
    /// 序列化为 pretty JSON 字符串。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            "{\"success\":false,\"error\":{\"code\":\"internal\",\"message\":\"serialize failed\"}}"
                .to_string()
        })
    }
}
