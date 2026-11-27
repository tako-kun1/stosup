pub mod counter;
pub mod highlight;
pub mod lottery;
pub mod media_reaction;
pub mod ng_word;
pub mod poll;
pub mod queue;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};

use crate::models::MatchMode;

/// コンパイル済み正規表現のキャッシュ（大文字小文字区別あり・なしを分離し `&str` ゼロアロケーション検索に対応）
const MAX_REGEX_CACHE_ENTRIES: usize = 512;
static REGEX_CACHE_CS: Lazy<RwLock<HashMap<String, Option<Arc<Regex>>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static REGEX_CACHE_CI: Lazy<RwLock<HashMap<String, Option<Arc<Regex>>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Unicode 仕様上、大文字・小文字の区別を持たず他の文字の小文字化結果にもならない安全な文字集合
#[inline]
fn is_case_invariant_char(c: char) -> bool {
    (c.is_ascii() && !c.is_ascii_alphabetic())
        || ('\u{3000}'..='\u{30FF}').contains(&c)
        || ('\u{4E00}'..='\u{9FFF}').contains(&c)
        || ('\u{3400}'..='\u{4DBF}').contains(&c)
        || ('\u{FF01}'..='\u{FF20}').contains(&c)
        || ('\u{FF3B}'..='\u{FF40}').contains(&c)
        || ('\u{FF5B}'..='\u{FF65}').contains(&c)
        || ('\u{1F000}'..='\u{1FAFF}').contains(&c)
}

/// 文字列全体が大文字・小文字の区別を持たない文字のみで構成されるか判定する
#[inline]
pub(crate) fn is_case_invariant_str(s: &str) -> bool {
    s.chars().all(is_case_invariant_char)
}

/// ASCII バイト列の高速な大文字小文字無視部分一致（ヒープ確保なし）
#[inline]
fn contains_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

/// キャッシュからコンパイル済み正規表現を取得する（キャッシュヒット時はヒープ確保なし）
pub fn get_or_compile_regex(pattern: &str, case_sensitive: bool) -> Option<Arc<Regex>> {
    let cache_lock = if case_sensitive {
        &REGEX_CACHE_CS
    } else {
        &REGEX_CACHE_CI
    };

    if let Ok(cache) = cache_lock.read() {
        if let Some(entry) = cache.get(pattern) {
            return entry.clone();
        }
    }

    let compiled = RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .build()
        .ok()
        .map(Arc::new);

    if let Ok(mut cache) = cache_lock.write() {
        if cache.len() >= MAX_REGEX_CACHE_ENTRIES {
            cache.clear();
        }
        cache.insert(pattern.to_string(), compiled.clone());
    }
    compiled
}

fn match_regex_cached(text: &str, pattern: &str, case_sensitive: bool) -> bool {
    get_or_compile_regex(pattern, case_sensitive)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

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
            if case_sensitive || trimmed.chars().all(is_case_invariant_char) {
                text.contains(trimmed)
            } else if trimmed.is_ascii()
                && text.chars().all(|c| c.is_ascii() || is_case_invariant_char(c))
            {
                contains_ascii_ignore_case(text.as_bytes(), trimmed.as_bytes())
            } else {
                text.to_lowercase().contains(&trimmed.to_lowercase())
            }
        }
        MatchMode::Exact => {
            let text_trimmed = text.trim();
            if case_sensitive || trimmed.chars().all(is_case_invariant_char) {
                text_trimmed == trimmed
            } else if trimmed.is_ascii()
                && text_trimmed
                    .chars()
                    .all(|c| c.is_ascii() || is_case_invariant_char(c))
            {
                text_trimmed.eq_ignore_ascii_case(trimmed)
            } else {
                text_trimmed.to_lowercase() == trimmed.to_lowercase()
            }
        }
        MatchMode::Regex => match_regex_cached(text, trimmed, case_sensitive),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_fast_paths_and_unicode() {
        // 日本語・絵文字のゼロアロケーション経路
        assert!(matches_pattern("大草原www", "草", MatchMode::Contains, false));
        assert!(matches_pattern("初見です！🎉", "🎉", MatchMode::Contains, false));
        assert!(matches_pattern("  初見  ", "初見", MatchMode::Exact, false));

        // 日本語＋ASCII 混在に対する大文字小文字無視マッチ
        assert!(matches_pattern("マジでWWW面白すぎ", "www", MatchMode::Contains, false));
        assert!(matches_pattern("  !JOIN  ", "!join", MatchMode::Exact, false));
        assert!(!matches_pattern("マジでWWW面白すぎ", "www", MatchMode::Contains, true));

        // 全角英字などの Unicode フォールバック経路
        assert!(matches_pattern("ＡＢＣテスト", "ａｂｃ", MatchMode::Contains, false));
        assert!(matches_pattern("  ＡＢＣ  ", "ａｂｃ", MatchMode::Exact, false));

        // 正規表現キャッシュ経路（繰り返し呼び出し＆不正パターン耐性）
        assert!(matches_pattern("888888", r"^8{3,}$", MatchMode::Regex, false));
        assert!(matches_pattern("888888", r"^8{3,}$", MatchMode::Regex, false));
        assert!(matches_pattern("Hello World", r"hello\s+world", MatchMode::Regex, false));
        assert!(!matches_pattern("Hello World", r"hello\s+world", MatchMode::Regex, true));
        assert!(!matches_pattern("abc", r"[invalid(", MatchMode::Regex, false));
    }
}
