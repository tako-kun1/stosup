use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::db::Database;
use crate::engine::media_reaction::{
    build_trigger_event, rule_has_playable_content, MediaCooldownTracker, MediaEvalContext,
};
use crate::engine::poll::PollRuntime;
use crate::engine::queue::QueueRuntime;
use crate::engine::{counter, highlight, lottery, matches_pattern};
use crate::license;
use crate::media;
use crate::models::{
    AppEvent, CommentItem, CommentLotteryConfig, CommentLotteryState, ConnectionMode,
    ConnectionStateDto, ConnectionStatus, FeatureFlags, FirstTimeCounterConfig,
    FirstTimeCounterState, FirstTimeUser, HighlightRule, ImportedMediaFile, LotteryResult,
    MatchMode, MediaReactionConfig, MediaReactionRule, MonetizationEventKind, MonetizationItem,
    OverlayConfig, PickedCommentState, PlatformKind, PollConfig, PollState, QueueConfig,
    QueueEntryStatus, QueueState, RawIncomingComment, ReleaseNotesInfo, RemoteAccessConfig,
    RemoteAccessStatus, UserLotteryRequest, UserRecord, WordCounterConfig,
};
use crate::overlay::{start_overlay_server, OverlayServerHandle, OverlaySharedState};
use crate::platform::{demo, extract_youtube_video_id, youtube, ProviderConnection};
use crate::remote;
use crate::updater;

static RUNTIME: OnceCell<Arc<AppRuntime>> = OnceCell::new();

pub struct AppRuntime {
    pub db: Arc<Mutex<Database>>,
    pub media_dir: PathBuf,
    pub overlay_state: Arc<OverlaySharedState>,
    pub overlay_server: Mutex<Option<OverlayServerHandle>>,
    pub remote_config: RwLock<RemoteAccessConfig>,
    pub feature_flags: RwLock<FeatureFlags>,
    pub connection_state: RwLock<ConnectionStateDto>,
    pub active_provider: Mutex<Option<ProviderConnection>>,
    pub recent_comments: RwLock<Vec<CommentItem>>,
    pub user_lottery_won_ids: Mutex<HashSet<String>>,
    pub comment_lottery: Mutex<CommentLotteryState>,
    pub first_time_state: Mutex<FirstTimeCounterState>,
    pub poll_runtime: Mutex<PollRuntime>,
    pub picked_comment: Mutex<PickedCommentState>,
    pub queue_runtime: Mutex<QueueRuntime>,
    pub media_config: Mutex<MediaReactionConfig>,
    pub media_cooldown: Mutex<MediaCooldownTracker>,
    pub cached_highlight_rules: RwLock<Vec<HighlightRule>>,
    pub cached_media_rules: RwLock<Vec<MediaReactionRule>>,
    pub cached_word_counters: RwLock<Vec<WordCounterConfig>>,
    pub last_session_id: Mutex<String>,
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
    let media_dir = db_path
        .parent()
        .map(|p| p.join("media"))
        .unwrap_or_else(|| PathBuf::from("media"));
    let _ = std::fs::create_dir_all(&media_dir);
    let db = Database::open(&db_path)?;
    let overlay_config = db.get_overlay_config()?;
    let remote_config = db.get_remote_access_config()?;
    let feature_flags = db.get_feature_flags()?;
    let first_time_config = db.get_first_time_config()?;
    let total_first_timers = db.count_all_time_first_timers()?;
    let first_time_state = FirstTimeCounterState {
        count: 0,
        total_detected_all_time: total_first_timers,
        recent_users: Vec::new(),
        config: first_time_config,
    };
    let initial_word_counters = db.list_word_counters()?;
    let counters = if feature_flags.word_counter_enabled {
        initial_word_counters.clone()
    } else {
        Vec::new()
    };
    let monetization_items = db.list_monetization_events(100)?;
    let poll_config = db.get_poll_config()?;
    let poll_rt = PollRuntime::with_config(poll_config);
    let poll_state = poll_rt.state.clone();
    let queue_state = db.get_queue_state()?;
    let queue_rt = QueueRuntime::from_state(queue_state.clone());
    let media_config = db.get_media_reaction_config()?;
    let initial_highlight_rules = db.list_highlight_rules()?;
    let initial_media_rules = db.list_media_reaction_rules()?;

    let db_arc = Arc::new(Mutex::new(db));
    let overlay_state = Arc::new(OverlaySharedState::new(
        media_dir.clone(),
        remote_config.clone(),
        overlay_config.clone(),
        counters,
        first_time_state.clone(),
        feature_flags.first_time_counter_enabled,
        monetization_items,
        feature_flags.superchat_enabled,
        poll_state,
        feature_flags.poll_enabled,
        queue_state,
        feature_flags.queue_enabled,
        media_config.clone(),
        feature_flags.media_reaction_enabled,
        feature_flags.pickup_enabled,
    ));
    overlay_state.set_obs_overlay_enabled(feature_flags.obs_overlay_enabled);
    let (event_tx, _) = broadcast::channel::<AppEvent>(512);
    let (comment_tx, comment_rx) = mpsc::unbounded_channel::<RawIncomingComment>();

    // OBS オーバーレイまたは LAN アクセス機能が有効な場合にローカルサーバーを起動
    let should_start_server = feature_flags.obs_overlay_enabled || remote_config.allow_lan_access;
    let server_handle = if should_start_server {
        match start_overlay_server(
            overlay_config.port,
            remote_config.allow_lan_access,
            Arc::clone(&overlay_state),
        )
        .await
        {
            Ok(h) => Some(h),
            Err(_) => start_overlay_server(
                0,
                remote_config.allow_lan_access,
                Arc::clone(&overlay_state),
            )
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
        media_dir,
        overlay_state,
        overlay_server: Mutex::new(server_handle),
        remote_config: RwLock::new(remote_config),
        feature_flags: RwLock::new(feature_flags),
        connection_state: RwLock::new(ConnectionStateDto::default()),
        active_provider: Mutex::new(None),
        recent_comments: RwLock::new(Vec::new()),
        user_lottery_won_ids: Mutex::new(HashSet::new()),
        comment_lottery: Mutex::new(CommentLotteryState::default()),
        first_time_state: Mutex::new(first_time_state),
        poll_runtime: Mutex::new(poll_rt),
        picked_comment: Mutex::new(PickedCommentState::default()),
        queue_runtime: Mutex::new(queue_rt),
        media_config: Mutex::new(media_config),
        media_cooldown: Mutex::new(MediaCooldownTracker::default()),
        cached_highlight_rules: RwLock::new(initial_highlight_rules),
        cached_media_rules: RwLock::new(initial_media_rules),
        cached_word_counters: RwLock::new(initial_word_counters),
        last_session_id: Mutex::new(String::new()),
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

        // コメントを利用する全機能が OFF の場合は一切のロック・DB・評価処理を行わず即時終了
        if !flags.comment_display_enabled
            && !flags.user_tracking_enabled
            && !flags.word_counter_enabled
            && !flags.first_time_counter_enabled
            && !flags.superchat_enabled
            && !flags.comment_lottery_enabled
            && !flags.poll_enabled
            && !flags.queue_enabled
            && !flags.media_reaction_enabled
        {
            return Ok(());
        }

        let ft_config = if flags.first_time_counter_enabled {
            Some(self.first_time_state.lock().await.config.clone())
        } else {
            None
        };

        let rules = if flags.word_highlight_enabled && flags.comment_display_enabled {
            self.cached_highlight_rules.read().await.clone()
        } else {
            Vec::new()
        };
        let media_rules = if flags.media_reaction_enabled {
            self.cached_media_rules.read().await.clone()
        } else {
            Vec::new()
        };
        let word_counters = if flags.word_counter_enabled {
            self.cached_word_counters.read().await.clone()
        } else {
            Vec::new()
        };

        // ユーザー記録が OFF のとき、BAN/メモ/初見判定を必要とする他機能もすべて OFF なら DB 参照すら省略
        let need_user_meta = (flags.comment_display_enabled && flags.user_memo_display_enabled)
            || flags.comment_display_enabled
            || flags.first_time_counter_enabled
            || flags.word_counter_enabled
            || flags.comment_lottery_enabled
            || flags.poll_enabled
            || flags.queue_enabled
            || flags.media_reaction_enabled;
        let need_db_step = flags.user_tracking_enabled
            || need_user_meta
            || (flags.word_counter_enabled && !word_counters.is_empty())
            || flags.first_time_counter_enabled
            || (flags.superchat_enabled && raw.event_kind.is_some());

        let (
            user_meta,
            counters_changed,
            updated_counters,
            first_time_trigger,
            new_all_time_ft,
            updated_monetization,
        ) = if need_db_step {
            let db = self.db.lock().await;
            let meta = if flags.user_tracking_enabled {
                db.record_user_comment(&session_id, &raw)?
            } else if need_user_meta {
                db.get_user_comment_meta(&raw.user_id, raw.platform)?
            } else {
                crate::db::UserCommentMeta {
                    is_banned: false,
                    memo: String::new(),
                    was_unregistered: false,
                    already_first_time_detected: false,
                }
            };
            let counters_changed = if flags.word_counter_enabled && !word_counters.is_empty() {
                counter::process_counters_for_comment_with_list(
                    &db,
                    &word_counters,
                    &raw.user_id,
                    meta.is_banned,
                    &raw.message,
                )?
            } else {
                false
            };
            let updated_counters = if counters_changed {
                Some(db.list_word_counters()?)
            } else {
                None
            };

            // 初見検知（生涯でまだ未検知のユーザーかつ BAN 除外条件を満たす場合）
            let mut trigger_reason: Option<&'static str> = None;
            if let Some(ref ft_cfg) = ft_config {
                if !meta.already_first_time_detected && (!ft_cfg.exclude_banned || !meta.is_banned)
                {
                    let kw_matched = ft_cfg.detect_keyword
                        && ft_cfg
                            .keywords
                            .iter()
                            .any(|kw| matches_pattern(&raw.message, kw, MatchMode::Contains, false));
                    if kw_matched {
                        trigger_reason = Some("初見ワード");
                    } else if ft_cfg.detect_unregistered && meta.was_unregistered {
                        trigger_reason = Some("未登録ユーザー");
                    }
                }
            }

            let new_all_time = if trigger_reason.is_some() {
                Some(db.mark_user_first_time_detected(&raw)?)
            } else {
                None
            };

            // スーパーチャット / メンバーシップ専用記録
            let updated_monetization = if flags.superchat_enabled {
                if let Some(ev_kind) = raw.event_kind {
                    let m_item = MonetizationItem {
                        id: raw.id.clone(),
                        session_id: session_id.clone(),
                        platform: raw.platform,
                        kind: ev_kind,
                        user_id: raw.user_id.clone(),
                        user_name: raw.user_name.clone(),
                        user_icon_url: raw.user_icon_url.clone(),
                        amount_display: raw
                            .amount_display
                            .clone()
                            .unwrap_or_else(|| "支援イベント".to_string()),
                        amount_micros: raw.amount_micros,
                        currency: raw.currency.clone(),
                        tier_color: raw
                            .tier_color
                            .clone()
                            .unwrap_or_else(|| "#FFB300".to_string()),
                        message: raw.message.clone(),
                        timestamp_ms: raw.timestamp_ms,
                        is_checked: false,
                        checked_at_ms: None,
                    };
                    db.insert_monetization_event(&m_item)?;
                    Some(db.list_monetization_events(150)?)
                } else {
                    None
                }
            } else {
                None
            };

            (
                meta,
                counters_changed,
                updated_counters,
                trigger_reason,
                new_all_time,
                updated_monetization,
            )
        } else {
            (
                crate::db::UserCommentMeta {
                    is_banned: false,
                    memo: String::new(),
                    was_unregistered: false,
                    already_first_time_detected: false,
                },
                false,
                None,
                None,
                None,
                None,
            )
        };

        let is_banned = user_meta.is_banned;
        let is_first_time = first_time_trigger.is_some();

        // 初見カウンター状態更新
        let updated_ft_state = if let Some(reason) = first_time_trigger {
            let mut ft = self.first_time_state.lock().await;
            ft.count += 1;
            if let Some(total) = new_all_time_ft {
                ft.total_detected_all_time = total;
            }
            ft.recent_users.insert(
                0,
                FirstTimeUser {
                    user_id: raw.user_id.clone(),
                    user_name: raw.user_name.clone(),
                    user_icon_url: raw.user_icon_url.clone(),
                    trigger_reason: reason.to_string(),
                    detected_at_ms: raw.timestamp_ms,
                },
            );
            if ft.recent_users.len() > 100 {
                ft.recent_users.truncate(100);
            }
            Some(ft.clone())
        } else {
            None
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
            user_memo: if flags.user_memo_display_enabled {
                user_meta.memo
            } else {
                String::new()
            },
            is_first_time,
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
            event_kind: raw.event_kind,
            amount_display: raw.amount_display,
            event_accent_color: raw.tier_color,
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

        // リアルタイム投票の自動集計
        let poll_updated = if flags.poll_enabled {
            let mut pr = self.poll_runtime.lock().await;
            if pr.try_cast_vote(&item.user_id, is_banned, &item.message) {
                Some(pr.state.clone())
            } else {
                None
            }
        } else {
            None
        };

        // 参加型キューの自動登録・辞退
        let queue_updated = if flags.queue_enabled {
            let mut qr = self.queue_runtime.lock().await;
            if qr.process_comment(
                &item.user_id,
                item.platform,
                &item.user_name,
                &item.user_icon_url,
                is_banned,
                is_first_time,
                &item.message,
                item.timestamp_ms,
            ) {
                Some(qr.state.clone())
            } else {
                None
            }
        } else {
            None
        };

        // 特定ワード反応演出（WordParty）評価
        let media_triggers = if flags.media_reaction_enabled && !media_rules.is_empty() {
            let allow_multiple = self.media_config.lock().await.allow_multiple_match;
            let mut cd = self.media_cooldown.lock().await;
            cd.evaluate_rules(
                &media_rules,
                &MediaEvalContext {
                    user_name: &item.user_name,
                    is_banned,
                    is_first_time,
                    message: &item.message,
                    event_kind: item.event_kind,
                    amount_micros: raw.amount_micros,
                    now_ms: item.timestamp_ms,
                    allow_multiple_match: allow_multiple,
                },
            )
        } else {
            Vec::new()
        };

        if flags.user_tracking_enabled {
            self.emit(AppEvent::users_updated());
        }

        if let Some(ft_state) = updated_ft_state {
            if flags.obs_overlay_enabled {
                self.overlay_state
                    .update_first_time(ft_state.clone(), flags.first_time_counter_enabled)
                    .await;
            }
            self.emit(AppEvent::first_time_updated(ft_state));
        }

        if counters_changed {
            if let Some(counters) = updated_counters {
                *self.cached_word_counters.write().await = counters.clone();
                if flags.obs_overlay_enabled {
                    self.overlay_state.update_counters(counters.clone()).await;
                }
                self.emit(AppEvent::counters_updated(counters));
            }
        }

        if let Some(cl_state) = lottery_updated {
            self.emit(AppEvent::comment_lottery_updated(cl_state));
        }

        if let Some(m_list) = updated_monetization {
            if flags.obs_overlay_enabled {
                self.overlay_state
                    .update_monetization(m_list.clone(), flags.superchat_enabled)
                    .await;
            }
            self.emit(AppEvent::monetization_updated(m_list));
        }

        if let Some(p_state) = poll_updated {
            if flags.obs_overlay_enabled {
                self.overlay_state
                    .update_poll(p_state.clone(), flags.poll_enabled)
                    .await;
            }
            self.emit(AppEvent::poll_updated(p_state));
        }

        if let Some(q_state) = queue_updated {
            {
                let db = self.db.lock().await;
                let _ = db.set_queue_state(&q_state);
            }
            if flags.obs_overlay_enabled {
                self.overlay_state
                    .update_queue(q_state.clone(), flags.queue_enabled)
                    .await;
            }
            self.emit(AppEvent::queue_updated(q_state));
        }

        for trig in media_triggers {
            if flags.obs_overlay_enabled {
                self.overlay_state.trigger_media(trig.clone());
            }
            self.emit(AppEvent::media_triggered(trig));
        }

        Ok(())
    }

    /// 新しい配信セッションへ切り替わった際の初見カウンター自動リセット判定
    async fn maybe_auto_reset_first_time_for_session(&self, session_id: &str) {
        if !self.feature_flags.read().await.first_time_counter_enabled {
            return;
        }
        let mut last = self.last_session_id.lock().await;
        if *last != session_id {
            let is_switch = !last.is_empty();
            *last = session_id.to_string();
            drop(last);

            if is_switch {
                let mut ft = self.first_time_state.lock().await;
                if ft.config.auto_reset_on_new_stream {
                    ft.count = 0;
                    ft.recent_users.clear();
                    let snapshot = ft.clone();
                    drop(ft);
                    let enabled = self.feature_flags.read().await.first_time_counter_enabled;
                    self.overlay_state
                        .update_first_time(snapshot.clone(), enabled)
                        .await;
                    self.emit(AppEvent::first_time_updated(snapshot));
                }
            }
        }
    }

    pub async fn save_first_time_config(
        &self,
        config: FirstTimeCounterConfig,
    ) -> Result<FirstTimeCounterState> {
        {
            let db = self.db.lock().await;
            db.set_first_time_config(&config)?;
        }
        let snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.config = config;
            ft.clone()
        };
        let enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(snapshot.clone(), enabled)
            .await;
        self.emit(AppEvent::first_time_updated(snapshot.clone()));
        Ok(snapshot)
    }

    pub async fn reset_first_time_session_count(&self) -> Result<FirstTimeCounterState> {
        let snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.count = 0;
            ft.recent_users.clear();
            ft.clone()
        };
        let enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(snapshot.clone(), enabled)
            .await;
        self.emit(AppEvent::first_time_updated(snapshot.clone()));
        Ok(snapshot)
    }

    pub async fn reset_first_time_all_history(&self) -> Result<FirstTimeCounterState> {
        {
            let db = self.db.lock().await;
            db.reset_all_first_time_history()?;
        }
        let snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.count = 0;
            ft.total_detected_all_time = 0;
            ft.recent_users.clear();
            ft.clone()
        };
        let enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(snapshot.clone(), enabled)
            .await;
        self.emit(AppEvent::first_time_updated(snapshot.clone()));
        Ok(snapshot)
    }

    pub async fn adjust_first_time_count(&self, delta: i64) -> Result<FirstTimeCounterState> {
        let snapshot = {
            let mut ft = self.first_time_state.lock().await;
            let next = (ft.count as i64 + delta).max(0) as u64;
            ft.count = next;
            ft.clone()
        };
        let enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(snapshot.clone(), enabled)
            .await;
        self.emit(AppEvent::first_time_updated(snapshot.clone()));
        Ok(snapshot)
    }

    /// 既存コメント一覧上のユーザーメモも即時更新する
    pub async fn refresh_memo_in_recent_comments(&self, user_id: &str, memo: &str) {
        let show_memo = self.feature_flags.read().await.user_memo_display_enabled;
        let updated = {
            let mut list = self.recent_comments.write().await;
            let mut changed = false;
            for c in list.iter_mut() {
                if c.user_id == user_id {
                    c.user_memo = if show_memo {
                        memo.to_string()
                    } else {
                        String::new()
                    };
                    changed = true;
                }
            }
            if changed {
                Some(list.clone())
            } else {
                None
            }
        };
        if let Some(comments) = updated {
            self.emit(AppEvent::comments_replaced(comments));
        }
    }

    pub async fn delete_user(&self, user_id: &str, platform: PlatformKind) -> Result<Vec<UserRecord>> {
        let total_ft = {
            let db = self.db.lock().await;
            db.delete_user(user_id, platform)?;
            db.count_all_time_first_timers()?
        };
        self.user_lottery_won_ids.lock().await.remove(user_id);
        let ft_snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.total_detected_all_time = total_ft;
            ft.recent_users.retain(|u| u.user_id != user_id);
            ft.clone()
        };
        let ft_enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(ft_snapshot.clone(), ft_enabled)
            .await;
        self.emit(AppEvent::first_time_updated(ft_snapshot));
        self.emit(AppEvent::users_updated());
        self.list_users().await
    }

    pub async fn reset_users(&self, keep_banned: bool) -> Result<Vec<UserRecord>> {
        let total_ft = {
            let db = self.db.lock().await;
            db.reset_users(keep_banned)?;
            db.count_all_time_first_timers()?
        };
        self.user_lottery_won_ids.lock().await.clear();
        let ft_snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.total_detected_all_time = total_ft;
            ft.recent_users.clear();
            ft.clone()
        };
        let ft_enabled = self.feature_flags.read().await.first_time_counter_enabled;
        self.overlay_state
            .update_first_time(ft_snapshot.clone(), ft_enabled)
            .await;
        self.emit(AppEvent::first_time_updated(ft_snapshot));
        self.emit(AppEvent::users_updated());
        self.list_users().await
    }

    /// 各機能の ON/OFF 設定を更新・永続化し、OBS サーバー起動停止や受付停止を即時反映する
    pub async fn update_feature_flags(&self, new_flags: FeatureFlags) -> Result<FeatureFlags> {
        {
            let db = self.db.lock().await;
            db.set_feature_flags(&new_flags)?;
        }
        *self.feature_flags.write().await = new_flags.clone();

        // OBS オーバーレイ / LAN サーバーの起動 / 停止を即時反映
        let allow_lan = self.remote_config.read().await.allow_lan_access;
        if new_flags.obs_overlay_enabled || allow_lan {
            let mut srv_guard = self.overlay_server.lock().await;
            if srv_guard.is_none() {
                let mut cfg = self.overlay_state.config.read().await.clone();
                let handle = match start_overlay_server(
                    cfg.port,
                    allow_lan,
                    Arc::clone(&self.overlay_state),
                )
                .await
                {
                    Ok(h) => h,
                    Err(_) => {
                        start_overlay_server(0, allow_lan, Arc::clone(&self.overlay_state)).await?
                    }
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

        // OBS オーバーレイ有効状態を即時反映（OFF 時は接続中の全 OBS WebSocket を即切断・/overlay/* を遮断）
        self.overlay_state
            .set_obs_overlay_enabled(new_flags.obs_overlay_enabled);

        // 投票・参加型キュー・コメント抽選を OFF にしたら進行中の受付も自動停止する
        let poll_snapshot = {
            let mut pr = self.poll_runtime.lock().await;
            let stopped = !new_flags.poll_enabled && pr.state.is_active;
            if stopped {
                pr.stop();
            }
            (pr.state.clone(), stopped)
        };
        if poll_snapshot.1 {
            self.emit(AppEvent::poll_updated(poll_snapshot.0.clone()));
        }

        let queue_snapshot = {
            let mut qr = self.queue_runtime.lock().await;
            let stopped = !new_flags.queue_enabled && qr.state.config.is_accepting;
            if stopped {
                qr.state.config.is_accepting = false;
            }
            (qr.state.clone(), stopped)
        };
        if queue_snapshot.1 {
            {
                let db = self.db.lock().await;
                let _ = db.set_queue_state(&queue_snapshot.0);
            }
            self.emit(AppEvent::queue_updated(queue_snapshot.0.clone()));
        }

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

        // OBS オーバーレイが有効な場合のみ各オーバーレイ状態を最新同期（OFF 時は不要なロック・DB 読込を省略）
        if new_flags.obs_overlay_enabled {
            // ワードカウント OFF 時は OBS オーバーレイ側を非表示、ON 復帰時は最新カウントを再同期
            if new_flags.word_counter_enabled {
                if let Ok(list) = self.list_word_counters().await {
                    self.overlay_state.update_counters(list).await;
                }
            } else {
                self.overlay_state.update_counters(Vec::new()).await;
            }

            // 初見カウンター ON/OFF を OBS オーバーレイへ即時同期
            {
                let ft = self.first_time_state.lock().await.clone();
                self.overlay_state
                    .update_first_time(ft, new_flags.first_time_counter_enabled)
                    .await;
            }

            // スパチャ・投票・ピックアップ・参加型キュー・メディア演出の ON/OFF も OBS へ即時同期
            let m_list = if new_flags.superchat_enabled {
                let db = self.db.lock().await;
                db.list_monetization_events(150).unwrap_or_default()
            } else {
                Vec::new()
            };
            self.overlay_state
                .update_monetization(m_list, new_flags.superchat_enabled)
                .await;

            self.overlay_state
                .update_poll(poll_snapshot.0, new_flags.poll_enabled)
                .await;

            let picked_snapshot = self.picked_comment.lock().await.clone();
            self.overlay_state
                .update_picked_comment(picked_snapshot, new_flags.pickup_enabled)
                .await;

            self.overlay_state
                .update_queue(queue_snapshot.0, new_flags.queue_enabled)
                .await;

            let media_cfg = self.media_config.lock().await.clone();
            self.overlay_state
                .update_media_config(media_cfg, new_flags.media_reaction_enabled)
                .await;
            if !new_flags.media_reaction_enabled {
                self.overlay_state.clear_media();
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
                self.maybe_auto_reset_first_time_for_session(&dto.session_id)
                    .await;
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
        self.maybe_auto_reset_first_time_for_session(&dto.session_id)
            .await;
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
                    event_kind: None,
                    amount_display: None,
                    amount_micros: 0,
                    currency: String::new(),
                    tier_color: None,
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
        let allow_lan = self.remote_config.read().await.allow_lan_access;
        let current_port = self
            .overlay_server
            .lock()
            .await
            .as_ref()
            .map(|h| h.port)
            .unwrap_or(0);

        let mut final_cfg = cfg.clone();
        if (obs_enabled || allow_lan) && cfg.port != current_port {
            let mut srv_guard = self.overlay_server.lock().await;
            if let Some(ref mut old) = *srv_guard {
                old.stop();
            }
            let new_handle =
                start_overlay_server(cfg.port, allow_lan, Arc::clone(&self.overlay_state)).await?;
            final_cfg.port = new_handle.port;
            *srv_guard = Some(new_handle);
        }

        self.overlay_state.update_config(final_cfg.clone()).await;
        {
            let db = self.db.lock().await;
            db.set_overlay_config(&final_cfg)?;
        }
        self.emit(AppEvent::overlay_config_updated(final_cfg.clone()));
        Ok(final_cfg)
    }

    // ==================== ローカルネットワーク接続・Web リモコン ====================

    pub async fn get_remote_access_status(&self) -> RemoteAccessStatus {
        let cfg = self.remote_config.read().await.clone();
        let (running, port) = {
            let srv = self.overlay_server.lock().await;
            match srv.as_ref() {
                Some(h) => (true, h.port),
                None => (false, self.overlay_state.config.read().await.port),
            }
        };
        remote::build_remote_access_status(cfg, running, port)
    }

    pub async fn save_remote_access_config(
        &self,
        mut cfg: RemoteAccessConfig,
    ) -> Result<RemoteAccessStatus> {
        if cfg.pin_code.trim().is_empty() {
            cfg.pin_code = remote::generate_pin_code();
        } else {
            cfg.pin_code = cfg.pin_code.trim().to_string();
        }

        {
            let db = self.db.lock().await;
            db.set_remote_access_config(&cfg)?;
        }
        *self.remote_config.write().await = cfg.clone();
        *self.overlay_state.remote_config.write().await = cfg.clone();

        // バインド先 (127.0.0.1 <-> 0.0.0.0) または起動状態の切り替えを即時反映
        let obs_enabled = self.feature_flags.read().await.obs_overlay_enabled;
        let need_server = obs_enabled || cfg.allow_lan_access;
        let desired_port = self.overlay_state.config.read().await.port;

        let mut srv_guard = self.overlay_server.lock().await;
        if !need_server {
            if let Some(ref mut old) = *srv_guard {
                old.stop();
            }
            *srv_guard = None;
        } else {
            let need_restart = match srv_guard.as_ref() {
                None => true,
                Some(h) => h.allow_lan_access != cfg.allow_lan_access,
            };
            if need_restart {
                if let Some(ref mut old) = *srv_guard {
                    old.stop();
                }
                *srv_guard = None;
                // 旧ソケットの解放をわずかに待つ
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                let new_handle = match start_overlay_server(
                    desired_port,
                    cfg.allow_lan_access,
                    Arc::clone(&self.overlay_state),
                )
                .await
                {
                    Ok(h) => h,
                    Err(_) => {
                        start_overlay_server(
                            0,
                            cfg.allow_lan_access,
                            Arc::clone(&self.overlay_state),
                        )
                        .await?
                    }
                };
                if new_handle.port != desired_port {
                    let mut oc = self.overlay_state.config.read().await.clone();
                    oc.port = new_handle.port;
                    self.overlay_state.update_config(oc.clone()).await;
                    let db = self.db.lock().await;
                    let _ = db.set_overlay_config(&oc);
                }
                *srv_guard = Some(new_handle);
            }
        }
        drop(srv_guard);

        Ok(self.get_remote_access_status().await)
    }

    pub async fn regenerate_remote_pin(&self) -> Result<RemoteAccessStatus> {
        let mut cfg = self.remote_config.read().await.clone();
        cfg.pin_code = remote::generate_pin_code();
        self.save_remote_access_config(cfg).await
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

    pub async fn check_license_status(&self, check_online: bool) -> Result<crate::models::LicenseAgreementStatus> {
        let remote = if check_online {
            license::fetch_remote_license_md().await
        } else {
            None
        };
        let db = self.db.lock().await;
        license::build_license_status_with_remote(&db, remote)
    }

    pub async fn accept_license_agreement(
        &self,
        content_hash: &str,
        revision_label: &str,
    ) -> Result<crate::models::LicenseAgreementStatus> {
        let db = self.db.lock().await;
        license::record_license_acceptance(&db, content_hash, revision_label)?;
        license::build_license_status_with_remote(&db, None)
    }

    // ユーティリティメソッド（API層から委譲）
    pub async fn list_users(&self) -> Result<Vec<UserRecord>> {
        let sid = self.connection_state.read().await.session_id.clone();
        let db = self.db.lock().await;
        db.list_users(if sid.is_empty() { None } else { Some(&sid) })
    }

    pub async fn count_users(&self) -> Result<usize> {
        let db = self.db.lock().await;
        db.count_users()
    }

    pub async fn list_highlight_rules(&self) -> Result<Vec<HighlightRule>> {
        Ok(self.cached_highlight_rules.read().await.clone())
    }

    pub async fn save_highlight_rule(&self, mut rule: HighlightRule) -> Result<Vec<HighlightRule>> {
        if rule.id.trim().is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        let list = {
            let db = self.db.lock().await;
            db.upsert_highlight_rule(&rule)?;
            db.list_highlight_rules()?
        };
        *self.cached_highlight_rules.write().await = list.clone();
        self.emit(AppEvent::highlight_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn delete_highlight_rule(&self, id: &str) -> Result<Vec<HighlightRule>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_highlight_rule(id)?;
            db.list_highlight_rules()?
        };
        *self.cached_highlight_rules.write().await = list.clone();
        self.emit(AppEvent::highlight_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn list_word_counters(&self) -> Result<Vec<WordCounterConfig>> {
        Ok(self.cached_word_counters.read().await.clone())
    }

    async fn sync_counters_to_overlay_and_clients(&self, list: Vec<WordCounterConfig>) {
        let flags = self.feature_flags.read().await.clone();
        if flags.word_counter_enabled && flags.obs_overlay_enabled {
            self.overlay_state.update_counters(list.clone()).await;
        }
        self.emit(AppEvent::counters_updated(list));
    }

    pub async fn save_word_counter(
        &self,
        mut counter: WordCounterConfig,
    ) -> Result<Vec<WordCounterConfig>> {
        if counter.id.trim().is_empty() {
            counter.id = uuid::Uuid::new_v4().to_string();
        }
        let list = {
            let db = self.db.lock().await;
            db.upsert_word_counter(&counter)?;
            db.list_word_counters()?
        };
        *self.cached_word_counters.write().await = list.clone();
        self.sync_counters_to_overlay_and_clients(list.clone())
            .await;
        Ok(list)
    }

    pub async fn delete_word_counter(&self, id: &str) -> Result<Vec<WordCounterConfig>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_word_counter(id)?;
            db.list_word_counters()?
        };
        *self.cached_word_counters.write().await = list.clone();
        self.sync_counters_to_overlay_and_clients(list.clone())
            .await;
        Ok(list)
    }

    pub async fn reset_word_counter(&self, id: &str) -> Result<Vec<WordCounterConfig>> {
        let list = {
            let db = self.db.lock().await;
            db.reset_word_counter(id)?;
            db.list_word_counters()?
        };
        *self.cached_word_counters.write().await = list.clone();
        self.sync_counters_to_overlay_and_clients(list.clone())
            .await;
        Ok(list)
    }

    pub async fn adjust_word_counter(
        &self,
        id: &str,
        delta: i64,
    ) -> Result<Vec<WordCounterConfig>> {
        let list = {
            let db = self.db.lock().await;
            db.adjust_word_counter(id, delta)?;
            db.list_word_counters()?
        };
        *self.cached_word_counters.write().await = list.clone();
        self.sync_counters_to_overlay_and_clients(list.clone())
            .await;
        Ok(list)
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

    // ==================== スパチャ / メンバーシップ管理 ====================

    pub async fn list_monetization_events(&self, limit: usize) -> Result<Vec<MonetizationItem>> {
        let db = self.db.lock().await;
        db.list_monetization_events(limit)
    }

    pub async fn set_monetization_checked(
        &self,
        id: &str,
        checked: bool,
    ) -> Result<Vec<MonetizationItem>> {
        let now_ms = Utc::now().timestamp_millis();
        let list = {
            let db = self.db.lock().await;
            db.set_monetization_checked(id, checked, now_ms)?;
            db.list_monetization_events(150)?
        };
        let enabled = self.feature_flags.read().await.superchat_enabled;
        self.overlay_state
            .update_monetization(list.clone(), enabled)
            .await;
        self.emit(AppEvent::monetization_updated(list.clone()));
        Ok(list)
    }

    pub async fn mark_all_monetization_checked(&self) -> Result<Vec<MonetizationItem>> {
        let now_ms = Utc::now().timestamp_millis();
        let list = {
            let db = self.db.lock().await;
            db.mark_all_monetization_checked(now_ms)?;
            db.list_monetization_events(150)?
        };
        let enabled = self.feature_flags.read().await.superchat_enabled;
        self.overlay_state
            .update_monetization(list.clone(), enabled)
            .await;
        self.emit(AppEvent::monetization_updated(list.clone()));
        Ok(list)
    }

    pub async fn delete_monetization_event(&self, id: &str) -> Result<Vec<MonetizationItem>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_monetization_event(id)?;
            db.list_monetization_events(150)?
        };
        let enabled = self.feature_flags.read().await.superchat_enabled;
        self.overlay_state
            .update_monetization(list.clone(), enabled)
            .await;
        self.emit(AppEvent::monetization_updated(list.clone()));
        Ok(list)
    }

    pub async fn clear_monetization_events(
        &self,
        checked_only: bool,
    ) -> Result<Vec<MonetizationItem>> {
        let list = {
            let db = self.db.lock().await;
            db.clear_monetization_events(checked_only)?;
            db.list_monetization_events(150)?
        };
        let enabled = self.feature_flags.read().await.superchat_enabled;
        self.overlay_state
            .update_monetization(list.clone(), enabled)
            .await;
        self.emit(AppEvent::monetization_updated(list.clone()));
        Ok(list)
    }

    pub async fn send_demo_monetization(
        &self,
        kind: MonetizationEventKind,
        user_name: &str,
        amount_display: &str,
        message: &str,
    ) -> Result<Vec<MonetizationItem>> {
        let flags = self.feature_flags.read().await.clone();
        if !flags.superchat_enabled {
            return Err(anyhow!(
                "スパチャ / メンバーシップ管理機能は現在オフになっています。機能を有効にしてから実行してください。"
            ));
        }
        let now_ms = Utc::now().timestamp_millis();
        let name = if user_name.trim().is_empty() {
            "応援リスナー".to_string()
        } else {
            user_name.trim().to_string()
        };
        let initial = name.chars().next().unwrap_or('応').to_string();
        let amount = if amount_display.trim().is_empty() {
            match kind {
                MonetizationEventKind::SuperChat => "¥1,000".to_string(),
                MonetizationEventKind::SuperSticker => "¥500".to_string(),
                MonetizationEventKind::MembershipNew => "新規メンバー加入".to_string(),
                MonetizationEventKind::MembershipMilestone => "メンバー 6ヶ月継続".to_string(),
                MonetizationEventKind::MembershipGift => "メンバーシップギフト 5件".to_string(),
            }
        } else {
            amount_display.trim().to_string()
        };
        let tier_color = match kind {
            MonetizationEventKind::SuperChat => "#FFCA28".to_string(),
            MonetizationEventKind::SuperSticker => "#E91E63".to_string(),
            MonetizationEventKind::MembershipNew
            | MonetizationEventKind::MembershipMilestone
            | MonetizationEventKind::MembershipGift => "#0F9D58".to_string(),
        };
        let msg = if message.trim().is_empty() {
            "テスト送信の応援メッセージです！".to_string()
        } else {
            message.trim().to_string()
        };

        let session_id = {
            let sid = self.connection_state.read().await.session_id.clone();
            if sid.is_empty() {
                "demo-local".to_string()
            } else {
                sid
            }
        };

        let item = MonetizationItem {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            platform: PlatformKind::Demo,
            kind,
            user_id: format!("demo-sc-{}", name),
            user_name: name.clone(),
            user_icon_url: demo::make_svg_avatar("%23f59e0b", &initial),
            amount_display: amount.clone(),
            amount_micros: 1_000_000_000,
            currency: "JPY".to_string(),
            tier_color: tier_color.clone(),
            message: msg.clone(),
            timestamp_ms: now_ms,
            is_checked: false,
            checked_at_ms: None,
        };

        let list = {
            let db = self.db.lock().await;
            db.insert_monetization_event(&item)?;
            db.list_monetization_events(150)?
        };

        if flags.obs_overlay_enabled {
            self.overlay_state
                .update_monetization(list.clone(), flags.superchat_enabled)
                .await;
        }
        self.emit(AppEvent::monetization_updated(list.clone()));

        // コメント欄へも同期表示（配信未接続でもテスト表示可能）
        if flags.comment_display_enabled {
            let c_item = CommentItem {
                id: item.id.clone(),
                session_id,
                platform: PlatformKind::Demo,
                user_id: item.user_id.clone(),
                user_name: name,
                user_icon_url: item.user_icon_url.clone(),
                user_memo: String::new(),
                is_first_time: false,
                message: msg,
                timestamp_ms: now_ms,
                is_owner: false,
                is_moderator: false,
                is_member: matches!(
                    kind,
                    MonetizationEventKind::MembershipNew
                        | MonetizationEventKind::MembershipMilestone
                        | MonetizationEventKind::MembershipGift
                ),
                is_banned: false,
                highlight_rule_id: None,
                bubble_bg_color: None,
                bubble_text_color: None,
                bubble_border_color: None,
                event_kind: Some(kind),
                amount_display: Some(amount),
                event_accent_color: Some(tier_color),
            };
            {
                let mut list_c = self.recent_comments.write().await;
                list_c.push(c_item.clone());
                if list_c.len() > 300 {
                    let over = list_c.len() - 300;
                    list_c.drain(0..over);
                }
            }
            self.emit(AppEvent::comment_received(c_item.clone()));
            if flags.obs_overlay_enabled {
                self.overlay_state.push_comment(c_item).await;
            }
        }

        Ok(list)
    }

    // ==================== リアルタイム投票 ====================

    pub async fn get_poll_state(&self) -> PollState {
        self.poll_runtime.lock().await.state.clone()
    }

    pub async fn save_poll_config(&self, config: PollConfig) -> Result<PollState> {
        {
            let db = self.db.lock().await;
            db.set_poll_config(&config)?;
        }
        let state = {
            let mut pr = self.poll_runtime.lock().await;
            pr.state.config = config;
            pr.state.clone()
        };
        let enabled = self.feature_flags.read().await.poll_enabled;
        self.overlay_state.update_poll(state.clone(), enabled).await;
        self.emit(AppEvent::poll_updated(state.clone()));
        Ok(state)
    }

    pub async fn start_poll(&self, config: PollConfig) -> Result<PollState> {
        if !self.feature_flags.read().await.poll_enabled {
            return Err(anyhow!(
                "リアルタイム投票機能は現在オフになっています。機能を有効にしてから開始してください。"
            ));
        }
        if config.options.len() < 2 {
            return Err(anyhow!("選択肢は2つ以上設定してください"));
        }
        {
            let db = self.db.lock().await;
            db.set_poll_config(&config)?;
        }
        let state = {
            let mut pr = self.poll_runtime.lock().await;
            pr.start(config)
        };
        self.overlay_state.update_poll(state.clone(), true).await;
        self.emit(AppEvent::poll_updated(state.clone()));
        Ok(state)
    }

    pub async fn stop_poll(&self) -> PollState {
        let state = {
            let mut pr = self.poll_runtime.lock().await;
            pr.stop()
        };
        let enabled = self.feature_flags.read().await.poll_enabled;
        self.overlay_state.update_poll(state.clone(), enabled).await;
        self.emit(AppEvent::poll_updated(state.clone()));
        state
    }

    pub async fn reset_poll(&self) -> PollState {
        let state = {
            let mut pr = self.poll_runtime.lock().await;
            pr.reset()
        };
        let enabled = self.feature_flags.read().await.poll_enabled;
        self.overlay_state.update_poll(state.clone(), enabled).await;
        self.emit(AppEvent::poll_updated(state.clone()));
        state
    }

    pub async fn clear_overlay_poll(&self) {
        self.overlay_state.clear_poll().await;
    }

    // ==================== コメントピックアップ ====================

    pub async fn get_picked_comment(&self) -> PickedCommentState {
        self.picked_comment.lock().await.clone()
    }

    pub async fn pick_comment(&self, comment: CommentItem) -> Result<PickedCommentState> {
        if !self.feature_flags.read().await.pickup_enabled {
            return Err(anyhow!(
                "コメントピックアップ機能は現在オフになっています。機能を有効にしてください。"
            ));
        }
        let state = PickedCommentState {
            comment: Some(comment),
            picked_at_ms: Some(Utc::now().timestamp_millis()),
        };
        *self.picked_comment.lock().await = state.clone();
        self.overlay_state
            .update_picked_comment(state.clone(), true)
            .await;
        self.emit(AppEvent::picked_comment_updated(state.clone()));
        Ok(state)
    }

    pub async fn clear_picked_comment(&self) -> PickedCommentState {
        let state = PickedCommentState::default();
        *self.picked_comment.lock().await = state.clone();
        let enabled = self.feature_flags.read().await.pickup_enabled;
        self.overlay_state
            .update_picked_comment(state.clone(), enabled)
            .await;
        self.emit(AppEvent::picked_comment_updated(state.clone()));
        state
    }

    // ==================== 参加型キュー ====================

    async fn persist_and_broadcast_queue(&self, state: QueueState) {
        {
            let db = self.db.lock().await;
            let _ = db.set_queue_state(&state);
        }
        let enabled = self.feature_flags.read().await.queue_enabled;
        self.overlay_state
            .update_queue(state.clone(), enabled)
            .await;
        self.emit(AppEvent::queue_updated(state));
    }

    pub async fn get_queue_state(&self) -> QueueState {
        self.queue_runtime.lock().await.state.clone()
    }

    pub async fn save_queue_config(&self, config: QueueConfig) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.update_config(config)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_add_manual(&self, user_name: &str, note: &str) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.add_manual(user_name, note)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_set_status(&self, user_id: &str, status: QueueEntryStatus) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.set_status(user_id, status)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_promote_next(&self) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.promote_next()
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_rotate_active(&self) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.rotate_active()
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_move_entry(&self, user_id: &str, delta: i32) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.move_entry(user_id, delta)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_remove_entry(&self, user_id: &str) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.remove_entry(user_id)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    pub async fn queue_reset(&self, reset_play_counts: bool) -> QueueState {
        let state = {
            let mut qr = self.queue_runtime.lock().await;
            qr.reset(reset_play_counts)
        };
        self.persist_and_broadcast_queue(state.clone()).await;
        state
    }

    // ==================== 特定ワード反応メディアアニメーション ====================

    pub fn import_media_file(&self, source_path: &str) -> Result<ImportedMediaFile> {
        media::import_media_file(&self.media_dir, source_path)
    }

    pub async fn list_media_reaction_rules(&self) -> Result<Vec<MediaReactionRule>> {
        Ok(self.cached_media_rules.read().await.clone())
    }

    pub async fn save_media_reaction_rule(
        &self,
        mut rule: MediaReactionRule,
    ) -> Result<Vec<MediaReactionRule>> {
        if rule.id.trim().is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        let list = {
            let db = self.db.lock().await;
            db.upsert_media_reaction_rule(&rule)?;
            db.list_media_reaction_rules()?
        };
        *self.cached_media_rules.write().await = list.clone();
        self.emit(AppEvent::media_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn delete_media_reaction_rule(&self, id: &str) -> Result<Vec<MediaReactionRule>> {
        let (list, unused_files) = {
            let db = self.db.lock().await;
            let files = db.delete_media_reaction_rule(id)?;
            (db.list_media_reaction_rules()?, files)
        };
        for fname in unused_files {
            media::remove_stored_media_file(&self.media_dir, &fname);
        }
        *self.cached_media_rules.write().await = list.clone();
        self.emit(AppEvent::media_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn get_media_reaction_config(&self) -> MediaReactionConfig {
        self.media_config.lock().await.clone()
    }

    pub async fn save_media_reaction_config(
        &self,
        config: MediaReactionConfig,
    ) -> Result<MediaReactionConfig> {
        {
            let db = self.db.lock().await;
            db.set_media_reaction_config(&config)?;
        }
        *self.media_config.lock().await = config.clone();
        let enabled = self.feature_flags.read().await.media_reaction_enabled;
        self.overlay_state
            .update_media_config(config.clone(), enabled)
            .await;
        self.emit(AppEvent::media_config_updated(config.clone()));
        Ok(config)
    }

    pub async fn test_media_reaction_rule(&self, id: &str) -> Result<()> {
        let flags = self.feature_flags.read().await.clone();
        if !flags.media_reaction_enabled {
            return Err(anyhow!(
                "特定ワード反応アニメーション機能は現在オフになっています。機能を有効にしてから実行してください。"
            ));
        }
        let rule = {
            self.cached_media_rules
                .read()
                .await
                .iter()
                .find(|r| r.id == id)
                .cloned()
                .ok_or_else(|| anyhow!("指定された演出ルールが見つかりません"))?
        };
        if !rule_has_playable_content(&rule) {
            return Err(anyhow!(
                "再生する画像・動画・音声ファイル、またはテロップを設定してください"
            ));
        }
        let now_ms = Utc::now().timestamp_millis();
        let sample_msg = rule.pattern.lines().next().unwrap_or(&rule.name);
        let trig = build_trigger_event(&rule, "テスト視聴者", sample_msg, now_ms);
        if flags.obs_overlay_enabled {
            self.overlay_state.trigger_media(trig.clone());
        }
        self.emit(AppEvent::media_triggered(trig));
        Ok(())
    }

    pub fn clear_overlay_media(&self) {
        self.overlay_state.clear_media();
    }
}
