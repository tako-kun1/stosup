use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::db::Database;
use crate::models::LicenseAgreementStatus;
use crate::updater::{CURRENT_APP_VERSION, GITHUB_REPO_NAME, GITHUB_REPO_OWNER};

/// アプリ本体へ同梱する最新の利用規約（オフライン時のフォールバック ＆ 基準版）
pub const BUNDLED_LICENSE_MD: &str = include_str!("../../License.md");

const KEY_ACCEPTED_HASH: &str = "license_accepted_hash";
const KEY_ACCEPTED_REV: &str = "license_accepted_revision";
const KEY_ACCEPTED_AT: &str = "license_accepted_at_ms";
const KEY_CACHED_BODY: &str = "license_cached_body";
const KEY_CACHED_HASH: &str = "license_cached_hash";
const KEY_CACHED_APP_VER: &str = "license_cached_app_version";

/// GitHub 上の最新 License.md の Raw URL
pub fn remote_license_raw_url() -> String {
    format!(
        "https://raw.githubusercontent.com/{}/{}/main/License.md",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME
    )
}

/// 改行コード差（CRLF / LF）による誤検知を防ぐため正規化する
pub fn normalize_license_text(raw: &str) -> String {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

/// 正規化済み利用規約本文の SHA-256 ハッシュ（小文字 HEX）を算出する
pub fn compute_license_hash(normalized: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 規約本文からバージョン表記・改定日ラベルを抽出する
pub fn extract_revision_label(normalized: &str) -> String {
    let mut ver: Option<String> = None;
    let mut date: Option<String> = None;

    for line in normalized.lines() {
        let trimmed = line.trim().trim_start_matches('-').trim();
        if let Some(rest) = trimmed
            .strip_prefix("規約バージョン:")
            .or_else(|| trimmed.strip_prefix("規約バージョン："))
        {
            let v = rest.trim().to_string();
            if !v.is_empty() {
                ver = Some(v);
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("最終更新日:")
            .or_else(|| trimmed.strip_prefix("最終更新日："))
            .or_else(|| trimmed.strip_prefix("改定日:"))
            .or_else(|| trimmed.strip_prefix("改定日："))
            .or_else(|| trimmed.strip_prefix("制定日:"))
            .or_else(|| trimmed.strip_prefix("制定日："))
        {
            let d = rest.trim().to_string();
            if !d.is_empty() {
                date = Some(d);
            }
        }
    }

    match (ver, date) {
        (Some(v), Some(d)) => format!("v{} ({})", v.trim_start_matches('v'), d),
        (Some(v), None) => format!("v{}", v.trim_start_matches('v')),
        (None, Some(d)) => d,
        (None, None) => format!("v{}", CURRENT_APP_VERSION),
    }
}

/// 規約本文からタイトル見出しを抽出する
pub fn extract_title(normalized: &str) -> String {
    for line in normalized.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    "Stosup ソフトウェア利用規約（使用許諾契約書）".to_string()
}

/// GitHub 上の最新 License.md をオンライン取得する（オフライン・障害時は None）
pub async fn fetch_remote_license_md() -> Option<String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("Stosup/{}", CURRENT_APP_VERSION))
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;

    let resp = client
        .get(remote_license_raw_url())
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().await.ok()?;
    let normalized = normalize_license_text(&body);
    // エラーページや不正レスポンスを弾く最低限の妥当性チェック
    if normalized.len() < 120
        || normalized.starts_with("<!DOCTYPE")
        || normalized.starts_with("<html")
        || !normalized.contains("Stosup")
    {
        return None;
    }
    Some(normalized)
}

/// DB の同意履歴とオンライン／同梱の最新規約を突き合わせて同意要否を判定する
pub fn build_license_status_with_remote(
    db: &Database,
    remote_normalized: Option<String>,
) -> Result<LicenseAgreementStatus> {
    let bundled_norm = normalize_license_text(BUNDLED_LICENSE_MD);
    let bundled_hash = compute_license_hash(&bundled_norm);

    let (active_body, active_hash, source) = if let Some(remote_body) = remote_normalized {
        let remote_hash = compute_license_hash(&remote_body);
        let _ = db.set_setting(KEY_CACHED_BODY, &remote_body);
        let _ = db.set_setting(KEY_CACHED_HASH, &remote_hash);
        let _ = db.set_setting(KEY_CACHED_APP_VER, CURRENT_APP_VERSION);
        (remote_body, remote_hash, "online".to_string())
    } else {
        // オフライン時：同一アプリバージョンで以前にオンライン取得した新しい規約があれば維持する
        let cached_body = db.get_setting(KEY_CACHED_BODY)?;
        let cached_app_ver = db.get_setting(KEY_CACHED_APP_VER)?;
        if let (Some(cb), Some(cv)) = (cached_body, cached_app_ver) {
            let cb_norm = normalize_license_text(&cb);
            if !cb_norm.is_empty() && cv == CURRENT_APP_VERSION {
                let ch = compute_license_hash(&cb_norm);
                (cb_norm, ch, "cached".to_string())
            } else {
                (bundled_norm, bundled_hash, "bundled".to_string())
            }
        } else {
            (bundled_norm, bundled_hash, "bundled".to_string())
        }
    };

    let accepted_hash = db
        .get_setting(KEY_ACCEPTED_HASH)?
        .filter(|s| !s.trim().is_empty());
    let accepted_revision = db
        .get_setting(KEY_ACCEPTED_REV)?
        .filter(|s| !s.trim().is_empty());
    let accepted_at_ms = db
        .get_setting(KEY_ACCEPTED_AT)?
        .and_then(|s| s.trim().parse::<i64>().ok());

    let needs_acceptance = accepted_hash.as_deref() != Some(active_hash.as_str());
    let is_updated_license = needs_acceptance && accepted_hash.is_some();

    Ok(LicenseAgreementStatus {
        title: extract_title(&active_body),
        revision_label: extract_revision_label(&active_body),
        markdown_body: active_body,
        content_hash: active_hash,
        source,
        accepted_hash,
        accepted_revision,
        accepted_at_ms,
        needs_acceptance,
        is_updated_license,
    })
}

/// ユーザーが現在の利用規約へ同意したことを永続化する
pub fn record_license_acceptance(
    db: &Database,
    content_hash: &str,
    revision_label: &str,
) -> Result<()> {
    let now_ms = Utc::now().timestamp_millis();
    db.set_setting(KEY_ACCEPTED_HASH, content_hash.trim())?;
    db.set_setting(KEY_ACCEPTED_REV, revision_label.trim())?;
    db.set_setting(KEY_ACCEPTED_AT, &now_ms.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_normalization_and_revision_extraction() {
        let lf = "# Stosup 規約\n\n## 附則\n- 規約バージョン: 1.2\n- 最終更新日: 2026-12-01\n";
        let crlf = "# Stosup 規約\r\n\r\n## 附則\r\n- 規約バージョン: 1.2\r\n- 最終更新日: 2026-12-01\r\n";
        let n1 = normalize_license_text(lf);
        let n2 = normalize_license_text(crlf);
        assert_eq!(n1, n2);
        assert_eq!(compute_license_hash(&n1), compute_license_hash(&n2));
        assert_eq!(extract_revision_label(&n1), "v1.2 (2026-12-01)");
        assert_eq!(extract_title(&n1), "Stosup 規約");
    }

    #[test]
    fn test_license_acceptance_and_update_redetection() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();

        // 1. 初回起動は未同意のため同意必須
        let s1 = build_license_status_with_remote(&db, None).unwrap();
        assert!(s1.needs_acceptance);
        assert!(!s1.is_updated_license);
        assert_eq!(s1.source, "bundled");
        assert!(s1.markdown_body.contains("第1条"));
        assert!(s1.markdown_body.contains("先行版（pre バージョン）"));
        assert_eq!(s1.revision_label, "v1.1 (2026-11-28)");

        // 2. 同意を記録すると通常起動可能になる
        record_license_acceptance(&db, &s1.content_hash, &s1.revision_label).unwrap();
        let s2 = build_license_status_with_remote(&db, None).unwrap();
        assert!(!s2.needs_acceptance);
        assert!(!s2.is_updated_license);
        assert!(s2.accepted_at_ms.is_some());

        // 3. オンラインで新しい規約が配信されたら再度同意が必要になる（更新検知）
        let updated_md = format!(
            "{}\n\n## 改定追記\n- 規約バージョン: 2.0\n- 最終更新日: 2026-12-10",
            BUNDLED_LICENSE_MD
        );
        let s3 = build_license_status_with_remote(&db, Some(normalize_license_text(&updated_md)))
            .unwrap();
        assert!(s3.needs_acceptance);
        assert!(s3.is_updated_license);
        assert_eq!(s3.source, "online");

        // 4. 次の起動がオフラインでも、キャッシュ済み改定規約が維持され同意をすり抜けられない
        let s4 = build_license_status_with_remote(&db, None).unwrap();
        assert!(s4.needs_acceptance);
        assert!(s4.is_updated_license);
        assert_eq!(s4.source, "cached");

        // 5. 新規約に同意すると再び通常起動可能になる
        record_license_acceptance(&db, &s4.content_hash, &s4.revision_label).unwrap();
        let s5 = build_license_status_with_remote(&db, None).unwrap();
        assert!(!s5.needs_acceptance);
    }
}
