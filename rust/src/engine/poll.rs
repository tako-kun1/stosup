use std::collections::HashMap;

use chrono::Utc;

use crate::models::{PollConfig, PollOption, PollState};

/// 投票のインメモリ実行状態（1アカウント1票の投票者マップを保持）
#[derive(Debug, Default)]
pub struct PollRuntime {
    pub state: PollState,
    /// user_id -> option_id
    pub voters: HashMap<String, String>,
}

impl PollRuntime {
    pub fn with_config(config: PollConfig) -> Self {
        Self {
            state: PollState {
                is_active: false,
                started_at_ms: None,
                ended_at_ms: None,
                config,
                total_votes: 0,
            },
            voters: HashMap::new(),
        }
    }

    pub fn start(&mut self, mut config: PollConfig) -> PollState {
        for (idx, opt) in config.options.iter_mut().enumerate() {
            if opt.id.trim().is_empty() {
                opt.id = format!("opt-{}", idx + 1);
            }
            opt.vote_count = 0;
        }
        self.voters.clear();
        self.state = PollState {
            is_active: true,
            started_at_ms: Some(Utc::now().timestamp_millis()),
            ended_at_ms: None,
            config,
            total_votes: 0,
        };
        self.state.clone()
    }

    pub fn update_config(&mut self, mut config: PollConfig) -> PollState {
        let mut used_ids = std::collections::HashSet::new();
        for (idx, opt) in config.options.iter_mut().enumerate() {
            let trimmed = opt.id.trim().to_string();
            if trimmed.is_empty() || !used_ids.insert(trimmed.clone()) {
                let mut seq = idx + 1;
                let mut candidate = format!("opt-{}", seq);
                while !used_ids.insert(candidate.clone()) {
                    seq += 1;
                    candidate = format!("opt-{}", seq);
                }
                opt.id = candidate;
            } else {
                opt.id = trimmed;
            }
        }
        // 既存の選択肢 ID に紐づく票数を引き継ぎ、削除された選択肢の投票履歴も整合させる
        let prev_counts: HashMap<String, u64> = self
            .state
            .config
            .options
            .iter()
            .map(|o| (o.id.clone(), o.vote_count))
            .collect();
        for opt in config.options.iter_mut() {
            if let Some(&cnt) = prev_counts.get(&opt.id) {
                opt.vote_count = cnt;
            } else {
                opt.vote_count = 0;
            }
        }
        let valid_ids: std::collections::HashSet<&str> =
            config.options.iter().map(|o| o.id.as_str()).collect();
        self.voters.retain(|_, opt_id| valid_ids.contains(opt_id.as_str()));
        self.state.config = config;
        self.recalc_total();
        self.state.clone()
    }

    pub fn stop(&mut self) -> PollState {
        self.state.is_active = false;
        self.state.ended_at_ms = Some(Utc::now().timestamp_millis());
        self.state.clone()
    }

    pub fn reset(&mut self) -> PollState {
        self.voters.clear();
        for opt in self.state.config.options.iter_mut() {
            opt.vote_count = 0;
        }
        self.state.total_votes = 0;
        self.state.started_at_ms = if self.state.is_active {
            Some(Utc::now().timestamp_millis())
        } else {
            None
        };
        self.state.ended_at_ms = None;
        self.state.clone()
    }

    pub fn remove_voter(&mut self, user_id: &str) -> bool {
        if let Some(prev_opt_id) = self.voters.remove(user_id) {
            if let Some(opt) = self
                .state
                .config
                .options
                .iter_mut()
                .find(|o| o.id == prev_opt_id)
            {
                opt.vote_count = opt.vote_count.saturating_sub(1);
            }
            self.recalc_total();
            return true;
        }
        false
    }

    /// コメントを評価して投票を反映する（変更があった場合 true）
    pub fn try_cast_vote(&mut self, user_id: &str, is_banned: bool, message: &str) -> bool {
        if !self.state.is_active {
            return false;
        }
        if self.state.config.exclude_banned && is_banned {
            return false;
        }
        let target_opt_id = match match_poll_option(message, &self.state.config.options) {
            Some(id) => id,
            None => return false,
        };

        if let Some(prev_opt_id) = self.voters.get(user_id).cloned() {
            if !self.state.config.allow_vote_change || prev_opt_id == target_opt_id {
                return false;
            }
            if let Some(old_opt) = self
                .state
                .config
                .options
                .iter_mut()
                .find(|o| o.id == prev_opt_id)
            {
                old_opt.vote_count = old_opt.vote_count.saturating_sub(1);
            }
        }

        if let Some(new_opt) = self
            .state
            .config
            .options
            .iter_mut()
            .find(|o| o.id == target_opt_id)
        {
            new_opt.vote_count += 1;
            self.voters.insert(user_id.to_string(), target_opt_id);
            self.recalc_total();
            return true;
        }
        false
    }

    fn recalc_total(&mut self) {
        self.state.total_votes = self.state.config.options.iter().map(|o| o.vote_count).sum();
    }
}

/// 全角英数字・丸数字を半角へ正規化する
fn normalize_vote_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.trim().chars() {
        let mapped = match ch {
            '０'..='９' => char::from_u32(ch as u32 - '０' as u32 + '0' as u32).unwrap_or(ch),
            'Ａ'..='Ｚ' => char::from_u32(ch as u32 - 'Ａ' as u32 + 'a' as u32).unwrap_or(ch),
            'ａ'..='ｚ' => char::from_u32(ch as u32 - 'ａ' as u32 + 'a' as u32).unwrap_or(ch),
            '①' => '1',
            '②' => '2',
            '③' => '3',
            '④' => '4',
            '⑤' => '5',
            '⑥' => '6',
            '⑦' => '7',
            '⑧' => '8',
            '⑨' => '9',
            '　' => ' ',
            other => other.to_ascii_lowercase(),
        };
        out.push(mapped);
    }
    out
}

fn match_short_token(norm_msg: &str, norm_kw: &str) -> bool {
    if norm_msg == norm_kw {
        return true;
    }
    if let Some(rest) = norm_msg.strip_prefix(norm_kw) {
        if let Some(first) = rest.chars().next() {
            // 「100」など別数字への誤爆を防ぎつつ、「1番」「1票」「1！」「1 雑談」等は許容
            return !first.is_ascii_alphanumeric();
        }
    }
    false
}

/// コメントから一致する投票選択肢 ID を判定する
pub fn match_poll_option(message: &str, options: &[PollOption]) -> Option<String> {
    let norm_msg = normalize_vote_text(message);
    if norm_msg.is_empty() {
        return None;
    }

    for (idx, opt) in options.iter().enumerate() {
        // 番号（1始まり）の自動受付もサポート
        let num_str = (idx + 1).to_string();
        if match_short_token(&norm_msg, &num_str) {
            return Some(opt.id.clone());
        }
        for kw in &opt.keywords {
            let norm_kw = normalize_vote_text(kw);
            if norm_kw.is_empty() {
                continue;
            }
            if norm_kw.chars().count() <= 2 && norm_kw.chars().all(|c| c.is_ascii_alphanumeric()) {
                if match_short_token(&norm_msg, &norm_kw) {
                    return Some(opt.id.clone());
                }
            } else if norm_msg.contains(&norm_kw) {
                return Some(opt.id.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_voting_change_and_ban_exclusion() {
        let mut rt = PollRuntime::with_config(PollConfig::default());
        rt.start(PollConfig::default());

        // 半角数字・丸数字・キーワード・誤爆防止
        assert!(rt.try_cast_vote("u1", false, "1"));
        assert!(rt.try_cast_vote("u2", false, "②番がいい！"));
        assert!(rt.try_cast_vote("u3", false, "ゲームやりたい"));
        assert!(!rt.try_cast_vote("u4", false, "100点満点！"));
        assert!(!rt.try_cast_vote("u-ban", true, "1"));

        assert_eq!(rt.state.total_votes, 3);
        assert_eq!(rt.state.config.options[0].vote_count, 1);
        assert_eq!(rt.state.config.options[1].vote_count, 2);

        // 投票変更（u1 が 1 -> 3 へ変更）
        assert!(rt.try_cast_vote("u1", false, "３"));
        assert_eq!(rt.state.config.options[0].vote_count, 0);
        assert_eq!(rt.state.config.options[2].vote_count, 1);
        assert_eq!(rt.state.total_votes, 3);

        // 投票変更禁止時は上書きされない
        rt.state.config.allow_vote_change = false;
        assert!(!rt.try_cast_vote("u1", false, "2"));
        assert_eq!(rt.state.config.options[2].vote_count, 1);

        // 締切後は受け付けない
        rt.stop();
        assert!(!rt.try_cast_vote("u5", false, "1"));
    }
}
