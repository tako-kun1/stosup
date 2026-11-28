use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db::Database;
use crate::models::{BackupExportResult, BackupImportResult};
use crate::updater::CURRENT_APP_VERSION;

const MAX_AUTO_BACKUP_GENERATIONS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    app: String,
    version: String,
    created_at_ms: i64,
    db_sha256: String,
    media_files: Vec<String>,
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 本日分の自動バックアップがまだ存在しないかをファイルシステムのみで確認（DB ロック不要）
pub fn needs_auto_backup_today(data_dir: &Path) -> Result<bool> {
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;

    let today_prefix = format!("auto-{}-", Utc::now().format("%Y%m%d"));
    let has_today = fs::read_dir(&backups_dir)?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .any(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with(&today_prefix) && s.ends_with(".db"))
                .unwrap_or(false)
        });
    Ok(!has_today)
}

/// 保持世代数を超えた古い自動バックアップファイルを整理する（DB ロック不要）
pub fn prune_old_auto_backups(data_dir: &Path) -> Result<()> {
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(());
    }
    let mut existing: Vec<PathBuf> = fs::read_dir(&backups_dir)?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with("auto-") && s.ends_with(".db"))
                .unwrap_or(false)
        })
        .collect();
    existing.sort();

    if existing.len() > MAX_AUTO_BACKUP_GENERATIONS {
        let remove_count = existing.len() - MAX_AUTO_BACKUP_GENERATIONS;
        for old in existing.into_iter().take(remove_count) {
            let _ = fs::remove_file(old);
        }
    }
    Ok(())
}

/// 自動世代バックアップ用の SQLite スナップショットを 1 件書き出す
pub fn create_auto_backup_snapshot(db: &Database, data_dir: &Path) -> Result<PathBuf> {
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;
    let snap_name = format!("auto-{}.db", Utc::now().format("%Y%m%d-%H%M%S"));
    let snap_path = backups_dir.join(snap_name);
    db.vacuum_into(&snap_path)?;
    Ok(snap_path)
}

fn is_already_compressed_media(name: &str) -> bool {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some("mp4" | "webm" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "mp3" | "ogg" | "m4a" | "aac")
    )
}

/// 一時ディレクトリへ整合性のある DB スナップショットだけを高速に書き出す（DB ロック保持時間を最小化）
pub fn stage_db_snapshot_for_export(db: &Database) -> Result<(PathBuf, PathBuf)> {
    let temp_dir = std::env::temp_dir().join(format!("stosup-exp-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir)?;
    let temp_db = temp_dir.join("stosup.db");
    db.vacuum_into(&temp_db)?;
    Ok((temp_dir, temp_db))
}

/// 退避済み DB スナップショットと media フォルダ一式を DB ロックなしで ZIP 化する
pub fn finalize_backup_zip_from_snapshot(
    temp_dir: &Path,
    temp_db: &Path,
    media_dir: &Path,
    dest_zip_path: &Path,
) -> Result<BackupExportResult> {
    let res = (|| -> Result<BackupExportResult> {
        if let Some(parent) = dest_zip_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let db_hash = sha256_file(temp_db)?;

        let file = File::create(dest_zip_path).with_context(|| {
            format!(
                "バックアップ先ファイルを作成できません: {}",
                dest_zip_path.display()
            )
        })?;
        let mut zip = ZipWriter::new(BufWriter::with_capacity(128 * 1024, file));
        let deflate_opts =
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let store_opts =
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        // 1. stosup.db
        zip.start_file("stosup.db", deflate_opts)?;
        {
            let mut f = File::open(temp_db)?;
            std::io::copy(&mut f, &mut zip)?;
        }

        // 2. media/*（既圧縮の動画・画像・音声は Stored で無圧縮格納し CPU 時間とメモリを大幅削減）
        let mut media_files = Vec::new();
        if media_dir.exists() {
            for entry in fs::read_dir(media_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.contains("..") || name.contains('/') || name.contains('\\') {
                    continue;
                }
                let zip_entry_name = format!("media/{}", name);
                let opts = if is_already_compressed_media(name) {
                    store_opts
                } else {
                    deflate_opts
                };
                zip.start_file(&zip_entry_name, opts)?;
                let mut f = File::open(&path)?;
                std::io::copy(&mut f, &mut zip)?;
                media_files.push(name.to_string());
            }
        }

        // 2b. mods/**（存在すれば導入済み MOD 一式も一緒にバックアップ）
        if let Some(mods_dir) = media_dir.parent().map(|p| p.join("mods")) {
            if mods_dir.is_dir() {
                add_dir_to_backup_zip(&mut zip, &mods_dir, "mods", deflate_opts, store_opts)?;
            }
        }

        // 3. manifest.json
        let created_at_ms = Utc::now().timestamp_millis();
        let manifest = BackupManifest {
            app: "Stosup".to_string(),
            version: CURRENT_APP_VERSION.to_string(),
            created_at_ms,
            db_sha256: db_hash,
            media_files: media_files.clone(),
        };
        zip.start_file("manifest.json", deflate_opts)?;
        let manifest_json = serde_json::to_vec_pretty(&manifest)?;
        zip.write_all(&manifest_json)?;
        let mut inner_writer = zip.finish()?;
        inner_writer.flush()?;

        let size_bytes = fs::metadata(dest_zip_path)?.len();

        Ok(BackupExportResult {
            file_path: dest_zip_path.to_string_lossy().to_string(),
            size_bytes,
            media_file_count: media_files.len() as u32,
            created_at_ms,
        })
    })();

    let _ = fs::remove_dir_all(temp_dir);
    res
}

/// SQLite データベースと media フォルダ一式を 1 つの ZIP ファイルへエクスポートする
pub fn export_backup_zip(
    db: &Database,
    media_dir: &Path,
    dest_zip_path: &Path,
) -> Result<BackupExportResult> {
    let (temp_dir, temp_db) = stage_db_snapshot_for_export(db)?;
    finalize_backup_zip_from_snapshot(&temp_dir, &temp_db, media_dir, dest_zip_path)
}

fn add_dir_to_backup_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    deflate_opts: SimpleFileOptions,
    store_opts: SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name.contains("..") || name.contains('/') || name.contains('\\')
        {
            continue;
        }
        let rel = format!("{}/{}", prefix, name);
        if path.is_dir() {
            add_dir_to_backup_zip(zip, &path, &rel, deflate_opts, store_opts)?;
        } else if path.is_file() {
            let opts = if is_already_compressed_media(name) {
                store_opts
            } else {
                deflate_opts
            };
            zip.start_file(&rel, opts)?;
            let mut f = File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}

/// 検証済みの復元ステージング結果（Drop 時に一時展開ディレクトリを確実に掃除する）
pub struct StagedBackupImport {
    stage_dir: PathBuf,
    staged_db: PathBuf,
    stage_media: PathBuf,
    stage_mods: PathBuf,
    has_mods: bool,
    zip_path: PathBuf,
    media_count: u32,
}

impl Drop for StagedBackupImport {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.stage_dir);
    }
}

const MAX_BACKUP_ENTRIES: usize = 10_000;
const MAX_BACKUP_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2GB（動画複数入りの引っ越しを許容しつつ ZIP ボムを遮断）
const MAX_BACKUP_SINGLE_FILE_BYTES: u64 = 512 * 1024 * 1024; // 単一ファイル上限 512MB
const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;

fn copy_with_limits<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    max_single: u64,
    total_extracted: &mut u64,
    max_total: u64,
) -> Result<u64> {
    let mut buf = [0u8; 64 * 1024];
    let mut file_bytes: u64 = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let n64 = n as u64;
        file_bytes = file_bytes.saturating_add(n64);
        *total_extracted = total_extracted.saturating_add(n64);
        if file_bytes > max_single {
            return Err(anyhow!(
                "バックアップ内のファイルサイズが上限（{} MB）を超えています",
                max_single / (1024 * 1024)
            ));
        }
        if *total_extracted > max_total {
            return Err(anyhow!(
                "バックアップの展開合計サイズが上限（{} MB）を超えています",
                max_total / (1024 * 1024)
            ));
        }
        writer.write_all(&buf[..n])?;
    }
    Ok(file_bytes)
}

/// バックアップ ZIP の展開・チェックサム検証・SQLite 整合性チェックを DB ロックなしで実施する
pub fn stage_and_validate_backup_zip(zip_path: &Path) -> Result<StagedBackupImport> {
    let file = File::open(zip_path)
        .with_context(|| format!("バックアップファイルを開けません: {}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file).context("有効な ZIP アーカイブではありません")?;

    if archive.len() > MAX_BACKUP_ENTRIES {
        return Err(anyhow!(
            "バックアップ内のファイル数が上限（{} 件）を超えています",
            MAX_BACKUP_ENTRIES
        ));
    }

    let stage_dir = std::env::temp_dir().join(format!("stosup-imp-{}", uuid::Uuid::new_v4()));
    let stage_media = stage_dir.join("media");
    let stage_mods = stage_dir.join("mods");
    fs::create_dir_all(&stage_media)?;

    let mut staged = StagedBackupImport {
        staged_db: stage_dir.join("stosup.db"),
        stage_media: stage_media.clone(),
        stage_mods: stage_mods.clone(),
        has_mods: false,
        stage_dir,
        zip_path: zip_path.to_path_buf(),
        media_count: 0,
    };

    let mut manifest_opt: Option<BackupManifest> = None;
    let mut has_db = false;
    let mut media_count: u32 = 0;
    let mut total_extracted: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        if name.contains("..")
            || name.starts_with('/')
            || name.contains(':')
            || name.contains('\0')
        {
            return Err(anyhow!("バックアップ内に不正なパスが含まれています: {}", name));
        }
        if name == "stosup.db" {
            let out_file = File::create(&staged.staged_db)?;
            let mut out = BufWriter::with_capacity(128 * 1024, out_file);
            copy_with_limits(
                &mut entry,
                &mut out,
                MAX_BACKUP_SINGLE_FILE_BYTES,
                &mut total_extracted,
                MAX_BACKUP_TOTAL_BYTES,
            )?;
            out.flush()?;
            has_db = true;
        } else if name == "manifest.json" {
            let mut buf = Vec::new();
            copy_with_limits(
                &mut entry,
                &mut buf,
                MAX_MANIFEST_BYTES,
                &mut total_extracted,
                MAX_BACKUP_TOTAL_BYTES,
            )?;
            let s = String::from_utf8(buf)
                .context("manifest.json が有効な UTF-8 ではありません")?;
            let parsed: BackupManifest = serde_json::from_str(&s)
                .context("manifest.json の形式が不正です")?;
            manifest_opt = Some(parsed);
        } else if let Some(rel) = name.strip_prefix("media/") {
            if entry.is_dir() || rel.is_empty() {
                continue;
            }
            if !crate::media::is_safe_stored_file_name(rel)
                || !crate::mods::is_safe_relative_asset_path(rel)
            {
                return Err(anyhow!(
                    "バックアップ内のメディアファイル名が不正です: {}",
                    rel
                ));
            }
            if crate::mods::is_forbidden_executable_path(rel) {
                return Err(anyhow!(
                    "セキュリティ保護のため、バックアップ内の実行可能ファイルは復元できません: {}",
                    name
                ));
            }
            let out_file = File::create(staged.stage_media.join(rel))?;
            let mut out = BufWriter::with_capacity(128 * 1024, out_file);
            copy_with_limits(
                &mut entry,
                &mut out,
                MAX_BACKUP_SINGLE_FILE_BYTES,
                &mut total_extracted,
                MAX_BACKUP_TOTAL_BYTES,
            )?;
            out.flush()?;
            media_count += 1;
        } else if let Some(rel) = name.strip_prefix("mods/") {
            let rel_clean = rel.trim_end_matches('/');
            if entry.is_dir() || rel_clean.is_empty() {
                continue;
            }
            if !crate::mods::is_safe_relative_asset_path(rel_clean) {
                return Err(anyhow!(
                    "バックアップ内の MOD ファイルパスが不正です: {}",
                    name
                ));
            }
            if crate::mods::is_forbidden_executable_path(rel_clean) {
                return Err(anyhow!(
                    "セキュリティ保護のため、バックアップ内の実行可能ファイルは復元できません: {}",
                    name
                ));
            }
            let out_path = staged.stage_mods.join(rel_clean);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let out_file = File::create(out_path)?;
            let mut out = BufWriter::with_capacity(64 * 1024, out_file);
            copy_with_limits(
                &mut entry,
                &mut out,
                MAX_BACKUP_SINGLE_FILE_BYTES,
                &mut total_extracted,
                MAX_BACKUP_TOTAL_BYTES,
            )?;
            out.flush()?;
            staged.has_mods = true;
        }
    }

    if !has_db {
        return Err(anyhow!("バックアップに stosup.db が含まれていません"));
    }

    if let Some(ref m) = manifest_opt {
        let actual_hash = sha256_file(&staged.staged_db)?;
        if !m.db_sha256.is_empty() && actual_hash != m.db_sha256 {
            return Err(anyhow!("データベースのチェックサムが一致しません（破損している可能性があります）"));
        }
    }

    // SQLite 整合性チェック
    {
        let check_conn = Connection::open(&staged.staged_db)
            .context("バックアップ内の SQLite データベースを開けませんでした")?;
        let integrity: String = check_conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap_or_else(|_| "failed".to_string());
        if integrity.to_lowercase() != "ok" {
            return Err(anyhow!("SQLite 整合性チェックに失敗しました: {}", integrity));
        }
    }

    staged.media_count = media_count;
    Ok(staged)
}

/// 検証済みステージングデータを使って現在の DB を安全退避したうえで差し替える
pub fn apply_staged_backup_zip(
    db: &mut Database,
    db_path: &Path,
    media_dir: &Path,
    staged: StagedBackupImport,
) -> Result<BackupImportResult> {
    // 復元前に現在の状態を退避バックアップ
    let data_dir = db_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;
    let safety_path = backups_dir.join(format!(
        "pre-restore-{}.db",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    db.vacuum_into(&safety_path)?;

    // DB 接続を一時的にインメモリへ逃がしてファイルを安全に差し替え
    db.replace_database_file(db_path, &staged.staged_db)?;

    // media フォルダを復元
    fs::create_dir_all(media_dir)?;
    if let Ok(entries) = fs::read_dir(&staged.stage_media) {
        for entry in entries.flatten() {
            let src = entry.path();
            if src.is_file() {
                if let Some(fname) = src.file_name() {
                    let _ = fs::copy(&src, media_dir.join(fname));
                }
            }
        }
    }

    // mods フォルダを復元（バックアップに mods/ が含まれていた場合）
    if staged.has_mods && staged.stage_mods.is_dir() {
        let target_mods_dir = data_dir.join("mods");
        let _ = crate::mods::copy_dir_recursive(&staged.stage_mods, &target_mods_dir);
    }

    let result = BackupImportResult {
        restored_from: staged.zip_path.to_string_lossy().to_string(),
        safety_backup_path: safety_path.to_string_lossy().to_string(),
        media_file_count: staged.media_count,
        restored_at_ms: Utc::now().timestamp_millis(),
    };
    drop(staged);
    Ok(result)
}

/// バックアップ ZIP を検証し、現在のデータを安全退避したうえで DB と media を復元する
pub fn import_backup_zip(
    db: &mut Database,
    db_path: &Path,
    media_dir: &Path,
    zip_path: &Path,
) -> Result<BackupImportResult> {
    let staged = stage_and_validate_backup_zip(zip_path)?;
    apply_staged_backup_zip(db, db_path, media_dir, staged)
}

/// Windows ネイティブ保存ダイアログでバックアップ保存先を選択する
pub fn pick_backup_save_path() -> Option<String> {
    let default_name = format!(
        "stosup-backup-{}.zip",
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    rfd::FileDialog::new()
        .set_title("Stosup バックアップを保存")
        .set_file_name(&default_name)
        .add_filter("Stosup バックアップ (*.zip)", &["zip"])
        .save_file()
        .map(|p| p.to_string_lossy().to_string())
}

/// Windows ネイティブ開くダイアログでバックアップ ZIP を選択する
pub fn pick_backup_open_path() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("復元する Stosup バックアップを選択")
        .add_filter("Stosup バックアップ (*.zip)", &["zip"])
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

/// オンエアレポート等の保存先選択ダイアログ
pub fn pick_export_file_path(default_name: &str, filter_label: &str, ext: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title("エクスポート先を選択")
        .set_file_name(default_name)
        .add_filter(filter_label, &[ext])
        .save_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppThemeMode;

    #[test]
    fn test_export_and_import_backup_zip_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("stosup.db");
        let media_dir = dir.path().join("media");
        fs::create_dir_all(&media_dir).unwrap();
        fs::write(media_dir.join("sample.png"), b"PNGDATA").unwrap();

        let mut db = Database::open(&db_path).unwrap();
        db.set_theme_mode(AppThemeMode::Light).unwrap();
        db.set_setting("custom_marker", "hello-backup").unwrap();

        let zip_path = dir.path().join("backup.zip");
        let exp = export_backup_zip(&db, &media_dir, &zip_path).unwrap();
        assert_eq!(exp.media_file_count, 1);
        assert!(exp.size_bytes > 0);

        // 変更を加えた後にバックアップから復元
        db.set_theme_mode(AppThemeMode::Dark).unwrap();
        db.set_setting("custom_marker", "modified").unwrap();
        let _ = fs::remove_file(media_dir.join("sample.png"));

        let imp = import_backup_zip(&mut db, &db_path, &media_dir, &zip_path).unwrap();
        assert_eq!(imp.media_file_count, 1);
        assert_eq!(db.get_theme_mode().unwrap(), AppThemeMode::Light);
        assert_eq!(
            db.get_setting("custom_marker").unwrap().as_deref(),
            Some("hello-backup")
        );
        assert!(media_dir.join("sample.png").exists());
    }

    #[test]
    fn test_stage_backup_rejects_drive_prefix_and_executables() {
        let dir = tempfile::tempdir().unwrap();
        // ドライブ相対パスを含む ZIP は拒否されること
        let bad_drive_zip = dir.path().join("bad_drive.zip");
        {
            let f = File::create(&bad_drive_zip).unwrap();
            let mut z = ZipWriter::new(f);
            let opts = SimpleFileOptions::default();
            z.start_file("media/D:evil.dat", opts).unwrap();
            z.write_all(b"evil").unwrap();
            z.finish().unwrap();
        }
        assert!(stage_and_validate_backup_zip(&bad_drive_zip).is_err());

        // mods 内に実行可能ファイルを含む ZIP は拒否されること
        let bad_exe_zip = dir.path().join("bad_exe.zip");
        {
            let f = File::create(&bad_exe_zip).unwrap();
            let mut z = ZipWriter::new(f);
            let opts = SimpleFileOptions::default();
            z.start_file("mods/my-mod/payload.exe", opts).unwrap();
            z.write_all(b"MZ").unwrap();
            z.finish().unwrap();
        }
        assert!(stage_and_validate_backup_zip(&bad_exe_zip).is_err());
    }
}
