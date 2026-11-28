use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{
    AppThemeMode, ConnectionHistoryItem, ConnectionMode, FeatureFlags, FirstTimeCounterConfig,
    HighlightRule, HotkeyBinding, LotteryResult, LotteryWinner, MatchMode, MediaAnimationKind,
    MediaEffectType, MediaKind, MediaPosition, MediaReactionConfig, MediaReactionRule,
    MediaTriggerType, MessageRun, MonetizationEventKind, MonetizationItem, NgWordAction,
    NgWordRule, ObsWsConfig, OverlayConfig, PlatformKind, PollConfig, QueueState,
    RawIncomingComment, RemoteAccessConfig, SavedChannelItem, StreamProfile, StreamSessionSummary,
    UserRecord, WordCounterConfig,
};

/// オンエアレポート集計用のコメント履歴 1 行
#[derive(Debug, Clone)]
pub struct SessionCommentRow {
    pub comment_id: String,
    pub user_id: String,
    pub user_name: String,
    pub message: String,
    pub timestamp_ms: i64,
    pub is_first_time: bool,
    pub is_member: bool,
    pub event_kind: Option<String>,
    pub amount_display: String,
    pub amount_micros: u64,
    pub currency: String,
}

/// コメント処理時に DB から取得するユーザーメタ情報
#[derive(Debug, Clone)]
pub struct UserCommentMeta {
    pub is_banned: bool,
    pub memo: String,
    pub was_unregistered: bool,
    pub already_first_time_detected: bool,
}

/// SQLite 永続化データベース
pub struct Database {
    conn: Connection,
}

impl Database {
    /// ファイルパスを指定してデータベースを開く
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).context("SQLiteデータベースのオープンに失敗しました")?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// テスト用インメモリデータベース
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -16000;
            PRAGMA mmap_size = 67108864;
            PRAGMA busy_timeout = 5000;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stream_sessions (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                stream_id TEXT NOT NULL,
                title TEXT NOT NULL,
                started_at_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_stream_sessions_lookup
                ON stream_sessions(platform, stream_id);
            CREATE INDEX IF NOT EXISTS idx_stream_sessions_started
                ON stream_sessions(started_at_ms DESC);

            CREATE TABLE IF NOT EXISTS users (
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                display_name TEXT NOT NULL,
                icon_url TEXT NOT NULL,
                watch_count INTEGER NOT NULL DEFAULT 0,
                total_comment_count INTEGER NOT NULL DEFAULT 0,
                is_banned INTEGER NOT NULL DEFAULT 0,
                ban_reason TEXT NOT NULL DEFAULT '',
                memo TEXT NOT NULL DEFAULT '',
                first_time_detected INTEGER NOT NULL DEFAULT 0,
                first_seen_ms INTEGER NOT NULL,
                last_seen_ms INTEGER NOT NULL,
                PRIMARY KEY (user_id, platform)
            );

            CREATE TABLE IF NOT EXISTS session_participants (
                session_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                comment_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (session_id, user_id, platform)
            );
            CREATE INDEX IF NOT EXISTS idx_session_participants_user
                ON session_participants(user_id, platform);

            CREATE TABLE IF NOT EXISTS highlight_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                pattern TEXT NOT NULL,
                match_mode TEXT NOT NULL,
                case_sensitive INTEGER NOT NULL DEFAULT 0,
                bg_color TEXT NOT NULL,
                text_color TEXT NOT NULL,
                border_color TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS word_counters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                keywords_json TEXT NOT NULL,
                match_mode TEXT NOT NULL,
                case_sensitive INTEGER NOT NULL DEFAULT 0,
                max_per_account INTEGER,
                exclude_banned INTEGER NOT NULL DEFAULT 1,
                accent_color TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                total_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS word_counter_user_counts (
                counter_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (counter_id, user_id),
                FOREIGN KEY (counter_id) REFERENCES word_counters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS lottery_history (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                winners_json TEXT NOT NULL,
                candidate_count INTEGER NOT NULL,
                drawn_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS monetization_events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                kind TEXT NOT NULL,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                user_icon_url TEXT NOT NULL,
                amount_display TEXT NOT NULL,
                amount_micros INTEGER NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT '',
                tier_color TEXT NOT NULL DEFAULT '#FFB300',
                message TEXT NOT NULL DEFAULT '',
                timestamp_ms INTEGER NOT NULL,
                is_checked INTEGER NOT NULL DEFAULT 0,
                checked_at_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_monetization_ts
                ON monetization_events(timestamp_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_users_last_seen
                ON users(last_seen_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_users_banned
                ON users(is_banned) WHERE is_banned = 1;
            CREATE INDEX IF NOT EXISTS idx_lottery_history_drawn
                ON lottery_history(drawn_at_ms DESC);

            CREATE TABLE IF NOT EXISTS media_reaction_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                pattern TEXT NOT NULL,
                match_mode TEXT NOT NULL,
                case_sensitive INTEGER NOT NULL DEFAULT 0,
                media_kind TEXT NOT NULL,
                stored_file_name TEXT NOT NULL,
                original_file_name TEXT NOT NULL,
                animation TEXT NOT NULL DEFAULT 'pop_in',
                position TEXT NOT NULL DEFAULT 'center',
                duration_ms INTEGER NOT NULL DEFAULT 4000,
                width_percent INTEGER NOT NULL DEFAULT 45,
                volume INTEGER NOT NULL DEFAULT 80,
                cooldown_sec INTEGER NOT NULL DEFAULT 3,
                exclude_banned INTEGER NOT NULL DEFAULT 1,
                caption_template TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS connection_history (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                stream_id TEXT NOT NULL,
                input_url TEXT NOT NULL,
                stream_title TEXT NOT NULL,
                mode TEXT NOT NULL,
                last_connected_ms INTEGER NOT NULL,
                is_favorite INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_conn_history_ts
                ON connection_history(is_favorite DESC, last_connected_ms DESC);

            CREATE TABLE IF NOT EXISTS ng_word_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                match_mode TEXT NOT NULL,
                case_sensitive INTEGER NOT NULL DEFAULT 0,
                action TEXT NOT NULL,
                replacement TEXT NOT NULL DEFAULT '***',
                enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS stream_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                payload_json TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_stream_profiles_updated
                ON stream_profiles(updated_at_ms DESC);

            CREATE TABLE IF NOT EXISTS session_comments (
                session_id TEXT NOT NULL,
                comment_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                is_first_time INTEGER NOT NULL DEFAULT 0,
                is_member INTEGER NOT NULL DEFAULT 0,
                event_kind TEXT,
                amount_display TEXT NOT NULL DEFAULT '',
                amount_micros INTEGER NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (session_id, comment_id)
            );
            CREATE INDEX IF NOT EXISTS idx_session_comments_session_ts
                ON session_comments(session_id, timestamp_ms ASC);
            ",
        )?;

        // 既存DB向けの安全マイグレーション（カラム未存在時のみ追加）
        let _ = self.conn.execute(
            "ALTER TABLE users ADD COLUMN first_time_detected INTEGER NOT NULL DEFAULT 0",
            [],
        );
        // カラム追加後に部分インデックスを作成（初見累計カウントの全表走査を回避）
        let _ = self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_first_time
                ON users(first_time_detected) WHERE first_time_detected = 1",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE monetization_events ADD COLUMN message_runs_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        let media_migrations = [
            "ALTER TABLE media_reaction_rules ADD COLUMN effect_type TEXT NOT NULL DEFAULT 'fixed'",
            "ALTER TABLE media_reaction_rules ADD COLUMN trigger_type TEXT NOT NULL DEFAULT 'keyword'",
            "ALTER TABLE media_reaction_rules ADD COLUMN min_superchat_amount INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE media_reaction_rules ADD COLUMN priority INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE media_reaction_rules ADD COLUMN sound_stored_file_name TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE media_reaction_rules ADD COLUMN sound_original_file_name TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE media_reaction_rules ADD COLUMN sound_volume INTEGER NOT NULL DEFAULT 80",
            "ALTER TABLE media_reaction_rules ADD COLUMN particle_count INTEGER NOT NULL DEFAULT 24",
            "ALTER TABLE media_reaction_rules ADD COLUMN particle_size INTEGER NOT NULL DEFAULT 64",
            "ALTER TABLE media_reaction_rules ADD COLUMN particle_speed INTEGER NOT NULL DEFAULT 100",
            "ALTER TABLE media_reaction_rules ADD COLUMN gravity INTEGER NOT NULL DEFAULT 100",
            "ALTER TABLE media_reaction_rules ADD COLUMN particle_text TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE media_reaction_rules ADD COLUMN ambient_preset TEXT NOT NULL DEFAULT 'snow'",
        ];
        for sql in media_migrations {
            let _ = self.conn.execute(sql, []);
        }

        self.seed_defaults_if_empty()?;
        Ok(())
    }

    fn seed_defaults_if_empty(&self) -> Result<()> {
        let seeded: Option<String> = self.get_setting("defaults_seeded")?;
        if seeded.is_some() {
            return Ok(());
        }

        // 初回起動時のサンプルルールを1件ずつ用意（ユーザーがすぐ試して編集・削除可能）
        let default_rules = vec![
            HighlightRule {
                id: "rule-first-time".to_string(),
                name: "初見さん歓迎".to_string(),
                pattern: "初見".to_string(),
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                bg_color: "#065F46E6".to_string(),
                text_color: "#ECFDF5".to_string(),
                border_color: "#34D399".to_string(),
                priority: 10,
                enabled: true,
            },
            HighlightRule {
                id: "rule-question".to_string(),
                name: "質問コメント".to_string(),
                pattern: "質問".to_string(),
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                bg_color: "#7C2D12E6".to_string(),
                text_color: "#FFF7ED".to_string(),
                border_color: "#FB923C".to_string(),
                priority: 5,
                enabled: true,
            },
        ];
        for r in default_rules {
            self.upsert_highlight_rule(&r)?;
        }

        let default_counters = vec![
            WordCounterConfig {
                id: "counter-kusa".to_string(),
                name: "草カウンター（上限なし）".to_string(),
                keywords: vec!["草".to_string(), "www".to_string()],
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                max_per_account: None,
                exclude_banned: true,
                accent_color: "#22C55E".to_string(),
                enabled: true,
                total_count: 0,
                unique_user_count: 0,
            },
            WordCounterConfig {
                id: "counter-join".to_string(),
                name: "参加表明（1人1回まで）".to_string(),
                keywords: vec!["参加".to_string(), "挙手".to_string()],
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                max_per_account: Some(1),
                exclude_banned: true,
                accent_color: "#38BDF8".to_string(),
                enabled: true,
                total_count: 0,
                unique_user_count: 0,
            },
        ];
        for c in default_counters {
            self.upsert_word_counter(&c)?;
        }

        self.set_setting("defaults_seeded", "1")?;
        Ok(())
    }

    /// 複数クエリを単一 SAVEPOINT トランザクションで実行する（ネスト対応・WAL 同期コスト集約）
    pub fn in_savepoint<T, F>(&self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce(&Self) -> Result<T>,
    {
        self.conn.execute_batch(&format!("SAVEPOINT {name};"))?;
        match f(self) {
            Ok(val) => {
                if let Err(e) = self
                    .conn
                    .execute_batch(&format!("RELEASE SAVEPOINT {name};"))
                {
                    let _ = self.conn.execute_batch(&format!(
                        "ROLLBACK TO SAVEPOINT {name}; RELEASE SAVEPOINT {name};"
                    ));
                    return Err(e.into());
                }
                Ok(val)
            }
            Err(e) => {
                let _ = self.conn.execute_batch(&format!(
                    "ROLLBACK TO SAVEPOINT {name}; RELEASE SAVEPOINT {name};"
                ));
                Err(e)
            }
        }
    }

    // ==================== 設定 (Key-Value) ====================

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM app_settings WHERE key = ?1")?;
        let res = stmt.query_row(params![key], |row| row.get(0)).optional()?;
        Ok(res)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )?;
        stmt.execute(params![key, value])?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_overlay_config(&self) -> Result<OverlayConfig> {
        if let Some(json) = self.get_setting("overlay_config")? {
            if let Ok(cfg) = serde_json::from_str(&json) {
                return Ok(cfg);
            }
        }
        Ok(OverlayConfig::default())
    }

    pub fn set_overlay_config(&self, cfg: &OverlayConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("overlay_config", &json)
    }

    pub fn get_feature_flags(&self) -> Result<FeatureFlags> {
        if let Some(json) = self.get_setting("feature_flags")? {
            if let Ok(flags) = serde_json::from_str(&json) {
                return Ok(flags);
            }
        }
        Ok(FeatureFlags::default())
    }

    pub fn set_feature_flags(&self, flags: &FeatureFlags) -> Result<()> {
        let json = serde_json::to_string(flags)?;
        self.set_setting("feature_flags", &json)
    }

    pub fn get_theme_mode(&self) -> Result<AppThemeMode> {
        if let Some(val) = self.get_setting("theme_mode")? {
            return Ok(AppThemeMode::from_str_lossy(&val));
        }
        Ok(AppThemeMode::Dark)
    }

    pub fn set_theme_mode(&self, mode: AppThemeMode) -> Result<()> {
        self.set_setting("theme_mode", mode.as_str())
    }

    pub fn get_first_time_config(&self) -> Result<FirstTimeCounterConfig> {
        if let Some(json) = self.get_setting("first_time_config")? {
            if let Ok(cfg) = serde_json::from_str(&json) {
                return Ok(cfg);
            }
        }
        Ok(FirstTimeCounterConfig::default())
    }

    pub fn set_first_time_config(&self, cfg: &FirstTimeCounterConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("first_time_config", &json)
    }

    pub fn get_poll_config(&self) -> Result<PollConfig> {
        if let Some(json) = self.get_setting("poll_config")? {
            if let Ok(cfg) = serde_json::from_str(&json) {
                return Ok(cfg);
            }
        }
        Ok(PollConfig::default())
    }

    pub fn set_poll_config(&self, cfg: &PollConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("poll_config", &json)
    }

    pub fn get_queue_state(&self) -> Result<QueueState> {
        if let Some(json) = self.get_setting("queue_state")? {
            if let Ok(st) = serde_json::from_str(&json) {
                return Ok(st);
            }
        }
        Ok(QueueState::default())
    }

    pub fn set_queue_state(&self, st: &QueueState) -> Result<()> {
        let json = serde_json::to_string(st)?;
        self.set_setting("queue_state", &json)
    }

    pub fn get_media_reaction_config(&self) -> Result<MediaReactionConfig> {
        if let Some(json) = self.get_setting("media_reaction_config")? {
            if let Ok(cfg) = serde_json::from_str(&json) {
                return Ok(cfg);
            }
        }
        Ok(MediaReactionConfig::default())
    }

    pub fn set_media_reaction_config(&self, cfg: &MediaReactionConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("media_reaction_config", &json)
    }

    pub fn get_remote_access_config(&self) -> Result<RemoteAccessConfig> {
        if let Some(json) = self.get_setting("remote_access_config")? {
            if let Ok(cfg) = serde_json::from_str(&json) {
                return Ok(cfg);
            }
        }
        // 初回はランダム 6 桁 PIN を自動生成して保存（LAN 接続自体はデフォルト OFF で安全）
        let cfg = RemoteAccessConfig {
            pin_code: crate::remote::generate_pin_code(),
            ..RemoteAccessConfig::default()
        };
        let _ = self.set_remote_access_config(&cfg);
        Ok(cfg)
    }

    pub fn set_remote_access_config(&self, cfg: &RemoteAccessConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("remote_access_config", &json)
    }

    // ==================== 配信セッション ====================

    /// 同一プラットフォーム・同一 stream_id の既存セッションがあれば再利用し、
    /// 無ければ新規セッションを作成する（再接続時の視聴回数二重加算を防止）
    pub fn get_or_create_session(
        &self,
        platform: PlatformKind,
        stream_id: &str,
        title: &str,
        now_ms: i64,
    ) -> Result<String> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM stream_sessions WHERE platform = ?1 AND stream_id = ?2 ORDER BY started_at_ms DESC LIMIT 1",
                params![platform.as_str(), stream_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(session_id) = existing {
            if !title.is_empty() {
                self.conn.execute(
                    "UPDATE stream_sessions SET title = ?1 WHERE id = ?2",
                    params![title, session_id],
                )?;
            }
            return Ok(session_id);
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO stream_sessions (id, platform, stream_id, title, started_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, platform.as_str(), stream_id, title, now_ms],
        )?;
        Ok(session_id)
    }

    // ==================== ユーザー & 視聴回数・コメント回数 ====================

    /// コメント受信時にユーザーを記録・更新する。
    /// 1配信セッションにつき初回コメント時のみ watch_count を +1 する。
    /// 戻り値: BAN状態・名前メモ・未登録だったか・すでに初見検知済みか
    pub fn record_user_comment(
        &self,
        session_id: &str,
        raw: &RawIncomingComment,
    ) -> Result<UserCommentMeta> {
        // 単独呼び出しでも原子性を保証するため SAVEPOINT でラップする
        self.in_savepoint("record_user_comment", |db| {
            db.record_user_comment_inner(session_id, raw)
        })
    }

    /// 既存トランザクション（外側の SAVEPOINT 内）から直接呼び出すための内部実装（二重 SAVEPOINT 回避）
    pub fn record_user_comment_inner(
        &self,
        session_id: &str,
        raw: &RawIncomingComment,
    ) -> Result<UserCommentMeta> {
        let platform_str = raw.platform.as_str();

        let existing: Option<(i64, String, i64)> = {
            let mut stmt = self.conn.prepare_cached(
                "SELECT is_banned, memo, first_time_detected FROM users WHERE user_id = ?1 AND platform = ?2",
            )?;
            stmt.query_row(params![raw.user_id, platform_str], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()?
        };

        let (was_unregistered, is_banned, memo, already_first_time_detected) = match existing {
            Some((b, memo, det)) => (false, b != 0, memo, det != 0),
            None => (true, false, String::new(), false),
        };

        // ユーザーテーブルへ UPSERT（is_banned / memo / first_time_detected は変更しない）
        {
            let mut stmt = self.conn.prepare_cached(
                "INSERT INTO users (
                    user_id, platform, display_name, icon_url,
                    watch_count, total_comment_count, is_banned, ban_reason, memo,
                    first_time_detected, first_seen_ms, last_seen_ms
                ) VALUES (?1, ?2, ?3, ?4, 0, 1, 0, '', '', 0, ?5, ?5)
                ON CONFLICT(user_id, platform) DO UPDATE SET
                    display_name = excluded.display_name,
                    icon_url = CASE WHEN excluded.icon_url != '' THEN excluded.icon_url ELSE users.icon_url END,
                    total_comment_count = users.total_comment_count + 1,
                    last_seen_ms = excluded.last_seen_ms",
            )?;
            stmt.execute(params![
                raw.user_id,
                platform_str,
                raw.user_name,
                raw.user_icon_url,
                raw.timestamp_ms
            ])?;
        }

        // この配信セッションでの参加が初回かを INSERT OR IGNORE で 1 クエリ判定
        let inserted = {
            let mut stmt = self.conn.prepare_cached(
                "INSERT OR IGNORE INTO session_participants (session_id, user_id, platform, comment_count)
                 VALUES (?1, ?2, ?3, 1)",
            )?;
            stmt.execute(params![session_id, raw.user_id, platform_str])?
        };

        if inserted > 0 {
            // 1配信につき初回コメントで視聴回数 +1
            let mut stmt = self.conn.prepare_cached(
                "UPDATE users SET watch_count = watch_count + 1
                 WHERE user_id = ?1 AND platform = ?2",
            )?;
            stmt.execute(params![raw.user_id, platform_str])?;
        } else {
            let mut stmt = self.conn.prepare_cached(
                "UPDATE session_participants SET comment_count = comment_count + 1
                 WHERE session_id = ?1 AND user_id = ?2 AND platform = ?3",
            )?;
            stmt.execute(params![session_id, raw.user_id, platform_str])?;
        }

        Ok(UserCommentMeta {
            is_banned,
            memo,
            was_unregistered,
            already_first_time_detected,
        })
    }

    /// ユーザー自動記録OFF時に既存のユーザー情報だけを参照する
    pub fn get_user_comment_meta(
        &self,
        user_id: &str,
        platform: PlatformKind,
    ) -> Result<UserCommentMeta> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT is_banned, memo, first_time_detected FROM users WHERE user_id = ?1 AND platform = ?2",
        )?;
        let row: Option<(i64, String, i64)> = stmt
            .query_row(params![user_id, platform.as_str()], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .optional()?;
        match row {
            Some((b, memo, det)) => Ok(UserCommentMeta {
                is_banned: b != 0,
                memo,
                was_unregistered: false,
                already_first_time_detected: det != 0,
            }),
            None => Ok(UserCommentMeta {
                is_banned: false,
                memo: String::new(),
                was_unregistered: true,
                already_first_time_detected: false,
            }),
        }
    }

    /// ユーザーを「初見検知済み」として永続マークする（未登録の場合は軽量レコードを作成）
    pub fn mark_user_first_time_detected(
        &self,
        raw: &RawIncomingComment,
    ) -> Result<u64> {
        let platform_str = raw.platform.as_str();
        {
            let mut stmt = self.conn.prepare_cached(
                "INSERT INTO users (
                    user_id, platform, display_name, icon_url,
                    watch_count, total_comment_count, is_banned, ban_reason, memo,
                    first_time_detected, first_seen_ms, last_seen_ms
                ) VALUES (?1, ?2, ?3, ?4, 0, 0, 0, '', '', 1, ?5, ?5)
                ON CONFLICT(user_id, platform) DO UPDATE SET
                    first_time_detected = 1",
            )?;
            stmt.execute(params![
                raw.user_id,
                platform_str,
                raw.user_name,
                raw.user_icon_url,
                raw.timestamp_ms
            ])?;
        }
        self.count_all_time_first_timers()
    }

    pub fn count_all_time_first_timers(&self) -> Result<u64> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT COUNT(*) FROM users WHERE first_time_detected = 1")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count.max(0) as u64)
    }

    pub fn count_users(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    pub fn reset_all_first_time_history(&self) -> Result<()> {
        self.conn
            .execute("UPDATE users SET first_time_detected = 0", [])?;
        Ok(())
    }

    pub fn is_user_banned(&self, user_id: &str) -> Result<bool> {
        let banned: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(is_banned) FROM users WHERE user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(banned.unwrap_or(0) != 0)
    }

    pub fn get_banned_user_ids(&self) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT user_id FROM users WHERE is_banned = 1")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut set = HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    pub fn set_user_ban(
        &self,
        user_id: &str,
        platform: PlatformKind,
        banned: bool,
        reason: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET is_banned = ?1, ban_reason = ?2
             WHERE user_id = ?3 AND platform = ?4",
            params![
                if banned { 1 } else { 0 },
                if banned { reason } else { "" },
                user_id,
                platform.as_str()
            ],
        )?;
        Ok(())
    }

    pub fn set_user_memo(
        &self,
        user_id: &str,
        platform: PlatformKind,
        memo: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET memo = ?1 WHERE user_id = ?2 AND platform = ?3",
            params![memo, user_id, platform.as_str()],
        )?;
        Ok(())
    }

    /// 特定のユーザーをユーザーリスト（および配信参加記録）から個別削除する
    pub fn delete_user(&self, user_id: &str, platform: PlatformKind) -> Result<()> {
        let p = platform.as_str();
        self.in_savepoint("delete_user", |db| {
            db.conn.execute(
                "DELETE FROM session_participants WHERE user_id = ?1 AND platform = ?2",
                params![user_id, p],
            )?;
            db.conn.execute(
                "DELETE FROM users WHERE user_id = ?1 AND platform = ?2",
                params![user_id, p],
            )?;
            Ok(())
        })
    }

    /// ユーザーリストを一括リセットする（keep_banned = true の場合は BAN 済みユーザーを残す）
    pub fn reset_users(&self, keep_banned: bool) -> Result<()> {
        self.in_savepoint("reset_users", |db| {
            if keep_banned {
                db.conn.execute(
                    "DELETE FROM session_participants
                     WHERE EXISTS (
                         SELECT 1 FROM users u
                         WHERE u.user_id = session_participants.user_id
                           AND u.platform = session_participants.platform
                           AND u.is_banned = 0
                     )",
                    [],
                )?;
                db.conn.execute("DELETE FROM users WHERE is_banned = 0", [])?;
            } else {
                db.conn.execute("DELETE FROM session_participants", [])?;
                db.conn.execute("DELETE FROM users", [])?;
            }
            Ok(())
        })
    }

    pub fn list_users(&self, current_session_id: Option<&str>) -> Result<Vec<UserRecord>> {
        let session_id = current_session_id.unwrap_or("");
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                u.user_id,
                u.platform,
                u.display_name,
                u.icon_url,
                u.watch_count,
                u.total_comment_count,
                COALESCE(sp.comment_count, 0) AS session_comment_count,
                u.is_banned,
                u.ban_reason,
                u.memo,
                u.first_seen_ms,
                u.last_seen_ms,
                CASE WHEN sp.user_id IS NOT NULL THEN 1 ELSE 0 END AS in_current_session
            FROM users u
            LEFT JOIN session_participants sp
                ON sp.user_id = u.user_id
                AND sp.platform = u.platform
                AND sp.session_id = ?1
            ORDER BY in_current_session DESC, u.last_seen_ms DESC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let platform_str: String = row.get(1)?;
            Ok(UserRecord {
                user_id: row.get(0)?,
                platform: PlatformKind::from_str_lossy(&platform_str),
                display_name: row.get(2)?,
                icon_url: row.get(3)?,
                watch_count: row.get::<_, i64>(4)? as u64,
                total_comment_count: row.get::<_, i64>(5)? as u64,
                session_comment_count: row.get::<_, i64>(6)? as u64,
                is_banned: row.get::<_, i64>(7)? != 0,
                ban_reason: row.get(8)?,
                memo: row.get(9)?,
                first_seen_ms: row.get(10)?,
                last_seen_ms: row.get(11)?,
                in_current_session: row.get::<_, i64>(12)? != 0,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ==================== 特定ワード反応ルール ====================

    pub fn list_highlight_rules(&self) -> Result<Vec<HighlightRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, pattern, match_mode, case_sensitive, bg_color, text_color, border_color, priority, enabled
             FROM highlight_rules
             ORDER BY priority DESC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let mode_str: String = row.get(3)?;
            Ok(HighlightRule {
                id: row.get(0)?,
                name: row.get(1)?,
                pattern: row.get(2)?,
                match_mode: MatchMode::from_str_lossy(&mode_str),
                case_sensitive: row.get::<_, i64>(4)? != 0,
                bg_color: row.get(5)?,
                text_color: row.get(6)?,
                border_color: row.get(7)?,
                priority: row.get(8)?,
                enabled: row.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_highlight_rule(&self, rule: &HighlightRule) -> Result<()> {
        self.conn.execute(
            "INSERT INTO highlight_rules (
                id, name, pattern, match_mode, case_sensitive,
                bg_color, text_color, border_color, priority, enabled
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                pattern = excluded.pattern,
                match_mode = excluded.match_mode,
                case_sensitive = excluded.case_sensitive,
                bg_color = excluded.bg_color,
                text_color = excluded.text_color,
                border_color = excluded.border_color,
                priority = excluded.priority,
                enabled = excluded.enabled",
            params![
                rule.id,
                rule.name,
                rule.pattern,
                rule.match_mode.as_str(),
                if rule.case_sensitive { 1 } else { 0 },
                rule.bg_color,
                rule.text_color,
                rule.border_color,
                rule.priority,
                if rule.enabled { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn delete_highlight_rule(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM highlight_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ==================== 特定ワードカウンター ====================

    pub fn list_word_counters(&self) -> Result<Vec<WordCounterConfig>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                wc.id,
                wc.name,
                wc.keywords_json,
                wc.match_mode,
                wc.case_sensitive,
                wc.max_per_account,
                wc.exclude_banned,
                wc.accent_color,
                wc.enabled,
                wc.total_count,
                (SELECT COUNT(*) FROM word_counter_user_counts uc WHERE uc.counter_id = wc.id AND uc.count > 0) AS unique_users
            FROM word_counters wc
            ORDER BY wc.name ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let keywords_json: String = row.get(2)?;
            let keywords: Vec<String> = serde_json::from_str(&keywords_json).unwrap_or_default();
            let mode_str: String = row.get(3)?;
            let max_per_account: Option<i64> = row.get(5)?;
            Ok(WordCounterConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                keywords,
                match_mode: MatchMode::from_str_lossy(&mode_str),
                case_sensitive: row.get::<_, i64>(4)? != 0,
                max_per_account: max_per_account.map(|v| v as u32),
                exclude_banned: row.get::<_, i64>(6)? != 0,
                accent_color: row.get(7)?,
                enabled: row.get::<_, i64>(8)? != 0,
                total_count: row.get::<_, i64>(9)? as u64,
                unique_user_count: row.get::<_, i64>(10)? as u64,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_word_counter(&self, counter: &WordCounterConfig) -> Result<()> {
        let keywords_json = serde_json::to_string(&counter.keywords)?;
        self.conn.execute(
            "INSERT INTO word_counters (
                id, name, keywords_json, match_mode, case_sensitive,
                max_per_account, exclude_banned, accent_color, enabled, total_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                keywords_json = excluded.keywords_json,
                match_mode = excluded.match_mode,
                case_sensitive = excluded.case_sensitive,
                max_per_account = excluded.max_per_account,
                exclude_banned = excluded.exclude_banned,
                accent_color = excluded.accent_color,
                enabled = excluded.enabled",
            params![
                counter.id,
                counter.name,
                keywords_json,
                counter.match_mode.as_str(),
                if counter.case_sensitive { 1 } else { 0 },
                counter.max_per_account.map(|v| v as i64),
                if counter.exclude_banned { 1 } else { 0 },
                counter.accent_color,
                if counter.enabled { 1 } else { 0 },
                counter.total_count as i64,
            ],
        )?;
        Ok(())
    }

    pub fn delete_word_counter(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM word_counters WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn reset_word_counter(&self, id: &str) -> Result<()> {
        self.in_savepoint("reset_word_counter", |db| {
            db.conn.execute(
                "DELETE FROM word_counter_user_counts WHERE counter_id = ?1",
                params![id],
            )?;
            db.conn.execute(
                "UPDATE word_counters SET total_count = 0 WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })
    }

    pub fn adjust_word_counter(&self, id: &str, delta: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE word_counters SET total_count = MAX(0, total_count + ?1) WHERE id = ?2",
            params![delta, id],
        )?;
        Ok(())
    }

    pub fn get_counter_user_counts(&self, counter_id: &str) -> Result<HashMap<String, u32>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT user_id, count FROM word_counter_user_counts WHERE counter_id = ?1",
        )?;
        let rows = stmt.query_map(params![counter_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
        })?;
        let mut map = HashMap::new();
        for r in rows {
            let (uid, cnt) = r?;
            map.insert(uid, cnt);
        }
        Ok(map)
    }

    pub fn increment_counter_for_user(
        &self,
        counter_id: &str,
        user_id: &str,
        max_per_account: Option<u32>,
    ) -> Result<bool> {
        // 上限ありの場合も条件付き UPSERT (WHERE count < ?3) で事前 SELECT を不要化し、
        // 上限到達時は 1 クエリ・未到達時は 2 クエリで原子的に更新する
        let changed = match max_per_account {
            Some(0) => return Ok(false),
            Some(limit) => {
                let mut stmt = self.conn.prepare_cached(
                    "INSERT INTO word_counter_user_counts (counter_id, user_id, count)
                     VALUES (?1, ?2, 1)
                     ON CONFLICT(counter_id, user_id) DO UPDATE SET count = count + 1
                     WHERE word_counter_user_counts.count < ?3",
                )?;
                stmt.execute(params![counter_id, user_id, limit as i64])?
            }
            None => {
                let mut stmt = self.conn.prepare_cached(
                    "INSERT INTO word_counter_user_counts (counter_id, user_id, count)
                     VALUES (?1, ?2, 1)
                     ON CONFLICT(counter_id, user_id) DO UPDATE SET count = count + 1",
                )?;
                stmt.execute(params![counter_id, user_id])?
            }
        };
        if changed == 0 {
            return Ok(false);
        }
        {
            let mut stmt = self.conn.prepare_cached(
                "UPDATE word_counters SET total_count = total_count + 1 WHERE id = ?1",
            )?;
            stmt.execute(params![counter_id])?;
        }
        Ok(true)
    }

    // ==================== 抽選履歴 ====================

    pub fn save_lottery_result(&self, result: &LotteryResult) -> Result<()> {
        let winners_json = serde_json::to_string(&result.winners)?;
        self.conn.execute(
            "INSERT INTO lottery_history (id, kind, winners_json, candidate_count, drawn_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                result.id,
                result.kind,
                winners_json,
                result.candidate_count as i64,
                result.drawn_at_ms
            ],
        )?;
        Ok(())
    }

    pub fn list_lottery_history(&self, limit: usize) -> Result<Vec<LotteryResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, winners_json, candidate_count, drawn_at_ms
             FROM lottery_history
             ORDER BY drawn_at_ms DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let winners_json: String = row.get(2)?;
            let winners: Vec<LotteryWinner> =
                serde_json::from_str(&winners_json).unwrap_or_default();
            Ok(LotteryResult {
                id: row.get(0)?,
                kind: row.get(1)?,
                winners,
                candidate_count: row.get::<_, i64>(3)? as u32,
                drawn_at_ms: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ==================== スーパーチャット / メンバーシップ ====================

    pub fn insert_monetization_event(&self, item: &MonetizationItem) -> Result<()> {
        let runs_json = if item.message_runs.is_empty() {
            "[]".to_string()
        } else {
            serde_json::to_string(&item.message_runs).unwrap_or_else(|_| "[]".to_string())
        };
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO monetization_events (
                id, session_id, platform, kind, user_id, user_name, user_icon_url,
                amount_display, amount_micros, currency, tier_color, message,
                message_runs_json, timestamp_ms, is_checked, checked_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        )?;
        stmt.execute(params![
            item.id,
            item.session_id,
            item.platform.as_str(),
            item.kind.as_str(),
            item.user_id,
            item.user_name,
            item.user_icon_url,
            item.amount_display,
            item.amount_micros as i64,
            item.currency,
            item.tier_color,
            item.message,
            runs_json,
            item.timestamp_ms,
            if item.is_checked { 1 } else { 0 },
            item.checked_at_ms,
        ])?;
        Ok(())
    }

    pub fn list_monetization_events(&self, limit: usize) -> Result<Vec<MonetizationItem>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                id, session_id, platform, kind, user_id, user_name, user_icon_url,
                amount_display, amount_micros, currency, tier_color, message,
                message_runs_json, timestamp_ms, is_checked, checked_at_ms
             FROM monetization_events
             ORDER BY timestamp_ms DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let platform_str: String = row.get(2)?;
            let kind_str: String = row.get(3)?;
            let runs_json: String = row.get(12)?;
            let message_runs: Vec<MessageRun> =
                serde_json::from_str(&runs_json).unwrap_or_default();
            Ok(MonetizationItem {
                id: row.get(0)?,
                session_id: row.get(1)?,
                platform: PlatformKind::from_str_lossy(&platform_str),
                kind: MonetizationEventKind::from_str_lossy(&kind_str),
                user_id: row.get(4)?,
                user_name: row.get(5)?,
                user_icon_url: row.get(6)?,
                amount_display: row.get(7)?,
                amount_micros: row.get::<_, i64>(8)?.max(0) as u64,
                currency: row.get(9)?,
                tier_color: row.get(10)?,
                message: row.get(11)?,
                message_runs,
                timestamp_ms: row.get(13)?,
                is_checked: row.get::<_, i64>(14)? != 0,
                checked_at_ms: row.get(15)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn set_monetization_checked(
        &self,
        id: &str,
        checked: bool,
        now_ms: i64,
    ) -> Result<()> {
        let checked_at = if checked { Some(now_ms) } else { None };
        self.conn.execute(
            "UPDATE monetization_events SET is_checked = ?1, checked_at_ms = ?2 WHERE id = ?3",
            params![if checked { 1 } else { 0 }, checked_at, id],
        )?;
        Ok(())
    }

    pub fn mark_all_monetization_checked(&self, now_ms: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE monetization_events SET is_checked = 1, checked_at_ms = COALESCE(checked_at_ms, ?1) WHERE is_checked = 0",
            params![now_ms],
        )?;
        Ok(())
    }

    pub fn delete_monetization_event(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM monetization_events WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_monetization_events(&self, checked_only: bool) -> Result<()> {
        if checked_only {
            self.conn
                .execute("DELETE FROM monetization_events WHERE is_checked = 1", [])?;
        } else {
            self.conn.execute("DELETE FROM monetization_events", [])?;
        }
        Ok(())
    }

    // ==================== 特定ワード反応演出 (WordParty) ====================

    pub fn list_media_reaction_rules(&self) -> Result<Vec<MediaReactionRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, name, pattern, match_mode, case_sensitive, media_kind,
                stored_file_name, original_file_name, animation, position,
                duration_ms, width_percent, volume, cooldown_sec,
                exclude_banned, caption_template, enabled,
                effect_type, trigger_type, min_superchat_amount, priority,
                sound_stored_file_name, sound_original_file_name, sound_volume,
                particle_count, particle_size, particle_speed, gravity,
                particle_text, ambient_preset
             FROM media_reaction_rules
             ORDER BY priority DESC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let mode_str: String = row.get(3)?;
            let kind_str: String = row.get(5)?;
            let anim_str: String = row.get(8)?;
            let pos_str: String = row.get(9)?;
            let effect_str: String = row.get(17)?;
            let trig_str: String = row.get(18)?;
            Ok(MediaReactionRule {
                id: row.get(0)?,
                name: row.get(1)?,
                pattern: row.get(2)?,
                match_mode: MatchMode::from_str_lossy(&mode_str),
                case_sensitive: row.get::<_, i64>(4)? != 0,
                media_kind: MediaKind::from_str_lossy(&kind_str),
                stored_file_name: row.get(6)?,
                original_file_name: row.get(7)?,
                animation: MediaAnimationKind::from_str_lossy(&anim_str),
                position: MediaPosition::from_str_lossy(&pos_str),
                duration_ms: row.get::<_, i64>(10)?.max(0) as u32,
                width_percent: row.get::<_, i64>(11)?.clamp(10, 100) as u32,
                volume: row.get::<_, i64>(12)?.clamp(0, 100) as u32,
                cooldown_sec: row.get::<_, i64>(13)?.max(0) as u32,
                exclude_banned: row.get::<_, i64>(14)? != 0,
                caption_template: row.get(15)?,
                enabled: row.get::<_, i64>(16)? != 0,
                effect_type: MediaEffectType::from_str_lossy(&effect_str),
                trigger_type: MediaTriggerType::from_str_lossy(&trig_str),
                min_superchat_amount: row.get::<_, i64>(19)?.max(0) as u64,
                priority: row.get::<_, i64>(20)? as i32,
                sound_stored_file_name: row.get(21)?,
                sound_original_file_name: row.get(22)?,
                sound_volume: row.get::<_, i64>(23)?.clamp(0, 100) as u32,
                particle_count: row.get::<_, i64>(24)?.clamp(1, 120) as u32,
                particle_size: row.get::<_, i64>(25)?.clamp(20, 260) as u32,
                particle_speed: row.get::<_, i64>(26)?.clamp(30, 250) as u32,
                gravity: row.get::<_, i64>(27)?.clamp(0, 200) as u32,
                particle_text: row.get(28)?,
                ambient_preset: row.get(29)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_media_reaction_rule(&self, rule: &MediaReactionRule) -> Result<()> {
        self.conn.execute(
            "INSERT INTO media_reaction_rules (
                id, name, pattern, match_mode, case_sensitive, media_kind,
                stored_file_name, original_file_name, animation, position,
                duration_ms, width_percent, volume, cooldown_sec,
                exclude_banned, caption_template, enabled,
                effect_type, trigger_type, min_superchat_amount, priority,
                sound_stored_file_name, sound_original_file_name, sound_volume,
                particle_count, particle_size, particle_speed, gravity,
                particle_text, ambient_preset
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24,
                ?25, ?26, ?27, ?28, ?29, ?30
            )
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                pattern = excluded.pattern,
                match_mode = excluded.match_mode,
                case_sensitive = excluded.case_sensitive,
                media_kind = excluded.media_kind,
                stored_file_name = excluded.stored_file_name,
                original_file_name = excluded.original_file_name,
                animation = excluded.animation,
                position = excluded.position,
                duration_ms = excluded.duration_ms,
                width_percent = excluded.width_percent,
                volume = excluded.volume,
                cooldown_sec = excluded.cooldown_sec,
                exclude_banned = excluded.exclude_banned,
                caption_template = excluded.caption_template,
                enabled = excluded.enabled,
                effect_type = excluded.effect_type,
                trigger_type = excluded.trigger_type,
                min_superchat_amount = excluded.min_superchat_amount,
                priority = excluded.priority,
                sound_stored_file_name = excluded.sound_stored_file_name,
                sound_original_file_name = excluded.sound_original_file_name,
                sound_volume = excluded.sound_volume,
                particle_count = excluded.particle_count,
                particle_size = excluded.particle_size,
                particle_speed = excluded.particle_speed,
                gravity = excluded.gravity,
                particle_text = excluded.particle_text,
                ambient_preset = excluded.ambient_preset",
            params![
                rule.id,
                rule.name,
                rule.pattern,
                rule.match_mode.as_str(),
                if rule.case_sensitive { 1 } else { 0 },
                rule.media_kind.as_str(),
                rule.stored_file_name,
                rule.original_file_name,
                rule.animation.as_str(),
                rule.position.as_str(),
                rule.duration_ms as i64,
                rule.width_percent as i64,
                rule.volume as i64,
                rule.cooldown_sec as i64,
                if rule.exclude_banned { 1 } else { 0 },
                rule.caption_template,
                if rule.enabled { 1 } else { 0 },
                rule.effect_type.as_str(),
                rule.trigger_type.as_str(),
                rule.min_superchat_amount as i64,
                rule.priority as i64,
                rule.sound_stored_file_name,
                rule.sound_original_file_name,
                rule.sound_volume as i64,
                rule.particle_count as i64,
                rule.particle_size as i64,
                rule.particle_speed as i64,
                rule.gravity as i64,
                rule.particle_text,
                rule.ambient_preset,
            ],
        )?;
        Ok(())
    }

    pub fn delete_media_reaction_rule(&self, id: &str) -> Result<Vec<String>> {
        let files: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT stored_file_name, sound_stored_file_name FROM media_reaction_rules WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        self.conn
            .execute("DELETE FROM media_reaction_rules WHERE id = ?1", params![id])?;

        let mut removable = Vec::new();
        if let Some((media_f, sound_f)) = files {
            for fname in [media_f, sound_f] {
                if fname.trim().is_empty() {
                    continue;
                }
                let still_used: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM media_reaction_rules
                     WHERE stored_file_name = ?1 OR sound_stored_file_name = ?1",
                    params![fname],
                    |row| row.get(0),
                )?;
                if still_used == 0 {
                    removable.push(fname);
                }
            }
        }
        Ok(removable)
    }

    // ==================== バックアップ / 整合性スナップショット ====================

    /// 整合性の取れた SQLite スナップショットを指定パスへ安全に書き出す
    pub fn vacuum_into(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            let _ = std::fs::remove_file(dest);
        }
        let escaped = dest.to_string_lossy().replace('\'', "''");
        let sql = format!("VACUUM INTO '{}'", escaped);
        self.conn
            .execute(&sql, [])
            .with_context(|| format!("SQLite スナップショット出力に失敗しました: {}", dest.display()))?;
        Ok(())
    }

    /// データベースファイルを安全に差し替えて再オープンする（バックアップ復元用）
    pub fn replace_database_file(&mut self, db_path: &Path, src_db: &Path) -> Result<()> {
        // 既存ファイルハンドルを確実に解放するため一時的にインメモリ DB へ差し替え
        self.conn = Connection::open_in_memory()?;
        for suffix in ["-wal", "-shm"] {
            let aux = Path::new(&format!("{}{}", db_path.to_string_lossy(), suffix)).to_path_buf();
            if aux.exists() {
                let _ = std::fs::remove_file(aux);
            }
        }
        std::fs::copy(src_db, db_path).with_context(|| {
            format!(
                "データベースファイルの差し替えに失敗しました: {}",
                db_path.display()
            )
        })?;
        self.conn = Connection::open(db_path)?;
        self.init_schema()?;
        Ok(())
    }

    // ==================== 接続履歴 ＆ お気に入りチャンネル ====================

    pub fn upsert_connection_history(&self, item: &ConnectionHistoryItem) -> Result<()> {
        let mode_str = match item.mode {
            ConnectionMode::YouTubeDataApi => "youtube_data_api",
            ConnectionMode::Demo => "demo",
            ConnectionMode::YouTubeInnerTube => "youtube_innertube",
        };
        self.conn.execute(
            "INSERT INTO connection_history (
                id, platform, stream_id, input_url, stream_title, mode, last_connected_ms, is_favorite
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                input_url = excluded.input_url,
                stream_title = excluded.stream_title,
                mode = excluded.mode,
                last_connected_ms = excluded.last_connected_ms",
            params![
                item.id,
                item.platform.as_str(),
                item.stream_id,
                item.input_url,
                item.stream_title,
                mode_str,
                item.last_connected_ms,
                if item.is_favorite { 1 } else { 0 },
            ],
        )?;
        // お気に入り以外で古い履歴は直近 25 件までに自動整理
        self.conn.execute(
            "DELETE FROM connection_history
             WHERE is_favorite = 0 AND id NOT IN (
                 SELECT id FROM connection_history
                 WHERE is_favorite = 0
                 ORDER BY last_connected_ms DESC
                 LIMIT 25
             )",
            [],
        )?;
        Ok(())
    }

    pub fn list_connection_history(&self) -> Result<Vec<ConnectionHistoryItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, platform, stream_id, input_url, stream_title, mode, last_connected_ms, is_favorite
             FROM connection_history
             ORDER BY is_favorite DESC, last_connected_ms DESC
             LIMIT 40",
        )?;
        let rows = stmt.query_map([], |row| {
            let plat: String = row.get(1)?;
            let mode_s: String = row.get(5)?;
            let mode = match mode_s.as_str() {
                "youtube_data_api" => ConnectionMode::YouTubeDataApi,
                "demo" => ConnectionMode::Demo,
                _ => ConnectionMode::YouTubeInnerTube,
            };
            Ok(ConnectionHistoryItem {
                id: row.get(0)?,
                platform: PlatformKind::from_str_lossy(&plat),
                stream_id: row.get(2)?,
                input_url: row.get(3)?,
                stream_title: row.get(4)?,
                mode,
                last_connected_ms: row.get(6)?,
                is_favorite: row.get::<_, i64>(7)? != 0,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn toggle_connection_history_favorite(&self, id: &str, fav: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE connection_history SET is_favorite = ?1 WHERE id = ?2",
            params![if fav { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    pub fn delete_connection_history(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM connection_history WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_saved_channels(&self) -> Result<Vec<SavedChannelItem>> {
        if let Some(raw) = self.get_setting("saved_channels")? {
            if let Ok(list) = serde_json::from_str::<Vec<SavedChannelItem>>(&raw) {
                return Ok(list);
            }
        }
        Ok(Vec::new())
    }

    pub fn set_saved_channels(&self, channels: &[SavedChannelItem]) -> Result<()> {
        let json = serde_json::to_string(channels)?;
        self.set_setting("saved_channels", &json)
    }

    // ==================== NG ワード防御ルール ====================

    pub fn list_ng_word_rules(&self) -> Result<Vec<NgWordRule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pattern, match_mode, case_sensitive, action, replacement, enabled
             FROM ng_word_rules
             ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let mode_s: String = row.get(2)?;
            let action_s: String = row.get(4)?;
            Ok(NgWordRule {
                id: row.get(0)?,
                pattern: row.get(1)?,
                match_mode: MatchMode::from_str_lossy(&mode_s),
                case_sensitive: row.get::<_, i64>(3)? != 0,
                action: NgWordAction::from_str_lossy(&action_s),
                replacement: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_ng_word_rule(&self, rule: &NgWordRule) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ng_word_rules (
                id, pattern, match_mode, case_sensitive, action, replacement, enabled
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                pattern = excluded.pattern,
                match_mode = excluded.match_mode,
                case_sensitive = excluded.case_sensitive,
                action = excluded.action,
                replacement = excluded.replacement,
                enabled = excluded.enabled",
            params![
                rule.id,
                rule.pattern,
                rule.match_mode.as_str(),
                if rule.case_sensitive { 1 } else { 0 },
                rule.action.as_str(),
                rule.replacement,
                if rule.enabled { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn delete_ng_word_rule(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM ng_word_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ==================== OBS WebSocket ＆ ホットキー設定 ====================

    pub fn get_obs_ws_config(&self) -> Result<ObsWsConfig> {
        if let Some(raw) = self.get_setting("obs_ws_config")? {
            if let Ok(cfg) = serde_json::from_str::<ObsWsConfig>(&raw) {
                return Ok(cfg);
            }
        }
        Ok(ObsWsConfig::default())
    }

    pub fn set_obs_ws_config(&self, cfg: &ObsWsConfig) -> Result<()> {
        let json = serde_json::to_string(cfg)?;
        self.set_setting("obs_ws_config", &json)
    }

    pub fn get_hotkey_bindings(&self) -> Result<Vec<HotkeyBinding>> {
        if let Some(raw) = self.get_setting("hotkey_bindings")? {
            if let Ok(list) = serde_json::from_str::<Vec<HotkeyBinding>>(&raw) {
                return Ok(list);
            }
        }
        Ok(Vec::new())
    }

    pub fn set_hotkey_bindings(&self, bindings: &[HotkeyBinding]) -> Result<()> {
        let json = serde_json::to_string(bindings)?;
        self.set_setting("hotkey_bindings", &json)
    }

    // ==================== 配信プロファイル ====================

    pub fn list_stream_profiles(&self) -> Result<Vec<StreamProfile>> {
        let mut stmt = self.conn.prepare(
            "SELECT payload_json FROM stream_profiles ORDER BY updated_at_ms DESC",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            if let Ok(p) = serde_json::from_str::<StreamProfile>(&r?) {
                list.push(p);
            }
        }
        Ok(list)
    }

    pub fn get_stream_profile(&self, id: &str) -> Result<Option<StreamProfile>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT payload_json FROM stream_profiles WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw.and_then(|s| serde_json::from_str::<StreamProfile>(&s).ok()))
    }

    pub fn upsert_stream_profile(&self, profile: &StreamProfile) -> Result<()> {
        let payload = serde_json::to_string(profile)?;
        self.conn.execute(
            "INSERT INTO stream_profiles (id, name, description, payload_json, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                payload_json = excluded.payload_json,
                updated_at_ms = excluded.updated_at_ms",
            params![
                profile.id,
                profile.name,
                profile.description,
                payload,
                profile.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn delete_stream_profile(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM stream_profiles WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_active_profile_id(&self) -> Result<String> {
        Ok(self
            .get_setting("active_profile_id")?
            .unwrap_or_default())
    }

    pub fn set_active_profile_id(&self, id: &str) -> Result<()> {
        self.set_setting("active_profile_id", id)
    }

    /// プロファイル切替時にルールの enabled フラグを単一トランザクションで一括更新
    pub fn apply_rule_enabled_maps(
        &self,
        highlight_map: &HashMap<String, bool>,
        counter_map: &HashMap<String, bool>,
        media_map: &HashMap<String, bool>,
        ng_map: &HashMap<String, bool>,
    ) -> Result<()> {
        self.in_savepoint("apply_rule_enabled_maps", |db| {
            if !highlight_map.is_empty() {
                let mut stmt = db
                    .conn
                    .prepare_cached("UPDATE highlight_rules SET enabled = ?1 WHERE id = ?2")?;
                for (id, en) in highlight_map {
                    let _ = stmt.execute(params![if *en { 1 } else { 0 }, id]);
                }
            }
            if !counter_map.is_empty() {
                let mut stmt = db
                    .conn
                    .prepare_cached("UPDATE word_counters SET enabled = ?1 WHERE id = ?2")?;
                for (id, en) in counter_map {
                    let _ = stmt.execute(params![if *en { 1 } else { 0 }, id]);
                }
            }
            if !media_map.is_empty() {
                let mut stmt = db
                    .conn
                    .prepare_cached("UPDATE media_reaction_rules SET enabled = ?1 WHERE id = ?2")?;
                for (id, en) in media_map {
                    let _ = stmt.execute(params![if *en { 1 } else { 0 }, id]);
                }
            }
            if !ng_map.is_empty() {
                let mut stmt = db
                    .conn
                    .prepare_cached("UPDATE ng_word_rules SET enabled = ?1 WHERE id = ?2")?;
                for (id, en) in ng_map {
                    let _ = stmt.execute(params![if *en { 1 } else { 0 }, id]);
                }
            }
            Ok(())
        })
    }

    // ==================== オンエアレポート（セッションコメント履歴） ====================

    pub fn insert_session_comment(
        &self,
        session_id: &str,
        raw: &RawIncomingComment,
        is_first_time: bool,
    ) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO session_comments (
                session_id, comment_id, user_id, user_name, message, timestamp_ms,
                is_first_time, is_member, event_kind, amount_display, amount_micros, currency
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )?;
        stmt.execute(params![
            session_id,
            raw.id,
            raw.user_id,
            raw.user_name,
            raw.message,
            raw.timestamp_ms,
            if is_first_time { 1 } else { 0 },
            if raw.is_member { 1 } else { 0 },
            raw.event_kind.map(|k| k.as_str()),
            raw.amount_display.as_deref().unwrap_or(""),
            raw.amount_micros as i64,
            raw.currency,
        ])?;
        Ok(())
    }

    pub fn list_stream_session_summaries(&self, limit: usize) -> Result<Vec<StreamSessionSummary>> {
        // 先に最新 limit 件のセッションだけ絞り込んでから session_comments を結合・集計（全過去セッション走査を回避）
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                s.id, s.platform, s.stream_id, s.title, s.started_at_ms,
                COALESCE(MAX(c.timestamp_ms), s.started_at_ms) AS last_ts,
                COUNT(c.comment_id) AS total_comments,
                COUNT(DISTINCT c.user_id) AS unique_users,
                COALESCE(SUM(c.is_first_time), 0) AS first_timers,
                COALESCE(SUM(CASE WHEN c.event_kind IS NOT NULL THEN 1 ELSE 0 END), 0) AS mon_cnt
             FROM (
                SELECT id, platform, stream_id, title, started_at_ms
                FROM stream_sessions
                ORDER BY started_at_ms DESC
                LIMIT ?1
             ) s
             LEFT JOIN session_comments c ON c.session_id = s.id
             GROUP BY s.id
             ORDER BY s.started_at_ms DESC",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let plat: String = row.get(1)?;
            Ok(StreamSessionSummary {
                session_id: row.get(0)?,
                platform: PlatformKind::from_str_lossy(&plat),
                stream_id: row.get(2)?,
                title: row.get(3)?,
                started_at_ms: row.get(4)?,
                last_comment_at_ms: row.get(5)?,
                total_comments: row.get::<_, i64>(6)?.max(0) as u64,
                unique_users: row.get::<_, i64>(7)?.max(0) as u64,
                first_time_users: row.get::<_, i64>(8)?.max(0) as u64,
                monetization_count: row.get::<_, i64>(9)?.max(0) as u64,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_stream_session_summary(&self, session_id: &str) -> Result<Option<StreamSessionSummary>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                s.id, s.platform, s.stream_id, s.title, s.started_at_ms,
                COALESCE(MAX(c.timestamp_ms), s.started_at_ms) AS last_ts,
                COUNT(c.comment_id) AS total_comments,
                COUNT(DISTINCT c.user_id) AS unique_users,
                COALESCE(SUM(c.is_first_time), 0) AS first_timers,
                COALESCE(SUM(CASE WHEN c.event_kind IS NOT NULL THEN 1 ELSE 0 END), 0) AS mon_cnt
             FROM stream_sessions s
             LEFT JOIN session_comments c ON c.session_id = s.id
             WHERE s.id = ?1
             GROUP BY s.id",
        )?;
        let row = stmt
            .query_row(params![session_id], |row| {
                let plat: String = row.get(1)?;
                Ok(StreamSessionSummary {
                    session_id: row.get(0)?,
                    platform: PlatformKind::from_str_lossy(&plat),
                    stream_id: row.get(2)?,
                    title: row.get(3)?,
                    started_at_ms: row.get(4)?,
                    last_comment_at_ms: row.get(5)?,
                    total_comments: row.get::<_, i64>(6)?.max(0) as u64,
                    unique_users: row.get::<_, i64>(7)?.max(0) as u64,
                    first_time_users: row.get::<_, i64>(8)?.max(0) as u64,
                    monetization_count: row.get::<_, i64>(9)?.max(0) as u64,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn list_session_comments(&self, session_id: &str) -> Result<Vec<SessionCommentRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT
                comment_id, user_id, user_name, message, timestamp_ms,
                is_first_time, is_member, event_kind, amount_display, amount_micros, currency
             FROM session_comments
             WHERE session_id = ?1
             ORDER BY timestamp_ms ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SessionCommentRow {
                comment_id: row.get(0)?,
                user_id: row.get(1)?,
                user_name: row.get(2)?,
                message: row.get(3)?,
                timestamp_ms: row.get(4)?,
                is_first_time: row.get::<_, i64>(5)? != 0,
                is_member: row.get::<_, i64>(6)? != 0,
                event_kind: row.get(7)?,
                amount_display: row.get(8)?,
                amount_micros: row.get::<_, i64>(9)?.max(0) as u64,
                currency: row.get(10)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_stream_session(&self, session_id: &str) -> Result<()> {
        self.in_savepoint("delete_stream_session", |db| {
            db.conn.execute(
                "DELETE FROM session_comments WHERE session_id = ?1",
                params![session_id],
            )?;
            db.conn.execute(
                "DELETE FROM session_participants WHERE session_id = ?1",
                params![session_id],
            )?;
            db.conn.execute(
                "DELETE FROM stream_sessions WHERE id = ?1",
                params![session_id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_comment(id: &str, user_id: &str, msg: &str, ts: i64) -> RawIncomingComment {
        RawIncomingComment {
            id: id.to_string(),
            platform: PlatformKind::YouTube,
            user_id: user_id.to_string(),
            user_name: format!("User {}", user_id),
            user_icon_url: "https://example.com/icon.png".to_string(),
            message: msg.to_string(),
            message_runs: Vec::new(),
            timestamp_ms: ts,
            is_owner: false,
            is_moderator: false,
            is_member: false,
            event_kind: None,
            amount_display: None,
            amount_micros: 0,
            currency: String::new(),
            tier_color: None,
        }
    }

    #[test]
    fn test_watch_count_increments_once_per_session() {
        let db = Database::open_in_memory().unwrap();
        let s1 = db
            .get_or_create_session(PlatformKind::YouTube, "stream-1", "配信1", 1000)
            .unwrap();
        // 同一配信内で3回コメントしても視聴回数は1、コメント回数は3
        db.record_user_comment(&s1, &sample_comment("c1", "u1", "こんちゃ", 1001))
            .unwrap();
        db.record_user_comment(&s1, &sample_comment("c2", "u1", "草", 1002))
            .unwrap();
        db.record_user_comment(&s1, &sample_comment("c3", "u1", "おつ", 1003))
            .unwrap();

        let users = db.list_users(Some(&s1)).unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].watch_count, 1);
        assert_eq!(users[0].total_comment_count, 3);
        assert_eq!(users[0].session_comment_count, 3);

        // 同じ stream_id に再接続しても視聴回数は増えない
        let s1_re = db
            .get_or_create_session(PlatformKind::YouTube, "stream-1", "配信1", 2000)
            .unwrap();
        assert_eq!(s1, s1_re);
        db.record_user_comment(&s1_re, &sample_comment("c4", "u1", "ただいま", 2001))
            .unwrap();
        let users = db.list_users(Some(&s1_re)).unwrap();
        assert_eq!(users[0].watch_count, 1);
        assert_eq!(users[0].total_comment_count, 4);

        // 別配信でコメントすると視聴回数が2になる
        let s2 = db
            .get_or_create_session(PlatformKind::YouTube, "stream-2", "配信2", 3000)
            .unwrap();
        db.record_user_comment(&s2, &sample_comment("c5", "u1", "2枠目！", 3001))
            .unwrap();
        let users = db.list_users(Some(&s2)).unwrap();
        assert_eq!(users[0].watch_count, 2);
        assert_eq!(users[0].total_comment_count, 5);
        assert_eq!(users[0].session_comment_count, 1);
    }

    #[test]
    fn test_word_counter_per_account_limit() {
        let db = Database::open_in_memory().unwrap();
        let counter = WordCounterConfig {
            id: "test-cnt".to_string(),
            name: "テスト".to_string(),
            keywords: vec!["参加".to_string()],
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            max_per_account: Some(2),
            exclude_banned: true,
            accent_color: "#38BDF8".to_string(),
            enabled: true,
            total_count: 0,
            unique_user_count: 0,
        };
        db.upsert_word_counter(&counter).unwrap();

        assert!(db
            .increment_counter_for_user("test-cnt", "u1", Some(2))
            .unwrap());
        assert!(db
            .increment_counter_for_user("test-cnt", "u1", Some(2))
            .unwrap());
        // 3回目は上限2により無視
        assert!(!db
            .increment_counter_for_user("test-cnt", "u1", Some(2))
            .unwrap());
        // 別ユーザーはカウントされる
        assert!(db
            .increment_counter_for_user("test-cnt", "u2", Some(2))
            .unwrap());

        let counters = db.list_word_counters().unwrap();
        let found = counters.iter().find(|c| c.id == "test-cnt").unwrap();
        assert_eq!(found.total_count, 3);
        assert_eq!(found.unique_user_count, 2);
    }

    #[test]
    fn test_feature_flags_persistence() {
        let db = Database::open_in_memory().unwrap();
        let defaults = db.get_feature_flags().unwrap();
        assert!(defaults.comment_display_enabled);
        assert!(defaults.word_highlight_enabled);
        assert!(defaults.obs_overlay_enabled);
        // 先行版（pre）の更新アナウンスはデフォルト OFF
        assert!(!defaults.include_prerelease_updates);

        let custom = FeatureFlags {
            comment_display_enabled: false,
            word_highlight_enabled: false,
            word_counter_enabled: true,
            user_lottery_enabled: false,
            comment_lottery_enabled: true,
            user_tracking_enabled: false,
            user_memo_display_enabled: false,
            first_time_counter_enabled: false,
            superchat_enabled: false,
            poll_enabled: true,
            pickup_enabled: false,
            queue_enabled: true,
            media_reaction_enabled: false,
            ng_word_filter_enabled: false,
            obs_overlay_enabled: false,
            auto_update_check_enabled: false,
            include_prerelease_updates: true,
        };
        db.set_feature_flags(&custom).unwrap();
        assert_eq!(db.get_feature_flags().unwrap(), custom);

        // 旧バージョン JSON（include_prerelease_updates 欠損）でも後方互換で false になる
        db.set_setting(
            "feature_flags",
            r#"{"comment_display_enabled":true,"auto_update_check_enabled":true}"#,
        )
        .unwrap();
        let migrated = db.get_feature_flags().unwrap();
        assert!(migrated.auto_update_check_enabled);
        assert!(!migrated.include_prerelease_updates);
    }

    #[test]
    fn test_theme_mode_and_overlay_custom_css_persistence() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_theme_mode().unwrap(), AppThemeMode::Dark);
        db.set_theme_mode(AppThemeMode::Light).unwrap();
        assert_eq!(db.get_theme_mode().unwrap(), AppThemeMode::Light);
        db.set_theme_mode(AppThemeMode::System).unwrap();
        assert_eq!(db.get_theme_mode().unwrap(), AppThemeMode::System);

        // 旧バージョンの OverlayConfig JSON からの後方互換読み込みを確認
        db.set_setting(
            "overlay_config",
            r##"{"port":18765,"comment_font_size":20,"comment_max_items":25,"show_icons":true,"show_timestamps":false,"default_bubble_bg":"#111111E6","default_text_color":"#FFFFFF"}"##,
        )
        .unwrap();
        let migrated = db.get_overlay_config().unwrap();
        assert_eq!(migrated.comment_font_size, 20);
        assert_eq!(migrated.comments_theme, "default");
        assert_eq!(migrated.comments_bg_color, "transparent");
        assert_eq!(migrated.comments_custom_css, "");

        // プリセット・背景色・カスタムCSSの保存・復元
        let mut updated = migrated;
        updated.comments_theme = "pop".to_string();
        updated.lottery_theme = "festive".to_string();
        updated.counter_theme = "scoreboard".to_string();
        updated.first_time_theme = "pastel".to_string();
        updated.comments_bg_color = "#00FF00".to_string();
        updated.first_time_bg_color = "#18181B".to_string();
        updated.comments_font_family = "Meiryo".to_string();
        updated.lottery_font_family = "Yu Mincho".to_string();
        updated.comments_custom_css = ".bubble { border-radius: 0px; }".to_string();
        db.set_overlay_config(&updated).unwrap();
        assert_eq!(db.get_overlay_config().unwrap(), updated);
    }

    #[test]
    fn test_first_time_detection_and_memo_persistence() {
        let db = Database::open_in_memory().unwrap();
        let s1 = db
            .get_or_create_session(PlatformKind::YouTube, "stream-ft-1", "初見テスト", 1000)
            .unwrap();

        // 初回コメントは未登録(was_unregistered = true)かつ未検知
        let c1 = sample_comment("m1", "u_new", "こんばんは", 1001);
        let meta1 = db.record_user_comment(&s1, &c1).unwrap();
        assert!(meta1.was_unregistered);
        assert!(!meta1.already_first_time_detected);
        assert_eq!(meta1.memo, "");

        // 初見検知済みとしてマーク
        let total = db.mark_user_first_time_detected(&c1).unwrap();
        assert_eq!(total, 1);

        // 名前メモを設定すると次回コメントから即座に返る
        db.set_user_memo("u_new", PlatformKind::YouTube, "常連イラストレーター")
            .unwrap();
        let c2 = sample_comment("m2", "u_new", "初見です！", 1002);
        let meta2 = db.record_user_comment(&s1, &c2).unwrap();
        assert!(!meta2.was_unregistered);
        assert!(meta2.already_first_time_detected);
        assert_eq!(meta2.memo, "常連イラストレーター");

        // デフォルトでは初見プロフィール表示はOFF
        let ft_cfg = db.get_first_time_config().unwrap();
        assert!(!ft_cfg.show_first_timer_profile);
    }

    #[test]
    fn test_delete_user_and_reset_users() {
        let db = Database::open_in_memory().unwrap();
        let s1 = db
            .get_or_create_session(PlatformKind::YouTube, "stream-del-1", "削除テスト", 1000)
            .unwrap();
        db.record_user_comment(&s1, &sample_comment("c1", "u1", "こんちゃ", 1001))
            .unwrap();
        db.record_user_comment(&s1, &sample_comment("c2", "u2", "やあ", 1002))
            .unwrap();
        db.record_user_comment(&s1, &sample_comment("c3", "u_ban", "荒らし", 1003))
            .unwrap();
        db.set_user_ban("u_ban", PlatformKind::YouTube, true, "荒らし")
            .unwrap();

        // 個別削除
        db.delete_user("u1", PlatformKind::YouTube).unwrap();
        let after_del = db.list_users(Some(&s1)).unwrap();
        assert_eq!(after_del.len(), 2);
        assert!(after_del.iter().all(|u| u.user_id != "u1"));

        // BAN ユーザーを残して一括リセット
        db.reset_users(true).unwrap();
        let after_keep_ban = db.list_users(Some(&s1)).unwrap();
        assert_eq!(after_keep_ban.len(), 1);
        assert_eq!(after_keep_ban[0].user_id, "u_ban");

        // BAN ユーザーも含めて完全一括リセット
        db.reset_users(false).unwrap();
        let after_all = db.list_users(Some(&s1)).unwrap();
        assert!(after_all.is_empty());
    }

    #[test]
    fn test_monetization_and_media_rules_persistence() {
        let db = Database::open_in_memory().unwrap();
        let sc = MonetizationItem {
            id: "sc-1".to_string(),
            session_id: "sess-1".to_string(),
            platform: PlatformKind::YouTube,
            kind: MonetizationEventKind::SuperChat,
            user_id: "u1".to_string(),
            user_name: "応援太郎".to_string(),
            user_icon_url: String::new(),
            amount_display: "¥1,000".to_string(),
            amount_micros: 1_000_000_000,
            currency: "JPY".to_string(),
            tier_color: "#FFCA28".to_string(),
            message: "応援してます！".to_string(),
            message_runs: Vec::new(),
            timestamp_ms: 1000,
            is_checked: false,
            checked_at_ms: None,
        };
        db.insert_monetization_event(&sc).unwrap();
        let items = db.list_monetization_events(50).unwrap();
        assert_eq!(items.len(), 1);
        assert!(!items[0].is_checked);

        db.set_monetization_checked("sc-1", true, 2000).unwrap();
        let checked = db.list_monetization_events(50).unwrap();
        assert!(checked[0].is_checked);
        assert_eq!(checked[0].checked_at_ms, Some(2000));

        db.clear_monetization_events(true).unwrap();
        assert!(db.list_monetization_events(50).unwrap().is_empty());

        let rule = MediaReactionRule {
            id: "mr-1".to_string(),
            name: "拍手アニメ".to_string(),
            pattern: "888\nパチパチ".to_string(),
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            effect_type: MediaEffectType::Cracker,
            trigger_type: MediaTriggerType::Keyword,
            min_superchat_amount: 0,
            priority: 10,
            media_kind: MediaKind::Image,
            stored_file_name: "clap.gif".to_string(),
            original_file_name: "clap.gif".to_string(),
            sound_stored_file_name: "clap.mp3".to_string(),
            sound_original_file_name: "clap.mp3".to_string(),
            sound_volume: 90,
            animation: MediaAnimationKind::Bounce,
            position: MediaPosition::BottomRight,
            duration_ms: 3500,
            width_percent: 40,
            volume: 70,
            particle_count: 30,
            particle_size: 64,
            particle_speed: 110,
            gravity: 100,
            particle_text: "👏,🎉".to_string(),
            ambient_preset: "snow".to_string(),
            cooldown_sec: 4,
            exclude_banned: true,
            caption_template: "{user} さんありがとう！".to_string(),
            enabled: true,
        };
        db.upsert_media_reaction_rule(&rule).unwrap();
        let list = db.list_media_reaction_rules().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].stored_file_name, "clap.gif");
        assert_eq!(list[0].effect_type, MediaEffectType::Cracker);
        assert_eq!(list[0].sound_stored_file_name, "clap.mp3");
        let deleted_files = db.delete_media_reaction_rule("mr-1").unwrap();
        assert_eq!(deleted_files, vec!["clap.gif".to_string(), "clap.mp3".to_string()]);
    }

    #[test]
    fn test_savepoint_rollback_and_session_summary_limit_ordering() {
        let db = Database::open_in_memory().unwrap();
        // セーブポイント内のエラー時はロールバックされ、部分書き込みが残らないこと
        let res: Result<()> = db.in_savepoint("rollback_test", |d| {
            d.set_setting("sp_key", "temp")?;
            Err(anyhow::anyhow!("forced failure"))
        });
        assert!(res.is_err());
        assert_eq!(db.get_setting("sp_key").unwrap(), None);

        // 複数セッションを時刻順で作成し、limit 付きサマリーが新しい順で正確に集計されること
        let s1 = db
            .get_or_create_session(PlatformKind::YouTube, "v1", "古い配信", 1_000)
            .unwrap();
        let s2 = db
            .get_or_create_session(PlatformKind::YouTube, "v2", "中間配信", 2_000)
            .unwrap();
        let s3 = db
            .get_or_create_session(PlatformKind::YouTube, "v3", "最新配信", 3_000)
            .unwrap();

        db.in_savepoint("batch_comments", |d| {
            d.insert_session_comment(&s1, &sample_comment("c1", "u1", "a", 1_100), true)?;
            d.insert_session_comment(&s2, &sample_comment("c2", "u1", "b", 2_100), false)?;
            d.insert_session_comment(&s3, &sample_comment("c3", "u1", "c", 3_100), true)?;
            d.insert_session_comment(&s3, &sample_comment("c4", "u2", "d", 3_200), false)?;
            Ok(())
        })
        .unwrap();

        let top2 = db.list_stream_session_summaries(2).unwrap();
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].session_id, s3);
        assert_eq!(top2[0].total_comments, 2);
        assert_eq!(top2[0].unique_users, 2);
        assert_eq!(top2[0].first_time_users, 1);
        assert_eq!(top2[1].session_id, s2);
        assert_eq!(top2[1].total_comments, 1);
    }
}
