pub mod demo;
pub mod youtube;
pub mod youtube_oauth;

use anyhow::{anyhow, Result};
use regex::Regex;
use tokio::sync::mpsc;

use crate::models::{PlatformKind, RawIncomingComment};

/// 将来の他プラットフォーム拡張（Twitch / ニコニコ生放送 / ツイキャス等）を見据えた共通インターフェース
pub struct ProviderConnection {
    pub platform: PlatformKind,
    pub stream_id: String,
    pub stream_title: String,
    pub live_chat_id: Option<String>,
    pub can_post_chat: bool,
    pub cancel_tx: tokio::sync::watch::Sender<bool>,
}

impl ProviderConnection {
    pub fn stop(&self) {
        let _ = self.cancel_tx.send(true);
    }
}

/// コメント受信チャネル送信側エイリアス
pub type IncomingCommentTx = mpsc::UnboundedSender<RawIncomingComment>;

/// YouTube の配信URLまたは動画ID文字列から11文字の video_id を抽出する
pub fn extract_youtube_video_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("YouTube 配信URL または 動画ID を入力してください"));
    }

    // すでに11文字の動画IDそのものの場合
    let id_only = Regex::new(r"^[A-Za-z0-9_-]{11}$").unwrap();
    if id_only.is_match(trimmed) {
        return Ok(trimmed.to_string());
    }

    // URLとしてパース
    if let Ok(parsed) = url::Url::parse(trimmed) {
        // ?v=XXXXXXXXXXX
        for (k, v) in parsed.query_pairs() {
            if k == "v" && id_only.is_match(&v) {
                return Ok(v.into_owned());
            }
        }
        // パスセグメントから /live/<id>, /embed/<id>, /video/<id>, /shorts/<id>, youtu.be/<id>
        if let Some(segments) = parsed.path_segments() {
            let segs: Vec<&str> = segments.collect();
            if parsed.host_str().unwrap_or("").contains("youtu.be") {
                if let Some(first) = segs.first() {
                    if id_only.is_match(first) {
                        return Ok((*first).to_string());
                    }
                }
            }
            for window in segs.windows(2) {
                if matches!(window[0], "live" | "embed" | "video" | "shorts")
                    && id_only.is_match(window[1])
                {
                    return Ok(window[1].to_string());
                }
            }
        }
    }

    Err(anyhow!(
        "有効な YouTube 動画ID または配信URLを認識できませんでした: {}",
        trimmed
    ))
}

/// YouTube 入力の解析種別（動画ID直接指定 または チャンネルURL/ハンドルからの自動検出）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YouTubeInputTarget {
    VideoId(String),
    ChannelLiveUrl(String),
}

/// 入力文字列が 動画ID/配信URL か、チャンネルURL/@ハンドル かを判定する
pub fn parse_youtube_input_target(input: &str) -> Result<YouTubeInputTarget> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "YouTube 配信URL・動画ID・またはチャンネルURL（@ハンドル）を入力してください"
        ));
    }

    if let Ok(vid) = extract_youtube_video_id(trimmed) {
        return Ok(YouTubeInputTarget::VideoId(vid));
    }

    // @handle 直接入力
    if trimmed.starts_with('@') && trimmed.len() >= 2 && !trimmed.contains(char::is_whitespace) {
        let handle = trimmed.trim_end_matches('/');
        return Ok(YouTubeInputTarget::ChannelLiveUrl(format!(
            "https://www.youtube.com/{}/live",
            handle
        )));
    }

    // UC... チャンネルID直接入力 (24文字)
    let ch_id_re = Regex::new(r"^UC[A-Za-z0-9_-]{22}$").unwrap();
    if ch_id_re.is_match(trimmed) {
        return Ok(YouTubeInputTarget::ChannelLiveUrl(format!(
            "https://www.youtube.com/channel/{}/live",
            trimmed
        )));
    }

    // チャンネルURL (/channel/UC..., /@handle, /c/..., /user/...)
    if let Ok(parsed) = url::Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
        if host.contains("youtube.com") {
            let segs: Vec<&str> = parsed
                .path_segments()
                .map(|s| s.filter(|p| !p.is_empty()).collect())
                .unwrap_or_default();
            if let Some(first) = segs.first() {
                if first.starts_with('@') {
                    return Ok(YouTubeInputTarget::ChannelLiveUrl(format!(
                        "https://www.youtube.com/{}/live",
                        first
                    )));
                }
                if matches!(*first, "channel" | "c" | "user") && segs.len() >= 2 {
                    return Ok(YouTubeInputTarget::ChannelLiveUrl(format!(
                        "https://www.youtube.com/{}/{}/live",
                        first, segs[1]
                    )));
                }
            }
        }
    }

    Err(anyhow!(
        "有効な YouTube 配信URL・動画ID・またはチャンネルURL（@ハンドル）を認識できませんでした: {}",
        trimmed
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_youtube_video_id() {
        assert_eq!(
            extract_youtube_video_id("dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&feature=share")
                .unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://www.youtube.com/live/dQw4w9WgXcQ?si=abc").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_youtube_video_id("https://studio.youtube.com/video/dQw4w9WgXcQ/livestreaming")
                .unwrap(),
            "dQw4w9WgXcQ"
        );
        assert!(extract_youtube_video_id("https://example.com/not-youtube").is_err());
    }

    #[test]
    fn test_parse_youtube_input_target_channel() {
        assert_eq!(
            parse_youtube_input_target("@takokun_ch").unwrap(),
            YouTubeInputTarget::ChannelLiveUrl("https://www.youtube.com/@takokun_ch/live".to_string())
        );
        assert_eq!(
            parse_youtube_input_target("https://www.youtube.com/@takokun_ch/streams").unwrap(),
            YouTubeInputTarget::ChannelLiveUrl("https://www.youtube.com/@takokun_ch/live".to_string())
        );
        assert_eq!(
            parse_youtube_input_target("UC1234567890123456789012").unwrap(),
            YouTubeInputTarget::ChannelLiveUrl(
                "https://www.youtube.com/channel/UC1234567890123456789012/live".to_string()
            )
        );
        assert_eq!(
            parse_youtube_input_target("dQw4w9WgXcQ").unwrap(),
            YouTubeInputTarget::VideoId("dQw4w9WgXcQ".to_string())
        );
    }
}
