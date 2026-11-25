use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::db::Database;
use crate::engine::{counter, highlight, lottery};
use crate::models::{
    AppEvent, CommentItem, CommentLotteryConfig, CommentLotteryState, ConnectionMode,
    ConnectionStateDto, ConnectionStatus, FeatureFlags, HighlightRule, LotteryResult,
    OverlayConfig, PlatformKind, RawIncomingComment, ReleaseNotesInfo, UserLotteryRequest,
    UserRecord, WordCounterConfig,
};
use crate::overlay::{start_overlay_server, OverlayServerHandle, OverlaySharedState};
use crate::platform::{demo, extract_youtube_video_id, youtube, ProviderConnection};
use crate::updater;

static RUNTIME: OnceCell<Arc<AppRuntime>> = OnceCell::new();

pub struct AppRuntime {
    pub db: Arc<Mutex<Database>>,
    pub overlay_state: Arc<OverlaySharedState>,
    pub overlay_server: Mutex<Option<OverlayServerHandle>>,
    pub feature_flags: RwLock<FeatureFlags>,
    pub connection_state: RwLock<ConnectionStateDto>,
    pub active_provider: Mutex<Option<ProviderConnection>>,
    pub recent_comments: RwLock<Vec<CommentItem>>,
    pub user_lottery_won_ids: Mutex<HashSet<String>>,
    pub comment_lottery: Mutex<CommentLotteryState>,
    pub event_tx: broadcast::Sender<AppEvent>,
    pub comment_tx: mpsc::UnboundedSender<RawIncomingComment>,
    pub launched_after_update: RwLock<bool>,
}

pub fn get_runtime() -> Result<Arc<AppRuntime>> {
    RUNTIME
        .get()
        .cloned()
        .ok_or_else(|| anyhow!("Stosup コアがまだ初期化されていません"))
}

fn default_db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Stosup").join("stosup.db")
}

/// アプリ起動時に一度だけランタイムを初期化する（二重呼び出し安全）
pub async fn initialize_runtime(after_update: bool) -> Result<Arc<AppRuntime>> {
    if let Some(existing) = RUNTIME.get() {
        if after_update {
            *existing.launched_after_update.write().await = true;
        }
        return Ok(Arc::clone(existing));
    }

    let db_path = default_db_path();
    let db = Database::open(&db_path)?;
    let overlay_config = db.get_overlay_config()?;
    let feature_flags = db.get_feature_flags()?;
    let counters = if feature_flags.word_counter_enabled {
        db.list_word_counters()?
    } else {
        Vec::new()
    };

    let db_arc = Arc::new(Mutex::new(db));
    let overlay_state = Arc::new(OverlaySharedState::new(overlay_config.clone(), counters));
    let (event_tx, _) = broadcast::channel::<AppEvent>(512);
    let (comment_tx, comment_rx) = mpsc::unbounded_channel::<RawIncomingComment>();

    // OBS オーバーレイ機能が有効な場合のみローカルサーバーを起動
    let server_handle = if feature_flags.obs_overlay_enabled {
        match start_overlay_server(overlay_config.port, Arc::clone(&overlay_state)).await {
            Ok(h) => Some(h),
            Err(_) => start_overlay_server(0, Arc::clone(&overlay_state))
                .await
                .ok(),
        }
    } else {
        None
    };

    if let Some(ref handle) = server_handle {
        if handle.port != overlay_config.port {
            let mut updated = overlay_config.clone();
            updated.port = handle.port;
            overlay_state.update_config(updated.clone()).await;
            let guard = db_arc.lock().await;
            let _ = guard.set_overlay_config(&updated);
        }
    }

    let runtime = Arc::new(AppRuntime {
        db: db_arc,
        overlay_state,
        overlay_server: Mutex::new(server_handle),
        feature_flags: RwLock::new(feature_flags),
        connection_state: RwLock::new(ConnectionStateDto::default()),
        active_provider: Mutex::new(None),
        recent_comments: RwLock::new(Vec::new()),
        user_lottery_won_ids: Mutex::new(HashSet::new()),
        comment_lottery: Mutex::new(CommentLotteryState::default()),
        event_tx,
        comment_tx,
        launched_after_update: RwLock::new(after_update),
    });

    // バックグラウンドコメント処理ループを開始
    spawn_comment_processor(Arc::clone(&runtime), comment_rx);

    let _ = RUNTIME.set(Arc::clone(&runtime));
    Ok(runtime)
}

fn spawn_comment_processor(
    runtime: Arc<AppRuntime>,
    mut comment_rx: mpsc::UnboundedReceiver<RawIncomingComment>,
) {
    tokio::spawn(async move {
        while let Some(raw) = comment_rx.recv().await {
            if let Err(e) = runtime.handle_incoming_comment(raw).await {
                let _ = runtime
                    .event_tx
                    .send(AppEvent::system_notice("error", format!("コメント処理エラー: {}", e)));
            }
        }
    });
}

impl AppRuntime {
    pub fn emit(&self, ev: AppEvent) {
        let _ = self.event_tx.send(ev);
    }

    /// 受信した生コメントを各機能の ON/OFF 設定に応じて処理する
    async fn handle_incoming_comment(&self, raw: RawIncomingComment) -> Result<()> {
        let session_id = self.connection_state.read().await.session_id.clone();
        if session_id.is_empty() {
            return Ok(());
        }

        let flags = self.feature_flags.read().await.clone();

        let (is_banned, rules, counters_changed, updated_counters) = {
            let db = self.db.lock().await;
            let is_banned = if flags.user_tracking_enabled {
                db.record_user_comment(&session_id, &raw)?
            } else {
                db.is_user_banned(&raw.user_id)?
            };
            let rules = if flags.word_highlight_enabled {
                db.list_highlight_rules()?
            } else {
                Vec::new()
            };
            let counters_changed = if flags.word_counter_enabled {
                counter::process_counters_for_comment(&db, &raw.user_id, is_banned, &raw.message)?
            } else {
                false
            };
            let updated_counters = if counters_changed {
                Some(db.list_word_counters()?)
            } else {
                None
            };
            (is_banned, rules, counters_changed, updated_counters)
        };

        // 特定ワード反応ルールを評価（機能が有効な場合のみ吹き出し色を適用）
        let matched_rule = if flags.word_highlight_enabled {
            highlight::evaluate_highlight(&raw.message, &rules)
        } else {
            None
        };

        let item = CommentItem {
            id: raw.id,
            session_id,
            platform: raw.platform,
            user_id: raw.user_id,
            user_name: raw.user_name,
            user_icon_url: raw.user_icon_url,
            message: raw.message,
            timestamp_ms: raw.timestamp_ms,
            is_owner: raw.is_owner,
            is_moderator: raw.is_moderator,
            is_member: raw.is_member,
            is_banned,
            highlight_rule_id: matched_rule.map(|r| r.id.clone()),
            bubble_bg_color: matched_rule.map(|r| r.bg_color.clone()),
            bubble_text_color: matched_rule.map(|r| r.text_color.clone()),
            bubble_border_color: matched_rule.map(|r| r.border_color.clone()),
        };

        // コメント表示機能が有効な場合のみ UI ＆ OBS へ反映
        if flags.comment_display_enabled {
            {
                let mut list = self.recent_comments.write().await;
                list.push(item.clone());
                if list.len() > 300 {
                    let over = list.len() - 300;
                    list.drain(0..over);
                }
            }
            self.emit(AppEvent::comment_received(item.clone()));
            if flags.obs_overlay_enabled {
                self.overlay_state.push_comment(item.clone()).await;
            }
        }

        // コメント抽選機能が有効かつ受付中の場合のみ自動収集
        let lottery_updated = if flags.comment_lottery_enabled {
            let mut cl = self.comment_lottery.lock().await;
            if lottery::try_collect_comment_entry(&mut cl, &item) {
                Some(cl.clone())
            } else {
                None
            }
        } else {
            None
        };

        if flags.user_tracking_enabled {
            self.emit(AppEvent::users_updated());
        }

        if counters_changed {
            if let Some(counters) = updated_counters {
                if flags.obs_overlay_enabled {
                    self.overlay_state.update_counters(counters.clone()).await;
                }
                self.emit(AppEvent::counters_updated(counters));
            }
        }

        if let Some(cl_state) = lottery_updated {
            self.emit(AppEvent::comment_lottery_updated(cl_state));
        }

        Ok(())
    }

    /// 各機能の ON/OFF 設定を更新・永続化し、OBS サーバー起動停止や受付停止を即時反映する
    pub async fn update_feature_flags(&self, new_flags: FeatureFlags) -> Result<FeatureFlags> {
        {
            let db = self.db.lock().await;
            db.set_feature_flags(&new_flags)?;
        }
        *self.feature_flags.write().await = new_flags.clone();

        // OBS オーバーレイサーバーの起動 / 停止を即時反映
        if new_flags.obs_overlay_enabled {
            let mut srv_guard = self.overlay_server.lock().await;
            if srv_guard.is_none() {
                let mut cfg = self.overlay_state.config.read().await.clone();
                let handle = match start_overlay_server(cfg.port, Arc::clone(&self.overlay_state)).await {
                    Ok(h) => h,
                    Err(_) => start_overlay_server(0, Arc::clone(&self.overlay_state)).await?,
                };
                if handle.port != cfg.port {
                    cfg.port = handle.port;
                    self.overlay_state.update_config(cfg.clone()).await;
                    let db = self.db.lock().await;
                    let _ = db.set_overlay_config(&cfg);
                }
                *srv_guard = Some(handle);
            }
        } else {
            let mut srv_guard = self.overlay_server.lock().await;
            if let Some(ref mut old) = *srv_guard {
                old.stop();
            }
            *srv_guard = None;
        }

        // ワードカウント OFF 時は OBS オーバーレイ側を非表示、ON 復帰時は最新カウントを再同期
        if new_flags.word_counter_enabled {
            if let Ok(list) = self.list_word_counters().await {
                self.overlay_state.update_counters(list).await;
            }
        } else {
            self.overlay_state.update_counters(Vec::new()).await;
        }

        // コメント抽選を OFF にしたら進行中の受付も停止
        if !new_flags.comment_lottery_enabled {
            let mut cl = self.comment_lottery.lock().await;
            if cl.is_accepting {
                cl.is_accepting = false;
                cl.ended_at_ms = Some(Utc::now().timestamp_millis());
                let updated = cl.clone();
                drop(cl);
                self.emit(AppEvent::comment_lottery_updated(updated));
            }
        }

        self.emit(AppEvent::feature_flags_updated(new_flags.clone()));
        Ok(new_flags)
    }

    pub async fn disconnect_current(&self) {
        if let Some(prev) = self.active_provider.lock().await.take() {
            prev.stop();
        }
        let mut st = self.connection_state.write().await;
        st.status = ConnectionStatus::Disconnected;
        st.error_message = None;
        st.can_post_chat = false;
        let dto = st.clone();
        drop(st);
        self.emit(AppEvent::connection_changed(dto));
    }

    pub async fn connect_youtube(&self, input: &str, mode: ConnectionMode) -> Result<ConnectionStateDto> {
        let video_id = extract_youtube_video_id(input)?;
        self.disconnect_current().await;

        {
            let mut st = self.connection_state.write().await;
            st.status = ConnectionStatus::Connecting;
            st.platform = PlatformKind::YouTube;
            st.mode = mode;
            st.stream_id = video_id.clone();
            st.error_message = None;
            let dto = st.clone();
            drop(st);
            self.emit(AppEvent::connection_changed(dto));
        }

        let runtime_ev = self.event_tx.clone();
        let on_error = move |msg: String| {
            let _ = runtime_ev.send(AppEvent::system_notice("error", msg));
        };

        let conn_res = match mode {
            ConnectionMode::YouTubeDataApi => {
                youtube::connect_youtube_data_api(
                    &video_id,
                    Arc::clone(&self.db),
                    self.comment_tx.clone(),
                    on_error,
                )
                .await
            }
            _ => {
                youtube::connect_youtube_innertube(
                    &video_id,
                    Arc::clone(&self.db),
                    self.comment_tx.clone(),
                    on_error,
                )
                .await
            }
        };

        match conn_res {
            Ok(conn) => {
                let now_ms = Utc::now().timestamp_millis();
                let session_id = {
                    let db = self.db.lock().await;
                    db.get_or_create_session(
                        PlatformKind::YouTube,
                        &conn.stream_id,
                        &conn.stream_title,
                        now_ms,
                    )?
                };
                let dto = ConnectionStateDto {
                    status: ConnectionStatus::Connected,
                    platform: PlatformKind::YouTube,
                    mode,
                    stream_title: conn.stream_title.clone(),
                    stream_id: conn.stream_id.clone(),
                    session_id,
                    error_message: None,
                    can_post_chat: conn.can_post_chat,
                };
                *self.active_provider.lock().await = Some(conn);
                *self.connection_state.write().await = dto.clone();
                self.emit(AppEvent::connection_changed(dto.clone()));
                self.emit(AppEvent::users_updated());
                Ok(dto)
            }
            Err(e) => {
                let dto = ConnectionStateDto {
                    status: ConnectionStatus::Error,
                    platform: PlatformKind::YouTube,
                    mode,
                    stream_title: String::new(),
                    stream_id: video_id,
                    session_id: String::new(),
                    error_message: Some(e.to_string()),
                    can_post_chat: false,
                };
                *self.connection_state.write().await = dto.clone();
                self.emit(AppEvent::connection_changed(dto));
                Err(e)
            }
        }
    }

    pub async fn connect_demo(&self, session_name: &str, auto_generate: bool) -> Result<ConnectionStateDto> {
        self.disconnect_current().await;
        let stream_id = if session_name.trim().is_empty() {
            format!("demo-{}", Utc::now().format("%Y%m%d-%H%M%S"))
        } else {
            session_name.trim().to_string()
        };

        let conn = demo::connect_demo(&stream_id, auto_generate, self.comment_tx.clone());
        let now_ms = Utc::now().timestamp_millis();
        let session_id = {
            let db = self.db.lock().await;
            db.get_or_create_session(PlatformKind::Demo, &conn.stream_id, &conn.stream_title, now_ms)?
        };

        let dto = ConnectionStateDto {
            status: ConnectionStatus::Connected,
            platform: PlatformKind::Demo,
            mode: ConnectionMode::Demo,
            stream_title: conn.stream_title.clone(),
            stream_id: conn.stream_id.clone(),
            session_id,
            error_message: None,
            can_post_chat: true,
        };
        *self.active_provider.lock().await = Some(conn);
        *self.connection_state.write().await = dto.clone();
        self.emit(AppEvent::connection_changed(dto.clone()));
        self.emit(AppEvent::users_updated());
        Ok(dto)
    }

    /// 配信チャットへコメントを投稿する（YouTube Data API v3 または デモモード）
    pub async fn post_chat_message(&self, text: &str) -> Result<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("投稿メッセージが空です"));
        }

        let (platform, live_chat_id) = {
            let guard = self.active_provider.lock().await;
            let provider = guard
                .as_ref()
                .ok_or_else(|| anyhow!("配信に接続していません"))?;
            (provider.platform, provider.live_chat_id.clone())
        };

        match platform {
            PlatformKind::Demo => {
                // デモモードでは配信者コメントとして即座にストリームへ流す
                let raw = RawIncomingComment {
                    id: uuid::Uuid::new_v4().to_string(),
                    platform: PlatformKind::Demo,
                    user_id: "demo-owner".to_string(),
                    user_name: "配信者（Stosup）".to_string(),
                    user_icon_url: demo::make_svg_avatar("%23f59e0b", "主"),
                    message: trimmed.to_string(),
                    timestamp_ms: Utc::now().timestamp_millis(),
                    is_owner: true,
                    is_moderator: false,
                    is_member: false,
                };
                let _ = self.comment_tx.send(raw);
                Ok("デモチャットへコメントを投稿しました".to_string())
            }
            PlatformKind::YouTube => {
                let cid = live_chat_id.ok_or_else(|| {
                    anyhow!(
                        "YouTube チャット投稿には設定画面での Google OAuth 認証およびライブチャットIDの取得が必要です"
                    )
                })?;
                youtube::post_live_chat_message(Arc::clone(&self.db), &cid, trimmed).await?;
                Ok("YouTube チャットへコメントを投稿しました".to_string())
            }
        }
    }

    pub async fn save_overlay_config_and_restart(&self, cfg: OverlayConfig) -> Result<OverlayConfig> {
        let obs_enabled = self.feature_flags.read().await.obs_overlay_enabled;
        let current_port = self
            .overlay_server
            .lock()
            .await
            .as_ref()
            .map(|h| h.port)
            .unwrap_or(0);

        let mut final_cfg = cfg.clone();
        if obs_enabled && cfg.port != current_port {
            let mut srv_guard = self.overlay_server.lock().await;
            if let Some(ref mut old) = *srv_guard {
                old.stop();
            }
            let new_handle = start_overlay_server(cfg.port, Arc::clone(&self.overlay_state)).await?;
            final_cfg.port = new_handle.port;
            *srv_guard = Some(new_handle);
        }

        self.overlay_state.update_config(final_cfg.clone()).await;
        let db = self.db.lock().await;
        db.set_overlay_config(&final_cfg)?;
        Ok(final_cfg)
    }

    pub async fn get_startup_release_notes(&self) -> Result<ReleaseNotesInfo> {
        let after_update = *self.launched_after_update.read().await;
        let prev = {
            let db = self.db.lock().await;
            let p = db.get_setting("last_launched_version")?;
            // 完全初回起動（記録なし）の場合は現在バージョンを初期記録しておく
            if p.is_none() && !after_update {
                let _ = updater::record_last_launched_version(&db);
            }
            p
        };
        Ok(updater::get_startup_release_notes(prev, after_update).await)
    }

    // ユーティリティメソッド（API層から委譲）
    pub async fn list_users(&self) -> Result<Vec<UserRecord>> {
        let sid = self.connection_state.read().await.session_id.clone();
        let db = self.db.lock().await;
        db.list_users(if sid.is_empty() { None } else { Some(&sid) })
    }

    pub async fn list_highlight_rules(&self) -> Result<Vec<HighlightRule>> {
        let db = self.db.lock().await;
        db.list_highlight_rules()
    }

    pub async fn list_word_counters(&self) -> Result<Vec<WordCounterConfig>> {
        let db = self.db.lock().await;
        db.list_word_counters()
    }

    pub async fn draw_user_lottery(&self, req: UserLotteryRequest) -> Result<LotteryResult> {
        let flags = self.feature_flags.read().await.clone();
        if !flags.user_lottery_enabled {
            return Err(anyhow!(
                "ユーザー抽選機能は現在オフになっています。機能を有効にしてから実行してください。"
            ));
        }
        let users = self.list_users().await?;
        let result = {
            let mut won = self.user_lottery_won_ids.lock().await;
            lottery::draw_user_lottery(&users, &req, &mut won)?
        };
        {
            let db = self.db.lock().await;
            db.save_lottery_result(&result)?;
        }
        if flags.obs_overlay_enabled {
            self.overlay_state.show_lottery(result.clone()).await;
        }
        self.emit(AppEvent::lottery_drawn(result.clone()));
        Ok(result)
    }

    pub async fn start_comment_lottery(&self, config: CommentLotteryConfig) -> Result<CommentLotteryState> {
        if !self.feature_flags.read().await.comment_lottery_enabled {
            return Err(anyhow!(
                "コメント抽選機能は現在オフになっています。機能を有効にしてから開始してください。"
            ));
        }
        let mut post_status = None;
        if config.post_to_chat && !config.start_message.trim().is_empty() {
            match self.post_chat_message(&config.start_message).await {
                Ok(msg) => post_status = Some(msg),
                Err(e) => {
                    post_status = Some(format!("開始コメント投稿スキップ/失敗: {}", e));
                }
            }
        }

        let state = {
            let mut cl = self.comment_lottery.lock().await;
            cl.config = config;
            cl.is_accepting = true;
            cl.started_at_ms = Some(Utc::now().timestamp_millis());
            cl.ended_at_ms = None;
            cl.entries.clear();
            cl.last_result = None;
            cl.last_post_status = post_status;
            cl.clone()
        };

        self.emit(AppEvent::comment_lottery_updated(state.clone()));
        Ok(state)
    }

    pub async fn stop_comment_lottery(&self) -> Result<CommentLotteryState> {
        let (should_post, end_msg) = {
            let cl = self.comment_lottery.lock().await;
            (cl.config.post_to_chat, cl.config.end_message.clone())
        };

        let mut post_status = None;
        if should_post && !end_msg.trim().is_empty() {
            match self.post_chat_message(&end_msg).await {
                Ok(msg) => post_status = Some(msg),
                Err(e) => {
                    post_status = Some(format!("終了コメント投稿スキップ/失敗: {}", e));
                }
            }
        }

        let state = {
            let mut cl = self.comment_lottery.lock().await;
            cl.is_accepting = false;
            cl.ended_at_ms = Some(Utc::now().timestamp_millis());
            cl.last_post_status = post_status;
            cl.clone()
        };

        self.emit(AppEvent::comment_lottery_updated(state.clone()));
        Ok(state)
    }

    pub async fn draw_comment_lottery(&self) -> Result<LotteryResult> {
        let flags = self.feature_flags.read().await.clone();
        if !flags.comment_lottery_enabled {
            return Err(anyhow!(
                "コメント抽選機能は現在オフになっています。機能を有効にしてから実行してください。"
            ));
        }
        let banned_ids = {
            let db = self.db.lock().await;
            db.get_banned_user_ids()?
        };

        let (result, updated_state) = {
            let mut cl = self.comment_lottery.lock().await;
            let res = lottery::draw_comment_lottery(&mut cl, &banned_ids)?;
            (res, cl.clone())
        };

        {
            let db = self.db.lock().await;
            db.save_lottery_result(&result)?;
        }

        if flags.obs_overlay_enabled {
            self.overlay_state.show_lottery(result.clone()).await;
        }
        self.emit(AppEvent::comment_lottery_updated(updated_state));
        self.emit(AppEvent::lottery_drawn(result.clone()));
        Ok(result)
    }
}
