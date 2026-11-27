use std::borrow::Cow;

use crate::engine::{get_or_compile_regex, is_case_invariant_str, matches_pattern};
use crate::models::{MatchMode, MessageRun, MessageRunKind, NgWordAction, NgWordRule};

/// NG ワード評価結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NgWordEvaluation {
    /// true の場合、アプリ・OBS・各種集計すべてからコメントを除外する
    pub drop_everywhere: bool,
    /// true の場合、OBS オーバーレイおよびスポットテロップへは流さない
    pub hide_from_overlay: bool,
    /// 伏せ字マスクが適用されたか
    pub masked: bool,
    /// 処理後の本文
    pub message: String,
    /// 処理後の絵文字混在ラン
    pub message_runs: Vec<MessageRun>,
}

/// インプレース評価のフラグ結果（非ヒット時の不要な文字列・配列クローンを回避）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NgWordFlags {
    pub drop_everywhere: bool,
    pub hide_from_overlay: bool,
    pub masked: bool,
}

fn replace_pattern_in_text(text: &str, rule: &NgWordRule) -> Option<String> {
    let pattern = rule.pattern.trim();
    if pattern.is_empty() {
        return None;
    }
    let repl = if rule.replacement.is_empty() {
        "***"
    } else {
        rule.replacement.as_str()
    };

    match rule.match_mode {
        MatchMode::Exact => {
            if matches_pattern(text, pattern, MatchMode::Exact, rule.case_sensitive) {
                Some(repl.to_string())
            } else {
                None
            }
        }
        MatchMode::Contains => {
            if rule.case_sensitive || is_case_invariant_str(pattern) {
                if text.contains(pattern) {
                    Some(text.replace(pattern, repl))
                } else {
                    None
                }
            } else {
                // 正規表現キャッシュを共有して毎回の再コンパイルを回避（1パス置換）
                let escaped = regex::escape(pattern);
                if let Some(re) = get_or_compile_regex(&escaped, false) {
                    if let Cow::Owned(replaced) = re.replace_all(text, regex::NoExpand(repl)) {
                        return Some(replaced);
                    }
                }
                None
            }
        }
        MatchMode::Regex => {
            if let Some(re) = get_or_compile_regex(pattern, rule.case_sensitive) {
                if let Cow::Owned(replaced) = re.replace_all(text, repl) {
                    return Some(replaced);
                }
            }
            None
        }
    }
}

/// 受信コメントの本文・ランをインプレースで評価する（非ヒット時はゼロアロケーション）
pub fn evaluate_ng_words_in_place(
    message: &mut String,
    runs: &mut Vec<MessageRun>,
    rules: &[NgWordRule],
) -> NgWordFlags {
    let mut flags = NgWordFlags::default();

    for rule in rules {
        if !rule.enabled || rule.pattern.trim().is_empty() {
            continue;
        }
        if !matches_pattern(message, &rule.pattern, rule.match_mode, rule.case_sensitive) {
            continue;
        }
        match rule.action {
            NgWordAction::HideEverywhere => {
                message.clear();
                runs.clear();
                return NgWordFlags {
                    drop_everywhere: true,
                    hide_from_overlay: true,
                    masked: false,
                };
            }
            NgWordAction::HideOnOverlay => {
                flags.hide_from_overlay = true;
            }
            NgWordAction::MaskText => {
                if let Some(next_msg) = replace_pattern_in_text(message, rule) {
                    flags.masked = true;
                    *message = next_msg;
                    if !runs.is_empty() {
                        // 各テキストランにも伏せ字置換をインプレース適用（絵文字境界を跨ぐ場合は安全にプレーンテキストへ統合）
                        let mut any_run_changed = false;
                        for r in runs.iter_mut() {
                            if r.kind == MessageRunKind::Text {
                                if let Some(replaced) = replace_pattern_in_text(&r.text, rule) {
                                    r.text = replaced;
                                    any_run_changed = true;
                                }
                            }
                        }
                        if !any_run_changed {
                            runs.clear();
                            runs.push(MessageRun {
                                kind: MessageRunKind::Text,
                                text: message.clone(),
                                emoji_url: None,
                                alt: None,
                                is_custom_emoji: false,
                            });
                        }
                    }
                }
            }
        }
    }

    flags
}

/// 受信コメントに対して NG ワードルール群を評価する
pub fn evaluate_ng_words(
    message: &str,
    runs: &[MessageRun],
    rules: &[NgWordRule],
) -> NgWordEvaluation {
    let mut current_msg = message.to_string();
    let mut current_runs = runs.to_vec();
    let flags = evaluate_ng_words_in_place(&mut current_msg, &mut current_runs, rules);
    NgWordEvaluation {
        drop_everywhere: flags.drop_everywhere,
        hide_from_overlay: flags.hide_from_overlay,
        masked: flags.masked,
        message: current_msg,
        message_runs: current_runs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_rule(pattern: &str, mode: MatchMode, action: NgWordAction) -> NgWordRule {
        NgWordRule {
            id: "r1".to_string(),
            pattern: pattern.to_string(),
            match_mode: mode,
            case_sensitive: false,
            action,
            replacement: "***".to_string(),
            enabled: true,
        }
    }

    #[test]
    fn test_ng_word_hide_everywhere_overlay_and_mask() {
        let rules = vec![
            mk_rule("ネタバレ", MatchMode::Contains, NgWordAction::HideOnOverlay),
            mk_rule("badword", MatchMode::Contains, NgWordAction::MaskText),
            mk_rule("荒らしURL", MatchMode::Contains, NgWordAction::HideEverywhere),
        ];

        let r1 = evaluate_ng_words("ここに荒らしURLがあります", &[], &rules);
        assert!(r1.drop_everywhere);

        let r2 = evaluate_ng_words("ボスのネタバレ注意！", &[], &rules);
        assert!(!r2.drop_everywhere);
        assert!(r2.hide_from_overlay);
        assert!(!r2.masked);

        let r3 = evaluate_ng_words("これは BADWORD です", &[], &rules);
        assert!(!r3.drop_everywhere);
        assert!(!r3.hide_from_overlay);
        assert!(r3.masked);
        assert_eq!(r3.message, "これは *** です");
    }

    #[test]
    fn test_ng_word_mask_preserves_emoji_runs() {
        let rules = vec![mk_rule("NG", MatchMode::Contains, NgWordAction::MaskText)];
        let runs = vec![
            MessageRun {
                kind: MessageRunKind::Text,
                text: "これNGだよ ".to_string(),
                emoji_url: None,
                alt: None,
                is_custom_emoji: false,
            },
            MessageRun {
                kind: MessageRunKind::Emoji,
                text: ":smile:".to_string(),
                emoji_url: Some("https://example.com/smile.png".to_string()),
                alt: Some(":smile:".to_string()),
                is_custom_emoji: true,
            },
        ];
        let res = evaluate_ng_words("これNGだよ :smile:", &runs, &rules);
        assert!(res.masked);
        assert_eq!(res.message, "これ***だよ :smile:");
        assert_eq!(res.message_runs.len(), 2);
        assert_eq!(res.message_runs[0].text, "これ***だよ ");
        assert_eq!(res.message_runs[1].kind, MessageRunKind::Emoji);
    }

    #[test]
    fn test_ng_word_in_place_no_match_and_regex_cache_and_dollar_literal() {
        let mut r_contains = mk_rule("bad", MatchMode::Contains, NgWordAction::MaskText);
        r_contains.replacement = "$$$".to_string();
        let r_regex = mk_rule(r"\d{4,}", MatchMode::Regex, NgWordAction::MaskText);
        let rules = vec![r_contains, r_regex];

        // 非ヒット時は文字列もランもそのまま保持される
        let mut msg_ok = "通常のコメントです".to_string();
        let mut runs_ok = Vec::new();
        let f_ok = evaluate_ng_words_in_place(&mut msg_ok, &mut runs_ok, &rules);
        assert!(!f_ok.drop_everywhere && !f_ok.hide_from_overlay && !f_ok.masked);
        assert_eq!(msg_ok, "通常のコメントです");

        // Contains 大文字小文字無視の置換で $$$ がリテラルとして展開されず適用される
        let mut msg_bad = "これは BAD な発言 12345 です".to_string();
        let mut runs_bad = Vec::new();
        let f_bad = evaluate_ng_words_in_place(&mut msg_bad, &mut runs_bad, &rules);
        assert!(f_bad.masked);
        assert_eq!(msg_bad, "これは $$$ な発言 *** です");

        // 日本語（大文字小文字なし）パターンの高速置換経路
        let jp_rules = vec![mk_rule("ネタバレ", MatchMode::Contains, NgWordAction::MaskText)];
        let mut msg_jp = "ラスボスのネタバレ注意！ネタバレ禁止".to_string();
        let mut runs_jp = Vec::new();
        let f_jp = evaluate_ng_words_in_place(&mut msg_jp, &mut runs_jp, &jp_rules);
        assert!(f_jp.masked);
        assert_eq!(msg_jp, "ラスボスの***注意！***禁止");
    }
}
