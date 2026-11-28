use std::collections::HashMap;

use chrono::Utc;

use crate::engine::matches_pattern;
use crate::models::{
    MatchMode, PlatformKind, QueueConfig, QueueEntry, QueueEntryStatus, QueueSortMode, QueueState,
};

/// 参加型キュー（順番待ちリスト）のインメモリ管理構造体
#[derive(Debug, Default)]
pub struct QueueRuntime {
    pub state: QueueState,
    /// user_id -> 累計参加回数（途中離脱・再参加しても回数を保持）
    pub play_counts: HashMap<String, u32>,
}

impl QueueRuntime {
    pub fn from_state(state: QueueState) -> Self {
        let mut play_counts = HashMap::new();
        for e in &state.entries {
            if e.play_count > 0 {
                play_counts.insert(e.user_id.clone(), e.play_count);
            }
        }
        Self { state, play_counts }
    }

    pub fn update_config(&mut self, config: QueueConfig) -> QueueState {
        self.state.config = config;
        self.sort_entries();
        self.state.clone()
    }

    /// コメントからの自動参加登録 / 辞退処理
    #[allow(clippy::too_many_arguments)]
    pub fn process_comment(
        &mut self,
        user_id: &str,
        platform: PlatformKind,
        user_name: &str,
        user_icon_url: &str,
        is_banned: bool,
        is_first_time: bool,
        message: &str,
        now_ms: i64,
    ) -> bool {
        if !self.state.config.is_accepting {
            return false;
        }
        if self.state.config.exclude_banned && is_banned {
            return false;
        }

        // 1. 辞退キーワード判定
        let is_leave = self
            .state
            .config
            .leave_keywords
            .iter()
            .any(|kw| matches_pattern(message, kw, MatchMode::Contains, false));
        if is_leave {
            let before = self.state.entries.len();
            self.state.entries.retain(|e| {
                !(e.user_id == user_id
                    && matches!(e.status, QueueEntryStatus::Waiting | QueueEntryStatus::Active))
            });
            return self.state.entries.len() != before;
        }

        // 2. 参加キーワード判定
        let is_join = self
            .state
            .config
            .join_keywords
            .iter()
            .any(|kw| matches_pattern(message, kw, MatchMode::Contains, false));
        if !is_join {
            return false;
        }

        // すでに参加中 or 待機中なら重複登録しない
        if let Some(existing) = self.state.entries.iter().find(|e| e.user_id == user_id) {
            if matches!(
                existing.status,
                QueueEntryStatus::Active | QueueEntryStatus::Waiting
            ) {
                return false;
            }
            if existing.status == QueueEntryStatus::Completed
                && !self.state.config.allow_rejoin_after_complete
            {
                return false;
            }
        }

        // 過去の Completed エントリを置き換えて再度待機列へ
        self.state.entries.retain(|e| e.user_id != user_id);
        let play_count = *self.play_counts.get(user_id).unwrap_or(&0);
        self.state.entries.push(QueueEntry {
            user_id: user_id.to_string(),
            platform,
            user_name: user_name.to_string(),
            user_icon_url: user_icon_url.to_string(),
            status: QueueEntryStatus::Waiting,
            play_count,
            is_first_time,
            note: String::new(),
            joined_at_ms: now_ms,
        });
        self.sort_entries();
        true
    }

    /// 手動でユーザーを待機列へ追加する
    pub fn add_manual(&mut self, user_name: &str, note: &str) -> QueueState {
        let name = user_name.trim();
        if !name.is_empty() {
            let uid = format!("manual-q-{}", uuid::Uuid::new_v4());
            self.state.entries.push(QueueEntry {
                user_id: uid,
                platform: PlatformKind::Demo,
                user_name: name.to_string(),
                user_icon_url: String::new(),
                status: QueueEntryStatus::Waiting,
                play_count: 0,
                is_first_time: false,
                note: note.trim().to_string(),
                joined_at_ms: Utc::now().timestamp_millis(),
            });
            self.sort_entries();
        }
        self.state.clone()
    }

    /// ステータス変更（Active -> Completed 時は参加回数を +1）
    pub fn set_status(&mut self, user_id: &str, next: QueueEntryStatus) -> QueueState {
        if let Some(entry) = self.state.entries.iter_mut().find(|e| e.user_id == user_id) {
            if entry.status == QueueEntryStatus::Active && next == QueueEntryStatus::Completed {
                entry.play_count += 1;
                self.play_counts
                    .insert(entry.user_id.clone(), entry.play_count);
            }
            entry.status = next;
            if next == QueueEntryStatus::Waiting {
                entry.joined_at_ms = Utc::now().timestamp_millis();
            }
            self.sort_entries();
        }
        self.state.clone()
    }

    /// 空き枠の分だけ待機列先頭から参加中へ繰り上げる
    pub fn promote_next(&mut self) -> QueueState {
        let max_slots = self.state.config.max_active_slots.max(1) as usize;
        let current_active = self
            .state
            .entries
            .iter()
            .filter(|e| e.status == QueueEntryStatus::Active)
            .count();
        let mut need = max_slots.saturating_sub(current_active);
        if need == 0 {
            return self.state.clone();
        }
        for entry in self.state.entries.iter_mut() {
            if need == 0 {
                break;
            }
            if entry.status == QueueEntryStatus::Waiting {
                entry.status = QueueEntryStatus::Active;
                need -= 1;
            }
        }
        self.sort_entries();
        self.state.clone()
    }

    /// 現在の参加中メンバー全員を完了（参加回数 +1）にし、次の待機者を枠数分自動繰り上げする
    pub fn rotate_active(&mut self) -> QueueState {
        for entry in self.state.entries.iter_mut() {
            if entry.status == QueueEntryStatus::Active {
                entry.status = QueueEntryStatus::Completed;
                entry.play_count += 1;
                self.play_counts
                    .insert(entry.user_id.clone(), entry.play_count);
            }
        }
        self.promote_next()
    }

    /// 待機列内での手動並び替え（delta: -1 で上へ、+1 で下へ）
    pub fn move_entry(&mut self, user_id: &str, delta: i32) -> QueueState {
        let waiting_indices: Vec<usize> = self
            .state
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.status == QueueEntryStatus::Waiting)
            .map(|(i, _)| i)
            .collect();
        if let Some(pos) = waiting_indices
            .iter()
            .position(|&i| self.state.entries[i].user_id == user_id)
        {
            let target_pos = (pos as i32 + delta).clamp(0, waiting_indices.len() as i32 - 1) as usize;
            if target_pos != pos {
                let idx_a = waiting_indices[pos];
                let idx_b = waiting_indices[target_pos];
                // 先着順モードでは joined_at_ms も交換しておき、次回参加時の sort_entries() で手動順が戻らないようにする
                if self.state.config.sort_mode == QueueSortMode::FirstCome {
                    let ta = self.state.entries[idx_a].joined_at_ms;
                    let tb = self.state.entries[idx_b].joined_at_ms;
                    if ta != tb {
                        self.state.entries[idx_a].joined_at_ms = tb;
                        self.state.entries[idx_b].joined_at_ms = ta;
                    } else if target_pos < pos {
                        self.state.entries[idx_a].joined_at_ms = ta.saturating_sub(1);
                    } else {
                        self.state.entries[idx_a].joined_at_ms = ta.saturating_add(1);
                    }
                }
                self.state.entries.swap(idx_a, idx_b);
            }
        }
        self.state.clone()
    }

    pub fn remove_entry(&mut self, user_id: &str) -> QueueState {
        self.state.entries.retain(|e| e.user_id != user_id);
        self.state.clone()
    }

    pub fn reset(&mut self, reset_play_counts: bool) -> QueueState {
        self.state.entries.clear();
        if reset_play_counts {
            self.play_counts.clear();
        }
        self.state.clone()
    }

    pub fn sort_entries(&mut self) {
        let mut active = Vec::new();
        let mut waiting = Vec::new();
        let mut completed = Vec::new();

        for e in self.state.entries.drain(..) {
            match e.status {
                QueueEntryStatus::Active => active.push(e),
                QueueEntryStatus::Waiting => waiting.push(e),
                QueueEntryStatus::Completed => completed.push(e),
            }
        }

        match self.state.config.sort_mode {
            QueueSortMode::FirstCome => {
                waiting.sort_by_key(|e| e.joined_at_ms);
            }
            QueueSortMode::FewestPlaysFirst => {
                waiting.sort_by_key(|e| (e.play_count, e.joined_at_ms));
            }
            QueueSortMode::FirstTimerFirst => {
                waiting.sort_by_key(|e| (!e.is_first_time, e.play_count, e.joined_at_ms));
            }
        }

        self.state.entries.extend(active);
        self.state.entries.extend(waiting);
        self.state.entries.extend(completed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_join_priority_rotate_and_leave() {
        let mut rt = QueueRuntime::default();
        let cfg = QueueConfig {
            is_accepting: true,
            max_active_slots: 2,
            sort_mode: QueueSortMode::FirstTimerFirst,
            ..Default::default()
        };
        rt.update_config(cfg);

        // 常連2人と初見1人が順番に参加希望
        assert!(rt.process_comment("u1", PlatformKind::Demo, "常連A", "", false, false, "参加希望！", 100));
        assert!(rt.process_comment("u2", PlatformKind::Demo, "常連B", "", false, false, "参加希望です", 200));
        assert!(rt.process_comment("u3", PlatformKind::Demo, "初見C", "", false, true, "!join", 300));
        // BAN ユーザーは拒否
        assert!(!rt.process_comment("u-ban", PlatformKind::Demo, "荒らし", "", true, true, "参加希望", 350));

        // 初見優先なので先頭は 初見C -> 常連A -> 常連B
        let waiting_ids: Vec<&str> = rt
            .state
            .entries
            .iter()
            .filter(|e| e.status == QueueEntryStatus::Waiting)
            .map(|e| e.user_id.as_str())
            .collect();
        assert_eq!(waiting_ids, vec!["u3", "u1", "u2"]);

        // 2枠分繰り上げ -> u3 と u1 が参加中に
        rt.promote_next();
        let active_ids: Vec<&str> = rt
            .state
            .entries
            .iter()
            .filter(|e| e.status == QueueEntryStatus::Active)
            .map(|e| e.user_id.as_str())
            .collect();
        assert_eq!(active_ids, vec!["u3", "u1"]);

        // 交代（rotate） -> u3・u1 は完了（回数+1）、u2 が参加中へ
        rt.rotate_active();
        assert_eq!(*rt.play_counts.get("u3").unwrap(), 1);
        assert_eq!(*rt.play_counts.get("u1").unwrap(), 1);
        let active_after: Vec<&str> = rt
            .state
            .entries
            .iter()
            .filter(|e| e.status == QueueEntryStatus::Active)
            .map(|e| e.user_id.as_str())
            .collect();
        assert_eq!(active_after, vec!["u2"]);

        // 辞退コマンドで u2 が離脱
        assert!(rt.process_comment("u2", PlatformKind::Demo, "常連B", "", false, false, "辞退します", 400));
        assert!(rt
            .state
            .entries
            .iter()
            .all(|e| e.user_id != "u2"));
    }
}
