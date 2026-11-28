use chrono::Utc;
use flutter_rust_bridge::frb;

use crate::backup;
use crate::engine::lottery;
use crate::fonts;
use crate::frb_generated::StreamSink;
use crate::media;
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

    loop {
        match rx.recv().await {
            Ok(ev) => {
                if sink.add(ev).is_err() {
                    break;
                }
            }
            // 高頻度コメントバーストで一時的に遅延してもイベント購読を切断せず継続
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
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
    let comments = runtime
        .recent_comments
        .read()
        .await
        .iter()
        .cloned()
        .collect();
    Ok(comments)
}

pub async fn clear_recent_comments() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.recent_comments.write().await.clear();
    runtime.emit(AppEvent::comments_replaced(Vec::new()));
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
        message_runs: Vec::new(),
        timestamp_ms: Utc::now().timestamp_millis(),
        is_owner: false,
        is_moderator: false,
        is_member: false,
        event_kind: None,
        amount_display: None,
        amount_micros: 0,
        currency: String::new(),
        tier_color: None,
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

pub async fn save_highlight_rule(rule: HighlightRule) -> Result<Vec<HighlightRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_highlight_rule(rule)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_highlight_rule(id: String) -> Result<Vec<HighlightRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_highlight_rule(&id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 特定ワードカウンター ====================

pub async fn list_word_counters() -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.list_word_counters().await.map_err(|e| e.to_string())
}

pub async fn save_word_counter(
    counter: WordCounterConfig,
) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_word_counter(counter)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_word_counter(id: String) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_word_counter(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_word_counter(id: String) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .reset_word_counter(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn adjust_word_counter(
    id: String,
    delta: i64,
) -> Result<Vec<WordCounterConfig>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .adjust_word_counter(&id, delta)
        .await
        .map_err(|e| e.to_string())
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
    // BANした場合、進行中のコメント抽選・投票・参加型キューからも即座に除外してUI同期
    if banned {
        {
            let mut cl = runtime.comment_lottery.lock().await;
            let before = cl.entries.len();
            cl.entries.retain(|e| e.user_id != user_id);
            if cl.entries.len() != before {
                let state = cl.clone();
                drop(cl);
                runtime.emit(AppEvent::comment_lottery_updated(state));
            }
        }
        {
            let mut pr = runtime.poll_runtime.lock().await;
            if pr.remove_voter(&user_id) {
                let p_state = pr.state.clone();
                drop(pr);
                let enabled = runtime.feature_flags.read().await.poll_enabled;
                runtime.overlay_state.update_poll(p_state.clone(), enabled).await;
                runtime.emit(AppEvent::poll_updated(p_state));
            }
        }
        {
            let mut qr = runtime.queue_runtime.lock().await;
            let before = qr.state.entries.len();
            qr.state.entries.retain(|e| e.user_id != user_id);
            if qr.state.entries.len() != before {
                let q_state = qr.state.clone();
                drop(qr);
                let enabled = runtime.feature_flags.read().await.queue_enabled;
                runtime.overlay_state.update_queue(q_state.clone(), enabled).await;
                runtime.emit(AppEvent::queue_updated(q_state));
            }
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
    runtime
        .refresh_memo_in_recent_comments(&user_id, &memo)
        .await;
    runtime.emit(AppEvent::users_updated());
    runtime.list_users().await.map_err(|e| e.to_string())
}

pub async fn delete_user(
    user_id: String,
    platform: PlatformKind,
) -> Result<Vec<UserRecord>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_user(&user_id, platform)
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_users(keep_banned: bool) -> Result<Vec<UserRecord>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .reset_users(keep_banned)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 初見カウンター ====================

pub async fn get_first_time_state() -> Result<FirstTimeCounterState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let state = runtime.first_time_state.lock().await.clone();
    Ok(state)
}

pub async fn save_first_time_config(
    config: FirstTimeCounterConfig,
) -> Result<FirstTimeCounterState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_first_time_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_first_time_session_count() -> Result<FirstTimeCounterState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .reset_first_time_session_count()
        .await
        .map_err(|e| e.to_string())
}

pub async fn reset_first_time_all_history() -> Result<FirstTimeCounterState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .reset_first_time_all_history()
        .await
        .map_err(|e| e.to_string())
}

pub async fn adjust_first_time_count(delta: i64) -> Result<FirstTimeCounterState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .adjust_first_time_count(delta)
        .await
        .map_err(|e| e.to_string())
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
    runtime.emit(AppEvent::user_lottery_winners_updated(Vec::new()));
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

// ==================== 各機能の有効/無効（ON/OFF） ====================

pub async fn get_feature_flags() -> Result<FeatureFlags, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let flags = runtime.feature_flags.read().await.clone();
    Ok(flags)
}

pub async fn save_feature_flags(flags: FeatureFlags) -> Result<FeatureFlags, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .update_feature_flags(flags)
        .await
        .map_err(|e| e.to_string())
}

// ==================== アプリ外観テーマ ====================

pub async fn get_theme_mode() -> Result<AppThemeMode, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    db.get_theme_mode().map_err(|e| e.to_string())
}

pub async fn save_theme_mode(mode: AppThemeMode) -> Result<AppThemeMode, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let db = runtime.db.lock().await;
    db.set_theme_mode(mode).map_err(|e| e.to_string())?;
    Ok(mode)
}

// ==================== OBS オーバーレイ ====================

/// OS にインストールされているシステムフォント一覧を取得する
pub fn list_system_fonts() -> Vec<String> {
    fonts::list_installed_system_fonts()
}

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
    let include_prerelease = match get_runtime() {
        Ok(runtime) => runtime.feature_flags.read().await.include_prerelease_updates,
        Err(_) => false,
    };
    updater::check_for_updates(include_prerelease)
        .await
        .map_err(|e| e.to_string())
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

// ==================== スーパーチャット / メンバーシップ ====================

pub async fn list_monetization_events(limit: u32) -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_monetization_events(limit as usize)
        .await
        .map_err(|e| e.to_string())
}

pub async fn set_monetization_checked(
    id: String,
    checked: bool,
) -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .set_monetization_checked(&id, checked)
        .await
        .map_err(|e| e.to_string())
}

pub async fn mark_all_monetization_checked() -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .mark_all_monetization_checked()
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_monetization_event(id: String) -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_monetization_event(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn clear_monetization_events(
    checked_only: bool,
) -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .clear_monetization_events(checked_only)
        .await
        .map_err(|e| e.to_string())
}

pub async fn send_demo_monetization(
    kind: MonetizationEventKind,
    user_name: String,
    amount_display: String,
    message: String,
) -> Result<Vec<MonetizationItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .send_demo_monetization(kind, &user_name, &amount_display, &message)
        .await
        .map_err(|e| e.to_string())
}

// ==================== リアルタイム投票 / アンケート ====================

pub async fn get_poll_state() -> Result<PollState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_poll_state().await)
}

pub async fn save_poll_config(config: PollConfig) -> Result<PollState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_poll_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn start_poll(config: PollConfig) -> Result<PollState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.start_poll(config).await.map_err(|e| e.to_string())
}

pub async fn stop_poll() -> Result<PollState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.stop_poll().await)
}

pub async fn reset_poll() -> Result<PollState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.reset_poll().await)
}

pub async fn clear_overlay_poll() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.clear_overlay_poll().await;
    Ok(())
}

// ==================== コメントピックアップ（ピン留め表示） ====================

pub async fn get_picked_comment() -> Result<PickedCommentState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_picked_comment().await)
}

pub async fn pick_comment(comment: CommentItem) -> Result<PickedCommentState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .pick_comment(comment)
        .await
        .map_err(|e| e.to_string())
}

pub async fn clear_picked_comment() -> Result<PickedCommentState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.clear_picked_comment().await)
}

// ==================== 参加型キュー（順番待ちリスト） ====================

pub async fn get_queue_state() -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_queue_state().await)
}

pub async fn save_queue_config(config: QueueConfig) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.save_queue_config(config).await)
}

pub async fn queue_add_manual(user_name: String, note: String) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_add_manual(&user_name, &note).await)
}

pub async fn queue_set_status(
    user_id: String,
    status: QueueEntryStatus,
) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_set_status(&user_id, status).await)
}

pub async fn queue_promote_next() -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_promote_next().await)
}

pub async fn queue_rotate_active() -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_rotate_active().await)
}

pub async fn queue_move_entry(user_id: String, delta: i32) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_move_entry(&user_id, delta).await)
}

pub async fn queue_remove_entry(user_id: String) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_remove_entry(&user_id).await)
}

pub async fn queue_reset(reset_play_counts: bool) -> Result<QueueState, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.queue_reset(reset_play_counts).await)
}

// ==================== 特定ワード反応メディアアニメーション ====================

/// OS ネイティブのファイル選択ダイアログを開き、画像・動画・音声ファイルパスを返す
pub async fn pick_media_file_dialog() -> Option<String> {
    tokio::task::spawn_blocking(media::pick_media_file_dialog)
        .await
        .ok()
        .flatten()
}

/// OS ネイティブのファイル選択ダイアログを開き、効果音（SE）ファイルパスを返す
pub async fn pick_sound_file_dialog() -> Option<String> {
    tokio::task::spawn_blocking(media::pick_sound_file_dialog)
        .await
        .ok()
        .flatten()
}

/// 選択した画像・動画ファイルを Stosup のアプリ内フォルダへコピー保存する
pub async fn import_media_file(source_path: String) -> Result<ImportedMediaFile, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let media_dir = runtime.media_dir.clone();
    tokio::task::spawn_blocking(move || media::import_media_file(&media_dir, &source_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

pub async fn list_media_reaction_rules() -> Result<Vec<MediaReactionRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_media_reaction_rules()
        .await
        .map_err(|e| e.to_string())
}

pub async fn save_media_reaction_rule(
    rule: MediaReactionRule,
) -> Result<Vec<MediaReactionRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_media_reaction_rule(rule)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_media_reaction_rule(id: String) -> Result<Vec<MediaReactionRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_media_reaction_rule(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_media_reaction_config() -> Result<MediaReactionConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_media_reaction_config().await)
}

pub async fn save_media_reaction_config(
    config: MediaReactionConfig,
) -> Result<MediaReactionConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_media_reaction_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn test_media_reaction_rule(id: String) -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .test_media_reaction_rule(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn clear_overlay_media() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.clear_overlay_media();
    Ok(())
}

// ==================== ローカルネットワーク接続・Web リモコン ====================

pub async fn get_remote_access_status() -> Result<RemoteAccessStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_remote_access_status().await)
}

pub async fn save_remote_access_config(
    config: RemoteAccessConfig,
) -> Result<RemoteAccessStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_remote_access_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn regenerate_remote_pin() -> Result<RemoteAccessStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .regenerate_remote_pin()
        .await
        .map_err(|e| e.to_string())
}

// ==================== 利用規約（ライセンス）確認・同意・オンライン更新 ====================

pub async fn check_license_status(check_online: bool) -> Result<LicenseAgreementStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .check_license_status(check_online)
        .await
        .map_err(|e| e.to_string())
}

pub async fn accept_license_agreement(
    content_hash: String,
    revision_label: String,
) -> Result<LicenseAgreementStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .accept_license_agreement(&content_hash, &revision_label)
        .await
        .map_err(|e| e.to_string())
}

/// 利用規約に不同意の場合などにアプリケーションプロセスを即座に終了（タスクキル）する
#[frb(sync)]
pub fn terminate_app_process() {
    std::process::exit(0);
}

// ==================== 接続履歴・お気に入りチャンネル・自動検出 ====================

pub async fn resolve_youtube_live(input: String) -> Result<ResolvedYouTubeLive, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .resolve_youtube_live(&input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_connection_history() -> Result<Vec<ConnectionHistoryItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_connection_history()
        .await
        .map_err(|e| e.to_string())
}

pub async fn toggle_connection_history_favorite(
    id: String,
    favorite: bool,
) -> Result<Vec<ConnectionHistoryItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .toggle_connection_history_favorite(&id, favorite)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_connection_history(id: String) -> Result<Vec<ConnectionHistoryItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_connection_history(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_saved_channels() -> Result<Vec<SavedChannelItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_saved_channels()
        .await
        .map_err(|e| e.to_string())
}

pub async fn save_saved_channel(item: SavedChannelItem) -> Result<Vec<SavedChannelItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_saved_channel(item)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_saved_channel(id: String) -> Result<Vec<SavedChannelItem>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_saved_channel(&id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== NG ワード防御 ====================

pub async fn list_ng_word_rules() -> Result<Vec<NgWordRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.list_ng_word_rules().await)
}

pub async fn save_ng_word_rule(rule: NgWordRule) -> Result<Vec<NgWordRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_ng_word_rule(rule)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_ng_word_rule(id: String) -> Result<Vec<NgWordRule>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_ng_word_rule(&id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== OBS WebSocket 連携 ====================

pub async fn get_obs_ws_config() -> Result<ObsWsConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_obs_ws_config().await)
}

pub async fn save_obs_ws_config(config: ObsWsConfig) -> Result<ObsWsConfig, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_obs_ws_config(config)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_obs_ws_status() -> Result<ObsWsStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.get_obs_ws_status().await)
}

pub async fn connect_obs_ws() -> Result<ObsWsStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.connect_obs_ws().await.map_err(|e| e.to_string())
}

pub async fn disconnect_obs_ws() -> Result<ObsWsStatus, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.disconnect_obs_ws().await)
}

pub async fn refresh_obs_scenes() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.refresh_obs_scenes().await.map_err(|e| e.to_string())
}

pub async fn test_obs_rule(rule_id: String) -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .test_obs_rule(&rule_id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== ホットキー ＆ Stream Deck オートメーション ====================

pub async fn list_hotkey_bindings() -> Result<Vec<HotkeyBinding>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.list_hotkey_bindings().await)
}

pub async fn save_hotkey_bindings(
    bindings: Vec<HotkeyBinding>,
) -> Result<Vec<HotkeyBinding>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_hotkey_bindings(bindings)
        .await
        .map_err(|e| e.to_string())
}

pub async fn execute_automation_action(
    action: AutomationActionKind,
    target_id: String,
    secondary_target: String,
) -> Result<String, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .execute_automation_action(action, &target_id, &secondary_target)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 配信プロファイル切替 ====================

pub async fn list_stream_profiles() -> Result<Vec<StreamProfileSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_stream_profiles()
        .await
        .map_err(|e| e.to_string())
}

pub async fn save_current_as_profile(
    id: Option<String>,
    name: String,
    description: String,
) -> Result<Vec<StreamProfileSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .save_current_as_profile(id, &name, &description)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_template_profile(
    template_id: String,
    name: String,
) -> Result<Vec<StreamProfileSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .create_template_profile(&template_id, &name)
        .await
        .map_err(|e| e.to_string())
}

pub async fn apply_stream_profile(id: String) -> Result<Vec<StreamProfileSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .apply_stream_profile(&id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_stream_profile(id: String) -> Result<Vec<StreamProfileSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_stream_profile(&id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== バックアップ / 引っ越し (ZIP) ====================

pub async fn pick_backup_save_path() -> Option<String> {
    tokio::task::spawn_blocking(backup::pick_backup_save_path)
        .await
        .ok()
        .flatten()
}

pub async fn pick_backup_open_path() -> Option<String> {
    tokio::task::spawn_blocking(backup::pick_backup_open_path)
        .await
        .ok()
        .flatten()
}

pub async fn export_backup_zip(dest_path: String) -> Result<BackupExportResult, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .export_backup_zip(&dest_path)
        .await
        .map_err(|e| e.to_string())
}

pub async fn import_backup_zip(zip_path: String) -> Result<BackupImportResult, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .import_backup_zip(&zip_path)
        .await
        .map_err(|e| e.to_string())
}

// ==================== オンエアレポート ====================

pub async fn list_stream_sessions(limit: u32) -> Result<Vec<StreamSessionSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .list_stream_sessions(limit as usize)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_on_air_report(session_id: String) -> Result<OnAirReportDto, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .get_on_air_report(&session_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_stream_session(
    session_id: String,
) -> Result<Vec<StreamSessionSummary>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .delete_stream_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_report_csv_with_dialog(session_id: String) -> Result<Option<String>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let default_name = format!("stosup-report-{}.csv", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = tokio::task::spawn_blocking(move || {
        backup::pick_export_file_path(&default_name, "CSV ファイル (*.csv)", "csv")
    })
    .await
    .map_err(|e| e.to_string())?;
    match path {
        Some(p) => runtime
            .export_report_csv(&session_id, &p)
            .await
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

pub async fn export_report_json_with_dialog(session_id: String) -> Result<Option<String>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let default_name = format!("stosup-report-{}.json", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = tokio::task::spawn_blocking(move || {
        backup::pick_export_file_path(&default_name, "JSON ファイル (*.json)", "json")
    })
    .await
    .map_err(|e| e.to_string())?;
    match path {
        Some(p) => runtime
            .export_report_json(&session_id, &p)
            .await
            .map(Some)
            .map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

// ==================== オーバーレイスタジオ: テスト演出送信 ====================

pub async fn send_overlay_test_event(target: String) -> Result<String, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .send_overlay_test_event(&target)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 演出 MOD 管理 ====================

pub async fn list_installed_mods() -> Result<Vec<crate::models::InstalledMod>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.list_installed_mods().await)
}

pub async fn reload_mods() -> Result<Vec<crate::models::InstalledMod>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.reload_mods().await.map_err(|e| e.to_string())
}

pub async fn pick_mod_archive_dialog() -> Option<String> {
    tokio::task::spawn_blocking(crate::mods::pick_mod_archive_dialog)
        .await
        .ok()
        .flatten()
}

pub async fn import_mod_archive(
    archive_path: String,
) -> Result<crate::models::InstalledMod, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .import_mod_archive(&archive_path)
        .await
        .map_err(|e| e.to_string())
}

pub async fn set_mod_enabled(
    mod_id: String,
    enabled: bool,
) -> Result<Vec<crate::models::InstalledMod>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .set_mod_enabled(&mod_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

pub async fn uninstall_mod(mod_id: String) -> Result<Vec<crate::models::InstalledMod>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .uninstall_mod(&mod_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn open_mods_folder() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    let dir = runtime.mods_dir.clone();
    tokio::task::spawn_blocking(move || crate::mods::open_mods_folder(&dir))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

pub async fn get_mods_folder_path() -> Result<String, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.mods_folder_path_string())
}

pub async fn test_mod_reaction_effect(
    mod_id: String,
    effect_id: String,
    params_json: String,
    preview_only: bool,
) -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .test_mod_reaction_effect(&mod_id, &effect_id, &params_json, preview_only)
        .await
        .map_err(|e| e.to_string())
}

pub async fn test_mod_logic_script(
    mod_id: String,
    script_id: String,
    sample_message: String,
    preview_only: bool,
) -> Result<Vec<String>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime
        .test_mod_logic_script(&mod_id, &script_id, &sample_message, preview_only)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_mod_script_logs(
    limit: u32,
) -> Result<Vec<crate::models::ModScriptLogEntry>, String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    Ok(runtime.list_mod_script_logs(limit as usize).await)
}

pub async fn clear_mod_script_logs() -> Result<(), String> {
    let runtime = get_runtime().map_err(|e| e.to_string())?;
    runtime.clear_mod_script_logs().await;
    Ok(())
}
