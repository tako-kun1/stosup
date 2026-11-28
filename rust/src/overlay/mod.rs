mod remote_api;

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Response, StatusCode};
use axum::middleware;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::{broadcast, oneshot, watch, RwLock};

use crate::media::{content_type_for_file_name, is_safe_stored_file_name};
use crate::models::{
    CommentItem, FirstTimeCounterConfig, FirstTimeCounterState, FirstTimeUser, InstalledMod,
    LotteryResult, LotteryWinner, MatchMode, MediaReactionConfig, MediaTriggerEvent, MessageRun,
    MessageRunKind, MonetizationEventKind, MonetizationItem, OverlayConfig, PickedCommentState,
    PlatformKind, PollState, QueueConfig, QueueEntry, QueueEntryStatus, QueueState,
    RemoteAccessConfig, WordCounterConfig,
};
use crate::mods::{content_type_for_mod_asset, resolve_mod_asset_path};
use crate::platform::demo;

const COMMENTS_HTML: &str = include_str!("comments.html");
const LOTTERY_HTML: &str = include_str!("lottery.html");
const COUNTER_HTML: &str = include_str!("counter.html");
const FIRST_TIME_HTML: &str = include_str!("first_time.html");
const SUPERCHAT_HTML: &str = include_str!("superchat.html");
const POLL_HTML: &str = include_str!("poll.html");
const PICKUP_HTML: &str = include_str!("pickup.html");
const QUEUE_HTML: &str = include_str!("queue.html");
const MEDIA_HTML: &str = include_str!("media.html");
const OVERLAY_COMMON_JS: &str = include_str!("stosup_overlay.js");
const OVERLAY_MOD_SDK_JS: &str = include_str!("stosup_mod_sdk.js");
const APP_ICON_ICO: &[u8] = include_bytes!("../../../windows/runner/resources/app_icon.ico");

/// OBS オーバーレイ WebSocket の購読チャンネル種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum OverlayChannel {
    All = 0,
    Comments = 1,
    SuperChat = 2,
    Pickup = 3,
    Poll = 4,
    Queue = 5,
    Media = 6,
    Lottery = 7,
    Counter = 8,
    FirstTime = 9,
}

impl OverlayChannel {
    pub const COUNT: usize = 10;

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_query(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "comments" => Some(Self::Comments),
            "superchat" => Some(Self::SuperChat),
            "pickup" => Some(Self::Pickup),
            "poll" => Some(Self::Poll),
            "queue" => Some(Self::Queue),
            "media" => Some(Self::Media),
            "lottery" => Some(Self::Lottery),
            "counter" => Some(Self::Counter),
            "first_time" => Some(Self::FirstTime),
            _ => None,
        }
    }

    /// 配信メッセージのチャンネルがクライアントの購読フィルタに一致するか判定
    pub fn matches(self, filter: Option<Self>) -> bool {
        matches!(
            (self, filter),
            (Self::All, _) | (_, None) | (_, Some(Self::All))
        ) || filter == Some(self)
    }
}

/// Snapshot ペイロード本体（`OverlayMessage` 列挙体のスタックサイズ肥大化を防ぐため Box 化）
#[derive(Debug, Clone, Serialize)]
pub struct OverlaySnapshotPayload {
    pub config: OverlayConfig,
    pub comments: Vec<CommentItem>,
    pub counters: Vec<WordCounterConfig>,
    pub last_lottery: Option<LotteryResult>,
    pub first_time: FirstTimeCounterState,
    pub first_time_enabled: bool,
    pub monetization_items: Vec<MonetizationItem>,
    pub superchat_enabled: bool,
    pub poll_state: PollState,
    pub poll_enabled: bool,
    pub picked_comment: PickedCommentState,
    pub pickup_enabled: bool,
    pub queue_state: QueueState,
    pub queue_enabled: bool,
    pub media_config: MediaReactionConfig,
    pub media_enabled: bool,
    pub mods: Vec<InstalledMod>,
    pub mods_revision: u64,
}

/// オーバーレイ画面へ WebSocket で送信するメッセージ
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OverlayMessage {
    Snapshot(Box<OverlaySnapshotPayload>),
    Comment {
        comment: CommentItem,
    },
    Counters {
        counters: Vec<WordCounterConfig>,
    },
    FirstTime {
        first_time: FirstTimeCounterState,
        enabled: bool,
    },
    LotteryResult {
        result: LotteryResult,
    },
    ClearLottery,
    SuperChat {
        items: Vec<MonetizationItem>,
        enabled: bool,
    },
    Poll {
        poll: PollState,
        enabled: bool,
    },
    ClearPoll,
    PickedComment {
        picked: PickedCommentState,
        enabled: bool,
    },
    Queue {
        queue: QueueState,
        enabled: bool,
    },
    MediaTrigger {
        trigger: MediaTriggerEvent,
    },
    MediaConfig {
        media_config: MediaReactionConfig,
        enabled: bool,
    },
    ClearMedia,
    ConfigUpdated {
        config: Box<OverlayConfig>,
    },
    ModsUpdated {
        mods: Vec<InstalledMod>,
        revision: u64,
    },
}

impl OverlayMessage {
    pub fn channel(&self) -> OverlayChannel {
        match self {
            OverlayMessage::Snapshot(_)
            | OverlayMessage::ConfigUpdated { .. }
            | OverlayMessage::ModsUpdated { .. } => OverlayChannel::All,
            OverlayMessage::Comment { .. } => OverlayChannel::Comments,
            OverlayMessage::Counters { .. } => OverlayChannel::Counter,
            OverlayMessage::FirstTime { .. } => OverlayChannel::FirstTime,
            OverlayMessage::LotteryResult { .. } | OverlayMessage::ClearLottery => {
                OverlayChannel::Lottery
            }
            OverlayMessage::SuperChat { .. } => OverlayChannel::SuperChat,
            OverlayMessage::Poll { .. } | OverlayMessage::ClearPoll => OverlayChannel::Poll,
            OverlayMessage::PickedComment { .. } => OverlayChannel::Pickup,
            OverlayMessage::Queue { .. } => OverlayChannel::Queue,
            OverlayMessage::MediaTrigger { .. }
            | OverlayMessage::MediaConfig { .. }
            | OverlayMessage::ClearMedia => OverlayChannel::Media,
        }
    }
}

/// 本番データと完全分離されたプレビュー専用（/overlay/preview/*）の状態ストア
#[derive(Debug, Clone)]
pub struct PreviewOverlayStore {
    pub comments: Vec<CommentItem>,
    pub counters: Vec<WordCounterConfig>,
    pub last_lottery: Option<LotteryResult>,
    pub first_time: FirstTimeCounterState,
    pub monetization_items: Vec<MonetizationItem>,
    pub poll_state: PollState,
    pub picked_comment: PickedCommentState,
    pub queue_state: QueueState,
    pub media_config: MediaReactionConfig,
    pub sample_seq: u64,
}

impl PreviewOverlayStore {
    /// プレビュー URL を開いた直後からデザイン確認できるよう、初期サンプルデータで初期化する
    pub fn with_default_samples() -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let stamp_ok = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 48 48'><rect width='48' height='48' rx='10' fill='%23eab308'/><text x='50%25' y='55%25' dominant-baseline='middle' text-anchor='middle' fill='white' font-family='sans-serif' font-weight='bold' font-size='20'>★</text></svg>".to_string();

        let sample_c1 = CommentItem {
            id: "preview-init-c1".to_string(),
            session_id: "preview".to_string(),
            platform: PlatformKind::Demo,
            user_id: "preview-u1".to_string(),
            user_name: "たこやき太郎".to_string(),
            user_icon_url: demo::make_svg_avatar("%230ea5e9", "た"),
            user_memo: String::new(),
            is_first_time: true,
            message: "初見です！プレビュー環境なので本番配信には影響しません！".to_string(),
            message_runs: Vec::new(),
            timestamp_ms: now_ms - 15_000,
            is_owner: false,
            is_moderator: false,
            is_member: false,
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
        let sample_c2 = CommentItem {
            id: "preview-init-c2".to_string(),
            session_id: "preview".to_string(),
            platform: PlatformKind::Demo,
            user_id: "preview-u2".to_string(),
            user_name: "さくらもち".to_string(),
            user_icon_url: demo::make_svg_avatar("%23ec4899", "さ"),
            user_memo: String::new(),
            is_first_time: false,
            message: "フォント・縁取り・背景透過の見え方を実寸で確認できます！ :star:".to_string(),
            message_runs: vec![
                MessageRun {
                    kind: MessageRunKind::Text,
                    text: "フォント・縁取り・背景透過の見え方を実寸で確認できます！ ".to_string(),
                    emoji_url: None,
                    alt: None,
                    is_custom_emoji: false,
                },
                MessageRun {
                    kind: MessageRunKind::Emoji,
                    text: ":star:".to_string(),
                    emoji_url: Some(stamp_ok),
                    alt: Some(":star:".to_string()),
                    is_custom_emoji: true,
                },
            ],
            timestamp_ms: now_ms - 5_000,
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

        let sample_counters = vec![
            WordCounterConfig {
                id: "preview-cnt-1".to_string(),
                name: "草カウンター".to_string(),
                keywords: vec!["草".to_string(), "www".to_string()],
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                max_per_account: None,
                exclude_banned: true,
                accent_color: "#22C55E".to_string(),
                enabled: true,
                total_count: 128,
                unique_user_count: 42,
            },
            WordCounterConfig {
                id: "preview-cnt-2".to_string(),
                name: "ナイス！".to_string(),
                keywords: vec!["ナイス".to_string(), "GG".to_string()],
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                max_per_account: Some(1),
                exclude_banned: true,
                accent_color: "#38BDF8".to_string(),
                enabled: true,
                total_count: 42,
                unique_user_count: 42,
            },
        ];

        let sample_lottery = LotteryResult {
            id: "preview-init-lot".to_string(),
            kind: "プレビュー抽選".to_string(),
            winners: vec![LotteryWinner {
                user_id: "preview-w1".to_string(),
                user_name: "ラッキー視聴者".to_string(),
                user_icon_url: demo::make_svg_avatar("%23eab308", "当"),
                comment_text: Some("参加希望します！当たりますように！".to_string()),
                won_at_ms: now_ms,
            }],
            candidate_count: 24,
            drawn_at_ms: now_ms,
        };

        let ft_cfg = FirstTimeCounterConfig {
            show_first_timer_profile: true,
            ..Default::default()
        };
        let sample_first_time = FirstTimeCounterState {
            count: 12,
            total_detected_all_time: 128,
            recent_users: vec![
                FirstTimeUser {
                    user_id: "preview-ft1".to_string(),
                    user_name: "初見のねこ".to_string(),
                    user_icon_url: demo::make_svg_avatar("%23f59e0b", "ね"),
                    trigger_reason: "初見ワード".to_string(),
                    detected_at_ms: now_ms - 8_000,
                },
                FirstTimeUser {
                    user_id: "preview-ft2".to_string(),
                    user_name: "みずいろ".to_string(),
                    user_icon_url: demo::make_svg_avatar("%23818cf8", "み"),
                    trigger_reason: "未登録".to_string(),
                    detected_at_ms: now_ms - 40_000,
                },
            ],
            config: ft_cfg,
        };

        let sample_sc = vec![
            MonetizationItem {
                id: "preview-init-sc1".to_string(),
                session_id: "preview".to_string(),
                platform: PlatformKind::Demo,
                kind: MonetizationEventKind::SuperChat,
                user_id: "preview-sc1".to_string(),
                user_name: "さくらもち".to_string(),
                user_icon_url: demo::make_svg_avatar("%23ec4899", "さ"),
                amount_display: "¥1,000".to_string(),
                amount_micros: 1_000_000_000,
                currency: "JPY".to_string(),
                tier_color: "#FFCA28".to_string(),
                message: "プレビュー用の応援スーパーチャットです！".to_string(),
                message_runs: Vec::new(),
                timestamp_ms: now_ms - 10_000,
                is_checked: false,
                checked_at_ms: None,
            },
            MonetizationItem {
                id: "preview-init-sc2".to_string(),
                session_id: "preview".to_string(),
                platform: PlatformKind::Demo,
                kind: MonetizationEventKind::MembershipNew,
                user_id: "preview-sc2".to_string(),
                user_name: "夜ふかし勢".to_string(),
                user_icon_url: demo::make_svg_avatar("%2322c55e", "夜"),
                amount_display: "メンバー加入".to_string(),
                amount_micros: 0,
                currency: "JPY".to_string(),
                tier_color: "#2BA640".to_string(),
                message: "新しくメンバーシップに参加しました！".to_string(),
                message_runs: Vec::new(),
                timestamp_ms: now_ms - 35_000,
                is_checked: false,
                checked_at_ms: None,
            },
        ];

        let mut sample_poll = PollState {
            is_active: true,
            started_at_ms: Some(now_ms - 60_000),
            ..Default::default()
        };
        if sample_poll.config.options.len() >= 3 {
            sample_poll.config.options[0].vote_count = 42;
            sample_poll.config.options[1].vote_count = 29;
            sample_poll.config.options[2].vote_count = 15;
            sample_poll.total_votes = 86;
        }

        let sample_pickup = PickedCommentState {
            comment: Some(CommentItem {
                id: "preview-init-pick".to_string(),
                session_id: "preview".to_string(),
                platform: PlatformKind::Demo,
                user_id: "preview-u1".to_string(),
                user_name: "たこやき太郎".to_string(),
                user_icon_url: demo::make_svg_avatar("%230ea5e9", "た"),
                user_memo: String::new(),
                is_first_time: false,
                message: "質問です！今日の配信で一番おすすめしたいポイントはどこですか？".to_string(),
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
            }),
            picked_at_ms: Some(now_ms),
        };

        let sample_queue = QueueState {
            config: QueueConfig {
                is_accepting: true,
                max_active_slots: 3,
                ..QueueConfig::default()
            },
            entries: vec![
                QueueEntry {
                    user_id: "preview-q1".to_string(),
                    platform: PlatformKind::Demo,
                    user_name: "たこやき太郎".to_string(),
                    user_icon_url: demo::make_svg_avatar("%230ea5e9", "た"),
                    status: QueueEntryStatus::Active,
                    play_count: 1,
                    is_first_time: true,
                    note: String::new(),
                    joined_at_ms: now_ms - 180_000,
                },
                QueueEntry {
                    user_id: "preview-q2".to_string(),
                    platform: PlatformKind::Demo,
                    user_name: "さくらもち".to_string(),
                    user_icon_url: demo::make_svg_avatar("%23ec4899", "さ"),
                    status: QueueEntryStatus::Active,
                    play_count: 2,
                    is_first_time: false,
                    note: String::new(),
                    joined_at_ms: now_ms - 150_000,
                },
                QueueEntry {
                    user_id: "preview-q3".to_string(),
                    platform: PlatformKind::Demo,
                    user_name: "ゲーマーK".to_string(),
                    user_icon_url: demo::make_svg_avatar("%2322c55e", "K"),
                    status: QueueEntryStatus::Waiting,
                    play_count: 0,
                    is_first_time: true,
                    note: String::new(),
                    joined_at_ms: now_ms - 90_000,
                },
                QueueEntry {
                    user_id: "preview-q4".to_string(),
                    platform: PlatformKind::Demo,
                    user_name: "みずいろ".to_string(),
                    user_icon_url: demo::make_svg_avatar("%23818cf8", "み"),
                    status: QueueEntryStatus::Waiting,
                    play_count: 1,
                    is_first_time: false,
                    note: String::new(),
                    joined_at_ms: now_ms - 45_000,
                },
            ],
        };

        Self {
            comments: vec![sample_c1, sample_c2],
            counters: sample_counters,
            last_lottery: Some(sample_lottery),
            first_time: sample_first_time,
            monetization_items: sample_sc,
            poll_state: sample_poll,
            picked_comment: sample_pickup,
            queue_state: sample_queue,
            media_config: MediaReactionConfig::default(),
            sample_seq: 0,
        }
    }
}

/// チャンネル別 WebSocket 購読者数の RAII ガード（切断時に自動デクリメント）
struct ChannelSubGuard {
    counters: Arc<[AtomicUsize; OverlayChannel::COUNT]>,
    idx: usize,
}

impl ChannelSubGuard {
    fn acquire(counters: Arc<[AtomicUsize; OverlayChannel::COUNT]>, ch: OverlayChannel) -> Self {
        let idx = ch.index();
        counters[idx].fetch_add(1, Ordering::AcqRel);
        Self { counters, idx }
    }
}

impl Drop for ChannelSubGuard {
    fn drop(&mut self) {
        self.counters[self.idx].fetch_sub(1, Ordering::AcqRel);
    }
}

fn new_channel_counters() -> Arc<[AtomicUsize; OverlayChannel::COUNT]> {
    Arc::new(std::array::from_fn(|_| AtomicUsize::new(0)))
}

fn has_subscriber_for_channel(
    counters: &[AtomicUsize; OverlayChannel::COUNT],
    total_receivers: usize,
    channel: OverlayChannel,
) -> bool {
    if total_receivers == 0 {
        return false;
    }
    let tracked_total: usize = counters.iter().map(|c| c.load(Ordering::Acquire)).sum();
    // テスト等からの直接 tx.subscribe()（未計測レシーバ）がある場合は後方互換で配信する
    if tracked_total < total_receivers {
        return true;
    }
    if channel == OverlayChannel::All {
        return tracked_total > 0;
    }
    counters[OverlayChannel::All.index()].load(Ordering::Acquire) > 0
        || counters[channel.index()].load(Ordering::Acquire) > 0
}

const PIN_MAX_FAILURES: u32 = 5;
const PIN_LOCKOUT_DURATION: Duration = Duration::from_secs(30);
const PIN_RATE_LIMITER_MAX_ENTRIES: usize = 1024;

#[derive(Debug, Clone)]
struct PinAttemptState {
    failed_count: u32,
    locked_until: Option<Instant>,
    last_attempt: Instant,
}

/// オーバーレイ配信時に配信者専用プライベートメモ（`user_memo`）を除去する
fn sanitize_comment_for_overlay(mut comment: CommentItem) -> CommentItem {
    comment.user_memo.clear();
    comment
}

fn sanitize_picked_for_overlay(mut picked: PickedCommentState) -> PickedCommentState {
    if let Some(ref mut c) = picked.comment {
        c.user_memo.clear();
    }
    picked
}

pub struct OverlaySharedState {
    pub media_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub installed_mods: RwLock<Vec<InstalledMod>>,
    pub mods_revision: AtomicU64,
    pub remote_config: RwLock<RemoteAccessConfig>,
    allow_lan_access: AtomicBool,
    pub remote_auth_gen: watch::Sender<u64>,
    pin_rate_limiter: std::sync::Mutex<HashMap<IpAddr, PinAttemptState>>,
    pub config: RwLock<OverlayConfig>,
    pub recent_comments: RwLock<VecDeque<CommentItem>>,
    pub counters: RwLock<Vec<WordCounterConfig>>,
    pub last_lottery: RwLock<Option<LotteryResult>>,
    pub first_time: RwLock<FirstTimeCounterState>,
    pub first_time_enabled: RwLock<bool>,
    pub monetization_items: RwLock<Vec<MonetizationItem>>,
    pub superchat_enabled: RwLock<bool>,
    pub poll_state: RwLock<PollState>,
    pub poll_enabled: RwLock<bool>,
    pub picked_comment: RwLock<PickedCommentState>,
    pub pickup_enabled: RwLock<bool>,
    pub queue_state: RwLock<QueueState>,
    pub queue_enabled: RwLock<bool>,
    pub media_config: RwLock<MediaReactionConfig>,
    pub media_enabled: RwLock<bool>,
    pub obs_overlay_enabled: AtomicBool,
    obs_enabled_tx: watch::Sender<bool>,
    pub tx: broadcast::Sender<Arc<(OverlayChannel, String)>>,
    channel_subs: Arc<[AtomicUsize; OverlayChannel::COUNT]>,
    /// プレビュー専用（/overlay/preview/*）の独立状態と独立ブロードキャストチャンネル
    pub preview_store: RwLock<PreviewOverlayStore>,
    pub preview_tx: broadcast::Sender<Arc<(OverlayChannel, String)>>,
    preview_channel_subs: Arc<[AtomicUsize; OverlayChannel::COUNT]>,
}

impl OverlaySharedState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        media_dir: PathBuf,
        mods_dir: PathBuf,
        installed_mods: Vec<InstalledMod>,
        remote_config: RemoteAccessConfig,
        config: OverlayConfig,
        counters: Vec<WordCounterConfig>,
        first_time: FirstTimeCounterState,
        first_time_enabled: bool,
        monetization_items: Vec<MonetizationItem>,
        superchat_enabled: bool,
        poll_state: PollState,
        poll_enabled: bool,
        queue_state: QueueState,
        queue_enabled: bool,
        media_config: MediaReactionConfig,
        media_enabled: bool,
        pickup_enabled: bool,
    ) -> Self {
        let (tx, _) = broadcast::channel(512);
        let (preview_tx, _) = broadcast::channel(256);
        let (obs_enabled_tx, _) = watch::channel(true);
        let (remote_auth_gen, _) = watch::channel(0u64);
        let initial_allow_lan = remote_config.allow_lan_access;
        Self {
            media_dir,
            mods_dir,
            installed_mods: RwLock::new(installed_mods),
            mods_revision: AtomicU64::new(1),
            remote_config: RwLock::new(remote_config),
            allow_lan_access: AtomicBool::new(initial_allow_lan),
            remote_auth_gen,
            pin_rate_limiter: std::sync::Mutex::new(HashMap::new()),
            config: RwLock::new(config),
            recent_comments: RwLock::new(VecDeque::new()),
            counters: RwLock::new(counters),
            last_lottery: RwLock::new(None),
            first_time: RwLock::new(first_time),
            first_time_enabled: RwLock::new(first_time_enabled),
            monetization_items: RwLock::new(monetization_items),
            superchat_enabled: RwLock::new(superchat_enabled),
            poll_state: RwLock::new(poll_state),
            poll_enabled: RwLock::new(poll_enabled),
            picked_comment: RwLock::new(PickedCommentState::default()),
            pickup_enabled: RwLock::new(pickup_enabled),
            queue_state: RwLock::new(queue_state),
            queue_enabled: RwLock::new(queue_enabled),
            media_config: RwLock::new(media_config),
            media_enabled: RwLock::new(media_enabled),
            obs_overlay_enabled: AtomicBool::new(true),
            obs_enabled_tx,
            tx,
            channel_subs: new_channel_counters(),
            preview_store: RwLock::new(PreviewOverlayStore::with_default_samples()),
            preview_tx,
            preview_channel_subs: new_channel_counters(),
        }
    }

    /// LAN アクセス許可設定を更新（ip_access_guard 用 AtomicBool 同期＆認証変更時の既存 `/ws/remote` 即時切断）
    pub async fn update_remote_config(&self, cfg: RemoteAccessConfig) {
        self.allow_lan_access
            .store(cfg.allow_lan_access, Ordering::Relaxed);
        let mut guard = self.remote_config.write().await;
        let auth_changed = guard.pin_code != cfg.pin_code
            || guard.require_pin != cfg.require_pin
            || guard.remote_control_enabled != cfg.remote_control_enabled
            || guard.allow_lan_access != cfg.allow_lan_access;
        *guard = cfg;
        drop(guard);
        if auth_changed {
            let next_gen = self.remote_auth_gen.borrow().wrapping_add(1);
            let _ = self.remote_auth_gen.send(next_gen);
        }
    }

    /// 指定 IP が PIN 失敗回数超過によるロック中であれば残り秒数を返す
    pub fn check_pin_rate_limit(&self, ip: IpAddr) -> Option<u64> {
        let mut map = self.pin_rate_limiter.lock().ok()?;
        let now = Instant::now();
        if let Some(entry) = map.get_mut(&ip) {
            if let Some(until) = entry.locked_until {
                if now < until {
                    return Some(until.duration_since(now).as_secs().max(1));
                }
                // ロック期限経過後は再試行できるようリセット
                entry.locked_until = None;
                entry.failed_count = 0;
            }
        }
        None
    }

    /// PIN 認証失敗を記録し、ロック状態に入った場合はロック秒数を返す
    pub fn record_pin_failure(&self, ip: IpAddr) -> Option<u64> {
        let mut map = self.pin_rate_limiter.lock().ok()?;
        let now = Instant::now();
        if map.len() >= PIN_RATE_LIMITER_MAX_ENTRIES && !map.contains_key(&ip) {
            // 古いエントリを掃除してメモリ枯渇を防止
            map.retain(|_, v| {
                now.duration_since(v.last_attempt) < Duration::from_secs(300)
                    || v.locked_until.map(|u| now < u).unwrap_or(false)
            });
            if map.len() >= PIN_RATE_LIMITER_MAX_ENTRIES {
                map.clear();
            }
        }
        let entry = map.entry(ip).or_insert(PinAttemptState {
            failed_count: 0,
            locked_until: None,
            last_attempt: now,
        });
        entry.last_attempt = now;
        entry.failed_count = entry.failed_count.saturating_add(1);
        if entry.failed_count >= PIN_MAX_FAILURES {
            entry.locked_until = Some(now + PIN_LOCKOUT_DURATION);
            return Some(PIN_LOCKOUT_DURATION.as_secs());
        }
        None
    }

    /// PIN 認証成功時に該当 IP の失敗カウントをクリアする
    pub fn clear_pin_failures(&self, ip: IpAddr) {
        if let Ok(mut map) = self.pin_rate_limiter.lock() {
            map.remove(&ip);
        }
    }

    #[inline]
    pub fn is_allow_lan_access(&self) -> bool {
        self.allow_lan_access.load(Ordering::Relaxed)
    }

    /// OBS オーバーレイ配信の有効 / 無効を切り替える（OFF 時は接続中の全 OBS WebSocket を即時切断）
    pub fn set_obs_overlay_enabled(&self, enabled: bool) {
        self.obs_overlay_enabled.store(enabled, Ordering::Relaxed);
        let _ = self.obs_enabled_tx.send(enabled);
    }

    pub fn is_obs_overlay_enabled(&self) -> bool {
        self.obs_overlay_enabled.load(Ordering::Relaxed)
    }

    #[inline]
    fn should_broadcast(&self, channel: OverlayChannel) -> bool {
        self.is_obs_overlay_enabled()
            && has_subscriber_for_channel(&self.channel_subs, self.tx.receiver_count(), channel)
    }

    #[inline]
    fn should_broadcast_preview(&self, channel: OverlayChannel) -> bool {
        self.is_obs_overlay_enabled()
            && has_subscriber_for_channel(
                &self.preview_channel_subs,
                self.preview_tx.receiver_count(),
                channel,
            )
    }

    pub fn broadcast(&self, msg: &OverlayMessage) {
        // OBS オーバーレイ無効または該当チャンネルの購読者 0 件のときは JSON シリアライズ自体をスキップ
        let channel = msg.channel();
        if !self.should_broadcast(channel) {
            return;
        }
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.tx.send(Arc::new((channel, json)));
        }
    }

    /// プレビュー専用チャンネル（/overlay/preview/*）のみにブロードキャスト（本番には一切送信しない）
    pub fn broadcast_preview(&self, msg: &OverlayMessage) {
        let channel = msg.channel();
        if !self.should_broadcast_preview(channel) {
            return;
        }
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.preview_tx.send(Arc::new((channel, json)));
        }
    }

    /// プレビュー専用（/overlay/preview/*）の初期 Snapshot を構築する（本番状態は参照しない）
    pub async fn build_preview_snapshot(&self, channel: Option<OverlayChannel>) -> OverlayMessage {
        let include = |ch: OverlayChannel| ch.matches(channel);
        let store = self.preview_store.read().await;
        OverlayMessage::Snapshot(Box::new(OverlaySnapshotPayload {
            config: self.config.read().await.clone(),
            comments: if include(OverlayChannel::Comments) {
                store.comments.clone()
            } else {
                Vec::new()
            },
            counters: if include(OverlayChannel::Counter) {
                store.counters.clone()
            } else {
                Vec::new()
            },
            last_lottery: if include(OverlayChannel::Lottery) {
                store.last_lottery.clone()
            } else {
                None
            },
            first_time: if include(OverlayChannel::FirstTime) {
                store.first_time.clone()
            } else {
                FirstTimeCounterState::default()
            },
            first_time_enabled: include(OverlayChannel::FirstTime),
            monetization_items: if include(OverlayChannel::SuperChat) {
                store.monetization_items.clone()
            } else {
                Vec::new()
            },
            superchat_enabled: include(OverlayChannel::SuperChat),
            poll_state: if include(OverlayChannel::Poll) {
                store.poll_state.clone()
            } else {
                PollState::default()
            },
            poll_enabled: include(OverlayChannel::Poll),
            picked_comment: if include(OverlayChannel::Pickup) {
                store.picked_comment.clone()
            } else {
                PickedCommentState::default()
            },
            pickup_enabled: include(OverlayChannel::Pickup),
            queue_state: if include(OverlayChannel::Queue) {
                store.queue_state.clone()
            } else {
                QueueState::default()
            },
            queue_enabled: include(OverlayChannel::Queue),
            media_config: if include(OverlayChannel::Media) {
                store.media_config.clone()
            } else {
                MediaReactionConfig::default()
            },
            media_enabled: include(OverlayChannel::Media),
            mods: self.enabled_mods_snapshot().await,
            mods_revision: self.mods_revision.load(Ordering::Acquire),
        }))
    }

    /// 接続先オーバーレイのチャンネルに必要な項目だけを詰め込んだ軽量 Snapshot を構築する
    pub async fn build_snapshot(&self, channel: Option<OverlayChannel>) -> OverlayMessage {
        let include = |ch: OverlayChannel| ch.matches(channel);
        OverlayMessage::Snapshot(Box::new(OverlaySnapshotPayload {
            config: self.config.read().await.clone(),
            comments: if include(OverlayChannel::Comments) {
                self.recent_comments
                    .read()
                    .await
                    .iter()
                    .cloned()
                    .map(sanitize_comment_for_overlay)
                    .collect()
            } else {
                Vec::new()
            },
            counters: if include(OverlayChannel::Counter) {
                self.counters.read().await.clone()
            } else {
                Vec::new()
            },
            last_lottery: if include(OverlayChannel::Lottery) {
                self.last_lottery.read().await.clone()
            } else {
                None
            },
            first_time: if include(OverlayChannel::FirstTime) {
                self.first_time.read().await.clone()
            } else {
                FirstTimeCounterState::default()
            },
            first_time_enabled: if include(OverlayChannel::FirstTime) {
                *self.first_time_enabled.read().await
            } else {
                false
            },
            monetization_items: if include(OverlayChannel::SuperChat) {
                self.monetization_items.read().await.clone()
            } else {
                Vec::new()
            },
            superchat_enabled: if include(OverlayChannel::SuperChat) {
                *self.superchat_enabled.read().await
            } else {
                false
            },
            poll_state: if include(OverlayChannel::Poll) {
                self.poll_state.read().await.clone()
            } else {
                PollState::default()
            },
            poll_enabled: if include(OverlayChannel::Poll) {
                *self.poll_enabled.read().await
            } else {
                false
            },
            picked_comment: if include(OverlayChannel::Pickup) {
                sanitize_picked_for_overlay(self.picked_comment.read().await.clone())
            } else {
                PickedCommentState::default()
            },
            pickup_enabled: if include(OverlayChannel::Pickup) {
                *self.pickup_enabled.read().await
            } else {
                false
            },
            queue_state: if include(OverlayChannel::Queue) {
                self.queue_state.read().await.clone()
            } else {
                QueueState::default()
            },
            queue_enabled: if include(OverlayChannel::Queue) {
                *self.queue_enabled.read().await
            } else {
                false
            },
            media_config: if include(OverlayChannel::Media) {
                self.media_config.read().await.clone()
            } else {
                MediaReactionConfig::default()
            },
            media_enabled: if include(OverlayChannel::Media) {
                *self.media_enabled.read().await
            } else {
                false
            },
            mods: self.enabled_mods_snapshot().await,
            mods_revision: self.mods_revision.load(Ordering::Acquire),
        }))
    }

    /// 有効かつエラーのない導入済み MOD の一覧スナップショットを返す
    pub async fn enabled_mods_snapshot(&self) -> Vec<InstalledMod> {
        self.installed_mods
            .read()
            .await
            .iter()
            .filter(|m| m.enabled && m.load_error.is_none())
            .cloned()
            .collect()
    }

    /// MOD 一覧を更新し、本番・プレビュー双方の接続中オーバーレイへ即時配信する
    pub async fn update_mods(&self, mods: Vec<InstalledMod>) {
        *self.installed_mods.write().await = mods;
        let rev = self.mods_revision.fetch_add(1, Ordering::AcqRel) + 1;
        let send_prod = self.should_broadcast(OverlayChannel::All);
        let send_prev = self.should_broadcast_preview(OverlayChannel::All);
        if send_prod || send_prev {
            let enabled_mods = self.enabled_mods_snapshot().await;
            let msg = OverlayMessage::ModsUpdated {
                mods: enabled_mods,
                revision: rev,
            };
            if send_prod {
                self.broadcast(&msg);
            }
            if send_prev {
                self.broadcast_preview(&msg);
            }
        }
    }

    pub async fn push_comment(&self, comment: CommentItem) {
        let comment = sanitize_comment_for_overlay(comment);
        let max_items = self.config.read().await.comment_max_items.max(10) as usize;
        let need_send = self.should_broadcast(OverlayChannel::Comments);
        if need_send {
            {
                let mut list = self.recent_comments.write().await;
                list.push_back(comment.clone());
                while list.len() > max_items {
                    list.pop_front();
                }
            }
            self.broadcast(&OverlayMessage::Comment { comment });
        } else {
            let mut list = self.recent_comments.write().await;
            list.push_back(comment);
            while list.len() > max_items {
                list.pop_front();
            }
        }
    }

    pub async fn update_counters(&self, counters: Vec<WordCounterConfig>) {
        if self.should_broadcast(OverlayChannel::Counter) {
            *self.counters.write().await = counters.clone();
            self.broadcast(&OverlayMessage::Counters { counters });
        } else {
            *self.counters.write().await = counters;
        }
    }

    pub async fn update_first_time(&self, state: FirstTimeCounterState, enabled: bool) {
        *self.first_time_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::FirstTime) {
            *self.first_time.write().await = state.clone();
            self.broadcast(&OverlayMessage::FirstTime {
                first_time: state,
                enabled,
            });
        } else {
            *self.first_time.write().await = state;
        }
    }

    pub async fn show_lottery(&self, result: LotteryResult) {
        if self.should_broadcast(OverlayChannel::Lottery) {
            *self.last_lottery.write().await = Some(result.clone());
            self.broadcast(&OverlayMessage::LotteryResult { result });
        } else {
            *self.last_lottery.write().await = Some(result);
        }
    }

    pub async fn clear_lottery(&self) {
        *self.last_lottery.write().await = None;
        self.broadcast(&OverlayMessage::ClearLottery);
    }

    pub async fn update_monetization(&self, items: Vec<MonetizationItem>, enabled: bool) {
        *self.superchat_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::SuperChat) {
            *self.monetization_items.write().await = items.clone();
            self.broadcast(&OverlayMessage::SuperChat { items, enabled });
        } else {
            *self.monetization_items.write().await = items;
        }
    }

    pub async fn update_poll(&self, poll: PollState, enabled: bool) {
        *self.poll_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::Poll) {
            *self.poll_state.write().await = poll.clone();
            self.broadcast(&OverlayMessage::Poll { poll, enabled });
        } else {
            *self.poll_state.write().await = poll;
        }
    }

    pub async fn clear_poll(&self) {
        self.broadcast(&OverlayMessage::ClearPoll);
    }

    pub async fn update_picked_comment(&self, picked: PickedCommentState, enabled: bool) {
        let picked = sanitize_picked_for_overlay(picked);
        *self.pickup_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::Pickup) {
            *self.picked_comment.write().await = picked.clone();
            self.broadcast(&OverlayMessage::PickedComment { picked, enabled });
        } else {
            *self.picked_comment.write().await = picked;
        }
    }

    pub async fn update_queue(&self, queue: QueueState, enabled: bool) {
        *self.queue_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::Queue) {
            *self.queue_state.write().await = queue.clone();
            self.broadcast(&OverlayMessage::Queue { queue, enabled });
        } else {
            *self.queue_state.write().await = queue;
        }
    }

    pub async fn update_media_config(&self, media_config: MediaReactionConfig, enabled: bool) {
        *self.media_enabled.write().await = enabled;
        if self.should_broadcast(OverlayChannel::Media) {
            *self.media_config.write().await = media_config.clone();
            self.broadcast(&OverlayMessage::MediaConfig {
                media_config,
                enabled,
            });
        } else {
            *self.media_config.write().await = media_config;
        }
    }

    pub fn trigger_media(&self, trigger: MediaTriggerEvent) {
        if self.should_broadcast(OverlayChannel::Media) {
            self.broadcast(&OverlayMessage::MediaTrigger { trigger });
        }
    }

    pub fn clear_media(&self) {
        self.broadcast(&OverlayMessage::ClearMedia);
    }

    pub async fn update_config(&self, config: OverlayConfig) {
        let send_prod = self.should_broadcast(OverlayChannel::All);
        let send_prev = self.should_broadcast_preview(OverlayChannel::All);
        if send_prod || send_prev {
            *self.config.write().await = config.clone();
            let msg = OverlayMessage::ConfigUpdated {
                config: Box::new(config),
            };
            if send_prod {
                self.broadcast(&msg);
            }
            // デザイン・CSS・スタイルの変更はプレビュー環境にもリアルタイム反映する
            if send_prev {
                self.broadcast_preview(&msg);
            }
        } else {
            *self.config.write().await = config;
        }
    }

    // ==================== プレビュー専用（本番非干渉）更新メソッド ====================

    pub async fn push_preview_comment(&self, comment: CommentItem) {
        let max_items = self.config.read().await.comment_max_items.max(10) as usize;
        {
            let mut store = self.preview_store.write().await;
            store.comments.push(comment.clone());
            if store.comments.len() > max_items {
                let drop_count = store.comments.len() - max_items;
                store.comments.drain(0..drop_count);
            }
        }
        self.broadcast_preview(&OverlayMessage::Comment { comment });
    }

    pub async fn update_preview_counters(&self, counters: Vec<WordCounterConfig>) {
        self.preview_store.write().await.counters = counters.clone();
        self.broadcast_preview(&OverlayMessage::Counters { counters });
    }

    pub async fn update_preview_first_time(&self, state: FirstTimeCounterState) {
        self.preview_store.write().await.first_time = state.clone();
        self.broadcast_preview(&OverlayMessage::FirstTime {
            first_time: state,
            enabled: true,
        });
    }

    pub async fn show_preview_lottery(&self, result: LotteryResult) {
        self.preview_store.write().await.last_lottery = Some(result.clone());
        self.broadcast_preview(&OverlayMessage::LotteryResult { result });
    }

    pub async fn update_preview_monetization(&self, items: Vec<MonetizationItem>) {
        self.preview_store.write().await.monetization_items = items.clone();
        self.broadcast_preview(&OverlayMessage::SuperChat {
            items,
            enabled: true,
        });
    }

    pub async fn update_preview_poll(&self, poll: PollState) {
        self.preview_store.write().await.poll_state = poll.clone();
        self.broadcast_preview(&OverlayMessage::Poll {
            poll,
            enabled: true,
        });
    }

    pub async fn update_preview_picked_comment(&self, picked: PickedCommentState) {
        self.preview_store.write().await.picked_comment = picked.clone();
        self.broadcast_preview(&OverlayMessage::PickedComment {
            picked,
            enabled: true,
        });
    }

    pub async fn update_preview_queue(&self, queue: QueueState) {
        self.preview_store.write().await.queue_state = queue.clone();
        self.broadcast_preview(&OverlayMessage::Queue {
            queue,
            enabled: true,
        });
    }

    pub fn trigger_preview_media(&self, trigger: MediaTriggerEvent) {
        self.broadcast_preview(&OverlayMessage::MediaTrigger { trigger });
    }
}

pub struct OverlayServerHandle {
    pub port: u16,
    pub allow_lan_access: bool,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OverlayServerHandle {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// ローカル HTTP + WebSocket サーバーを起動する（allow_lan = true 時は 0.0.0.0 で LAN 公開）
pub fn start_overlay_server(
    port: u16,
    allow_lan: bool,
    shared: Arc<OverlaySharedState>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OverlayServerHandle>> + Send>> {
    Box::pin(async move {
        let bind_ip = if allow_lan {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        };
        let addr = SocketAddr::from((bind_ip, port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let actual_port = listener.local_addr()?.port();

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(favicon_handler))
            .route("/remote", get(remote_api::remote_page_handler))
            .route("/api/remote/auth_info", get(remote_api::auth_info_handler))
            .route("/api/remote/verify", post(remote_api::verify_pin_handler))
            .route("/api/remote/state", get(remote_api::remote_state_handler))
            .route("/api/remote/action", post(remote_api::remote_action_handler))
            .route(
                "/api/trigger/{action}",
                get(remote_api::stream_deck_trigger_handler)
                    .post(remote_api::stream_deck_trigger_handler),
            )
            .route(
                "/api/remote/upload_media",
                post(remote_api::upload_media_handler)
                    .layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
            )
            .route("/ws/remote", get(remote_api::remote_ws_handler))
            .route(
                "/overlay/stosup-overlay.js",
                get(overlay_common_js_handler),
            )
            .route(
                "/overlay/stosup-mod-sdk.js",
                get(overlay_mod_sdk_js_handler),
            )
            .route("/api/mods/runtime", get(mods_runtime_api_handler))
            .route("/mods/{mod_id}/{*asset_path}", get(mod_asset_handler))
            .route(
                "/overlay/mod/{mod_id}/{overlay_id}",
                get(mod_custom_overlay_handler),
            )
            .route(
                "/overlay/preview/mod/{mod_id}/{overlay_id}",
                get(mod_custom_overlay_handler),
            )
            .route(
                "/overlay/comments",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COMMENTS_HTML)
                }),
            )
            .route(
                "/overlay/lottery",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, LOTTERY_HTML)
                }),
            )
            .route(
                "/overlay/counter",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COUNTER_HTML)
                }),
            )
            .route(
                "/overlay/first_time",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, FIRST_TIME_HTML)
                }),
            )
            .route(
                "/overlay/superchat",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, SUPERCHAT_HTML)
                }),
            )
            .route(
                "/overlay/poll",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, POLL_HTML)
                }),
            )
            .route(
                "/overlay/pickup",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, PICKUP_HTML)
                }),
            )
            .route(
                "/overlay/queue",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, QUEUE_HTML)
                }),
            )
            .route(
                "/overlay/media",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, MEDIA_HTML)
                }),
            )
            // 本番データに一切影響しないプレビュー・テスト専用環境 (/overlay/preview/*)
            .route(
                "/overlay/preview/comments",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COMMENTS_HTML)
                }),
            )
            .route(
                "/overlay/preview/lottery",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, LOTTERY_HTML)
                }),
            )
            .route(
                "/overlay/preview/counter",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COUNTER_HTML)
                }),
            )
            .route(
                "/overlay/preview/first_time",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, FIRST_TIME_HTML)
                }),
            )
            .route(
                "/overlay/preview/superchat",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, SUPERCHAT_HTML)
                }),
            )
            .route(
                "/overlay/preview/poll",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, POLL_HTML)
                }),
            )
            .route(
                "/overlay/preview/pickup",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, PICKUP_HTML)
                }),
            )
            .route(
                "/overlay/preview/queue",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, QUEUE_HTML)
                }),
            )
            .route(
                "/overlay/preview/media",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, MEDIA_HTML)
                }),
            )
            .route("/media/{file_name}", get(media_file_handler))
            .route("/ws", get(ws_handler))
            .layer(middleware::from_fn_with_state(
                Arc::clone(&shared),
                remote_api::ip_access_guard,
            ))
            .with_state(shared);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        });

        Ok(OverlayServerHandle {
            port: actual_port,
            allow_lan_access: allow_lan,
            shutdown_tx: Some(shutdown_tx),
        })
    })
}

/// 既存サーバー停止直後のソケット解放待ちを考慮し、短時間リトライ付きでサーバーを起動する
pub async fn start_overlay_server_with_retry(
    port: u16,
    allow_lan: bool,
    shared: Arc<OverlaySharedState>,
) -> Result<OverlayServerHandle> {
    if port == 0 {
        return start_overlay_server(0, allow_lan, shared).await;
    }
    const BACKOFF_MS: [u64; 5] = [30, 50, 80, 120, 160];
    let mut last_err = match start_overlay_server(port, allow_lan, Arc::clone(&shared)).await {
        Ok(handle) => return Ok(handle),
        Err(e) => e,
    };
    for wait_ms in BACKOFF_MS {
        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        match start_overlay_server(port, allow_lan, Arc::clone(&shared)).await {
            Ok(handle) => return Ok(handle),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

const NO_STORE_CACHE_CONTROL: &str = "no-store, no-cache, must-revalidate";

/// バージョン更新時に OBS Browser Source (CEF) が旧 JS をキャッシュ再利用するのを防ぐため
/// 共通スクリプト URL にバージョン付きクエリを付与する
fn with_versioned_overlay_scripts(html: &str) -> String {
    let ver = crate::updater::CURRENT_APP_VERSION;
    html.replace(
        "src=\"/overlay/stosup-overlay.js\"",
        &format!("src=\"/overlay/stosup-overlay.js?v={}\"", ver),
    )
    .replace(
        "src=\"/overlay/stosup-mod-sdk.js\"",
        &format!("src=\"/overlay/stosup-mod-sdk.js?v={}\"", ver),
    )
}

fn serve_overlay_html(
    State(state): State<Arc<OverlaySharedState>>,
    html: &'static str,
) -> impl IntoResponse {
    if !state.is_obs_overlay_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CACHE_CONTROL, NO_STORE_CACHE_CONTROL)],
            Html(
                "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><title>オーバーレイスタジオ停止中</title></head>\
                 <body style='margin:0;background:transparent;color:#a1a1aa;font-family:sans-serif;padding:12px;font-size:13px'>\
                 Stosup のオーバーレイスタジオ機能は現在オフになっています。</body></html>"
                    .to_string(),
            ),
        );
    }
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, NO_STORE_CACHE_CONTROL)],
        Html(with_versioned_overlay_scripts(html)),
    )
}

async fn favicon_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        APP_ICON_ICO,
    )
}

async fn overlay_common_js_handler() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, NO_STORE_CACHE_CONTROL),
        ],
        OVERLAY_COMMON_JS,
    )
}

async fn overlay_mod_sdk_js_handler() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, NO_STORE_CACHE_CONTROL),
        ],
        OVERLAY_MOD_SDK_JS,
    )
}

async fn mods_runtime_api_handler(
    State(state): State<Arc<OverlaySharedState>>,
) -> impl IntoResponse {
    let mods = state.enabled_mods_snapshot().await;
    let rev = state.mods_revision.load(Ordering::Acquire);
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "revision": rev,
            "mods": mods,
        })),
    )
}

/// HTML 内で `<head>` 開始タグの閉じ `>` 位置（直後）を探す（`<header>` 等の別タグ誤一致を防止）
fn find_head_tag_insert_pos(lower_html: &str) -> Option<usize> {
    let bytes = lower_html.as_bytes();
    let mut offset = 0;
    while let Some(rel) = lower_html[offset..].find("<head") {
        let start = offset + rel;
        let after = start + 5;
        let next_byte = bytes.get(after).copied();
        if matches!(next_byte, Some(b'>' | b' ' | b'\t' | b'\r' | b'\n' | b'/')) {
            if let Some(gt_rel) = lower_html[start..].find('>') {
                return Some(start + gt_rel + 1);
            }
            return None;
        }
        offset = after;
    }
    None
}

/// 独立カスタムオーバーレイ HTML の `<head>` へ `<base>` および Stosup SDK を自動注入する
fn inject_mod_overlay_head(raw_html: &str, mod_id: &str) -> String {
    let lower = raw_html.to_ascii_lowercase();
    let mut injection = String::new();
    if !lower.contains("<base ") {
        injection.push_str(&format!("\n<base href=\"/mods/{}/\" />", mod_id));
    }
    let ver = crate::updater::CURRENT_APP_VERSION;
    if !lower.contains("stosup-overlay.js") {
        injection.push_str(&format!(
            "\n<script src=\"/overlay/stosup-overlay.js?v={}\"></script>",
            ver
        ));
    }
    if !lower.contains("stosup-mod-sdk.js") {
        injection.push_str(&format!(
            "\n<script src=\"/overlay/stosup-mod-sdk.js?v={}\"></script>",
            ver
        ));
    }
    if injection.is_empty() {
        return raw_html.to_string();
    }
    if let Some(insert_at) = find_head_tag_insert_pos(&lower) {
        let mut out = String::with_capacity(raw_html.len() + injection.len() + 2);
        out.push_str(&raw_html[..insert_at]);
        out.push_str(&injection);
        out.push_str(&raw_html[insert_at..]);
        return out;
    }
    format!("{}\n{}", injection, raw_html)
}

async fn mod_custom_overlay_handler(
    Path((mod_id, overlay_id)): Path<(String, String)>,
    State(state): State<Arc<OverlaySharedState>>,
) -> Response<Body> {
    if !state.is_obs_overlay_enabled() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(
                "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'></head>\
                 <body style='margin:0;background:transparent;color:#a1a1aa;font-family:sans-serif;padding:12px;font-size:13px'>\
                 Stosup のオーバーレイスタジオ機能は現在オフになっています。</body></html>",
            ))
            .unwrap();
    }

    let entry_rel = {
        let mods = state.installed_mods.read().await;
        mods.iter()
            .find(|m| m.id == mod_id && m.enabled && m.load_error.is_none())
            .and_then(|m| {
                m.custom_overlays
                    .iter()
                    .find(|o| o.overlay_id == overlay_id)
                    .map(|o| o.entry.clone())
            })
    };

    let Some(entry_rel) = entry_rel else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(
                "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'></head>\
                 <body style='margin:0;background:transparent;color:#fca5a5;font-family:sans-serif;padding:12px;font-size:13px'>\
                 指定された MOD オーバーレイが見つからないか、現在無効になっています。</body></html>",
            ))
            .unwrap();
    };

    let Ok(html_path) = resolve_mod_asset_path(&state.mods_dir, &mod_id, &entry_rel) else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Mod overlay entry not found"))
            .unwrap();
    };

    let Ok(raw_html) = tokio::fs::read_to_string(&html_path).await else {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Failed to read mod overlay HTML"))
            .unwrap();
    };

    let final_html = inject_mod_overlay_head(&raw_html, &mod_id);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, NO_STORE_CACHE_CONTROL)
        .body(Body::from(final_html))
        .unwrap()
}

/// 導入済み MOD ディレクトリ配下の静的アセット（CSS/JS/画像/動画/音声/フォント等）を安全に配信する
async fn mod_asset_handler(
    Path((mod_id, asset_path)): Path<(String, String)>,
    State(state): State<Arc<OverlaySharedState>>,
) -> Response<Body> {
    if !state.is_obs_overlay_enabled() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Body::from("OBS overlay is disabled"))
            .unwrap();
    }

    let content_type = content_type_for_mod_asset(&asset_path);
    let is_safe_raster_icon =
        content_type.starts_with("image/") && content_type != "image/svg+xml";

    // 有効な MOD のみ全アセットを配信。無効化 MOD は管理画面表示用の安全なラスター画像アイコンのみ許可
    let is_enabled_or_icon = {
        let mods = state.installed_mods.read().await;
        mods.iter().any(|m| {
            m.id == mod_id
                && m.load_error.is_none()
                && (m.enabled
                    || (is_safe_raster_icon
                        && !m.icon_url.is_empty()
                        && m.icon_url == format!("/mods/{}/{}", mod_id, asset_path)))
        })
    };
    if !is_enabled_or_icon {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Mod is disabled or not installed"))
            .unwrap();
    }

    let Ok(resolved) = resolve_mod_asset_path(&state.mods_dir, &mod_id, &asset_path) else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Mod asset not found"))
            .unwrap();
    };

    let Ok(data) = tokio::fs::read(&resolved).await else {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Failed to read mod asset"))
            .unwrap();
    };

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONTENT_LENGTH, data.len());
    if content_type == "image/svg+xml" {
        builder = builder.header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; img-src data:; style-src 'unsafe-inline'; sandbox",
        );
    }
    builder.body(Body::from(data)).unwrap()
}

/// ローカル保存済みメディアファイルを配信する（Range シーク読み出し対応で大容量動画も高速・省メモリ）
async fn media_file_handler(
    Path(file_name): Path<String>,
    headers: HeaderMap,
    State(state): State<Arc<OverlaySharedState>>,
) -> Response<Body> {
    if !state.is_obs_overlay_enabled() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Body::from("OBS overlay is disabled"))
            .unwrap();
    }
    if !is_safe_stored_file_name(&file_name) {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid file name"))
            .unwrap();
    }

    let path = state.media_dir.join(&file_name);
    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Media not found"))
                .unwrap();
        }
    };

    let total_len = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Metadata error"))
                .unwrap();
        }
    };

    let content_type = content_type_for_file_name(&file_name);
    let cache_control = "public, max-age=86400, immutable";

    if let Some(range_val) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(range_spec) = range_val.strip_prefix("bytes=") {
            let mut parts = range_spec.split('-');
            let start = parts
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let requested_end = parts
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|e| e.min(total_len.saturating_sub(1)));

            if total_len > 0 && start < total_len {
                // Range 指定時は最大 2MB チャンクで必要部分のみ seek 読み出し
                const MAX_CHUNK: u64 = 2 * 1024 * 1024;
                let end = match requested_end {
                    Some(e) if e >= start => e.min(start.saturating_add(MAX_CHUNK - 1)),
                    _ => (total_len - 1).min(start.saturating_add(MAX_CHUNK - 1)),
                };
                let chunk_len = (end - start + 1) as usize;
                let mut buf = vec![0u8; chunk_len];
                if file.seek(SeekFrom::Start(start)).await.is_ok()
                    && file.read_exact(&mut buf).await.is_ok()
                {
                    let mut builder = Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
                        .header(header::ACCEPT_RANGES, "bytes")
                        .header(
                            header::CONTENT_RANGE,
                            format!("bytes {}-{}/{}", start, end, total_len),
                        )
                        .header(header::CONTENT_LENGTH, buf.len())
                        .header(header::CACHE_CONTROL, cache_control);
                    if content_type == "image/svg+xml" {
                        builder = builder.header(
                            header::CONTENT_SECURITY_POLICY,
                            "default-src 'none'; img-src data:; style-src 'unsafe-inline'; sandbox",
                        );
                    }
                    return builder.body(Body::from(buf)).unwrap();
                }
            }
        }
    }

    let mut data = Vec::with_capacity(total_len.min(32 * 1024 * 1024) as usize);
    if file.read_to_end(&mut data).await.is_err() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Read error"))
            .unwrap();
    }

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, cache_control);
    if content_type == "image/svg+xml" {
        builder = builder.header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; img-src data:; style-src 'unsafe-inline'; sandbox",
        );
    }
    builder.body(Body::from(data)).unwrap()
}

async fn index_handler() -> Html<&'static str> {
    Html(
        "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><link rel='icon' href='/favicon.ico'><title>Stosup オーバーレイスタジオ</title></head>\
         <body style='font-family:sans-serif;background:#18181b;color:#f4f4f5;padding:32px'>\
         <h1>Stosup オーバーレイスタジオ ＆ ポケットスイッチャー稼働中</h1>\
         <p style='margin:12px 0'><a style='color:#4ade80;font-weight:bold;font-size:18px' href='/remote'>▶ ポケットスイッチャーを開く (/remote)</a></p>\
         <h2 style='font-size:15px;color:#d4d4d8;margin-top:20px'>本番 OBS 用オーバーレイ</h2>\
         <ul>\
           <li><a style='color:#60a5fa' href='/overlay/comments'>/overlay/comments (トークモニター)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/superchat'>/overlay/superchat (エールボックス)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/pickup'>/overlay/pickup (スポットテロップ)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/poll'>/overlay/poll (ライブアンケート)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/queue'>/overlay/queue (エントリーレーン)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/media'>/overlay/media (リアクションステージ)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/lottery'>/overlay/lottery (ドローボックス)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/counter'>/overlay/counter (コールカウンター)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/first_time'>/overlay/first_time (ウェルカムカウンター)</a></li>\
         </ul>\
         <h2 style='font-size:15px;color:#fbbf24;margin-top:20px'>テスト・プレビュー専用オーバーレイ（本番データ非干渉）</h2>\
         <ul>\
           <li><a style='color:#fcd34d' href='/overlay/preview/comments'>/overlay/preview/comments</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/superchat'>/overlay/preview/superchat</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/pickup'>/overlay/preview/pickup</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/poll'>/overlay/preview/poll</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/queue'>/overlay/preview/queue</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/media'>/overlay/preview/media</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/lottery'>/overlay/preview/lottery</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/counter'>/overlay/preview/counter</a></li>\
           <li><a style='color:#fcd34d' href='/overlay/preview/first_time'>/overlay/preview/first_time</a></li>\
         </ul></body></html>",
    )
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<Arc<OverlaySharedState>>,
) -> axum::response::Response {
    if remote_api::is_cross_site_browser_request(&headers, false) {
        return (
            StatusCode::FORBIDDEN,
            "Cross-site WebSocket connection is not allowed",
        )
            .into_response();
    }
    if !state.is_obs_overlay_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "OBS overlay is currently disabled",
        )
            .into_response();
    }
    let channel = query
        .get("channel")
        .and_then(|s| OverlayChannel::from_query(s));
    let is_preview = query
        .get("preview")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    ws.on_upgrade(move |socket| handle_socket(socket, state, channel, is_preview))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<OverlaySharedState>,
    channel: Option<OverlayChannel>,
    is_preview: bool,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut obs_rx = state.obs_enabled_tx.subscribe();
    if !*obs_rx.borrow() {
        return;
    }

    let mut rx = if is_preview {
        state.preview_tx.subscribe()
    } else {
        state.tx.subscribe()
    };

    // チャンネル別の購読者数を RAII 追跡（該当チャンネル購読者 0 件時の JSON 化を完全スキップ）
    let sub_channel = channel.unwrap_or(OverlayChannel::All);
    let _sub_guard = ChannelSubGuard::acquire(
        Arc::clone(if is_preview {
            &state.preview_channel_subs
        } else {
            &state.channel_subs
        }),
        sub_channel,
    );

    let snapshot = if is_preview {
        state.build_preview_snapshot(channel).await
    } else {
        state.build_snapshot(channel).await
    };
    if let Ok(json) = serde_json::to_string(&snapshot) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // 単一タスクの select! ループで送受信・切断検知を完結（切断時の残留タスクを完全防止）
    loop {
        tokio::select! {
            recv = rx.recv() => {
                match recv {
                    Ok(payload) => {
                        if !payload.0.matches(channel) {
                            continue;
                        }
                        if sender
                            .send(Message::Text(payload.1.as_str().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            changed = obs_rx.changed() => {
                if changed.is_err() || !*obs_rx.borrow() {
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlatformKind;

    fn make_test_state() -> OverlaySharedState {
        OverlaySharedState::new(
            PathBuf::from("."),
            PathBuf::from("."),
            Vec::new(),
            RemoteAccessConfig::default(),
            OverlayConfig::default(),
            Vec::new(),
            FirstTimeCounterState::default(),
            true,
            Vec::new(),
            true,
            PollState::default(),
            true,
            QueueState::default(),
            true,
            MediaReactionConfig::default(),
            true,
            true,
        )
    }

    #[test]
    fn test_overlay_channel_query_and_matching() {
        assert_eq!(OverlayChannel::from_query("media"), Some(OverlayChannel::Media));
        assert_eq!(
            OverlayChannel::from_query("first_time"),
            Some(OverlayChannel::FirstTime)
        );
        assert_eq!(OverlayChannel::from_query("unknown"), None);

        // 特定チャンネルは同一チャンネルと All(ConfigUpdated) のみ受信
        assert!(OverlayChannel::Comments.matches(Some(OverlayChannel::Comments)));
        assert!(!OverlayChannel::Comments.matches(Some(OverlayChannel::Media)));
        assert!(OverlayChannel::All.matches(Some(OverlayChannel::Media)));

        // channel 未指定 (None) の外部クライアントは後方互換で全チャンネル受信
        assert!(OverlayChannel::Comments.matches(None));
        assert!(OverlayChannel::Media.matches(None));
    }

    #[tokio::test]
    async fn test_overlay_snapshot_channel_pruning_and_broadcast() {
        let state = make_test_state();
        state
            .push_comment(CommentItem {
                id: "c1".to_string(),
                session_id: "s1".to_string(),
                platform: PlatformKind::Demo,
                user_id: "u1".to_string(),
                user_name: "Tester".to_string(),
                user_icon_url: String::new(),
                user_memo: String::new(),
                is_first_time: false,
                message: "hello".to_string(),
                message_runs: Vec::new(),
                timestamp_ms: 1000,
                is_owner: false,
                is_moderator: false,
                is_member: false,
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
            })
            .await;

        // media チャンネル向け Snapshot には comments が含まれない（軽量化）
        match state.build_snapshot(Some(OverlayChannel::Media)).await {
            OverlayMessage::Snapshot(snap) => assert!(snap.comments.is_empty()),
            _ => panic!("expected Snapshot"),
        }
        // comments チャンネル向け Snapshot には comments が含まれる
        let comments_snap = state.build_snapshot(Some(OverlayChannel::Comments)).await;
        match &comments_snap {
            OverlayMessage::Snapshot(snap) => assert_eq!(snap.comments.len(), 1),
            _ => panic!("expected Snapshot"),
        }
        // Box 化後も Snapshot / ConfigUpdated のワイヤー JSON 形式がフラットに維持され、enum サイズも軽量であること
        assert!(std::mem::size_of::<OverlayMessage>() <= 512);
        let snap_val: serde_json::Value = serde_json::to_value(&comments_snap).unwrap();
        assert_eq!(snap_val["type"], "snapshot");
        assert!(snap_val["config"].is_object());
        assert_eq!(snap_val["comments"].as_array().unwrap().len(), 1);
        let cfg_val: serde_json::Value = serde_json::to_value(OverlayMessage::ConfigUpdated {
            config: Box::new(OverlayConfig::default()),
        })
        .unwrap();
        assert_eq!(cfg_val["type"], "config_updated");
        assert!(cfg_val["config"].is_object());

        // 購読中チャンネルのブロードキャストに正しいチャンネルタグが付与される
        let mut rx = state.tx.subscribe();
        state.clear_media();
        let payload = rx.recv().await.unwrap();
        assert_eq!(payload.0, OverlayChannel::Media);
        assert!(payload.1.contains("clear_media"));

        // OBS オーバーレイ無効化後はブロードキャストが停止され、切断シグナルも送出される
        let obs_rx = state.obs_enabled_tx.subscribe();
        state.set_obs_overlay_enabled(false);
        assert!(!state.is_obs_overlay_enabled());
        assert!(!*obs_rx.borrow());
        state.clear_media();
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_preview_channel_is_isolated_from_production() {
        let state = make_test_state();
        let mut prod_rx = state.tx.subscribe();
        let mut preview_rx = state.preview_tx.subscribe();

        // プレビューへコメントを送っても本番状態・本番チャンネルには一切届かない
        state
            .push_preview_comment(CommentItem {
                id: "prev-1".to_string(),
                session_id: "preview".to_string(),
                platform: PlatformKind::Demo,
                user_id: "pu1".to_string(),
                user_name: "PreviewTester".to_string(),
                user_icon_url: String::new(),
                user_memo: String::new(),
                is_first_time: false,
                message: "preview only".to_string(),
                message_runs: Vec::new(),
                timestamp_ms: 2000,
                is_owner: false,
                is_moderator: false,
                is_member: false,
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
            })
            .await;

        assert!(state.recent_comments.read().await.is_empty());
        assert!(prod_rx.try_recv().is_err());
        let p_msg = preview_rx.recv().await.unwrap();
        assert_eq!(p_msg.0, OverlayChannel::Comments);
        assert!(p_msg.1.contains("preview only"));

        // 本番へコメントを送ってもプレビューチャンネルには一切届かない
        state
            .push_comment(CommentItem {
                id: "prod-1".to_string(),
                session_id: "live".to_string(),
                platform: PlatformKind::Demo,
                user_id: "u-live".to_string(),
                user_name: "LiveViewer".to_string(),
                user_icon_url: String::new(),
                user_memo: String::new(),
                is_first_time: false,
                message: "production only".to_string(),
                message_runs: Vec::new(),
                timestamp_ms: 3000,
                is_owner: false,
                is_moderator: false,
                is_member: false,
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
            })
            .await;

        assert!(preview_rx.try_recv().is_err());
        let live_msg = prod_rx.recv().await.unwrap();
        assert!(live_msg.1.contains("production only"));

        // デザイン設定変更 (update_config) だけは本番・プレビュー双方に即時配信される
        state.update_config(OverlayConfig::default()).await;
        assert!(prod_rx.try_recv().is_ok());
        assert!(preview_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_per_channel_subscriber_skips_unrelated_broadcasts() {
        let state = make_test_state();
        let mut counter_rx = state.tx.subscribe();
        let guard = ChannelSubGuard::acquire(
            Arc::clone(&state.channel_subs),
            OverlayChannel::Counter,
        );

        // Counter のみ購読中 → Media イベントはシリアライズ・送信ごとスキップされる
        state.clear_media();
        assert!(counter_rx.try_recv().is_err());

        // Counter イベントと All (ConfigUpdated) イベントは正常に配信される
        state.update_counters(Vec::new()).await;
        let m1 = counter_rx.try_recv().unwrap();
        assert_eq!(m1.0, OverlayChannel::Counter);

        state.update_config(OverlayConfig::default()).await;
        let m2 = counter_rx.try_recv().unwrap();
        assert_eq!(m2.0, OverlayChannel::All);

        drop(guard);
        assert_eq!(
            state.channel_subs[OverlayChannel::Counter.index()].load(Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_inject_mod_overlay_head_ignores_header_tag() {
        let html = "<!DOCTYPE html><html><header>top</header><head><title>x</title></head><body></body></html>";
        let injected = inject_mod_overlay_head(html, "my-mod");
        // <header> 直後ではなく <head> 直後に注入されること
        let header_end = injected.find("</header>").unwrap();
        let base_pos = injected.find("<base href=\"/mods/my-mod/\"").unwrap();
        assert!(base_pos > header_end);
    }

    #[test]
    fn test_pin_rate_limiter_locks_after_five_failures() {
        let state = make_test_state();
        let ip: IpAddr = "192.168.1.99".parse().unwrap();
        assert!(state.check_pin_rate_limit(ip).is_none());
        for _ in 0..4 {
            assert!(state.record_pin_failure(ip).is_none());
        }
        assert!(state.record_pin_failure(ip).is_some());
        assert!(state.check_pin_rate_limit(ip).is_some());
        state.clear_pin_failures(ip);
        assert!(state.check_pin_rate_limit(ip).is_none());
    }

    #[test]
    fn test_overlay_html_cache_busting_and_sync_mods_guards() {
        let ver = crate::updater::CURRENT_APP_VERSION;
        let expected_script = format!("src=\"/overlay/stosup-overlay.js?v={}\"", ver);
        for (name, html) in [
            ("comments", COMMENTS_HTML),
            ("superchat", SUPERCHAT_HTML),
            ("pickup", PICKUP_HTML),
            ("poll", POLL_HTML),
            ("queue", QUEUE_HTML),
            ("lottery", LOTTERY_HTML),
            ("counter", COUNTER_HTML),
            ("first_time", FIRST_TIME_HTML),
            ("media", MEDIA_HTML),
        ] {
            let rendered = with_versioned_overlay_scripts(html);
            assert!(
                rendered.contains(&expected_script),
                "{name}.html にバージョン付き stosup-overlay.js が埋め込まれていません"
            );
            // 旧キャッシュ JS でもクラッシュしないよう、すべての syncMods 呼び出しに typeof ガードがあること
            for line in html.lines() {
                if line.contains("StosupOverlay.syncMods(") {
                    assert!(
                        line.contains("typeof window.StosupOverlay.syncMods === 'function'")
                            || html.contains("typeof window.StosupOverlay.syncMods === 'function'"),
                        "{name}.html に無ガードの syncMods 呼び出しが残っています: {line}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_start_overlay_server_with_retry_rebinds_same_port_after_stop() {
        let state = Arc::new(make_test_state());
        let mut h1 = start_overlay_server(0, false, Arc::clone(&state))
            .await
            .unwrap();
        let port = h1.port;
        h1.stop();

        let mut h2 = start_overlay_server_with_retry(port, false, Arc::clone(&state))
            .await
            .expect("停止直後の同一ポート再バインドがリトライ付きで成功すること");
        assert_eq!(h2.port, port);
        h2.stop();
    }
}
