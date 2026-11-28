use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 対応配信プラットフォーム（将来の拡張を前提とした列挙型）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformKind {
    YouTube,
    Demo,
}

impl PlatformKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformKind::YouTube => "youtube",
            PlatformKind::Demo => "demo",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "youtube" => PlatformKind::YouTube,
            _ => PlatformKind::Demo,
        }
    }
}

/// 配信チャットへの接続方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionMode {
    /// YouTube Live InnerTube（APIクォータ消費なし・URLのみで取得可能）
    YouTubeInnerTube,
    /// YouTube Data API v3 公式ポーリング（OAuth認証が必要）
    YouTubeDataApi,
    /// ローカル検証用デモ配信モード
    Demo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStateDto {
    pub status: ConnectionStatus,
    pub platform: PlatformKind,
    pub mode: ConnectionMode,
    pub stream_title: String,
    pub stream_id: String,
    pub session_id: String,
    pub error_message: Option<String>,
    pub can_post_chat: bool,
    #[serde(default)]
    pub reconnect_attempt: u32,
}

impl Default for ConnectionStateDto {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            platform: PlatformKind::YouTube,
            mode: ConnectionMode::YouTubeInnerTube,
            stream_title: String::new(),
            stream_id: String::new(),
            session_id: String::new(),
            error_message: None,
            can_post_chat: false,
            reconnect_attempt: 0,
        }
    }
}

/// 過去の配信接続履歴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHistoryItem {
    pub id: String,
    pub platform: PlatformKind,
    pub stream_id: String,
    pub input_url: String,
    pub stream_title: String,
    pub mode: ConnectionMode,
    pub last_connected_ms: i64,
    pub is_favorite: bool,
}

/// 自動検出用のお気に入り登録チャンネル
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedChannelItem {
    pub id: String,
    pub label: String,
    /// 例: "@mychannel", "https://www.youtube.com/@mychannel", "UCxxxxxxxx"
    pub channel_input: String,
    pub last_video_id: String,
    pub last_checked_ms: i64,
}

/// チャンネルから自動解決した配信枠情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedYouTubeLive {
    pub video_id: String,
    pub title: String,
    pub is_live: bool,
    pub is_upcoming: bool,
    pub source_input: String,
}

/// コメント本文を構成するテキスト / 絵文字・スタンプ要素
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRunKind {
    Text,
    Emoji,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRun {
    pub kind: MessageRunKind,
    pub text: String,
    #[serde(default)]
    pub emoji_url: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub is_custom_emoji: bool,
}

/// ワードの一致判定方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchMode {
    Contains,
    Exact,
    Regex,
}

impl MatchMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchMode::Contains => "contains",
            MatchMode::Exact => "exact",
            MatchMode::Regex => "regex",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "exact" => MatchMode::Exact,
            "regex" => MatchMode::Regex,
            _ => MatchMode::Contains,
        }
    }
}

/// 特定ワード反応による吹き出し色変更ルール
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    pub bg_color: String,
    pub text_color: String,
    pub border_color: String,
    pub priority: i32,
    pub enabled: bool,
}

/// スーパーチャット・メンバーシップ等の支援イベント種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonetizationEventKind {
    SuperChat,
    SuperSticker,
    MembershipNew,
    MembershipMilestone,
    MembershipGift,
}

impl MonetizationEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MonetizationEventKind::SuperChat => "super_chat",
            MonetizationEventKind::SuperSticker => "super_sticker",
            MonetizationEventKind::MembershipNew => "membership_new",
            MonetizationEventKind::MembershipMilestone => "membership_milestone",
            MonetizationEventKind::MembershipGift => "membership_gift",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "super_sticker" => MonetizationEventKind::SuperSticker,
            "membership_new" => MonetizationEventKind::MembershipNew,
            "membership_milestone" => MonetizationEventKind::MembershipMilestone,
            "membership_gift" => MonetizationEventKind::MembershipGift,
            _ => MonetizationEventKind::SuperChat,
        }
    }
}

/// 配信から受信し、ルール評価済みのコメント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentItem {
    pub id: String,
    pub session_id: String,
    pub platform: PlatformKind,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub user_memo: String,
    pub is_first_time: bool,
    pub message: String,
    #[serde(default)]
    pub message_runs: Vec<MessageRun>,
    pub timestamp_ms: i64,
    pub is_owner: bool,
    pub is_moderator: bool,
    pub is_member: bool,
    pub is_banned: bool,
    #[serde(default)]
    pub is_ng_masked: bool,
    #[serde(default)]
    pub hidden_from_overlay: bool,
    pub highlight_rule_id: Option<String>,
    pub bubble_bg_color: Option<String>,
    pub bubble_text_color: Option<String>,
    pub bubble_border_color: Option<String>,
    /// スパチャ・メンバーシップ等のイベント種別（通常コメントは None）
    #[serde(default)]
    pub event_kind: Option<MonetizationEventKind>,
    /// スパチャ金額やメンバー継続期間などの表示ラベル
    #[serde(default)]
    pub amount_display: Option<String>,
    /// スパチャ・メンバーシップのティア強調色
    #[serde(default)]
    pub event_accent_color: Option<String>,
}

/// スーパーチャット / メンバーシップ専用管理アイテム（お礼済み・読み忘れチェック対応）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonetizationItem {
    pub id: String,
    pub session_id: String,
    pub platform: PlatformKind,
    pub kind: MonetizationEventKind,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub amount_display: String,
    pub amount_micros: u64,
    pub currency: String,
    pub tier_color: String,
    pub message: String,
    #[serde(default)]
    pub message_runs: Vec<MessageRun>,
    pub timestamp_ms: i64,
    pub is_checked: bool,
    pub checked_at_ms: Option<i64>,
}

/// リアルタイム投票の選択肢
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollOption {
    pub id: String,
    pub label: String,
    pub keywords: Vec<String>,
    pub color: String,
    pub vote_count: u64,
}

/// リアルタイム投票の設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollConfig {
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub options: Vec<PollOption>,
    /// 同一ユーザーが別選択肢へ投票を変更できるか
    #[serde(default = "default_true")]
    pub allow_vote_change: bool,
    #[serde(default = "default_true")]
    pub exclude_banned: bool,
    #[serde(default = "default_true")]
    pub show_percentage: bool,
    #[serde(default = "default_true")]
    pub show_vote_count: bool,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            question: "次の企画はどれがいい？".to_string(),
            options: vec![
                PollOption {
                    id: "opt-1".to_string(),
                    label: "雑談トーク".to_string(),
                    keywords: vec!["1".to_string(), "雑談".to_string()],
                    color: "#3B82F6".to_string(),
                    vote_count: 0,
                },
                PollOption {
                    id: "opt-2".to_string(),
                    label: "参加型ゲーム".to_string(),
                    keywords: vec!["2".to_string(), "ゲーム".to_string()],
                    color: "#22C55E".to_string(),
                    vote_count: 0,
                },
                PollOption {
                    id: "opt-3".to_string(),
                    label: "歌枠・リクエスト".to_string(),
                    keywords: vec!["3".to_string(), "歌".to_string()],
                    color: "#F59E0B".to_string(),
                    vote_count: 0,
                },
            ],
            allow_vote_change: true,
            exclude_banned: true,
            show_percentage: true,
            show_vote_count: true,
        }
    }
}

/// リアルタイム投票の現在状態
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PollState {
    pub is_active: bool,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub config: PollConfig,
    pub total_votes: u64,
}

/// コメントピックアップ（ピン留め表示）の現在状態
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PickedCommentState {
    pub comment: Option<CommentItem>,
    pub picked_at_ms: Option<i64>,
}

/// 参加型キューのエントリ状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueEntryStatus {
    Active,
    Waiting,
    Completed,
}

/// 参加型キューの並び順・優先方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueSortMode {
    /// 先着順
    FirstCome,
    /// 参加回数が少ない人優先（同数なら先着）
    FewestPlaysFirst,
    /// 初見さん優先 → 参加回数が少ない順 → 先着
    FirstTimerFirst,
}

/// 参加型キューの設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConfig {
    #[serde(default)]
    pub is_accepting: bool,
    #[serde(default = "default_queue_slots")]
    pub max_active_slots: u32,
    #[serde(default = "default_queue_join_keywords")]
    pub join_keywords: Vec<String>,
    #[serde(default = "default_queue_leave_keywords")]
    pub leave_keywords: Vec<String>,
    #[serde(default = "default_queue_sort_mode")]
    pub sort_mode: QueueSortMode,
    #[serde(default = "default_true")]
    pub exclude_banned: bool,
    #[serde(default = "default_true")]
    pub allow_rejoin_after_complete: bool,
}

fn default_queue_slots() -> u32 {
    3
}

fn default_queue_join_keywords() -> Vec<String> {
    vec!["参加希望".to_string(), "!join".to_string()]
}

fn default_queue_leave_keywords() -> Vec<String> {
    vec!["辞退".to_string(), "キャンセル".to_string(), "!leave".to_string()]
}

fn default_queue_sort_mode() -> QueueSortMode {
    QueueSortMode::FirstCome
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            is_accepting: false,
            max_active_slots: default_queue_slots(),
            join_keywords: default_queue_join_keywords(),
            leave_keywords: default_queue_leave_keywords(),
            sort_mode: default_queue_sort_mode(),
            exclude_banned: true,
            allow_rejoin_after_complete: true,
        }
    }
}

/// 参加型キューの1人分のエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub user_id: String,
    pub platform: PlatformKind,
    pub user_name: String,
    pub user_icon_url: String,
    pub status: QueueEntryStatus,
    pub play_count: u32,
    pub is_first_time: bool,
    pub note: String,
    pub joined_at_ms: i64,
}

/// 参加型キューの全体状態
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueState {
    pub config: QueueConfig,
    pub entries: Vec<QueueEntry>,
}

/// ワード反応メディア種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    Image,
    Video,
    Audio,
}

impl MediaKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Video => "video",
            MediaKind::Audio => "audio",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "video" => MediaKind::Video,
            "audio" => MediaKind::Audio,
            _ => MediaKind::Image,
        }
    }
}

/// WordParty 演出タイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaEffectType {
    /// 指定位置への固定表示（画像・GIF・透過WebM/MP4・音声）
    Fixed,
    /// 画面左右から勢いよく飛び出すクラッカー演出
    Cracker,
    /// 画面上部からアイテムがヒラヒラ降ってくる落下物演出
    Drop,
    /// 画面全体に紙吹雪が舞うお祝い演出
    Confetti,
    /// 画面端から放物線で投げ込まれてバウンドする投てき演出
    Throw,
    /// 雪・桜吹雪・雨・キラキラ・ハート・火の粉などの環境演出
    Ambient,
    /// ユーザー作成 MOD による独自カスタム演出
    Mod,
}

impl MediaEffectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaEffectType::Fixed => "fixed",
            MediaEffectType::Cracker => "cracker",
            MediaEffectType::Drop => "drop",
            MediaEffectType::Confetti => "confetti",
            MediaEffectType::Throw => "throw",
            MediaEffectType::Ambient => "ambient",
            MediaEffectType::Mod => "mod",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cracker" => MediaEffectType::Cracker,
            "drop" => MediaEffectType::Drop,
            "confetti" => MediaEffectType::Confetti,
            "throw" => MediaEffectType::Throw,
            "ambient" => MediaEffectType::Ambient,
            "mod" => MediaEffectType::Mod,
            _ => MediaEffectType::Fixed,
        }
    }
}

/// WordParty 演出の発動トリガー条件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaTriggerType {
    /// 指定キーワード（複数行可）に一致
    Keyword,
    /// すべてのコメントに反応
    AnyComment,
    /// スーパーチャット / スーパーステッカー受信時
    SuperChat,
    /// メンバーシップ加入・継続・ギフト受信時
    Membership,
    /// 初見さん検知時
    FirstTime,
}

impl MediaTriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaTriggerType::Keyword => "keyword",
            MediaTriggerType::AnyComment => "any_comment",
            MediaTriggerType::SuperChat => "super_chat",
            MediaTriggerType::Membership => "membership",
            MediaTriggerType::FirstTime => "first_time",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "any_comment" => MediaTriggerType::AnyComment,
            "super_chat" => MediaTriggerType::SuperChat,
            "membership" => MediaTriggerType::Membership,
            "first_time" => MediaTriggerType::FirstTime,
            _ => MediaTriggerType::Keyword,
        }
    }
}

/// 複数同時トリガー時のメディア再生方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaPlaybackMode {
    /// キューで順番に1つずつ再生
    Queue,
    /// 画面上に同時に重ねて再生
    Overlay,
    /// 再生中のものを上書きして最新のみ再生
    Replace,
}

/// メディア登場・退場アニメーション種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaAnimationKind {
    Fade,
    PopIn,
    SlideUp,
    SlideLeft,
    Bounce,
    ZoomSpin,
    None,
}

impl MediaAnimationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaAnimationKind::Fade => "fade",
            MediaAnimationKind::PopIn => "pop_in",
            MediaAnimationKind::SlideUp => "slide_up",
            MediaAnimationKind::SlideLeft => "slide_left",
            MediaAnimationKind::Bounce => "bounce",
            MediaAnimationKind::ZoomSpin => "zoom_spin",
            MediaAnimationKind::None => "none",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pop_in" => MediaAnimationKind::PopIn,
            "slide_up" => MediaAnimationKind::SlideUp,
            "slide_left" => MediaAnimationKind::SlideLeft,
            "bounce" => MediaAnimationKind::Bounce,
            "zoom_spin" => MediaAnimationKind::ZoomSpin,
            "none" => MediaAnimationKind::None,
            _ => MediaAnimationKind::Fade,
        }
    }
}

/// メディア表示位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaPosition {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    FullScreen,
}

impl MediaPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaPosition::Center => "center",
            MediaPosition::TopLeft => "top_left",
            MediaPosition::TopRight => "top_right",
            MediaPosition::BottomLeft => "bottom_left",
            MediaPosition::BottomRight => "bottom_right",
            MediaPosition::FullScreen => "full_screen",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "top_left" => MediaPosition::TopLeft,
            "top_right" => MediaPosition::TopRight,
            "bottom_left" => MediaPosition::BottomLeft,
            "bottom_right" => MediaPosition::BottomRight,
            "full_screen" => MediaPosition::FullScreen,
            _ => MediaPosition::Center,
        }
    }
}

/// ワード反応メディア全体の再生設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaReactionConfig {
    #[serde(default = "default_media_playback_mode")]
    pub playback_mode: MediaPlaybackMode,
    /// マスター音量 (0〜100)
    #[serde(default = "default_media_volume")]
    pub master_volume: u32,
    /// 重ね表示時の同時最大再生数
    #[serde(default = "default_media_max_concurrent")]
    pub max_concurrent: u32,
    /// 1つのコメントで複数ルールを同時発動する（false = 優先順位最上位1件のみ）
    #[serde(default = "default_true")]
    pub allow_multiple_match: bool,
}

fn default_media_playback_mode() -> MediaPlaybackMode {
    MediaPlaybackMode::Overlay
}

fn default_media_volume() -> u32 {
    80
}

fn default_media_max_concurrent() -> u32 {
    8
}

impl Default for MediaReactionConfig {
    fn default() -> Self {
        Self {
            playback_mode: default_media_playback_mode(),
            master_volume: default_media_volume(),
            max_concurrent: default_media_max_concurrent(),
            allow_multiple_match: true,
        }
    }
}

/// 特定ワード反応演出（WordParty）のルール
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaReactionRule {
    pub id: String,
    pub name: String,
    /// 反応ワード（改行区切りで複数登録可能）
    pub pattern: String,
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    /// 演出タイプ（固定表示 / クラッカー / 落下物 / 紙吹雪 / 投てき / 環境演出）
    pub effect_type: MediaEffectType,
    /// 発動トリガー条件（キーワード / 全コメント / スパチャ / メンバー / 初見）
    pub trigger_type: MediaTriggerType,
    /// スパチャ発動時の最低金額（0 = 全額）
    pub min_superchat_amount: u64,
    /// ルール優先順位（大きいほど先に判定）
    pub priority: i32,
    pub media_kind: MediaKind,
    /// Stosup 内にコピー保存された画像・動画・音声ファイル名（空 = テキスト/標準シェイプ）
    pub stored_file_name: String,
    /// ユーザーが選んだ元のファイル名
    pub original_file_name: String,
    /// 同時再生する効果音（SE）ファイル名（空 = 効果音なし）
    pub sound_stored_file_name: String,
    pub sound_original_file_name: String,
    pub sound_volume: u32,
    pub animation: MediaAnimationKind,
    pub position: MediaPosition,
    /// 表示・再生時間（ミリ秒）。動画で 0 の場合は動画の最後まで自動再生
    pub duration_ms: u32,
    /// 固定表示時の画面幅に対する割合（10〜100%）
    pub width_percent: u32,
    /// 動画/音声の音量（0〜100）
    pub volume: u32,
    /// パーティクル個数（1〜120）
    pub particle_count: u32,
    /// パーティクルサイズ px（24〜240）
    pub particle_size: u32,
    /// パーティクル速度 %（30〜250）
    pub particle_speed: u32,
    /// 重力強さ %（0〜200）
    pub gravity: u32,
    /// 画像未指定時に飛ばす絵文字・テキスト（カンマ区切りで複数指定可）
    pub particle_text: String,
    /// 環境演出プリセット ("snow" | "sakura" | "rain" | "sparkle" | "hearts" | "embers")
    pub ambient_preset: String,
    /// 連打防止クールダウン秒数
    pub cooldown_sec: u32,
    pub exclude_banned: bool,
    /// 任意テロップ（`{user}` で発動者名を置換、空なら非表示）
    pub caption_template: String,
    pub enabled: bool,
    /// MOD 演出使用時の MOD ID（例: "starlight-burst-effect"）
    #[serde(default)]
    pub mod_id: String,
    /// MOD 演出使用時の演出 ID（例: "starlight_burst"）
    #[serde(default)]
    pub mod_effect_id: String,
    /// MOD 演出へ渡すユーザー設定パラメータの JSON 文字列（例: "{}"）
    #[serde(default = "default_empty_json_object")]
    pub mod_params_json: String,
}

fn default_empty_json_object() -> String {
    "{}".to_string()
}

/// アプリ内へインポート完了したメディア情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedMediaFile {
    pub stored_file_name: String,
    pub original_file_name: String,
    pub media_kind: MediaKind,
    pub file_size_bytes: u64,
}

/// OBS メディアオーバーレイへ送信する再生トリガーイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTriggerEvent {
    pub event_id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub effect_type: MediaEffectType,
    pub media_url: String,
    pub media_kind: MediaKind,
    pub sound_url: Option<String>,
    pub sound_volume: u32,
    pub animation: MediaAnimationKind,
    pub position: MediaPosition,
    pub duration_ms: u32,
    pub width_percent: u32,
    pub volume: u32,
    pub particle_count: u32,
    pub particle_size: u32,
    pub particle_speed: u32,
    pub gravity: u32,
    pub particle_text: String,
    pub ambient_preset: String,
    pub caption: String,
    pub triggered_by: String,
    pub triggered_at_ms: i64,
    /// MOD 演出発火時の MOD ID
    #[serde(default)]
    pub mod_id: String,
    /// MOD 演出発火時の演出 ID
    #[serde(default)]
    pub mod_effect_id: String,
    /// MOD 演出へ渡すカスタムパラメータ JSON
    #[serde(default = "default_empty_json_object")]
    pub mod_params_json: String,
}

/// 初見カウンターの設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstTimeCounterConfig {
    /// 未登録ユーザーの初回コメントを初見として検知する
    #[serde(default = "default_true")]
    pub detect_unregistered: bool,
    /// 「初見」などの特定ワード発言で初見として検知する
    #[serde(default = "default_true")]
    pub detect_keyword: bool,
    /// 初見とみなすキーワード一覧
    #[serde(default = "default_first_time_keywords")]
    pub keywords: Vec<String>,
    /// 新しい配信に接続したとき表示カウントを自動で 0 にリセットする
    #[serde(default = "default_true")]
    pub auto_reset_on_new_stream: bool,
    /// 初見の人のアイコンと名前を表示するオプション（デフォルト: OFF）
    #[serde(default)]
    pub show_first_timer_profile: bool,
    /// BAN 済みユーザーを初見カウントから除外する
    #[serde(default = "default_true")]
    pub exclude_banned: bool,
    /// アクセントカラー
    #[serde(default = "default_first_time_accent")]
    pub accent_color: String,
}

fn default_first_time_keywords() -> Vec<String> {
    vec!["初見".to_string(), "しょけん".to_string()]
}

fn default_first_time_accent() -> String {
    "#16A34A".to_string()
}

impl Default for FirstTimeCounterConfig {
    fn default() -> Self {
        Self {
            detect_unregistered: true,
            detect_keyword: true,
            keywords: default_first_time_keywords(),
            auto_reset_on_new_stream: true,
            show_first_timer_profile: false,
            exclude_banned: true,
            accent_color: default_first_time_accent(),
        }
    }
}

/// 検知された初見ユーザー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstTimeUser {
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    /// "未登録" または "初見ワード"
    pub trigger_reason: String,
    pub detected_at_ms: i64,
}

/// 初見カウンターの現在状態
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FirstTimeCounterState {
    /// 現在の配信での初見カウント（配信ごとに0へリセット可能）
    pub count: u64,
    /// ソフトウェア全体でこれまでに初見検知した累計人数
    pub total_detected_all_time: u64,
    /// 今回の配信で検知された初見ユーザー一覧（新しい順）
    pub recent_users: Vec<FirstTimeUser>,
    pub config: FirstTimeCounterConfig,
}

/// 特定ワードカウンターの設定と現在カウント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCounterConfig {
    pub id: String,
    pub name: String,
    pub keywords: Vec<String>,
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    /// None = アカウント毎の回数上限なし / Some(n) = アカウント毎に n 回まで
    pub max_per_account: Option<u32>,
    pub exclude_banned: bool,
    pub accent_color: String,
    pub enabled: bool,
    pub total_count: u64,
    pub unique_user_count: u64,
}

/// コメントに参加したユーザー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: String,
    pub platform: PlatformKind,
    pub display_name: String,
    pub icon_url: String,
    /// 1配信につき1回でもコメントすれば +1 される視聴回数
    pub watch_count: u64,
    pub total_comment_count: u64,
    pub session_comment_count: u64,
    pub is_banned: bool,
    pub ban_reason: String,
    pub memo: String,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
    pub in_current_session: bool,
}

/// 抽選の重複許可設定
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LotteryRepeatMode {
    /// 1度のみ（一度当選したユーザーはリセットまで除外）
    OnceOnly,
    /// 複数回（すでに当選したユーザーも引き続き抽選対象）
    AllowMultiple,
}

/// ユーザー抽選の抽出条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLotteryRequest {
    pub current_session_only: bool,
    pub min_watch_count: u64,
    pub min_comment_count: u64,
    pub repeat_mode: LotteryRepeatMode,
    pub winner_count: u32,
}

/// 抽選の当選者情報（アイコン・名前・任意でコメント）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotteryWinner {
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub comment_text: Option<String>,
    pub won_at_ms: i64,
}

/// 抽選結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LotteryResult {
    pub id: String,
    /// "user" または "comment"
    pub kind: String,
    pub winners: Vec<LotteryWinner>,
    pub candidate_count: u32,
    pub drawn_at_ms: i64,
}

/// コメント抽選の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentLotteryConfig {
    pub start_message: String,
    pub end_message: String,
    pub target_keyword: String,
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    pub one_entry_per_user: bool,
    pub repeat_mode: LotteryRepeatMode,
    pub winner_count: u32,
    pub post_to_chat: bool,
}

impl Default for CommentLotteryConfig {
    fn default() -> Self {
        Self {
            start_message: "【抽選開始】「参加」とコメントして応募してください！".to_string(),
            end_message: "【抽選終了】応募を締め切りました！".to_string(),
            target_keyword: "参加".to_string(),
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            one_entry_per_user: true,
            repeat_mode: LotteryRepeatMode::OnceOnly,
            winner_count: 1,
            post_to_chat: true,
        }
    }
}

/// コメント抽選の応募エントリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentLotteryEntry {
    pub comment_id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub message: String,
    pub timestamp_ms: i64,
}

/// コメント抽選の現在状態
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentLotteryState {
    pub is_accepting: bool,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub config: CommentLotteryConfig,
    pub entries: Vec<CommentLotteryEntry>,
    pub already_won_user_ids: Vec<String>,
    pub last_result: Option<LotteryResult>,
    pub last_post_status: Option<String>,
}

/// ローカルネットワーク（LAN）接続・Web リモコン設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAccessConfig {
    /// ローカルネットワーク内の他端末からの接続を許可するか（デフォルト: OFF / 同一端末 127.0.0.1 のみ許可）
    #[serde(default)]
    pub allow_lan_access: bool,
    /// Web リモコン操作画面 (/remote および操作 API) を有効にするか（デフォルト: ON）
    #[serde(default = "default_true")]
    pub remote_control_enabled: bool,
    /// Web リモコン操作時に PIN コード認証を必須にするか（デフォルト: ON）
    #[serde(default = "default_true")]
    pub require_pin: bool,
    /// 6桁等のアクセス PIN コード
    #[serde(default = "default_remote_pin")]
    pub pin_code: String,
    /// QR コードに PIN を埋め込んでワンタップログインできるようにするか
    #[serde(default = "default_true")]
    pub embed_pin_in_qr: bool,
}

fn default_remote_pin() -> String {
    crate::remote::generate_pin_code()
}

impl Default for RemoteAccessConfig {
    fn default() -> Self {
        Self {
            allow_lan_access: false,
            remote_control_enabled: true,
            require_pin: true,
            pin_code: default_remote_pin(),
            embed_pin_in_qr: true,
        }
    }
}

/// QR コード描画用の 2D モジュール情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrCodeMatrix {
    pub size: u32,
    /// 行優先の size * size 個の真偽値（true = 黒ドット）
    pub modules: Vec<bool>,
}

/// ローカルネットワーク接続・Web リモコンの現在稼働状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAccessStatus {
    pub config: RemoteAccessConfig,
    pub server_running: bool,
    pub port: u16,
    pub bound_address: String,
    pub local_ips: Vec<String>,
    pub local_url: String,
    pub lan_urls: Vec<String>,
    pub primary_remote_url: String,
    pub qr_target_url: String,
    pub qr_code: Option<QrCodeMatrix>,
}

/// アプリ外観テーマ（ライト / ダーク / システム設定）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppThemeMode {
    System,
    Light,
    Dark,
}

impl AppThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppThemeMode::System => "system",
            AppThemeMode::Light => "light",
            AppThemeMode::Dark => "dark",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => AppThemeMode::Light,
            "dark" => AppThemeMode::Dark,
            _ => AppThemeMode::System,
        }
    }
}

fn default_theme_name() -> String {
    "default".to_string()
}

fn default_overlay_port() -> u16 {
    18765
}

fn default_comment_font_size() -> u32 {
    18
}

fn default_comment_max_items() -> u32 {
    30
}

fn default_bubble_bg() -> String {
    "#202124E6".to_string()
}

fn default_text_color() -> String {
    "#F4F4F5".to_string()
}

fn default_overlay_bg() -> String {
    "transparent".to_string()
}

/// OBS ブラウザソースオーバーレイの設定 ＆ 各機能デザインプリセット・背景色・カスタムCSS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayConfig {
    #[serde(default = "default_overlay_port")]
    pub port: u16,
    #[serde(default = "default_comment_font_size")]
    pub comment_font_size: u32,
    #[serde(default = "default_comment_max_items")]
    pub comment_max_items: u32,
    #[serde(default = "default_true")]
    pub show_icons: bool,
    #[serde(default)]
    pub show_timestamps: bool,
    #[serde(default = "default_bubble_bg")]
    pub default_bubble_bg: String,
    #[serde(default = "default_text_color")]
    pub default_text_color: String,
    /// コメント表示オーバーレイのデザインプリセット ("broadcast" | "editorial" | "tactical" | "variety" | "cinema" | "edge_glow" | "mod:{mod_id}:{theme_id}")
    #[serde(default = "default_theme_name")]
    pub comments_theme: String,
    /// 抽選結果オーバーレイのデザインプリセット ("broadcast" | "editorial" | "tactical" | "variety" | "cinema" | "edge_glow" | "mod:{mod_id}:{theme_id}")
    #[serde(default = "default_theme_name")]
    pub lottery_theme: String,
    /// ワードカウンターオーバーレイのデザインプリセット ("broadcast" | "editorial" | "tactical" | "variety" | "cinema" | "edge_glow" | "mod:{mod_id}:{theme_id}")
    #[serde(default = "default_theme_name")]
    pub counter_theme: String,
    /// 初見カウンターオーバーレイのデザインプリセット ("broadcast" | "editorial" | "tactical" | "variety" | "cinema" | "edge_glow" | "mod:{mod_id}:{theme_id}")
    #[serde(default = "default_theme_name")]
    pub first_time_theme: String,
    /// スパチャ・メンバーオーバーレイのデザインプリセット
    #[serde(default = "default_theme_name")]
    pub superchat_theme: String,
    /// リアルタイム投票オーバーレイのデザインプリセット
    #[serde(default = "default_theme_name")]
    pub poll_theme: String,
    /// コメントピックアップオーバーレイのデザインプリセット
    #[serde(default = "default_theme_name")]
    pub pickup_theme: String,
    /// 参加型キューオーバーレイのデザインプリセット
    #[serde(default = "default_theme_name")]
    pub queue_theme: String,
    /// ワード反応メディアオーバーレイのデザインプリセット
    #[serde(default = "default_theme_name")]
    pub media_theme: String,
    /// 各オーバーレイの背景色 ("transparent" または "#RRGGBB" / "#RRGGBBAA")
    #[serde(default = "default_overlay_bg")]
    pub comments_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub lottery_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub counter_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub first_time_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub superchat_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub poll_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub pickup_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub queue_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub media_bg_color: String,
    /// 各オーバーレイのシステムフォントファミリー名（空文字 = プリセット標準）
    #[serde(default)]
    pub comments_font_family: String,
    #[serde(default)]
    pub lottery_font_family: String,
    #[serde(default)]
    pub counter_font_family: String,
    #[serde(default)]
    pub first_time_font_family: String,
    #[serde(default)]
    pub superchat_font_family: String,
    #[serde(default)]
    pub poll_font_family: String,
    #[serde(default)]
    pub pickup_font_family: String,
    #[serde(default)]
    pub queue_font_family: String,
    #[serde(default)]
    pub media_font_family: String,
    /// コメント表示用ユーザーカスタム CSS
    #[serde(default)]
    pub comments_custom_css: String,
    /// 抽選結果用ユーザーカスタム CSS
    #[serde(default)]
    pub lottery_custom_css: String,
    /// ワードカウンター用ユーザーカスタム CSS
    #[serde(default)]
    pub counter_custom_css: String,
    /// 初見カウンター用ユーザーカスタム CSS
    #[serde(default)]
    pub first_time_custom_css: String,
    /// スパチャ・メンバー用ユーザーカスタム CSS
    #[serde(default)]
    pub superchat_custom_css: String,
    /// リアルタイム投票用ユーザーカスタム CSS
    #[serde(default)]
    pub poll_custom_css: String,
    /// コメントピックアップ用ユーザーカスタム CSS
    #[serde(default)]
    pub pickup_custom_css: String,
    /// 参加型キュー用ユーザーカスタム CSS
    #[serde(default)]
    pub queue_custom_css: String,
    /// ワード反応メディア用ユーザーカスタム CSS
    #[serde(default)]
    pub media_custom_css: String,
    /// 各オーバーレイ別の GUI ビジュアルスタイル設定（キー: "comments", "superchat", "pickup", "poll", "queue", "media", "lottery", "counter", "first_time"）
    #[serde(default)]
    pub visual_styles: HashMap<String, OverlayVisualStyle>,
}

/// ソフト単体で高度なデザイン・テキスト装飾を行うための構造化スタイル
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayVisualStyle {
    /// true のとき GUI スタイルをプリセットに上書き適用する
    #[serde(default)]
    pub enabled: bool,
    /// 本文フォントサイズ (px / 0 = プリセット既定)
    #[serde(default)]
    pub font_size: u32,
    /// フォントウェイト (0 = プリセット既定, 300..=900)
    #[serde(default)]
    pub font_weight: u32,
    /// 字間 (0.1px 単位: 例 5 = 0.5px, -5 = -0.5px)
    #[serde(default)]
    pub letter_spacing_tenths: i32,
    /// 行間 (% 単位: 0 = プリセット既定, 例 145 = 1.45)
    #[serde(default)]
    pub line_height_percent: u32,
    /// 文字色（空文字 = プリセット既定）
    #[serde(default)]
    pub text_color: String,
    /// 文字2色グラデーションを有効にするか
    #[serde(default)]
    pub use_text_gradient: bool,
    #[serde(default = "default_grad_start")]
    pub text_gradient_start: String,
    #[serde(default = "default_grad_end")]
    pub text_gradient_end: String,
    /// グラデーション角度 (0..=360)
    #[serde(default = "default_grad_angle")]
    pub text_gradient_angle: u32,
    /// 投稿者名・見出し色（空文字 = プリセット既定）
    #[serde(default)]
    pub author_color: String,
    /// アクセント色（空文字 = プリセット既定）
    #[serde(default)]
    pub accent_color: String,
    /// 内側縁取り（袋文字）太さ (px / 0 = なし)
    #[serde(default)]
    pub stroke_width: u32,
    #[serde(default = "default_stroke_color")]
    pub stroke_color: String,
    /// 外側二重縁取り太さ (px / 0 = なし)
    #[serde(default)]
    pub outer_stroke_width: u32,
    #[serde(default = "default_outer_stroke_color")]
    pub outer_stroke_color: String,
    /// 文字ドロップシャドウ
    #[serde(default)]
    pub text_shadow_enabled: bool,
    #[serde(default = "default_shadow_color")]
    pub text_shadow_color: String,
    #[serde(default = "default_shadow_blur")]
    pub text_shadow_blur: u32,
    #[serde(default = "default_shadow_offset")]
    pub text_shadow_offset_x: i32,
    #[serde(default = "default_shadow_offset")]
    pub text_shadow_offset_y: i32,
    /// テキストのエッジ発光（Edge Glow）を有効にするか
    #[serde(default)]
    pub text_glow_enabled: bool,
    /// エッジ発光色 (HEX)
    #[serde(default = "default_glow_color")]
    pub text_glow_color: String,
    /// エッジ発光の広がり（ぼかし半径: 1..=40px）
    #[serde(default = "default_glow_radius")]
    pub text_glow_radius: u32,
    /// エッジ発光の強度（10..=200%）
    #[serde(default = "default_glow_intensity")]
    pub text_glow_intensity: u32,
    /// エッジ発光の芯・輪郭にじみ太さ（0..=10px）
    #[serde(default = "default_glow_spread")]
    pub text_glow_spread: u32,
    /// カード背景色（空文字 = プリセット既定）
    #[serde(default)]
    pub card_bg_color: String,
    /// カード背景不透明度 (0..=100, デフォルト 92)
    #[serde(default = "default_card_opacity")]
    pub card_bg_opacity: u32,
    /// 角丸半径 (-1 = プリセット既定, 0..=28px)
    #[serde(default = "default_border_radius_inherit")]
    pub border_radius: i32,
    /// 枠線太さ (-1 = プリセット既定, 0..=6px)
    #[serde(default = "default_border_width_inherit")]
    pub border_width: i32,
    /// 枠線色（空文字 = プリセット既定）
    #[serde(default)]
    pub border_color: String,
    /// 影の強さ ("inherit" | "none" | "soft" | "medium" | "hard")
    #[serde(default = "default_inherit_str")]
    pub card_shadow: String,
    /// 余白サイズ ("inherit" | "compact" | "normal" | "relaxed")
    #[serde(default = "default_inherit_str")]
    pub padding_scale: String,
    /// 出現アニメーション ("inherit" | "wipe" | "slide" | "fade" | "pop" | "none")
    #[serde(default = "default_inherit_str")]
    pub entrance_animation: String,
}

fn default_grad_start() -> String {
    "#FFFFFF".to_string()
}
fn default_grad_end() -> String {
    "#FDE047".to_string()
}
fn default_grad_angle() -> u32 {
    180
}
fn default_stroke_color() -> String {
    "#111114".to_string()
}
fn default_outer_stroke_color() -> String {
    "#FFFFFF".to_string()
}
fn default_shadow_color() -> String {
    "#000000".to_string()
}
fn default_shadow_blur() -> u32 {
    6
}
fn default_shadow_offset() -> i32 {
    2
}
fn default_glow_color() -> String {
    "#38BDF8".to_string()
}
fn default_glow_radius() -> u32 {
    12
}
fn default_glow_intensity() -> u32 {
    85
}
fn default_glow_spread() -> u32 {
    2
}
fn default_card_opacity() -> u32 {
    92
}
fn default_border_radius_inherit() -> i32 {
    -1
}
fn default_border_width_inherit() -> i32 {
    -1
}
fn default_inherit_str() -> String {
    "inherit".to_string()
}

impl Default for OverlayVisualStyle {
    fn default() -> Self {
        Self {
            enabled: false,
            font_size: 0,
            font_weight: 0,
            letter_spacing_tenths: 0,
            line_height_percent: 0,
            text_color: String::new(),
            use_text_gradient: false,
            text_gradient_start: default_grad_start(),
            text_gradient_end: default_grad_end(),
            text_gradient_angle: default_grad_angle(),
            author_color: String::new(),
            accent_color: String::new(),
            stroke_width: 0,
            stroke_color: default_stroke_color(),
            outer_stroke_width: 0,
            outer_stroke_color: default_outer_stroke_color(),
            text_shadow_enabled: false,
            text_shadow_color: default_shadow_color(),
            text_shadow_blur: default_shadow_blur(),
            text_shadow_offset_x: default_shadow_offset(),
            text_shadow_offset_y: default_shadow_offset(),
            text_glow_enabled: false,
            text_glow_color: default_glow_color(),
            text_glow_radius: default_glow_radius(),
            text_glow_intensity: default_glow_intensity(),
            text_glow_spread: default_glow_spread(),
            card_bg_color: String::new(),
            card_bg_opacity: default_card_opacity(),
            border_radius: default_border_radius_inherit(),
            border_width: default_border_width_inherit(),
            border_color: String::new(),
            card_shadow: default_inherit_str(),
            padding_scale: default_inherit_str(),
            entrance_animation: default_inherit_str(),
        }
    }
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            port: default_overlay_port(),
            comment_font_size: default_comment_font_size(),
            comment_max_items: default_comment_max_items(),
            show_icons: true,
            show_timestamps: false,
            default_bubble_bg: default_bubble_bg(),
            default_text_color: default_text_color(),
            comments_theme: "broadcast".to_string(),
            lottery_theme: "broadcast".to_string(),
            counter_theme: "broadcast".to_string(),
            first_time_theme: "broadcast".to_string(),
            superchat_theme: "broadcast".to_string(),
            poll_theme: "broadcast".to_string(),
            pickup_theme: "broadcast".to_string(),
            queue_theme: "broadcast".to_string(),
            media_theme: "broadcast".to_string(),
            comments_bg_color: default_overlay_bg(),
            lottery_bg_color: default_overlay_bg(),
            counter_bg_color: default_overlay_bg(),
            first_time_bg_color: default_overlay_bg(),
            superchat_bg_color: default_overlay_bg(),
            poll_bg_color: default_overlay_bg(),
            pickup_bg_color: default_overlay_bg(),
            queue_bg_color: default_overlay_bg(),
            media_bg_color: default_overlay_bg(),
            comments_font_family: String::new(),
            lottery_font_family: String::new(),
            counter_font_family: String::new(),
            first_time_font_family: String::new(),
            superchat_font_family: String::new(),
            poll_font_family: String::new(),
            pickup_font_family: String::new(),
            queue_font_family: String::new(),
            media_font_family: String::new(),
            comments_custom_css: String::new(),
            lottery_custom_css: String::new(),
            counter_custom_css: String::new(),
            first_time_custom_css: String::new(),
            superchat_custom_css: String::new(),
            poll_custom_css: String::new(),
            pickup_custom_css: String::new(),
            queue_custom_css: String::new(),
            media_custom_css: String::new(),
            visual_styles: HashMap::new(),
        }
    }
}

/// YouTube OAuth 2.0 連携状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeOAuthStatus {
    pub client_id: String,
    pub has_client_secret: bool,
    pub is_authenticated: bool,
    pub account_name: Option<String>,
    pub expires_at_ms: Option<i64>,
}

/// GitHub Releases アップデート確認結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    /// pre が付くプレリリース版かどうか（true の場合は自動更新せず利用可能アナウンスのみ）
    pub is_prerelease: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: String,
    pub installer_url: Option<String>,
    pub installer_name: Option<String>,
    pub checksum_url: Option<String>,
    pub html_url: String,
}

/// アップデートダウンロード進捗
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub status: String,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

/// アップデート後初回起動時のリリースノート情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNotesInfo {
    pub version: String,
    pub should_show_on_startup: bool,
    pub title: String,
    pub markdown_body: String,
    pub previous_version: Option<String>,
}

/// 利用規約（ライセンス）の取得・同意状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseAgreementStatus {
    pub title: String,
    pub markdown_body: String,
    pub revision_label: String,
    pub content_hash: String,
    /// "online" | "cached" | "bundled"
    pub source: String,
    pub accepted_hash: Option<String>,
    pub accepted_revision: Option<String>,
    pub accepted_at_ms: Option<i64>,
    pub needs_acceptance: bool,
    pub is_updated_license: bool,
}

/// プラットフォームから受信する生のコメントデータ
#[derive(Debug, Clone)]
pub struct RawIncomingComment {
    pub id: String,
    pub platform: PlatformKind,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub message: String,
    pub message_runs: Vec<MessageRun>,
    pub timestamp_ms: i64,
    pub is_owner: bool,
    pub is_moderator: bool,
    pub is_member: bool,
    pub event_kind: Option<MonetizationEventKind>,
    pub amount_display: Option<String>,
    pub amount_micros: u64,
    pub currency: String,
    pub tier_color: Option<String>,
}

/// NG ワード検知時の動作モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NgWordAction {
    /// アプリ・OBS・各種集計すべてから完全に除外する
    HideEverywhere,
    /// アプリ内のトークモニターでは警告表示し、OBS オーバーレイへは流さない
    HideOnOverlay,
    /// 該当ワードを指定文字（例: ***）に伏せ字置換して表示する
    MaskText,
}

impl NgWordAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            NgWordAction::HideEverywhere => "hide_everywhere",
            NgWordAction::HideOnOverlay => "hide_on_overlay",
            NgWordAction::MaskText => "mask_text",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hide_everywhere" => NgWordAction::HideEverywhere,
            "hide_on_overlay" => NgWordAction::HideOnOverlay,
            _ => NgWordAction::MaskText,
        }
    }
}

/// NG ワード防御ルール
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NgWordRule {
    pub id: String,
    pub pattern: String,
    pub match_mode: MatchMode,
    pub case_sensitive: bool,
    pub action: NgWordAction,
    #[serde(default = "default_ng_replacement")]
    pub replacement: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_ng_replacement() -> String {
    "***".to_string()
}

/// OBS WebSocket 自動連動トリガー種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObsTriggerType {
    SuperChatMinAmount,
    Membership,
    Keyword,
    FirstTime,
    LotteryWinner,
    Manual,
}

/// OBS WebSocket 自動連動アクション種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObsActionType {
    SwitchScene,
    SetSceneItemVisible,
    ToggleSceneItem,
}

/// OBS WebSocket 自動連動ルール
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObsAutomationRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub trigger_type: ObsTriggerType,
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub min_amount_jpy: u64,
    pub action_type: ObsActionType,
    #[serde(default)]
    pub target_scene: String,
    #[serde(default)]
    pub target_source: String,
    #[serde(default = "default_true")]
    pub set_visible: bool,
    /// 0 の場合は復帰しない。1 以上なら指定ミリ秒後に反対の表示状態へ自動復帰
    #[serde(default)]
    pub auto_revert_ms: u32,
}

/// OBS WebSocket v5 接続＆自動連動設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObsWsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_obs_host")]
    pub host: String,
    #[serde(default = "default_obs_port")]
    pub port: u16,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]
    pub reset_first_time_on_stream_start: bool,
    #[serde(default)]
    pub reset_counters_on_stream_start: bool,
    #[serde(default)]
    pub rules: Vec<ObsAutomationRule>,
}

fn default_obs_host() -> String {
    "127.0.0.1".to_string()
}
fn default_obs_port() -> u16 {
    4455
}

impl Default for ObsWsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_obs_host(),
            port: default_obs_port(),
            password: String::new(),
            auto_connect: false,
            reset_first_time_on_stream_start: false,
            reset_counters_on_stream_start: false,
            rules: Vec::new(),
        }
    }
}

/// OBS WebSocket 現在の接続状態と取得済みシーン／ソース一覧
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsWsStatus {
    pub connected: bool,
    pub connecting: bool,
    pub obs_version: String,
    pub current_scene: String,
    pub scenes: Vec<String>,
    /// シーン名 -> ソース名一覧
    pub scene_sources: HashMap<String, Vec<String>>,
    pub is_streaming: bool,
    pub last_error: Option<String>,
}

/// グローバルホットキー / Stream Deck トリガーで実行可能なアクション
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomationActionKind {
    ClearPickup,
    PickLatestComment,
    TriggerMediaRule,
    ClearMedia,
    EndPoll,
    AdvanceQueue,
    ResetFirstTime,
    ToggleFeature,
    ObsSwitchScene,
    ObsToggleSource,
}

/// グローバルホットキー / Stream Deck 連携バインディング
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBinding {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    /// 例: "F1".."F12", "0".."9", "A".."Z", "SPACE", "ESCAPE"
    #[serde(default)]
    pub key: String,
    pub action: AutomationActionKind,
    /// ルール ID / シーン名 / 機能キーなどの追加引数
    #[serde(default)]
    pub target_id: String,
    /// ソース名等の第2引数
    #[serde(default)]
    pub secondary_target: String,
}

/// 配信プロファイル（機能ON/OFF・オーバーレイデザイン・ルール有効状態のスナップショット）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub feature_flags: FeatureFlags,
    pub overlay_config: OverlayConfig,
    /// ルール ID -> enabled のスナップショット
    #[serde(default)]
    pub highlight_enabled_map: HashMap<String, bool>,
    #[serde(default)]
    pub counter_enabled_map: HashMap<String, bool>,
    #[serde(default)]
    pub media_enabled_map: HashMap<String, bool>,
    #[serde(default)]
    pub ng_word_enabled_map: HashMap<String, bool>,
    pub updated_at_ms: i64,
}

/// プロファイル一覧表示用の軽量サマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProfileSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub updated_at_ms: i64,
    pub is_active: bool,
}

/// バックアップ書き出し結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupExportResult {
    pub file_path: String,
    pub size_bytes: u64,
    pub media_file_count: u32,
    pub created_at_ms: i64,
}

/// バックアップ復元結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupImportResult {
    pub restored_from: String,
    pub safety_backup_path: String,
    pub media_file_count: u32,
    pub restored_at_ms: i64,
}

/// オンエアレポート: セッション一覧項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSessionSummary {
    pub session_id: String,
    pub platform: PlatformKind,
    pub stream_id: String,
    pub title: String,
    pub started_at_ms: i64,
    pub last_comment_at_ms: i64,
    pub total_comments: u64,
    pub unique_users: u64,
    pub first_time_users: u64,
    pub monetization_count: u64,
}

/// オンエアレポート: 1分ごとの盛り上がりバケット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMinuteBucket {
    pub minute_offset: u32,
    pub timestamp_ms: i64,
    pub comment_count: u64,
    pub monetization_count: u64,
    pub first_time_count: u64,
    pub sample_messages: Vec<String>,
}

/// オンエアレポート: 盛り上がりピーク区間
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPeakItem {
    pub minute_offset: u32,
    pub elapsed_label: String,
    pub timestamp_ms: i64,
    pub comment_count: u64,
    pub sample_messages: Vec<String>,
}

/// オンエアレポート: 頻出ワード集計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportWordRank {
    pub word: String,
    pub count: u64,
}

/// オンエアレポート: アクティブ視聴者集計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportUserRank {
    pub user_id: String,
    pub user_name: String,
    pub comment_count: u64,
    pub is_member: bool,
}

/// オンエアレポート詳細 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnAirReportDto {
    pub session: StreamSessionSummary,
    pub duration_minutes: u32,
    pub avg_comments_per_min_tenths: u32,
    pub max_comments_per_min: u64,
    pub member_comment_count: u64,
    pub monetization_totals: Vec<String>,
    pub buckets: Vec<ReportMinuteBucket>,
    pub peaks: Vec<ReportPeakItem>,
    pub top_words: Vec<ReportWordRank>,
    pub top_users: Vec<ReportUserRank>,
}

fn default_true() -> bool {
    true
}

/// 各機能の有効 / 無効（ON / OFF）切り替え設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// コメント表示機能（アプリ内・OBSコメント表示）
    #[serde(default = "default_true")]
    pub comment_display_enabled: bool,
    /// 特定ワード反応（吹き出し背景色の変更）
    #[serde(default = "default_true")]
    pub word_highlight_enabled: bool,
    /// 特定ワードカウント機能
    #[serde(default = "default_true")]
    pub word_counter_enabled: bool,
    /// ユーザー抽選システム
    #[serde(default = "default_true")]
    pub user_lottery_enabled: bool,
    /// コメント抽選システム
    #[serde(default = "default_true")]
    pub comment_lottery_enabled: bool,
    /// 参加ユーザーリスト自動記録（視聴回数・コメント回数集計）
    #[serde(default = "default_true")]
    pub user_tracking_enabled: bool,
    /// コメント欄上の名前メモ表示
    #[serde(default = "default_true")]
    pub user_memo_display_enabled: bool,
    /// 初見カウンター機能
    #[serde(default = "default_true")]
    pub first_time_counter_enabled: bool,
    /// スーパーチャット / メンバーシップ専用管理機能
    #[serde(default = "default_true")]
    pub superchat_enabled: bool,
    /// リアルタイム投票 / アンケート機能
    #[serde(default = "default_true")]
    pub poll_enabled: bool,
    /// コメントピックアップ（ピン留め表示）機能
    #[serde(default = "default_true")]
    pub pickup_enabled: bool,
    /// 参加型キュー（順番待ちリスト）機能
    #[serde(default = "default_true")]
    pub queue_enabled: bool,
    /// 特定ワード反応メディアアニメーション機能
    #[serde(default = "default_true")]
    pub media_reaction_enabled: bool,
    /// NG ワード防御フィルタ機能
    #[serde(default = "default_true")]
    pub ng_word_filter_enabled: bool,
    /// OBS Studio オーバーレイローカルサーバー
    #[serde(default = "default_true")]
    pub obs_overlay_enabled: bool,
    /// 起動時の GitHub 自動アップデート確認
    #[serde(default = "default_true")]
    pub auto_update_check_enabled: bool,
    /// 先行版（pre）の更新アナウンスを含めるかどうか（デフォルト: OFF）
    #[serde(default)]
    pub include_prerelease_updates: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            comment_display_enabled: true,
            word_highlight_enabled: true,
            word_counter_enabled: true,
            user_lottery_enabled: true,
            comment_lottery_enabled: true,
            user_tracking_enabled: true,
            user_memo_display_enabled: true,
            first_time_counter_enabled: true,
            superchat_enabled: true,
            poll_enabled: true,
            pickup_enabled: true,
            queue_enabled: true,
            media_reaction_enabled: true,
            ng_word_filter_enabled: true,
            obs_overlay_enabled: true,
            auto_update_check_enabled: true,
            include_prerelease_updates: false,
        }
    }
}

/// MOD パラメータの入力型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModParamKind {
    String,
    Number,
    Color,
    Bool,
    Select,
}

/// リアクション演出 MOD が公開するカスタム設定パラメータ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModParamSchema {
    pub key: String,
    pub label: String,
    pub kind: ModParamKind,
    pub default_value: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub options: Vec<String>,
    pub description: String,
}

/// MOD が提供するリアクションステージ独自演出定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModReactionEffectDef {
    pub mod_id: String,
    pub mod_name: String,
    pub effect_id: String,
    pub name: String,
    pub description: String,
    pub entry: String,
    pub css: String,
    pub default_duration_ms: u32,
    pub params: Vec<ModParamSchema>,
}

/// MOD が提供する 9 種 OBS オーバーレイ向け追加テーマ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModOverlayThemeDef {
    pub mod_id: String,
    pub mod_name: String,
    pub theme_id: String,
    /// `mod:{mod_id}:{theme_id}` 形式の保存用テーマ ID
    pub full_theme_id: String,
    pub name: String,
    pub description: String,
    pub targets: Vec<String>,
    pub css: String,
    pub script: String,
    pub preview_bg: String,
    pub preview_fg: String,
    pub preview_border: String,
}

/// MOD が提供する独立カスタムオーバーレイ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModCustomOverlayDef {
    pub mod_id: String,
    pub mod_name: String,
    pub overlay_id: String,
    pub name: String,
    pub description: String,
    pub entry: String,
    pub channels: Vec<String>,
    pub recommended_width: u32,
    pub recommended_height: u32,
    /// 本番オーバーレイ相対パス (`/overlay/mod/{mod_id}/{overlay_id}`)
    pub overlay_path: String,
    /// 本番非干渉プレビュー相対パス (`/overlay/preview/mod/{mod_id}/{overlay_id}`)
    pub preview_path: String,
}

/// MOD が提供する Rhai ロジックスクリプト定義（Rust 不要で裏側の条件判定・演出制御を記述）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLogicScriptDef {
    pub mod_id: String,
    pub mod_name: String,
    pub script_id: String,
    pub name: String,
    pub description: String,
    pub entry: String,
    /// 購読するイベント一覧 ("comment" / "superchat" / "first_time" / "lottery")
    pub events: Vec<String>,
}

/// ロジックスクリプトの実行ログエントリ（MOD 管理画面デバッグコンソール向け）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModScriptLogEntry {
    pub timestamp_ms: i64,
    pub mod_id: String,
    pub script_id: String,
    /// "info" / "warn" / "error" / "action"
    pub level: String,
    pub message: String,
}

/// DB 永続化用の MOD 有効/無効および導入日時状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSavedState {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub installed_at_ms: i64,
}

/// 導入済み MOD パッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub min_stosup_version: String,
    pub homepage: String,
    pub license: String,
    pub icon_url: String,
    pub folder_name: String,
    pub enabled: bool,
    pub installed_at_ms: i64,
    pub load_error: Option<String>,
    pub reaction_effects: Vec<ModReactionEffectDef>,
    pub overlay_themes: Vec<ModOverlayThemeDef>,
    pub custom_overlays: Vec<ModCustomOverlayDef>,
    pub logic_scripts: Vec<ModLogicScriptDef>,
}

/// リアルタイムイベント種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppEventKind {
    CommentReceived,
    CommentsReplaced,
    ConnectionChanged,
    CountersUpdated,
    FirstTimeUpdated,
    UsersUpdated,
    CommentLotteryUpdated,
    LotteryDrawn,
    MonetizationUpdated,
    PollUpdated,
    PickedCommentUpdated,
    QueueUpdated,
    MediaTriggered,
    MediaRulesUpdated,
    MediaConfigUpdated,
    HighlightRulesUpdated,
    NgWordRulesUpdated,
    OverlayConfigUpdated,
    UserLotteryWinnersUpdated,
    FeatureFlagsUpdated,
    ObsWsStatusChanged,
    ProfilesUpdated,
    ConnectionHistoryUpdated,
    ModsUpdated,
    SystemNotice,
}

/// Rust から Flutter UI・Web リモコンへ送信するリアルタイムイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEvent {
    pub kind: AppEventKind,
    pub comment: Option<CommentItem>,
    pub comments: Option<Vec<CommentItem>>,
    pub connection: Option<ConnectionStateDto>,
    pub counters: Option<Vec<WordCounterConfig>>,
    pub first_time_state: Option<FirstTimeCounterState>,
    pub comment_lottery: Option<CommentLotteryState>,
    pub lottery_result: Option<LotteryResult>,
    pub monetization_items: Option<Vec<MonetizationItem>>,
    pub poll_state: Option<PollState>,
    pub picked_comment: Option<PickedCommentState>,
    pub queue_state: Option<QueueState>,
    pub media_trigger: Option<MediaTriggerEvent>,
    pub media_rules: Option<Vec<MediaReactionRule>>,
    pub media_config: Option<MediaReactionConfig>,
    pub highlight_rules: Option<Vec<HighlightRule>>,
    pub ng_word_rules: Option<Vec<NgWordRule>>,
    pub overlay_config: Option<OverlayConfig>,
    pub user_lottery_won_ids: Option<Vec<String>>,
    pub feature_flags: Option<FeatureFlags>,
    pub obs_ws_status: Option<ObsWsStatus>,
    pub profiles: Option<Vec<StreamProfileSummary>>,
    pub mods: Option<Vec<InstalledMod>>,
    pub notice_level: Option<String>,
    pub notice_message: Option<String>,
}

impl AppEvent {
    fn empty(kind: AppEventKind) -> Self {
        Self {
            kind,
            comment: None,
            comments: None,
            connection: None,
            counters: None,
            first_time_state: None,
            comment_lottery: None,
            lottery_result: None,
            monetization_items: None,
            poll_state: None,
            picked_comment: None,
            queue_state: None,
            media_trigger: None,
            media_rules: None,
            media_config: None,
            highlight_rules: None,
            ng_word_rules: None,
            overlay_config: None,
            user_lottery_won_ids: None,
            feature_flags: None,
            obs_ws_status: None,
            profiles: None,
            mods: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn comment_received(comment: CommentItem) -> Self {
        let mut e = Self::empty(AppEventKind::CommentReceived);
        e.comment = Some(comment);
        e
    }

    pub fn comments_replaced(comments: Vec<CommentItem>) -> Self {
        let mut e = Self::empty(AppEventKind::CommentsReplaced);
        e.comments = Some(comments);
        e
    }

    pub fn connection_changed(state: ConnectionStateDto) -> Self {
        let mut e = Self::empty(AppEventKind::ConnectionChanged);
        e.connection = Some(state);
        e
    }

    pub fn counters_updated(counters: Vec<WordCounterConfig>) -> Self {
        let mut e = Self::empty(AppEventKind::CountersUpdated);
        e.counters = Some(counters);
        e
    }

    pub fn first_time_updated(state: FirstTimeCounterState) -> Self {
        let mut e = Self::empty(AppEventKind::FirstTimeUpdated);
        e.first_time_state = Some(state);
        e
    }

    pub fn users_updated() -> Self {
        Self::empty(AppEventKind::UsersUpdated)
    }

    pub fn comment_lottery_updated(state: CommentLotteryState) -> Self {
        let mut e = Self::empty(AppEventKind::CommentLotteryUpdated);
        e.comment_lottery = Some(state);
        e
    }

    pub fn lottery_drawn(result: LotteryResult) -> Self {
        let mut e = Self::empty(AppEventKind::LotteryDrawn);
        e.lottery_result = Some(result);
        e
    }

    pub fn monetization_updated(items: Vec<MonetizationItem>) -> Self {
        let mut e = Self::empty(AppEventKind::MonetizationUpdated);
        e.monetization_items = Some(items);
        e
    }

    pub fn poll_updated(state: PollState) -> Self {
        let mut e = Self::empty(AppEventKind::PollUpdated);
        e.poll_state = Some(state);
        e
    }

    pub fn picked_comment_updated(state: PickedCommentState) -> Self {
        let mut e = Self::empty(AppEventKind::PickedCommentUpdated);
        e.picked_comment = Some(state);
        e
    }

    pub fn queue_updated(state: QueueState) -> Self {
        let mut e = Self::empty(AppEventKind::QueueUpdated);
        e.queue_state = Some(state);
        e
    }

    pub fn media_triggered(trigger: MediaTriggerEvent) -> Self {
        let mut e = Self::empty(AppEventKind::MediaTriggered);
        e.media_trigger = Some(trigger);
        e
    }

    pub fn media_rules_updated(rules: Vec<MediaReactionRule>) -> Self {
        let mut e = Self::empty(AppEventKind::MediaRulesUpdated);
        e.media_rules = Some(rules);
        e
    }

    pub fn media_config_updated(config: MediaReactionConfig) -> Self {
        let mut e = Self::empty(AppEventKind::MediaConfigUpdated);
        e.media_config = Some(config);
        e
    }

    pub fn highlight_rules_updated(rules: Vec<HighlightRule>) -> Self {
        let mut e = Self::empty(AppEventKind::HighlightRulesUpdated);
        e.highlight_rules = Some(rules);
        e
    }

    pub fn ng_word_rules_updated(rules: Vec<NgWordRule>) -> Self {
        let mut e = Self::empty(AppEventKind::NgWordRulesUpdated);
        e.ng_word_rules = Some(rules);
        e
    }

    pub fn overlay_config_updated(config: OverlayConfig) -> Self {
        let mut e = Self::empty(AppEventKind::OverlayConfigUpdated);
        e.overlay_config = Some(config);
        e
    }

    pub fn user_lottery_winners_updated(ids: Vec<String>) -> Self {
        let mut e = Self::empty(AppEventKind::UserLotteryWinnersUpdated);
        e.user_lottery_won_ids = Some(ids);
        e
    }

    pub fn feature_flags_updated(flags: FeatureFlags) -> Self {
        let mut e = Self::empty(AppEventKind::FeatureFlagsUpdated);
        e.feature_flags = Some(flags);
        e
    }

    pub fn obs_ws_status_changed(status: ObsWsStatus) -> Self {
        let mut e = Self::empty(AppEventKind::ObsWsStatusChanged);
        e.obs_ws_status = Some(status);
        e
    }

    pub fn profiles_updated(profiles: Vec<StreamProfileSummary>) -> Self {
        let mut e = Self::empty(AppEventKind::ProfilesUpdated);
        e.profiles = Some(profiles);
        e
    }

    pub fn connection_history_updated() -> Self {
        Self::empty(AppEventKind::ConnectionHistoryUpdated)
    }

    pub fn mods_updated(mods: Vec<InstalledMod>) -> Self {
        let mut e = Self::empty(AppEventKind::ModsUpdated);
        e.mods = Some(mods);
        e
    }

    pub fn system_notice(level: impl Into<String>, message: impl Into<String>) -> Self {
        let mut e = Self::empty(AppEventKind::SystemNotice);
        e.notice_level = Some(level.into());
        e.notice_message = Some(message.into());
        e
    }
}
