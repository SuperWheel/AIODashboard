//! 受限包格式、完整性、跨进程锁和可恢复的文件/数据库安装事务。
use crate::{
    plugin_manifest::{self, PluginManifest},
    plugin_service, CoreError, CoreResult,
};
use dashboard_domain::Actor;
use dashboard_storage::plugin_repo::{self, PluginInstallMetadata};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};

pub const FILE_LIMIT: u64 = 8_000_000;
pub const TOTAL_LIMIT: u64 = 32_000_000;
pub const ARCHIVE_LIMIT: u64 = 16_000_000;
pub const FILE_COUNT: usize = 1024;
fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Validation(e.to_string())
}
fn sync_dir(path: &Path) -> CoreResult<()> {
    File::open(path).map_err(err)?.sync_all().map_err(err)
}

pub fn regular_file(path: &Path) -> CoreResult<()> {
    let m = fs::symlink_metadata(path).map_err(err)?;
    if !m.file_type().is_file() {
        return Err(err("只允许普通文件，拒绝符号链接和特殊文件"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if m.nlink() > 1 {
            return Err(err("拒绝硬链接"));
        }
    }
    Ok(())
}
pub fn lock(root: &Path) -> CoreResult<File> {
    fs::create_dir_all(root).map_err(err)?;
    let path = root.join(".plugin-lock");
    if path.exists() {
        regular_file(&path)?;
    }
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(err)?;
    f.lock().map_err(err)?;
    Ok(f)
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', ':'])
        && !name.chars().any(|c| c.is_control())
        && !name.starts_with('/')
        && name
            .trim_end_matches('/')
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
        && Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
#[derive(Clone)]
pub struct Bundle {
    pub manifest: PluginManifest,
    pub files: BTreeMap<String, Vec<u8>>,
    pub hash: String,
}
pub fn read_bundle(dir: &Path) -> CoreResult<Bundle> {
    plugin_manifest::load_from_dir(dir)?;
    let mut files = BTreeMap::new();
    collect(dir, dir, &mut files, &mut 0, &mut 0)?;
    let manifest_bytes = files
        .get("manifest.json")
        .ok_or_else(|| err("缺少 manifest"))?;
    if manifest_bytes.len() > 65536 {
        return Err(err("manifest 超过 64KB"));
    }
    let manifest: PluginManifest = serde_json::from_slice(manifest_bytes).map_err(err)?;
    manifest.validate()?;
    if dir.file_name().and_then(|v| v.to_str()) != Some(manifest.id.as_str())
        || !files.contains_key(&manifest.entry)
    {
        return Err(err("manifest 与包内容不一致"));
    }
    let hash = content_hash(&files);
    Ok(Bundle {
        manifest,
        files,
        hash,
    })
}
fn content_hash(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut h = Sha256::new();
    for (name, bytes) in files {
        h.update((name.len() as u64).to_le_bytes());
        h.update(name.as_bytes());
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    format!("{:x}", h.finalize())
}
fn collect(
    base: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    total: &mut u64,
    count: &mut usize,
) -> CoreResult<()> {
    if fs::symlink_metadata(dir)
        .map_err(err)?
        .file_type()
        .is_symlink()
    {
        return Err(err("拒绝目录符号链接"));
    }
    for entry in fs::read_dir(dir).map_err(err)? {
        let entry = entry.map_err(err)?;
        *count += 1;
        if *count > FILE_COUNT {
            return Err(err("包文件/目录数量超限"));
        }
        let path = entry.path();
        if path
            .components()
            .count()
            .saturating_sub(base.components().count())
            > 16
        {
            return Err(err("包目录深度超限"));
        }
        if ["node_modules", ".git"].contains(&entry.file_name().to_string_lossy().as_ref()) {
            continue;
        }
        let name = path
            .strip_prefix(base)
            .map_err(err)?
            .to_str()
            .ok_or_else(|| err("路径必须为 UTF-8"))?
            .to_string();
        if !safe_name(&name) {
            return Err(err("包内路径非法"));
        }
        let meta = fs::symlink_metadata(&path).map_err(err)?;
        if meta.file_type().is_symlink() {
            return Err(err("拒绝符号链接"));
        }
        if meta.is_dir() {
            collect(base, &path, files, total, count)?;
            continue;
        }
        regular_file(&path)?;
        if meta.len() > FILE_LIMIT || files.len() >= FILE_COUNT {
            return Err(err("包文件过大或数量超限"));
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(err)?
            .take(FILE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        *total += bytes.len() as u64;
        if bytes.len() as u64 > FILE_LIMIT || *total > TOTAL_LIMIT {
            return Err(err("包解压大小超限"));
        }
        if files
            .keys()
            .any(|s| s.to_lowercase() == name.to_lowercase())
        {
            return Err(err("包内路径大小写冲突"));
        }
        files.insert(name, bytes);
    }
    Ok(())
}
fn write_bundle(dir: &Path, bundle: &Bundle) -> CoreResult<()> {
    for (name, bytes) in &bundle.files {
        let p = dir.join(name);
        fs::create_dir_all(p.parent().ok_or_else(|| err("无父目录"))?).map_err(err)?;
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&p)
            .map_err(err)?;
        f.write_all(bytes).map_err(err)?;
        f.sync_all().map_err(err)?;
    }
    Ok(())
}
fn read_archive(path: &Path, staging: &Path) -> CoreResult<(Bundle, String)> {
    regular_file(path)?;
    let bytes = fs::read(path).map_err(err)?;
    if bytes.len() as u64 > ARCHIVE_LIMIT {
        return Err(err("ZIP 超过 16MB"));
    }
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(err)?;
    if archive.len() > FILE_COUNT {
        return Err(err("ZIP 文件数量超限"));
    }
    let mut names = HashSet::new();
    let mut roots = HashSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(err)?;
        let name = file.name().to_string();
        if !safe_name(&name)
            || file.encrypted()
            || !names.insert(name.trim_end_matches('/').to_lowercase())
        {
            return Err(err("ZIP 路径非法、重复或加密"));
        }
        let kind = file.unix_mode().unwrap_or(0) & 0o170000;
        if ![0, 0o100000, 0o040000].contains(&kind) {
            return Err(err("ZIP 拒绝链接和特殊文件"));
        }
        // 不解释可携带链接目标的 Unix extra fields。
        let mut extra = file.extra_data().unwrap_or(&[]);
        while extra.len() >= 4 {
            let tag = u16::from_le_bytes([extra[0], extra[1]]);
            let len = u16::from_le_bytes([extra[2], extra[3]]) as usize;
            if [0x000d, 0x756e].contains(&tag) || extra.len() < len + 4 {
                return Err(err("ZIP 不支持 Unix 链接扩展"));
            }
            extra = &extra[len + 4..];
        }
        let root = name.split('/').next().unwrap_or_default();
        plugin_service::validate_plugin_id(root)?;
        roots.insert(root.to_string());
        if roots.len() != 1 {
            return Err(err("ZIP 必须只有一个插件根目录"));
        }
        let out = staging.join(&name);
        if file.is_dir() {
            fs::create_dir_all(out).map_err(err)?;
            continue;
        }
        if !name.contains('/') || file.size() > FILE_LIMIT {
            return Err(err("ZIP 根目录或文件大小非法"));
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(FILE_LIMIT + 1)
            .read_to_end(&mut data)
            .map_err(err)?;
        total += data.len() as u64;
        if data.len() as u64 > FILE_LIMIT || total > TOTAL_LIMIT {
            return Err(err("ZIP 解压大小超限"));
        }
        fs::create_dir_all(out.parent().ok_or_else(|| err("ZIP 父目录非法"))?).map_err(err)?;
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(out)
            .map_err(err)?;
        f.write_all(&data).map_err(err)?;
    }
    let id = roots.into_iter().next().ok_or_else(|| err("空 ZIP"))?;
    Ok((read_bundle(&staging.join(id))?, hash))
}

#[derive(Serialize, Deserialize)]
struct Journal {
    id: String,
    operation: String,
    old_metadata: Option<PluginInstallMetadata>,
    had_old: bool,
}
fn json_file(path: &Path, value: &impl Serialize) -> CoreResult<()> {
    let parent = path.parent().ok_or_else(|| err("缺少目录"))?;
    let mut f = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
    f.write_all(&serde_json::to_vec(value).map_err(err)?)
        .map_err(err)?;
    f.as_file().sync_all().map_err(err)?;
    f.persist(path).map_err(err)?;
    sync_dir(parent)?;
    Ok(())
}
/// 必须持有 root lock；commit 标志在 SQLite 中，崩溃后仅恢复未提交文件。
pub fn recover(conn: &Connection, root: &Path) -> CoreResult<()> {
    let transactions = root.join(".transactions");
    if !transactions.exists() {
        return Ok(());
    }
    for item in fs::read_dir(&transactions).map_err(err)? {
        let tx = item.map_err(err)?.path();
        let jp = tx.join("journal.json");
        if !jp.exists() {
            fs::remove_dir_all(&tx).map_err(err)?;
            continue;
        }
        let j: Journal = serde_json::from_slice(&fs::read(&jp).map_err(err)?).map_err(err)?;
        plugin_service::validate_plugin_id(&j.id)?;
        let target = root.join(&j.id);
        let committed = plugin_repo::install_metadata(conn, &j.id)?
            .is_some_and(|m| m.install_operation.as_deref() == Some(j.operation.as_str()));
        if committed {
            if tx.join("old").exists() {
                let backup = root.join(".backups").join(&j.id);
                if backup.exists() {
                    fs::remove_dir_all(&backup).map_err(err)?;
                }
                fs::create_dir_all(backup.join("payload")).map_err(err)?;
                json_file(&backup.join("metadata.json"), &j.old_metadata)?;
                fs::rename(tx.join("old"), backup.join("payload").join(&j.id)).map_err(err)?;
            }
        } else if tx.join("old").exists() {
            if target.exists() {
                fs::remove_dir_all(&target).map_err(err)?;
            }
            fs::rename(tx.join("old"), &target).map_err(err)?;
        } else if !j.had_old && !tx.join("new").join(&j.id).exists() && target.exists() {
            fs::remove_dir_all(&target).map_err(err)?;
        }
        fs::remove_dir_all(tx).map_err(err)?;
        sync_dir(root)?;
    }
    Ok(())
}
#[derive(Debug, Serialize)]
pub struct InstallResult {
    pub id: String,
    pub version: String,
    pub sha256: Option<String>,
    pub content_sha256: String,
    pub enabled: bool,
}
struct ReplaceOptions<'a> {
    source: &'a str,
    sha256: Option<String>,
    action: &'a str,
    actor: Actor,
    fail: u8,
}
fn replace(
    conn: &Connection,
    root: &Path,
    b: &Bundle,
    options: ReplaceOptions<'_>,
) -> CoreResult<InstallResult> {
    let ReplaceOptions {
        source,
        sha256,
        action,
        actor,
        fail,
    } = options;
    let id = &b.manifest.id;
    let target = root.join(id);
    let op = uuid::Uuid::new_v4().to_string();
    let tx = root.join(".transactions").join(&op);
    fs::create_dir_all(tx.join("new")).map_err(err)?;
    write_bundle(&tx.join("new").join(id), b)?;
    let mut old_meta = plugin_repo::install_metadata(conn, id)?;
    let old_bundle = if target.exists() {
        read_bundle(&target).ok()
    } else {
        None
    };
    let previous = old_bundle
        .as_ref()
        .map(|b| b.manifest.version.clone())
        .or_else(|| old_meta.as_ref().and_then(|m| m.installed_version.clone()));
    if let Some(old) = &old_bundle {
        if old_meta.is_none() {
            old_meta = Some(PluginInstallMetadata {
                source: "local".into(),
                installed_version: Some(old.manifest.version.clone()),
                ..Default::default()
            });
        }
        if let Some(meta) = old_meta.as_mut() {
            if meta.content_sha256.is_none() {
                meta.content_sha256 = Some(old.hash.clone());
            }
        }
    }
    let j = Journal {
        id: id.clone(),
        operation: op.clone(),
        old_metadata: old_meta,
        had_old: target.exists(),
    };
    json_file(&tx.join("journal.json"), &j)?;
    let result: CoreResult<()> = (|| {
        if target.exists() {
            fs::rename(&target, tx.join("old")).map_err(err)?;
        }
        if fail == 1 {
            return Err(err("injected replacement failure"));
        }
        fs::rename(tx.join("new").join(id), &target).map_err(err)?;
        sync_dir(root)?;
        sync_dir(&tx)?;
        if fail == 2 {
            return Err(err("injected metadata failure"));
        }
        plugin_repo::immediate(conn, |c| -> CoreResult<()> {
            plugin_repo::ensure_registered(c, id, chrono::Utc::now())?;
            plugin_repo::save_install(
                c,
                id,
                &PluginInstallMetadata {
                    source: source.into(),
                    sha256: sha256.clone(),
                    installed_version: Some(b.manifest.version.clone()),
                    previous_version: previous,
                    content_sha256: Some(b.hash.clone()),
                    install_operation: Some(op),
                    ..Default::default()
                },
            )?;
            crate::log_activity(
                c,
                chrono::Utc::now(),
                actor,
                action,
                "plugin",
                Some(id),
                &serde_json::json!({"version":b.manifest.version,"content_sha256":b.hash}),
            );
            Ok(())
        })
    })();
    recover(conn, root)?;
    result?;
    crate::snapshot::refresh(conn);
    Ok(InstallResult {
        id: id.clone(),
        version: b.manifest.version.clone(),
        sha256,
        content_sha256: b.hash.clone(),
        enabled: false,
    })
}
pub fn install(
    conn: &Connection,
    root: &Path,
    source: &Path,
    expected: Option<&str>,
) -> CoreResult<InstallResult> {
    let _guard = lock(root)?;
    recover(conn, root)?;
    let (b, sha, kind) = read_source(source, expected)?;
    replace(
        conn,
        root,
        &b,
        ReplaceOptions {
            source: kind,
            sha256: sha,
            action: "plugin.install",
            actor: Actor::Cli,
            fail: 0,
        },
    )
}

// 预检和安装必须读取完全相同的包格式；临时解压不写入插件根目录。
fn read_source(
    source: &Path,
    expected: Option<&str>,
) -> CoreResult<(Bundle, Option<String>, &'static str)> {
    let temp = tempfile::tempdir().map_err(err)?;
    let (b, sha, kind) = if source.is_dir() {
        if expected.is_some() {
            return Err(err("目录导入不接受 archive SHA-256"));
        }
        (read_bundle(source)?, None, "directory")
    } else {
        if fs::metadata(source).map_err(err)?.len() > ARCHIVE_LIMIT {
            return Err(err("ZIP 超过 16MB"));
        }
        let (b, sha) = read_archive(source, temp.path())?;
        if expected.is_some_and(|e| e.to_lowercase() != sha) {
            return Err(err("SHA-256 不匹配"));
        }
        (b, Some(sha), "zip")
    };
    plugin_manifest::require_current(&b.manifest)?;
    Ok((b, sha, kind))
}

/// 桌面导入预检凭据。只约束本次确认的内容，不授予插件运行权限。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCheck {
    pub id: String,
    pub content_sha256: String,
    pub archive_sha256: Option<String>,
    pub target_stamp: String,
}

#[derive(Debug, Serialize)]
pub struct ImportPreview {
    pub manifest: PluginManifest,
    pub source: String,
    pub source_path: String,
    pub replacing: bool,
    pub current_version: Option<String>,
    pub current_enabled: bool,
    pub check: ImportCheck,
}

struct ImportTarget {
    replacing: bool,
    version: Option<String>,
    enabled: bool,
    stamp: String,
}

fn import_target(conn: &Connection, root: &Path, id: &str) -> CoreResult<ImportTarget> {
    let path = root.join(id);
    let registration = plugin_repo::get_registration(conn, id)?;
    let metadata = plugin_repo::install_metadata(conn, id)?;
    let mut files = BTreeMap::new();
    let exists = match fs::symlink_metadata(&path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
            // 即使旧 manifest 损坏，也可预检后替换；链接与特殊文件仍拒绝。
            collect(&path, &path, &mut files, &mut 0, &mut 0)?;
            true
        }
        Ok(_) => return Err(err("现有插件路径不是普通目录，无法替换")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(err(e)),
    };
    let manifest = files
        .get("manifest.json")
        .and_then(|bytes| serde_json::from_slice::<PluginManifest>(bytes).ok());
    let version = manifest
        .map(|m| m.version)
        .or_else(|| metadata.as_ref().and_then(|m| m.installed_version.clone()));
    let state = serde_json::to_vec(&serde_json::json!({
        "exists": exists,
        "content": content_hash(&files),
        "registration": registration,
        "metadata": metadata,
    }))
    .map_err(err)?;
    Ok(ImportTarget {
        replacing: exists || registration.is_some(),
        version,
        enabled: registration.is_some_and(|r| r.enabled),
        stamp: format!("{:x}", Sha256::digest(state)),
    })
}

/// 只读预检：不注册、不恢复事务、不写活动日志、不修改已安装插件。
pub fn preview_import(conn: &Connection, root: &Path, source: &Path) -> CoreResult<ImportPreview> {
    let (b, sha, kind) = read_source(source, None)?;
    let target = import_target(conn, root, &b.manifest.id)?;
    Ok(ImportPreview {
        check: ImportCheck {
            id: b.manifest.id.clone(),
            content_sha256: b.hash,
            archive_sha256: sha,
            target_stamp: target.stamp,
        },
        manifest: b.manifest,
        source: kind.into(),
        source_path: source.display().to_string(),
        replacing: target.replacing,
        current_version: target.version,
        current_enabled: target.enabled,
    })
}

/// GUI 确认后调用；在安装锁内重新检查来源与目标，沿用现有安装事务。
pub fn import_checked(
    conn: &Connection,
    root: &Path,
    source: &Path,
    check: &ImportCheck,
    actor: Actor,
) -> CoreResult<InstallResult> {
    let _guard = lock(root)?;
    recover(conn, root)?;
    let (b, sha, kind) = read_source(source, None)?;
    if b.manifest.id != check.id || b.hash != check.content_sha256 || sha != check.archive_sha256 {
        return Err(err("来源内容已变化，请重新检查插件后再导入"));
    }
    if import_target(conn, root, &b.manifest.id)?.stamp != check.target_stamp {
        return Err(err("已安装插件的版本或状态已变化，请重新检查后再确认替换"));
    }
    replace(
        conn,
        root,
        &b,
        ReplaceOptions {
            source: kind,
            sha256: sha,
            action: "plugin.install",
            actor,
            fail: 0,
        },
    )
}
pub fn rollback(conn: &Connection, root: &Path, id: &str) -> CoreResult<InstallResult> {
    plugin_service::validate_plugin_id(id)?;
    let _guard = lock(root)?;
    recover(conn, root)?;
    let backup = root.join(".backups").join(id);
    if !backup.exists() {
        return Err(CoreError::NotFound(format!("plugin {id} 的备份不存在")));
    }
    let b = read_bundle(&backup.join("payload").join(id))?;
    let meta: Option<PluginInstallMetadata> =
        serde_json::from_slice(&fs::read(backup.join("metadata.json")).map_err(err)?)
            .map_err(err)?;
    if meta
        .as_ref()
        .and_then(|m| m.content_sha256.as_ref())
        .is_some_and(|h| h != &b.hash)
    {
        return Err(err("备份完整性校验失败"));
    }
    plugin_manifest::require_current(&b.manifest)?;
    replace(
        conn,
        root,
        &b,
        ReplaceOptions {
            source: meta.as_ref().map_or("local", |m| m.source.as_str()),
            sha256: meta.as_ref().and_then(|m| m.sha256.clone()),
            action: "plugin.rollback",
            actor: Actor::Cli,
            fail: 0,
        },
    )
}
pub fn pack(
    conn: &Connection,
    root: &Path,
    id: &str,
    output: &Path,
) -> CoreResult<serde_json::Value> {
    plugin_service::validate_plugin_id(id)?;
    let _guard = lock(root)?;
    recover(conn, root)?;
    let b = read_bundle(&root.join(id))?;
    plugin_manifest::require_current(&b.manifest)?;
    let out = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir().map_err(err)?.join(output)
    };
    let parent = out.parent().ok_or_else(|| err("输出路径无父目录"))?;
    fs::create_dir_all(parent).map_err(err)?;
    if parent
        .canonicalize()
        .map_err(err)?
        .starts_with(root.join(id).canonicalize().map_err(err)?)
    {
        return Err(err("ZIP 输出不能在插件包内"));
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
    {
        let mut zip = zip::ZipWriter::new(temp.as_file_mut());
        for (name, bytes) in &b.files {
            zip.start_file(
                format!("{id}/{name}"),
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(0o644),
            )
            .map_err(err)?;
            zip.write_all(bytes).map_err(err)?;
        }
        zip.finish().map_err(err)?;
    }
    if temp.as_file().metadata().map_err(err)?.len() > ARCHIVE_LIMIT {
        return Err(err("ZIP 超过 16MB"));
    }
    let sha = format!("{:x}", Sha256::digest(fs::read(temp.path()).map_err(err)?));
    temp.persist(&out).map_err(err)?;
    crate::log_activity(
        conn,
        chrono::Utc::now(),
        Actor::Cli,
        "plugin.pack",
        "plugin",
        Some(id),
        &serde_json::json!({"sha256":sha}),
    );
    crate::snapshot::refresh(conn);
    Ok(
        serde_json::json!({"id":id,"version":b.manifest.version,"archive":out,"sha256":sha,"content_sha256":b.hash}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_failure_restores_old_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("plugins");
        fs::create_dir_all(root.join("com.test.p")).unwrap();
        let c = dashboard_storage::open(&tmp.path().join("db")).unwrap();
        fs::write(root.join("com.test.p/manifest.json"),r#"{"id":"com.test.p","name":"P","version":"1.0.0","entry":"main.js","api_version":"plugin.protocol/v2"}"#).unwrap();
        fs::write(root.join("com.test.p/main.js"), "old").unwrap();
        let b = read_bundle(&root.join("com.test.p")).unwrap();
        for fail in [1, 2] {
            assert!(replace(
                &c,
                &root,
                &b,
                ReplaceOptions {
                    source: "zip",
                    sha256: None,
                    action: "plugin.install",
                    actor: Actor::Cli,
                    fail,
                },
            )
            .is_err());
            assert_eq!(
                fs::read_to_string(root.join("com.test.p/main.js")).unwrap(),
                "old"
            );
        }
    }
    #[test]
    fn invalid_paths() {
        for s in ["../a", "/a", "a/../b", "a\\b", "C:/a", "a//b", "a/./b"] {
            assert!(!safe_name(s), "{s}");
        }
    }
}
