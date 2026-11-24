use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::extract::Query;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{oneshot, Mutex};

use crate::db::Database;
use crate::models::YouTubeOAuthStatus;

const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/youtube.force-ssl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_ms: i64,
    pub account_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
}

pub fn save_oauth_client_config(
    db: &Database,
    client_id: &str,
    client_secret: &str,
) -> Result<()> {
    db.set_setting("yt_oauth_client_id", client_id.trim())?;
    db.set_setting("yt_oauth_client_secret", client_secret.trim())?;
    Ok(())
}

pub fn get_oauth_status(db: &Database) -> Result<YouTubeOAuthStatus> {
    let client_id = db.get_setting("yt_oauth_client_id")?.unwrap_or_default();
    let client_secret = db
        .get_setting("yt_oauth_client_secret")?
        .unwrap_or_default();
    let tokens = load_tokens(db)?;
    Ok(YouTubeOAuthStatus {
        client_id,
        has_client_secret: !client_secret.is_empty(),
        is_authenticated: tokens.is_some(),
        account_name: tokens.as_ref().and_then(|t| t.account_name.clone()),
        expires_at_ms: tokens.as_ref().map(|t| t.expires_at_ms),
    })
}

pub fn clear_oauth_tokens(db: &Database) -> Result<()> {
    db.delete_setting("yt_oauth_tokens")?;
    Ok(())
}

pub fn load_tokens(db: &Database) -> Result<Option<StoredOAuthTokens>> {
    if let Some(json) = db.get_setting("yt_oauth_tokens")? {
        if let Ok(tokens) = serde_json::from_str::<StoredOAuthTokens>(&json) {
            return Ok(Some(tokens));
        }
    }
    Ok(None)
}

fn save_tokens(db: &Database, tokens: &StoredOAuthTokens) -> Result<()> {
    let json = serde_json::to_string(tokens)?;
    db.set_setting("yt_oauth_tokens", &json)
}

/// ブラウザを開いて Google OAuth 2.0 (Loopback + PKCE) 認証を行う
pub async fn run_oauth_login(db: Arc<Mutex<Database>>) -> Result<YouTubeOAuthStatus> {
    let (client_id, client_secret) = {
        let guard = db.lock().await;
        let cid = guard
            .get_setting("yt_oauth_client_id")?
            .unwrap_or_default();
        let csec = guard
            .get_setting("yt_oauth_client_secret")?
            .unwrap_or_default();
        (cid, csec)
    };

    if client_id.trim().is_empty() {
        return Err(anyhow!(
            "Google OAuth クライアントID が設定されていません。先に設定画面から入力して保存してください。"
        ));
    }

    // PKCE code_verifier & code_challenge (S256)
    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
    let challenge_hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(challenge_hash);

    let mut state_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let expected_state = URL_SAFE_NO_PAD.encode(state_bytes);

    // ローカルループバックの一時待受ポートを確保
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("OAuthコールバック用ローカルサーバーの起動に失敗しました")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{}/oauth/callback", port);

    let (code_tx, code_rx) = oneshot::channel::<Result<String>>();
    let shared_tx = Arc::new(Mutex::new(Some(code_tx)));
    let expected_state_clone = expected_state.clone();

    let app = Router::new().route(
        "/oauth/callback",
        get(move |Query(params): Query<HashMap<String, String>>| {
            let shared_tx = Arc::clone(&shared_tx);
            let expected_state = expected_state_clone.clone();
            async move {
                let mut guard = shared_tx.lock().await;
                if let Some(tx) = guard.take() {
                    if let Some(err) = params.get("error") {
                        let _ = tx.send(Err(anyhow!("Google認証がキャンセルまたは拒否されました: {}", err)));
                        return Html(
                            "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><title>Stosup 認証エラー</title></head>\
                             <body style='font-family:sans-serif;background:#0f172a;color:#f8fafc;display:flex;align-items:center;justify-content:center;height:100vh;margin:0'>\
                             <div style='text-align:center'><h2>認証がキャンセルされました</h2><p>このタブを閉じて Stosup に戻ってください。</p></div></body></html>"
                                .to_string(),
                        );
                    }
                    let state = params.get("state").cloned().unwrap_or_default();
                    let code = params.get("code").cloned().unwrap_or_default();
                    if state != expected_state || code.is_empty() {
                        let _ = tx.send(Err(anyhow!("OAuth state の検証に失敗しました")));
                        return Html(
                            "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><title>Stosup 認証エラー</title></head>\
                             <body style='font-family:sans-serif;background:#0f172a;color:#f8fafc;display:flex;align-items:center;justify-content:center;height:100vh;margin:0'>\
                             <div style='text-align:center'><h2>認証検証エラー</h2><p>もう一度 Stosup からやり直してください。</p></div></body></html>"
                                .to_string(),
                        );
                    }
                    let _ = tx.send(Ok(code));
                }
                Html(
                    "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><title>Stosup 認証完了</title></head>\
                     <body style='font-family:sans-serif;background:#0f172a;color:#f8fafc;display:flex;align-items:center;justify-content:center;height:100vh;margin:0'>\
                     <div style='text-align:center;padding:32px;border-radius:16px;background:#1e293b;border:1px solid #334155'>\
                     <h2 style='color:#38bdf8;margin-top:0'>Stosup との YouTube 連携が完了しました！</h2>\
                     <p>このブラウザタブを閉じて Stosup アプリにお戻りください。</p></div></body></html>"
                        .to_string(),
                )
            }
        }),
    );

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    // 認証URL生成＆既定ブラウザで起動
    let mut auth_url = url::Url::parse(OAUTH_AUTH_URL)?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", OAUTH_SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &expected_state)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    open_url_in_browser(auth_url.as_str())?;

    // 最大180秒コールバックを待機
    let code = tokio::time::timeout(std::time::Duration::from_secs(180), code_rx)
        .await
        .map_err(|_| anyhow!("Google認証がタイムアウトしました（3分以内に完了してください）"))??;
    let _ = shutdown_tx.send(());
    let code = code?;

    // 認可コードをトークンへ交換
    let client = reqwest::Client::new();
    let mut form = vec![
        ("code", code.as_str()),
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier.as_str()),
    ];
    if !client_secret.is_empty() {
        form.push(("client_secret", client_secret.as_str()));
    }

    let resp = client
        .post(OAUTH_TOKEN_URL)
        .form(&form)
        .send()
        .await
        .context("トークン交換リクエストに失敗しました")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("トークン交換に失敗しました: {}", body));
    }

    let token_res: TokenResponse = resp.json().await?;
    let expires_in = token_res.expires_in.unwrap_or(3600);
    let expires_at_ms = Utc::now().timestamp_millis() + expires_in * 1000;

    // 認証されたチャンネル名を取得
    let account_name = fetch_authenticated_channel_name(&client, &token_res.access_token)
        .await
        .ok();

    let stored = StoredOAuthTokens {
        access_token: token_res.access_token,
        refresh_token: token_res.refresh_token,
        expires_at_ms,
        account_name,
    };

    let guard = db.lock().await;
    save_tokens(&guard, &stored)?;
    get_oauth_status(&guard)
}

/// 有効なアクセストークンを取得する（期限切れ間近なら自動リフレッシュ）
pub async fn get_valid_access_token(db: Arc<Mutex<Database>>) -> Result<String> {
    let (client_id, client_secret, stored) = {
        let guard = db.lock().await;
        let cid = guard
            .get_setting("yt_oauth_client_id")?
            .unwrap_or_default();
        let csec = guard
            .get_setting("yt_oauth_client_secret")?
            .unwrap_or_default();
        let tokens = load_tokens(&guard)?;
        (cid, csec, tokens)
    };

    let mut tokens = stored.ok_or_else(|| {
        anyhow!("YouTube アカウントが連携されていません。設定画面から OAuth 認証を行ってください。")
    })?;

    let now_ms = Utc::now().timestamp_millis();
    if tokens.expires_at_ms - now_ms > 60_000 {
        return Ok(tokens.access_token);
    }

    let refresh_token = tokens.refresh_token.clone().ok_or_else(|| {
        anyhow!("リフレッシュトークンがありません。設定画面から再度 Google 認証を行ってください。")
    })?;

    let client = reqwest::Client::new();
    let mut form = vec![
        ("client_id", client_id.as_str()),
        ("refresh_token", refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];
    if !client_secret.is_empty() {
        form.push(("client_secret", client_secret.as_str()));
    }

    let resp = client.post(OAUTH_TOKEN_URL).form(&form).send().await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "アクセストークンの自動更新に失敗しました（再認証してください）: {}",
            body
        ));
    }

    let refreshed: TokenResponse = resp.json().await?;
    tokens.access_token = refreshed.access_token;
    tokens.expires_at_ms = Utc::now().timestamp_millis() + refreshed.expires_in.unwrap_or(3600) * 1000;
    if refreshed.refresh_token.is_some() {
        tokens.refresh_token = refreshed.refresh_token;
    }

    let guard = db.lock().await;
    save_tokens(&guard, &tokens)?;
    Ok(tokens.access_token)
}

async fn fetch_authenticated_channel_name(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<String> {
    let resp = client
        .get("https://www.googleapis.com/youtube/v3/channels")
        .query(&[("part", "snippet"), ("mine", "true")])
        .bearer_auth(access_token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("チャンネル情報取得失敗"));
    }
    let val: serde_json::Value = resp.json().await?;
    let title = val["items"][0]["snippet"]["title"]
        .as_str()
        .unwrap_or("YouTubeアカウント")
        .to_string();
    Ok(title)
}

/// 既定ブラウザでURLを開く（Windows ネイティブコマンド使用）
pub fn open_url_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .context("ブラウザの起動に失敗しました")?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    Ok(())
}
