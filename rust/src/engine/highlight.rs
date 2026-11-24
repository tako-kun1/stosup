use crate::engine::matches_pattern;
use crate::models::HighlightRule;

/// コメント本文に対して優先度の高い一致ルールを返す
pub fn evaluate_highlight<'a>(
    message: &str,
    rules: &'a [HighlightRule],
) -> Option<&'a HighlightRule> {
    let mut matched: Option<&HighlightRule> = None;
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        if matches_pattern(message, &rule.pattern, rule.match_mode, rule.case_sensitive) {
            match matched {
                None => matched = Some(rule),
                Some(prev) if rule.priority > prev.priority => matched = Some(rule),
                _ => {}
            }
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MatchMode;

    #[test]
    fn test_highlight_priority_and_modes() {
        let rules = vec![
            HighlightRule {
                id: "low".to_string(),
                name: "低優先".to_string(),
                pattern: "おめでとう".to_string(),
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                bg_color: "#111111".to_string(),
                text_color: "#FFFFFF".to_string(),
                border_color: "#222222".to_string(),
                priority: 1,
                enabled: true,
            },
            HighlightRule {
                id: "high".to_string(),
                name: "高優先".to_string(),
                pattern: "^誕生日.*おめでとう$".to_string(),
                match_mode: MatchMode::Regex,
                case_sensitive: false,
                bg_color: "#FF0000".to_string(),
                text_color: "#FFFFFF".to_string(),
                border_color: "#FFFF00".to_string(),
                priority: 10,
                enabled: true,
            },
            HighlightRule {
                id: "disabled".to_string(),
                name: "無効".to_string(),
                pattern: "誕生日".to_string(),
                match_mode: MatchMode::Contains,
                case_sensitive: false,
                bg_color: "#00FF00".to_string(),
                text_color: "#FFFFFF".to_string(),
                border_color: "#00FF00".to_string(),
                priority: 100,
                enabled: false,
            },
        ];

        let r1 = evaluate_highlight("誕生日おめでとう", &rules).unwrap();
        assert_eq!(r1.id, "high");

        let r2 = evaluate_highlight("昇格おめでとう！", &rules).unwrap();
        assert_eq!(r2.id, "low");

        assert!(evaluate_highlight("普通のコメント", &rules).is_none());
    }
}
