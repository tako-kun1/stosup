use std::collections::HashSet;

use anyhow::{anyhow, Result};
use chrono::Utc;
use rand::seq::SliceRandom;

use crate::engine::matches_pattern;
use crate::models::{
    CommentItem, CommentLotteryConfig, CommentLotteryEntry, CommentLotteryState,
    LotteryRepeatMode, LotteryResult, LotteryWinner, UserLotteryRequest, UserRecord,
};

/// ユーザー抽選を実行する（BAN済みユーザーは常に完全除外）
pub fn draw_user_lottery(
    users: &[UserRecord],
    req: &UserLotteryRequest,
    already_won_user_ids: &mut HashSet<String>,
) -> Result<LotteryResult> {
    let candidates: Vec<&UserRecord> = users
        .iter()
        .filter(|u| !u.is_banned)
        .filter(|u| !req.current_session_only || u.in_current_session)
        .filter(|u| u.watch_count >= req.min_watch_count)
        .filter(|u| u.total_comment_count >= req.min_comment_count)
        .filter(|u| {
            req.repeat_mode == LotteryRepeatMode::AllowMultiple
                || !already_won_user_ids.contains(&u.user_id)
        })
        .collect();

    if candidates.is_empty() {
        return Err(anyhow!(
            "抽選対象となるユーザーがいません（フィルタ条件または既当選設定を確認してください）"
        ));
    }

    let now_ms = Utc::now().timestamp_millis();
    let count = (req.winner_count.max(1) as usize).min(candidates.len());
    let mut rng = rand::thread_rng();
    let picked: Vec<&UserRecord> = candidates.choose_multiple(&mut rng, count).copied().collect();

    let mut winners = Vec::with_capacity(picked.len());
    for u in picked {
        if req.repeat_mode == LotteryRepeatMode::OnceOnly {
            already_won_user_ids.insert(u.user_id.clone());
        }
        winners.push(LotteryWinner {
            user_id: u.user_id.clone(),
            user_name: u.display_name.clone(),
            user_icon_url: u.icon_url.clone(),
            comment_text: None,
            won_at_ms: now_ms,
        });
    }

    Ok(LotteryResult {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "user".to_string(),
        winners,
        candidate_count: candidates.len() as u32,
        drawn_at_ms: now_ms,
    })
}

/// コメント抽選の受付中に届いたコメントを評価してエントリー追加する
pub fn try_collect_comment_entry(
    state: &mut CommentLotteryState,
    comment: &CommentItem,
) -> bool {
    if !state.is_accepting {
        return false;
    }
    // BANユーザーは一切参加させない
    if comment.is_banned {
        return false;
    }
    // 開始時刻以前のコメントは除外
    if let Some(started_at) = state.started_at_ms {
        if comment.timestamp_ms < started_at {
            return false;
        }
    }
    // 開始コメント・終了コメント自身のエコーバックを除外
    let trimmed = comment.message.trim();
    if !state.config.start_message.trim().is_empty()
        && trimmed == state.config.start_message.trim()
    {
        return false;
    }
    if !state.config.end_message.trim().is_empty() && trimmed == state.config.end_message.trim() {
        return false;
    }

    // 特定ワード判定（空欄の場合はすべてのコメントを対象）
    let kw = state.config.target_keyword.trim();
    if !kw.is_empty()
        && !matches_pattern(
            &comment.message,
            kw,
            state.config.match_mode,
            state.config.case_sensitive,
        )
    {
        return false;
    }

    // 1ユーザー1口設定の場合は重複登録しない
    if state.config.one_entry_per_user
        && state.entries.iter().any(|e| e.user_id == comment.user_id)
    {
        return false;
    }

    state.entries.push(CommentLotteryEntry {
        comment_id: comment.id.clone(),
        user_id: comment.user_id.clone(),
        user_name: comment.user_name.clone(),
        user_icon_url: comment.user_icon_url.clone(),
        message: comment.message.clone(),
        timestamp_ms: comment.timestamp_ms,
    });
    true
}

/// コメント抽選を実行する（最新のBANユーザーセットでも二重チェックして除外）
pub fn draw_comment_lottery(
    state: &mut CommentLotteryState,
    banned_user_ids: &HashSet<String>,
) -> Result<LotteryResult> {
    let won_set: HashSet<String> = state.already_won_user_ids.iter().cloned().collect();

    let candidates: Vec<&CommentLotteryEntry> = state
        .entries
        .iter()
        .filter(|e| !banned_user_ids.contains(&e.user_id))
        .filter(|e| {
            state.config.repeat_mode == LotteryRepeatMode::AllowMultiple
                || !won_set.contains(&e.user_id)
        })
        .collect();

    if candidates.is_empty() {
        return Err(anyhow!(
            "抽選可能な応募コメントがありません（BAN除外・既当選除外後の候補が0件です）"
        ));
    }

    let now_ms = Utc::now().timestamp_millis();
    let target_count = state.config.winner_count.max(1) as usize;
    let mut rng = rand::thread_rng();

    // 1回の抽選で同一ユーザーが複数当選しないようにしつつ抽選
    let mut pool = candidates.clone();
    pool.shuffle(&mut rng);

    let mut picked_users = HashSet::new();
    let mut winners = Vec::new();

    for entry in pool {
        if winners.len() >= target_count {
            break;
        }
        if picked_users.contains(&entry.user_id) {
            continue;
        }
        picked_users.insert(entry.user_id.clone());
        if state.config.repeat_mode == LotteryRepeatMode::OnceOnly
            && !state.already_won_user_ids.contains(&entry.user_id)
        {
            state.already_won_user_ids.push(entry.user_id.clone());
        }
        winners.push(LotteryWinner {
            user_id: entry.user_id.clone(),
            user_name: entry.user_name.clone(),
            user_icon_url: entry.user_icon_url.clone(),
            comment_text: Some(entry.message.clone()),
            won_at_ms: now_ms,
        });
    }

    let result = LotteryResult {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "comment".to_string(),
        winners,
        candidate_count: candidates.len() as u32,
        drawn_at_ms: now_ms,
    };
    state.last_result = Some(result.clone());
    Ok(result)
}

pub fn update_comment_lottery_config(
    state: &mut CommentLotteryState,
    config: CommentLotteryConfig,
) {
    state.config = config;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MatchMode, PlatformKind};

    fn make_user(id: &str, banned: bool, in_session: bool) -> UserRecord {
        UserRecord {
            user_id: id.to_string(),
            platform: PlatformKind::YouTube,
            display_name: format!("Name-{}", id),
            icon_url: "https://example.com/a.png".to_string(),
            watch_count: 2,
            total_comment_count: 5,
            session_comment_count: if in_session { 1 } else { 0 },
            is_banned: banned,
            ban_reason: String::new(),
            memo: String::new(),
            first_seen_ms: 100,
            last_seen_ms: 200,
            in_current_session: in_session,
        }
    }

    #[test]
    fn test_user_lottery_excludes_banned_and_respects_once_only() {
        let users = vec![
            make_user("u1", false, true),
            make_user("u2", false, true),
            make_user("banned", true, true),
        ];
        let mut won = HashSet::new();
        let req = UserLotteryRequest {
            current_session_only: true,
            min_watch_count: 0,
            min_comment_count: 0,
            repeat_mode: LotteryRepeatMode::OnceOnly,
            winner_count: 1,
        };

        let r1 = draw_user_lottery(&users, &req, &mut won).unwrap();
        assert_eq!(r1.candidate_count, 2);
        assert_ne!(r1.winners[0].user_id, "banned");

        let r2 = draw_user_lottery(&users, &req, &mut won).unwrap();
        assert_eq!(r2.candidate_count, 1);
        assert_ne!(r2.winners[0].user_id, r1.winners[0].user_id);
        assert_ne!(r2.winners[0].user_id, "banned");

        // 3回目は残り候補ゼロでエラー
        assert!(draw_user_lottery(&users, &req, &mut won).is_err());

        // 複数回モードならまた引ける（ただしBANユーザーは絶対に出ない）
        let req_multi = UserLotteryRequest {
            repeat_mode: LotteryRepeatMode::AllowMultiple,
            ..req
        };
        for _ in 0..20 {
            let rm = draw_user_lottery(&users, &req_multi, &mut won).unwrap();
            assert_ne!(rm.winners[0].user_id, "banned");
        }
    }

    #[test]
    fn test_comment_lottery_window_and_ban_exclusion() {
        let mut state = CommentLotteryState {
            is_accepting: true,
            started_at_ms: Some(1000),
            config: CommentLotteryConfig {
                start_message: "【開始】参加してね".to_string(),
                end_message: "【終了】しめきり".to_string(),
                target_keyword: "参加".to_string(),
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                one_entry_per_user: true,
                repeat_mode: LotteryRepeatMode::OnceOnly,
                winner_count: 1,
                post_to_chat: false,
            },
            ..Default::default()
        };

        let mk = |id: &str, uid: &str, msg: &str, ts: i64, banned: bool| CommentItem {
            id: id.to_string(),
            session_id: "s1".to_string(),
            platform: PlatformKind::YouTube,
            user_id: uid.to_string(),
            user_name: format!("U-{}", uid),
            user_icon_url: String::new(),
            user_memo: String::new(),
            is_first_time: false,
            message: msg.to_string(),
            timestamp_ms: ts,
            is_owner: false,
            is_moderator: false,
            is_member: false,
            is_banned: banned,
            highlight_rule_id: None,
            bubble_bg_color: None,
            bubble_text_color: None,
            bubble_border_color: None,
        };

        // 開始前のコメントは除外
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("1", "u1", "参加します", 999, false)
        ));
        // 開始メッセージ自体は除外
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("2", "owner", "【開始】参加してね", 1000, false)
        ));
        // 対象外コメントは除外
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("3", "u1", "こんにちは", 1005, false)
        ));
        // BANユーザーは除外
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("4", "bad", "参加希望", 1010, true)
        ));
        // 正常応募
        assert!(try_collect_comment_entry(
            &mut state,
            &mk("5", "u1", "参加希望！", 1020, false)
        ));
        // 1ユーザー1口なので2回目は無視
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("6", "u1", "もう一回参加！", 1030, false)
        ));
        // 応募後にBANされたユーザーも抽選時に除外される
        assert!(try_collect_comment_entry(
            &mut state,
            &mk("7", "u2", "参加", 1040, false)
        ));

        state.is_accepting = false;
        // 受付終了後のコメントは除外
        assert!(!try_collect_comment_entry(
            &mut state,
            &mk("8", "u3", "参加", 1050, false)
        ));

        let mut banned_now = HashSet::new();
        banned_now.insert("u2".to_string());
        let res = draw_comment_lottery(&mut state, &banned_now).unwrap();
        assert_eq!(res.candidate_count, 1);
        assert_eq!(res.winners[0].user_id, "u1");
        assert_eq!(res.winners[0].comment_text.as_deref(), Some("参加希望！"));
    }
}
