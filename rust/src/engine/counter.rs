use anyhow::Result;

use crate::db::Database;
use crate::engine::matches_pattern;
use crate::models::WordCounterConfig;

/// 受信コメントに対して有効な全ワードカウンターを評価し、カウントされたものがあれば true を返す
pub fn process_counters_for_comment(
    db: &Database,
    user_id: &str,
    is_banned: bool,
    message: &str,
) -> Result<bool> {
    let counters = db.list_word_counters()?;
    let mut changed = false;

    for counter in counters {
        if !counter.enabled {
            continue;
        }
        if counter.exclude_banned && is_banned {
            continue;
        }
        if matches_counter(message, &counter)
            && db.increment_counter_for_user(&counter.id, user_id, counter.max_per_account)?
        {
            changed = true;
        }
    }

    Ok(changed)
}

pub fn matches_counter(message: &str, counter: &WordCounterConfig) -> bool {
    counter.keywords.iter().any(|kw| {
        matches_pattern(message, kw, counter.match_mode, counter.case_sensitive)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MatchMode;

    #[test]
    fn test_counter_unlimited_vs_per_account_and_ban() {
        let db = Database::open_in_memory().unwrap();
        let unlimited = WordCounterConfig {
            id: "u".to_string(),
            name: "上限なし".to_string(),
            keywords: vec!["草".to_string()],
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            max_per_account: None,
            exclude_banned: true,
            accent_color: "#22C55E".to_string(),
            enabled: true,
            total_count: 0,
            unique_user_count: 0,
        };
        let limited = WordCounterConfig {
            id: "l".to_string(),
            name: "1回限定".to_string(),
            keywords: vec!["草".to_string()],
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            max_per_account: Some(1),
            exclude_banned: true,
            accent_color: "#38BDF8".to_string(),
            enabled: true,
            total_count: 0,
            unique_user_count: 0,
        };
        db.upsert_word_counter(&unlimited).unwrap();
        db.upsert_word_counter(&limited).unwrap();

        // 同一ユーザーが2回コメント
        assert!(process_counters_for_comment(&db, "user1", false, "大草原").unwrap());
        assert!(process_counters_for_comment(&db, "user1", false, "草すぎ").unwrap());
        // BANユーザーは除外
        assert!(!process_counters_for_comment(&db, "banned_u", true, "草").unwrap());

        let list = db.list_word_counters().unwrap();
        let u_res = list.iter().find(|c| c.id == "u").unwrap();
        let l_res = list.iter().find(|c| c.id == "l").unwrap();
        assert_eq!(u_res.total_count, 2);
        assert_eq!(l_res.total_count, 1);
    }
}
