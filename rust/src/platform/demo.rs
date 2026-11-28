use std::time::Duration;

use chrono::Utc;
use rand::seq::SliceRandom;
use tokio::sync::watch;

use crate::models::{
    MessageRun, MessageRunKind, MonetizationEventKind, PlatformKind, RawIncomingComment,
};
use crate::platform::{IncomingCommentTx, ProviderConnection};

struct DemoUser {
    id: &'static str,
    name: &'static str,
    color: &'static str,
    initial: &'static str,
    is_mod: bool,
    is_member: bool,
}

const DEMO_USERS: &[DemoUser] = &[
    DemoUser {
        id: "demo-user-01",
        name: "たこやき太郎",
        color: "%2338bdf8",
        initial: "た",
        is_mod: false,
        is_member: true,
    },
    DemoUser {
        id: "demo-user-02",
        name: "さくらもち",
        color: "%23f472b6",
        initial: "さ",
        is_mod: false,
        is_member: true,
    },
    DemoUser {
        id: "demo-user-03",
        name: "ゲーマーK",
        color: "%2322c55e",
        initial: "K",
        is_mod: true,
        is_member: true,
    },
    DemoUser {
        id: "demo-user-04",
        name: "初見のねこ",
        color: "%23f59e0b",
        initial: "ね",
        is_mod: false,
        is_member: false,
    },
    DemoUser {
        id: "demo-user-05",
        name: "みずいろ",
        color: "%23818cf8",
        initial: "み",
        is_mod: false,
        is_member: false,
    },
    DemoUser {
        id: "demo-user-06",
        name: "夜ふかし勢",
        color: "%23ec4899",
        initial: "夜",
        is_mod: false,
        is_member: true,
    },
];

const DEMO_MESSAGES: &[&str] = &[
    "初見です！配信楽しみにしてました！",
    "草www 今のめちゃくちゃ面白かった",
    "参加します！よろしくお願いします！",
    "質問なのですが、次の企画はいつ頃予定ですか？",
    "おおおおお！ナイスプレイ！！",
    "参加希望です！",
    "草",
    "こんばんは〜今日も見に来ました！",
    "音量ばっちり聞こえてます！",
    "参加！当たりますように！",
    "wwwwwww",
    "初見です、フォローしました！",
];

/// オフラインでもアイコン表示が綺麗に動く SVG Data URI を生成
pub fn make_svg_avatar(bg_encoded: &str, initial: &str) -> String {
    format!(
        "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'>\
         <rect width='64' height='64' rx='32' fill='{}'/>\
         <text x='50%25' y='54%25' dominant-baseline='middle' text-anchor='middle' \
         fill='white' font-family='sans-serif' font-weight='bold' font-size='28'>{}</text></svg>",
        bg_encoded, initial
    )
}

/// オフラインでもスタンプ表示を確認できるデモ用カスタム絵文字 SVG Data URI
fn make_demo_stamp(bg_hex: &str, glyph: &str) -> String {
    format!(
        "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 48 48'>\
         <rect width='48' height='48' rx='10' fill='{}'/>\
         <text x='50%25' y='55%25' dominant-baseline='middle' text-anchor='middle' \
         fill='white' font-family='sans-serif' font-weight='bold' font-size='22'>{}</text></svg>",
        bg_hex, glyph
    )
}

fn build_demo_runs(msg: &str, idx: usize) -> (String, Vec<MessageRun>) {
    if idx % 3 == 1 {
        let (code, url) = if idx % 2 == 1 {
            (":stosup_star:", make_demo_stamp("%23eab308", "★"))
        } else {
            (":stosup_gg:", make_demo_stamp("%2310b981", "GG"))
        };
        let full = format!("{} {}", msg, code);
        let runs = vec![
            MessageRun {
                kind: MessageRunKind::Text,
                text: format!("{} ", msg),
                emoji_url: None,
                alt: None,
                is_custom_emoji: false,
            },
            MessageRun {
                kind: MessageRunKind::Emoji,
                text: code.to_string(),
                emoji_url: Some(url),
                alt: Some(code.to_string()),
                is_custom_emoji: true,
            },
        ];
        (full, runs)
    } else {
        (
            msg.to_string(),
            vec![MessageRun {
                kind: MessageRunKind::Text,
                text: msg.to_string(),
                emoji_url: None,
                alt: None,
                is_custom_emoji: false,
            }],
        )
    }
}

/// 配信していなくても全機能を検証できるデモ配信プロバイダーを起動する
pub fn connect_demo(
    stream_id: &str,
    auto_generate: bool,
    comment_tx: IncomingCommentTx,
) -> ProviderConnection {
    let (cancel_tx, mut cancel_rx) = watch::channel(false);

    if auto_generate {
        tokio::spawn(async move {
            // 開始直後に少しだけサンプルコメントを流す
            let mut idx = 0usize;
            loop {
                let delay_ms = if idx < 3 { 700 } else { 2600 };
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                    _ = cancel_rx.changed() => { break; }
                }

                let (user, msg) = {
                    let mut rng = rand::thread_rng();
                    let u = DEMO_USERS.choose(&mut rng).unwrap();
                    let m = DEMO_MESSAGES.choose(&mut rng).unwrap();
                    (u, *m)
                };

                let (event_kind, amount_display, amount_micros, currency, tier_color, final_msg) =
                    if idx > 0 && idx.is_multiple_of(6) {
                        (
                            Some(MonetizationEventKind::SuperChat),
                            Some("¥1,000".to_string()),
                            1_000_000_000u64,
                            "JPY".to_string(),
                            Some("#FFCA28".to_string()),
                            "いつも配信ありがとう！応援してます！".to_string(),
                        )
                    } else if idx > 0 && idx.is_multiple_of(9) {
                        (
                            Some(MonetizationEventKind::MembershipNew),
                            Some("新規メンバー加入".to_string()),
                            0u64,
                            String::new(),
                            Some("#0F9D58".to_string()),
                            "メンバーシップに加入しました！よろしくお願いします！".to_string(),
                        )
                    } else {
                        (None, None, 0u64, String::new(), None, msg.to_string())
                    };

                let (final_msg_with_code, message_runs) = build_demo_runs(&final_msg, idx);

                let raw = RawIncomingComment {
                    id: uuid::Uuid::new_v4().to_string(),
                    platform: PlatformKind::Demo,
                    user_id: user.id.to_string(),
                    user_name: user.name.to_string(),
                    user_icon_url: make_svg_avatar(user.color, user.initial),
                    message: final_msg_with_code,
                    message_runs,
                    timestamp_ms: Utc::now().timestamp_millis(),
                    is_owner: false,
                    is_moderator: user.is_mod,
                    is_member: user.is_member || event_kind == Some(MonetizationEventKind::MembershipNew),
                    event_kind,
                    amount_display,
                    amount_micros,
                    currency,
                    tier_color,
                };
                if comment_tx.send(raw).is_err() {
                    break;
                }
                idx += 1;
            }
        });
    }

    ProviderConnection {
        platform: PlatformKind::Demo,
        stream_id: stream_id.to_string(),
        stream_title: format!("デモ配信セッション ({})", stream_id),
        live_chat_id: Some("demo-live-chat".to_string()),
        can_post_chat: true,
        cancel_tx,
    }
}
