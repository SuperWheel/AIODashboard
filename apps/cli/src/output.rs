//! 输出渲染：人类可读表格 / 文本。

/// 截断过长文本。
pub fn trunc(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
