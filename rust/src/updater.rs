use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::db::Database;
use crate::models::{ReleaseNotesInfo, UpdateDownloadProgress, UpdateInfo};

pub const GITHUB_REPO_OWNER: &str = "tako-kun1";
pub const GITHUB_REPO_NAME: &str = "stosup";
pub const CURRENT_APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// アプリ本体に同梱された CHANGELOG.md（オフライン時のフォールバックに使用）
const BUNDLED_CHANGELOG: &str = include_str!("../../CHANGELOG.md");

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn github_client(timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(format!("Stosup/{}", CURRENT_APP_VERSION))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("HTTPクライアントの作成に失敗しました")
}

/// セマンティックバージョンを比較して latest > current か判定する
pub fn is_newer_version(current: &str, latest_tag: &str) -> bool {
    let c = current.trim().trim_start_matches('v');
    let l = latest_tag.trim().trim_start_matches('v');
    match (Version::parse(c), Version::parse(l)) {
        (Ok(cv), Ok(lv)) => lv > cv,
        _ => false,
    }
}

/// GitHub Releases の最新リリースを確認する
pub async fn check_for_updates() -> Result<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME
    );
    let client = github_client(15)?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("GitHub Releases API への接続に失敗しました")?;

    // リリースがまだ存在しない場合（初回公開前）
    if resp.status().as_u16() == 404 {
        return Ok(UpdateInfo {
            current_version: CURRENT_APP_VERSION.to_string(),
            latest_version: CURRENT_APP_VERSION.to_string(),
            has_update: false,
            release_name: format!("v{}", CURRENT_APP_VERSION),
            release_notes: extract_changelog_section(BUNDLED_CHANGELOG, CURRENT_APP_VERSION),
            published_at: String::new(),
            installer_url: None,
            installer_name: None,
            checksum_url: None,
            html_url: format!(
                "https://github.com/{}/{}/releases",
                GITHUB_REPO_OWNER, GITHUB_REPO_NAME
            ),
        });
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "アップデート確認に失敗しました (HTTP {}): {}",
            status,
            body
        ));
    }

    let release: GitHubRelease = resp.json().await?;
    let latest_clean = release.tag_name.trim_start_matches('v').to_string();
    let has_update = is_newer_version(CURRENT_APP_VERSION, &release.tag_name);

    let installer_asset = release
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".msi"))
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase().ends_with(".exe") && a.name.contains("Setup"))
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase().ends_with(".exe"))
        });

    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name.to_uppercase().contains("SHA256"));

    Ok(UpdateInfo {
        current_version: CURRENT_APP_VERSION.to_string(),
        latest_version: latest_clean,
        has_update,
        release_name: release.name.unwrap_or(release.tag_name),
        release_notes: release.body.unwrap_or_default(),
        published_at: release.published_at.unwrap_or_default(),
        installer_url: installer_asset.map(|a| a.browser_download_url.clone()),
        installer_name: installer_asset.map(|a| a.name.clone()),
        checksum_url: checksum_asset.map(|a| a.browser_download_url.clone()),
        html_url: release.html_url,
    })
}

/// インストーラーをダウンロードし、チェックサムがある場合は SHA-256 を検証する
pub async fn download_installer(
    installer_url: &str,
    installer_name: &str,
    checksum_url: Option<&str>,
    mut on_progress: impl FnMut(UpdateDownloadProgress),
) -> Result<String> {
    let temp_dir = std::env::temp_dir().join("StosupUpdate");
    tokio::fs::create_dir_all(&temp_dir).await?;
    let safe_name = PathBuf::from(installer_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Stosup-x64.msi")
        .to_string();
    let dest_path = temp_dir.join(&safe_name);

    let client = github_client(300)?;
    let resp = client
        .get(installer_url)
        .send()
        .await
        .context("インストーラーのダウンロード開始に失敗しました")?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "インストーラーのダウンロードに失敗しました (HTTP {})",
            resp.status()
        ));
    }

    let total_bytes = resp.content_length();
    let mut file = tokio::fs::File::create(&dest_path).await?;
    let mut stream = resp.bytes_stream();
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;

    on_progress(UpdateDownloadProgress {
        downloaded_bytes: 0,
        total_bytes,
        status: "downloading".to_string(),
        file_path: None,
        error: None,
    });

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("ダウンロード中に通信が中断されました")?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        on_progress(UpdateDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes,
            status: "downloading".to_string(),
            file_path: None,
            error: None,
        });
    }
    file.flush().await?;
    drop(file);

    // SHA-256 チェックサム検証
    if let Some(cs_url) = checksum_url {
        on_progress(UpdateDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes,
            status: "verifying".to_string(),
            file_path: None,
            error: None,
        });

        let actual_hash = format!("{:x}", hasher.finalize());
        if let Ok(cs_resp) = client.get(cs_url).send().await {
            if cs_resp.status().is_success() {
                if let Ok(cs_text) = cs_resp.text().await {
                    if let Some(expected_hash) = parse_sha256_for_file(&cs_text, &safe_name) {
                        if !actual_hash.eq_ignore_ascii_case(&expected_hash) {
                            let _ = tokio::fs::remove_file(&dest_path).await;
                            return Err(anyhow!(
                                "インストーラーの SHA-256 検証に失敗しました（ファイルが破損している可能性があります）"
                            ));
                        }
                    }
                }
            }
        }
    }

    let path_str = dest_path.to_string_lossy().to_string();
    on_progress(UpdateDownloadProgress {
        downloaded_bytes: downloaded,
        total_bytes,
        status: "ready".to_string(),
        file_path: Some(path_str.clone()),
        error: None,
    });

    Ok(path_str)
}

/// ダウンロード済みの Windows インストーラー（.msi / .exe）を起動する
pub fn launch_installer(installer_path: &str, silent: bool) -> Result<()> {
    let path = PathBuf::from(installer_path);
    if !path.exists() {
        return Err(anyhow!(
            "インストーラーファイルが見つかりません: {}",
            installer_path
        ));
    }

    let is_msi = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("msi"))
        .unwrap_or(false);

    if is_msi {
        if silent {
            // サイレント MSI 更新の完了後に Stosup を --after-update 付きで自動再起動するラッパーを生成
            let script_dir = path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(std::env::temp_dir);
            let script_path = script_dir.join("apply_msi_update.cmd");
            let msi_str = path.to_string_lossy().replace('"', "");
            let exe_after = std::env::current_exe()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| r"%LOCALAPPDATA%\Stosup\stosup.exe".to_string());

            let script = format!(
                "@echo off\r\n\
                 timeout /t 2 /nobreak >nul\r\n\
                 msiexec /i \"{msi}\" /qn /norestart\r\n\
                 if exist \"%LOCALAPPDATA%\\Stosup\\stosup.exe\" (\r\n\
                   start \"\" \"%LOCALAPPDATA%\\Stosup\\stosup.exe\" --after-update\r\n\
                 ) else if exist \"{fallback}\" (\r\n\
                   start \"\" \"{fallback}\" --after-update\r\n\
                 )\r\n\
                 del \"%~f0\" >nul 2>&1\r\n",
                msi = msi_str,
                fallback = exe_after.replace('"', ""),
            );
            std::fs::write(&script_path, script)
                .context("自動アップデートスクリプトの作成に失敗しました")?;

            let mut cmd = std::process::Command::new("cmd");
            cmd.args(["/C", &script_path.to_string_lossy()]);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.spawn()
                .context("MSI 自動アップデートの起動に失敗しました")?;
        } else {
            std::process::Command::new("msiexec")
                .args(["/i", &path.to_string_lossy()])
                .spawn()
                .context("MSI インストーラーの起動に失敗しました")?;
        }
        return Ok(());
    }

    let mut cmd = std::process::Command::new(&path);
    if silent {
        cmd.args(["/SILENT", "/CLOSEAPPLICATIONS", "/AUTOUPDATE"]);
    }
    cmd.spawn()
        .context("インストーラーの起動に失敗しました")?;
    Ok(())
}

/// 起動時のリリースノート表示要否と内容を取得する
pub async fn get_startup_release_notes(
    db_last_version: Option<String>,
    launched_after_update: bool,
) -> ReleaseNotesInfo {
    let current = CURRENT_APP_VERSION.to_string();
    let should_show = launched_after_update
        || match &db_last_version {
            Some(prev) => prev != &current,
            None => false,
        };

    let markdown_body = fetch_release_notes_body(&current).await;

    ReleaseNotesInfo {
        version: current.clone(),
        should_show_on_startup: should_show,
        title: format!("Stosup v{} アップデート内容", current),
        markdown_body,
        previous_version: db_last_version,
    }
}

/// 現在バージョンのリリースノートを常に取得する（設定画面から手動閲覧用）
pub async fn get_current_release_notes(db_last_version: Option<String>) -> ReleaseNotesInfo {
    let current = CURRENT_APP_VERSION.to_string();
    let markdown_body = fetch_release_notes_body(&current).await;
    ReleaseNotesInfo {
        version: current.clone(),
        should_show_on_startup: false,
        title: format!("Stosup v{} リリースノート", current),
        markdown_body,
        previous_version: db_last_version,
    }
}

pub fn record_last_launched_version(db: &Database) -> Result<()> {
    db.set_setting("last_launched_version", CURRENT_APP_VERSION)
}

async fn fetch_release_notes_body(version: &str) -> String {
    // まず GitHub Releases API から該当タグのリリース本文を取得（タイムアウト短め）
    let tag_url = format!(
        "https://api.github.com/repos/{}/{}/releases/tags/v{}",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME, version
    );
    if let Ok(client) = github_client(4) {
        if let Ok(resp) = client
            .get(&tag_url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(rel) = resp.json::<GitHubRelease>().await {
                    if let Some(body) = rel.body {
                        if !body.trim().is_empty() {
                            return body;
                        }
                    }
                }
            }
        }
    }

    // オフラインまたは未公開時は同梱 CHANGELOG.md から該当バージョンを抽出
    extract_changelog_section(BUNDLED_CHANGELOG, version)
}

/// CHANGELOG.md から指定バージョン (`## [X.Y.Z]`) のセクション本文を抽出する
pub fn extract_changelog_section(changelog: &str, version: &str) -> String {
    let target = format!("## [{}]", version.trim().trim_start_matches('v'));
    let mut collecting = false;
    let mut lines = Vec::new();

    for line in changelog.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## [") {
            if collecting {
                break;
            }
            if trimmed.starts_with(&target) {
                collecting = true;
                continue;
            }
        } else if collecting {
            lines.push(line);
        }
    }

    let extracted = lines.join("\n").trim().to_string();
    if extracted.is_empty() {
        format!("バージョン v{} へのアップデートが適用されました。", version)
    } else {
        extracted
    }
}

fn parse_sha256_for_file(checksum_content: &str, file_name: &str) -> Option<String> {
    for line in checksum_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[1].trim_start_matches('*');
            if name.eq_ignore_ascii_case(file_name) {
                return Some(parts[0].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparison() {
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.9.9", "v1.0.0"));
        assert!(!is_newer_version("0.1.0", "v0.1.0"));
        assert!(!is_newer_version("1.0.0", "v0.9.9"));
    }

    #[test]
    fn test_extract_changelog_section() {
        let md = "\
# 更新履歴

## [0.2.0] - 2026-12-01

### 新機能
- Twitch対応

## [0.1.0] - 2026-11-25

### 新機能
- 初回リリース
";
        let s02 = extract_changelog_section(md, "0.2.0");
        assert!(s02.contains("Twitch対応"));
        assert!(!s02.contains("初回リリース"));

        let s01 = extract_changelog_section(md, "0.1.0");
        assert!(s01.contains("初回リリース"));
        assert!(!s01.contains("Twitch対応"));
    }

    #[test]
    fn test_parse_sha256_file() {
        let txt = "aabbcc112233  Stosup-v0.3.0-x64.msi\n9988776655 *other.zip\n";
        assert_eq!(
            parse_sha256_for_file(txt, "Stosup-v0.3.0-x64.msi").as_deref(),
            Some("aabbcc112233")
        );
        assert_eq!(
            parse_sha256_for_file(txt, "other.zip").as_deref(),
            Some("9988776655")
        );
    }
}
