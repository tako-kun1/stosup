mod remote_api;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Response, StatusCode};
use axum::middleware;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::{broadcast, oneshot, watch, RwLock};
use tower_http::cors::CorsLayer;

use crate::media::{content_type_for_file_name, is_safe_stored_file_name};
use crate::models::{
    CommentItem, FirstTimeCounterState, LotteryResult, MediaReactionConfig, MediaTriggerEvent,
    MonetizationItem, OverlayConfig, PickedCommentState, PollState, QueueState, RemoteAccessConfig,
    WordCounterConfig,
};

const COMMENTS_HTML: &str = include_str!("comments.html");
const LOTTERY_HTML: &str = include_str!("lottery.html");
const COUNTER_HTML: &str = include_str!("counter.html");
const FIRST_TIME_HTML: &str = include_str!("first_time.html");
const SUPERCHAT_HTML: &str = include_str!("superchat.html");
const POLL_HTML: &str = include_str!("poll.html");
const PICKUP_HTML: &str = include_str!("pickup.html");
const QUEUE_HTML: &str = include_str!("queue.html");
const MEDIA_HTML: &str = include_str!("media.html");
const APP_ICON_ICO: &[u8] = include_bytes!("../../../windows/runner/resources/app_icon.ico");

/// OBS オーバーレイ WebSocket の購読チャンネル種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayChannel {
    All,
    Comments,
    SuperChat,
    Pickup,
    Poll,
    Queue,
    Media,
    Lottery,
    Counter,
    FirstTime,
}

impl OverlayChannel {
    pub fn from_query(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "comments" => Some(Self::Comments),
            "superchat" => Some(Self::SuperChat),
            "pickup" => Some(Self::Pickup),
            "poll" => Some(Self::Poll),
            "queue" => Some(Self::Queue),
            "media" => Some(Self::Media),
            "lottery" => Some(Self::Lottery),
            "counter" => Some(Self::Counter),
            "first_time" => Some(Self::FirstTime),
            _ => None,
        }
    }

    /// 配信メッセージのチャンネルがクライアントの購読フィルタに一致するか判定
    pub fn matches(self, filter: Option<Self>) -> bool {
        matches!(
            (self, filter),
            (Self::All, _) | (_, None) | (_, Some(Self::All))
        ) || filter == Some(self)
    }
}

/// オーバーレイ画面へ WebSocket で送信するメッセージ
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OverlayMessage {
    Snapshot {
        config: OverlayConfig,
        comments: Vec<CommentItem>,
        counters: Vec<WordCounterConfig>,
        last_lottery: Option<LotteryResult>,
        first_time: FirstTimeCounterState,
        first_time_enabled: bool,
        monetization_items: Vec<MonetizationItem>,
        superchat_enabled: bool,
        poll_state: PollState,
        poll_enabled: bool,
        picked_comment: PickedCommentState,
        pickup_enabled: bool,
        queue_state: QueueState,
        queue_enabled: bool,
        media_config: MediaReactionConfig,
        media_enabled: bool,
    },
    Comment {
        comment: CommentItem,
    },
    Counters {
        counters: Vec<WordCounterConfig>,
    },
    FirstTime {
        first_time: FirstTimeCounterState,
        enabled: bool,
    },
    LotteryResult {
        result: LotteryResult,
    },
    ClearLottery,
    SuperChat {
        items: Vec<MonetizationItem>,
        enabled: bool,
    },
    Poll {
        poll: PollState,
        enabled: bool,
    },
    ClearPoll,
    PickedComment {
        picked: PickedCommentState,
        enabled: bool,
    },
    Queue {
        queue: QueueState,
        enabled: bool,
    },
    MediaTrigger {
        trigger: MediaTriggerEvent,
    },
    MediaConfig {
        media_config: MediaReactionConfig,
        enabled: bool,
    },
    ClearMedia,
    ConfigUpdated {
        config: OverlayConfig,
    },
}

impl OverlayMessage {
    pub fn channel(&self) -> OverlayChannel {
        match self {
            OverlayMessage::Snapshot { .. } | OverlayMessage::ConfigUpdated { .. } => {
                OverlayChannel::All
            }
            OverlayMessage::Comment { .. } => OverlayChannel::Comments,
            OverlayMessage::Counters { .. } => OverlayChannel::Counter,
            OverlayMessage::FirstTime { .. } => OverlayChannel::FirstTime,
            OverlayMessage::LotteryResult { .. } | OverlayMessage::ClearLottery => {
                OverlayChannel::Lottery
            }
            OverlayMessage::SuperChat { .. } => OverlayChannel::SuperChat,
            OverlayMessage::Poll { .. } | OverlayMessage::ClearPoll => OverlayChannel::Poll,
            OverlayMessage::PickedComment { .. } => OverlayChannel::Pickup,
            OverlayMessage::Queue { .. } => OverlayChannel::Queue,
            OverlayMessage::MediaTrigger { .. }
            | OverlayMessage::MediaConfig { .. }
            | OverlayMessage::ClearMedia => OverlayChannel::Media,
        }
    }
}

pub struct OverlaySharedState {
    pub media_dir: PathBuf,
    pub remote_config: RwLock<RemoteAccessConfig>,
    pub config: RwLock<OverlayConfig>,
    pub recent_comments: RwLock<Vec<CommentItem>>,
    pub counters: RwLock<Vec<WordCounterConfig>>,
    pub last_lottery: RwLock<Option<LotteryResult>>,
    pub first_time: RwLock<FirstTimeCounterState>,
    pub first_time_enabled: RwLock<bool>,
    pub monetization_items: RwLock<Vec<MonetizationItem>>,
    pub superchat_enabled: RwLock<bool>,
    pub poll_state: RwLock<PollState>,
    pub poll_enabled: RwLock<bool>,
    pub picked_comment: RwLock<PickedCommentState>,
    pub pickup_enabled: RwLock<bool>,
    pub queue_state: RwLock<QueueState>,
    pub queue_enabled: RwLock<bool>,
    pub media_config: RwLock<MediaReactionConfig>,
    pub media_enabled: RwLock<bool>,
    pub obs_overlay_enabled: AtomicBool,
    obs_enabled_tx: watch::Sender<bool>,
    pub tx: broadcast::Sender<Arc<(OverlayChannel, String)>>,
}

impl OverlaySharedState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        media_dir: PathBuf,
        remote_config: RemoteAccessConfig,
        config: OverlayConfig,
        counters: Vec<WordCounterConfig>,
        first_time: FirstTimeCounterState,
        first_time_enabled: bool,
        monetization_items: Vec<MonetizationItem>,
        superchat_enabled: bool,
        poll_state: PollState,
        poll_enabled: bool,
        queue_state: QueueState,
        queue_enabled: bool,
        media_config: MediaReactionConfig,
        media_enabled: bool,
        pickup_enabled: bool,
    ) -> Self {
        let (tx, _) = broadcast::channel(512);
        let (obs_enabled_tx, _) = watch::channel(true);
        Self {
            media_dir,
            remote_config: RwLock::new(remote_config),
            config: RwLock::new(config),
            recent_comments: RwLock::new(Vec::new()),
            counters: RwLock::new(counters),
            last_lottery: RwLock::new(None),
            first_time: RwLock::new(first_time),
            first_time_enabled: RwLock::new(first_time_enabled),
            monetization_items: RwLock::new(monetization_items),
            superchat_enabled: RwLock::new(superchat_enabled),
            poll_state: RwLock::new(poll_state),
            poll_enabled: RwLock::new(poll_enabled),
            picked_comment: RwLock::new(PickedCommentState::default()),
            pickup_enabled: RwLock::new(pickup_enabled),
            queue_state: RwLock::new(queue_state),
            queue_enabled: RwLock::new(queue_enabled),
            media_config: RwLock::new(media_config),
            media_enabled: RwLock::new(media_enabled),
            obs_overlay_enabled: AtomicBool::new(true),
            obs_enabled_tx,
            tx,
        }
    }

    /// OBS オーバーレイ配信の有効 / 無効を切り替える（OFF 時は接続中の全 OBS WebSocket を即時切断）
    pub fn set_obs_overlay_enabled(&self, enabled: bool) {
        self.obs_overlay_enabled.store(enabled, Ordering::Relaxed);
        let _ = self.obs_enabled_tx.send(enabled);
    }

    pub fn is_obs_overlay_enabled(&self) -> bool {
        self.obs_overlay_enabled.load(Ordering::Relaxed)
    }

    pub fn broadcast(&self, msg: &OverlayMessage) {
        // OBS オーバーレイ無効または接続クライアント 0 件のときは JSON シリアライズ自体をスキップ
        if !self.is_obs_overlay_enabled() || self.tx.receiver_count() == 0 {
            return;
        }
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.tx.send(Arc::new((msg.channel(), json)));
        }
    }

    /// 接続先オーバーレイのチャンネルに必要な項目だけを詰め込んだ軽量 Snapshot を構築する
    pub async fn build_snapshot(&self, channel: Option<OverlayChannel>) -> OverlayMessage {
        let include = |ch: OverlayChannel| ch.matches(channel);
        OverlayMessage::Snapshot {
            config: self.config.read().await.clone(),
            comments: if include(OverlayChannel::Comments) {
                self.recent_comments.read().await.clone()
            } else {
                Vec::new()
            },
            counters: if include(OverlayChannel::Counter) {
                self.counters.read().await.clone()
            } else {
                Vec::new()
            },
            last_lottery: if include(OverlayChannel::Lottery) {
                self.last_lottery.read().await.clone()
            } else {
                None
            },
            first_time: if include(OverlayChannel::FirstTime) {
                self.first_time.read().await.clone()
            } else {
                FirstTimeCounterState::default()
            },
            first_time_enabled: if include(OverlayChannel::FirstTime) {
                *self.first_time_enabled.read().await
            } else {
                false
            },
            monetization_items: if include(OverlayChannel::SuperChat) {
                self.monetization_items.read().await.clone()
            } else {
                Vec::new()
            },
            superchat_enabled: if include(OverlayChannel::SuperChat) {
                *self.superchat_enabled.read().await
            } else {
                false
            },
            poll_state: if include(OverlayChannel::Poll) {
                self.poll_state.read().await.clone()
            } else {
                PollState::default()
            },
            poll_enabled: if include(OverlayChannel::Poll) {
                *self.poll_enabled.read().await
            } else {
                false
            },
            picked_comment: if include(OverlayChannel::Pickup) {
                self.picked_comment.read().await.clone()
            } else {
                PickedCommentState::default()
            },
            pickup_enabled: if include(OverlayChannel::Pickup) {
                *self.pickup_enabled.read().await
            } else {
                false
            },
            queue_state: if include(OverlayChannel::Queue) {
                self.queue_state.read().await.clone()
            } else {
                QueueState::default()
            },
            queue_enabled: if include(OverlayChannel::Queue) {
                *self.queue_enabled.read().await
            } else {
                false
            },
            media_config: if include(OverlayChannel::Media) {
                self.media_config.read().await.clone()
            } else {
                MediaReactionConfig::default()
            },
            media_enabled: if include(OverlayChannel::Media) {
                *self.media_enabled.read().await
            } else {
                false
            },
        }
    }

    pub async fn push_comment(&self, comment: CommentItem) {
        let max_items = self.config.read().await.comment_max_items.max(10) as usize;
        {
            let mut list = self.recent_comments.write().await;
            list.push(comment.clone());
            if list.len() > max_items {
                let drop_count = list.len() - max_items;
                list.drain(0..drop_count);
            }
        }
        self.broadcast(&OverlayMessage::Comment { comment });
    }

    pub async fn update_counters(&self, counters: Vec<WordCounterConfig>) {
        *self.counters.write().await = counters.clone();
        self.broadcast(&OverlayMessage::Counters { counters });
    }

    pub async fn update_first_time(&self, state: FirstTimeCounterState, enabled: bool) {
        *self.first_time.write().await = state.clone();
        *self.first_time_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::FirstTime {
            first_time: state,
            enabled,
        });
    }

    pub async fn show_lottery(&self, result: LotteryResult) {
        *self.last_lottery.write().await = Some(result.clone());
        self.broadcast(&OverlayMessage::LotteryResult { result });
    }

    pub async fn clear_lottery(&self) {
        *self.last_lottery.write().await = None;
        self.broadcast(&OverlayMessage::ClearLottery);
    }

    pub async fn update_monetization(&self, items: Vec<MonetizationItem>, enabled: bool) {
        *self.monetization_items.write().await = items.clone();
        *self.superchat_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::SuperChat { items, enabled });
    }

    pub async fn update_poll(&self, poll: PollState, enabled: bool) {
        *self.poll_state.write().await = poll.clone();
        *self.poll_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::Poll { poll, enabled });
    }

    pub async fn clear_poll(&self) {
        self.broadcast(&OverlayMessage::ClearPoll);
    }

    pub async fn update_picked_comment(&self, picked: PickedCommentState, enabled: bool) {
        *self.picked_comment.write().await = picked.clone();
        *self.pickup_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::PickedComment { picked, enabled });
    }

    pub async fn update_queue(&self, queue: QueueState, enabled: bool) {
        *self.queue_state.write().await = queue.clone();
        *self.queue_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::Queue { queue, enabled });
    }

    pub async fn update_media_config(&self, media_config: MediaReactionConfig, enabled: bool) {
        *self.media_config.write().await = media_config.clone();
        *self.media_enabled.write().await = enabled;
        self.broadcast(&OverlayMessage::MediaConfig {
            media_config,
            enabled,
        });
    }

    pub fn trigger_media(&self, trigger: MediaTriggerEvent) {
        self.broadcast(&OverlayMessage::MediaTrigger { trigger });
    }

    pub fn clear_media(&self) {
        self.broadcast(&OverlayMessage::ClearMedia);
    }

    pub async fn update_config(&self, config: OverlayConfig) {
        *self.config.write().await = config.clone();
        self.broadcast(&OverlayMessage::ConfigUpdated { config });
    }
}

pub struct OverlayServerHandle {
    pub port: u16,
    pub allow_lan_access: bool,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OverlayServerHandle {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// ローカル HTTP + WebSocket サーバーを起動する（allow_lan = true 時は 0.0.0.0 で LAN 公開）
pub fn start_overlay_server(
    port: u16,
    allow_lan: bool,
    shared: Arc<OverlaySharedState>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OverlayServerHandle>> + Send>> {
    Box::pin(async move {
        let bind_ip = if allow_lan {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        };
        let addr = SocketAddr::from((bind_ip, port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let actual_port = listener.local_addr()?.port();

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(favicon_handler))
            .route("/remote", get(remote_api::remote_page_handler))
            .route("/api/remote/auth_info", get(remote_api::auth_info_handler))
            .route("/api/remote/verify", post(remote_api::verify_pin_handler))
            .route("/api/remote/state", get(remote_api::remote_state_handler))
            .route("/api/remote/action", post(remote_api::remote_action_handler))
            .route(
                "/api/remote/upload_media",
                post(remote_api::upload_media_handler)
                    .layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
            )
            .route("/ws/remote", get(remote_api::remote_ws_handler))
            .route(
                "/overlay/comments",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COMMENTS_HTML)
                }),
            )
            .route(
                "/overlay/lottery",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, LOTTERY_HTML)
                }),
            )
            .route(
                "/overlay/counter",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, COUNTER_HTML)
                }),
            )
            .route(
                "/overlay/first_time",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, FIRST_TIME_HTML)
                }),
            )
            .route(
                "/overlay/superchat",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, SUPERCHAT_HTML)
                }),
            )
            .route(
                "/overlay/poll",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, POLL_HTML)
                }),
            )
            .route(
                "/overlay/pickup",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, PICKUP_HTML)
                }),
            )
            .route(
                "/overlay/queue",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, QUEUE_HTML)
                }),
            )
            .route(
                "/overlay/media",
                get(|st: State<Arc<OverlaySharedState>>| async move {
                    serve_overlay_html(st, MEDIA_HTML)
                }),
            )
            .route("/media/{file_name}", get(media_file_handler))
            .route("/ws", get(ws_handler))
            .layer(middleware::from_fn_with_state(
                Arc::clone(&shared),
                remote_api::ip_access_guard,
            ))
            .layer(CorsLayer::permissive())
            .with_state(shared);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        });

        Ok(OverlayServerHandle {
            port: actual_port,
            allow_lan_access: allow_lan,
            shutdown_tx: Some(shutdown_tx),
        })
    })
}

fn serve_overlay_html(
    State(state): State<Arc<OverlaySharedState>>,
    html: &'static str,
) -> (StatusCode, Html<&'static str>) {
    if !state.is_obs_overlay_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(
                "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><title>OBS オーバーレイ停止中</title></head>\
                 <body style='margin:0;background:transparent;color:#a1a1aa;font-family:sans-serif;padding:12px;font-size:13px'>\
                 Stosup の OBS オーバーレイ機能は現在オフになっています。</body></html>",
            ),
        );
    }
    (StatusCode::OK, Html(html))
}

async fn favicon_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        APP_ICON_ICO,
    )
}

/// ローカル保存済みメディアファイルを配信する（Range シーク読み出し対応で大容量動画も高速・省メモリ）
async fn media_file_handler(
    Path(file_name): Path<String>,
    headers: HeaderMap,
    State(state): State<Arc<OverlaySharedState>>,
) -> Response<Body> {
    if !state.is_obs_overlay_enabled() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Body::from("OBS overlay is disabled"))
            .unwrap();
    }
    if !is_safe_stored_file_name(&file_name) {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid file name"))
            .unwrap();
    }

    let path = state.media_dir.join(&file_name);
    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Media not found"))
                .unwrap();
        }
    };

    let total_len = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Metadata error"))
                .unwrap();
        }
    };

    let content_type = content_type_for_file_name(&file_name);
    let cache_control = "public, max-age=86400, immutable";

    if let Some(range_val) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(range_spec) = range_val.strip_prefix("bytes=") {
            let mut parts = range_spec.split('-');
            let start = parts
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let requested_end = parts
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|e| e.min(total_len.saturating_sub(1)));

            if total_len > 0 && start < total_len {
                // Range 指定時は最大 2MB チャンクで必要部分のみ seek 読み出し
                const MAX_CHUNK: u64 = 2 * 1024 * 1024;
                let end = match requested_end {
                    Some(e) if e >= start => e.min(start.saturating_add(MAX_CHUNK - 1)),
                    _ => (total_len - 1).min(start.saturating_add(MAX_CHUNK - 1)),
                };
                let chunk_len = (end - start + 1) as usize;
                let mut buf = vec![0u8; chunk_len];
                if file.seek(SeekFrom::Start(start)).await.is_ok()
                    && file.read_exact(&mut buf).await.is_ok()
                {
                    return Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::ACCEPT_RANGES, "bytes")
                        .header(
                            header::CONTENT_RANGE,
                            format!("bytes {}-{}/{}", start, end, total_len),
                        )
                        .header(header::CONTENT_LENGTH, buf.len())
                        .header(header::CACHE_CONTROL, cache_control)
                        .body(Body::from(buf))
                        .unwrap();
                }
            }
        }
    }

    let mut data = Vec::with_capacity(total_len.min(32 * 1024 * 1024) as usize);
    if file.read_to_end(&mut data).await.is_err() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Read error"))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, cache_control)
        .body(Body::from(data))
        .unwrap()
}

async fn index_handler() -> Html<&'static str> {
    Html(
        "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><link rel='icon' href='/favicon.ico'><title>Stosup OBS オーバーレイ</title></head>\
         <body style='font-family:sans-serif;background:#18181b;color:#f4f4f5;padding:32px'>\
         <h1>Stosup OBS オーバーレイ ＆ Web リモコンサーバー稼働中</h1>\
         <p style='margin:12px 0'><a style='color:#4ade80;font-weight:bold;font-size:18px' href='/remote'>▶ Web リモートコントロールを開く (/remote)</a></p>\
         <ul>\
           <li><a style='color:#60a5fa' href='/overlay/comments'>/overlay/comments (コメント表示)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/superchat'>/overlay/superchat (スパチャ・メンバー表示)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/pickup'>/overlay/pickup (コメントピックアップ表示)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/poll'>/overlay/poll (リアルタイム投票・アンケート)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/queue'>/overlay/queue (参加型キュー順番待ち)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/media'>/overlay/media (特定ワード反応アニメーション・動画)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/lottery'>/overlay/lottery (抽選演出・結果表示)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/counter'>/overlay/counter (ワードカウンター表示)</a></li>\
           <li><a style='color:#60a5fa' href='/overlay/first_time'>/overlay/first_time (初見カウンター表示)</a></li>\
         </ul></body></html>",
    )
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<HashMap<String, String>>,
    State(state): State<Arc<OverlaySharedState>>,
) -> axum::response::Response {
    if !state.is_obs_overlay_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "OBS overlay is currently disabled",
        )
            .into_response();
    }
    let channel = query
        .get("channel")
        .and_then(|s| OverlayChannel::from_query(s));
    ws.on_upgrade(move |socket| handle_socket(socket, state, channel))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<OverlaySharedState>,
    channel: Option<OverlayChannel>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut obs_rx = state.obs_enabled_tx.subscribe();
    if !*obs_rx.borrow() {
        return;
    }

    let snapshot = state.build_snapshot(channel).await;
    if let Ok(json) = serde_json::to_string(&snapshot) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    let mut rx = state.tx.subscribe();

    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(payload) => {
                    if !payload.0.matches(channel) {
                        continue;
                    }
                    if sender
                        .send(Message::Text(payload.1.as_str().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
        _ = obs_rx.wait_for(|&enabled| !enabled) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlatformKind;

    fn make_test_state() -> OverlaySharedState {
        OverlaySharedState::new(
            PathBuf::from("."),
            RemoteAccessConfig::default(),
            OverlayConfig::default(),
            Vec::new(),
            FirstTimeCounterState::default(),
            true,
            Vec::new(),
            true,
            PollState::default(),
            true,
            QueueState::default(),
            true,
            MediaReactionConfig::default(),
            true,
            true,
        )
    }

    #[test]
    fn test_overlay_channel_query_and_matching() {
        assert_eq!(OverlayChannel::from_query("media"), Some(OverlayChannel::Media));
        assert_eq!(
            OverlayChannel::from_query("first_time"),
            Some(OverlayChannel::FirstTime)
        );
        assert_eq!(OverlayChannel::from_query("unknown"), None);

        // 特定チャンネルは同一チャンネルと All(ConfigUpdated) のみ受信
        assert!(OverlayChannel::Comments.matches(Some(OverlayChannel::Comments)));
        assert!(!OverlayChannel::Comments.matches(Some(OverlayChannel::Media)));
        assert!(OverlayChannel::All.matches(Some(OverlayChannel::Media)));

        // channel 未指定 (None) の外部クライアントは後方互換で全チャンネル受信
        assert!(OverlayChannel::Comments.matches(None));
        assert!(OverlayChannel::Media.matches(None));
    }

    #[tokio::test]
    async fn test_overlay_snapshot_channel_pruning_and_broadcast() {
        let state = make_test_state();
        state
            .push_comment(CommentItem {
                id: "c1".to_string(),
                session_id: "s1".to_string(),
                platform: PlatformKind::Demo,
                user_id: "u1".to_string(),
                user_name: "Tester".to_string(),
                user_icon_url: String::new(),
                user_memo: String::new(),
                is_first_time: false,
                message: "hello".to_string(),
                timestamp_ms: 1000,
                is_owner: false,
                is_moderator: false,
                is_member: false,
                is_banned: false,
                highlight_rule_id: None,
                bubble_bg_color: None,
                bubble_text_color: None,
                bubble_border_color: None,
                event_kind: None,
                amount_display: None,
                event_accent_color: None,
            })
            .await;

        // media チャンネル向け Snapshot には comments が含まれない（軽量化）
        match state.build_snapshot(Some(OverlayChannel::Media)).await {
            OverlayMessage::Snapshot { comments, .. } => assert!(comments.is_empty()),
            _ => panic!("expected Snapshot"),
        }
        // comments チャンネル向け Snapshot には comments が含まれる
        match state.build_snapshot(Some(OverlayChannel::Comments)).await {
            OverlayMessage::Snapshot { comments, .. } => assert_eq!(comments.len(), 1),
            _ => panic!("expected Snapshot"),
        }

        // 購読中チャンネルのブロードキャストに正しいチャンネルタグが付与される
        let mut rx = state.tx.subscribe();
        state.clear_media();
        let payload = rx.recv().await.unwrap();
        assert_eq!(payload.0, OverlayChannel::Media);
        assert!(payload.1.contains("clear_media"));

        // OBS オーバーレイ無効化後はブロードキャストが停止され、切断シグナルも送出される
        let obs_rx = state.obs_enabled_tx.subscribe();
        state.set_obs_overlay_enabled(false);
        assert!(!state.is_obs_overlay_enabled());
        assert!(!*obs_rx.borrow());
        state.clear_media();
        assert!(rx.try_recv().is_err());
    }
}
