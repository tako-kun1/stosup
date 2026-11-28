use std::collections::HashMap;

use crate::engine::matches_pattern;
use crate::models::{
    MediaEffectType, MediaReactionRule, MediaTriggerEvent, MediaTriggerType, MonetizationEventKind,
};

/// WordParty 演出判定に渡すコメントコンテキスト
pub struct MediaEvalContext<'a> {
    pub user_name: &'a str,
    pub is_banned: bool,
    pub is_first_time: bool,
    pub message: &'a str,
    pub event_kind: Option<MonetizationEventKind>,
    pub amount_micros: u64,
    pub now_ms: i64,
    pub allow_multiple_match: bool,
}

/// 特定ワード反応演出（WordParty）の連打防止クールダウン管理
#[derive(Debug, Default)]
pub struct MediaCooldownTracker {
    last_triggered_ms: HashMap<String, i64>,
}

impl MediaCooldownTracker {
    pub fn evaluate_rules(
        &mut self,
        rules: &[MediaReactionRule],
        ctx: &MediaEvalContext<'_>,
    ) -> Vec<MediaTriggerEvent> {
        let mut triggers = Vec::new();
        for rule in rules {
            if !rule.enabled || !rule_has_playable_content(rule) {
                continue;
            }
            if rule.exclude_banned && ctx.is_banned {
                continue;
            }
            if !matches_trigger(rule, ctx) {
                continue;
            }

            let cd_ms = (rule.cooldown_sec as i64) * 1000;
            if let Some(&last) = self.last_triggered_ms.get(&rule.id) {
                if cd_ms > 0 && ctx.now_ms - last < cd_ms {
                    continue;
                }
            }
            self.last_triggered_ms.insert(rule.id.clone(), ctx.now_ms);
            triggers.push(build_trigger_event(
                rule,
                ctx.user_name,
                ctx.message,
                ctx.now_ms,
            ));

            if !ctx.allow_multiple_match {
                break;
            }
        }
        triggers
    }
}

/// ルールに再生可能なビジュアル・パーティクル・音声・テロップのいずれかがあるか確認
pub fn rule_has_playable_content(rule: &MediaReactionRule) -> bool {
    if !rule.stored_file_name.trim().is_empty() || !rule.sound_stored_file_name.trim().is_empty() {
        return true;
    }
    match rule.effect_type {
        // パーティクル・環境演出系は画像未指定でも絵文字/紙吹雪/雪などで再生可能
        MediaEffectType::Cracker
        | MediaEffectType::Drop
        | MediaEffectType::Confetti
        | MediaEffectType::Throw
        | MediaEffectType::Ambient => true,
        MediaEffectType::Fixed => !rule.caption_template.trim().is_empty(),
        MediaEffectType::Mod => {
            !rule.mod_id.trim().is_empty() && !rule.mod_effect_id.trim().is_empty()
        }
    }
}

fn matches_trigger(rule: &MediaReactionRule, ctx: &MediaEvalContext<'_>) -> bool {
    match rule.trigger_type {
        MediaTriggerType::AnyComment => true,
        MediaTriggerType::FirstTime => ctx.is_first_time,
        MediaTriggerType::SuperChat => {
            let is_sc = matches!(
                ctx.event_kind,
                Some(MonetizationEventKind::SuperChat | MonetizationEventKind::SuperSticker)
            );
            if !is_sc {
                return false;
            }
            if rule.min_superchat_amount > 0 {
                let amount_units = ctx.amount_micros / 1_000_000;
                if amount_units < rule.min_superchat_amount {
                    return false;
                }
            }
            true
        }
        MediaTriggerType::Membership => matches!(
            ctx.event_kind,
            Some(
                MonetizationEventKind::MembershipNew
                    | MonetizationEventKind::MembershipMilestone
                    | MonetizationEventKind::MembershipGift
            )
        ),
        MediaTriggerType::Keyword => {
            // WordParty と同様に 1行1パターン（空欄時はすべてのコメントに反応・Vec 確保なしの単一走査）
            let mut has_pattern = false;
            for raw_line in rule.pattern.lines() {
                let pat = raw_line.trim();
                if pat.is_empty() {
                    continue;
                }
                has_pattern = true;
                if matches_pattern(ctx.message, pat, rule.match_mode, rule.case_sensitive) {
                    return true;
                }
            }
            !has_pattern
        }
    }
}

pub fn build_trigger_event(
    rule: &MediaReactionRule,
    user_name: &str,
    message: &str,
    now_ms: i64,
) -> MediaTriggerEvent {
    let caption = rule
        .caption_template
        .replace("{user}", user_name)
        .replace("{message}", message);
    let media_url = if rule.stored_file_name.trim().is_empty() {
        String::new()
    } else {
        format!("/media/{}", rule.stored_file_name)
    };
    let sound_url = if rule.sound_stored_file_name.trim().is_empty() {
        None
    } else {
        Some(format!("/media/{}", rule.sound_stored_file_name))
    };

    MediaTriggerEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        effect_type: rule.effect_type,
        media_url,
        media_kind: rule.media_kind,
        sound_url,
        sound_volume: rule.sound_volume.min(100),
        animation: rule.animation,
        position: rule.position,
        duration_ms: rule.duration_ms,
        width_percent: rule.width_percent.clamp(10, 100),
        volume: rule.volume.min(100),
        particle_count: rule.particle_count.clamp(1, 120),
        particle_size: rule.particle_size.clamp(20, 260),
        particle_speed: rule.particle_speed.clamp(30, 250),
        gravity: rule.gravity.clamp(0, 200),
        particle_text: rule.particle_text.clone(),
        ambient_preset: rule.ambient_preset.clone(),
        caption,
        triggered_by: user_name.to_string(),
        triggered_at_ms: now_ms,
        mod_id: rule.mod_id.clone(),
        mod_effect_id: rule.mod_effect_id.clone(),
        mod_params_json: if rule.mod_params_json.trim().is_empty() {
            "{}".to_string()
        } else {
            rule.mod_params_json.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MatchMode, MediaAnimationKind, MediaKind, MediaPosition};

    fn base_rule(id: &str, pattern: &str) -> MediaReactionRule {
        MediaReactionRule {
            id: id.to_string(),
            name: "テスト演出".to_string(),
            pattern: pattern.to_string(),
            match_mode: MatchMode::Contains,
            case_sensitive: false,
            effect_type: MediaEffectType::Cracker,
            trigger_type: MediaTriggerType::Keyword,
            min_superchat_amount: 0,
            priority: 10,
            media_kind: MediaKind::Image,
            stored_file_name: String::new(),
            original_file_name: String::new(),
            sound_stored_file_name: "se.mp3".to_string(),
            sound_original_file_name: "se.mp3".to_string(),
            sound_volume: 85,
            animation: MediaAnimationKind::PopIn,
            position: MediaPosition::Center,
            duration_ms: 4000,
            width_percent: 45,
            volume: 80,
            particle_count: 30,
            particle_size: 64,
            particle_speed: 100,
            gravity: 100,
            particle_text: "🎉,✨".to_string(),
            ambient_preset: "snow".to_string(),
            cooldown_sec: 5,
            exclude_banned: true,
            caption_template: "{user} さんが発動！".to_string(),
            enabled: true,
            mod_id: String::new(),
            mod_effect_id: String::new(),
            mod_params_json: "{}".to_string(),
        }
    }

    #[test]
    fn test_media_reaction_matching_and_cooldown() {
        let rule = base_rule("mr-1", "ナイス\n888");
        let mut tracker = MediaCooldownTracker::default();

        // BAN ユーザーは発動しない
        assert!(tracker
            .evaluate_rules(
                &[rule.clone()],
                &MediaEvalContext {
                    user_name: "荒らし",
                    is_banned: true,
                    is_first_time: false,
                    message: "ナイス！",
                    event_kind: None,
                    amount_micros: 0,
                    now_ms: 10_000,
                    allow_multiple_match: true,
                }
            )
            .is_empty());

        // 複数行パターンの2行目「888」で発動・SE URL 付き
        let t1 = tracker.evaluate_rules(
            &[rule.clone()],
            &MediaEvalContext {
                user_name: "太郎",
                is_banned: false,
                is_first_time: false,
                message: "888888",
                event_kind: None,
                amount_micros: 0,
                now_ms: 10_000,
                allow_multiple_match: true,
            },
        );
        assert_eq!(t1.len(), 1);
        assert_eq!(t1[0].effect_type, MediaEffectType::Cracker);
        assert_eq!(t1[0].sound_url.as_deref(), Some("/media/se.mp3"));
        assert_eq!(t1[0].caption, "太郎 さんが発動！");

        // クールダウン中（2秒後）は発動しない
        assert!(tracker
            .evaluate_rules(
                &[rule.clone()],
                &MediaEvalContext {
                    user_name: "花子",
                    is_banned: false,
                    is_first_time: false,
                    message: "ナイス",
                    event_kind: None,
                    amount_micros: 0,
                    now_ms: 12_000,
                    allow_multiple_match: true,
                }
            )
            .is_empty());

        // クールダウン経過後（5秒後）は再発動
        let t2 = tracker.evaluate_rules(
            &[rule],
            &MediaEvalContext {
                user_name: "花子",
                is_banned: false,
                is_first_time: false,
                message: "ナイス",
                event_kind: None,
                amount_micros: 0,
                now_ms: 15_000,
                allow_multiple_match: true,
            },
        );
        assert_eq!(t2.len(), 1);
    }

    #[test]
    fn test_wordparty_triggers_and_single_vs_multiple_match() {
        let mut sc_rule = base_rule("sc", "");
        sc_rule.trigger_type = MediaTriggerType::SuperChat;
        sc_rule.min_superchat_amount = 1000;
        sc_rule.effect_type = MediaEffectType::Confetti;
        sc_rule.cooldown_sec = 0;

        let mut ft_rule = base_rule("ft", "");
        ft_rule.trigger_type = MediaTriggerType::FirstTime;
        ft_rule.effect_type = MediaEffectType::Drop;
        ft_rule.cooldown_sec = 0;

        let rules = vec![sc_rule, ft_rule];
        let mut tracker = MediaCooldownTracker::default();

        // 500円スパチャは min=1000 未満なので不発、初見のみ発動
        let res1 = tracker.evaluate_rules(
            &rules,
            &MediaEvalContext {
                user_name: "新人",
                is_banned: false,
                is_first_time: true,
                message: "初見です！",
                event_kind: Some(MonetizationEventKind::SuperChat),
                amount_micros: 500_000_000,
                now_ms: 1_000,
                allow_multiple_match: true,
            },
        );
        assert_eq!(res1.len(), 1);
        assert_eq!(res1[0].rule_id, "ft");

        // 1000円スパチャ + 初見: 複数判定 ON なら両方発動
        let res2 = tracker.evaluate_rules(
            &rules,
            &MediaEvalContext {
                user_name: "太客",
                is_banned: false,
                is_first_time: true,
                message: "応援！",
                event_kind: Some(MonetizationEventKind::SuperChat),
                amount_micros: 1_000_000_000,
                now_ms: 2_000,
                allow_multiple_match: true,
            },
        );
        assert_eq!(res2.len(), 2);

        // 複数判定 OFF なら先頭（優先順位上位）の1件のみ発動
        let res3 = tracker.evaluate_rules(
            &rules,
            &MediaEvalContext {
                user_name: "太客",
                is_banned: false,
                is_first_time: true,
                message: "応援！",
                event_kind: Some(MonetizationEventKind::SuperChat),
                amount_micros: 2_000_000_000,
                now_ms: 3_000,
                allow_multiple_match: false,
            },
        );
        assert_eq!(res3.len(), 1);
        assert_eq!(res3[0].rule_id, "sc");
    }
}
