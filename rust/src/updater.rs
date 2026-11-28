use std::cmp::Ordering;
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

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
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

/// バージョン文字列（`v0.5.1`, `0.5.1-pre`, `0.5.1pre1`, `0.5.1_pre` 等）を SemVer へ正規化パースする
pub fn parse_version(raw: &str) -> Option<Version> {
    let trimmed = raw
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V');
    if let Ok(v) = Version::parse(trimmed) {
        return Some(v);
    }

    // ハイフン無し・アンダースコア・ドット区切りで pre 等が続く表記（例: 0.5.1pre, 0.5.1_pre, 0.5.1.pre1）を正規化
    let mut split_idx = None;
    let mut dot_count = 0usize;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            continue;
        }
        if ch == '.' && dot_count < 2 {
            dot_count += 1;
            continue;
        }
        split_idx = Some(idx);
        break;
    }

    if let Some(idx) = split_idx {
        let (core, rest) = trimmed.split_at(idx);
        let rest_clean = rest.trim_start_matches(|c| c == '-' || c == '_' || c == '.');
        if !core.is_empty() && !rest_clean.is_empty() {
            let normalized_pre = rest_clean.replace('_', "-");
            let candidate = format!("{}-{}", core, normalized_pre);
            if let Ok(v) = Version::parse(&candidate) {
                return Some(v);
            }
        }
    }
    None
}

/// タグ・バージョン文字列に pre（または SemVer プレリリース識別子）が付いているか判定する
pub fn is_pre_version_tag(version_or_tag: &str) -> bool {
    let lower = version_or_tag.trim().to_ascii_lowercase();
    if lower.contains("pre") {
        return true;
    }
    parse_version(version_or_tag)
        .map(|v| !v.pre.is_empty())
        .unwrap_or(false)
}

fn is_release_prerelease(release: &GitHubRelease) -> bool {
    release.prerelease || is_pre_version_tag(&release.tag_name)
}

/// 同一ナンバリング（major.minor.patch）では pre 無し正式版を常に上位とする比較
/// 例: 0.5.1-pre < 0.5.1 / 0.5.0 < 0.5.1-pre < 0.5.1
fn compare_version_with_pre(
    a_ver: &Version,
    a_is_pre: bool,
    b_ver: &Version,
    b_is_pre: bool,
) -> Ordering {
    (a_ver.major, a_ver.minor, a_ver.patch, !a_is_pre)
        .cmp(&(b_ver.major, b_ver.minor, b_ver.patch, !b_is_pre))
        .then_with(|| a_ver.cmp(b_ver))
}

/// セマンティックバージョンを比較して latest > current か判定する（例: 0.5.1-pre < 0.5.1）
pub fn is_newer_version(current: &str, latest_tag: &str) -> bool {
    let cur_pre = is_pre_version_tag(current);
    let lat_pre = is_pre_version_tag(latest_tag);
    match (parse_version(current), parse_version(latest_tag)) {
        (Some(cv), Some(lv)) => {
            compare_version_with_pre(&lv, lat_pre, &cv, cur_pre) == Ordering::Greater
        }
        _ => false,
    }
}

fn build_update_info_from_release(
    current_version: &str,
    release: &GitHubRelease,
    has_update: bool,
    is_prerelease: bool,
) -> UpdateInfo {
    let latest_clean = release
        .tag_name
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string();

    // pre が付くプレリリース版は自動更新させないためインストーラー URL を渡さずアナウンス専用にする
    let (installer_url, installer_name, checksum_url) = if is_prerelease {
        (None, None, None)
    } else {
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

        (
            installer_asset.map(|a| a.browser_download_url.clone()),
            installer_asset.map(|a| a.name.clone()),
            checksum_asset.map(|a| a.browser_download_url.clone()),
        )
    };

    UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest_clean,
        has_update,
        is_prerelease,
        release_name: release
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| release.tag_name.clone()),
        release_notes: release.body.clone().unwrap_or_default(),
        published_at: release.published_at.clone().unwrap_or_default(),
        installer_url,
        installer_name,
        checksum_url,
        html_url: release.html_url.clone(),
    }
}

fn empty_update_info(current_version: &str) -> UpdateInfo {
    UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: current_version.to_string(),
        has_update: false,
        is_prerelease: false,
        release_name: format!("v{}", current_version),
        release_notes: extract_changelog_section(BUNDLED_CHANGELOG, current_version),
        published_at: String::new(),
        installer_url: None,
        installer_name: None,
        checksum_url: None,
        html_url: format!(
            "https://github.com/{}/{}/releases",
            GITHUB_REPO_OWNER, GITHUB_REPO_NAME
        ),
    }
}

/// 複数リリース一覧から最適な更新候補を選定する
/// - 同一ナンバリングに pre 無し正式版がある場合は必ず正式版が優先される（0.5.1-pre < 0.5.1）
/// - 現在より新しい正式版がある場合は自動更新可能な正式版を最優先する
/// - `include_prerelease` が true かつ新しい正式版がなく、新しい pre 版のみ存在する場合はアナウンス専用（is_prerelease = true）として返す
/// - `include_prerelease` が false の場合は pre 版を更新・アナウンス対象に含めない
fn select_update_from_releases(
    current_version: &str,
    releases: &[GitHubRelease],
    include_prerelease: bool,
) -> UpdateInfo {
    let cur_ver = parse_version(current_version);
    let cur_is_pre = is_pre_version_tag(current_version);

    let mut candidates: Vec<(&GitHubRelease, Version, bool)> = releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let ver = parse_version(&r.tag_name)?;
            let is_pre = is_release_prerelease(r);
            Some((r, ver, is_pre))
        })
        .collect();

    if candidates.is_empty() {
        return empty_update_info(current_version);
    }

    // 高いバージョン順（同一ナンバリングは pre 無し正式版が上位）へ降順ソート
    candidates.sort_by(|(_, av, ap), (_, bv, bp)| compare_version_with_pre(bv, *bp, av, *ap));

    if let Some(ref cv) = cur_ver {
        // 1. 現在より新しい正式版（pre 無し）があれば最優先で自動更新対象とする
        if let Some((rel, _, _)) = candidates.iter().find(|(_, ver, is_pre)| {
            !*is_pre && compare_version_with_pre(ver, false, cv, cur_is_pre) == Ordering::Greater
        }) {
            return build_update_info_from_release(current_version, rel, true, false);
        }

        // 2. 設定が ON の場合のみ、新しい正式版がなく現在より新しい pre 版があればアナウンス専用として通知する
        if include_prerelease {
            if let Some((rel, _, _)) = candidates.iter().find(|(_, ver, is_pre)| {
                *is_pre && compare_version_with_pre(ver, true, cv, cur_is_pre) == Ordering::Greater
            }) {
                return build_update_info_from_release(current_version, rel, true, true);
            }
        }
    }

    // 3. 更新なし（正式版が存在すればそれを最新情報として返し、pre 版のみかつ設定 OFF なら現在版情報を返す）
    if let Some((best_rel, _, _)) = candidates.iter().find(|(_, _, is_pre)| !*is_pre) {
        return build_update_info_from_release(current_version, best_rel, false, false);
    }
    if include_prerelease {
        let (best_rel, _, best_is_pre) = candidates[0];
        return build_update_info_from_release(current_version, best_rel, false, best_is_pre);
    }
    empty_update_info(current_version)
}

/// GitHub Releases のリリース一覧を確認し、正式版優先・pre 版アナウンス専用ルールで判定する
pub async fn check_for_updates(include_prerelease: bool) -> Result<UpdateInfo> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=30",
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
        return Ok(select_update_from_releases(
            CURRENT_APP_VERSION,
            &[],
            include_prerelease,
        ));
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

    let releases: Vec<GitHubRelease> = resp.json().await?;
    Ok(select_update_from_releases(
        CURRENT_APP_VERSION,
        &releases,
        include_prerelease,
    ))
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
                 if exist \"{current}\" (\r\n\
                   start \"\" \"{current}\" --after-update\r\n\
                 ) else if exist \"%LOCALAPPDATA%\\Stosup\\stosup.exe\" (\r\n\
                   start \"\" \"%LOCALAPPDATA%\\Stosup\\stosup.exe\" --after-update\r\n\
                 )\r\n\
                 del \"%~f0\" >nul 2>&1\r\n",
                msi = msi_str,
                current = exe_after.replace('"', ""),
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

    fn mock_release(tag: &str, prerelease: bool, draft: bool) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            name: Some(format!("Stosup {}", tag)),
            body: Some(format!("Notes for {}", tag)),
            published_at: Some("2026-11-28T00:00:00Z".to_string()),
            html_url: format!("https://github.com/tako-kun1/stosup/releases/tag/{}", tag),
            draft,
            prerelease,
            assets: vec![GitHubAsset {
                name: format!("Stosup-{}-x64.msi", tag),
                browser_download_url: format!(
                    "https://github.com/tako-kun1/stosup/releases/download/{}/Stosup-{}-x64.msi",
                    tag, tag
                ),
            }],
        }
    }

    #[test]
    fn test_semver_comparison() {
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.9.9", "v1.0.0"));
        assert!(!is_newer_version("0.1.0", "v0.1.0"));
        assert!(!is_newer_version("1.0.0", "v0.9.9"));

        // pre バージョン比較: 0.5.0 < 0.5.1-pre < 0.5.1
        assert!(is_newer_version("0.5.0", "v0.5.1-pre"));
        assert!(is_newer_version("0.5.1-pre", "v0.5.1"));
        assert!(!is_newer_version("0.5.1", "v0.5.1-pre"));
        assert!(is_newer_version("0.5.1-pre.1", "v0.5.1-pre.2"));
        assert!(is_newer_version("0.5.1pre", "v0.5.1"));
        assert!(!is_newer_version("0.5.1", "v0.5.1pre"));
        assert!(is_pre_version_tag("v0.5.1-pre"));
        assert!(is_pre_version_tag("0.5.1pre1"));
        assert!(is_pre_version_tag("0.6.0-pre01"));
        assert!(is_pre_version_tag("v0.6.0-pre01"));
        assert!(!is_pre_version_tag("v0.5.1"));
        assert!(is_newer_version("0.5.1", "v0.6.0-pre01"));
        assert!(is_newer_version("0.6.0-pre01", "v0.6.0-pre02"));
        assert!(is_newer_version("0.6.0-pre01", "v0.6.0"));
        assert!(!is_newer_version("0.6.0", "v0.6.0-pre01"));
    }

    #[test]
    fn test_pre_release_is_announcement_only_when_enabled_and_hidden_when_disabled() {
        let releases = vec![
            mock_release("v0.5.1-pre", true, false),
            mock_release("v0.5.0", false, false),
        ];
        // デフォルト（OFF）では pre 版を通知対象に含めない
        let info_disabled = select_update_from_releases("0.5.0", &releases, false);
        assert!(!info_disabled.has_update);
        assert!(!info_disabled.is_prerelease);
        assert_eq!(info_disabled.latest_version, "0.5.0");

        // ON にした場合のみアナウンス専用として返す
        let info_enabled = select_update_from_releases("0.5.0", &releases, true);
        assert!(info_enabled.has_update);
        assert!(info_enabled.is_prerelease);
        assert_eq!(info_enabled.latest_version, "0.5.1-pre");
        // 自動更新をさせないためインストーラー URL は渡さない
        assert!(info_enabled.installer_url.is_none());
        assert!(info_enabled.installer_name.is_none());

        // pre 版しかリリースされていない状態で OFF の場合は現在版で更新なしを返す
        let only_pre = vec![mock_release("v0.5.1-pre", true, false)];
        let only_pre_off = select_update_from_releases("0.5.0", &only_pre, false);
        assert!(!only_pre_off.has_update);
        assert!(!only_pre_off.is_prerelease);
        assert_eq!(only_pre_off.latest_version, "0.5.0");
    }

    #[test]
    fn test_stable_prioritized_over_same_number_pre() {
        // GitHub 上で prerelease フラグが付いていない 0.5.1-pre が混在しても 0.5.1 を優先する
        let releases = vec![
            mock_release("v0.5.1-pre", false, false),
            mock_release("v0.5.1", false, false),
            mock_release("v0.5.0", false, false),
        ];
        let info = select_update_from_releases("0.5.0", &releases, true);
        assert!(info.has_update);
        assert!(!info.is_prerelease);
        assert_eq!(info.latest_version, "0.5.1");
        assert!(info.installer_url.is_some());

        // 0.5.1-pre 利用中にも同一ナンバリングの正式版 0.5.1 が優先利用される（pre 通知 OFF でも正式版は案内する）
        let from_pre = select_update_from_releases("0.5.1-pre", &releases, false);
        assert!(from_pre.has_update);
        assert!(!from_pre.is_prerelease);
        assert_eq!(from_pre.latest_version, "0.5.1");
        assert!(from_pre.installer_url.is_some());

        // すでに 0.5.1 なら 0.5.1-pre は更新扱いにならない
        let from_stable = select_update_from_releases("0.5.1", &releases, true);
        assert!(!from_stable.has_update);
    }

    #[test]
    fn test_stable_update_prioritized_over_newer_pre_and_ignores_drafts() {
        let releases = vec![
            mock_release("v0.6.0", false, true), // draft は無視
            mock_release("v0.5.2-pre", true, false),
            mock_release("v0.5.1", false, false),
            mock_release("v0.5.0", false, false),
        ];
        // 0.5.0 からはまず自動更新可能な正式版 0.5.1 が優先される
        let from_050 = select_update_from_releases("0.5.0", &releases, true);
        assert!(from_050.has_update);
        assert!(!from_050.is_prerelease);
        assert_eq!(from_050.latest_version, "0.5.1");

        // 0.5.1 へ上げた後、pre 通知 OFF なら 0.5.2-pre は通知しない
        let from_051_off = select_update_from_releases("0.5.1", &releases, false);
        assert!(!from_051_off.has_update);
        assert!(!from_051_off.is_prerelease);
        assert_eq!(from_051_off.latest_version, "0.5.1");

        // 0.5.1 へ上げた後、pre 通知 ON なら 0.5.2-pre がアナウンス専用で通知される
        let from_051 = select_update_from_releases("0.5.1", &releases, true);
        assert!(from_051.has_update);
        assert!(from_051.is_prerelease);
        assert_eq!(from_051.latest_version, "0.5.2-pre");
        assert!(from_051.installer_url.is_none());
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
