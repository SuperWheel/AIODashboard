//! 插件 cron 调度器（Rust 驱动）。
//!
//! 背景：面板隐藏到托盘后，macOS WebKit 会节流 webview 的 JS 定时器；
//! 因此 cron 精度归 Rust——本模块扫描启用插件的 `permissions.cron` 声明，
//! 到点通过 `plugin-cron` 事件派发给前端桥，再路由给插件的 handler。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use croner::Cron;
use serde_json::json;
use tauri::{AppHandle, Emitter};

use dashboard_core as core;

pub const CRON_EVENT: &str = "plugin-cron";

/// 一个表达式一个 stop 标志；rescan 时全部置位旧循环自行退出。
#[derive(Default)]
pub struct CronScheduler {
    stop_flags: Mutex<Vec<Arc<AtomicBool>>>,
}

fn lock_flags(
    flags: &Mutex<Vec<Arc<AtomicBool>>>,
) -> std::sync::MutexGuard<'_, Vec<Arc<AtomicBool>>> {
    // 锁毒化时恢复内部数据：调度循环 panic 不应永久卡死插件定时
    flags.lock().unwrap_or_else(|p| p.into_inner())
}

impl CronScheduler {
    /// 重新扫描：停掉全部旧循环，为启用插件的每条 cron 表达式各起一个循环。
    pub fn rescan(&self, app: &AppHandle) {
        for f in lock_flags(&self.stop_flags).drain(..) {
            f.store(true, Ordering::Relaxed);
        }

        let root = core::plugin_manifest::plugins_root();
        for d in core::plugin_manifest::scan_plugins_dir(&root) {
            let Some(m) = d.manifest else { continue };
            let enabled = dashboard_storage::open_default()
                .ok()
                .and_then(|c| core::plugin_service::get_registration(&c, &m.id).ok())
                .map(|r| r.enabled)
                .unwrap_or(false);
            if !enabled {
                continue;
            }
            for expr in &m.permissions.cron {
                let stop = Arc::new(AtomicBool::new(false));
                lock_flags(&self.stop_flags).push(stop.clone());
                spawn_loop(app.clone(), m.id.clone(), expr.clone(), stop);
            }
        }
    }

    /// 当前调度的表达式数量（测试 / 状态展示用）。
    pub fn scheduled_count(&self) -> usize {
        lock_flags(&self.stop_flags).len()
    }
}

fn spawn_loop(app: AppHandle, plugin_id: String, expr: String, stop: Arc<AtomicBool>) {
    let cron = match Cron::new(&expr).parse() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[plugin-cron] 插件 {plugin_id} 的 cron '{expr}' 解析失败: {e}");
            return;
        }
    };
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let now = Utc::now();
            let Some(next) = next_occurrence(&cron, now) else {
                return;
            };
            // 500ms 切片睡眠：响应停机标志，同时保持分钟级精度足够
            while !stop.load(Ordering::Relaxed) && Utc::now() < next {
                std::thread::sleep(Duration::from_millis(500));
            }
            if stop.load(Ordering::Relaxed) {
                return;
            }
            let _ = app.emit(CRON_EVENT, json!({ "plugin_id": plugin_id, "expr": expr }));
        }
    });
}

fn next_occurrence(cron: &Cron, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    cron.find_next_occurrence(&after, false).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合法的 5 段 cron 能解析且下一次触发在未来。
    #[test]
    fn parses_five_field_cron() {
        let cron = Cron::new("*/1 * * * *").parse().expect("parse");
        let next = next_occurrence(&cron, Utc::now());
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now() - chrono::Duration::seconds(1));
    }

    #[test]
    fn invalid_cron_fails_parse() {
        assert!(Cron::new("not a cron").parse().is_err());
    }
}
