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
        }
    }
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

/// 配信から受信し、ルール評価済みのコメント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentItem {
    pub id: String,
    pub session_id: String,
    pub platform: PlatformKind,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub message: String,
    pub timestamp_ms: i64,
    pub is_owner: bool,
    pub is_moderator: bool,
    pub is_member: bool,
    pub is_banned: bool,
    pub highlight_rule_id: Option<String>,
    pub bubble_bg_color: Option<String>,
    pub bubble_text_color: Option<String>,
    pub bubble_border_color: Option<String>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for CommentLotteryState {
    fn default() -> Self {
        Self {
            is_accepting: false,
            started_at_ms: None,
            ended_at_ms: None,
            config: CommentLotteryConfig::default(),
            entries: Vec::new(),
            already_won_user_ids: Vec::new(),
            last_result: None,
            last_post_status: None,
        }
    }
}

/// OBS ブラウザソースオーバーレイの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub port: u16,
    pub comment_font_size: u32,
    pub comment_max_items: u32,
    pub show_icons: bool,
    pub show_timestamps: bool,
    pub default_bubble_bg: String,
    pub default_text_color: String,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            port: 18765,
            comment_font_size: 18,
            comment_max_items: 30,
            show_icons: true,
            show_timestamps: false,
            default_bubble_bg: "#1E293BE6".to_string(),
            default_text_color: "#F8FAFC".to_string(),
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

/// プラットフォームから受信する生のコメントデータ
#[derive(Debug, Clone)]
pub struct RawIncomingComment {
    pub id: String,
    pub platform: PlatformKind,
    pub user_id: String,
    pub user_name: String,
    pub user_icon_url: String,
    pub message: String,
    pub timestamp_ms: i64,
    pub is_owner: bool,
    pub is_moderator: bool,
    pub is_member: bool,
}

/// リアルタイムイベント種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppEventKind {
    CommentReceived,
    ConnectionChanged,
    CountersUpdated,
    UsersUpdated,
    CommentLotteryUpdated,
    LotteryDrawn,
    SystemNotice,
}

/// Rust から Flutter UI へ送信するリアルタイムイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEvent {
    pub kind: AppEventKind,
    pub comment: Option<CommentItem>,
    pub connection: Option<ConnectionStateDto>,
    pub counters: Option<Vec<WordCounterConfig>>,
    pub comment_lottery: Option<CommentLotteryState>,
    pub lottery_result: Option<LotteryResult>,
    pub notice_level: Option<String>,
    pub notice_message: Option<String>,
}

impl AppEvent {
    pub fn comment_received(comment: CommentItem) -> Self {
        Self {
            kind: AppEventKind::CommentReceived,
            comment: Some(comment),
            connection: None,
            counters: None,
            comment_lottery: None,
            lottery_result: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn connection_changed(state: ConnectionStateDto) -> Self {
        Self {
            kind: AppEventKind::ConnectionChanged,
            comment: None,
            connection: Some(state),
            counters: None,
            comment_lottery: None,
            lottery_result: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn counters_updated(counters: Vec<WordCounterConfig>) -> Self {
        Self {
            kind: AppEventKind::CountersUpdated,
            comment: None,
            connection: None,
            counters: Some(counters),
            comment_lottery: None,
            lottery_result: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn users_updated() -> Self {
        Self {
            kind: AppEventKind::UsersUpdated,
            comment: None,
            connection: None,
            counters: None,
            comment_lottery: None,
            lottery_result: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn comment_lottery_updated(state: CommentLotteryState) -> Self {
        Self {
            kind: AppEventKind::CommentLotteryUpdated,
            comment: None,
            connection: None,
            counters: None,
            comment_lottery: Some(state),
            lottery_result: None,
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn lottery_drawn(result: LotteryResult) -> Self {
        Self {
            kind: AppEventKind::LotteryDrawn,
            comment: None,
            connection: None,
            counters: None,
            comment_lottery: None,
            lottery_result: Some(result),
            notice_level: None,
            notice_message: None,
        }
    }

    pub fn system_notice(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: AppEventKind::SystemNotice,
            comment: None,
            connection: None,
            counters: None,
            comment_lottery: None,
            lottery_result: None,
            notice_level: Some(level.into()),
            notice_message: Some(message.into()),
        }
    }
}
