//! 桌面导入接口：系统选择器及 Core 参数转换，安装规则由 Core 统一执行。
use super::{conn, R};
use dashboard_core::{plugin_manifest, plugin_package, plugin_runtime::PluginRuntime};
use dashboard_domain::Actor;
use std::path::Path;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceKind {
    Zip,
    Directory,
}

#[tauri::command]
pub async fn plugin_pick_import_source(
    app: tauri::AppHandle,
    kind: ImportSourceKind,
) -> R<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || -> R<Option<String>> {
        let mut picker = app.dialog().file();
        if let Some(window) = app.get_webview_window("main") {
            picker = picker.set_parent(&window);
        }
        let selected = match kind {
            ImportSourceKind::Zip => picker
                .set_title("选择插件 ZIP")
                .add_filter("插件 ZIP", &["zip"])
                .blocking_pick_file(),
            ImportSourceKind::Directory => picker
                .set_title("选择包含 manifest.json 的插件文件夹")
                .blocking_pick_folder(),
        };
        selected
            .map(|file| {
                file.into_path()
                    .map_err(|e| e.to_string())?
                    .into_os_string()
                    .into_string()
                    .map_err(|_| "插件路径必须为 UTF-8".to_string())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn plugin_preview_import(source: String) -> R<plugin_package::ImportPreview> {
    tauri::async_runtime::spawn_blocking(move || {
        let c = conn()?;
        plugin_package::preview_import(&c, &plugin_manifest::plugins_root(), Path::new(&source))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn plugin_import(
    app: tauri::AppHandle,
    source: String,
    check: plugin_package::ImportCheck,
) -> R<plugin_package::InstallResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let c = conn()?;
        let result = plugin_package::import_checked(
            &c,
            &plugin_manifest::plugins_root(),
            Path::new(&source),
            &check,
            Actor::User,
        )
        .map_err(|e| e.to_string())?;
        app.state::<PluginRuntime>().revoke_plugin(&result.id);
        app.state::<super::plugin_cron::CronScheduler>()
            .rescan(&app);
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_ipc_accepts_only_supported_sources_and_complete_checks() {
        assert!(serde_json::from_str::<ImportSourceKind>("\"zip\"").is_ok());
        assert!(serde_json::from_str::<ImportSourceKind>("\"directory\"").is_ok());
        assert!(serde_json::from_str::<ImportSourceKind>("\"url\"").is_err());
        let check = serde_json::json!({
            "id": "com.test.p", "content_sha256": "content",
            "archive_sha256": null, "target_stamp": "target"
        });
        assert!(serde_json::from_value::<plugin_package::ImportCheck>(check.clone()).is_ok());
        let mut forged = check;
        forged["actor"] = serde_json::json!("cli");
        assert!(serde_json::from_value::<plugin_package::ImportCheck>(forged).is_err());
        assert!(
            serde_json::from_value::<plugin_package::ImportCheck>(serde_json::json!({
                "id": "com.test.p", "content_sha256": "content"
            }))
            .is_err()
        );
    }
}
