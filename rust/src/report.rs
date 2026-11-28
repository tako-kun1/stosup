use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;

use crate::db::{Database, SessionCommentRow};
use crate::models::{
    OnAirReportDto, ReportMinuteBucket, ReportPeakItem, ReportUserRank, ReportWordRank,
    StreamSessionSummary,
};

/// 頻出ワード集計から除外する短いノイズ語（O(1) 判定用 HashSet）
static STOP_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "の", "に", "は", "を", "た", "が", "で", "て", "と", "し", "れ", "さ", "ある", "いる",
        "も", "する", "から", "な", "こと", "として", "い", "や", "れる", "など", "なっ", "ない",
        "この", "ため", "その", "あっ", "よう", "また", "もの", "という", "あり", "まで", "られ",
        "なる", "へ", "か", "だ", "これ", "によって", "により", "おり", "より", "による", "ず",
        "なり", "られる", "において", "ば", "なかっ", "なく", "しかし", "について", "せ", "だっ",
        "です", "ます", "ました", "ですね", "ですよ", "the", "and", "for", "that", "this", "with",
    ]
    .into_iter()
    .collect()
});

fn format_elapsed(minute_offset: u32) -> String {
    let h = minute_offset / 60;
    let m = minute_offset % 60;
    format!("{:02}:{:02}:00", h, m)
}

fn collect_tokens_into<'a>(message: &'a str, out: &mut HashSet<Cow<'a, str>>) {
    out.clear();
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return;
    }

    // 空白・句読点・記号で区切ってトークン化（不要な Vec 確保・重複 String 複製を排除）
    for raw in trimmed.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '、' | '。' | '！' | '？' | '!' | '?' | ',' | '.' | '（' | '）' | '(' | ')' | '「' | '」' | '【' | '】' | '・' | '/' | '〜' | '~'
            )
    }) {
        let w = raw.trim();
        if w.is_empty() {
            continue;
        }
        // 長すぎる文章そのままではなく、2〜18文字（または単独「草」や「w」連続）を抽出
        let char_len = w.chars().count();
        if w == "草" || w.chars().all(|c| c == 'w' || c == 'W' || c == 'ｗ' || c == 'Ｗ') {
            if w == "草" {
                out.insert(Cow::Borrowed("草"));
            } else if char_len >= 2 {
                out.insert(Cow::Borrowed("www"));
            }
            continue;
        }
        if w.chars().all(|c| c == '8' || c == '８') && char_len >= 3 {
            out.insert(Cow::Borrowed("888"));
            continue;
        }
        if !(2..=18).contains(&char_len) {
            continue;
        }
        // ASCII 大文字を含まない日本語トークンは to_lowercase() すら省略可能
        if w.bytes().any(|b| b.is_ascii_uppercase()) {
            let lower = w.to_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                continue;
            }
            if STOP_WORDS.contains(lower.as_str()) {
                continue;
            }
        } else {
            if w.starts_with("http://") || w.starts_with("https://") {
                continue;
            }
            if STOP_WORDS.contains(w) {
                continue;
            }
        }
        out.insert(Cow::Borrowed(w));
    }
}

/// 取得済みセッションサマリー・コメント行からオンエアレポートを純粋計算で構築する（DB ロック不要）
pub fn build_on_air_report_from_rows(
    summary: StreamSessionSummary,
    rows: &[SessionCommentRow],
) -> OnAirReportDto {
    let start_ms = rows
        .first()
        .map(|r| r.timestamp_ms.min(summary.started_at_ms))
        .unwrap_or(summary.started_at_ms);
    let end_ms = rows
        .last()
        .map(|r| r.timestamp_ms)
        .unwrap_or(start_ms);
    let duration_minutes = (((end_ms - start_ms).max(0) / 60_000) as u32) + 1;

    let mut bucket_map: BTreeMap<u32, ReportMinuteBucket> = BTreeMap::new();
    let mut word_counts: HashMap<&str, u64> = HashMap::new();
    let mut user_counts: HashMap<&str, (&str, u64, bool)> = HashMap::new();
    let mut currency_totals: BTreeMap<&str, u64> = BTreeMap::new();
    let mut member_comment_count = 0u64;
    let mut seen_in_msg: HashSet<Cow<'_, str>> = HashSet::with_capacity(16);

    for row in rows {
        let offset = ((row.timestamp_ms - start_ms).max(0) / 60_000) as u32;
        let b = bucket_map.entry(offset).or_insert_with(|| ReportMinuteBucket {
            minute_offset: offset,
            timestamp_ms: start_ms + (offset as i64) * 60_000,
            comment_count: 0,
            monetization_count: 0,
            first_time_count: 0,
            sample_messages: Vec::new(),
        });
        b.comment_count += 1;
        if row.event_kind.is_some() {
            b.monetization_count += 1;
        }
        if row.is_first_time {
            b.first_time_count += 1;
        }
        if b.sample_messages.len() < 4 {
            let trimmed_msg = row.message.trim();
            if !trimmed_msg.is_empty() {
                b.sample_messages
                    .push(format!("{}: {}", row.user_name, trimmed_msg));
            }
        }

        if row.is_member {
            member_comment_count += 1;
        }
        if row.amount_micros > 0 {
            let cur = if row.currency.is_empty() {
                "JPY"
            } else {
                row.currency.as_str()
            };
            *currency_totals.entry(cur).or_insert(0) += row.amount_micros;
        }

        let u_entry = user_counts
            .entry(row.user_id.as_str())
            .or_insert_with(|| (row.user_name.as_str(), 0, row.is_member));
        u_entry.0 = row.user_name.as_str();
        u_entry.1 += 1;
        u_entry.2 |= row.is_member;

        collect_tokens_into(&row.message, &mut seen_in_msg);
        for tok in seen_in_msg.drain() {
            if let Cow::Borrowed(s) = tok {
                *word_counts.entry(s).or_insert(0) += 1;
            }
        }
    }

    // グラフが途切れないよう、最大 360 分までは空き分も 0 埋め（長時間配信は実データ分のみ）
    let mut buckets: Vec<ReportMinuteBucket> = Vec::with_capacity(
        if duration_minutes <= 360 {
            duration_minutes as usize
        } else {
            bucket_map.len()
        },
    );
    if duration_minutes <= 360 {
        for m in 0..duration_minutes {
            if let Some(b) = bucket_map.remove(&m) {
                buckets.push(b);
            } else {
                buckets.push(ReportMinuteBucket {
                    minute_offset: m,
                    timestamp_ms: start_ms + (m as i64) * 60_000,
                    comment_count: 0,
                    monetization_count: 0,
                    first_time_count: 0,
                    sample_messages: Vec::new(),
                });
            }
        }
    } else {
        buckets.extend(bucket_map.into_values());
    }

    let max_comments_per_min = buckets.iter().map(|b| b.comment_count).max().unwrap_or(0);
    let total_comments = rows.len() as u64;
    let avg_comments_per_min_tenths = if duration_minutes > 0 {
        ((total_comments * 10) / duration_minutes as u64) as u32
    } else {
        0
    };

    // 盛り上がりピーク TOP 5
    let mut peak_candidates: Vec<&ReportMinuteBucket> = buckets
        .iter()
        .filter(|b| b.comment_count > 0)
        .collect();
    peak_candidates.sort_by(|a, b| {
        b.comment_count
            .cmp(&a.comment_count)
            .then_with(|| a.minute_offset.cmp(&b.minute_offset))
    });
    let peaks: Vec<ReportPeakItem> = peak_candidates
        .into_iter()
        .take(5)
        .map(|b| ReportPeakItem {
            minute_offset: b.minute_offset,
            elapsed_label: format_elapsed(b.minute_offset),
            timestamp_ms: b.timestamp_ms,
            comment_count: b.comment_count,
            sample_messages: b.sample_messages.clone(),
        })
        .collect();

    // 頻出ワード TOP 15
    let mut words_vec: Vec<(&str, u64)> = word_counts.into_iter().collect();
    words_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top_words: Vec<ReportWordRank> = words_vec
        .into_iter()
        .take(15)
        .map(|(word, count)| ReportWordRank {
            word: word.to_string(),
            count,
        })
        .collect();

    // アクティブ視聴者 TOP 10
    let mut users_vec: Vec<(&str, &str, u64, bool)> = user_counts
        .into_iter()
        .map(|(uid, (name, cnt, mem))| (uid, name, cnt, mem))
        .collect();
    users_vec.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(b.1)));
    let top_users: Vec<ReportUserRank> = users_vec
        .into_iter()
        .take(10)
        .map(|(user_id, user_name, comment_count, is_member)| ReportUserRank {
            user_id: user_id.to_string(),
            user_name: user_name.to_string(),
            comment_count,
            is_member,
        })
        .collect();

    let monetization_totals: Vec<String> = currency_totals
        .into_iter()
        .map(|(cur, micros)| {
            let val = micros as f64 / 1_000_000.0;
            if cur == "JPY" {
                format!("¥{:.0}", val)
            } else {
                format!("{} {:.2}", cur, val)
            }
        })
        .collect();

    let mut session_summary: StreamSessionSummary = summary;
    session_summary.total_comments = total_comments;

    OnAirReportDto {
        session: session_summary,
        duration_minutes,
        avg_comments_per_min_tenths,
        max_comments_per_min,
        member_comment_count,
        monetization_totals,
        buckets,
        peaks,
        top_words,
        top_users,
    }
}

/// 指定セッションのオンエアレポートを集計生成する
pub fn build_on_air_report(db: &Database, session_id: &str) -> Result<OnAirReportDto> {
    let summary = db
        .get_stream_session_summary(session_id)?
        .ok_or_else(|| anyhow!("指定された配信セッションが見つかりません"))?;
    let rows = db.list_session_comments(session_id)?;
    Ok(build_on_air_report_from_rows(summary, &rows))
}

fn csv_escape(s: &str) -> Cow<'_, str> {
    // Excel / スプレッドシートの CSV Formula Injection 対策（先頭の数式開始文字を無害化）
    let needs_formula_guard = s.starts_with(['=', '+', '-', '@', '\t', '\r']);
    let needs_quotes =
        s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if needs_formula_guard {
        let guarded = format!("'{}", s);
        if needs_quotes {
            Cow::Owned(format!("\"{}\"", guarded.replace('"', "\"\"")))
        } else {
            Cow::Owned(guarded)
        }
    } else if needs_quotes {
        Cow::Owned(format!("\"{}\"", s.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(s)
    }
}

/// 取得済みコメント行を Excel 互換 UTF-8 BOM 付き CSV へ高速バッファ書き出しする（DB ロック不要）
pub fn write_session_csv_from_rows(rows: &[SessionCommentRow], dest_path: &Path) -> Result<String> {
    let file = File::create(dest_path)?;
    let mut f = BufWriter::with_capacity(64 * 1024, file);
    // UTF-8 BOM
    f.write_all(&[0xEF, 0xBB, 0xBF])?;
    writeln!(
        f,
        "timestamp_ms,elapsed_sec,user_id,user_name,is_first_time,is_member,event_kind,amount,message"
    )?;
    let start_ms = rows.first().map(|r| r.timestamp_ms).unwrap_or(0);
    for r in rows {
        let elapsed_sec = ((r.timestamp_ms - start_ms).max(0)) / 1000;
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{}",
            r.timestamp_ms,
            elapsed_sec,
            csv_escape(&r.user_id),
            csv_escape(&r.user_name),
            if r.is_first_time { "1" } else { "0" },
            if r.is_member { "1" } else { "0" },
            csv_escape(r.event_kind.as_deref().unwrap_or("")),
            csv_escape(&r.amount_display),
            csv_escape(&r.message),
        )?;
    }
    f.flush()?;
    Ok(dest_path.to_string_lossy().to_string())
}

/// 取得済みデータからレポート JSON を高速バッファ書き出しする（DB ロック不要）
pub fn write_session_json_from_rows(
    summary: StreamSessionSummary,
    rows: &[SessionCommentRow],
    dest_path: &Path,
) -> Result<String> {
    let report = build_on_air_report_from_rows(summary, rows);
    let file = File::create(dest_path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    serde_json::to_writer_pretty(&mut writer, &report)?;
    writer.flush()?;
    Ok(dest_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PlatformKind, RawIncomingComment};

    fn mk_raw(id: &str, uid: &str, name: &str, msg: &str, ts: i64) -> RawIncomingComment {
        RawIncomingComment {
            id: id.to_string(),
            platform: PlatformKind::YouTube,
            user_id: uid.to_string(),
            user_name: name.to_string(),
            user_icon_url: String::new(),
            message: msg.to_string(),
            message_runs: Vec::new(),
            timestamp_ms: ts,
            is_owner: false,
            is_moderator: false,
            is_member: uid == "u1",
            event_kind: None,
            amount_display: None,
            amount_micros: 0,
            currency: String::new(),
            tier_color: None,
        }
    }

    #[test]
    fn test_on_air_report_buckets_peaks_and_words() {
        let db = Database::open_in_memory().unwrap();
        let base = 1_700_000_000_000i64;
        let sid = db
            .get_or_create_session(PlatformKind::YouTube, "v-rep-1", "レポートテスト配信", base)
            .unwrap();

        let comments = vec![
            (mk_raw("c1", "u1", "太郎", "初見です 草", base + 1_000), true),
            (mk_raw("c2", "u2", "花子", "草 面白すぎ", base + 65_000), true),
            (mk_raw("c3", "u1", "太郎", "ナイスプレイ！ 草", base + 70_000), false),
            (mk_raw("c4", "u3", "三郎", "ナイスプレイ！ 8888", base + 75_000), false),
        ];
        for (c, ft) in comments {
            db.insert_session_comment(&sid, &c, ft).unwrap();
        }

        let rep = build_on_air_report(&db, &sid).unwrap();
        assert_eq!(rep.session.total_comments, 4);
        assert_eq!(rep.max_comments_per_min, 3);
        assert_eq!(rep.peaks[0].minute_offset, 1);
        assert_eq!(rep.peaks[0].elapsed_label, "00:01:00");
        assert!(rep.top_words.iter().any(|w| w.word == "草" && w.count == 3));
        assert_eq!(rep.top_users[0].user_id, "u1");
        assert_eq!(rep.top_users[0].comment_count, 2);

        // CSV Formula Injection 防御の検証
        assert_eq!(csv_escape("=CMD|' /C calc'!A0"), "'=CMD|' /C calc'!A0");
        assert_eq!(csv_escape("+1+1,test"), "\"'+1+1,test\"");
        assert_eq!(csv_escape("通常メッセージ"), "通常メッセージ");
    }
}
