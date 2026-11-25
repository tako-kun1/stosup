use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};

use qrcode::{Color, QrCode};
use rand::Rng;

use crate::models::{QrCodeMatrix, RemoteAccessConfig, RemoteAccessStatus};

/// 6 桁のランダム数字 PIN コードを生成する
pub fn generate_pin_code() -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100_000..1_000_000);
    num.to_string()
}

/// 接続元 IP がループバック（同一 PC）か判定する
pub fn is_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.to_ipv4_mapped().map(|v4| v4.is_loopback()).unwrap_or(false)
        }
    }
}

/// 接続元 IP がローカルネットワーク（ループバックまたはプライベート / リンクローカル）か判定する
pub fn is_private_lan_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_lan_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_private_lan_v4(mapped);
            }
            is_private_lan_v6(v6)
        }
    }
}

fn is_private_lan_v4(v4: Ipv4Addr) -> bool {
    v4.is_loopback() || v4.is_private() || v4.is_link_local()
}

fn is_private_lan_v6(v6: Ipv6Addr) -> bool {
    if v6.is_loopback() {
        return true;
    }
    let seg0 = v6.segments()[0];
    // Unique Local Address (fc00::/7) または Link-Local (fe80::/10)
    (seg0 & 0xfe00) == 0xfc00 || (seg0 & 0xffc0) == 0xfe80
}

/// アクセス制御判定：設定と接続元 IP に基づいてリクエストを許可するか判定する
pub fn is_ip_allowed(ip: IpAddr, allow_lan: bool) -> bool {
    if is_loopback_ip(ip) {
        return true;
    }
    if allow_lan {
        return is_private_lan_ip(ip);
    }
    false
}

/// この PC のローカルネットワーク IPv4 アドレス候補を取得する（パケット送出なし）
pub fn detect_local_lan_ips() -> Vec<String> {
    let mut found = BTreeSet::new();

    // デフォルトゲートウェイ向けの主要インターフェース IP を UDP connect で安全に取得
    for probe in ["192.168.1.1:80", "10.0.0.1:80", "172.16.0.1:80", "8.8.8.8:80"] {
        if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
            if sock.connect(probe).is_ok() {
                if let Ok(local) = sock.local_addr() {
                    if let IpAddr::V4(v4) = local.ip() {
                        if !v4.is_loopback() && !v4.is_unspecified() && is_private_lan_v4(v4) {
                            found.insert(v4.to_string());
                        }
                    }
                }
            }
        }
    }

    // Windows では ipconfig 出力からもプライベート IPv4 を補助収集（複数 NIC 対応）
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if let Ok(output) = std::process::Command::new("ipconfig")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if !line.contains("IPv4") {
                    continue;
                }
                if let Some(pos) = line.rfind(':') {
                    let candidate = line[pos + 1..].trim().trim_end_matches("(Preferred)").trim();
                    if let Ok(v4) = candidate.parse::<Ipv4Addr>() {
                        if !v4.is_loopback() && !v4.is_unspecified() && v4.is_private() {
                            found.insert(v4.to_string());
                        }
                    }
                }
            }
        }
    }

    // 192.168.x.x → 10.x.x.x → 172.x.x.x の順に使いやすい優先度で並べる
    let mut list: Vec<String> = found.into_iter().collect();
    list.sort_by_key(|ip| {
        if ip.starts_with("192.168.") {
            (0, ip.clone())
        } else if ip.starts_with("10.") {
            (1, ip.clone())
        } else {
            (2, ip.clone())
        }
    });
    list
}

/// URL 文字列から QR コードの 2D モジュール行列を生成する
pub fn generate_qr_matrix(text: &str) -> Option<QrCodeMatrix> {
    let code = QrCode::new(text.as_bytes()).ok()?;
    let width = code.width();
    let mut modules = Vec::with_capacity(width * width);
    for y in 0..width {
        for x in 0..width {
            modules.push(code[(x, y)] == Color::Dark);
        }
    }
    Some(QrCodeMatrix {
        size: width as u32,
        modules,
    })
}

/// 現在の LAN リモートアクセス状態 DTO を構築する
pub fn build_remote_access_status(
    config: RemoteAccessConfig,
    server_running: bool,
    port: u16,
) -> RemoteAccessStatus {
    let local_ips = detect_local_lan_ips();
    let local_url = format!("http://127.0.0.1:{}/remote", port);
    let lan_urls: Vec<String> = local_ips
        .iter()
        .map(|ip| format!("http://{}:{}/remote", ip, port))
        .collect();

    let primary_remote_url = if config.allow_lan_access {
        lan_urls
            .first()
            .cloned()
            .unwrap_or_else(|| local_url.clone())
    } else {
        local_url.clone()
    };

    let qr_target_url = if config.require_pin
        && config.embed_pin_in_qr
        && !config.pin_code.trim().is_empty()
    {
        format!("{}?pin={}", primary_remote_url, config.pin_code.trim())
    } else {
        primary_remote_url.clone()
    };

    let qr_code = generate_qr_matrix(&qr_target_url);
    let bound_address = if !server_running {
        "停止中".to_string()
    } else if config.allow_lan_access {
        format!("0.0.0.0:{}", port)
    } else {
        format!("127.0.0.1:{}", port)
    };

    RemoteAccessStatus {
        config,
        server_running,
        port,
        bound_address,
        local_ips,
        local_url,
        lan_urls,
        primary_remote_url,
        qr_target_url,
        qr_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_access_control_and_qr_generation() {
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let lan_192: IpAddr = "192.168.1.42".parse().unwrap();
        let lan_10: IpAddr = "10.0.0.15".parse().unwrap();
        let lan_172: IpAddr = "172.16.5.2".parse().unwrap();
        let public_ip: IpAddr = "8.8.8.8".parse().unwrap();

        // LAN 許可 OFF: ループバックのみ許可
        assert!(is_ip_allowed(loopback, false));
        assert!(!is_ip_allowed(lan_192, false));
        assert!(!is_ip_allowed(public_ip, false));

        // LAN 許可 ON: プライベート LAN のみ許可、グローバル IP は拒否
        assert!(is_ip_allowed(loopback, true));
        assert!(is_ip_allowed(lan_192, true));
        assert!(is_ip_allowed(lan_10, true));
        assert!(is_ip_allowed(lan_172, true));
        assert!(!is_ip_allowed(public_ip, true));

        // PIN 生成（6桁数字）
        let pin = generate_pin_code();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));

        // QR コード行列生成
        let qr = generate_qr_matrix("http://192.168.1.10:18765/remote?pin=123456").unwrap();
        assert!(qr.size >= 21);
        assert_eq!(qr.modules.len(), (qr.size * qr.size) as usize);
    }
}
