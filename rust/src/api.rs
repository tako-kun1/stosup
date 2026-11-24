use chrono::Utc;
use flutter_rust_bridge::frb;

use crate::engine::lottery;
use crate::frb_generated::StreamSink;
pub use crate::models::*;
use crate::platform::{demo, youtube_oauth};
use crate::state::{get_runtime, initialize_runtime};
use crate::updater;

/// コアランタイム初期化（アプリ起動時に呼び出す）
pub async fn init_app(after_update: bool) -> Result<(), String> {
    initialize_runtime(after_update)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// 現在のアプリバージョンを取得
#[frb(sync)]
pub fn get_app_version() -> String {
    updater::CURRENT_APP_VERSION.to_string()
}

/// 既定ブラウザでURLを開く
pub fn open_external_url(url: String) -> Result<(), String> {
    youtube_oauth::open_url_in_browser(&url).map_err(|e| e.to_string())
}

/// Rust からのリアルタイムイベントを Flutter 側で購読する
pub async fn subscribe_events(sink: StreamSink<AppEvent>) -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let mut rx = runtime.event_tx.subscribe();

    while let Ok(ev) = rx.recv().await {
        if sink.add(ev).is_err() {
            break;
        }
    }
    Ok(())
}

// ==================== 配信接続・コメント ====================

pub async fn connect_youtube(
    url_or_video_id: String,
    mode: ConnectionMode,
) -> Result<ConnectionStateDto, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .connect_youtube(&url_or_video_id, mode)
        .await
        .map_err(|e| e.to_string())
}

pub async fn connect_demo(
    session_name: String,
    auto_generate: bool,
) -> Result<ConnectionStateDto, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .connect_demo(&session_name, auto_generate)
        .await
        .map_err(|e| e.to_string())
}

pub async fn disconnect_stream() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.disconnect_current().await;
    Ok(())
}

pub async fn get_connection_state() -> Result<ConnectionStateDto, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let state = runtime.connection_state.read().await.clone();
    Ok(state)
}

pub async fn get_recent_comments() -> Result<Vec<CommentItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let comments = runtime.recent_comments.read().await.clone();
    Ok(comments)
}

pub async fn clear_recent_comments() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.recent_comments.write().await.clear();
    Ok(())
}

/// デモ・テスト用に任意のユーザー名・コメントを手動注入する
pub async fn send_demo_comment(
    user_id: String,
    user_name: String,
    message: String,
) -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let uid = if user_id.trim().is_empty() {
        format!("manual-{}", user_name.trim())
    } else {
        user_id.trim().to_string()
    };
    let name = if user_name.trim().is_empty() {
        "テスト視聴者".to_string()
    } else {
        user_name.trim().to_string()
    };
    let initial = name.chars().next().unwrap_or('T').to_string();

    let raw = RawIncomingComment {
        id: uuid::Uuid::new_v4().to_string(),
        platform: PlatformKind::Demo,
        user_id: uid,
        user_name: name,
        user_icon_url: demo::make_svg_avatar("%2338bdf8", &initial),
        message,
        timestamp_ms: Utc::now().timestamp_millis(),
        is_owner: false,
        is_moderator: false,
        is_member: false,
    };
    runtime
        .comment_tx
        .send(raw)
        .map_err(|e| format!("コメント送信エラー: {}", e))
}

pub async fn post_chat_message(text: String) -> Result<String, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .post_chat_message(&text)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 特定ワード反応（吹き出し色変更） ====================

pub async fn list_highlight_rules() -> Result<Vec<HighlightRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.list_highlight_rules().await.map_err(|e| e.to_string())
}

pub async fn save_highlight_rule(mut rule: HighlightRule) -> Result<Vec<HighlightRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    if rule.id.trim().is_empty() {
        rule.id = uuid::Uuid::new_v4().to_string();
    }
    let db = runtime.db.lock().await;
    db.upsert_highlight_rule(&rule).map_err(|e| e.to_string())?;
    db.list_highlight_rules().map_err(|e| e.to_string())
}

pub async fn delete_highlight_rule(id: String) -> Result<Vec<HighlightRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    db.delete_highlight_rule(&id).map_err(|e| e.to_string())?;
    db.list_highlight_rules().map_err(|e| e.to_string())
}

// ==================== 特定ワードカウンター ====================

pub async fn list_word_counters() -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.list_word_counters().await.map_err(|e| e.to_string())
}

pub async fn save_word_counter(
    mut counter: WordCounterConfig,
) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    if counter.id.trim().is_empty() {
        counter.id = uuid::Uuid::new_v4().to_string();
    }
    let list = {
        let db = runtime.db.lock().await;
        db.upsert_word_counter(&counter).map_err(|e| e.to_string())?;
        db.list_word_counters().map_err(|e| e.to_string())?
    };
    runtime.overlay_state.update_counters(list.clone()).await;
    runtime.emit(AppEvent::counters_updated(list.clone()));
    Ok(list)
}

pub async fn delete_word_counter(id: String) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let list = {
        let db = runtime.db.lock().await;
        db.delete_word_counter(&id).map_err(|e| e.to_string())?;
        db.list_word_counters().map_err(|e| e.to_string())?
    };
    runtime.overlay_state.update_counters(list.clone()).await;
    runtime.emit(AppEvent::counters_updated(list.clone()));
    Ok(list)
}

pub async fn reset_word_counter(id: String) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let list = {
        let db = runtime.db.lock().await;
        db.reset_word_counter(&id).map_err(|e| e.to_string())?;
        db.list_word_counters().map_err(|e| e.to_string())?
    };
    runtime.overlay_state.update_counters(list.clone()).await;
    runtime.emit(AppEvent::counters_updated(list.clone()));
    Ok(list)
}

pub async fn adjust_word_counter(
    id: String,
    delta: i64,
) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let list = {
        let db = runtime.db.lock().await;
        db.adjust_word_counter(&id, delta).map_err(|e| e.to_string())?;
        db.list_word_counters().map_err(|e| e.to_string())?
    };
    runtime.overlay_state.update_counters(list.clone()).await;
    runtime.emit(AppEvent::counters_updated(list.clone()));
    Ok(list)
}

// ==================== ユーザーリスト & ローカルBAN ====================

pub async fn list_users() -> Result<Vec<UserRecord>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.list_users().await.map_err(|e| e.to_string())
}

pub async fn set_user_ban(
    user_id: String,
    platform: PlatformKind,
    banned: bool,
    reason: String,
) -> Result<Vec<UserRecord>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    {
        let db = runtime.db.lock().await;
        db.set_user_ban(&user_id, platform, banned, &reason)
            .map_err(|e| e.to_string())?;
    }
    // BANした場合、進行中のコメント抽選エントリーからも即座に除外してUI同期
    if banned {
        let mut cl = runtime.comment_lottery.lock().await;
        let before = cl.entries.len();
        cl.entries.retain(|e| e.user_id != user_id);
        if cl.entries.len() != before {
            let state = cl.clone();
            drop(cl);
            runtime.emit(AppEvent::comment_lottery_updated(state));
        }
    }
    runtime.emit(AppEvent::users_updated());
    runtime.list_users().await.map_err(|e| e.to_string())
}

pub async fn set_user_memo(
    user_id: String,
    platform: PlatformKind,
    memo: String,
) -> Result<Vec<UserRecord>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    {
        let db = runtime.db.lock().await;
        db.set_user_memo(&user_id, platform, &memo)
            .map_err(|e| e.to_string())?;
    }
    runtime.emit(AppEvent::users_updated());
    runtime.list_users().await.map_err(|e| e.to_string())
}

// ==================== ユーザー抽選 ====================

pub async fn draw_user_lottery(req: UserLotteryRequest) -> Result<LotteryResult, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .draw_user_lottery(req)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_user_lottery_won_ids() -> Result<Vec<String>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let won = runtime.user_lottery_won_ids.lock().await;
    Ok(won.iter().cloned().collect())
}

pub async fn reset_user_lottery_winners() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.user_lottery_won_ids.lock().await.clear();
    Ok(())
}

pub async fn clear_overlay_lottery() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.overlay_state.clear_lottery().await;
    Ok(())
}

pub async fn list_lottery_history(limit: u32) -> Result<Vec<LotteryResult>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    db.list_lottery_history(limit as usize)
        .map_err(|e| e.to_string())
}

// ==================== コメント抽選 ====================

pub async fn get_comment_lottery_state() -> Result<CommentLotteryState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let state = runtime.comment_lottery.lock().await.clone();
    Ok(state)
}

pub async fn update_comment_lottery_config(
    config: CommentLotteryConfig,
) -> Result<CommentLotteryState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let state = {
        let mut cl = runtime.comment_lottery.lock().await;
        lottery::update_comment_lottery_config(&mut cl, config);
        cl.clone()
    };
    runtime.emit(AppEvent::comment_lottery_updated(state.clone()));
    Ok(state)
}

pub async fn start_comment_lottery(
    config: CommentLotteryConfig,
) -> Result<CommentLotteryState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .start_comment_lottery(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn stop_comment_lottery() -> Result<CommentLotteryState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .stop_comment_lottery()
        .await
        .map_err(|e| e.to_string())
}

pub async fn draw_comment_lottery() -> Result<LotteryResult, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .draw_comment_lottery()
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_comment_lottery(clear_winners_only: bool) -> Result<CommentLotteryState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let state = {
        let mut cl = runtime.comment_lottery.lock().await;
        cl.already_won_user_ids.clear();
        if !clear_winners_only {
            cl.is_accepting = false;
            cl.entries.clear();
            cl.started_at_ms = None;
            cl.ended_at_ms = None;
            cl.last_result = None;
            cl.last_post_status = None;
        }
        cl.clone()
    };
    runtime.emit(AppEvent::comment_lottery_updated(state.clone()));
    Ok(state)
}

// ==================== OBS オーバーレイ ====================

pub async fn get_overlay_config() -> Result<OverlayConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let cfg = runtime.overlay_state.config.read().await.clone();
    Ok(cfg)
}

pub async fn save_overlay_config(config: OverlayConfig) -> Result<OverlayConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_overlay_config_and_restart(config)
        .await
        .map_err(|e| e.to_string())
}

// ==================== YouTube OAuth 2.0 ====================

pub async fn get_youtube_oauth_status() -> Result<YouTubeOAuthStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    youtube_oauth::get_oauth_status(&db).map_err(|e| e.to_string())
}

pub async fn save_youtube_oauth_client(
    client_id: String,
    client_secret: String,
) -> Result<YouTubeOAuthStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    youtube_oauth::save_oauth_client_config(&db, &client_id, &client_secret)
        .map_err(|e| e.to_string())?;
    youtube_oauth::get_oauth_status(&db).map_err(|e| e.to_string())
}

pub async fn start_youtube_oauth_login() -> Result<YouTubeOAuthStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    youtube_oauth::run_oauth_login(runtime.db.clone())
        .await
        .map_err(|e| e.to_string())
}

pub async fn disconnect_youtube_oauth() -> Result<YouTubeOAuthStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    youtube_oauth::clear_oauth_tokens(&db).map_err(|e| e.to_string())?;
    youtube_oauth::get_oauth_status(&db).map_err(|e| e.to_string())
}

// ==================== 自動アップデート & リリースノート ====================

pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    updater::check_for_updates().await.map_err(|e| e.to_string())
}

pub async fn download_update_installer(
    installer_url: String,
    installer_name: String,
    checksum_url: Option<String>,
    sink: StreamSink<UpdateDownloadProgress>,
) -> Result<(), String> {
    let res = updater::download_installer(
        &installer_url,
        &installer_name,
        checksum_url.as_deref(),
        |progress| {
            let _ = sink.add(progress);
        },
    )
    .await;

    if let Err(e) = res {
        let _ = sink.add(UpdateDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: None,
            status: "error".to_string(),
            file_path: None,
            error: Some(e.to_string()),
        });
        return Err(e.to_string());
    }
    Ok(())
}

pub fn launch_update_installer(installer_path: String, silent: bool) -> Result<(), String> {
    updater::launch_installer(&installer_path, silent).map_err(|e| e.to_string())
}

pub async fn get_startup_release_notes() -> Result<ReleaseNotesInfo, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .get_startup_release_notes()
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_current_release_notes() -> Result<ReleaseNotesInfo, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let prev = {
        let db = runtime.db.lock().await;
        db.get_setting("last_launched_version")
            .map_err(|e| e.to_string())?
    };
    Ok(updater::get_current_release_notes(prev).await)
}

pub async fn mark_release_notes_seen() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    updater::record_last_launched_version(&db).map_err(|e| e.to_string())
}
