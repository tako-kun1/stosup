use std::collections::HashMap;

use chrono::Utc;

use crate::models::{FeatureFlags, OverlayConfig, StreamProfile};

/// よく使われる配信スタイル別のプロファイルテンプレートを生成する
pub fn build_preset_profile(template_id: &str, name: &str) -> StreamProfile {
    let mut flags = FeatureFlags::default();
    let mut overlay = OverlayConfig::default();
    let description: String;

    match template_id {
        "talk" => {
            description = "雑談・ラジオ配信向け。トークモニター・スポットテロップ・エールボックス・ウェルカムカウンターを中心に構成。".to_string();
            flags.queue_enabled = false;
            flags.user_lottery_enabled = false;
            flags.comment_lottery_enabled = false;
            flags.word_counter_enabled = false;
            overlay.comments_theme = "editorial".to_string();
            overlay.pickup_theme = "editorial".to_string();
            overlay.superchat_theme = "editorial".to_string();
            overlay.first_time_theme = "editorial".to_string();
        }
        "game_queue" => {
            description = "参加型ゲーム配信向け。エントリーレーン・ライブアンケート・タクティカルHUDオーバーレイを中心に構成。".to_string();
            flags.queue_enabled = true;
            flags.poll_enabled = true;
            flags.comment_lottery_enabled = false;
            overlay.comments_theme = "cinema".to_string();
            overlay.queue_theme = "tactical".to_string();
            overlay.poll_theme = "tactical".to_string();
            overlay.pickup_theme = "tactical".to_string();
            overlay.counter_theme = "tactical".to_string();
            overlay.first_time_theme = "tactical".to_string();
        }
        "singing" => {
            description = "歌枠・バラエティ企画向け。リアクションステージ・コールカウンター・賑やかなバラエティテロップを中心に構成。".to_string();
            flags.queue_enabled = false;
            flags.media_reaction_enabled = true;
            flags.word_counter_enabled = true;
            overlay.comments_theme = "variety".to_string();
            overlay.pickup_theme = "variety".to_string();
            overlay.superchat_theme = "variety".to_string();
            overlay.counter_theme = "variety".to_string();
            overlay.lottery_theme = "variety".to_string();
        }
        _ => {
            description = "すべての機能を標準設定（オンエア・テロップ）で有効にした標準プロファイル。".to_string();
            overlay.comments_theme = "broadcast".to_string();
            overlay.pickup_theme = "broadcast".to_string();
            overlay.superchat_theme = "broadcast".to_string();
            overlay.poll_theme = "broadcast".to_string();
            overlay.queue_theme = "broadcast".to_string();
            overlay.lottery_theme = "broadcast".to_string();
            overlay.counter_theme = "broadcast".to_string();
            overlay.first_time_theme = "broadcast".to_string();
            overlay.media_theme = "broadcast".to_string();
        }
    }

    StreamProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: if name.trim().is_empty() {
            "新規プロファイル".to_string()
        } else {
            name.trim().to_string()
        },
        description,
        feature_flags: flags,
        overlay_config: overlay,
        highlight_enabled_map: HashMap::new(),
        counter_enabled_map: HashMap::new(),
        media_enabled_map: HashMap::new(),
        ng_word_enabled_map: HashMap::new(),
        updated_at_ms: Utc::now().timestamp_millis(),
    }
}
