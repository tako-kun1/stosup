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
    pub user_memo: String,
    pub is_first_time: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstTimeCounterState {
    /// 現在の配信での初見カウント（配信ごとに0へリセット可能）
    pub count: u64,
    /// ソフトウェア全体でこれまでに初見検知した累計人数
    pub total_detected_all_time: u64,
    /// 今回の配信で検知された初見ユーザー一覧（新しい順）
    pub recent_users: Vec<FirstTimeUser>,
    pub config: FirstTimeCounterConfig,
}

impl Default for FirstTimeCounterState {
    fn default() -> Self {
        Self {
            count: 0,
            total_detected_all_time: 0,
            recent_users: Vec::new(),
            config: FirstTimeCounterConfig::default(),
        }
    }
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
    /// コメント表示オーバーレイのデザインプリセット ("default" | "minimal" | "pop" | "neon" | "japanese")
    #[serde(default = "default_theme_name")]
    pub comments_theme: String,
    /// 抽選結果オーバーレイのデザインプリセット ("default" | "festive" | "minimal" | "neon" | "cute")
    #[serde(default = "default_theme_name")]
    pub lottery_theme: String,
    /// ワードカウンターオーバーレイのデザインプリセット ("default" | "compact" | "scoreboard" | "neon" | "pastel")
    #[serde(default = "default_theme_name")]
    pub counter_theme: String,
    /// 初見カウンターオーバーレイのデザインプリセット ("default" | "compact" | "scoreboard" | "neon" | "pastel")
    #[serde(default = "default_theme_name")]
    pub first_time_theme: String,
    /// 各オーバーレイの背景色 ("transparent" または "#RRGGBB" / "#RRGGBBAA")
    #[serde(default = "default_overlay_bg")]
    pub comments_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub lottery_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub counter_bg_color: String,
    #[serde(default = "default_overlay_bg")]
    pub first_time_bg_color: String,
    /// 各オーバーレイのシステムフォントファミリー名（空文字 = プリセット標準）
    #[serde(default)]
    pub comments_font_family: String,
    #[serde(default)]
    pub lottery_font_family: String,
    #[serde(default)]
    pub counter_font_family: String,
    #[serde(default)]
    pub first_time_font_family: String,
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
            comments_theme: default_theme_name(),
            lottery_theme: default_theme_name(),
            counter_theme: default_theme_name(),
            first_time_theme: default_theme_name(),
            comments_bg_color: default_overlay_bg(),
            lottery_bg_color: default_overlay_bg(),
            counter_bg_color: default_overlay_bg(),
            first_time_bg_color: default_overlay_bg(),
            comments_font_family: String::new(),
            lottery_font_family: String::new(),
            counter_font_family: String::new(),
            first_time_font_family: String::new(),
            comments_custom_css: String::new(),
            lottery_custom_css: String::new(),
            counter_custom_css: String::new(),
            first_time_custom_css: String::new(),
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
    /// OBS Studio オーバーレイローカルサーバー
    #[serde(default = "default_true")]
    pub obs_overlay_enabled: bool,
    /// 起動時の GitHub 自動アップデート確認
    #[serde(default = "default_true")]
    pub auto_update_check_enabled: bool,
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
            obs_overlay_enabled: true,
            auto_update_check_enabled: true,
        }
    }
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
    FeatureFlagsUpdated,
    SystemNotice,
}

/// Rust から Flutter UI へ送信するリアルタイムイベント
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
    pub feature_flags: Option<FeatureFlags>,
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
            feature_flags: None,
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

    pub fn feature_flags_updated(flags: FeatureFlags) -> Self {
        let mut e = Self::empty(AppEventKind::FeatureFlagsUpdated);
        e.feature_flags = Some(flags);
        e
    }

    pub fn system_notice(level: impl Into<String>, message: impl Into<String>) -> Self {
        let mut e = Self::empty(AppEventKind::SystemNotice);
        e.notice_level = Some(level.into());
        e.notice_message = Some(message.into());
        e
    }
}
