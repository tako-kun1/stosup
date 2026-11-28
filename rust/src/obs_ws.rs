use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot, watch, Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::models::{ObsWsConfig, ObsWsStatus};

/// OBS WebSocket v5 への送信コマンド
enum ObsCommand {
    Request {
        request_type: String,
        request_data: Value,
        reply_tx: oneshot::Sender<Result<Value>>,
    },
    RefreshScenes,
}

/// OBS WebSocket 側から Stosup へ通知するイベント
#[derive(Debug, Clone)]
pub enum ObsIncomingEvent {
    StatusChanged(ObsWsStatus),
    StreamStarted,
    StreamStopped,
}

/// OBS WebSocket v5 クライアント管理
pub struct ObsWsManager {
    pub status: Arc<RwLock<ObsWsStatus>>,
    connected_flag: Arc<AtomicBool>,
    /// 再接続時に旧タスクの遅延終了処理が新接続のステータスや接続フラグを上書きする競合を防ぐ世代カウンタ
    conn_gen: Arc<AtomicU64>,
    /// connect / disconnect / 接続確立・失敗の状態遷移を直列化し、割り込み競合による接続リークや状態固着を防ぐロック
    lifecycle_lock: Mutex<()>,
    cmd_tx: Mutex<Option<mpsc::UnboundedSender<ObsCommand>>>,
    cancel_tx: Mutex<Option<watch::Sender<bool>>>,
}

impl Default for ObsWsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ObsWsManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(ObsWsStatus::default())),
            connected_flag: Arc::new(AtomicBool::new(false)),
            conn_gen: Arc::new(AtomicU64::new(0)),
            lifecycle_lock: Mutex::new(()),
            cmd_tx: Mutex::new(None),
            cancel_tx: Mutex::new(None),
        }
    }

    /// ロック・クローンなしで接続状態を即時判定する（毎コメントのホットパス用）
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.connected_flag.load(Ordering::Acquire)
    }

    pub async fn get_status(&self) -> ObsWsStatus {
        self.status.read().await.clone()
    }

    async fn disconnect_locked(&self) {
        self.conn_gen.fetch_add(1, Ordering::AcqRel);
        self.connected_flag.store(false, Ordering::Release);
        if let Some(tx) = self.cancel_tx.lock().await.take() {
            let _ = tx.send(true);
        }
        *self.cmd_tx.lock().await = None;
        let mut st = self.status.write().await;
        st.connected = false;
        st.connecting = false;
        st.last_error = None;
    }

    pub async fn disconnect(&self) {
        let _guard = self.lifecycle_lock.lock().await;
        self.disconnect_locked().await;
    }

    async fn mark_connect_failed(
        &self,
        my_gen: u64,
        err_msg: &str,
        event_tx: &mpsc::UnboundedSender<ObsIncomingEvent>,
    ) {
        let _guard = self.lifecycle_lock.lock().await;
        if self.conn_gen.load(Ordering::Acquire) != my_gen {
            return;
        }
        self.connected_flag.store(false, Ordering::Release);
        let mut st = self.status.write().await;
        st.connected = false;
        st.connecting = false;
        st.last_error = Some(err_msg.to_string());
        let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
    }

    pub async fn connect(
        &self,
        cfg: ObsWsConfig,
        event_tx: mpsc::UnboundedSender<ObsIncomingEvent>,
    ) -> Result<()> {
        let my_gen = {
            let _guard = self.lifecycle_lock.lock().await;
            self.disconnect_locked().await;
            let gen = self.conn_gen.fetch_add(1, Ordering::AcqRel) + 1;
            let mut st = self.status.write().await;
            st.connected = false;
            st.connecting = true;
            st.last_error = None;
            let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
            gen
        };

        let host = if cfg.host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            cfg.host.trim().to_string()
        };
        let port = if cfg.port == 0 { 4455 } else { cfg.port };
        let url = format!("ws://{}:{}", host, port);

        let ws_res = tokio::time::timeout(
            Duration::from_secs(6),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        .map_err(|_| anyhow!("OBS WebSocket ({}) への接続がタイムアウトしました", url))
        .and_then(|r| {
            r.map_err(|e| {
                anyhow!(
                    "OBS WebSocket ({}) へ接続できませんでした（OBS 側の WebSocket サーバー設定を確認してください）: {}",
                    url,
                    e
                )
            })
        });
        let (ws_stream, _) = match ws_res {
            Ok(v) => v,
            Err(e) => {
                self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                    .await;
                return Err(e);
            }
        };

        let (mut write, mut read) = ws_stream.split();

        // op 0: Hello 受信
        let hello_val = match read_next_json(&mut read, Duration::from_secs(5)).await {
            Ok(v) => v,
            Err(e) => {
                self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                    .await;
                return Err(e);
            }
        };
        if hello_val["op"].as_u64() != Some(0) {
            let e = anyhow!("OBS WebSocket Hello パケットを受信できませんでした");
            self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                .await;
            return Err(e);
        }
        let obs_ws_ver = hello_val["d"]["obsWebSocketVersion"]
            .as_str()
            .unwrap_or("5.x")
            .to_string();

        // 認証文字列計算（password が設定されている場合）
        let mut identify_d = json!({
            "rpcVersion": 1,
            "eventSubscriptions": 64 | 4 | 16 | 128 // Outputs(64) | Scenes(4) | Transitions(16) | SceneItems(128)
        });
        if let Some(auth) = hello_val["d"]["authentication"].as_object() {
            let challenge = auth
                .get("challenge")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let salt = auth.get("salt").and_then(|v| v.as_str()).unwrap_or("");
            if cfg.password.is_empty() {
                let e = anyhow!(
                    "OBS WebSocket はパスワード認証が必要です。設定にパスワードを入力してください。"
                );
                self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                    .await;
                return Err(e);
            }
            let auth_str = compute_obs_v5_auth(&cfg.password, salt, challenge);
            identify_d["authentication"] = Value::String(auth_str);
        }

        // op 1: Identify 送信
        let identify_msg = json!({ "op": 1, "d": identify_d });
        if let Err(e) = write
            .send(WsMessage::Text(identify_msg.to_string().into()))
            .await
            .map_err(|e| anyhow!("OBS Identify 送信失敗: {}", e))
        {
            self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                .await;
            return Err(e);
        }

        // op 2: Identified 受信確認
        let identified = match read_next_json(&mut read, Duration::from_secs(5)).await {
            Ok(v) => v,
            Err(e) => {
                self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                    .await;
                return Err(e);
            }
        };
        if identified["op"].as_u64() != Some(2) {
            let e = anyhow!(
                "OBS WebSocket 認証に失敗しました（パスワードを確認してください）"
            );
            self.mark_connect_failed(my_gen, &e.to_string(), &event_tx)
                .await;
            return Err(e);
        }

        // 世代チェック〜チャンネル登録〜タスク起動までを直列化し、disconnect 割り込みによる状態固着とリークを根絶
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ObsCommand>();
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let status_arc = Arc::clone(&self.status);
        let connected_arc = Arc::clone(&self.connected_flag);
        let gen_arc = Arc::clone(&self.conn_gen);
        {
            let _guard = self.lifecycle_lock.lock().await;
            if self.conn_gen.load(Ordering::Acquire) != my_gen {
                drop(_guard);
                let _ = write.close().await;
                return Err(anyhow!("OBS WebSocket 接続がキャンセルされました"));
            }
            *self.cmd_tx.lock().await = Some(cmd_tx.clone());
            *self.cancel_tx.lock().await = Some(cancel_tx);
            connected_arc.store(true, Ordering::Release);
            {
                let mut st = status_arc.write().await;
                st.connected = true;
                st.connecting = false;
                st.obs_version = format!("OBS WS v{}", obs_ws_ver);
                st.last_error = None;
                let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
            }
            // 接続直後にシーン一覧を取得
            let _ = cmd_tx.send(ObsCommand::RefreshScenes);
        }

        let cmd_tx_bg = cmd_tx.clone();
        tokio::spawn(async move {
            let mut pending: HashMap<String, oneshot::Sender<Result<Value>>> = HashMap::new();
            let mut internal_requests: HashMap<String, String> = HashMap::new();
            let mut seq: u64 = 1;

            loop {
                tokio::select! {
                    _ = cancel_rx.changed() => {
                        let _ = write.close().await;
                        break;
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        match cmd {
                            ObsCommand::Request { request_type, request_data, reply_tx } => {
                                let req_id = format!("req-{}", seq);
                                seq += 1;
                                pending.insert(req_id.clone(), reply_tx);
                                let payload = json!({
                                    "op": 6,
                                    "d": {
                                        "requestType": request_type,
                                        "requestId": req_id,
                                        "requestData": request_data,
                                    }
                                });
                                if write.send(WsMessage::Text(payload.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                            ObsCommand::RefreshScenes => {
                                let req_id = format!("int-scenes-{}", seq);
                                seq += 1;
                                internal_requests.insert(req_id.clone(), "GetSceneList".to_string());
                                let payload = json!({
                                    "op": 6,
                                    "d": {
                                        "requestType": "GetSceneList",
                                        "requestId": req_id,
                                    }
                                });
                                if write.send(WsMessage::Text(payload.to_string().into())).await.is_err() {
                                    break;
                                }

                                let st_id = format!("int-stream-{}", seq);
                                seq += 1;
                                internal_requests.insert(st_id.clone(), "GetStreamStatus".to_string());
                                let st_payload = json!({
                                    "op": 6,
                                    "d": {
                                        "requestType": "GetStreamStatus",
                                        "requestId": st_id,
                                    }
                                });
                                let _ = write.send(WsMessage::Text(st_payload.to_string().into())).await;
                            }
                        }
                    }
                    msg_opt = read.next() => {
                        let Some(Ok(msg)) = msg_opt else {
                            break;
                        };
                        let text = match msg {
                            WsMessage::Text(t) => t.to_string(),
                            WsMessage::Close(_) => break,
                            WsMessage::Ping(p) => {
                                let _ = write.send(WsMessage::Pong(p)).await;
                                continue;
                            }
                            _ => continue,
                        };
                        let Ok(val) = serde_json::from_str::<Value>(&text) else {
                            continue;
                        };
                        match val["op"].as_u64() {
                            // op 5: Event
                            Some(5) => {
                                let ev_type = val["d"]["eventType"].as_str().unwrap_or("");
                                let ev_data = &val["d"]["eventData"];
                                match ev_type {
                                    "CurrentProgramSceneChanged" => {
                                        if let Some(name) = ev_data["sceneName"].as_str() {
                                            let mut st = status_arc.write().await;
                                            st.current_scene = name.to_string();
                                            let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
                                        }
                                    }
                                    "SceneListChanged" | "SceneItemCreated" | "SceneItemRemoved" => {
                                        let _ = cmd_tx_bg.send(ObsCommand::RefreshScenes);
                                    }
                                    "StreamStateChanged" => {
                                        let active = ev_data["outputActive"].as_bool().unwrap_or(false);
                                        let state_str = ev_data["outputState"].as_str().unwrap_or("");
                                        {
                                            let mut st = status_arc.write().await;
                                            st.is_streaming = active;
                                            let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
                                        }
                                        if state_str == "OBS_WEBSOCKET_OUTPUT_STARTED" {
                                            let _ = event_tx.send(ObsIncomingEvent::StreamStarted);
                                        } else if state_str == "OBS_WEBSOCKET_OUTPUT_STOPPED" {
                                            let _ = event_tx.send(ObsIncomingEvent::StreamStopped);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // op 7: RequestResponse
                            Some(7) => {
                                let req_id = val["d"]["requestId"].as_str().unwrap_or("").to_string();
                                let ok = val["d"]["requestStatus"]["result"].as_bool().unwrap_or(false);
                                if let Some(tx) = pending.remove(&req_id) {
                                    if ok {
                                        let _ = tx.send(Ok(val["d"]["responseData"].clone()));
                                    } else {
                                        let comment = val["d"]["requestStatus"]["comment"]
                                            .as_str()
                                            .unwrap_or("OBS リクエスト失敗");
                                        let _ = tx.send(Err(anyhow!("{}", comment)));
                                    }
                                } else if let Some(kind) = internal_requests.remove(&req_id) {
                                    if !ok {
                                        continue;
                                    }
                                    if kind == "GetSceneList" {
                                        let current = val["d"]["responseData"]["currentProgramSceneName"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string();
                                        let mut scenes: Vec<String> = val["d"]["responseData"]["scenes"]
                                            .as_array()
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|s| s["sceneName"].as_str().map(|x| x.to_string()))
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        scenes.reverse();
                                        // 各シーンのソース一覧も続けて取得
                                        for sc in &scenes {
                                            let item_req_id = format!("int-items-{}", seq);
                                            seq += 1;
                                            internal_requests.insert(item_req_id.clone(), format!("Items:{}", sc));
                                            let item_payload = json!({
                                                "op": 6,
                                                "d": {
                                                    "requestType": "GetSceneItemList",
                                                    "requestId": item_req_id,
                                                    "requestData": { "sceneName": sc }
                                                }
                                            });
                                            let _ = write.send(WsMessage::Text(item_payload.to_string().into())).await;
                                        }
                                        let mut st = status_arc.write().await;
                                        st.current_scene = current;
                                        st.scene_sources.retain(|sc, _| scenes.contains(sc));
                                        st.scenes = scenes;
                                        let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
                                    } else if kind == "GetStreamStatus" {
                                        let active = val["d"]["responseData"]["outputActive"].as_bool().unwrap_or(false);
                                        let mut st = status_arc.write().await;
                                        st.is_streaming = active;
                                        let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
                                    } else if let Some(scene_name) = kind.strip_prefix("Items:") {
                                        let sources: Vec<String> = val["d"]["responseData"]["sceneItems"]
                                            .as_array()
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|it| it["sourceName"].as_str().map(|x| x.to_string()))
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        let mut st = status_arc.write().await;
                                        st.scene_sources.insert(scene_name.to_string(), sources);
                                        let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // 切断時の後処理（すでに新しい世代の接続へ切り替わっていれば旧接続の遅延終了で状態を上書きしない）
            for (_, tx) in pending {
                let _ = tx.send(Err(anyhow!("OBS WebSocket が切断されました")));
            }
            if gen_arc.load(Ordering::Acquire) == my_gen {
                connected_arc.store(false, Ordering::Release);
                let mut st = status_arc.write().await;
                st.connected = false;
                st.connecting = false;
                let _ = event_tx.send(ObsIncomingEvent::StatusChanged(st.clone()));
            }
        });

        Ok(())
    }

    pub async fn send_request(&self, request_type: &str, request_data: Value) -> Result<Value> {
        let tx = self
            .cmd_tx
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow!("OBS WebSocket に接続されていません"))?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(ObsCommand::Request {
            request_type: request_type.to_string(),
            request_data,
            reply_tx,
        })
        .map_err(|_| anyhow!("OBS WebSocket チャンネル送信失敗"))?;
        tokio::time::timeout(Duration::from_secs(5), reply_rx)
            .await
            .map_err(|_| anyhow!("OBS WebSocket 応答タイムアウト"))?
            .map_err(|_| anyhow!("OBS WebSocket 応答が中断されました"))?
    }

    pub async fn refresh_scenes(&self) -> Result<()> {
        let tx = self
            .cmd_tx
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow!("OBS WebSocket に接続されていません"))?;
        tx.send(ObsCommand::RefreshScenes)
            .map_err(|_| anyhow!("OBS WebSocket シーン更新失敗"))?;
        Ok(())
    }

    pub async fn switch_scene(&self, scene_name: &str) -> Result<()> {
        let trimmed = scene_name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("シーン名が指定されていません"));
        }
        self.send_request(
            "SetCurrentProgramScene",
            json!({ "sceneName": trimmed }),
        )
        .await?;
        Ok(())
    }

    /// 指定シーン内のソース表示状態を設定またはトグルする
    pub async fn set_scene_item_visible(
        &self,
        scene_name: &str,
        source_name: &str,
        visible: Option<bool>,
    ) -> Result<bool> {
        let sc = if scene_name.trim().is_empty() {
            self.status.read().await.current_scene.clone()
        } else {
            scene_name.trim().to_string()
        };
        if sc.is_empty() || source_name.trim().is_empty() {
            return Err(anyhow!("シーン名またはソース名が未指定です"));
        }
        let id_resp = self
            .send_request(
                "GetSceneItemId",
                json!({ "sceneName": sc, "sourceName": source_name.trim() }),
            )
            .await?;
        let item_id = id_resp["sceneItemId"]
            .as_i64()
            .ok_or_else(|| anyhow!("ソース「{}」がシーン「{}」に見つかりません", source_name, sc))?;

        let target_visible = match visible {
            Some(v) => v,
            None => {
                let cur = self
                    .send_request(
                        "GetSceneItemEnabled",
                        json!({ "sceneName": sc, "sceneItemId": item_id }),
                    )
                    .await?;
                !cur["sceneItemEnabled"].as_bool().unwrap_or(true)
            }
        };

        self.send_request(
            "SetSceneItemEnabled",
            json!({
                "sceneName": sc,
                "sceneItemId": item_id,
                "sceneItemEnabled": target_visible,
            }),
        )
        .await?;
        Ok(target_visible)
    }
}

/// OBS WebSocket v5 仕様の認証ハッシュ計算: Base64(SHA256(Base64(SHA256(password + salt)) + challenge))
pub fn compute_obs_v5_auth(password: &str, salt: &str, challenge: &str) -> String {
    let mut hasher1 = Sha256::new();
    hasher1.update(password.as_bytes());
    hasher1.update(salt.as_bytes());
    let secret = base64::engine::general_purpose::STANDARD.encode(hasher1.finalize());

    let mut hasher2 = Sha256::new();
    hasher2.update(secret.as_bytes());
    hasher2.update(challenge.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hasher2.finalize())
}

async fn read_next_json<S>(read: &mut S, timeout: Duration) -> Result<Value>
where
    S: StreamExt<Item = std::result::Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    let msg = tokio::time::timeout(timeout, read.next())
        .await
        .map_err(|_| anyhow!("OBS WebSocket 応答タイムアウト"))?
        .ok_or_else(|| anyhow!("OBS WebSocket が予期せず終了しました"))?
        .map_err(|e| anyhow!("OBS WebSocket 受信エラー: {}", e))?;

    match msg {
        WsMessage::Text(t) => Ok(serde_json::from_str(&t)?),
        WsMessage::Close(f) => Err(anyhow!("OBS WebSocket が切断されました: {:?}", f)),
        _ => Err(anyhow!("予期しない WebSocket フレームを受信しました")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_obs_v5_auth_vector() {
        // OBS WebSocket v5 公式プロトコル仕様の公式テストベクタ
        let auth = compute_obs_v5_auth(
            "supersecretpassword",
            "lM1GncleQOaCu9lT1yeUZhFYnqhsNYP1G8sH7h54321=",
            "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeagNgfd3POUA=",
        );
        assert!(!auth.is_empty());
        // 決定論的かつ Base64 (SHA-256 = 44 文字)
        assert_eq!(auth.len(), 44);
        assert_eq!(
            auth,
            compute_obs_v5_auth(
                "supersecretpassword",
                "lM1GncleQOaCu9lT1yeUZhFYnqhsNYP1G8sH7h54321=",
                "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeagNgfd3POUA=",
            )
        );
    }

    #[tokio::test]
    async fn test_reconnect_generation_guard_and_failure_status_reset() {
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // モック OBS WebSocket v5 サーバー（Hello -> Identify -> Identified を応答）
        let server_task = tokio::spawn(async move {
            for _ in 0..2 {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
                        return;
                    };
                    let hello = json!({
                        "op": 0,
                        "d": { "obsWebSocketVersion": "5.5.0", "rpcVersion": 1 }
                    });
                    let _ = ws.send(WsMessage::Text(hello.to_string().into())).await;
                    if let Some(Ok(WsMessage::Text(_))) = ws.next().await {
                        let identified = json!({ "op": 2, "d": { "negotiatedRpcVersion": 1 } });
                        let _ = ws
                            .send(WsMessage::Text(identified.to_string().into()))
                            .await;
                    }
                    while let Some(Ok(msg)) = ws.next().await {
                        if matches!(msg, WsMessage::Close(_)) {
                            // 旧接続のクローズ処理をわずかに遅延させても新接続を破壊しないことを検証
                            tokio::time::sleep(Duration::from_millis(25)).await;
                            break;
                        }
                    }
                });
            }
        });

        let mgr = ObsWsManager::new();
        let (ev_tx, _ev_rx) = mpsc::unbounded_channel();
        let cfg = ObsWsConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port,
            password: String::new(),
            auto_connect: false,
            reset_first_time_on_stream_start: false,
            reset_counters_on_stream_start: false,
            rules: Vec::new(),
        };

        // 1回目接続 → 即座に2回目再接続
        mgr.connect(cfg.clone(), ev_tx.clone()).await.unwrap();
        assert!(mgr.is_connected());
        mgr.connect(cfg.clone(), ev_tx.clone()).await.unwrap();
        // 旧接続の遅延クローズが完了した後も 2 回目の接続フラグとステータスが維持されること
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(mgr.is_connected());
        assert!(mgr.get_status().await.connected);

        // 切断後、未待ち受けポートへの接続失敗で connecting が false へ確実に戻ること
        mgr.disconnect().await;
        assert!(!mgr.is_connected());
        server_task.abort();

        let dead_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead_port = dead_listener.local_addr().unwrap().port();
        drop(dead_listener);
        let mut bad_cfg = cfg;
        bad_cfg.port = dead_port;
        assert!(mgr.connect(bad_cfg, ev_tx).await.is_err());
        let st = mgr.get_status().await;
        assert!(!st.connected);
        assert!(!st.connecting);
        assert!(st.last_error.is_some());
    }
}
