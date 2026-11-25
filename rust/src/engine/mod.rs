pub mod counter;
pub mod highlight;
pub mod lottery;
pub mod media_reaction;
pub mod poll;
pub mod queue;

use regex::RegexBuilder;

use crate::models::MatchMode;

/// 共通テキストマッチング判定関数
pub fn matches_pattern(
    text: &str,
    pattern: &str,
    mode: MatchMode,
    case_sensitive: bool,
) -> bool {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return false;
    }
    match mode {
        MatchMode::Contains => {
            if case_sensitive {
                text.contains(trimmed)
            } else {
                text.to_lowercase().contains(&trimmed.to_lowercase())
            }
        }
        MatchMode::Exact => {
            if case_sensitive {
                text.trim() == trimmed
            } else {
                text.trim().to_lowercase() == trimmed.to_lowercase()
            }
        }
        MatchMode::Regex => RegexBuilder::new(trimmed)
            .case_insensitive(!case_sensitive)
            .build()
            .map(|re| re.is_match(text))
            .unwrap_or(false),
    }
}
