use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::{broadcast, oneshot, RwLock};
use tower_http::cors::CorsLayer;

use crate::models::{CommentItem, LotteryResult, OverlayConfig, WordCounterConfig};

const COMMENTS_HTML: &str = include_str!("comments.html");
const LOTTERY_HTML: &str = include_str!("lottery.html");
const COUNTER_HTML: &str = include_str!("counter.html");
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
    },
    Comment {
        comment: CommentItem,
    },
    Counters {
        counters: Vec<WordCounterConfig>,
    },
    LotteryResult {
        result: LotteryResult,
    },
    ClearLottery,
    ConfigUpdated {
        config: OverlayConfig,
    },
}

pub struct OverlaySharedState {
    pub config: RwLock<OverlayConfig>,
    pub recent_comments: RwLock<Vec<CommentItem>>,
    pub counters: RwLock<Vec<WordCounterConfig>>,
    pub last_lottery: RwLock<Option<LotteryResult>>,
    pub tx: broadcast::Sender<String>,
}

impl OverlaySharedState {
    pub fn new(config: OverlayConfig, counters: Vec<WordCounterConfig>) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            config: RwLock::new(config),
            recent_comments: RwLock::new(Vec::new()),
            counters: RwLock::new(counters),
            last_lottery: RwLock::new(None),
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

    pub async fn show_lottery(&self, result: LotteryResult) {
        *self.last_lottery.write().await = Some(result.clone());
        self.broadcast(&OverlayMessage::LotteryResult { result });
    }

    pub async fn clear_lottery(&self) {
        *self.last_lottery.write().await = None;
        self.broadcast(&OverlayMessage::ClearLottery);
    }

    pub async fn update_config(&self, config: OverlayConfig) {
        *self.config.write().await = config.clone();
        self.broadcast(&OverlayMessage::ConfigUpdated { config });
    }
}

pub struct OverlayServerHandle {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OverlayServerHandle {
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// ローカル HTTP + WebSocket サーバーを起動する
pub async fn start_overlay_server(
    port: u16,
    shared: Arc<OverlaySharedState>,
) -> Result<OverlayServerHandle> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/favicon.ico", get(favicon_handler))
        .route("/overlay/comments", get(|| async { Html(COMMENTS_HTML) }))
        .route("/overlay/lottery", get(|| async { Html(LOTTERY_HTML) }))
        .route("/overlay/counter", get(|| async { Html(COUNTER_HTML) }))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(shared);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    Ok(OverlayServerHandle {
        port: actual_port,
        shutdown_tx: Some(shutdown_tx),
    })
}

async fn favicon_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "image/x-icon")],
        APP_ICON_ICO,
    )
}

async fn index_handler() -> Html<&'static str> {
    Html(
        "<!DOCTYPE html><html lang='ja'><head><meta charset='utf-8'><link rel='icon' href='/favicon.ico'><title>Stosup OBS オーバーレイ</title></head>\
         <body style='font-family:sans-serif;background:#0f172a;color:#f8fafc;padding:32px'>\
         <h1>Stosup OBS オーバーレイサーバー稼働中</h1>\
         <ul>\
           <li><a style='color:#38bdf8' href='/overlay/comments'>/overlay/comments (コメント表示)</a></li>\
           <li><a style='color:#38bdf8' href='/overlay/lottery'>/overlay/lottery (抽選演出・結果表示)</a></li>\
           <li><a style='color:#38bdf8' href='/overlay/counter'>/overlay/counter (ワードカウンター表示)</a></li>\
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

    // 接続直後に最新スナップショットを配信
    let snapshot = OverlayMessage::Snapshot {
        config: state.config.read().await.clone(),
        comments: state.recent_comments.read().await.clone(),
        counters: state.counters.read().await.clone(),
        last_lottery: state.last_lottery.read().await.clone(),
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
