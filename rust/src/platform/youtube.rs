use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};
use tokio::sync::{watch, Mutex};

use crate::db::Database;
use crate::models::{
    ConnectionStatus, MessageRun, MessageRunKind, MonetizationEventKind, PlatformKind,
    RawIncomingComment, ResolvedYouTubeLive,
};
use crate::platform::{
    parse_youtube_input_target, youtube_oauth, IncomingCommentTx, ProviderConnection,
    YouTubeInputTarget,
};

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

static RE_CANONICAL_VID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<link\s+rel="canonical"\s+href="https://www\.youtube\.com/watch\?v=([A-Za-z0-9_-]{11})""#)
        .unwrap()
});
static RE_VIDEO_ID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""videoId"\s*:\s*"([A-Za-z0-9_-]{11})""#).unwrap());
static RE_TITLE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<title>(.*?)</title>"#).unwrap());
static RE_API_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""INNERTUBE_API_KEY"\s*:\s*"([^"]+)""#).unwrap());
static RE_CLIENT_VER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""INNERTUBE_CLIENT_VERSION"\s*:\s*"([^"]+)""#).unwrap());
static RE_CONT_FALLBACK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#""continuation"\s*:\s*"([^"]{20,})""#).unwrap());
static RE_CONT_PATTERNS: Lazy<[Regex; 3]> = Lazy::new(|| {
    [
        Regex::new(r#""invalidationContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#)
            .unwrap(),
        Regex::new(r#""timedContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#)
            .unwrap(),
        Regex::new(r#""reloadContinuationData"\s*:\s*\{[^}]*?"continuation"\s*:\s*"([^"]+)""#)
            .unwrap(),
    ]
});

/// FIFO 上限付き重複排除セット（長時間配信でのメモリ肥大化と全消去による再取得スパイクを両方防止）
struct BoundedIdSet {
    max_len: usize,
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl BoundedIdSet {
    fn new(max_len: usize) -> Self {
        Self {
            max_len: max_len.max(1),
            set: HashSet::new(),
            order: VecDeque::new(),
        }
    }

    fn insert(&mut self, id: String) -> bool {
        if self.set.contains(&id) {
            return false;
        }
        if self.set.len() >= self.max_len {
            if let Some(oldest) = self.order.pop_front() {
                self.set.remove(&oldest);
            }
        }
        self.set.insert(id.clone());
        self.order.push_back(id);
        true
    }
}

struct InnerTubeBootstrap {
    api_key: String,
    client_version: String,
    continuation: String,
    stream_title: String,
}

/// チャンネル URL / @ハンドル / 動画 URL から現在のライブ配信または待機枠の video_id を解決する
pub async fn resolve_youtube_live_target(input: &str) -> Result<ResolvedYouTubeLive> {
    let target = parse_youtube_input_target(input)?;
    match target {
        YouTubeInputTarget::VideoId(vid) => Ok(ResolvedYouTubeLive {
            video_id: vid,
            title: String::new(),
            is_live: true,
            is_upcoming: false,
            source_input: input.trim().to_string(),
        }),
        YouTubeInputTarget::ChannelLiveUrl(live_url) => {
            let client = reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(Duration::from_secs(15))
                .build()?;
            let html = client
                .get(&live_url)
                .header("Accept-Language", "ja-JP,ja;q=0.9,en-US;q=0.8")
                .send()
                .await
                .context("チャンネルのライブページ取得に失敗しました")?
                .text()
                .await?;
            parse_channel_live_html(&html, input.trim())
        }
    }
}

fn parse_channel_live_html(html: &str, source_input: &str) -> Result<ResolvedYouTubeLive> {
    let video_id = RE_CANONICAL_VID
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .or_else(|| {
            // /live ページ内にライブ配信枠の videoId がある場合のみ抽出
            if html.contains(r#""isLive":true"#)
                || html.contains(r#""isUpcoming":true"#)
                || html.contains(r#""liveBroadcastDetails""#)
            {
                RE_VIDEO_ID
                    .captures(html)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow!("指定されたチャンネルで現在配信中・待機中のライブ枠が見つかりませんでした")
        })?;

    let title = RE_TITLE
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().replace(" - YouTube", "").trim().to_string())
        .unwrap_or_else(|| format!("YouTube Live ({})", video_id));
    let is_live = html.contains(r#""isLiveNow":true"#) || html.contains(r#""isLive":true"#);
    let is_upcoming = html.contains(r#""isUpcoming":true"#);

    Ok(ResolvedYouTubeLive {
        video_id,
        title,
        is_live,
        is_upcoming,
        source_input: source_input.to_string(),
    })
}

/// YouTube Live に InnerTube 方式（クォータ消費なし）で接続する。
/// 通信瞬断時は指数バックオフ＋再ブートストラップで自動再接続する。
pub async fn connect_youtube_innertube(
    video_id: &str,
    db: Arc<Mutex<Database>>,
    comment_tx: IncomingCommentTx,
    on_error: impl Fn(String) + Send + Sync + 'static,
    on_status: impl Fn(ConnectionStatus, u32, Option<String>) + Send + Sync + 'static,
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
        let mut api_key = bootstrap.api_key;
        let mut client_version = bootstrap.client_version;
        let mut continuation = bootstrap.continuation;
        let mut seen_ids = BoundedIdSet::new(15_000);
        let mut consecutive_errors = 0u32;
        const MAX_RECONNECT_ATTEMPTS: u32 = 10;

        loop {
            if *cancel_rx.borrow() {
                break;
            }

            match poll_innertube_chat(&client, &api_key, &client_version, &continuation).await {
                Ok((comments, next_cont, timeout_ms)) => {
                    if consecutive_errors > 0 {
                        consecutive_errors = 0;
                        on_status(ConnectionStatus::Connected, 0, None);
                    }
                    for c in comments {
                        if seen_ids.insert(c.id.clone()) {
                            let _ = comment_tx.send(c);
                        }
                    }
                    match next_cont {
                        Some(nc) => continuation = nc,
                        None => {
                            on_status(
                                ConnectionStatus::Disconnected,
                                0,
                                Some("ライブチャットが終了しました。".to_string()),
                            );
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
                    if consecutive_errors > MAX_RECONNECT_ATTEMPTS {
                        let msg = format!(
                            "YouTube チャット再接続上限に達しました（video={}）: {}",
                            video_id_owned, e
                        );
                        on_status(ConnectionStatus::Error, consecutive_errors, Some(msg.clone()));
                        on_error(msg);
                        break;
                    }

                    let backoff_sec = match consecutive_errors {
                        1 => 2,
                        2 => 4,
                        3 => 8,
                        4 => 15,
                        _ => 25,
                    };
                    on_status(
                        ConnectionStatus::Reconnecting,
                        consecutive_errors,
                        Some(format!(
                            "通信エラーのため自動再接続中 ({}/{}): {}",
                            consecutive_errors, MAX_RECONNECT_ATTEMPTS, e
                        )),
                    );

                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(backoff_sec)) => {}
                        _ = cancel_rx.changed() => { break; }
                    }

                    // 2回以上連続失敗した場合は continuation 失効の可能性があるため再ブートストラップ
                    if consecutive_errors >= 2 {
                        if let Ok(fresh) = fetch_innertube_bootstrap(&client, &video_id_owned).await
                        {
                            api_key = fresh.api_key;
                            client_version = fresh.client_version;
                            continuation = fresh.continuation;
                        }
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

/// YouTube Data API v3 公式ポーリングで接続する（OAuth 認証必須・自動再接続対応）
pub async fn connect_youtube_data_api(
    video_id: &str,
    db: Arc<Mutex<Database>>,
    comment_tx: IncomingCommentTx,
    on_error: impl Fn(String) + Send + Sync + 'static,
    on_status: impl Fn(ConnectionStatus, u32, Option<String>) + Send + Sync + 'static,
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
        let mut seen_ids = BoundedIdSet::new(15_000);
        let mut first_poll = true;
        let mut consecutive_errors = 0u32;
        const MAX_RECONNECT_ATTEMPTS: u32 = 10;

        loop {
            if *cancel_rx.borrow() {
                break;
            }

            let access_token = match youtube_oauth::get_valid_access_token(Arc::clone(&db)).await {
                Ok(t) => t,
                Err(e) => {
                    let msg = format!("OAuth トークン取得エラー: {}", e);
                    on_status(ConnectionStatus::Error, 0, Some(msg.clone()));
                    on_error(msg);
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
                    if consecutive_errors > 0 {
                        consecutive_errors = 0;
                        on_status(ConnectionStatus::Connected, 0, None);
                    }
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
                    if status.as_u16() >= 500 || status.as_u16() == 429 {
                        consecutive_errors += 1;
                        if consecutive_errors <= MAX_RECONNECT_ATTEMPTS {
                            on_status(
                                ConnectionStatus::Reconnecting,
                                consecutive_errors,
                                Some(format!("YouTube Data API 一時エラー ({}) 再試行中...", status)),
                            );
                            let wait = Duration::from_secs((3 * consecutive_errors as u64).min(30));
                            tokio::select! {
                                _ = tokio::time::sleep(wait) => {}
                                _ = cancel_rx.changed() => { break; }
                            }
                            continue;
                        }
                    }
                    let msg = format!("YouTube Data API エラー ({}): {}", status, body);
                    on_status(ConnectionStatus::Error, consecutive_errors, Some(msg.clone()));
                    on_error(msg);
                    break;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors > MAX_RECONNECT_ATTEMPTS {
                        let msg = format!("YouTube Data API 再接続上限に達しました: {}", e);
                        on_status(ConnectionStatus::Error, consecutive_errors, Some(msg.clone()));
                        on_error(msg);
                        break;
                    }
                    on_status(
                        ConnectionStatus::Reconnecting,
                        consecutive_errors,
                        Some(format!(
                            "YouTube Data API 通信エラー ({}/{}): {}",
                            consecutive_errors, MAX_RECONNECT_ATTEMPTS, e
                        )),
                    );
                    let wait = Duration::from_secs((3 * consecutive_errors as u64).min(25));
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
    let popout_url = format!(
        "https://www.youtube.com/live_chat?is_popout=1&v={}",
        video_id
    );

    // 配信ページと live_chat ポップアウトを並列取得し接続開始レイテンシを短縮
    let watch_fut = async {
        let resp = client
            .get(&watch_url)
            .header("Accept-Language", "ja-JP,ja;q=0.9,en-US;q=0.8")
            .send()
            .await
            .context("YouTube 配信ページの取得に失敗しました")?;
        resp.text().await.map_err(anyhow::Error::from)
    };
    let popout_fut = async {
        match client
            .get(&popout_url)
            .header("Accept-Language", "ja-JP,ja;q=0.9,en-US;q=0.8")
            .send()
            .await
        {
            Ok(resp) => resp.text().await.unwrap_or_default(),
            Err(_) => String::new(),
        }
    };

    let (html_res, popout_text) = tokio::join!(watch_fut, popout_fut);
    let html = html_res?;

    let api_key = RE_API_KEY
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("YouTube API キーの抽出に失敗しました（配信URLをご確認ください）"))?;

    let client_version = RE_CLIENT_VER
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "2.20250101.00.00".to_string());

    let stream_title = RE_TITLE
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().replace(" - YouTube", "").trim().to_string())
        .unwrap_or_else(|| format!("YouTube Live ({})", video_id));

    let continuation = extract_live_chat_continuation(&popout_text)
        .or_else(|| extract_live_chat_continuation(&html))
        .or_else(|| {
            RE_CONT_FALLBACK
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
    for re in RE_CONT_PATTERNS.iter() {
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().to_string());
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
    let (message, message_runs) = extract_runs_with_emojis(&renderer["message"]["runs"]);
    if message.is_empty() && message_runs.is_empty() {
        return None;
    }

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        message_runs,
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
    let (mut message, mut message_runs) = extract_runs_with_emojis(&renderer["message"]["runs"]);
    if message.is_empty() {
        message = format!("（メッセージなしのスーパーチャット: {}）", amount_display);
        message_runs = Vec::new();
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
        message_runs,
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
    let sticker_url = renderer["sticker"]["thumbnails"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|t| t["url"].as_str())
        .map(|u| {
            if u.starts_with("//") {
                format!("https:{}", u)
            } else {
                u.to_string()
            }
        })
        .filter(|u| u.starts_with("https://"));
    let message_runs = if let Some(url) = sticker_url {
        vec![
            MessageRun {
                kind: MessageRunKind::Emoji,
                text: format!(":{}:", label),
                emoji_url: Some(url),
                alt: Some(label.to_string()),
                is_custom_emoji: true,
            },
            MessageRun {
                kind: MessageRunKind::Text,
                text: format!(" {}", label),
                emoji_url: None,
                alt: None,
                is_custom_emoji: false,
            },
        ]
    } else {
        Vec::new()
    };
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
        message_runs,
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
    let (primary, _) = extract_runs_with_emojis(&renderer["headerPrimaryText"]["runs"]);
    let (subtext, _) = extract_runs_with_emojis(&renderer["headerSubtext"]["runs"]);
    let (body, body_runs) = extract_runs_with_emojis(&renderer["message"]["runs"]);
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
    let (message, message_runs) = if !body.is_empty() {
        (body, body_runs)
    } else if !subtext.is_empty() {
        (subtext, Vec::new())
    } else {
        ("メンバーシップに加入しました！".to_string(), Vec::new())
    };

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        message_runs,
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
    let (primary, primary_runs) = extract_runs_with_emojis(&header["primaryText"]["runs"]);
    let (message, message_runs) = if primary.is_empty() {
        ("メンバーシップギフトを贈りました！".to_string(), Vec::new())
    } else {
        (primary, primary_runs)
    };

    Some(RawIncomingComment {
        id,
        platform: PlatformKind::YouTube,
        user_id,
        user_name,
        user_icon_url,
        message,
        message_runs,
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

/// InnerTube の message.runs からプレーンテキストと絵文字・メンバースタンプ画像付きランを抽出する
pub fn extract_runs_with_emojis(runs: &Value) -> (String, Vec<MessageRun>) {
    let mut out = String::new();
    let mut parsed_runs = Vec::new();
    if let Some(arr) = runs.as_array() {
        for run in arr {
            if let Some(t) = run["text"].as_str() {
                out.push_str(t);
                parsed_runs.push(MessageRun {
                    kind: MessageRunKind::Text,
                    text: t.to_string(),
                    emoji_url: None,
                    alt: None,
                    is_custom_emoji: false,
                });
            } else if let Some(emoji) = run["emoji"].as_object() {
                let shortcut = emoji
                    .get("shortcuts")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let emoji_id = emoji
                    .get("emojiId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let label = emoji
                    .get("image")
                    .and_then(|im| im["accessibility"]["accessibilityData"]["label"].as_str())
                    .map(|s| s.to_string());
                let display_code = shortcut
                    .clone()
                    .or_else(|| emoji_id.clone())
                    .or_else(|| label.clone())
                    .unwrap_or_default();
                let is_custom = emoji
                    .get("isCustomEmoji")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let img_url = emoji
                    .get("image")
                    .and_then(|im| im["thumbnails"].as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str())
                    .map(|u| {
                        if u.starts_with("//") {
                            format!("https:{}", u)
                        } else {
                            u.to_string()
                        }
                    })
                    .filter(|u| u.starts_with("https://"));

                out.push_str(&display_code);
                if let Some(url) = img_url {
                    parsed_runs.push(MessageRun {
                        kind: MessageRunKind::Emoji,
                        text: display_code.clone(),
                        emoji_url: Some(url),
                        alt: label.or(Some(display_code)),
                        is_custom_emoji: is_custom,
                    });
                } else if !display_code.is_empty() {
                    parsed_runs.push(MessageRun {
                        kind: MessageRunKind::Text,
                        text: display_code,
                        emoji_url: None,
                        alt: None,
                        is_custom_emoji: false,
                    });
                }
            }
        }
    }
    (out, parsed_runs)
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
        message_runs: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_runs_with_custom_emoji_and_text() {
        let runs = json!([
            { "text": "こんちゃ！" },
            {
                "emoji": {
                    "emojiId": "UC123/abc",
                    "shortcuts": [":_custom_wave:"],
                    "isCustomEmoji": true,
                    "image": {
                        "thumbnails": [
                            { "url": "https://yt3.ggpht.com/stamp-24.png" },
                            { "url": "https://yt3.ggpht.com/stamp-48.png" }
                        ],
                        "accessibility": {
                            "accessibilityData": { "label": "手を振るスタンプ" }
                        }
                    }
                }
            },
            { "text": " 楽しみ！" }
        ]);

        let (text, parsed) = extract_runs_with_emojis(&runs);
        assert_eq!(text, "こんちゃ！:_custom_wave: 楽しみ！");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].kind, MessageRunKind::Text);
        assert_eq!(parsed[1].kind, MessageRunKind::Emoji);
        assert!(parsed[1].is_custom_emoji);
        assert_eq!(
            parsed[1].emoji_url.as_deref(),
            Some("https://yt3.ggpht.com/stamp-48.png")
        );
        assert_eq!(parsed[1].alt.as_deref(), Some("手を振るスタンプ"));
    }

    #[test]
    fn test_parse_channel_live_html() {
        let html = r#"<html><head><title>雑談ライブ配信 - YouTube</title><link rel="canonical" href="https://www.youtube.com/watch?v=AbCdEfGhIjK"></head><body>{"isLiveNow":true}</body></html>"#;
        let resolved = parse_channel_live_html(html, "@mych").unwrap();
        assert_eq!(resolved.video_id, "AbCdEfGhIjK");
        assert_eq!(resolved.title, "雑談ライブ配信");
        assert!(resolved.is_live);
    }

    #[test]
    fn test_bounded_id_set_fifo_eviction() {
        let mut set = BoundedIdSet::new(3);
        assert!(set.insert("id-1".to_string()));
        assert!(set.insert("id-2".to_string()));
        assert!(set.insert("id-3".to_string()));
        // 重複は拒否される
        assert!(!set.insert("id-2".to_string()));
        // 上限超過時は最古 (id-1) のみ追い出し、直近 (id-2, id-3) は保持
        assert!(set.insert("id-4".to_string()));
        assert!(!set.insert("id-2".to_string()));
        assert!(!set.insert("id-3".to_string()));
        assert!(!set.insert("id-4".to_string()));
        assert!(set.insert("id-1".to_string()));
    }
}
