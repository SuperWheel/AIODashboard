use dashboard_core::{plugin_package as package, plugin_runtime::PluginRuntime, plugin_service};
use dashboard_domain::Actor;
use dashboard_storage::plugin_repo;
use serde_json::json;
use std::{fs, io::Write, path::Path};

const ID: &str = "com.test.import";

fn source(base: &Path, version: &str) -> std::path::PathBuf {
    let dir = base.join(ID);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("manifest.json"),
        json!({
            "id": ID, "name": "导入测试", "version": version,
            "entry": "main.js", "api_version": "plugin.protocol/v2",
            "permissions": {"core": ["task.read"]}
        })
        .to_string(),
    )
    .unwrap();
    fs::write(dir.join("main.js"), "export function onload() {}").unwrap();
    dir
}

fn zip_source(dir: &Path, archive: &Path) {
    let mut zip = zip::ZipWriter::new(fs::File::create(archive).unwrap());
    for name in ["manifest.json", "main.js"] {
        zip.start_file(
            format!("{ID}/{name}"),
            zip::write::SimpleFileOptions::default().unix_permissions(0o644),
        )
        .unwrap();
        zip.write_all(&fs::read(dir.join(name)).unwrap()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn zip_and_directory_preview_are_read_only_and_equivalent() {
    let tmp = tempfile::tempdir().unwrap();
    let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    let root = tmp.path().join("plugins");
    let dir = source(&tmp.path().join("source"), "1.0.0");
    let archive = tmp.path().join("source.zip");
    zip_source(&dir, &archive);
    let before = fs::read(dir.join("manifest.json")).unwrap();
    let d = package::preview_import(&c, &root, &dir).unwrap();
    let z = package::preview_import(&c, &root, &archive).unwrap();
    assert_eq!(d.check.content_sha256, z.check.content_sha256);
    assert_eq!(d.check.target_stamp, z.check.target_stamp);
    assert_eq!(d.manifest.name, z.manifest.name);
    assert_eq!(d.source, "directory");
    assert_eq!(z.source, "zip");
    assert!(z.check.archive_sha256.is_some());
    assert!(d.check.archive_sha256.is_none());
    assert!(!d.replacing);
    assert!(!d.current_enabled);
    assert!(!root.exists());
    assert!(plugin_repo::list_registrations(&c).unwrap().is_empty());
    assert!(dashboard_core::list_activity(&c, 100, None)
        .unwrap()
        .is_empty());
    assert_eq!(fs::read(dir.join("manifest.json")).unwrap(), before);
}

#[test]
fn checked_import_is_disabled_audited_and_cannot_be_submitted_twice() {
    let tmp = tempfile::tempdir().unwrap();
    let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    let root = tmp.path().join("plugins");
    let dir = source(&tmp.path().join("source"), "1.0.0");
    let preview = package::preview_import(&c, &root, &dir).unwrap();
    let result = package::import_checked(&c, &root, &dir, &preview.check, Actor::User).unwrap();
    assert!(!result.enabled);
    assert_eq!(result.content_sha256, preview.check.content_sha256);
    assert!(!plugin_service::get_registration(&c, ID).unwrap().enabled);
    assert!(plugin_repo::install_metadata(&c, ID)
        .unwrap()
        .unwrap()
        .approved_sha256
        .is_none());
    assert!(PluginRuntime::default().open(&c, &root, ID).is_err());
    let logs = dashboard_core::list_activity(&c, 100, None).unwrap();
    assert!(logs
        .iter()
        .any(|l| l.action == "plugin.install" && matches!(l.actor, Actor::User)));
    assert!(package::import_checked(&c, &root, &dir, &preview.check, Actor::User).is_err());
    assert_eq!(
        dashboard_core::list_activity(&c, 100, None).unwrap().len(),
        logs.len()
    );
    // CLI 仍使用原入口和原 JSON 字段，并记录 CLI 来源。
    let cli = package::install(&c, &root, &dir, None).unwrap();
    assert_eq!(cli.id, result.id);
    assert!(dashboard_core::list_activity(&c, 100, None)
        .unwrap()
        .iter()
        .any(|l| l.action == "plugin.install" && matches!(l.actor, Actor::Cli)));
}

#[test]
fn source_changes_and_concurrent_target_changes_require_new_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    let root = tmp.path().join("plugins");
    let dir = source(&tmp.path().join("source"), "1.0.0");
    package::install(&c, &root, &dir, None).unwrap();
    let old = package::read_bundle(&root.join(ID)).unwrap().hash;
    let preview = package::preview_import(&c, &root, &dir).unwrap();
    fs::write(dir.join("main.js"), "changed source").unwrap();
    let error = package::import_checked(&c, &root, &dir, &preview.check, Actor::User).unwrap_err();
    assert!(error.to_string().contains("来源内容已变化"));
    assert_eq!(package::read_bundle(&root.join(ID)).unwrap().hash, old);

    let preview = package::preview_import(&c, &root, &dir).unwrap();
    plugin_service::set_plugin_enabled(&c, &root, ID, true, Actor::Cli).unwrap();
    assert!(package::import_checked(&c, &root, &dir, &preview.check, Actor::User).is_err());
    assert!(plugin_service::get_registration(&c, ID).unwrap().enabled);

    let preview = package::preview_import(&c, &root, &dir).unwrap();
    fs::write(root.join(ID).join("main.js"), "external target edit").unwrap();
    assert!(package::import_checked(&c, &root, &dir, &preview.check, Actor::User).is_err());
    assert_eq!(
        fs::read_to_string(root.join(ID).join("main.js")).unwrap(),
        "external target edit"
    );
}

#[test]
fn update_requires_review_preserves_backup_and_revokes_old_authorization() {
    let tmp = tempfile::tempdir().unwrap();
    let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    let root = tmp.path().join("plugins");
    let dir = source(&tmp.path().join("source"), "2.0.0");
    package::install(&c, &root, &dir, None).unwrap();
    plugin_service::set_plugin_enabled(&c, &root, ID, true, Actor::User).unwrap();
    let rt = PluginRuntime::default();
    let session = rt.open(&c, &root, ID).unwrap();
    let same = package::preview_import(&c, &root, &dir).unwrap();
    assert!(same.replacing);
    assert!(same.current_enabled);
    assert_eq!(same.current_version.as_deref(), Some("2.0.0"));
    let lower = source(&tmp.path().join("source"), "1.0.0");
    let archive = tmp.path().join("lower.zip");
    zip_source(&lower, &archive);
    let preview = package::preview_import(&c, &root, &archive).unwrap();
    assert!(preview.replacing);
    assert_eq!(preview.manifest.version, "1.0.0");
    assert_eq!(preview.current_version.as_deref(), Some("2.0.0"));
    package::import_checked(&c, &root, &archive, &preview.check, Actor::User).unwrap();
    assert!(rt.load_source(&c, &root, &session.token).is_err());
    assert!(!plugin_service::get_registration(&c, ID).unwrap().enabled);
    let metadata = plugin_repo::install_metadata(&c, ID).unwrap().unwrap();
    assert_eq!(metadata.previous_version.as_deref(), Some("2.0.0"));
    assert_eq!(metadata.source, "zip");
    assert!(metadata.approved_sha256.is_none());
    assert_eq!(package::rollback(&c, &root, ID).unwrap().version, "2.0.0");
}

#[test]
fn invalid_sources_never_change_installed_plugin_and_damaged_manifest_can_be_repaired() {
    let tmp = tempfile::tempdir().unwrap();
    let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    let root = tmp.path().join("plugins");
    let dir = source(&tmp.path().join("source"), "1.0.0");
    package::install(&c, &root, &dir, None).unwrap();
    let before = package::read_bundle(&root.join(ID)).unwrap().hash;
    let bad_zip = tmp.path().join("broken.zip");
    fs::write(&bad_zip, "invalid ZIP").unwrap();
    assert!(package::preview_import(&c, &root, &bad_zip).is_err());
    fs::remove_file(dir.join("main.js")).unwrap();
    assert!(package::preview_import(&c, &root, &dir).is_err());
    source(&tmp.path().join("source"), "1.0.0");
    let manifest = dir.join("manifest.json");
    let current = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        current.replace("plugin.protocol/v2", "plugin.protocol/v1"),
    )
    .unwrap();
    assert!(package::preview_import(&c, &root, &dir).is_err());
    assert_eq!(package::read_bundle(&root.join(ID)).unwrap().hash, before);
    fs::write(&manifest, current).unwrap();
    fs::write(root.join(ID).join("manifest.json"), "broken manifest").unwrap();
    let repair = package::preview_import(&c, &root, &dir).unwrap();
    assert!(repair.replacing);
    assert_eq!(repair.current_version.as_deref(), Some("1.0.0"));
    package::import_checked(&c, &root, &dir, &repair.check, Actor::User).unwrap();
    assert_eq!(package::read_bundle(&root.join(ID)).unwrap().hash, before);
}
