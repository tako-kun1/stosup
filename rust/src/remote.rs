use std::collections::{BTreeMap, HashSet};
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

#[derive(Debug, Clone)]
struct AdapterIpCandidate {
    ip: Ipv4Addr,
    has_gateway: bool,
    is_virtual: bool,
}

fn is_usable_lan_v4(v4: Ipv4Addr) -> bool {
    v4.is_private() && !v4.is_loopback() && !v4.is_unspecified() && !v4.is_link_local()
}

/// サブネットマスク（上位ビットが連続して 1 の 255.x.x.x 等）かどうかを判定する
fn is_subnet_mask_v4(v4: Ipv4Addr) -> bool {
    let bits = u32::from(v4);
    if bits == 0 || bits == u32::MAX {
        return true;
    }
    // 255.0.0.0 以上で上位ビットが連続して 1 ならサブネットマスク
    v4.octets()[0] == 255 && (!bits).wrapping_add(1).is_power_of_two()
}

/// 英語 "(Preferred)" や日本語 "(優先)" など括弧サフィックスを除去して IPv4 を取り出す
fn extract_ipv4_from_value(raw: &str) -> Option<Ipv4Addr> {
    let before_paren = raw.split('(').next().unwrap_or(raw).trim();
    before_paren.parse::<Ipv4Addr>().ok()
}

/// Hyper-V / WSL / Docker / VPN 等の仮想アダプタ名かどうかを判定する
fn is_virtual_adapter_name(header: &str) -> bool {
    let lower = header.to_ascii_lowercase();
    const VIRTUAL_KEYWORDS: &[&str] = &[
        "vethernet",
        "wsl",
        "hyper-v",
        "docker",
        "vmware",
        "virtualbox",
        "vbox",
        "tailscale",
        "zerotier",
        "loopback",
        "bluetooth",
        "wireguard",
        "nordlynx",
        "hamachi",
        "radmin",
        "tap-windows",
        "virtual",
        "vpn",
    ];
    VIRTUAL_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Windows の `ipconfig` 出力をロケール非依存（日本語/英語両対応）でアダプタ単位に解析する
fn parse_ipconfig_adapters(text: &str) -> Vec<AdapterIpCandidate> {
    let mut results = Vec::new();
    let mut current_virtual = false;
    let mut current_ips: Vec<Ipv4Addr> = Vec::new();
    let mut current_has_gateway = false;

    let flush = |out: &mut Vec<AdapterIpCandidate>,
                 ips: &mut Vec<Ipv4Addr>,
                 has_gw: &mut bool,
                 is_virt: bool| {
        for ip in ips.drain(..) {
            out.push(AdapterIpCandidate {
                ip,
                has_gateway: *has_gw,
                is_virtual: is_virt,
            });
        }
        *has_gw = false;
    };

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if line.is_empty() {
            continue;
        }

        // 先頭にインデントがなく ':' で終わる行はアダプタセクションヘッダ
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        if !is_indented && line.ends_with(':') {
            flush(
                &mut results,
                &mut current_ips,
                &mut current_has_gateway,
                current_virtual,
            );
            current_virtual = is_virtual_adapter_name(line);
            continue;
        }

        if let Some(pos) = line.rfind(':') {
            let value = line[pos + 1..].trim();
            if line.contains("IPv4") {
                if let Some(v4) = extract_ipv4_from_value(value) {
                    if is_usable_lan_v4(v4) {
                        current_ips.push(v4);
                    }
                }
            } else if let Some(v4) = extract_ipv4_from_value(value) {
                // 日本語 Windows の Shift-JIS 文字化けに依存せず、サブネットマスク以外の有効 IPv4 をゲートウェイとして検出
                if !v4.is_unspecified() && !v4.is_loopback() && !is_subnet_mask_v4(v4) {
                    current_has_gateway = true;
                }
            }
        } else if let Some(v4) = extract_ipv4_from_value(line.trim()) {
            // デフォルトゲートウェイが IPv6 の次行にインデント単独行で出力されるケースに対応
            if !v4.is_unspecified() && !v4.is_loopback() && !is_subnet_mask_v4(v4) {
                current_has_gateway = true;
            }
        }
    }

    flush(
        &mut results,
        &mut current_ips,
        &mut current_has_gateway,
        current_virtual,
    );
    results
}

fn ip_range_priority(v4: Ipv4Addr) -> u8 {
    let o = v4.octets();
    if o[0] == 192 && o[1] == 168 {
        0
    } else if o[0] == 10 {
        1
    } else {
        2
    }
}

/// 収集済み候補を「実 LAN のデフォルトゲートウェイ付き 192.168.x.x」最優先で並べ替える
fn rank_local_lan_ips(
    adapter_candidates: &[AdapterIpCandidate],
    udp_default_ips: &HashSet<Ipv4Addr>,
    udp_lan_ips: &HashSet<Ipv4Addr>,
) -> Vec<String> {
    // 各 IP の最良メタデータ（has_gateway, is_virtual, is_default_route）を集約
    let mut by_ip: BTreeMap<Ipv4Addr, (bool, bool, bool)> = BTreeMap::new();

    for c in adapter_candidates {
        if !is_usable_lan_v4(c.ip) {
            continue;
        }
        let entry = by_ip.entry(c.ip).or_insert((false, c.is_virtual, false));
        entry.0 |= c.has_gateway;
        entry.1 &= c.is_virtual;
    }

    for &ip in udp_default_ips {
        if !is_usable_lan_v4(ip) {
            continue;
        }
        let entry = by_ip.entry(ip).or_insert((false, false, false));
        entry.2 = true;
    }

    for &ip in udp_lan_ips {
        if !is_usable_lan_v4(ip) {
            continue;
        }
        by_ip.entry(ip).or_insert((false, false, false));
    }

    let has_physical = by_ip
        .values()
        .any(|(has_gw, is_virt, is_def)| !*is_virt || *has_gw || *is_def);

    let mut ranked: Vec<(u8, u8, Ipv4Addr)> = by_ip
        .into_iter()
        .filter(|(_, (has_gw, is_virt, is_def))| {
            // 実 LAN IP が見つかっている場合は WSL / Hyper-V 等の仮想 NIC のみの IP を除外
            !has_physical || !*is_virt || *has_gw || *is_def
        })
        .map(|(ip, (has_gw, is_virt, is_def))| {
            let tier = if !is_virt && has_gw && is_def {
                0
            } else if !is_virt && has_gw {
                1
            } else if !is_virt && is_def {
                2
            } else if !is_virt {
                3
            } else if has_gw || is_def {
                4
            } else {
                5
            };
            (tier, ip_range_priority(ip), ip)
        })
        .collect();

    ranked.sort_by_key(|&(tier, band, ip)| (tier, band, u32::from(ip)));
    ranked.into_iter().map(|(_, _, ip)| ip.to_string()).collect()
}

/// この PC のローカルネットワーク IPv4 アドレス候補を取得する（パケット送出なし）
pub fn detect_local_lan_ips() -> Vec<String> {
    let mut udp_default_ips = HashSet::new();
    let mut udp_lan_ips = HashSet::new();

    // デフォルトルート向けの主要インターフェース IP を UDP connect で安全に取得
    for probe in ["8.8.8.8:80", "1.1.1.1:80"] {
        if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
            if sock.connect(probe).is_ok() {
                if let Ok(local) = sock.local_addr() {
                    if let IpAddr::V4(v4) = local.ip() {
                        if is_usable_lan_v4(v4) {
                            udp_default_ips.insert(v4);
                        }
                    }
                }
            }
        }
    }

    for probe in [
        "192.168.0.1:80",
        "192.168.1.1:80",
        "10.0.0.1:80",
        "172.16.0.1:80",
    ] {
        if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
            if sock.connect(probe).is_ok() {
                if let Ok(local) = sock.local_addr() {
                    if let IpAddr::V4(v4) = local.ip() {
                        if is_usable_lan_v4(v4) {
                            udp_lan_ips.insert(v4);
                        }
                    }
                }
            }
        }
    }

    // Windows では ipconfig 出力からも実 LAN アダプタとデフォルトゲートウェイ有無を収集
    #[cfg(target_os = "windows")]
    let adapter_candidates = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("ipconfig")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|output| {
                let text = String::from_utf8_lossy(&output.stdout);
                parse_ipconfig_adapters(&text)
            })
            .unwrap_or_default()
    };

    #[cfg(not(target_os = "windows"))]
    let adapter_candidates: Vec<AdapterIpCandidate> = Vec::new();

    rank_local_lan_ips(&adapter_candidates, &udp_default_ips, &udp_lan_ips)
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
    build_remote_access_status_with_ips(config, server_running, port, detect_local_lan_ips())
}

fn build_remote_access_status_with_ips(
    config: RemoteAccessConfig,
    server_running: bool,
    port: u16,
    local_ips: Vec<String>,
) -> RemoteAccessStatus {
    let local_url = format!("http://127.0.0.1:{}/remote", port);
    let lan_urls: Vec<String> = local_ips
        .iter()
        .map(|ip| format!("http://{}:{}/remote", ip, port))
        .collect();

    // LAN IP（192.168.x.x 等）が検出できた場合は常に LAN IP をリモート操作の代表 URL・QR とする
    let primary_remote_url = lan_urls
        .first()
        .cloned()
        .unwrap_or_else(|| local_url.clone());

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

    #[test]
    fn test_ipconfig_parsing_prefers_real_lan_and_filters_virtual_adapters() {
        // 日本語 Windows の文字化け + (優先) + Tailscale + WSL/Hyper-V 仮想アダプタ混在を模擬
        let sample = "\
Unknown adapter Tailscale:
   IPv4 Address. . . . . . . . . . . : 100.90.90.1
   Default Gateway . . . . . . . . . :

Ethernet adapter \u{FFFD}\u{FFFD}\u{FFFD} 5:
   IPv4 \u{FFFD}A\u{FFFD}h\u{FFFD}\u{FFFD}\u{FFFD}X . . . . . . . . : 192.168.0.7(\u{FFFD}D\u{FFFD}\u{FFFD})
   \u{FFFD}T\u{FFFD}u\u{FFFD}l\u{FFFD}b\u{FFFD}g \u{FFFD}}\u{FFFD}X\u{FFFD}N . . . . : 255.255.255.0
   \u{FFFD}f\u{FFFD}t\u{FFFD}H\u{FFFD}\u{FFFD}\u{FFFD}g \u{FFFD}Q\u{FFFD}[\u{FFFD}g\u{FFFD}E\u{FFFD}F\u{FFFD}C . : 192.168.0.1

Ethernet adapter vEthernet (Default Switch) 2:
   IPv4 Address. . . . . . . . . . . : 172.26.112.1(Preferred)
   Subnet Mask . . . . . . . . . . . : 255.255.240.0
   Default Gateway . . . . . . . . . :

Ethernet adapter vEthernet (WSL (Hyper-V firewall)):
   IPv4 Address. . . . . . . . . . . : 172.30.128.1
   Subnet Mask . . . . . . . . . . . : 255.255.240.0
   Default Gateway . . . . . . . . . :
";
        let candidates = parse_ipconfig_adapters(sample);
        let ranked = rank_local_lan_ips(&candidates, &HashSet::new(), &HashSet::new());
        assert_eq!(ranked, vec!["192.168.0.7".to_string()]);
    }

    #[test]
    fn test_remote_status_uses_lan_ip_instead_of_localhost() {
        let cfg = RemoteAccessConfig {
            allow_lan_access: true,
            remote_control_enabled: true,
            require_pin: true,
            pin_code: "654321".to_string(),
            embed_pin_in_qr: true,
        };
        let status = build_remote_access_status_with_ips(
            cfg,
            true,
            18765,
            vec!["192.168.0.7".to_string()],
        );
        assert_eq!(status.primary_remote_url, "http://192.168.0.7:18765/remote");
        assert_eq!(
            status.qr_target_url,
            "http://192.168.0.7:18765/remote?pin=654321"
        );
        assert!(!status.primary_remote_url.contains("127.0.0.1"));

        // 実機検出結果にループバックやリンクローカル (169.254.x.x) が含まれないことを確認
        let detected = detect_local_lan_ips();
        for ip in &detected {
            assert!(!ip.starts_with("127."));
            assert!(!ip.starts_with("169.254."));
        }
    }
}
