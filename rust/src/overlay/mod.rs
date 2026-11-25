mod remote_api;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, Response, StatusCode};
use axum::middleware;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::{broadcast, oneshot, RwLock};
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
    pub tx: broadcast::Sender<String>,
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
            tx,
        }
    }

    pub fn broadcast(&self, msg: &OverlayMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.tx.send(json);
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
            .route("/overlay/comments", get(|| async { Html(COMMENTS_HTML) }))
            .route("/overlay/lottery", get(|| async { Html(LOTTERY_HTML) }))
            .route("/overlay/counter", get(|| async { Html(COUNTER_HTML) }))
            .route("/overlay/first_time", get(|| async { Html(FIRST_TIME_HTML) }))
            .route("/overlay/superchat", get(|| async { Html(SUPERCHAT_HTML) }))
            .route("/overlay/poll", get(|| async { Html(POLL_HTML) }))
            .route("/overlay/pickup", get(|| async { Html(PICKUP_HTML) }))
            .route("/overlay/queue", get(|| async { Html(QUEUE_HTML) }))
            .route("/overlay/media", get(|| async { Html(MEDIA_HTML) }))
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
    State(state): State<Arc<OverlaySharedState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<OverlaySharedState>) {
    let (mut sender, mut receiver) = socket.split();

    let snapshot = OverlayMessage::Snapshot {
        config: state.config.read().await.clone(),
        comments: state.recent_comments.read().await.clone(),
        counters: state.counters.read().await.clone(),
        last_lottery: state.last_lottery.read().await.clone(),
        first_time: state.first_time.read().await.clone(),
        first_time_enabled: *state.first_time_enabled.read().await,
        monetization_items: state.monetization_items.read().await.clone(),
        superchat_enabled: *state.superchat_enabled.read().await,
        poll_state: state.poll_state.read().await.clone(),
        poll_enabled: *state.poll_enabled.read().await,
        picked_comment: state.picked_comment.read().await.clone(),
        pickup_enabled: *state.pickup_enabled.read().await,
        queue_state: state.queue_state.read().await.clone(),
        queue_enabled: *state.queue_enabled.read().await,
        media_config: state.media_config.read().await.clone(),
        media_enabled: *state.media_enabled.read().await,
    };
    if let Ok(json) = serde_json::to_string(&snapshot) {
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    let mut rx = state.tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
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
    }
}
