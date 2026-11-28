use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::OnceCell;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::backup;
use crate::db::Database;
use crate::engine::media_reaction::{
    build_trigger_event, rule_has_playable_content, MediaCooldownTracker, MediaEvalContext,
};
use crate::engine::ng_word;
use crate::engine::poll::PollRuntime;
use crate::engine::queue::QueueRuntime;
use crate::engine::{counter, highlight, lottery, matches_pattern};
use crate::hotkeys::HotkeyManager;
use crate::license;
use crate::media;
use crate::models::{
    AppEvent, AutomationActionKind, BackupExportResult, BackupImportResult, CommentItem,
    CommentLotteryConfig, CommentLotteryState, ConnectionHistoryItem, ConnectionMode,
    ConnectionStateDto, ConnectionStatus, FeatureFlags, FirstTimeCounterConfig,
    FirstTimeCounterState, FirstTimeUser, HighlightRule, HotkeyBinding, ImportedMediaFile,
    LotteryResult, LotteryWinner, MatchMode, MediaReactionConfig, MediaReactionRule, MessageRun,
    MessageRunKind, MonetizationEventKind, MonetizationItem, NgWordRule, ObsActionType,
    ObsAutomationRule, ObsTriggerType, ObsWsConfig, ObsWsStatus, OnAirReportDto, OverlayConfig,
    PickedCommentState, PlatformKind, PollConfig, PollState, QueueConfig, QueueEntryStatus,
    QueueState, RawIncomingComment, ReleaseNotesInfo, RemoteAccessConfig, RemoteAccessStatus,
    ResolvedYouTubeLive, SavedChannelItem, StreamProfile, StreamProfileSummary,
    StreamSessionSummary, UserLotteryRequest, UserRecord, WordCounterConfig,
};
use crate::obs_ws::{ObsIncomingEvent, ObsWsManager};
use crate::overlay::{start_overlay_server, OverlayServerHandle, OverlaySharedState};
use crate::platform::{demo, youtube, ProviderConnection};
use crate::profile;
use crate::remote;
use crate::report;
use crate::updater;

static RUNTIME: OnceCell<Arc<AppRuntime>> = OnceCell::new();

pub struct AppRuntime {
    pub db: Arc<Mutex<Database>>,
    pub db_path: PathBuf,
    pub media_dir: PathBuf,
    pub overlay_state: Arc<OverlaySharedState>,
    pub overlay_server: Mutex<Option<OverlayServerHandle>>,
    pub remote_config: RwLock<RemoteAccessConfig>,
    pub feature_flags: RwLock<FeatureFlags>,
    pub connection_state: RwLock<ConnectionStateDto>,
    pub active_provider: Mutex<Option<ProviderConnection>>,
    pub recent_comments: RwLock<VecDeque<CommentItem>>,
    pub user_lottery_won_ids: Mutex<HashSet<String>>,
    pub comment_lottery: Mutex<CommentLotteryState>,
    pub first_time_state: Mutex<FirstTimeCounterState>,
    pub poll_runtime: Mutex<PollRuntime>,
    pub picked_comment: Mutex<PickedCommentState>,
    pub queue_runtime: Mutex<QueueRuntime>,
    pub media_config: Mutex<MediaReactionConfig>,
    pub media_cooldown: Mutex<MediaCooldownTracker>,
    pub cached_highlight_rules: RwLock<Arc<Vec<HighlightRule>>>,
    pub cached_media_rules: RwLock<Arc<Vec<MediaReactionRule>>>,
    pub cached_word_counters: RwLock<Arc<Vec<WordCounterConfig>>>,
    pub cached_ng_word_rules: RwLock<Arc<Vec<NgWordRule>>>,
    pub obs_ws: Arc<ObsWsManager>,
    pub obs_ws_config: RwLock<ObsWsConfig>,
    pub obs_ws_event_tx: mpsc::UnboundedSender<ObsIncomingEvent>,
    pub hotkey_manager: Arc<HotkeyManager>,
    pub hotkey_bindings: RwLock<Vec<HotkeyBinding>>,
    pub active_profile_id: RwLock<String>,
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
    let data_dir = db_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let media_dir = data_dir.join("media");
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
    let initial_ng_rules = db.list_ng_word_rules()?;
    let initial_obs_ws_cfg = db.get_obs_ws_config()?;
    let initial_hotkeys = db.get_hotkey_bindings()?;
    let initial_active_profile = db.get_active_profile_id()?;

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
    let (obs_ws_event_tx, obs_ws_event_rx) = mpsc::unbounded_channel::<ObsIncomingEvent>();
    let obs_ws = Arc::new(ObsWsManager::new());
    let hotkey_manager = Arc::new(HotkeyManager::new());
    let hotkey_rx = hotkey_manager.start(initial_hotkeys.clone());

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

    let auto_connect_obs = initial_obs_ws_cfg.enabled && initial_obs_ws_cfg.auto_connect;
    let obs_cfg_for_auto = initial_obs_ws_cfg.clone();

    let runtime = Arc::new(AppRuntime {
        db: db_arc,
        db_path,
        media_dir,
        overlay_state,
        overlay_server: Mutex::new(server_handle),
        remote_config: RwLock::new(remote_config),
        feature_flags: RwLock::new(feature_flags),
        connection_state: RwLock::new(ConnectionStateDto::default()),
        active_provider: Mutex::new(None),
        recent_comments: RwLock::new(VecDeque::new()),
        user_lottery_won_ids: Mutex::new(HashSet::new()),
        comment_lottery: Mutex::new(CommentLotteryState::default()),
        first_time_state: Mutex::new(first_time_state),
        poll_runtime: Mutex::new(poll_rt),
        picked_comment: Mutex::new(PickedCommentState::default()),
        queue_runtime: Mutex::new(queue_rt),
        media_config: Mutex::new(media_config),
        media_cooldown: Mutex::new(MediaCooldownTracker::default()),
        cached_highlight_rules: RwLock::new(Arc::new(initial_highlight_rules)),
        cached_media_rules: RwLock::new(Arc::new(initial_media_rules)),
        cached_word_counters: RwLock::new(Arc::new(initial_word_counters)),
        cached_ng_word_rules: RwLock::new(Arc::new(initial_ng_rules)),
        obs_ws,
        obs_ws_config: RwLock::new(initial_obs_ws_cfg),
        obs_ws_event_tx,
        hotkey_manager,
        hotkey_bindings: RwLock::new(initial_hotkeys),
        active_profile_id: RwLock::new(initial_active_profile),
        last_session_id: Mutex::new(String::new()),
        event_tx,
        comment_tx,
        launched_after_update: RwLock::new(after_update),
    });

    // バックグラウンドコメント処理ループを開始
    spawn_comment_processor(Arc::clone(&runtime), comment_rx);
    spawn_obs_ws_event_processor(Arc::clone(&runtime), obs_ws_event_rx);
    spawn_hotkey_processor(Arc::clone(&runtime), hotkey_rx);

    // 1日1回の自動世代バックアップは起動クリティカルパスから切り離しバックグラウンド実行
    {
        let db_for_backup = Arc::clone(&runtime.db);
        let backup_data_dir = data_dir.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            // 本日分のバックアップ有無は DB ロックなしでブロッキングプール上で事前確認
            let check_dir = backup_data_dir.clone();
            let needs = tokio::task::spawn_blocking(move || {
                backup::needs_auto_backup_today(&check_dir).unwrap_or(false)
            })
            .await
            .unwrap_or(false);
            if !needs {
                return;
            }
            let created = {
                let db_guard = db_for_backup.lock().await;
                backup::create_auto_backup_snapshot(&db_guard, &backup_data_dir).is_ok()
            };
            if created {
                let prune_dir = backup_data_dir;
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = backup::prune_old_auto_backups(&prune_dir);
                })
                .await;
            }
        });
    }

    if auto_connect_obs {
        let rt_clone = Arc::clone(&runtime);
        tokio::spawn(async move {
            let _ = rt_clone
                .obs_ws
                .connect(obs_cfg_for_auto, rt_clone.obs_ws_event_tx.clone())
                .await;
        });
    }

    let _ = RUNTIME.set(Arc::clone(&runtime));
    Ok(runtime)
}

fn spawn_obs_ws_event_processor(
    runtime: Arc<AppRuntime>,
    mut rx: mpsc::UnboundedReceiver<ObsIncomingEvent>,
) {
    tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                ObsIncomingEvent::StatusChanged(status) => {
                    runtime.emit(AppEvent::obs_ws_status_changed(status));
                }
                ObsIncomingEvent::StreamStarted => {
                    let cfg = runtime.obs_ws_config.read().await.clone();
                    if cfg.reset_first_time_on_stream_start {
                        let _ = runtime.reset_first_time_session_count().await;
                    }
                    if cfg.reset_counters_on_stream_start {
                        let ids: Vec<String> = runtime
                            .cached_word_counters
                            .read()
                            .await
                            .iter()
                            .map(|c| c.id.clone())
                            .collect();
                        for cid in ids {
                            let _ = runtime.reset_word_counter(&cid).await;
                        }
                    }
                    runtime.emit(AppEvent::system_notice(
                        "info",
                        "OBS の配信開始を検知しました",
                    ));
                }
                ObsIncomingEvent::StreamStopped => {
                    runtime.emit(AppEvent::system_notice(
                        "info",
                        "OBS の配信終了を検知しました",
                    ));
                }
            }
        }
    });
}

fn spawn_hotkey_processor(
    runtime: Arc<AppRuntime>,
    mut rx: mpsc::UnboundedReceiver<String>,
) {
    tokio::spawn(async move {
        while let Some(binding_id) = rx.recv().await {
            if let Err(e) = runtime.execute_hotkey_by_id(&binding_id).await {
                runtime.emit(AppEvent::system_notice(
                    "warning",
                    format!("ホットキー実行エラー: {}", e),
                ));
            }
        }
    });
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
    async fn handle_incoming_comment(&self, mut raw: RawIncomingComment) -> Result<()> {
        let session_id = self.connection_state.read().await.session_id.clone();
        if session_id.is_empty() {
            return Ok(());
        }

        let flags = self.feature_flags.read().await.clone();

        // NG ワード防御評価（有効かつルール存在時のみインプレース評価・非ヒット時ゼロアロケーション）
        let ng_flags = if flags.ng_word_filter_enabled {
            let ng_rules = Arc::clone(&*self.cached_ng_word_rules.read().await);
            if !ng_rules.is_empty() {
                ng_word::evaluate_ng_words_in_place(&mut raw.message, &mut raw.message_runs, &ng_rules)
            } else {
                ng_word::NgWordFlags::default()
            }
        } else {
            ng_word::NgWordFlags::default()
        };

        if ng_flags.drop_everywhere {
            return Ok(());
        }

        let hidden_from_overlay = ng_flags.hide_from_overlay;
        let is_ng_masked = ng_flags.masked;

        // コメントを利用する全機能が OFF でもオンエアレポート用に最低限のセッション記録のみ考慮
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

        // ルール群は Arc::clone のみで共有取得（毎コメントの Vec 複製を撤廃）
        let rules = if flags.word_highlight_enabled && flags.comment_display_enabled {
            Some(Arc::clone(&*self.cached_highlight_rules.read().await))
        } else {
            None
        };
        let media_rules = if flags.media_reaction_enabled {
            Some(Arc::clone(&*self.cached_media_rules.read().await))
        } else {
            None
        };
        let word_counters = if flags.word_counter_enabled {
            Some(Arc::clone(&*self.cached_word_counters.read().await))
        } else {
            None
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

        // コメント1件に関わるすべての SQLite 書き込みを単一 SAVEPOINT トランザクションへ集約（WAL 同期回数 3〜6回 -> 1回）
        let (
            user_meta,
            counters_changed,
            updated_counters,
            first_time_trigger,
            new_all_time_ft,
            updated_monetization,
        ) = {
            let db = self.db.lock().await;
            db.in_savepoint("incoming_comment", |db| {
                let meta = if flags.user_tracking_enabled {
                    db.record_user_comment_inner(&session_id, &raw)?
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
                let counters_changed = if let Some(ref wc) = word_counters {
                    if !wc.is_empty() {
                        counter::process_counters_for_comment_with_list(
                            db,
                            wc,
                            &raw.user_id,
                            meta.is_banned,
                            &raw.message,
                        )?
                    } else {
                        false
                    }
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
                    if !meta.already_first_time_detected
                        && (!ft_cfg.exclude_banned || !meta.is_banned)
                    {
                        let kw_matched = ft_cfg.detect_keyword
                            && ft_cfg.keywords.iter().any(|kw| {
                                matches_pattern(&raw.message, kw, MatchMode::Contains, false)
                            });
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

                // オンエアレポート用の軽量セッションコメント履歴を記録
                let _ = db.insert_session_comment(&session_id, &raw, trigger_reason.is_some());

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
                            message_runs: raw.message_runs.clone(),
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

                Ok((
                    meta,
                    counters_changed,
                    updated_counters,
                    trigger_reason,
                    new_all_time,
                    updated_monetization,
                ))
            })?
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
        let matched_rule = rules
            .as_deref()
            .and_then(|r| highlight::evaluate_highlight(&raw.message, r));

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
            message_runs: raw.message_runs,
            timestamp_ms: raw.timestamp_ms,
            is_owner: raw.is_owner,
            is_moderator: raw.is_moderator,
            is_member: raw.is_member,
            is_banned,
            is_ng_masked,
            hidden_from_overlay,
            highlight_rule_id: matched_rule.map(|r| r.id.clone()),
            bubble_bg_color: matched_rule.map(|r| r.bg_color.clone()),
            bubble_text_color: matched_rule.map(|r| r.text_color.clone()),
            bubble_border_color: matched_rule.map(|r| r.border_color.clone()),
            event_kind: raw.event_kind,
            amount_display: raw.amount_display,
            event_accent_color: raw.tier_color,
        };

        // コメント表示機能が有効な場合のみ UI ＆ OBS へ反映（NG の OBS 非表示時は OBS へ流さない）
        if flags.comment_display_enabled {
            {
                let mut list = self.recent_comments.write().await;
                list.push_back(item.clone());
                while list.len() > 300 {
                    list.pop_front();
                }
            }
            self.emit(AppEvent::comment_received(item.clone()));
            if flags.obs_overlay_enabled && !item.hidden_from_overlay {
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
        let media_triggers = if let Some(ref m_rules) = media_rules {
            if !m_rules.is_empty() {
                let allow_multiple = self.media_config.lock().await.allow_multiple_match;
                let mut cd = self.media_cooldown.lock().await;
                cd.evaluate_rules(
                    m_rules,
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
            }
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
                *self.cached_word_counters.write().await = Arc::new(counters.clone());
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

        // OBS WebSocket 自動連動ルールの評価（非同期実行）
        self.evaluate_obs_rules_for_comment(&item, raw.amount_micros, is_first_time)
            .await;

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
                Some(list.iter().cloned().collect())
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
        let resolved = youtube::resolve_youtube_live_target(input).await?;
        let video_id = resolved.video_id.clone();
        self.disconnect_current().await;

        {
            let mut st = self.connection_state.write().await;
            st.status = ConnectionStatus::Connecting;
            st.platform = PlatformKind::YouTube;
            st.mode = mode;
            st.stream_id = video_id.clone();
            st.error_message = None;
            st.reconnect_attempt = 0;
            let dto = st.clone();
            drop(st);
            self.emit(AppEvent::connection_changed(dto));
        }

        let runtime_ev = self.event_tx.clone();
        let on_error = move |msg: String| {
            let _ = runtime_ev.send(AppEvent::system_notice("error", msg));
        };

        let on_status = move |status: ConnectionStatus, attempt: u32, err_msg: Option<String>| {
            if let Ok(rt) = get_runtime() {
                tokio::spawn(async move {
                    let dto = {
                        let mut st = rt.connection_state.write().await;
                        if st.status == ConnectionStatus::Disconnected
                            && status == ConnectionStatus::Reconnecting
                        {
                            return;
                        }
                        st.status = status;
                        st.reconnect_attempt = attempt;
                        st.error_message = err_msg.clone();
                        st.clone()
                    };
                    rt.emit(AppEvent::connection_changed(dto));
                    if status == ConnectionStatus::Connected && attempt == 0 {
                        rt.emit(AppEvent::system_notice(
                            "success",
                            "YouTube チャットへの自動再接続が完了しました",
                        ));
                    }
                });
            }
        };

        let conn_res = match mode {
            ConnectionMode::YouTubeDataApi => {
                youtube::connect_youtube_data_api(
                    &video_id,
                    Arc::clone(&self.db),
                    self.comment_tx.clone(),
                    on_error,
                    on_status,
                )
                .await
            }
            _ => {
                youtube::connect_youtube_innertube(
                    &video_id,
                    Arc::clone(&self.db),
                    self.comment_tx.clone(),
                    on_error,
                    on_status,
                )
                .await
            }
        };

        match conn_res {
            Ok(conn) => {
                let now_ms = Utc::now().timestamp_millis();
                let session_id = {
                    let db = self.db.lock().await;
                    let sid = db.get_or_create_session(
                        PlatformKind::YouTube,
                        &conn.stream_id,
                        &conn.stream_title,
                        now_ms,
                    )?;
                    let hist = ConnectionHistoryItem {
                        id: format!("yt:{}", conn.stream_id),
                        platform: PlatformKind::YouTube,
                        stream_id: conn.stream_id.clone(),
                        input_url: input.trim().to_string(),
                        stream_title: conn.stream_title.clone(),
                        mode,
                        last_connected_ms: now_ms,
                        is_favorite: false,
                    };
                    let _ = db.upsert_connection_history(&hist);
                    sid
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
                    reconnect_attempt: 0,
                };
                *self.active_provider.lock().await = Some(conn);
                *self.connection_state.write().await = dto.clone();
                self.maybe_auto_reset_first_time_for_session(&dto.session_id)
                    .await;
                self.emit(AppEvent::connection_changed(dto.clone()));
                self.emit(AppEvent::connection_history_updated());
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
                    reconnect_attempt: 0,
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
            reconnect_attempt: 0,
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
                    message_runs: Vec::new(),
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
        // ipconfig 起動を含む LAN IP 検出をブロッキングプールへ逃がし、Tokio ワーカースレッドを占有しない
        let cfg_clone = cfg.clone();
        tokio::task::spawn_blocking(move || {
            remote::build_remote_access_status(cfg_clone, running, port)
        })
        .await
        .unwrap_or_else(|_| remote::build_remote_access_status(cfg, running, port))
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
        self.overlay_state.update_remote_config(cfg.clone()).await;

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
        Ok((**self.cached_highlight_rules.read().await).clone())
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
        *self.cached_highlight_rules.write().await = Arc::new(list.clone());
        self.emit(AppEvent::highlight_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn delete_highlight_rule(&self, id: &str) -> Result<Vec<HighlightRule>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_highlight_rule(id)?;
            db.list_highlight_rules()?
        };
        *self.cached_highlight_rules.write().await = Arc::new(list.clone());
        self.emit(AppEvent::highlight_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn list_word_counters(&self) -> Result<Vec<WordCounterConfig>> {
        Ok((**self.cached_word_counters.read().await).clone())
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
        *self.cached_word_counters.write().await = Arc::new(list.clone());
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
        *self.cached_word_counters.write().await = Arc::new(list.clone());
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
        *self.cached_word_counters.write().await = Arc::new(list.clone());
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
        *self.cached_word_counters.write().await = Arc::new(list.clone());
        self.sync_counters_to_overlay_and_clients(list.clone())
            .await;
        Ok(list)
    }

    pub async fn draw_user_lottery(&self, req: UserLotteryRequest) -> Result<LotteryResult> {
        let flags = self.feature_flags.read().await.clone();
        if !flags.user_lottery_enabled {
            return Err(anyhow!(
                "ドローボックス（ユーザー抽選）は現在オフになっています。機能を有効にしてから実行してください。"
            ));
        }
        let users = self.list_users().await?;
        let (result, won_ids) = {
            let mut won = self.user_lottery_won_ids.lock().await;
            let res = lottery::draw_user_lottery(&users, &req, &mut won)?;
            let ids: Vec<String> = won.iter().cloned().collect();
            (res, ids)
        };
        {
            let db = self.db.lock().await;
            db.save_lottery_result(&result)?;
        }
        if flags.obs_overlay_enabled {
            self.overlay_state.show_lottery(result.clone()).await;
        }
        self.emit(AppEvent::user_lottery_winners_updated(won_ids));
        self.emit(AppEvent::lottery_drawn(result.clone()));
        Ok(result)
    }

    pub async fn start_comment_lottery(&self, config: CommentLotteryConfig) -> Result<CommentLotteryState> {
        if !self.feature_flags.read().await.comment_lottery_enabled {
            return Err(anyhow!(
                "ドローボックス（コメント抽選）は現在オフになっています。機能を有効にしてから開始してください。"
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
                "ドローボックス（コメント抽選）は現在オフになっています。機能を有効にしてから実行してください。"
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
        self.evaluate_obs_rules_for_lottery().await;
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
                "エールボックス機能は現在オフになっています。機能を有効にしてから実行してください。"
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
            message_runs: Vec::new(),
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
                message_runs: Vec::new(),
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
                is_ng_masked: false,
                hidden_from_overlay: false,
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
                list_c.push_back(c_item.clone());
                while list_c.len() > 300 {
                    list_c.pop_front();
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
                "ライブアンケート機能は現在オフになっています。機能を有効にしてから開始してください。"
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
                "スポットテロップ機能は現在オフになっています。機能を有効にしてください。"
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
        Ok((**self.cached_media_rules.read().await).clone())
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
        *self.cached_media_rules.write().await = Arc::new(list.clone());
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
        *self.cached_media_rules.write().await = Arc::new(list.clone());
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
                "リアクションステージ機能は現在オフになっています。機能を有効にしてから実行してください。"
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

    // ==================== 接続履歴・お気に入りチャンネル・自動検出 ====================

    pub async fn resolve_youtube_live(&self, input: &str) -> Result<ResolvedYouTubeLive> {
        youtube::resolve_youtube_live_target(input).await
    }

    pub async fn list_connection_history(&self) -> Result<Vec<ConnectionHistoryItem>> {
        let db = self.db.lock().await;
        db.list_connection_history()
    }

    pub async fn toggle_connection_history_favorite(
        &self,
        id: &str,
        favorite: bool,
    ) -> Result<Vec<ConnectionHistoryItem>> {
        let list = {
            let db = self.db.lock().await;
            db.toggle_connection_history_favorite(id, favorite)?;
            db.list_connection_history()?
        };
        self.emit(AppEvent::connection_history_updated());
        Ok(list)
    }

    pub async fn delete_connection_history(&self, id: &str) -> Result<Vec<ConnectionHistoryItem>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_connection_history(id)?;
            db.list_connection_history()?
        };
        self.emit(AppEvent::connection_history_updated());
        Ok(list)
    }

    pub async fn list_saved_channels(&self) -> Result<Vec<SavedChannelItem>> {
        let db = self.db.lock().await;
        db.get_saved_channels()
    }

    pub async fn save_saved_channel(
        &self,
        mut item: SavedChannelItem,
    ) -> Result<Vec<SavedChannelItem>> {
        if item.channel_input.trim().is_empty() {
            return Err(anyhow!("チャンネル URL または @ハンドルを入力してください"));
        }
        if item.id.trim().is_empty() {
            item.id = uuid::Uuid::new_v4().to_string();
        }
        if item.label.trim().is_empty() {
            item.label = item.channel_input.trim().to_string();
        }
        let db = self.db.lock().await;
        let mut list = db.get_saved_channels()?;
        if let Some(existing) = list.iter_mut().find(|c| c.id == item.id) {
            *existing = item;
        } else {
            list.push(item);
        }
        db.set_saved_channels(&list)?;
        Ok(list)
    }

    pub async fn delete_saved_channel(&self, id: &str) -> Result<Vec<SavedChannelItem>> {
        let db = self.db.lock().await;
        let mut list = db.get_saved_channels()?;
        list.retain(|c| c.id != id);
        db.set_saved_channels(&list)?;
        Ok(list)
    }

    // ==================== NG ワード防御ルール ====================

    pub async fn list_ng_word_rules(&self) -> Vec<NgWordRule> {
        (**self.cached_ng_word_rules.read().await).clone()
    }

    pub async fn save_ng_word_rule(&self, mut rule: NgWordRule) -> Result<Vec<NgWordRule>> {
        if rule.pattern.trim().is_empty() {
            return Err(anyhow!("NG ワードのパターンを入力してください"));
        }
        if rule.id.trim().is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        let list = {
            let db = self.db.lock().await;
            db.upsert_ng_word_rule(&rule)?;
            db.list_ng_word_rules()?
        };
        *self.cached_ng_word_rules.write().await = Arc::new(list.clone());
        self.emit(AppEvent::ng_word_rules_updated(list.clone()));
        Ok(list)
    }

    pub async fn delete_ng_word_rule(&self, id: &str) -> Result<Vec<NgWordRule>> {
        let list = {
            let db = self.db.lock().await;
            db.delete_ng_word_rule(id)?;
            db.list_ng_word_rules()?
        };
        *self.cached_ng_word_rules.write().await = Arc::new(list.clone());
        self.emit(AppEvent::ng_word_rules_updated(list.clone()));
        Ok(list)
    }

    // ==================== OBS WebSocket v5 連携 ====================

    pub async fn get_obs_ws_config(&self) -> ObsWsConfig {
        self.obs_ws_config.read().await.clone()
    }

    pub async fn save_obs_ws_config(&self, cfg: ObsWsConfig) -> Result<ObsWsConfig> {
        {
            let db = self.db.lock().await;
            db.set_obs_ws_config(&cfg)?;
        }
        *self.obs_ws_config.write().await = cfg.clone();
        if !cfg.enabled {
            self.obs_ws.disconnect().await;
            let st = self.obs_ws.get_status().await;
            self.emit(AppEvent::obs_ws_status_changed(st));
        }
        Ok(cfg)
    }

    pub async fn get_obs_ws_status(&self) -> ObsWsStatus {
        self.obs_ws.get_status().await
    }

    pub async fn connect_obs_ws(&self) -> Result<ObsWsStatus> {
        let cfg = self.obs_ws_config.read().await.clone();
        self.obs_ws
            .connect(cfg, self.obs_ws_event_tx.clone())
            .await?;
        let st = self.obs_ws.get_status().await;
        Ok(st)
    }

    pub async fn disconnect_obs_ws(&self) -> ObsWsStatus {
        self.obs_ws.disconnect().await;
        let st = self.obs_ws.get_status().await;
        self.emit(AppEvent::obs_ws_status_changed(st.clone()));
        st
    }

    pub async fn refresh_obs_scenes(&self) -> Result<()> {
        self.obs_ws.refresh_scenes().await
    }

    async fn execute_obs_rule_with(
        obs: Arc<ObsWsManager>,
        rule: &ObsAutomationRule,
    ) -> Result<()> {
        match rule.action_type {
            ObsActionType::SwitchScene => {
                obs.switch_scene(&rule.target_scene).await?;
            }
            ObsActionType::SetSceneItemVisible => {
                let applied = obs
                    .set_scene_item_visible(
                        &rule.target_scene,
                        &rule.target_source,
                        Some(rule.set_visible),
                    )
                    .await?;
                if rule.auto_revert_ms > 0 {
                    let scene = rule.target_scene.clone();
                    let source = rule.target_source.clone();
                    let delay = rule.auto_revert_ms as u64;
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        let _ = obs
                            .set_scene_item_visible(&scene, &source, Some(!applied))
                            .await;
                    });
                }
            }
            ObsActionType::ToggleSceneItem => {
                let applied = obs
                    .set_scene_item_visible(&rule.target_scene, &rule.target_source, None)
                    .await?;
                if rule.auto_revert_ms > 0 {
                    let scene = rule.target_scene.clone();
                    let source = rule.target_source.clone();
                    let delay = rule.auto_revert_ms as u64;
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        let _ = obs
                            .set_scene_item_visible(&scene, &source, Some(!applied))
                            .await;
                    });
                }
            }
        }
        Ok(())
    }

    async fn execute_obs_rule(&self, rule: &ObsAutomationRule) -> Result<()> {
        Self::execute_obs_rule_with(Arc::clone(&self.obs_ws), rule).await
    }

    pub async fn test_obs_rule(&self, rule_id: &str) -> Result<()> {
        let rule = self
            .obs_ws_config
            .read()
            .await
            .rules
            .iter()
            .find(|r| r.id == rule_id)
            .cloned()
            .ok_or_else(|| anyhow!("指定された OBS 連動ルールが見つかりません"))?;
        self.execute_obs_rule(&rule).await
    }

    async fn evaluate_obs_rules_for_comment(
        &self,
        item: &CommentItem,
        amount_micros: u64,
        is_first_time: bool,
    ) {
        // OBS 未接続時はロックもクローンも行わず即座にスキップ
        if !self.obs_ws.is_connected() {
            return;
        }
        let amount_jpy = amount_micros / 1_000_000;
        let matched_rules: Vec<ObsAutomationRule> = {
            let cfg = self.obs_ws_config.read().await;
            if !cfg.enabled || cfg.rules.is_empty() {
                return;
            }
            cfg.rules
                .iter()
                .filter(|rule| {
                    if !rule.enabled {
                        return false;
                    }
                    match rule.trigger_type {
                        ObsTriggerType::SuperChatMinAmount => {
                            matches!(
                                item.event_kind,
                                Some(
                                    MonetizationEventKind::SuperChat
                                        | MonetizationEventKind::SuperSticker
                                )
                            ) && amount_jpy >= rule.min_amount_jpy
                        }
                        ObsTriggerType::Membership => matches!(
                            item.event_kind,
                            Some(
                                MonetizationEventKind::MembershipNew
                                    | MonetizationEventKind::MembershipMilestone
                                    | MonetizationEventKind::MembershipGift
                            )
                        ),
                        ObsTriggerType::FirstTime => is_first_time,
                        ObsTriggerType::Keyword => {
                            !rule.pattern.trim().is_empty()
                                && matches_pattern(
                                    &item.message,
                                    &rule.pattern,
                                    MatchMode::Contains,
                                    false,
                                )
                        }
                        ObsTriggerType::LotteryWinner | ObsTriggerType::Manual => false,
                    }
                })
                .cloned()
                .collect()
        };

        if matched_rules.is_empty() {
            return;
        }
        // OBS WebSocket の応答待ちでコメント処理ループが詰まらないよう非同期タスクへ分離
        let obs = Arc::clone(&self.obs_ws);
        tokio::spawn(async move {
            for rule in matched_rules {
                let _ = Self::execute_obs_rule_with(Arc::clone(&obs), &rule).await;
            }
        });
    }

    async fn evaluate_obs_rules_for_lottery(&self) {
        if !self.obs_ws.is_connected() {
            return;
        }
        let matched_rules: Vec<ObsAutomationRule> = {
            let cfg = self.obs_ws_config.read().await;
            if !cfg.enabled || cfg.rules.is_empty() {
                return;
            }
            cfg.rules
                .iter()
                .filter(|rule| rule.enabled && rule.trigger_type == ObsTriggerType::LotteryWinner)
                .cloned()
                .collect()
        };
        if matched_rules.is_empty() {
            return;
        }
        let obs = Arc::clone(&self.obs_ws);
        tokio::spawn(async move {
            for rule in matched_rules {
                let _ = Self::execute_obs_rule_with(Arc::clone(&obs), &rule).await;
            }
        });
    }

    // ==================== グローバルホットキー ＆ Stream Deck アクション実行 ====================

    pub async fn list_hotkey_bindings(&self) -> Vec<HotkeyBinding> {
        self.hotkey_bindings.read().await.clone()
    }

    pub async fn save_hotkey_bindings(
        &self,
        mut bindings: Vec<HotkeyBinding>,
    ) -> Result<Vec<HotkeyBinding>> {
        for b in bindings.iter_mut() {
            if b.id.trim().is_empty() {
                b.id = uuid::Uuid::new_v4().to_string();
            }
            b.key = b.key.trim().to_ascii_uppercase();
        }
        {
            let db = self.db.lock().await;
            db.set_hotkey_bindings(&bindings)?;
        }
        *self.hotkey_bindings.write().await = bindings.clone();
        self.hotkey_manager.update_bindings(bindings.clone());
        Ok(bindings)
    }

    pub async fn execute_hotkey_by_id(&self, binding_id: &str) -> Result<String> {
        let binding = self
            .hotkey_bindings
            .read()
            .await
            .iter()
            .find(|b| b.id == binding_id)
            .cloned()
            .ok_or_else(|| anyhow!("指定されたホットキー設定が見つかりません"))?;
        self.execute_automation_action(
            binding.action,
            &binding.target_id,
            &binding.secondary_target,
        )
        .await
    }

    pub async fn execute_automation_action(
        &self,
        action: AutomationActionKind,
        target_id: &str,
        secondary_target: &str,
    ) -> Result<String> {
        match action {
            AutomationActionKind::ClearPickup => {
                self.clear_picked_comment().await;
                Ok("スポットテロップを解除しました".to_string())
            }
            AutomationActionKind::PickLatestComment => {
                let latest = self
                    .recent_comments
                    .read()
                    .await
                    .iter()
                    .rev()
                    .find(|c| !c.hidden_from_overlay)
                    .cloned()
                    .ok_or_else(|| anyhow!("表示できる最新コメントがありません"))?;
                self.pick_comment(latest).await?;
                Ok("最新コメントをスポットテロップに表示しました".to_string())
            }
            AutomationActionKind::TriggerMediaRule => {
                self.test_media_reaction_rule(target_id).await?;
                Ok("リアクションステージを発動しました".to_string())
            }
            AutomationActionKind::ClearMedia => {
                self.clear_overlay_media();
                Ok("リアクションステージをクリアしました".to_string())
            }
            AutomationActionKind::EndPoll => {
                self.stop_poll().await;
                Ok("ライブアンケートを締め切りました".to_string())
            }
            AutomationActionKind::AdvanceQueue => {
                self.queue_promote_next().await;
                Ok("エントリーレーンの次メンバーを繰り上げました".to_string())
            }
            AutomationActionKind::ResetFirstTime => {
                self.reset_first_time_session_count().await?;
                Ok("ウェルカムカウンターをリセットしました".to_string())
            }
            AutomationActionKind::ToggleFeature => {
                let mut f = self.feature_flags.read().await.clone();
                match target_id {
                    "comment_display" => f.comment_display_enabled = !f.comment_display_enabled,
                    "word_highlight" => f.word_highlight_enabled = !f.word_highlight_enabled,
                    "word_counter" => f.word_counter_enabled = !f.word_counter_enabled,
                    "first_time_counter" => {
                        f.first_time_counter_enabled = !f.first_time_counter_enabled
                    }
                    "superchat" => f.superchat_enabled = !f.superchat_enabled,
                    "poll" => f.poll_enabled = !f.poll_enabled,
                    "pickup" => f.pickup_enabled = !f.pickup_enabled,
                    "queue" => f.queue_enabled = !f.queue_enabled,
                    "media_reaction" => f.media_reaction_enabled = !f.media_reaction_enabled,
                    "ng_word_filter" => f.ng_word_filter_enabled = !f.ng_word_filter_enabled,
                    "obs_overlay" => f.obs_overlay_enabled = !f.obs_overlay_enabled,
                    _ => return Err(anyhow!("不明な機能キー: {}", target_id)),
                }
                self.update_feature_flags(f).await?;
                Ok(format!("機能 ({}) の ON/OFF を切り替えました", target_id))
            }
            AutomationActionKind::ObsSwitchScene => {
                self.obs_ws.switch_scene(target_id).await?;
                Ok(format!("OBS シーンを「{}」へ切り替えました", target_id))
            }
            AutomationActionKind::ObsToggleSource => {
                let vis = self
                    .obs_ws
                    .set_scene_item_visible(target_id, secondary_target, None)
                    .await?;
                Ok(format!(
                    "OBS ソース「{}」を{}にしました",
                    secondary_target,
                    if vis { "表示" } else { "非表示" }
                ))
            }
        }
    }

    // ==================== 配信プロファイル切替 ====================

    pub async fn list_stream_profiles(&self) -> Result<Vec<StreamProfileSummary>> {
        let active_id = self.active_profile_id.read().await.clone();
        let db = self.db.lock().await;
        let list = db.list_stream_profiles()?;
        Ok(list
            .into_iter()
            .map(|p| StreamProfileSummary {
                is_active: !active_id.is_empty() && p.id == active_id,
                id: p.id,
                name: p.name,
                description: p.description,
                updated_at_ms: p.updated_at_ms,
            })
            .collect())
    }

    pub async fn save_current_as_profile(
        &self,
        id: Option<String>,
        name: &str,
        description: &str,
    ) -> Result<Vec<StreamProfileSummary>> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("プロファイル名を入力してください"));
        }
        let prof_id = id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let flags = self.feature_flags.read().await.clone();
        let overlay = self.overlay_state.config.read().await.clone();
        let highlight_enabled_map: HashMap<String, bool> = self
            .cached_highlight_rules
            .read()
            .await
            .iter()
            .map(|r| (r.id.clone(), r.enabled))
            .collect();
        let counter_enabled_map: HashMap<String, bool> = self
            .cached_word_counters
            .read()
            .await
            .iter()
            .map(|c| (c.id.clone(), c.enabled))
            .collect();
        let media_enabled_map: HashMap<String, bool> = self
            .cached_media_rules
            .read()
            .await
            .iter()
            .map(|r| (r.id.clone(), r.enabled))
            .collect();
        let ng_word_enabled_map: HashMap<String, bool> = self
            .cached_ng_word_rules
            .read()
            .await
            .iter()
            .map(|r| (r.id.clone(), r.enabled))
            .collect();

        let prof = StreamProfile {
            id: prof_id.clone(),
            name: trimmed.to_string(),
            description: description.trim().to_string(),
            feature_flags: flags,
            overlay_config: overlay,
            highlight_enabled_map,
            counter_enabled_map,
            media_enabled_map,
            ng_word_enabled_map,
            updated_at_ms: Utc::now().timestamp_millis(),
        };
        {
            let db = self.db.lock().await;
            db.upsert_stream_profile(&prof)?;
            db.set_active_profile_id(&prof_id)?;
        }
        *self.active_profile_id.write().await = prof_id;
        let summaries = self.list_stream_profiles().await?;
        self.emit(AppEvent::profiles_updated(summaries.clone()));
        Ok(summaries)
    }

    pub async fn create_template_profile(
        &self,
        template_id: &str,
        name: &str,
    ) -> Result<Vec<StreamProfileSummary>> {
        let prof = profile::build_preset_profile(template_id, name);
        {
            let db = self.db.lock().await;
            db.upsert_stream_profile(&prof)?;
        }
        let summaries = self.list_stream_profiles().await?;
        self.emit(AppEvent::profiles_updated(summaries.clone()));
        Ok(summaries)
    }

    pub async fn apply_stream_profile(&self, id: &str) -> Result<Vec<StreamProfileSummary>> {
        let prof = {
            let db = self.db.lock().await;
            db.get_stream_profile(id)?
                .ok_or_else(|| anyhow!("指定されたプロファイルが見つかりません"))?
        };
        {
            let db = self.db.lock().await;
            db.apply_rule_enabled_maps(
                &prof.highlight_enabled_map,
                &prof.counter_enabled_map,
                &prof.media_enabled_map,
                &prof.ng_word_enabled_map,
            )?;
            db.set_active_profile_id(&prof.id)?;
        }
        *self.active_profile_id.write().await = prof.id.clone();

        // キャッシュを最新化
        let (h_rules, c_list, m_rules, ng_rules) = {
            let db = self.db.lock().await;
            (
                db.list_highlight_rules()?,
                db.list_word_counters()?,
                db.list_media_reaction_rules()?,
                db.list_ng_word_rules()?,
            )
        };
        *self.cached_highlight_rules.write().await = Arc::new(h_rules.clone());
        *self.cached_word_counters.write().await = Arc::new(c_list.clone());
        *self.cached_media_rules.write().await = Arc::new(m_rules.clone());
        *self.cached_ng_word_rules.write().await = Arc::new(ng_rules.clone());

        // オーバーレイと機能フラグを反映（ポート番号と先行版更新アナウンス設定は現在の値を維持）
        let mut next_overlay = prof.overlay_config.clone();
        next_overlay.port = self.overlay_state.config.read().await.port;
        self.save_overlay_config_and_restart(next_overlay).await?;
        let mut next_flags = prof.feature_flags.clone();
        next_flags.include_prerelease_updates =
            self.feature_flags.read().await.include_prerelease_updates;
        self.update_feature_flags(next_flags).await?;

        self.emit(AppEvent::highlight_rules_updated(h_rules));
        self.emit(AppEvent::counters_updated(c_list));
        self.emit(AppEvent::media_rules_updated(m_rules));
        self.emit(AppEvent::ng_word_rules_updated(ng_rules));

        let summaries = self.list_stream_profiles().await?;
        self.emit(AppEvent::profiles_updated(summaries.clone()));
        self.emit(AppEvent::system_notice(
            "success",
            format!("配信プロファイル「{}」へ切り替えました", prof.name),
        ));
        Ok(summaries)
    }

    pub async fn delete_stream_profile(&self, id: &str) -> Result<Vec<StreamProfileSummary>> {
        {
            let db = self.db.lock().await;
            db.delete_stream_profile(id)?;
            if *self.active_profile_id.read().await == id {
                db.set_active_profile_id("")?;
            }
        }
        if *self.active_profile_id.read().await == id {
            self.active_profile_id.write().await.clear();
        }
        let summaries = self.list_stream_profiles().await?;
        self.emit(AppEvent::profiles_updated(summaries.clone()));
        Ok(summaries)
    }

    // ==================== バックアップ / 引っ越し (ZIP) ====================

    pub async fn export_backup_zip(&self, dest_path: &str) -> Result<BackupExportResult> {
        // DB ロックは整合性スナップショット作成時のみ保持し、ZIP 圧縮・メディアコピーはロック外で並列実行
        let (temp_dir, temp_db) = {
            let db = self.db.lock().await;
            backup::stage_db_snapshot_for_export(&db)?
        };
        let media_dir = self.media_dir.clone();
        let dest = PathBuf::from(dest_path);
        tokio::task::spawn_blocking(move || {
            backup::finalize_backup_zip_from_snapshot(&temp_dir, &temp_db, &media_dir, &dest)
        })
        .await
        .map_err(|e| anyhow!("バックアップ出力タスクに失敗しました: {}", e))?
    }

    pub async fn import_backup_zip(&self, zip_path: &str) -> Result<BackupImportResult> {
        // ZIP 展開・SHA-256・整合性チェックは DB ロック外かつブロッキングスレッドで実行
        let zip_buf = PathBuf::from(zip_path);
        let staged = tokio::task::spawn_blocking(move || {
            backup::stage_and_validate_backup_zip(&zip_buf)
        })
        .await
        .map_err(|e| anyhow!("バックアップ検証タスクに失敗しました: {}", e))??;

        let res = {
            let mut db = self.db.lock().await;
            backup::apply_staged_backup_zip(&mut db, &self.db_path, &self.media_dir, staged)?
        };
        // DB 復元後に全ランタイム状態・キャッシュ・OBS オーバーレイを再同期
        let (
            overlay_cfg,
            remote_cfg,
            flags,
            h_rules,
            c_list,
            m_rules,
            ng_rules,
            obs_cfg,
            hotkeys,
            m_cfg,
            ft_cfg,
            total_ft,
            mon_items,
            poll_cfg,
            q_state,
            active_prof,
        ) = {
            let db = self.db.lock().await;
            (
                db.get_overlay_config()?,
                db.get_remote_access_config()?,
                db.get_feature_flags()?,
                db.list_highlight_rules()?,
                db.list_word_counters()?,
                db.list_media_reaction_rules()?,
                db.list_ng_word_rules()?,
                db.get_obs_ws_config()?,
                db.get_hotkey_bindings()?,
                db.get_media_reaction_config()?,
                db.get_first_time_config()?,
                db.count_all_time_first_timers()?,
                db.list_monetization_events(150)?,
                db.get_poll_config()?,
                db.get_queue_state()?,
                db.get_active_profile_id()?,
            )
        };

        *self.remote_config.write().await = remote_cfg.clone();
        self.overlay_state.update_remote_config(remote_cfg).await;
        *self.cached_highlight_rules.write().await = Arc::new(h_rules.clone());
        *self.cached_word_counters.write().await = Arc::new(c_list.clone());
        *self.cached_media_rules.write().await = Arc::new(m_rules.clone());
        *self.cached_ng_word_rules.write().await = Arc::new(ng_rules.clone());
        *self.obs_ws_config.write().await = obs_cfg;
        *self.hotkey_bindings.write().await = hotkeys.clone();
        self.hotkey_manager.update_bindings(hotkeys);
        *self.media_config.lock().await = m_cfg.clone();
        *self.active_profile_id.write().await = active_prof;
        let ft_snapshot = {
            let mut ft = self.first_time_state.lock().await;
            ft.config = ft_cfg;
            ft.total_detected_all_time = total_ft;
            ft.clone()
        };
        let poll_snapshot = {
            let mut pr = self.poll_runtime.lock().await;
            pr.state.config = poll_cfg;
            pr.state.clone()
        };
        *self.queue_runtime.lock().await = QueueRuntime::from_state(q_state.clone());

        self.save_overlay_config_and_restart(overlay_cfg).await?;
        self.update_feature_flags(flags).await?;

        self.emit(AppEvent::highlight_rules_updated(h_rules));
        self.emit(AppEvent::counters_updated(c_list));
        self.emit(AppEvent::media_rules_updated(m_rules));
        self.emit(AppEvent::media_config_updated(m_cfg));
        self.emit(AppEvent::ng_word_rules_updated(ng_rules));
        self.emit(AppEvent::first_time_updated(ft_snapshot));
        self.emit(AppEvent::poll_updated(poll_snapshot));
        self.emit(AppEvent::monetization_updated(mon_items));
        self.emit(AppEvent::queue_updated(q_state));
        self.emit(AppEvent::users_updated());
        if let Ok(profs) = self.list_stream_profiles().await {
            self.emit(AppEvent::profiles_updated(profs));
        }

        Ok(res)
    }

    // ==================== オンエアレポート ====================

    pub async fn list_stream_sessions(&self, limit: usize) -> Result<Vec<StreamSessionSummary>> {
        let db = self.db.lock().await;
        db.list_stream_session_summaries(limit)
    }

    pub async fn get_on_air_report(&self, session_id: &str) -> Result<OnAirReportDto> {
        // DB 読み出し直後にロックを解放し、重いトークン集計・ソートはブロッキングスレッドへ逃がす
        let (summary, rows) = {
            let db = self.db.lock().await;
            let summary = db
                .get_stream_session_summary(session_id)?
                .ok_or_else(|| anyhow!("指定された配信セッションが見つかりません"))?;
            let rows = db.list_session_comments(session_id)?;
            (summary, rows)
        };
        tokio::task::spawn_blocking(move || report::build_on_air_report_from_rows(summary, &rows))
            .await
            .map_err(|e| anyhow!("レポート集計タスクに失敗しました: {}", e))
    }

    pub async fn delete_stream_session(&self, session_id: &str) -> Result<Vec<StreamSessionSummary>> {
        let db = self.db.lock().await;
        db.delete_stream_session(session_id)?;
        db.list_stream_session_summaries(50)
    }

    pub async fn export_report_csv(&self, session_id: &str, dest_path: &str) -> Result<String> {
        let rows = {
            let db = self.db.lock().await;
            db.list_session_comments(session_id)?
        };
        let dest = PathBuf::from(dest_path);
        tokio::task::spawn_blocking(move || report::write_session_csv_from_rows(&rows, &dest))
            .await
            .map_err(|e| anyhow!("CSV 出力タスクに失敗しました: {}", e))?
    }

    pub async fn export_report_json(&self, session_id: &str, dest_path: &str) -> Result<String> {
        let (summary, rows) = {
            let db = self.db.lock().await;
            let summary = db
                .get_stream_session_summary(session_id)?
                .ok_or_else(|| anyhow!("指定された配信セッションが見つかりません"))?;
            let rows = db.list_session_comments(session_id)?;
            (summary, rows)
        };
        let dest = PathBuf::from(dest_path);
        tokio::task::spawn_blocking(move || {
            report::write_session_json_from_rows(summary, &rows, &dest)
        })
        .await
        .map_err(|e| anyhow!("JSON 出力タスクに失敗しました: {}", e))?
    }

    // ==================== オーバーレイスタジオ: プレビュー専用テスト演出送信 ====================

    /// デザイン調整時にプレビュー専用環境（/overlay/preview/*）へサンプルデータを即時送信する。
    /// 本番オーバーレイ（/overlay/*）や本番の配信データ・DB には一切影響しない。
    pub async fn send_overlay_test_event(&self, target: &str) -> Result<String> {
        let now_ms = Utc::now().timestamp_millis();
        let seq = {
            let mut store = self.overlay_state.preview_store.write().await;
            store.sample_seq = store.sample_seq.wrapping_add(1);
            store.sample_seq
        };

        match target {
            "comments" => {
                let (user_name, color, initial, msg, is_first, is_member, stamp_bg, stamp_glyph) =
                    match seq % 3 {
                        0 => (
                            "スタジオ視聴者",
                            "%232563eb",
                            "視",
                            "プレビュー確認用のコメントです！文字の縁取りや背景の透過をチェックできます。",
                            false,
                            true,
                            "%23eab308",
                            "OK",
                        ),
                        1 => (
                            "初見のねこ",
                            "%23f59e0b",
                            "ね",
                            "初見です！レイアウトめっちゃ見やすいですね！",
                            true,
                            false,
                            "%2310b981",
                            "GG",
                        ),
                        _ => (
                            "さくらもち",
                            "%23ec4899",
                            "さ",
                            "スタンプ混在コメントの表示バランス確認テストです！",
                            false,
                            true,
                            "%23ec4899",
                            "★",
                        ),
                    };
                let stamp_url = format!(
                    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 48 48'>\
                     <rect width='48' height='48' rx='10' fill='{}'/>\
                     <text x='50%25' y='55%25' dominant-baseline='middle' text-anchor='middle' \
                     fill='white' font-family='sans-serif' font-weight='bold' font-size='20'>{}</text></svg>",
                    stamp_bg, stamp_glyph
                );
                let sample = CommentItem {
                    id: format!("preview-c-{}-{}", now_ms, seq),
                    session_id: "preview".to_string(),
                    platform: PlatformKind::Demo,
                    user_id: format!("preview-user-{}", seq % 3),
                    user_name: user_name.to_string(),
                    user_icon_url: demo::make_svg_avatar(color, initial),
                    user_memo: String::new(),
                    is_first_time: is_first,
                    message: format!("{} :preview_stamp:", msg),
                    message_runs: vec![
                        MessageRun {
                            kind: MessageRunKind::Text,
                            text: format!("{} ", msg),
                            emoji_url: None,
                            alt: None,
                            is_custom_emoji: false,
                        },
                        MessageRun {
                            kind: MessageRunKind::Emoji,
                            text: ":preview_stamp:".to_string(),
                            emoji_url: Some(stamp_url),
                            alt: Some("プレビュースタンプ".to_string()),
                            is_custom_emoji: true,
                        },
                    ],
                    timestamp_ms: now_ms,
                    is_owner: false,
                    is_moderator: false,
                    is_member,
                    is_banned: false,
                    is_ng_masked: false,
                    hidden_from_overlay: false,
                    highlight_rule_id: None,
                    bubble_bg_color: None,
                    bubble_text_color: None,
                    bubble_border_color: None,
                    event_kind: None,
                    amount_display: None,
                    event_accent_color: None,
                };
                self.overlay_state.push_preview_comment(sample).await;
                Ok("プレビュー環境 (/overlay/preview/comments) へテストコメントを送信しました（本番には影響しません）".to_string())
            }
            "pickup" => {
                let text = if seq % 2 == 0 {
                    "質問です！今日の配信で一番おすすめしたいポイントはどこですか？"
                } else {
                    "デザイン確認用のスポットテロップです！読みやすさと余白を確認できます。"
                };
                let sample = CommentItem {
                    id: format!("preview-p-{}-{}", now_ms, seq),
                    session_id: "preview".to_string(),
                    platform: PlatformKind::Demo,
                    user_id: "preview-user".to_string(),
                    user_name: if seq % 2 == 0 {
                        "たこやき太郎".to_string()
                    } else {
                        "さくらもち".to_string()
                    },
                    user_icon_url: demo::make_svg_avatar(
                        if seq % 2 == 0 { "%230ea5e9" } else { "%23ec4899" },
                        if seq % 2 == 0 { "た" } else { "さ" },
                    ),
                    user_memo: String::new(),
                    is_first_time: false,
                    message: text.to_string(),
                    message_runs: Vec::new(),
                    timestamp_ms: now_ms,
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
                    event_kind: None,
                    amount_display: None,
                    event_accent_color: None,
                };
                let picked = PickedCommentState {
                    comment: Some(sample),
                    picked_at_ms: Some(now_ms),
                };
                self.overlay_state
                    .update_preview_picked_comment(picked)
                    .await;
                Ok("プレビュー環境 (/overlay/preview/pickup) へテストテロップを送信しました（本番には影響しません）".to_string())
            }
            "superchat" => {
                let (kind, name, avatar_bg, initial, amount, micros, tier, msg) = match seq % 3 {
                    0 => (
                        MonetizationEventKind::SuperChat,
                        "さくらもち",
                        "%23ec4899",
                        "さ",
                        "¥1,000",
                        1_000_000_000,
                        "#FFCA28",
                        "プレビュー用の応援スーパーチャットです！",
                    ),
                    1 => (
                        MonetizationEventKind::SuperChat,
                        "たこやき太郎",
                        "%23ef4444",
                        "た",
                        "¥10,000",
                        10_000_000_000,
                        "#E62117",
                        "赤スパチャ表示テスト！文字の視認性とカード装飾を確認できます！",
                    ),
                    _ => (
                        MonetizationEventKind::MembershipNew,
                        "夜ふかし勢",
                        "%2322c55e",
                        "夜",
                        "メンバー加入",
                        0,
                        "#2BA640",
                        "新しくメンバーシップに参加しました！（プレビュー）",
                    ),
                };
                let sample = MonetizationItem {
                    id: format!("preview-sc-{}-{}", now_ms, seq),
                    session_id: "preview".to_string(),
                    platform: PlatformKind::Demo,
                    kind,
                    user_id: format!("preview-sc-{}", seq % 3),
                    user_name: name.to_string(),
                    user_icon_url: demo::make_svg_avatar(avatar_bg, initial),
                    amount_display: amount.to_string(),
                    amount_micros: micros,
                    currency: "JPY".to_string(),
                    tier_color: tier.to_string(),
                    message: msg.to_string(),
                    message_runs: Vec::new(),
                    timestamp_ms: now_ms,
                    is_checked: false,
                    checked_at_ms: None,
                };
                let mut items = self
                    .overlay_state
                    .preview_store
                    .read()
                    .await
                    .monetization_items
                    .clone();
                items.insert(0, sample);
                items.truncate(8);
                self.overlay_state.update_preview_monetization(items).await;
                Ok("プレビュー環境 (/overlay/preview/superchat) へテストスパチャを送信しました（本番には影響しません）".to_string())
            }
            "lottery" => {
                let sample = LotteryResult {
                    id: format!("preview-lot-{}-{}", now_ms, seq),
                    kind: "プレビュー抽選".to_string(),
                    winners: vec![LotteryWinner {
                        user_id: format!("preview-w-{}", seq),
                        user_name: if seq % 2 == 0 {
                            "ラッキー視聴者".to_string()
                        } else {
                            "たこやき太郎".to_string()
                        },
                        user_icon_url: demo::make_svg_avatar("%23eab308", "当"),
                        comment_text: Some(
                            "参加希望します！当たりますように！（プレビューテスト）".to_string(),
                        ),
                        won_at_ms: now_ms,
                    }],
                    candidate_count: 24 + (seq % 10) as u32,
                    drawn_at_ms: now_ms,
                };
                self.overlay_state.show_preview_lottery(sample).await;
                Ok("プレビュー環境 (/overlay/preview/lottery) へテスト当選結果を送信しました（本番には影響しません）".to_string())
            }
            "poll" => {
                let mut sample = self
                    .overlay_state
                    .preview_store
                    .read()
                    .await
                    .poll_state
                    .clone();
                sample.is_active = true;
                sample.started_at_ms = Some(now_ms);
                if sample.config.options.len() >= 3 {
                    let idx = (seq as usize) % 3;
                    sample.config.options[idx].vote_count += 5;
                    sample.total_votes = sample
                        .config
                        .options
                        .iter()
                        .map(|o| o.vote_count)
                        .sum();
                }
                self.overlay_state.update_preview_poll(sample).await;
                Ok("プレビュー環境 (/overlay/preview/poll) へテスト投票データを送信しました（本番には影響しません）".to_string())
            }
            "first_time" => {
                let mut ft = self
                    .overlay_state
                    .preview_store
                    .read()
                    .await
                    .first_time
                    .clone();
                ft.count += 1;
                ft.total_detected_all_time += 1;
                ft.config.show_first_timer_profile = true;
                ft.recent_users.insert(
                    0,
                    FirstTimeUser {
                        user_id: format!("preview-ft-{}", seq),
                        user_name: format!("プレビュー初見さん#{}", ft.count),
                        user_icon_url: demo::make_svg_avatar("%2338bdf8", "初"),
                        trigger_reason: "初見ワード".to_string(),
                        detected_at_ms: now_ms,
                    },
                );
                ft.recent_users.truncate(5);
                self.overlay_state.update_preview_first_time(ft).await;
                Ok("プレビュー環境 (/overlay/preview/first_time) へテスト初見カウントを送信しました（本番には影響しません）".to_string())
            }
            "counter" => {
                let mut counters = self
                    .overlay_state
                    .preview_store
                    .read()
                    .await
                    .counters
                    .clone();
                for (i, c) in counters.iter_mut().enumerate() {
                    c.total_count += if i == 0 { 3 } else { 1 };
                    c.unique_user_count += 1;
                }
                self.overlay_state.update_preview_counters(counters).await;
                Ok("プレビュー環境 (/overlay/preview/counter) へテストカウントを送信しました（本番には影響しません）".to_string())
            }
            "queue" => {
                let mut q = self
                    .overlay_state
                    .preview_store
                    .read()
                    .await
                    .queue_state
                    .clone();
                q.config.is_accepting = true;
                // プレビュー上で順番待ちの動きが見えるよう待機列をローテーション
                if q.entries.len() > 2 {
                    let tail = q.entries.pop().unwrap();
                    q.entries.insert(2, tail);
                }
                self.overlay_state.update_preview_queue(q).await;
                Ok("プレビュー環境 (/overlay/preview/queue) へテスト順番待ちデータを送信しました（本番には影響しません）".to_string())
            }
            "media" => {
                use crate::models::{
                    MediaAnimationKind, MediaEffectType, MediaKind, MediaPosition,
                    MediaTriggerEvent,
                };
                let effect = match seq % 3 {
                    0 => MediaEffectType::Confetti,
                    1 => MediaEffectType::Cracker,
                    _ => MediaEffectType::Drop,
                };
                let trigger = MediaTriggerEvent {
                    event_id: format!("preview-media-{}-{}", now_ms, seq),
                    rule_id: "preview-rule".to_string(),
                    rule_name: "プレビュー演出テスト".to_string(),
                    effect_type: effect,
                    media_url: String::new(),
                    media_kind: MediaKind::Image,
                    sound_url: None,
                    sound_volume: 0,
                    animation: MediaAnimationKind::PopIn,
                    position: MediaPosition::Center,
                    duration_ms: 4200,
                    width_percent: 36,
                    volume: 0,
                    particle_count: 40,
                    particle_size: 28,
                    particle_speed: 60,
                    gravity: 50,
                    particle_text: "🎉,✨,★,👏".to_string(),
                    ambient_preset: "sparkle".to_string(),
                    caption: format!("プレビュー演出テスト #{seq}（本番には表示されません）"),
                    triggered_by: "プレビュー視聴者".to_string(),
                    triggered_at_ms: now_ms,
                };
                self.overlay_state.trigger_preview_media(trigger);
                Ok("プレビュー環境 (/overlay/preview/media) へテスト演出を送信しました（本番には影響しません）".to_string())
            }
            _ => Ok("プレビュー環境を更新しました".to_string()),
        }
    }
}
