use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{
    HighlightRule, LotteryResult, LotteryWinner, MatchMode, OverlayConfig, PlatformKind,
    RawIncomingComment, UserRecord, WordCounterConfig,
};

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
            ",
        )?;

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

    // ==================== 設定 (Key-Value) ====================

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let res = self
            .conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(res)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
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
    /// 戻り値: 最新の is_banned 状態
    pub fn record_user_comment(
        &self,
        session_id: &str,
        raw: &RawIncomingComment,
    ) -> Result<bool> {
        let platform_str = raw.platform.as_str();

        // ユーザーテーブルへ UPSERT
        self.conn.execute(
            "INSERT INTO users (
                user_id, platform, display_name, icon_url,
                watch_count, total_comment_count, is_banned, ban_reason, memo,
                first_seen_ms, last_seen_ms
            ) VALUES (?1, ?2, ?3, ?4, 0, 1, 0, '', '', ?5, ?5)
            ON CONFLICT(user_id, platform) DO UPDATE SET
                display_name = excluded.display_name,
                icon_url = CASE WHEN excluded.icon_url != '' THEN excluded.icon_url ELSE users.icon_url END,
                total_comment_count = users.total_comment_count + 1,
                last_seen_ms = excluded.last_seen_ms",
            params![
                raw.user_id,
                platform_str,
                raw.user_name,
                raw.user_icon_url,
                raw.timestamp_ms
            ],
        )?;

        // この配信セッションでの参加が初回かを確認
        let already_in_session: Option<i64> = self
            .conn
            .query_row(
                "SELECT comment_count FROM session_participants
                 WHERE session_id = ?1 AND user_id = ?2 AND platform = ?3",
                params![session_id, raw.user_id, platform_str],
                |row| row.get(0),
            )
            .optional()?;

        if already_in_session.is_none() {
            self.conn.execute(
                "INSERT INTO session_participants (session_id, user_id, platform, comment_count)
                 VALUES (?1, ?2, ?3, 1)",
                params![session_id, raw.user_id, platform_str],
            )?;
            // 1配信につき初回コメントで視聴回数 +1
            self.conn.execute(
                "UPDATE users SET watch_count = watch_count + 1
                 WHERE user_id = ?1 AND platform = ?2",
                params![raw.user_id, platform_str],
            )?;
        } else {
            self.conn.execute(
                "UPDATE session_participants SET comment_count = comment_count + 1
                 WHERE session_id = ?1 AND user_id = ?2 AND platform = ?3",
                params![session_id, raw.user_id, platform_str],
            )?;
        }

        let is_banned: i64 = self.conn.query_row(
            "SELECT is_banned FROM users WHERE user_id = ?1 AND platform = ?2",
            params![raw.user_id, platform_str],
            |row| row.get(0),
        )?;

        Ok(is_banned != 0)
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

    pub fn list_users(&self, current_session_id: Option<&str>) -> Result<Vec<UserRecord>> {
        let session_id = current_session_id.unwrap_or("");
        let mut stmt = self.conn.prepare(
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
        let mut stmt = self.conn.prepare(
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
        self.conn.execute(
            "DELETE FROM word_counter_user_counts WHERE counter_id = ?1",
            params![id],
        )?;
        self.conn.execute(
            "UPDATE word_counters SET total_count = 0 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn adjust_word_counter(&self, id: &str, delta: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE word_counters SET total_count = MAX(0, total_count + ?1) WHERE id = ?2",
            params![delta, id],
        )?;
        Ok(())
    }

    pub fn get_counter_user_counts(&self, counter_id: &str) -> Result<HashMap<String, u32>> {
        let mut stmt = self.conn.prepare(
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
        let current: u32 = self
            .conn
            .query_row(
                "SELECT count FROM word_counter_user_counts WHERE counter_id = ?1 AND user_id = ?2",
                params![counter_id, user_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0) as u32;

        if let Some(limit) = max_per_account {
            if current >= limit {
                return Ok(false);
            }
        }

        self.conn.execute(
            "INSERT INTO word_counter_user_counts (counter_id, user_id, count)
             VALUES (?1, ?2, 1)
             ON CONFLICT(counter_id, user_id) DO UPDATE SET count = count + 1",
            params![counter_id, user_id],
        )?;
        self.conn.execute(
            "UPDATE word_counters SET total_count = total_count + 1 WHERE id = ?1",
            params![counter_id],
        )?;
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
            timestamp_ms: ts,
            is_owner: false,
            is_moderator: false,
            is_member: false,
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
}
