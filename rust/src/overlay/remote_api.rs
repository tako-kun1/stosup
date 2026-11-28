use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::media;
use crate::models::{
    AppEvent, AutomationActionKind, HighlightRule, LotteryRepeatMode, MatchMode,
    MediaAnimationKind, MediaEffectType, MediaKind, MediaPlaybackMode, MediaPosition,
    MediaReactionConfig, MediaReactionRule, MediaTriggerType, PlatformKind, PollConfig,
    PollOption, QueueEntryStatus, UserLotteryRequest, WordCounterConfig,
};
use crate::overlay::OverlaySharedState;
use crate::remote;
use crate::state::get_runtime;
use crate::updater;

const REMOTE_HTML: &str = include_str!("remote.html");

/// 接続元 IP が許可対象（ループバックまたは許可されたプライベート LAN）かをロックフリーで検証するミドルウェア
pub async fn ip_access_guard(
    State(state): State<Arc<OverlaySharedState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let allow_lan = state.is_allow_lan_access();
    if !remote::is_ip_allowed(addr.ip(), allow_lan) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from(
                "403 Forbidden: この端末からのアクセスは許可されていません。Stosup の設定画面でローカルネットワーク接続を許可してください。",
            ))
            .unwrap();
    }
    next.run(req).await
}

/// Web リモコン UI ページ (/remote)
pub async fn remote_page_handler(
    State(state): State<Arc<OverlaySharedState>>,
) -> impl IntoResponse {
    let cfg = state.remote_config.read().await.clone();
    if !cfg.remote_control_enabled {
        return (
            StatusCode::FORBIDDEN,
            Html(
                "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><title>Stosup ポケットスイッチャー無効</title></head>\
                 <body style='font-family:sans-serif;background:#18181b;color:#f4f4f5;display:flex;align-items:center;justify-content:center;min-height:100vh;padding:24px;text-align:center'>\
                 <div><h1>ポケットスイッチャーが無効になっています</h1><p style='color:#a1a1aa;margin-top:8px'>PC 側の Stosup「設定・機能管理」画面でポケットスイッチャー操作画面を有効にしてください。</p></div></body></html>",
            ),
        );
    }
    (StatusCode::OK, Html(REMOTE_HTML))
}

/// タイミング攻撃を防ぐための定数時間文字列比較
pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let max_len = ab.len().max(bb.len());
    let mut diff: u8 = if ab.len() == bb.len() { 0 } else { 1 };
    for i in 0..max_len {
        let x = ab.get(i).copied().unwrap_or(0);
        let y = bb.get(i).copied().unwrap_or(0);
        diff |= x ^ y;
    }
    diff == 0
}

fn parse_host_header(raw: &str) -> Option<(String, u16)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = url::Url::parse(&format!("http://{}", trimmed)).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let port = parsed.port_or_known_default().unwrap_or(80);
    Some((host, port))
}

fn parse_url_origin(raw: &str) -> Option<(String, u16)> {
    let parsed = url::Url::parse(raw.trim()).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let port = parsed.port_or_known_default()?;
    Some((host, port))
}

/// 外部 Web サイト由来（クロスサイト）のブラウザ要求かどうかを判定する。
/// Stream Deck / Loupedeck / curl 等のネイティブ HTTP クライアントは `Sec-Fetch-Site` / `Origin` / `Referer`
/// を送らないため透過許可され、既存のトリガー URL 互換性を維持したまま Drive-by CSRF / CSWSH を遮断できる。
pub(crate) fn is_cross_site_browser_request(headers: &HeaderMap, check_referer: bool) -> bool {
    // 1. Fetch Metadata (`Sec-Fetch-Site`) による判定
    if let Some(sfs) = headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_lowercase())
    {
        if !sfs.is_empty() && sfs != "same-origin" && sfs != "none" {
            return true;
        }
    }

    let expected_host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_host_header);

    // 2. `Origin` ヘッダが存在する場合は自サーバーと完全一致（ホスト＋ポート）のみ許可
    if let Some(origin_raw) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        let origin_trim = origin_raw.trim();
        if !origin_trim.is_empty() {
            if origin_trim.eq_ignore_ascii_case("null") {
                return true;
            }
            let Some(origin_hp) = parse_url_origin(origin_trim) else {
                return true;
            };
            let Some(ref host_hp) = expected_host else {
                return true;
            };
            if &origin_hp != host_hp {
                return true;
            }
        }
    }

    // 3. GET トリガー等では `Referer` が存在する場合も外部オリジンを拒否（`<img src=...>` CSRF 対策）
    if check_referer {
        if let Some(ref_raw) = headers.get(header::REFERER).and_then(|v| v.to_str().ok()) {
            let ref_trim = ref_raw.trim();
            if !ref_trim.is_empty() {
                let Some(ref_hp) = parse_url_origin(ref_trim) else {
                    return true;
                };
                let Some(ref host_hp) = expected_host else {
                    return true;
                };
                if &ref_hp != host_hp {
                    return true;
                }
            }
        }
    }

    false
}

/// オーバーレイ／MOD ページからの `/api/trigger/*` 呼び出しを拒否するための Referer 判定
fn is_overlay_or_mod_referer(headers: &HeaderMap) -> bool {
    let Some(ref_raw) = headers.get(header::REFERER).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(parsed) = url::Url::parse(ref_raw.trim()) else {
        return false;
    };
    let path = parsed.path();
    path.starts_with("/overlay/") || path.starts_with("/mods/")
}

/// CSS インジェクションを防ぐため、色指定を `#RGB` / `#RRGGBB` / `#RRGGBBAA` のみに正規化する
fn sanitize_hex_color(raw: Option<&str>, fallback: &str) -> String {
    let Some(s) = raw.map(|v| v.trim()).filter(|v| !v.is_empty()) else {
        return fallback.to_string();
    };
    let hex = s.trim_start_matches('#');
    if (hex.len() == 3 || hex.len() == 6 || hex.len() == 8)
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        format!("#{}", hex.to_ascii_uppercase())
    } else {
        fallback.to_string()
    }
}

/// 認証要否・バージョン情報 (/api/remote/auth_info)
pub async fn auth_info_handler(
    State(state): State<Arc<OverlaySharedState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if is_cross_site_browser_request(&headers, false) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "外部サイトからのクロスオリジン要求は許可されていません" })),
        );
    }
    let cfg = state.remote_config.read().await.clone();
    (
        StatusCode::OK,
        Json(json!({
            "version": updater::CURRENT_APP_VERSION,
            "remote_control_enabled": cfg.remote_control_enabled,
            "require_pin": cfg.require_pin,
            "allow_lan_access": cfg.allow_lan_access,
        })),
    )
}

#[derive(Deserialize)]
pub struct PinVerifyRequest {
    pub pin: String,
}

/// PIN 検証エンドポイント (/api/remote/verify)
pub async fn verify_pin_handler(
    State(state): State<Arc<OverlaySharedState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<PinVerifyRequest>,
) -> impl IntoResponse {
    if is_cross_site_browser_request(&headers, false) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "外部サイトからのクロスオリジン要求は許可されていません" })),
        );
    }
    let cfg = state.remote_config.read().await.clone();
    if !cfg.remote_control_enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "ポケットスイッチャー操作が無効です" })),
        );
    }
    if !cfg.require_pin {
        return (StatusCode::OK, Json(json!({ "ok": true })));
    }
    let ip = addr.ip();
    if let Some(wait_secs) = state.check_pin_rate_limit(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "ok": false,
                "error": format!("PIN の試行回数が上限を超えました。{} 秒後に再試行してください。", wait_secs)
            })),
        );
    }
    if constant_time_eq(body.pin.trim(), cfg.pin_code.trim()) {
        state.clear_pin_failures(ip);
        return (StatusCode::OK, Json(json!({ "ok": true })));
    }
    if let Some(lock_secs) = state.record_pin_failure(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "ok": false,
                "error": format!("PIN を複数回間違えたため一時ロックされました。{} 秒後に再試行してください。", lock_secs)
            })),
        );
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "ok": false, "error": "PIN コードが正しくありません" })),
    )
}

async fn check_remote_auth(
    state: &OverlaySharedState,
    ip: IpAddr,
    headers: &HeaderMap,
    query: &HashMap<String, String>,
) -> Result<(), (StatusCode, Json<Value>)> {
    if is_cross_site_browser_request(headers, false) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "外部サイトからのクロスオリジン要求は許可されていません" })),
        ));
    }
    let cfg = state.remote_config.read().await.clone();
    if !cfg.remote_control_enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "ポケットスイッチャー操作が無効になっています" })),
        ));
    }
    if !cfg.require_pin {
        return Ok(());
    }
    if let Some(wait_secs) = state.check_pin_rate_limit(ip) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "ok": false,
                "error": format!("PIN の試行回数が上限を超えました。{} 秒後に再試行してください。", wait_secs)
            })),
        ));
    }
    let provided = headers
        .get("x-stosup-pin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .or_else(|| query.get("pin").map(|s| s.trim().to_string()))
        .unwrap_or_default();
    if constant_time_eq(&provided, cfg.pin_code.trim()) {
        state.clear_pin_failures(ip);
        Ok(())
    } else {
        if let Some(lock_secs) = state.record_pin_failure(ip) {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "ok": false,
                    "error": format!("PIN を複数回間違えたため一時ロックされました。{} 秒後に再試行してください。", lock_secs)
                })),
            ));
        }
        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "error": "PIN 認証が必要です" })),
        ))
    }
}

/// 現在の Stosup 全体状態スナップショットを返す (/api/remote/state)
pub async fn remote_state_handler(
    State(state): State<Arc<OverlaySharedState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(resp) = check_remote_auth(&state, addr.ip(), &headers, &query).await {
        return resp;
    }
    let runtime = match get_runtime() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
        }
    };

    let connection = runtime.connection_state.read().await.clone();
    let comments: Vec<_> = {
        let list = runtime.recent_comments.read().await;
        let start = list.len().saturating_sub(80);
        list.iter().skip(start).cloned().collect()
    };
    let monetization = runtime.list_monetization_events(80).await.unwrap_or_default();
    let poll = runtime.get_poll_state().await;
    let picked = runtime.get_picked_comment().await;
    let queue = runtime.get_queue_state().await;
    let counters = runtime.list_word_counters().await.unwrap_or_default();
    let first_time = runtime.first_time_state.lock().await.clone();
    let media_rules = runtime.list_media_reaction_rules().await.unwrap_or_default();
    let media_config = runtime.get_media_reaction_config().await;
    let highlight_rules = runtime.list_highlight_rules().await.unwrap_or_default();
    let feature_flags = runtime.feature_flags.read().await.clone();
    let comment_lottery = runtime.comment_lottery.lock().await.clone();
    let user_lottery_won_count = runtime.user_lottery_won_ids.lock().await.len();
    let users_count = runtime.count_users().await.unwrap_or(0);
    let overlay_config = runtime.overlay_state.config.read().await.clone();
    let overlay_port = overlay_config.port;

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "version": updater::CURRENT_APP_VERSION,
            "overlay_port": overlay_port,
            "overlay_config": overlay_config,
            "connection": connection,
            "comments": comments,
            "monetization": monetization,
            "poll": poll,
            "picked": picked,
            "queue": queue,
            "counters": counters,
            "first_time": first_time,
            "media_rules": media_rules,
            "media_config": media_config,
            "highlight_rules": highlight_rules,
            "feature_flags": feature_flags,
            "comment_lottery": comment_lottery,
            "user_lottery_won_count": user_lottery_won_count,
            "users_count": users_count,
        })),
    )
}

/// URL パーセントデコード（日本語ファイル名対応）
fn percent_decode_utf8(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = (bytes[i + 1] as char).to_digit(16);
            let h2 = (bytes[i + 2] as char).to_digit(16);
            if let (Some(a), Some(b)) = (h1, h2) {
                out.push(((a << 4) | b) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

/// Web リモコンから画像・動画・効果音素材をアップロードする (/api/remote/upload_media)
pub async fn upload_media_handler(
    State(state): State<Arc<OverlaySharedState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> impl IntoResponse {
    if let Err(resp) = check_remote_auth(&state, addr.ip(), &headers, &query).await {
        return resp;
    }
    let runtime = match get_runtime() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
        }
    };

    let raw_name = headers
        .get("x-file-name")
        .and_then(|v| v.to_str().ok())
        .or_else(|| query.get("name").map(|s| s.as_str()))
        .unwrap_or("");
    let file_name = percent_decode_utf8(raw_name);
    let media_dir = runtime.media_dir.clone();

    let res = tokio::task::spawn_blocking(move || {
        media::import_media_bytes(&media_dir, &file_name, &body)
    })
    .await;

    match res {
        Ok(Ok(imported)) => (
            StatusCode::OK,
            Json(json!({ "ok": true, "file": imported })),
        ),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e.to_string() })),
        ),
    }
}

fn parse_match_mode(s: &str) -> MatchMode {
    match s {
        "Exact" | "exact" => MatchMode::Exact,
        "Regex" | "regex" => MatchMode::Regex,
        _ => MatchMode::Contains,
    }
}

fn parse_effect_type(s: &str) -> MediaEffectType {
    match s {
        "Fixed" | "fixed" => MediaEffectType::Fixed,
        "Drop" | "drop" => MediaEffectType::Drop,
        "Confetti" | "confetti" => MediaEffectType::Confetti,
        "Throw" | "throw" => MediaEffectType::Throw,
        "Ambient" | "ambient" => MediaEffectType::Ambient,
        "Mod" | "mod" => MediaEffectType::Mod,
        _ => MediaEffectType::Cracker,
    }
}

fn parse_trigger_type(s: &str) -> MediaTriggerType {
    match s {
        "AnyComment" | "any_comment" => MediaTriggerType::AnyComment,
        "SuperChat" | "super_chat" => MediaTriggerType::SuperChat,
        "Membership" | "membership" => MediaTriggerType::Membership,
        "FirstTime" | "first_time" => MediaTriggerType::FirstTime,
        _ => MediaTriggerType::Keyword,
    }
}

fn parse_media_kind(s: &str) -> MediaKind {
    match s {
        "Video" | "video" => MediaKind::Video,
        "Audio" | "audio" => MediaKind::Audio,
        _ => MediaKind::Image,
    }
}

fn parse_anim_kind(s: &str) -> MediaAnimationKind {
    match s {
        "Fade" | "fade" => MediaAnimationKind::Fade,
        "SlideUp" | "slide_up" => MediaAnimationKind::SlideUp,
        "SlideLeft" | "slide_left" => MediaAnimationKind::SlideLeft,
        "Bounce" | "bounce" => MediaAnimationKind::Bounce,
        "ZoomSpin" | "zoom_spin" => MediaAnimationKind::ZoomSpin,
        "None" | "none" => MediaAnimationKind::None,
        _ => MediaAnimationKind::PopIn,
    }
}

fn parse_position(s: &str) -> MediaPosition {
    match s {
        "TopLeft" | "top_left" => MediaPosition::TopLeft,
        "TopRight" | "top_right" => MediaPosition::TopRight,
        "BottomLeft" | "bottom_left" => MediaPosition::BottomLeft,
        "BottomRight" | "bottom_right" => MediaPosition::BottomRight,
        "FullScreen" | "full_screen" => MediaPosition::FullScreen,
        _ => MediaPosition::Center,
    }
}

fn build_quick_media_preset(key: &str) -> MediaReactionRule {
    let mut base = MediaReactionRule {
        id: String::new(),
        name: "おめでとうクラッカー".to_string(),
        pattern: "おめでとう\nおめ\n祝".to_string(),
        match_mode: MatchMode::Contains,
        case_sensitive: false,
        effect_type: MediaEffectType::Cracker,
        trigger_type: MediaTriggerType::Keyword,
        min_superchat_amount: 0,
        priority: 20,
        media_kind: MediaKind::Image,
        stored_file_name: String::new(),
        original_file_name: String::new(),
        sound_stored_file_name: String::new(),
        sound_original_file_name: String::new(),
        sound_volume: 85,
        animation: MediaAnimationKind::PopIn,
        position: MediaPosition::Center,
        duration_ms: 4000,
        width_percent: 45,
        volume: 80,
        particle_count: 32,
        particle_size: 56,
        particle_speed: 105,
        gravity: 100,
        particle_text: "🎉,🎊,✨".to_string(),
        ambient_preset: "snow".to_string(),
        cooldown_sec: 2,
        exclude_banned: true,
        caption_template: String::new(),
        enabled: true,
        mod_id: String::new(),
        mod_effect_id: String::new(),
        mod_params_json: "{}".to_string(),
    };
    match key {
        "kusa" => {
            base.name = "草シャワー（落下物）".to_string();
            base.pattern = "草\nwww\nワロタ".to_string();
            base.effect_type = MediaEffectType::Drop;
            base.priority = 10;
            base.animation = MediaAnimationKind::Fade;
            base.duration_ms = 4500;
            base.particle_count = 22;
            base.particle_size = 60;
            base.particle_speed = 100;
            base.particle_text = "草,🌿,w".to_string();
        }
        "confetti" => {
            base.name = "888 紙吹雪".to_string();
            base.pattern = "888\nパチパチ\n👏".to_string();
            base.effect_type = MediaEffectType::Confetti;
            base.priority = 15;
            base.animation = MediaAnimationKind::Fade;
            base.duration_ms = 4800;
            base.particle_count = 55;
            base.particle_size = 28;
            base.particle_speed = 100;
            base.gravity = 90;
            base.particle_text = String::new();
            base.cooldown_sec = 3;
        }
        "throw" => {
            base.name = "ナイス投てき".to_string();
            base.pattern = "ナイス\n神プレイ\nすご".to_string();
            base.effect_type = MediaEffectType::Throw;
            base.priority = 12;
            base.duration_ms = 3200;
            base.particle_count = 6;
            base.particle_size = 88;
            base.particle_text = "🎯,⭐,👍".to_string();
        }
        "superchat_sakura" => {
            base.name = "スパチャ桜吹雪".to_string();
            base.pattern = String::new();
            base.effect_type = MediaEffectType::Ambient;
            base.trigger_type = MediaTriggerType::SuperChat;
            base.priority = 30;
            base.position = MediaPosition::TopRight;
            base.duration_ms = 6500;
            base.particle_count = 36;
            base.particle_size = 32;
            base.particle_speed = 100;
            base.particle_text = String::new();
            base.ambient_preset = "sakura".to_string();
            base.cooldown_sec = 1;
            base.caption_template = "{user} さんスパチャありがとう！".to_string();
        }
        "snow" => {
            base.name = "雪の環境演出".to_string();
            base.pattern = "雪\n寒い\n冬".to_string();
            base.effect_type = MediaEffectType::Ambient;
            base.priority = 5;
            base.animation = MediaAnimationKind::Fade;
            base.duration_ms = 6500;
            base.particle_count = 32;
            base.particle_size = 28;
            base.particle_speed = 95;
            base.particle_text = String::new();
            base.ambient_preset = "snow".to_string();
            base.cooldown_sec = 5;
        }
        _ => {}
    }
    base
}

/// Web リモコンからの各種操作コマンドを実行する (/api/remote/action)
pub async fn remote_action_handler(
    State(state): State<Arc<OverlaySharedState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if let Err(resp) = check_remote_auth(&state, addr.ip(), &headers, &query).await {
        return resp;
    }
    let runtime = match get_runtime() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
        }
    };

    let action = body["action"].as_str().unwrap_or("");
    let res: Result<Value, String> = match action {
        "post_chat" => {
            let text = body["text"].as_str().unwrap_or("");
            runtime
                .post_chat_message(text)
                .await
                .map(|msg| json!({ "message": msg }))
                .map_err(|e| e.to_string())
        }
        "clear_comments" => {
            runtime.recent_comments.write().await.clear();
            runtime.emit(crate::models::AppEvent::comments_replaced(Vec::new()));
            Ok(json!({ "message": "コメント一覧をクリアしました" }))
        }
        "pick_comment" => {
            let cid = body["comment_id"].as_str().unwrap_or("");
            let found = {
                let list = runtime.recent_comments.read().await;
                list.iter().rev().find(|c| c.id == cid).cloned()
            };
            match found {
                Some(c) => {
                    let already = runtime
                        .picked_comment
                        .lock()
                        .await
                        .comment
                        .as_ref()
                        .map(|p| p.id == cid)
                        .unwrap_or(false);
                    if already {
                        runtime.clear_picked_comment().await;
                        Ok(json!({ "message": "スポットテロップを解除しました" }))
                    } else {
                        runtime
                            .pick_comment(c)
                            .await
                            .map(|_| json!({ "message": "スポットテロップに表示しました" }))
                            .map_err(|e| e.to_string())
                    }
                }
                None => Err("対象のコメントが見つかりませんでした".to_string()),
            }
        }
        "pick_monetization" => {
            let id = body["id"].as_str().unwrap_or("");
            let list = runtime.list_monetization_events(150).await.unwrap_or_default();
            match list.into_iter().find(|it| it.id == id) {
                Some(it) => {
                    let item = crate::models::CommentItem {
                        id: it.id,
                        session_id: it.session_id,
                        platform: it.platform,
                        user_id: it.user_id,
                        user_name: it.user_name,
                        user_icon_url: it.user_icon_url,
                        user_memo: String::new(),
                        is_first_time: false,
                        message: it.message,
                        message_runs: it.message_runs,
                        timestamp_ms: it.timestamp_ms,
                        is_owner: false,
                        is_moderator: false,
                        is_member: true,
                        is_banned: false,
                        is_ng_masked: false,
                        hidden_from_overlay: false,
                        highlight_rule_id: None,
                        bubble_bg_color: None,
                        bubble_text_color: None,
                        bubble_border_color: None,
                        event_kind: Some(it.kind),
                        amount_display: Some(it.amount_display),
                        event_accent_color: Some(it.tier_color),
                    };
                    runtime
                        .pick_comment(item)
                        .await
                        .map(|_| json!({ "message": "エールをスポットテロップに表示しました" }))
                        .map_err(|e| e.to_string())
                }
                None => Err("対象のエール履歴が見つかりませんでした".to_string()),
            }
        }
        "clear_picked_comment" => {
            runtime.clear_picked_comment().await;
            Ok(json!({ "message": "スポットテロップ表示を解除しました" }))
        }
        "set_monetization_checked" => {
            let id = body["id"].as_str().unwrap_or("");
            let checked = body["checked"].as_bool().unwrap_or(true);
            runtime
                .set_monetization_checked(id, checked)
                .await
                .map(|_| json!({ "message": "お礼済み状態を更新しました" }))
                .map_err(|e| e.to_string())
        }
        "mark_all_monetization_checked" => runtime
            .mark_all_monetization_checked()
            .await
            .map(|_| json!({ "message": "すべてお礼済みにしました" }))
            .map_err(|e| e.to_string()),
        "start_poll" => {
            let cfg = runtime.get_poll_state().await.config;
            runtime
                .start_poll(cfg)
                .await
                .map(|_| json!({ "message": "ライブアンケートの受付を開始しました" }))
                .map_err(|e| e.to_string())
        }
        "stop_poll" => {
            runtime.stop_poll().await;
            Ok(json!({ "message": "ライブアンケートの受付を締め切りました" }))
        }
        "reset_poll" => {
            runtime.reset_poll().await;
            Ok(json!({ "message": "ライブアンケートをリセットしました" }))
        }
        "clear_overlay_poll" => {
            runtime.clear_overlay_poll().await;
            Ok(json!({ "message": "OBS ライブアンケートの表示をクリアしました" }))
        }
        "queue_toggle_accepting" => {
            let mut cfg = runtime.get_queue_state().await.config;
            cfg.is_accepting = body["accepting"].as_bool().unwrap_or(!cfg.is_accepting);
            runtime.save_queue_config(cfg).await;
            Ok(json!({ "message": "エントリーレーンの自動受付を切り替えました" }))
        }
        "queue_promote_next" => {
            runtime.queue_promote_next().await;
            Ok(json!({ "message": "空き枠へ待機者を繰り上げました" }))
        }
        "queue_rotate_active" => {
            runtime.queue_rotate_active().await;
            Ok(json!({ "message": "参加者を交代しました" }))
        }
        "queue_add_manual" => {
            let user_name = body["user_name"].as_str().unwrap_or("");
            let note = body["note"].as_str().unwrap_or("");
            runtime.queue_add_manual(user_name, note).await;
            Ok(json!({ "message": "待機列へ追加しました" }))
        }
        "queue_set_status" => {
            let user_id = body["user_id"].as_str().unwrap_or("");
            let status = match body["status"].as_str().unwrap_or("Waiting") {
                "Active" => QueueEntryStatus::Active,
                "Completed" => QueueEntryStatus::Completed,
                _ => QueueEntryStatus::Waiting,
            };
            runtime.queue_set_status(user_id, status).await;
            Ok(json!({ "message": "参加ステータスを更新しました" }))
        }
        "queue_move_entry" => {
            let user_id = body["user_id"].as_str().unwrap_or("");
            let delta = body["delta"].as_i64().unwrap_or(0) as i32;
            runtime.queue_move_entry(user_id, delta).await;
            Ok(json!({ "message": "順番を変更しました" }))
        }
        "queue_remove_entry" => {
            let user_id = body["user_id"].as_str().unwrap_or("");
            runtime.queue_remove_entry(user_id).await;
            Ok(json!({ "message": "エントリーレーンから削除しました" }))
        }
        "test_media_rule" => {
            let id = body["id"].as_str().unwrap_or("");
            runtime
                .test_media_reaction_rule(id)
                .await
                .map(|_| json!({ "message": "リアクションステージ演出を発動しました" }))
                .map_err(|e| e.to_string())
        }
        "save_media_rule" => {
            let r = &body["rule"];
            let rule_id = r["id"].as_str().unwrap_or("").trim().to_string();
            let effect_type = parse_effect_type(r["effect_type"].as_str().unwrap_or("Cracker"));
            let stored_file_name = r["stored_file_name"].as_str().unwrap_or("").trim().to_string();
            let sound_stored_file_name = r["sound_stored_file_name"].as_str().unwrap_or("").trim().to_string();
            if !stored_file_name.is_empty() && !media::is_safe_stored_file_name(&stored_file_name) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "メディアファイル名が不正です" })),
                );
            }
            if !sound_stored_file_name.is_empty()
                && !media::is_safe_stored_file_name(&sound_stored_file_name)
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "効果音ファイル名が不正です" })),
                );
            }
            let caption_template = r["caption_template"].as_str().unwrap_or("").trim().to_string();
            if effect_type == MediaEffectType::Fixed
                && stored_file_name.is_empty()
                && sound_stored_file_name.is_empty()
                && caption_template.is_empty()
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "ok": false,
                        "error": "固定表示では画像・動画・音声ファイルまたはテロップを設定してください"
                    })),
                );
            }
            // 既存ルールの部分更新時に MOD 演出設定が欠落送信されても破壊されないよう既存値を保持する
            let existing_rule = if !rule_id.is_empty() {
                runtime
                    .list_media_reaction_rules()
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .find(|x| x.id == rule_id)
            } else {
                None
            };
            let mod_id = match r.get("mod_id").and_then(|v| v.as_str()) {
                Some(s) => s.trim().to_string(),
                None => existing_rule
                    .as_ref()
                    .map(|x| x.mod_id.clone())
                    .unwrap_or_default(),
            };
            let mod_effect_id = match r.get("mod_effect_id").and_then(|v| v.as_str()) {
                Some(s) => s.trim().to_string(),
                None => existing_rule
                    .as_ref()
                    .map(|x| x.mod_effect_id.clone())
                    .unwrap_or_default(),
            };
            let mod_params_json = match r.get("mod_params_json").and_then(|v| v.as_str()) {
                Some(s) => {
                    let t = s.trim();
                    if t.is_empty() { "{}".to_string() } else { t.to_string() }
                }
                None => existing_rule
                    .as_ref()
                    .map(|x| x.mod_params_json.clone())
                    .unwrap_or_else(|| "{}".to_string()),
            };
            let pattern = r["pattern"].as_str().unwrap_or("").trim().to_string();
            let raw_name = r["name"].as_str().unwrap_or("").trim().to_string();
            let name = if !raw_name.is_empty() {
                raw_name
            } else {
                pattern
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty())
                    .unwrap_or("リアクションステージ演出")
                    .to_string()
            };
            let rule = MediaReactionRule {
                id: rule_id,
                name,
                pattern,
                match_mode: parse_match_mode(r["match_mode"].as_str().unwrap_or("Contains")),
                case_sensitive: r["case_sensitive"].as_bool().unwrap_or(false),
                effect_type,
                trigger_type: parse_trigger_type(r["trigger_type"].as_str().unwrap_or("Keyword")),
                min_superchat_amount: r["min_superchat_amount"].as_u64().unwrap_or(0),
                priority: r["priority"].as_i64().unwrap_or(10).clamp(-999, 999) as i32,
                media_kind: parse_media_kind(r["media_kind"].as_str().unwrap_or("Image")),
                stored_file_name,
                original_file_name: r["original_file_name"].as_str().unwrap_or("").trim().to_string(),
                sound_stored_file_name,
                sound_original_file_name: r["sound_original_file_name"].as_str().unwrap_or("").trim().to_string(),
                sound_volume: r["sound_volume"].as_u64().unwrap_or(85).clamp(0, 100) as u32,
                animation: parse_anim_kind(r["animation"].as_str().unwrap_or("PopIn")),
                position: parse_position(r["position"].as_str().unwrap_or("Center")),
                duration_ms: r["duration_ms"].as_u64().unwrap_or(4000).min(120_000) as u32,
                width_percent: r["width_percent"].as_u64().unwrap_or(45).clamp(10, 100) as u32,
                volume: r["volume"].as_u64().unwrap_or(80).clamp(0, 100) as u32,
                particle_count: r["particle_count"].as_u64().unwrap_or(28).clamp(1, 120) as u32,
                particle_size: r["particle_size"].as_u64().unwrap_or(64).clamp(20, 260) as u32,
                particle_speed: r["particle_speed"].as_u64().unwrap_or(100).clamp(30, 250) as u32,
                gravity: r["gravity"].as_u64().unwrap_or(100).clamp(0, 200) as u32,
                particle_text: r["particle_text"].as_str().unwrap_or("").trim().to_string(),
                ambient_preset: r["ambient_preset"].as_str().unwrap_or("snow").trim().to_string(),
                cooldown_sec: r["cooldown_sec"].as_u64().unwrap_or(3).min(3600) as u32,
                exclude_banned: r["exclude_banned"].as_bool().unwrap_or(true),
                caption_template,
                enabled: r["enabled"].as_bool().unwrap_or(true),
                mod_id,
                mod_effect_id,
                mod_params_json,
            };
            let is_new = rule.id.is_empty();
            runtime
                .save_media_reaction_rule(rule)
                .await
                .map(|list| {
                    json!({
                        "message": if is_new { "リアクションステージ演出を追加しました" } else { "リアクションステージ演出を更新しました" },
                        "media_rules": list
                    })
                })
                .map_err(|e| e.to_string())
        }
        "add_media_preset" => {
            let key = body["preset"].as_str().unwrap_or("cracker");
            let rule = build_quick_media_preset(key);
            let title = rule.name.clone();
            runtime
                .save_media_reaction_rule(rule)
                .await
                .map(|list| {
                    json!({
                        "message": format!("定番テンプレート「{}」を追加しました", title),
                        "media_rules": list
                    })
                })
                .map_err(|e| e.to_string())
        }
        "toggle_media_rule" => {
            let id = body["id"].as_str().unwrap_or("");
            let enabled = body["enabled"].as_bool().unwrap_or(true);
            let existing = runtime
                .list_media_reaction_rules()
                .await
                .unwrap_or_default()
                .into_iter()
                .find(|r| r.id == id);
            match existing {
                Some(mut rule) => {
                    rule.enabled = enabled;
                    runtime
                        .save_media_reaction_rule(rule)
                        .await
                        .map(|list| {
                            json!({
                                "message": if enabled { "演出を有効にしました" } else { "演出を無効にしました" },
                                "media_rules": list
                            })
                        })
                        .map_err(|e| e.to_string())
                }
                None => Err("対象の演出ルールが見つかりませんでした".to_string()),
            }
        }
        "delete_media_rule" => {
            let id = body["id"].as_str().unwrap_or("");
            runtime
                .delete_media_reaction_rule(id)
                .await
                .map(|list| {
                    json!({
                        "message": "リアクションステージ演出を削除しました",
                        "media_rules": list
                    })
                })
                .map_err(|e| e.to_string())
        }
        "save_media_config" => {
            let c = &body["config"];
            let mode = match c["playback_mode"].as_str().unwrap_or("Overlay") {
                "Queue" | "queue" => MediaPlaybackMode::Queue,
                "Replace" | "replace" => MediaPlaybackMode::Replace,
                _ => MediaPlaybackMode::Overlay,
            };
            let cfg = MediaReactionConfig {
                playback_mode: mode,
                master_volume: c["master_volume"].as_u64().unwrap_or(80).clamp(0, 100) as u32,
                max_concurrent: c["max_concurrent"].as_u64().unwrap_or(8).clamp(1, 12) as u32,
                allow_multiple_match: c["allow_multiple_match"].as_bool().unwrap_or(true),
            };
            runtime
                .save_media_reaction_config(cfg)
                .await
                .map(|saved| {
                    json!({
                        "message": "リアクションステージ全体設定を保存しました",
                        "media_config": saved
                    })
                })
                .map_err(|e| e.to_string())
        }
        "set_media_bg_color" => {
            let raw = body["bg_color"].as_str().unwrap_or("transparent").trim();
            let normalized = if raw.is_empty() || raw.eq_ignore_ascii_case("transparent") {
                "transparent".to_string()
            } else {
                let hex = raw.trim_start_matches('#');
                if (hex.len() == 6 || hex.len() == 8)
                    && hex.chars().all(|c| c.is_ascii_hexdigit())
                {
                    format!("#{}", hex.to_ascii_uppercase())
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "ok": false,
                            "error": "背景色は transparent または #RRGGBB / #RRGGBBAA 形式で指定してください"
                        })),
                    );
                }
            };
            let mut oc = runtime.overlay_state.config.read().await.clone();
            oc.media_bg_color = normalized;
            runtime
                .save_overlay_config_and_restart(oc)
                .await
                .map(|saved| {
                    json!({
                        "message": "リアクションステージの背景色を更新しました",
                        "overlay_config": saved
                    })
                })
                .map_err(|e| e.to_string())
        }
        "save_poll_config" => {
            let c = &body["config"];
            let question = c["question"].as_str().unwrap_or("").trim().to_string();
            let raw_opts = c["options"].as_array().cloned().unwrap_or_default();
            let default_colors = ["#3B82F6", "#22C55E", "#F59E0B", "#EC4899", "#A855F7", "#06B6D4"];
            let options: Vec<PollOption> = raw_opts
                .iter()
                .enumerate()
                .filter_map(|(i, o)| {
                    let label = o["label"].as_str().unwrap_or("").trim().to_string();
                    if label.is_empty() {
                        return None;
                    }
                    let keywords: Vec<String> = o["keywords"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                                .filter(|s| !s.is_empty())
                                .collect()
                        })
                        .filter(|v: &Vec<String>| !v.is_empty())
                        .unwrap_or_else(|| vec![(i + 1).to_string(), label.clone()]);
                    Some(PollOption {
                        id: o["id"]
                            .as_str()
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("opt-{}", i + 1)),
                        label,
                        keywords,
                        color: sanitize_hex_color(
                            o["color"].as_str(),
                            default_colors[i % default_colors.len()],
                        ),
                        vote_count: o["vote_count"].as_u64().unwrap_or(0),
                    })
                })
                .collect();
            if options.len() < 2 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "選択肢は2つ以上入力してください" })),
                );
            }
            let cur = runtime.get_poll_state().await.config;
            let cfg = PollConfig {
                question: if question.is_empty() { cur.question } else { question },
                options,
                allow_vote_change: c["allow_vote_change"].as_bool().unwrap_or(cur.allow_vote_change),
                exclude_banned: c["exclude_banned"].as_bool().unwrap_or(cur.exclude_banned),
                show_percentage: c["show_percentage"].as_bool().unwrap_or(cur.show_percentage),
                show_vote_count: c["show_vote_count"].as_bool().unwrap_or(cur.show_vote_count),
            };
            runtime
                .save_poll_config(cfg)
                .await
                .map(|state| json!({ "message": "ライブアンケート設定を保存しました", "poll": state }))
                .map_err(|e| e.to_string())
        }
        "save_word_counter" => {
            let c = &body["counter"];
            let name = c["name"].as_str().unwrap_or("").trim().to_string();
            if name.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "カウンター名を入力してください" })),
                );
            }
            let keywords: Vec<String> = c["keywords"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if keywords.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "対象キーワードを1つ以上入力してください" })),
                );
            }
            let counter = WordCounterConfig {
                id: c["id"].as_str().unwrap_or("").trim().to_string(),
                name,
                keywords,
                match_mode: parse_match_mode(c["match_mode"].as_str().unwrap_or("Contains")),
                case_sensitive: c["case_sensitive"].as_bool().unwrap_or(false),
                max_per_account: c["max_per_account"].as_u64().map(|v| v.max(1) as u32),
                exclude_banned: c["exclude_banned"].as_bool().unwrap_or(true),
                accent_color: sanitize_hex_color(c["accent_color"].as_str(), "#3B82F6"),
                enabled: c["enabled"].as_bool().unwrap_or(true),
                total_count: c["total_count"].as_u64().unwrap_or(0),
                unique_user_count: c["unique_user_count"].as_u64().unwrap_or(0),
            };
            runtime
                .save_word_counter(counter)
                .await
                .map(|counters| json!({ "message": "コールカウンターを保存しました", "counters": counters }))
                .map_err(|e| e.to_string())
        }
        "delete_word_counter" => {
            let id = body["id"].as_str().unwrap_or("");
            runtime
                .delete_word_counter(id)
                .await
                .map(|counters| json!({ "message": "コールカウンターを削除しました", "counters": counters }))
                .map_err(|e| e.to_string())
        }
        "save_highlight_rule" => {
            let r = &body["rule"];
            let pattern = r["pattern"].as_str().unwrap_or("").trim().to_string();
            if pattern.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": "反応ワードを入力してください" })),
                );
            }
            let rule = HighlightRule {
                id: r["id"].as_str().unwrap_or("").trim().to_string(),
                name: r["name"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(&pattern)
                    .to_string(),
                pattern,
                match_mode: parse_match_mode(r["match_mode"].as_str().unwrap_or("Contains")),
                case_sensitive: r["case_sensitive"].as_bool().unwrap_or(false),
                bg_color: sanitize_hex_color(r["bg_color"].as_str(), "#FEF08A"),
                text_color: sanitize_hex_color(r["text_color"].as_str(), "#18181B"),
                border_color: sanitize_hex_color(r["border_color"].as_str(), "#EAB308"),
                priority: r["priority"].as_i64().unwrap_or(10) as i32,
                enabled: r["enabled"].as_bool().unwrap_or(true),
            };
            runtime
                .save_highlight_rule(rule)
                .await
                .map(|list| json!({ "message": "ハイライトサインルールを保存しました", "highlight_rules": list }))
                .map_err(|e| e.to_string())
        }
        "delete_highlight_rule" => {
            let id = body["id"].as_str().unwrap_or("");
            runtime
                .delete_highlight_rule(id)
                .await
                .map(|list| json!({ "message": "ハイライトサインルールを削除しました", "highlight_rules": list }))
                .map_err(|e| e.to_string())
        }
        "clear_overlay_media" => {
            runtime.clear_overlay_media();
            Ok(json!({ "message": "再生中の演出を停止しました" }))
        }
        "adjust_word_counter" => {
            let id = body["id"].as_str().unwrap_or("");
            let delta = body["delta"].as_i64().unwrap_or(0);
            runtime
                .adjust_word_counter(id, delta)
                .await
                .map(|_| json!({ "message": "カウンターを更新しました" }))
                .map_err(|e| e.to_string())
        }
        "reset_word_counter" => {
            let id = body["id"].as_str().unwrap_or("");
            runtime
                .reset_word_counter(id)
                .await
                .map(|_| json!({ "message": "カウンターをリセットしました" }))
                .map_err(|e| e.to_string())
        }
        "adjust_first_time" => {
            let delta = body["delta"].as_i64().unwrap_or(0);
            runtime
                .adjust_first_time_count(delta)
                .await
                .map(|_| json!({ "message": "初見カウントを更新しました" }))
                .map_err(|e| e.to_string())
        }
        "reset_first_time_session" => runtime
            .reset_first_time_session_count()
            .await
            .map(|_| json!({ "message": "本日の初見カウントを 0 にリセットしました" }))
            .map_err(|e| e.to_string()),
        "draw_user_lottery" => {
            let current_only = body["current_session_only"].as_bool().unwrap_or(true);
            let winner_count = body["winner_count"].as_u64().unwrap_or(1).clamp(1, 20) as u32;
            let repeat_mode = if body["repeat_mode"].as_str() == Some("AllowMultiple") {
                LotteryRepeatMode::AllowMultiple
            } else {
                LotteryRepeatMode::OnceOnly
            };
            runtime
                .draw_user_lottery(UserLotteryRequest {
                    current_session_only: current_only,
                    min_watch_count: 0,
                    min_comment_count: 0,
                    repeat_mode,
                    winner_count,
                })
                .await
                .map(|res| {
                    let names: Vec<String> =
                        res.winners.iter().map(|w| w.user_name.clone()).collect();
                    json!({
                        "message": format!("当選: {}", names.join(", ")),
                        "result": res
                    })
                })
                .map_err(|e| e.to_string())
        }
        "reset_user_lottery_winners" => {
            runtime.user_lottery_won_ids.lock().await.clear();
            runtime.emit(AppEvent::user_lottery_winners_updated(Vec::new()));
            Ok(json!({ "message": "既当選リストをリセットしました" }))
        }
        "clear_overlay_lottery" => {
            runtime.overlay_state.clear_lottery().await;
            Ok(json!({ "message": "OBS ドローボックスの表示をクリアしました" }))
        }
        "start_comment_lottery" => {
            let cfg = runtime.comment_lottery.lock().await.config.clone();
            runtime
                .start_comment_lottery(cfg)
                .await
                .map(|_| json!({ "message": "コメント抽選の受付を開始しました" }))
                .map_err(|e| e.to_string())
        }
        "stop_comment_lottery" => runtime
            .stop_comment_lottery()
            .await
            .map(|_| json!({ "message": "コメント抽選の受付を終了しました" }))
            .map_err(|e| e.to_string()),
        "draw_comment_lottery" => runtime
            .draw_comment_lottery()
            .await
            .map(|res| {
                let names: Vec<String> = res.winners.iter().map(|w| w.user_name.clone()).collect();
                json!({
                    "message": format!("当選: {}", names.join(", ")),
                    "result": res
                })
            })
            .map_err(|e| e.to_string()),
        "set_user_memo" => {
            let user_id = body["user_id"].as_str().unwrap_or("");
            let platform = PlatformKind::from_str_lossy(body["platform"].as_str().unwrap_or("youtube"));
            let memo = body["memo"].as_str().unwrap_or("");
            let db_res = {
                let db = runtime.db.lock().await;
                db.set_user_memo(user_id, platform, memo)
                    .map_err(|e| e.to_string())
            };
            match db_res {
                Ok(_) => {
                    runtime.refresh_memo_in_recent_comments(user_id, memo).await;
                    runtime.emit(crate::models::AppEvent::users_updated());
                    Ok(json!({ "message": "名前メモを保存しました" }))
                }
                Err(e) => Err(e),
            }
        }
        "set_feature_flag" => {
            let flag = body["flag"].as_str().unwrap_or("");
            let enabled = body["enabled"].as_bool().unwrap_or(true);
            let mut flags = runtime.feature_flags.read().await.clone();
            match flag {
                "comment_display" => flags.comment_display_enabled = enabled,
                "word_highlight" => flags.word_highlight_enabled = enabled,
                "word_counter" => flags.word_counter_enabled = enabled,
                "first_time_counter" => flags.first_time_counter_enabled = enabled,
                "superchat" => flags.superchat_enabled = enabled,
                "poll" => flags.poll_enabled = enabled,
                "pickup" => flags.pickup_enabled = enabled,
                "queue" => flags.queue_enabled = enabled,
                "media_reaction" => flags.media_reaction_enabled = enabled,
                "user_lottery" => flags.user_lottery_enabled = enabled,
                "comment_lottery" => flags.comment_lottery_enabled = enabled,
                "user_memo_display" => flags.user_memo_display_enabled = enabled,
                _ => return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "不明な機能フラグです" }))),
            }
            runtime
                .update_feature_flags(flags)
                .await
                .map(|_| json!({ "message": "機能の ON/OFF を更新しました" }))
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("未対応のアクションです: {}", action)),
    };

    match res {
        Ok(data) => (StatusCode::OK, Json(json!({ "ok": true, "data": data }))),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": err })),
        ),
    }
}

/// Web リモコン用リアルタイム通知 WebSocket (/ws/remote)
pub async fn remote_ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<OverlaySharedState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if let Err((status, body)) = check_remote_auth(&state, addr.ip(), &headers, &query).await {
        return (status, body).into_response();
    }
    ws.on_upgrade(move |socket| handle_remote_socket(socket, state))
}

async fn handle_remote_socket(socket: WebSocket, state: Arc<OverlaySharedState>) {
    let runtime = match get_runtime() {
        Ok(r) => r,
        Err(_) => return,
    };
    let (mut sender, mut receiver) = socket.split();
    let mut rx = runtime.event_tx.subscribe();
    let mut auth_rx = state.remote_auth_gen.subscribe();
    // 接続直後の現在世代を記録し、以降の PIN 変更・無効化・LAN 設定変更を検知したら即時切断する
    let _ = auth_rx.borrow_and_update();

    // 単一タスクの select! ループで残留タスクを防止しつつ、バースト時の Lagged でも切断しない
    loop {
        tokio::select! {
            recv = rx.recv() => {
                match recv {
                    Ok(ev) => {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            auth_changed = auth_rx.changed() => {
                let _ = auth_changed;
                let _ = sender.send(Message::Close(None)).await;
                break;
            }
        }
    }
}

/// Stream Deck / Loupedeck / ローカル Webhook 向けワンクリック HTTP トリガー
/// 例:
///   GET http://127.0.0.1:18765/api/trigger/clear_pickup
///   GET http://127.0.0.1:18765/api/trigger/media?id=<rule_id>
///   GET http://127.0.0.1:18765/api/trigger/hotkey?id=<binding_id>
///   GET http://127.0.0.1:18765/api/trigger/obs_scene?scene=<scene_name>
pub async fn stream_deck_trigger_handler(
    Path(action): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<OverlaySharedState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // 外部サイト経由の CSRF やオーバーレイ／MOD ページからの自動発火を遮断（Stream Deck ネイティブクライアントは透過許可）
    if is_cross_site_browser_request(&headers, true) || is_overlay_or_mod_referer(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": "ブラウザの外部サイトやオーバーレイページからのトリガー実行は許可されていません" })),
        );
    }
    // 同一 PC（127.0.0.1 / ::1）からの Stream Deck 呼び出しは直接許可。LAN 端末からは PIN 認証を要求
    if !addr.ip().is_loopback() {
        if let Err(err) = check_remote_auth(&state, addr.ip(), &headers, &query).await {
            return err;
        }
    }

    let runtime = match get_runtime() {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
        }
    };

    let id = query.get("id").cloned().unwrap_or_default();
    let target = query
        .get("target")
        .or_else(|| query.get("scene"))
        .or_else(|| query.get("feature"))
        .cloned()
        .unwrap_or_else(|| id.clone());
    let secondary = query
        .get("source")
        .or_else(|| query.get("secondary"))
        .cloned()
        .unwrap_or_default();

    let res = match action.trim().to_ascii_lowercase().as_str() {
        "hotkey" => runtime.execute_hotkey_by_id(&id).await,
        "clear_pickup" => {
            runtime
                .execute_automation_action(AutomationActionKind::ClearPickup, "", "")
                .await
        }
        "pick_latest" => {
            runtime
                .execute_automation_action(AutomationActionKind::PickLatestComment, "", "")
                .await
        }
        "media" | "trigger_media" => {
            runtime
                .execute_automation_action(AutomationActionKind::TriggerMediaRule, &target, "")
                .await
        }
        "clear_media" => {
            runtime
                .execute_automation_action(AutomationActionKind::ClearMedia, "", "")
                .await
        }
        "end_poll" => {
            runtime
                .execute_automation_action(AutomationActionKind::EndPoll, "", "")
                .await
        }
        "advance_queue" => {
            runtime
                .execute_automation_action(AutomationActionKind::AdvanceQueue, "", "")
                .await
        }
        "reset_first_time" => {
            runtime
                .execute_automation_action(AutomationActionKind::ResetFirstTime, "", "")
                .await
        }
        "toggle_feature" => {
            runtime
                .execute_automation_action(AutomationActionKind::ToggleFeature, &target, "")
                .await
        }
        "obs_scene" => {
            runtime
                .execute_automation_action(AutomationActionKind::ObsSwitchScene, &target, "")
                .await
        }
        "obs_source" => {
            runtime
                .execute_automation_action(
                    AutomationActionKind::ObsToggleSource,
                    &target,
                    &secondary,
                )
                .await
        }
        "apply_profile" => runtime
            .apply_stream_profile(&target)
            .await
            .map(|_| "配信プロファイルを切り替えました".to_string()),
        _ => Err(anyhow::anyhow!("不明なトリガーアクションです: {}", action)),
    };

    match res {
        Ok(msg) => (StatusCode::OK, Json(json!({ "ok": true, "message": msg }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_constant_time_eq_and_hex_color_sanitize() {
        assert!(constant_time_eq("123456", "123456"));
        assert!(!constant_time_eq("123456", "123457"));
        assert!(!constant_time_eq("123456", "12345"));

        assert_eq!(sanitize_hex_color(Some("#abc"), "#000000"), "#ABC");
        assert_eq!(sanitize_hex_color(Some("3b82f6"), "#000000"), "#3B82F6");
        assert_eq!(
            sanitize_hex_color(Some("red;background:url(evil)"), "#3B82F6"),
            "#3B82F6"
        );
    }

    #[test]
    fn test_cross_site_browser_detection_preserves_stream_deck() {
        // Stream Deck などネイティブクライアント（Host のみ）は許可
        let mut native = HeaderMap::new();
        native.insert(header::HOST, HeaderValue::from_static("127.0.0.1:18765"));
        assert!(!is_cross_site_browser_request(&native, true));

        // 同一オリジンブラウザ要求は許可
        let mut same = HeaderMap::new();
        same.insert(header::HOST, HeaderValue::from_static("127.0.0.1:18765"));
        same.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:18765"),
        );
        same.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        assert!(!is_cross_site_browser_request(&same, true));

        // 外部サイトからのクロスオリジン要求は拒否
        let mut cross = HeaderMap::new();
        cross.insert(header::HOST, HeaderValue::from_static("127.0.0.1:18765"));
        cross.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        cross.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(is_cross_site_browser_request(&cross, true));

        // 外部 Referer（<img src=...> CSRF）も拒否
        let mut img_csrf = HeaderMap::new();
        img_csrf.insert(header::HOST, HeaderValue::from_static("127.0.0.1:18765"));
        img_csrf.insert(
            header::REFERER,
            HeaderValue::from_static("https://evil.example/page"),
        );
        assert!(is_cross_site_browser_request(&img_csrf, true));

        // オーバーレイ／MOD ページからのトリガー呼び出しも拒否
        let mut mod_ref = HeaderMap::new();
        mod_ref.insert(
            header::REFERER,
            HeaderValue::from_static("http://127.0.0.1:18765/overlay/mod/foo/bar"),
        );
        assert!(is_overlay_or_mod_referer(&mod_ref));
    }
}
