//! 输出渲染：人类可读表格 / 文本。

use chrono::{DateTime, Local, Utc};

/// 截断过长文本。
pub fn trunc(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

pub fn fmt_due(due: Option<DateTime<Utc>>) -> String {
    match due {
        None => "-".to_string(),
        Some(dt) => {
            let local: DateTime<Local> = dt.with_timezone(&Local);
            let base = local.format("%Y-%m-%d %H:%M").to_string();
            if dt < Utc::now() {
                format!("{base} [逾期]")
            } else {
                base
            }
        }
    }
}
