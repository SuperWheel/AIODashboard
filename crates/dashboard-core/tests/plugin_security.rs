use dashboard_core::{
    plugin_runtime::{PluginCall, PluginRuntime},
    plugin_service as service,
};
use dashboard_domain::Actor;
use serde_json::json;
use std::{fs, path::Path};

fn fixture(root: &Path, id: &str) {
    let dir = root.join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.json"), json!({"id":id,"name":"Test","version":"1.0.0","entry":"main.js","api_version":"plugin.protocol/v2","permissions":{"core":["task.read"],"storage_quota_bytes":12}}).to_string()).unwrap();
    fs::write(dir.join("main.js"), "export function onload() {}").unwrap();
}

#[test]
fn context_permissions_quota_revocation_and_integrity() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("plugins");
    let conn = dashboard_storage::open(&tmp.path().join("db.sqlite")).unwrap();
    fixture(&root, "com.test.a");
    let rt = PluginRuntime::default();
    assert!(rt.open(&conn, &root, "com.test.a").is_err());
    service::set_plugin_enabled(&conn, &root, "com.test.a", true, Actor::User).unwrap();
    let session = rt.open(&conn, &root, "com.test.a").unwrap();
    let call = |v| serde_json::from_value::<PluginCall>(v).unwrap();
    assert!(rt
        .call(
            &conn,
            &root,
            &session.token,
            call(json!({"method":"create_task","title":"unauthorized"}))
        )
        .is_err());
    rt.call(
        &conn,
        &root,
        &session.token,
        call(json!({"method":"kv_set","key":"中","value":"123456789"})),
    )
    .unwrap();
    assert!(rt
        .call(
            &conn,
            &root,
            &session.token,
            call(json!({"method":"kv_set","key":"x","value":"y"}))
        )
        .is_err());
    rt.call(
        &conn,
        &root,
        &session.token,
        call(json!({"method":"kv_set","key":"中","value":"1"})),
    )
    .unwrap();
    assert!(rt
        .call(
            &conn,
            &root,
            "forged",
            call(json!({"method":"kv_get","key":"中"}))
        )
        .is_err());
    service::set_plugin_enabled(&conn, &root, "com.test.a", false, Actor::Cli).unwrap();
    service::set_plugin_enabled(&conn, &root, "com.test.a", true, Actor::Cli).unwrap();
    assert!(
        rt.load_source(&conn, &root, &session.token).is_err(),
        "disable/re-enable must never revive a token"
    );
    let session = rt.open(&conn, &root, "com.test.a").unwrap();
    fs::write(
        root.join("com.test.a/main.js"),
        "export function onload(){return 3}",
    )
    .unwrap();
    assert!(rt.load_source(&conn, &root, &session.token).is_err());
    assert!(
        rt.open(&conn, &root, "com.test.a").is_err(),
        "changed code needs consent again"
    );
    let logs = dashboard_core::list_activity(&conn, 100, None).unwrap();
    assert!(logs.iter().any(|v| v.action == "plugin.denied"));
    assert!(logs
        .iter()
        .any(|v| v.action == "plugin.denied" && v.detail["action"] == "load_source"));
}

#[test]
fn cross_plugin_namespace_and_explicit_matrix() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("plugins");
    let conn = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    fixture(&root, "com.test.a");
    fixture(&root, "com.test.b");
    let rt = PluginRuntime::default();
    for id in ["com.test.a", "com.test.b"] {
        service::set_plugin_enabled(&conn, &root, id, true, Actor::User).unwrap();
    }
    let a = rt.open(&conn, &root, "com.test.a").unwrap();
    let b = rt.open(&conn, &root, "com.test.b").unwrap();
    rt.call(
        &conn,
        &root,
        &a.token,
        PluginCall::KvSet {
            key: "secret".into(),
            value: "A".into(),
        },
    )
    .unwrap();
    assert_eq!(
        rt.call(
            &conn,
            &root,
            &b.token,
            PluginCall::KvGet {
                key: "secret".into()
            }
        )
        .unwrap(),
        serde_json::Value::Null
    );
    assert!(serde_json::from_value::<PluginCall>(
        json!({"method":"kv_get","key":"secret","plugin_id":"com.test.a"})
    )
    .is_err());
    for action in [
        "core.context.read",
        "core.task.write",
        "ui.command:c",
        "event.emit:task.created",
        "event.on:plugin.com.test.a.child:secret",
        "cron:* * * * *",
    ] {
        assert!(
            rt.call(
                &conn,
                &root,
                &a.token,
                PluginCall::Authorize {
                    action: action.into()
                }
            )
            .is_err(),
            "{action}"
        );
    }
    for action in [
        "core.task.read",
        "event.emit:plugin.com.test.a:local",
        "event.on:panel.refresh",
    ] {
        rt.call(
            &conn,
            &root,
            &a.token,
            PluginCall::Authorize {
                action: action.into(),
            },
        )
        .unwrap();
    }
    rt.close(&a.token);
    assert!(rt.load_source(&conn, &root, &a.token).is_err());
}

#[test]
fn minimum_version_and_stale_consent_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("plugins");
    let conn = dashboard_storage::open(&tmp.path().join("db")).unwrap();
    fixture(&root, "com.test.a");
    assert!(service::set_plugin_enabled_checked(
        &conn,
        &root,
        "com.test.a",
        true,
        Actor::User,
        Some("stale")
    )
    .is_err());
    let p = root.join("com.test.a/manifest.json");
    let mut m: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    m["min_host_version"] = json!("99.0.0");
    fs::write(&p, m.to_string()).unwrap();
    assert!(dashboard_core::plugin_manifest::load_from_dir(&root.join("com.test.a")).is_err());
}

#[test]
fn quota_writes_are_atomic_across_connections() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("db");
    let c = dashboard_storage::open(&db).unwrap();
    drop(c);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let db = db.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let c = dashboard_storage::open(&db).unwrap();
                barrier.wait();
                service::kv_set_with_quota(&c, "com.test.a", &format!("k{n}"), "12345678", 10)
                    .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .filter(|ok| *ok)
            .count(),
        1
    );
}
