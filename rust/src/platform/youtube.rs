use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use regex::Regex;
use serde_json::{json, Value};
use tokio::sync::{watch, Mutex};

use crate::db::Database;
use crate::models::{MonetizationEventKind, PlatformKind, RawIncomingComment};
use crate::platform::{youtube_oauth, IncomingCommentTx, ProviderConnection};

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

struct InnerTubeBootstrap {
    api_key: String,
    client_version: String,
    continuation: String,
    stream_title: String,
}

/// YouTube Live に InnerTube 方式（クォータ消費なし）で接続する。
/// OAuth 認証済みの場合はチャット投稿用に live_chat_id も並行して自動解決する。
pub async fn connect_youtube_innertube(
    video_id: &str,
    db: Arc<Mutex<Database>>,
    comment_tx: IncomingCommentTx,
    on_error: impl Fn(String) + Send + Sync + 'static,
) -> Result<ProviderConnection> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()?;

    let bootstrap = fetch_innertube_bootstrap(&client, video_id).await?;

    // OAuth 認証済みなら投稿用 live_chat_id を解決（失敗しても受信自体は続行）
    let mut live_chat_id = None;
    let mut can_post_chat = false;
    if let Ok(token) = youtube_oauth::get_valid_access_token(Arc::clone(&db)).await {
        if let Ok((cid, _)) = fetch_live_chat_id_via_api(&client, &token, video_id).await {
            live_chat_id = Some(cid);
            can_post_chat = true;
        }
    }

    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let stream_title = bootstrap.stream_title.clone();
    let video_id_owned = video_id.to_string();

    tokio::spawn(async move {
        let mut continuation = bootstrap.continuation;
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut consecutive_errors = 0u32;

        loop {
            if *cancel_rx.borrow() {
                break;
            }

            match poll_innertube_chat(
                &client,
                &bootstrap.api_key,
                &bootstrap.client_version,
                &continuation,
            )
            .await
            {
                Ok((comments, next_cont, timeout_ms)) => {
                    consecutive_errors = 0;
                    for c in comments {
                        if seen_ids.insert(c.id.clone()) {
                            let _ = comment_tx.send(c);
                        }
                    }
                    // メモリ肥大化防止
                    if seen_ids.len() > 20_000 {
                        seen_ids.clear();
                    }
                    match next_cont {
                        Some(nc) => continuation = nc,
                        None => {
                            on_error("ライブチャットが終了しました。".to_string());
                            break;
                        }
                    }

                    let wait = Duration::from_millis(timeout_ms.clamp(800, 10_000));
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = cancel_rx.changed() => { break; }
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= 5 {
                        on_error(format!(
                            "YouTube チャット取得エラー（video={}）: {}",
                            video_id_owned, e
                        ));
                        break;
                    }
                    let wait = Duration::from_secs(2 * consecutive_errors as u64);
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = cancel_rx.changed() => { break; }
                    }
                }
            }
        }
    });

    Ok(ProviderConnection {
        platform: PlatformKind::YouTube,
        stream_id: video_id.to_string(),
        stream_title,
        live_chat_id,
        can_post_chat,
        cancel_tx,
    })
}

/// YouTube Data API v3 公式ポーリングで接続する（OAuth 認証必須）
pub async fn connect_youtube_data_api(
    video_id: &str,
    db: Arc<Mutex<Database>>,
    comment_tx: IncomingCommentTx,
    on_error: impl Fn(String) + Send + Sync + 'static,
) -> Result<ProviderConnection> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;

    let token = youtube_oauth::get_valid_access_token(Arc::clone(&db)).await?;
    let (live_chat_id, stream_title) =
        fetch_live_chat_id_via_api(&client, &token, video_id).await?;

    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let chat_id_clone = live_chat_id.clone();

    tokio::spawn(async move {
        let mut page_token: Option<String> = None;
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut first_poll = true;

        loop {
            if *cancel_rx.borrow() {
                break;
            }

            let access_token = match youtube_oauth::get_valid_access_token(Arc::clone(&db)).await {
                Ok(t) => t,
                Err(e) => {
                    on_error(format!("OAuth トークン取得エラー: {}", e));
                    break;
                }
            };

            let mut req = client
                .get("https://www.googleapis.com/youtube/v3/liveChat/messages")
                .bearer_auth(&access_token)
                .query(&[
                    ("liveChatId", chat_id_clone.as_str()),
                    ("part", "snippet,authorDetails"),
                    ("maxResults", "200"),
                ]);
            if let Some(ref pt) = page_token {
                req = req.query(&[("pageToken", pt.as_str())]);
            }

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(val) = resp.json::<Value>().await {
                        let poll_ms = val["pollingIntervalMillis"].as_u64().unwrap_or(5000);
                        page_token = val["nextPageToken"].as_str().map(|s| s.to_string());

                        if let Some(items) = val["items"].as_array() {
                            for item in items {
                                if let Some(comment) = parse_data_api_message(item) {
                                    // 初回ポーリングでは過去ログ重複を避けるため記録しつつ最新20件のみ流す
                                    if seen_ids.insert(comment.id.clone())
                                        && (!first_poll || items.len() <= 20)
                                    {
                                        let _ = comment_tx.send(comment);
                                    }
                                }
                            }
                        }
                        first_poll = false;

                        let wait = Duration::from_millis(poll_ms.clamp(2000, 15000));
                        tokio::select! {
                            _ = tokio::time::sleep(wait) => {}
                            _ = cancel_rx.changed() => { break; }
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    on_error(format!("YouTube Data API エラー ({}): {}", status, body));
                    break;
                }
                Err(e) => {
                    on_error(format!("YouTube Data API 通信エラー: {}", e));
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                        _ = cancel_rx.changed() => { break; }
                    }
                }
            }
        }
    });

    Ok(ProviderConnection {
        platform: PlatformKind::YouTube,
        stream_id: video_id.to_string(),
        stream_title,
        live_chat_id: Some(live_chat_id),
        can_post_chat: true,
        cancel_tx,
    })
}

/// YouTube Data API v3 でライブチャットへコメントを投稿する（コメント抽選の開始・終了コメント用）
pub async fn post_live_chat_message(
    db: Arc<Mutex<Database>>,
    live_chat_id: &str,
    message_text: &str,
) -> Result<()> {
    let token = youtube_oauth::get_valid_access_token(db).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let payload = json!({
        "snippet": {
            "liveChatId": live_chat_id,
            "type": "textMessageEvent",
            "textMessageDetails": {
                "messageText": message_text
            }
        }
    });

    let resp = client
        .post("https://www.googleapis.com/youtube/v3/liveChat/messages")
        .query(&[("part", "snippet")])
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await
        .context("YouTube チャット投稿リクエストに失敗しました")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "YouTube チャットへのコメント投稿に失敗しました ({}): {}",
            status,
            body
        ));
    }

    Ok(())
}

/// video_id から activeLiveChatId と配信タイトルを取得する
pub async fn fetch_live_chat_id_via_api(
    client: &reqwest::Client,
    access_token: &str,
    video_id: &str,
) -> Result<(String, String)> {
    let resp = client
        .get("https://www.googleapis.com/youtube/v3/videos")
        .query(&[("part", "liveStreamingDetails,snippet"), ("id", video_id)])
        .bearer_auth(access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("動画情報の取得に失敗しました: {}", body));
    }

    let val: Value = resp.json().await?;
    let item = val["items"]
        .get(0)
        .ok_or_else(|| anyhow!("指定された YouTube 動画が見つかりませんでした"))?;

    let title = item["snippet"]["title"]
        .as_str()
        .unwrap_or("YouTube Live")
        .to_string();
    let live_chat_id = item["liveStreamingDetails"]["activeLiveChatId"]
        .as_str()
        .ok_or_else(|| {
            anyhow!("アクティブなライブチャットが見つかりません（配信が開始されていないか、チャットが無効です）")
        })?
        .to_string();

    Ok((live_chat_id, title))
}

async fn fetch_innertube_bootstrap(
    client: &reqwest::Client,
    video_id: &str,
) -> Result<InnerTubeBootstrap> {
    let watch_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let html = client
        .get(&watch_url)
        .header("Accept-Language", "ja-JP,ja;q=0.9,en-US;q=0.8")
        .send()
        .await
        .context("YouTube 配信ページの取得に失敗しました")?
        .text()
        .await?;

    let api_key_re = Regex::new(r#""INNERTUBE_API_KEY"\s*:\s*"([^"]+)""#).unwrap();
    let client_ver_re = Regex::new(r#""INNERTUBE_CLIENT_VERSION"\s*:\s*"([^"]+)""#).unwrap();
    let title_re = Regex::new(r#"<title>(.*?)</title>"#).unwrap();
    let cont_re = Regex::new(r#""continuation"\s*:\s*"([^"]{20,})""#).unwrap();

    let api_key = api_key_re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("YouTube API キーの抽出に失敗しました（配信URLをご確認ください）"))?;

    let client_version = client_ver_re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "2.20250101.00.00".to_string());

    let stream_title = title_re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().replace(" - YouTube", "").trim().to_string())
        .unwrap_or_else(|| format!("YouTube Live ({})", video_id));

    // live_chat ポップアウトページから確実にチャット用 continuation を取得
    let popout_url = format!(
        "https://www.youtube.com/live_chat?is_popout=1&v={}",
        video_id
    );
    let popout_html = client
        .get(&popout_url)
        .header("Accept-Language", "ja-JP,ja;q=0.9,en-US;q=0.8")
        .send()
        .await
        .ok()
        .and_then(|r| Some(r))
        .map(|r| r.text());

    let popout_text = match popout_html {
        Some(fut) => fut.await.unwrap_or_default(),
        None => String::new(),
    };

    let continuation = extract_live_chat_continuation(&popout_text)
        .or_else(|| extract_live_chat_continuation(&html))
        .or_else(|| {
            cont_re
                .captures(&popout_text)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        })
        .ok_or_else(|| {
            anyhow!("ライブチャットの接続トークンを取得できませんでした（配信中でないか、チャットが無効の可能性があります）")
        })?;

    Ok(InnerTubeBootstrap {
        api_key,
        client_version,
        continuation,
        stream_title,
    })
}

fn extract_live_chat_continuation(html: &str) -> Option<String> {
    // subMenuItems の中にある「チャット」（リアルタイム）または最初の reloadContinuationData / invalidationContinuationData を探す
    let patterns = [
        r#""invalidationContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#,
        r#""timedContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#,
        r#""reloadContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#,
    ];
    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(html) {
                if let Some(m) = cap.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

async fn poll_innertube_chat(
    client: &reqwest::Client,
    api_key: &str,
    client_version: &str,
    continuation: &str,
) -> Result<(Vec<RawIncomingComment>, Option<String>, u64)> {
    let endpoint = format!(
        "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
        api_key
    );
    let body = json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": client_version,
                "hl": "ja",
                "gl": "JP"
            }
        },
        "continuation": continuation
    });

    let resp = client.post(&endpoint).json(&body).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("InnerTube HTTP エラー: {}", resp.status()));
    }

    let val: Value = resp.json().await?;
    let live_chat = &val["continuationContents"]["liveChatContinuation"];

    let mut comments = Vec::new();
    if let Some(actions) = live_chat["actions"].as_array() {
        for action in actions {
            if let Some(item) = action["addChatItemAction"]["item"].as_object() {
                if let Some(renderer) = item.get("liveChatTextMessageRenderer") {
                    if let Some(c) = parse_innertube_text_renderer(renderer) {
                        comments.push(c);
                    }
                } else if let Some(renderer) = item.get("liveChatPaidMessageRenderer") {
                    if let Some(c) = parse_innertube_paid_message_renderer(renderer) {
                        comments.push(c);
                    }
                } else if let Some(renderer) = item.get("liveChatPaidStickerRenderer") {
                    if let Some(c) = parse_innertube_paid_sticker_renderer(renderer) {
                        comments.push(c);
                    }
                } else if let Some(renderer) = item.get("liveChatMembershipItemRenderer") {
                    if let Some(c) = parse_innertube_membership_renderer(renderer) {
                        comments.push(c);
                    }
                } else if let Some(renderer) =
                    item.get("liveChatSponsorshipsGiftPurchaseAnnouncementRenderer")
                {
                    if let Some(c) = parse_innertube_gift_renderer(renderer) {
                        comments.push(c);
                    }
                }
            }
        }
    }

    let mut next_cont = None;
    let mut timeout_ms = 2500u64;

    if let Some(conts) = live_chat["continuations"].as_array() {
        for c in conts {
            if let Some(data) = c
                .get("invalidationContinuationData")
                .or_else(|| c.get("timedContinuationData"))
                .or_else(|| c.get("reloadContinuationData"))
            {
                if let Some(tok) = data["continuation"].as_str() {
                    next_cont = Some(tok.to_string());
                    if let Some(ms) = data["timeoutMs"].as_u64() {
                        timeout_ms = ms;
                    }
                    break;
                }
            }
        }
    }

    Ok((comments, next_cont, timeout_ms))
}

fn argb_u64_to_hex(val: u64) -> String {
    let r = (val >> 16) & 0xFF;
    let g = (val >> 8) & 0xFF;
    let b = val & 0xFF;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn estimate_amount_micros(display: &str) -> u64 {
    let digits: String = display
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if let Ok(num) = digits.parse::<f64>() {
        (num * 1_000_000.0).max(0.0) as u64
    } else {
        0
    }
}

fn extract_common_innertube_author(
    renderer: &Value,
) -> Option<(String, String, String, String, i64, bool, bool, bool)> {
    let id = renderer["id"].as_str()?.to_string();
    let user_id = renderer["authorExternalChannelId"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let user_name = renderer["authorName"]["simpleText"]
        .as_str()
        .unwrap_or("匿名ユーザー")
        .to_string();
    let user_icon_url = renderer["authorPhoto"]["thumbnails"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|t| t["url"].as_str())
        .unwrap_or("")
        .to_string();
    let timestamp_ms = renderer["timestampUsec"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|usec| usec / 1000)
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let mut is_owner = false;
    let mut is_moderator = false;
    let mut is_member = false;
    if let Some(badges) = renderer["authorBadges"].as_array() {
        for b in badges {
            let icon_type = b["liveChatAuthorBadgeRenderer"]["icon"]["iconType"]
                .as_str()
                .unwrap_or("");
            match icon_type {
                "OWNER" => is_owner = true,
                "MODERATOR" => is_moderator = true,
                _ => {}
            }
            if b["liveChatAuthorBadgeRenderer"]["customThumbnail"].is_object() {
                is_member = true;
            }
        }
    }

    Some((
        id,
        user_id,
        user_name,
        user_icon_url,
        timestamp_ms,
        is_owner,
        is_moderator,
        is_member,
    ))
}

fn parse_innertube_text_renderer(renderer: &Value) -> Option<RawIncomingComment> {
    let (id, user_id, user_name, user_icon_url, timestamp_ms, is_owner, is_moderator, is_member) =
        extract_common_innertube_author(renderer)?;
    let message = extract_runs_text(&renderer["message"]["runs"]);
    if message.is_empty() {
        return None;
    }

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner,
        is_moderator,
        is_member,
        event_kind: None,
        amount_display: None,
        amount_micros: 0,
        currency: String::new(),
        tier_color: None,
    })
}

fn parse_innertube_paid_message_renderer(renderer: &Value) -> Option<RawIncomingComment> {
    let (id, user_id, user_name, user_icon_url, timestamp_ms, is_owner, is_moderator, is_member) =
        extract_common_innertube_author(renderer)?;
    let amount_display = renderer["purchaseAmountText"]["simpleText"]
        .as_str()
        .unwrap_or("Super Chat")
        .to_string();
    let mut message = extract_runs_text(&renderer["message"]["runs"]);
    if message.is_empty() {
        message = format!("（メッセージなしのスーパーチャット: {}）", amount_display);
    }
    let tier_color = renderer["bodyBackgroundColor"]
        .as_u64()
        .or_else(|| renderer["headerBackgroundColor"].as_u64())
        .map(argb_u64_to_hex)
        .unwrap_or_else(|| "#FFB300".to_string());
    let amount_micros = estimate_amount_micros(&amount_display);

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner,
        is_moderator,
        is_member,
        event_kind: Some(MonetizationEventKind::SuperChat),
        amount_display: Some(amount_display),
        amount_micros,
        currency: String::new(),
        tier_color: Some(tier_color),
    })
}

fn parse_innertube_paid_sticker_renderer(renderer: &Value) -> Option<RawIncomingComment> {
    let (id, user_id, user_name, user_icon_url, timestamp_ms, is_owner, is_moderator, is_member) =
        extract_common_innertube_author(renderer)?;
    let amount_display = renderer["purchaseAmountText"]["simpleText"]
        .as_str()
        .unwrap_or("Super Sticker")
        .to_string();
    let label = renderer["sticker"]["accessibility"]["accessibilityData"]["label"]
        .as_str()
        .unwrap_or("スーパーステッカー");
    let message = format!("ステッカー: {}", label);
    let tier_color = renderer["backgroundColor"]
        .as_u64()
        .map(argb_u64_to_hex)
        .unwrap_or_else(|| "#E91E63".to_string());
    let amount_micros = estimate_amount_micros(&amount_display);

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner,
        is_moderator,
        is_member,
        event_kind: Some(MonetizationEventKind::SuperSticker),
        amount_display: Some(amount_display),
        amount_micros,
        currency: String::new(),
        tier_color: Some(tier_color),
    })
}

fn parse_innertube_membership_renderer(renderer: &Value) -> Option<RawIncomingComment> {
    let (id, user_id, user_name, user_icon_url, timestamp_ms, is_owner, is_moderator, _) =
        extract_common_innertube_author(renderer)?;
    let primary = extract_runs_text(&renderer["headerPrimaryText"]["runs"]);
    let subtext = extract_runs_text(&renderer["headerSubtext"]["runs"]);
    let body = extract_runs_text(&renderer["message"]["runs"]);
    let is_milestone = !primary.is_empty();
    let kind = if is_milestone {
        MonetizationEventKind::MembershipMilestone
    } else {
        MonetizationEventKind::MembershipNew
    };
    let amount_display = if !primary.is_empty() {
        primary
    } else if !subtext.is_empty() {
        subtext.clone()
    } else {
        "新規メンバー加入".to_string()
    };
    let message = if !body.is_empty() {
        body
    } else if !subtext.is_empty() {
        subtext
    } else {
        "メンバーシップに加入しました！".to_string()
    };

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner,
        is_moderator,
        is_member: true,
        event_kind: Some(kind),
        amount_display: Some(amount_display),
        amount_micros: 0,
        currency: String::new(),
        tier_color: Some("#0F9D58".to_string()),
    })
}

fn parse_innertube_gift_renderer(renderer: &Value) -> Option<RawIncomingComment> {
    let id = renderer["id"].as_str()?.to_string();
    let header = &renderer["header"]["liveChatSponsorshipsHeaderRenderer"];
    let user_id = renderer["authorExternalChannelId"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let user_name = header["authorName"]["simpleText"]
        .as_str()
        .unwrap_or("メンバーシップギフト")
        .to_string();
    let user_icon_url = header["authorPhoto"]["thumbnails"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|t| t["url"].as_str())
        .unwrap_or("")
        .to_string();
    let timestamp_ms = renderer["timestampUsec"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|usec| usec / 1000)
        .unwrap_or_else(|| Utc::now().timestamp_millis());
    let primary = extract_runs_text(&header["primaryText"]["runs"]);
    let message = if primary.is_empty() {
        "メンバーシップギフトを贈りました！".to_string()
    } else {
        primary.clone()
    };

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner: false,
        is_moderator: false,
        is_member: true,
        event_kind: Some(MonetizationEventKind::MembershipGift),
        amount_display: Some("メンバーシップギフト".to_string()),
        amount_micros: 0,
        currency: String::new(),
        tier_color: Some("#0F9D58".to_string()),
    })
}

fn extract_runs_text(runs: &Value) -> String {
    let mut out = String::new();
    if let Some(arr) = runs.as_array() {
        for run in arr {
            if let Some(t) = run["text"].as_str() {
                out.push_str(t);
            } else if let Some(emoji) = run["emoji"].as_object() {
                if let Some(sc) = emoji
                    .get("shortcuts")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                {
                    out.push_str(sc);
                } else if let Some(eid) = emoji.get("emojiId").and_then(|v| v.as_str()) {
                    out.push_str(eid);
                }
            }
        }
    }
    out
}

fn super_chat_tier_color(tier: u64) -> &'static str {
    match tier {
        1 => "#1E88E5",
        2 => "#00E5FF",
        3 => "#1DE9B6",
        4 => "#FFCA28",
        5 => "#F57C00",
        6 => "#E91E63",
        _ => "#E62117",
    }
}

fn parse_data_api_message(item: &Value) -> Option<RawIncomingComment> {
    let id = item["id"].as_str()?.to_string();
    let snippet = &item["snippet"];
    let author = &item["authorDetails"];
    let msg_type = snippet["type"].as_str().unwrap_or("textMessageEvent");
    let mut message = snippet["displayMessage"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let user_id = author["channelId"].as_str().unwrap_or("unknown").to_string();
    let user_name = author["displayName"]
        .as_str()
        .unwrap_or("匿名ユーザー")
        .to_string();
    let user_icon_url = author["profileImageUrl"].as_str().unwrap_or("").to_string();

    let timestamp_ms = snippet["publishedAt"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let mut event_kind = None;
    let mut amount_display = None;
    let mut amount_micros = 0u64;
    let mut currency = String::new();
    let mut tier_color = None;

    match msg_type {
        "superChatEvent" => {
            let d = &snippet["superChatDetails"];
            event_kind = Some(MonetizationEventKind::SuperChat);
            amount_display = d["amountDisplayString"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| Some("Super Chat".to_string()));
            amount_micros = d["amountMicros"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| d["amountMicros"].as_u64())
                .unwrap_or(0);
            currency = d["currency"].as_str().unwrap_or("").to_string();
            tier_color = Some(super_chat_tier_color(d["tier"].as_u64().unwrap_or(4)).to_string());
            if let Some(uc) = d["userComment"].as_str() {
                if !uc.is_empty() {
                    message = uc.to_string();
                }
            }
        }
        "superStickerEvent" => {
            let d = &snippet["superStickerDetails"];
            event_kind = Some(MonetizationEventKind::SuperSticker);
            amount_display = d["amountDisplayString"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| Some("Super Sticker".to_string()));
            amount_micros = d["amountMicros"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| d["amountMicros"].as_u64())
                .unwrap_or(0);
            currency = d["currency"].as_str().unwrap_or("").to_string();
            tier_color = Some(super_chat_tier_color(d["tier"].as_u64().unwrap_or(4)).to_string());
            if message.is_empty() {
                let alt = d["superStickerMetadata"]["altText"]
                    .as_str()
                    .unwrap_or("スーパーステッカー");
                message = format!("ステッカー: {}", alt);
            }
        }
        "newSponsorEvent" => {
            event_kind = Some(MonetizationEventKind::MembershipNew);
            let level = snippet["newSponsorDetails"]["memberLevelName"]
                .as_str()
                .unwrap_or("新規メンバー");
            amount_display = Some(level.to_string());
            tier_color = Some("#0F9D58".to_string());
            if message.is_empty() {
                message = format!("{} に加入しました！", level);
            }
        }
        "memberMilestoneChatEvent" => {
            event_kind = Some(MonetizationEventKind::MembershipMilestone);
            let months = snippet["memberMilestoneChatDetails"]["memberMonth"]
                .as_u64()
                .unwrap_or(1);
            amount_display = Some(format!("メンバー {}ヶ月継続", months));
            tier_color = Some("#0F9D58".to_string());
            if let Some(uc) = snippet["memberMilestoneChatDetails"]["userComment"].as_str() {
                if !uc.is_empty() {
                    message = uc.to_string();
                }
            }
        }
        "membershipGiftingEvent" => {
            event_kind = Some(MonetizationEventKind::MembershipGift);
            let cnt = snippet["membershipGiftingDetails"]["giftMembershipsCount"]
                .as_u64()
                .unwrap_or(1);
            amount_display = Some(format!("メンバーシップギフト {}件", cnt));
            tier_color = Some("#0F9D58".to_string());
            if message.is_empty() {
                message = format!("メンバーシップギフトを {} 件贈りました！", cnt);
            }
        }
        _ => {
            if message.is_empty() {
                return None;
            }
        }
    }

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        timestamp_ms,
        is_owner: author["isChatOwner"].as_bool().unwrap_or(false),
        is_moderator: author["isChatModerator"].as_bool().unwrap_or(false),
        is_member: author["isChatSponsor"].as_bool().unwrap_or(false)
            || matches!(
                event_kind,
                Some(
                    MonetizationEventKind::MembershipNew
                        | MonetizationEventKind::MembershipMilestone
                        | MonetizationEventKind::MembershipGift
                )
            ),
        event_kind,
        amount_display,
        amount_micros,
        currency,
        tier_color,
    })
}
