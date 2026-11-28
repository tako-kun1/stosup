use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Deserialize;
use zip::ZipArchive;

use crate::models::{
    InstalledMod, ModCustomOverlayDef, ModLogicScriptDef, ModOverlayThemeDef, ModParamKind,
    ModParamSchema, ModReactionEffectDef, ModSavedState,
};

/// 展開可能なアーカイブの上限（ZIP ボム対策）
const MAX_ARCHIVE_FILES: usize = 1000;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 128 * 1024 * 1024; // 128 MB

/// 許可されているオーバーレイ対象キー一覧
pub const VALID_OVERLAY_TARGETS: &[&str] = &[
    "comments",
    "superchat",
    "pickup",
    "poll",
    "queue",
    "media",
    "lottery",
    "counter",
    "first_time",
];

/// 許可されている WebSocket チャンネル名一覧
pub const VALID_WS_CHANNELS: &[&str] = &[
    "all",
    "comments",
    "superchat",
    "pickup",
    "poll",
    "queue",
    "media",
    "lottery",
    "counter",
    "first_time",
];

/// ロジックスクリプト（.rhai）が購読できるイベント種別一覧
pub const VALID_SCRIPT_EVENTS: &[&str] = &["comment", "superchat", "first_time", "lottery"];

/// ZIP 導入時に安全のため拒否する実行可能拡張子
const FORBIDDEN_EXECUTABLE_EXTS: &[&str] = &[
    "exe", "dll", "bat", "cmd", "ps1", "psm1", "psd1", "vbs", "vbe", "jse", "wsf", "wsh", "msi",
    "msp", "msix", "appx", "com", "scr", "pif", "cpl", "hta", "jar", "sh", "bash", "zsh", "reg",
    "inf", "lnk", "sys", "drv", "msc", "chm", "url", "scf", "iso", "vhd", "vhdx", "py", "pyw",
    "rb", "pl", "xll",
];

/// MOD アイコンとして許可する安全なラスター画像拡張子（無効化 MOD からの HTML/SVG 配信を防止）
const ALLOWED_ICON_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "ico", "avif", "bmp"];

/// Windows 予約デバイス名（ファイル名として作成されると異常挙動や検証回避の原因になる）
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Deserialize)]
struct RawModManifest {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    id: String,
    name: String,
    #[serde(default = "default_mod_version")]
    version: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    min_stosup_version: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    license: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    reaction_effects: Vec<RawReactionEffect>,
    #[serde(default)]
    overlay_themes: Vec<RawOverlayTheme>,
    #[serde(default)]
    custom_overlays: Vec<RawCustomOverlay>,
    #[serde(default)]
    logic_scripts: Vec<RawLogicScript>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_mod_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Deserialize)]
struct RawModParam {
    key: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    default_value: serde_json::Value,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    step: Option<f64>,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct RawReactionEffect {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    entry: String,
    #[serde(default)]
    css: String,
    #[serde(default = "default_effect_duration")]
    default_duration_ms: u32,
    #[serde(default)]
    params: Vec<RawModParam>,
}

fn default_effect_duration() -> u32 {
    4000
}

#[derive(Debug, Deserialize)]
struct RawOverlayTheme {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    targets: Vec<String>,
    css: String,
    #[serde(default)]
    script: String,
    #[serde(default)]
    preview_bg: String,
    #[serde(default)]
    preview_fg: String,
    #[serde(default)]
    preview_border: String,
}

#[derive(Debug, Deserialize)]
struct RawCustomOverlay {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    entry: String,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default = "default_overlay_width")]
    recommended_width: u32,
    #[serde(default = "default_overlay_height")]
    recommended_height: u32,
}

#[derive(Debug, Deserialize)]
struct RawLogicScript {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    entry: String,
    #[serde(default)]
    events: Vec<String>,
}

fn default_overlay_width() -> u32 {
    800
}

fn default_overlay_height() -> u32 {
    600
}

/// MOD ID・コンポーネント ID 共通の安全な識別子検証（英小文字・数字・ハイフン・アンダースコア、2〜64文字）
pub fn is_valid_mod_id(id: &str) -> bool {
    let len = id.len();
    if !(2..=64).contains(&len) {
        return false;
    }
    let bytes = id.as_bytes();
    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-' || *b == b'_')
}

/// MOD 内相対パスの安全性検証（パストラバーサル・絶対パス・バックスラッシュ・隠しパス・Windows 末尾ドット/空白バイパスを遮断）
pub fn is_safe_relative_asset_path(rel: &str) -> bool {
    if rel.is_empty() || rel.len() > 260 {
        return false;
    }
    // 未トリム文字列に前後空白や制御文字があれば即拒否（Windows API による末尾空白除去での拡張子偽装を防止）
    if rel.trim() != rel || rel.chars().any(|c| c.is_control()) {
        return false;
    }
    if rel.starts_with('/')
        || rel.starts_with('\\')
        || rel.contains('\\')
        || rel.contains('\0')
        || rel.contains(':')
    {
        return false;
    }
    let mut has_normal = false;
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.is_empty()
                    || s.starts_with('.')
                    || s == ".."
                    || s.ends_with('.')
                    || s.ends_with(' ')
                    || s.starts_with(' ')
                {
                    return false;
                }
                // Windows 予約デバイス名チェック（拡張子を除いたステム部）
                let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
                if WINDOWS_RESERVED_NAMES.contains(&stem.as_str()) {
                    return false;
                }
                has_normal = true;
            }
            _ => return false,
        }
    }
    has_normal
}

pub fn is_forbidden_executable_path(rel_path: &str) -> bool {
    let cleaned = rel_path.trim().trim_end_matches(['.', ' ']);
    let ext = Path::new(cleaned)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    FORBIDDEN_EXECUTABLE_EXTS.contains(&ext.as_str())
}

fn is_allowed_icon_path(rel_path: &str) -> bool {
    let ext = Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    ALLOWED_ICON_EXTS.contains(&ext.as_str())
}

/// MOD 内静的アセットの HTTP Content-Type を拡張子から判定する
pub fn content_type_for_mod_asset(rel_path: &str) -> &'static str {
    let ext = Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" | "cjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" | "md" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" | "aac" => "audio/mp4",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "wasm" => "application/wasm",
        "glsl" | "vert" | "frag" | "rhai" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn json_value_to_default_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn parse_param_kind(raw: &str) -> ModParamKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "number" | "int" | "float" | "range" => ModParamKind::Number,
        "color" | "colour" | "hex" => ModParamKind::Color,
        "bool" | "boolean" | "toggle" => ModParamKind::Bool,
        "select" | "enum" | "choice" => ModParamKind::Select,
        _ => ModParamKind::String,
    }
}

fn normalize_hex_fallback(raw: &str, fallback: &str) -> String {
    let t = raw.trim();
    if t.starts_with('#') && (t.len() == 7 || t.len() == 9) && t[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        t.to_string()
    } else {
        fallback.to_string()
    }
}

/// MOD ディレクトリ単体の `mod.json` を検証・ロードする
fn parse_mod_folder(
    mod_dir: &Path,
    folder_name: &str,
    saved_states: &HashMap<String, ModSavedState>,
) -> InstalledMod {
    let manifest_path = mod_dir.join("mod.json");
    let make_error_mod = |id: String, name: String, err: String| InstalledMod {
        id,
        name,
        version: String::new(),
        author: String::new(),
        description: String::new(),
        min_stosup_version: String::new(),
        homepage: String::new(),
        license: String::new(),
        icon_url: String::new(),
        folder_name: folder_name.to_string(),
        enabled: false,
        installed_at_ms: 0,
        load_error: Some(err),
        reaction_effects: Vec::new(),
        overlay_themes: Vec::new(),
        custom_overlays: Vec::new(),
        logic_scripts: Vec::new(),
    };

    if !manifest_path.is_file() {
        return make_error_mod(
            folder_name.to_string(),
            folder_name.to_string(),
            "mod.json が見つかりません".to_string(),
        );
    }

    let raw_text = match fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            return make_error_mod(
                folder_name.to_string(),
                folder_name.to_string(),
                format!("mod.json を読み込めませんでした: {}", e),
            );
        }
    };

    // UTF-8 BOM を除去してパース
    let trimmed_text = raw_text.trim_start_matches('\u{feff}');
    let manifest: RawModManifest = match serde_json::from_str(trimmed_text) {
        Ok(m) => m,
        Err(e) => {
            return make_error_mod(
                folder_name.to_string(),
                folder_name.to_string(),
                format!("mod.json の JSON 構文エラー: {}", e),
            );
        }
    };

    let mod_id = manifest.id.trim().to_string();
    let mod_name = if manifest.name.trim().is_empty() {
        mod_id.clone()
    } else {
        manifest.name.trim().to_string()
    };

    if manifest.schema_version != 1 {
        return make_error_mod(
            mod_id,
            mod_name,
            format!(
                "未対応の schema_version ({}) です。現在は 1 のみ対応しています。",
                manifest.schema_version
            ),
        );
    }
    if !is_valid_mod_id(&mod_id) {
        return make_error_mod(
            folder_name.to_string(),
            mod_name,
            format!(
                "不正な MOD ID です（'{}'）。2〜64文字の半角英小文字・数字・ハイフン・アンダースコア（先頭は英数字）のみ使用できます。",
                mod_id
            ),
        );
    }

    let mut validation_errors: Vec<String> = Vec::new();

    // アイコン（安全なラスター画像拡張子のみ許可）
    let icon_rel = manifest.icon.trim().to_string();
    let icon_url = if !icon_rel.is_empty() {
        if !is_safe_relative_asset_path(&icon_rel) || !is_allowed_icon_path(&icon_rel) {
            validation_errors.push(format!(
                "icon は安全な画像ファイル（png/jpg/webp/gif 等）の相対パスで指定してください: {}",
                icon_rel
            ));
            String::new()
        } else if !mod_dir.join(&icon_rel).is_file() {
            validation_errors.push(format!("icon ファイルが見つかりません: {}", icon_rel));
            String::new()
        } else {
            format!("/mods/{}/{}", mod_id, icon_rel)
        }
    } else {
        String::new()
    };

    // 1. リアクション独自演出
    let mut reaction_effects = Vec::new();
    for ef in manifest.reaction_effects {
        let ef_id = ef.id.trim().to_string();
        if !is_valid_mod_id(&ef_id) {
            validation_errors.push(format!("reaction_effects の id が不正です: '{}'", ef_id));
            continue;
        }
        let entry = ef.entry.trim().to_string();
        if !is_safe_relative_asset_path(&entry) || !mod_dir.join(&entry).is_file() {
            validation_errors.push(format!(
                "リアクション演出 '{}' の entry ファイルが見つかりません: {}",
                ef_id, entry
            ));
            continue;
        }
        let css = ef.css.trim().to_string();
        if !css.is_empty() && (!is_safe_relative_asset_path(&css) || !mod_dir.join(&css).is_file())
        {
            validation_errors.push(format!(
                "リアクション演出 '{}' の css ファイルが見つかりません: {}",
                ef_id, css
            ));
            continue;
        }
        let params: Vec<ModParamSchema> = ef
            .params
            .into_iter()
            .filter_map(|p| {
                let key = p.key.trim().to_string();
                if key.is_empty() {
                    return None;
                }
                let label = if p.label.trim().is_empty() {
                    key.clone()
                } else {
                    p.label.trim().to_string()
                };
                Some(ModParamSchema {
                    key,
                    label,
                    kind: parse_param_kind(&p.kind),
                    default_value: json_value_to_default_string(&p.default_value),
                    min: p.min,
                    max: p.max,
                    step: p.step,
                    options: p.options,
                    description: p.description.trim().to_string(),
                })
            })
            .collect();

        reaction_effects.push(ModReactionEffectDef {
            mod_id: mod_id.clone(),
            mod_name: mod_name.clone(),
            effect_id: ef_id.clone(),
            name: if ef.name.trim().is_empty() {
                ef_id
            } else {
                ef.name.trim().to_string()
            },
            description: ef.description.trim().to_string(),
            entry,
            css,
            default_duration_ms: ef.default_duration_ms.clamp(500, 120_000),
            params,
        });
    }

    // 2. オーバーレイ追加テーマ
    let mut overlay_themes = Vec::new();
    for th in manifest.overlay_themes {
        let th_id = th.id.trim().to_string();
        if !is_valid_mod_id(&th_id) {
            validation_errors.push(format!("overlay_themes の id が不正です: '{}'", th_id));
            continue;
        }
        let css = th.css.trim().to_string();
        if !is_safe_relative_asset_path(&css) || !mod_dir.join(&css).is_file() {
            validation_errors.push(format!(
                "オーバーレイテーマ '{}' の css ファイルが見つかりません: {}",
                th_id, css
            ));
            continue;
        }
        let script = th.script.trim().to_string();
        if !script.is_empty()
            && (!is_safe_relative_asset_path(&script) || !mod_dir.join(&script).is_file())
        {
            validation_errors.push(format!(
                "オーバーレイテーマ '{}' の script ファイルが見つかりません: {}",
                th_id, script
            ));
            continue;
        }
        let mut targets: Vec<String> = th
            .targets
            .into_iter()
            .map(|t| t.trim().to_ascii_lowercase())
            .filter(|t| VALID_OVERLAY_TARGETS.contains(&t.as_str()))
            .collect();
        if targets.is_empty() {
            // targets 省略時は全 9 オーバーレイ対象
            targets = VALID_OVERLAY_TARGETS
                .iter()
                .map(|s| (*s).to_string())
                .collect();
        }
        overlay_themes.push(ModOverlayThemeDef {
            mod_id: mod_id.clone(),
            mod_name: mod_name.clone(),
            theme_id: th_id.clone(),
            full_theme_id: format!("mod:{}:{}", mod_id, th_id),
            name: if th.name.trim().is_empty() {
                th_id
            } else {
                th.name.trim().to_string()
            },
            description: th.description.trim().to_string(),
            targets,
            css,
            script,
            preview_bg: normalize_hex_fallback(&th.preview_bg, "#181C26"),
            preview_fg: normalize_hex_fallback(&th.preview_fg, "#F8FAFC"),
            preview_border: normalize_hex_fallback(&th.preview_border, "#38BDF8"),
        });
    }

    // 3. 独立カスタムオーバーレイ
    let mut custom_overlays = Vec::new();
    for ov in manifest.custom_overlays {
        let ov_id = ov.id.trim().to_string();
        if !is_valid_mod_id(&ov_id) {
            validation_errors.push(format!("custom_overlays の id が不正です: '{}'", ov_id));
            continue;
        }
        let entry = ov.entry.trim().to_string();
        if !is_safe_relative_asset_path(&entry) || !mod_dir.join(&entry).is_file() {
            validation_errors.push(format!(
                "独自オーバーレイ '{}' の entry HTML が見つかりません: {}",
                ov_id, entry
            ));
            continue;
        }
        let mut channels: Vec<String> = ov
            .channels
            .into_iter()
            .map(|c| c.trim().to_ascii_lowercase())
            .filter(|c| VALID_WS_CHANNELS.contains(&c.as_str()))
            .collect();
        if channels.is_empty() {
            channels.push("all".to_string());
        }
        custom_overlays.push(ModCustomOverlayDef {
            mod_id: mod_id.clone(),
            mod_name: mod_name.clone(),
            overlay_id: ov_id.clone(),
            name: if ov.name.trim().is_empty() {
                ov_id.clone()
            } else {
                ov.name.trim().to_string()
            },
            description: ov.description.trim().to_string(),
            entry,
            channels,
            recommended_width: ov.recommended_width.clamp(120, 3840),
            recommended_height: ov.recommended_height.clamp(80, 2160),
            overlay_path: format!("/overlay/mod/{}/{}", mod_id, ov_id),
            preview_path: format!("/overlay/preview/mod/{}/{}", mod_id, ov_id),
        });
    }

    // 4. ロジックスクリプト（.rhai）
    let mut logic_scripts = Vec::new();
    for sc in manifest.logic_scripts {
        let sc_id = sc.id.trim().to_string();
        if !is_valid_mod_id(&sc_id) {
            validation_errors.push(format!("logic_scripts の id が不正です: '{}'", sc_id));
            continue;
        }
        let entry = sc.entry.trim().to_string();
        if !is_safe_relative_asset_path(&entry) || !mod_dir.join(&entry).is_file() {
            validation_errors.push(format!(
                "ロジックスクリプト '{}' の entry ファイルが見つかりません: {}",
                sc_id, entry
            ));
            continue;
        }
        let script_path = mod_dir.join(&entry);
        // シンボリックリンクによる MOD ディレクトリ外ファイルの読み込みを防止
        let safe_script_path = match (fs::canonicalize(mod_dir), fs::canonicalize(&script_path)) {
            (Ok(root), Ok(target)) if target.starts_with(&root) && target.is_file() => target,
            _ => {
                validation_errors.push(format!(
                    "ロジックスクリプト '{}' ({}) は MOD ディレクトリ外の参照のため読み込めません",
                    sc_id, entry
                ));
                continue;
            }
        };
        match fs::read_to_string(&safe_script_path) {
            Ok(source) => {
                if source.len() > crate::mod_script::MAX_SCRIPT_SOURCE_BYTES {
                    validation_errors.push(format!(
                        "ロジックスクリプト '{}' のファイルサイズが上限（{} KB）を超えています",
                        sc_id,
                        crate::mod_script::MAX_SCRIPT_SOURCE_BYTES / 1024
                    ));
                    continue;
                }
                if let Err(err) = crate::mod_script::validate_rhai_syntax(&source) {
                    validation_errors.push(format!(
                        "ロジックスクリプト '{}' ({}) に {}",
                        sc_id, entry, err
                    ));
                    continue;
                }
            }
            Err(e) => {
                validation_errors.push(format!(
                    "ロジックスクリプト '{}' ({}) を UTF-8 テキストとして読み込めません: {}",
                    sc_id, entry, e
                ));
                continue;
            }
        }
        let mut events: Vec<String> = sc
            .events
            .into_iter()
            .map(|ev| ev.trim().to_ascii_lowercase())
            .filter(|ev| VALID_SCRIPT_EVENTS.contains(&ev.as_str()))
            .collect();
        if events.is_empty() {
            // events 省略時は全 4 イベントを購読
            events = VALID_SCRIPT_EVENTS
                .iter()
                .map(|s| (*s).to_string())
                .collect();
        }
        logic_scripts.push(ModLogicScriptDef {
            mod_id: mod_id.clone(),
            mod_name: mod_name.clone(),
            script_id: sc_id.clone(),
            name: if sc.name.trim().is_empty() {
                sc_id
            } else {
                sc.name.trim().to_string()
            },
            description: sc.description.trim().to_string(),
            entry,
            events,
        });
    }

    if reaction_effects.is_empty()
        && overlay_themes.is_empty()
        && custom_overlays.is_empty()
        && logic_scripts.is_empty()
        && validation_errors.is_empty()
    {
        validation_errors.push(
            "reaction_effects / overlay_themes / custom_overlays / logic_scripts のいずれかを1つ以上定義してください。"
                .to_string(),
        );
    }

    let load_error = if validation_errors.is_empty() {
        None
    } else {
        Some(validation_errors.join("\n"))
    };

    let saved = saved_states.get(&mod_id);
    let enabled = if load_error.is_some() {
        false
    } else {
        saved.map(|s| s.enabled).unwrap_or(true)
    };
    let installed_at_ms = saved.map(|s| s.installed_at_ms).unwrap_or(0);

    InstalledMod {
        id: mod_id,
        name: mod_name,
        version: manifest.version.trim().to_string(),
        author: manifest.author.trim().to_string(),
        description: manifest.description.trim().to_string(),
        min_stosup_version: manifest.min_stosup_version.trim().to_string(),
        homepage: manifest.homepage.trim().to_string(),
        license: manifest.license.trim().to_string(),
        icon_url,
        folder_name: folder_name.to_string(),
        enabled,
        installed_at_ms,
        load_error,
        reaction_effects,
        overlay_themes,
        custom_overlays,
        logic_scripts,
    }
}

/// `mods_dir` 配下の全 MOD フォルダをスキャンして一覧を返す
pub fn scan_installed_mods(
    mods_dir: &Path,
    saved_states: &HashMap<String, ModSavedState>,
) -> Vec<InstalledMod> {
    if !mods_dir.exists() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };

    let mut mods = Vec::new();
    let mut seen_ids = HashMap::<String, String>::new();

    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let Some(folder_name) = dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // 隠しフォルダや一時展開フォルダは無視
        if folder_name.starts_with('.') || folder_name.starts_with("__") {
            continue;
        }
        let mut parsed = parse_mod_folder(&dir, folder_name, saved_states);
        if parsed.load_error.is_none() {
            if let Some(prev_folder) = seen_ids.get(&parsed.id) {
                parsed.enabled = false;
                parsed.load_error = Some(format!(
                    "MOD ID '{}' がフォルダ '{}' と重複しているため無効化されました",
                    parsed.id, prev_folder
                ));
            } else {
                seen_ids.insert(parsed.id.clone(), folder_name.to_string());
            }
        }
        mods.push(parsed);
    }

    mods.sort_by(|a, b| {
        a.load_error
            .is_some()
            .cmp(&b.load_error.is_some())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    mods
}

/// 導入済み MOD から `mod_id` に対応する実ディレクトリを解決する
pub fn find_mod_directory(mods_dir: &Path, mod_id: &str) -> Option<PathBuf> {
    if !is_valid_mod_id(mod_id) || !mods_dir.exists() {
        return None;
    }
    // まずフォルダ名 == mod_id の高速パスを確認
    let direct = mods_dir.join(mod_id);
    if direct.is_dir() && direct.join("mod.json").is_file() {
        if let Ok(raw) = fs::read_to_string(direct.join("mod.json")) {
            if let Ok(m) = serde_json::from_str::<RawModManifest>(raw.trim_start_matches('\u{feff}'))
            {
                if m.id.trim() == mod_id {
                    return Some(direct);
                }
            }
        }
    }
    // 開発者が別名フォルダで配置している場合は全サブフォルダから mod.json の id 一致を探す
    let entries = fs::read_dir(mods_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("mod.json");
        if !manifest_path.is_file() {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&manifest_path) {
            if let Ok(m) = serde_json::from_str::<RawModManifest>(raw.trim_start_matches('\u{feff}'))
            {
                if m.id.trim() == mod_id {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// MOD 内静的アセットへの安全な絶対パスを解決する（パストラバーサル＆シンボリックリンク脱出を完全防御）
pub fn resolve_mod_asset_path(mods_dir: &Path, mod_id: &str, rel_path: &str) -> Result<PathBuf> {
    if !is_valid_mod_id(mod_id) {
        return Err(anyhow!("不正な MOD ID です"));
    }
    let clean_rel = rel_path.trim().trim_start_matches('/');
    if !is_safe_relative_asset_path(clean_rel) {
        return Err(anyhow!("不正なアセットパスです"));
    }
    let mod_dir = find_mod_directory(mods_dir, mod_id)
        .ok_or_else(|| anyhow!("MOD が見つかりません: {}", mod_id))?;
    let canon_root = fs::canonicalize(&mod_dir).context("MOD ルート解決に失敗しました")?;
    let candidate = mod_dir.join(clean_rel);
    let canon_target = fs::canonicalize(&candidate).context("指定ファイルが見つかりません")?;
    if !canon_target.starts_with(&canon_root) || !canon_target.is_file() {
        return Err(anyhow!("MOD ディレクトリ外へのアクセスは禁止されています"));
    }
    Ok(canon_target)
}

/// `.stosupmod` / `.zip` アーカイブを検証のうえ `mods_dir/<mod_id>` へ安全に導入する
pub fn import_mod_archive(
    mods_dir: &Path,
    archive_path: &Path,
    saved_states: &mut HashMap<String, ModSavedState>,
) -> Result<InstalledMod> {
    if !archive_path.is_file() {
        return Err(anyhow!(
            "指定された MOD ファイルが見つかりません: {}",
            archive_path.display()
        ));
    }

    let file = File::open(archive_path).with_context(|| {
        format!(
            "MOD アーカイブを開けませんでした: {}",
            archive_path.display()
        )
    })?;
    let mut archive = ZipArchive::new(file)
        .context("有効な ZIP / .stosupmod アーカイブではありません")?;

    if archive.is_empty() {
        return Err(anyhow!("アーカイブが空です"));
    }
    if archive.len() > MAX_ARCHIVE_FILES {
        return Err(anyhow!(
            "アーカイブ内のファイル数が上限（{} 件）を超えています",
            MAX_ARCHIVE_FILES
        ));
    }

    // ルート直下に mod.json があるか、単一サブフォルダ直下にあるかを検出
    let mut manifest_entry_path: Option<String> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let normalized = entry.name().replace('\\', "/");
        if normalized.starts_with("__MACOSX/") || normalized.contains("/__MACOSX/") {
            continue;
        }
        if normalized == "mod.json" {
            manifest_entry_path = Some(normalized);
            break;
        }
        let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() == 2 && parts[1] == "mod.json" && manifest_entry_path.is_none() {
            manifest_entry_path = Some(normalized);
        }
    }

    let manifest_entry = manifest_entry_path.ok_or_else(|| {
        anyhow!("アーカイブ内に mod.json が見つかりません（ルート直下または 1 階層フォルダ直下に配置してください）")
    })?;
    let strip_prefix = if let Some(idx) = manifest_entry.rfind('/') {
        format!("{}/", &manifest_entry[..idx])
    } else {
        String::new()
    };

    // 一時ステージングフォルダへ安全展開しながらサイズ・拡張子・パスを検証
    let stage_dir = std::env::temp_dir().join(format!("stosup-mod-imp-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&stage_dir)?;

    let extract_result = (|| -> Result<InstalledMod> {
        let mut total_written: u64 = 0;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let raw_name = entry.name().replace('\\', "/");
            if raw_name.starts_with("__MACOSX/") || raw_name.contains("/__MACOSX/") {
                continue;
            }
            let rel = if !strip_prefix.is_empty() {
                match raw_name.strip_prefix(&strip_prefix) {
                    Some(rest) => rest,
                    None => continue,
                }
            } else {
                raw_name.as_str()
            };
            let rel_trimmed = rel.trim_end_matches('/');
            if rel_trimmed.is_empty() {
                continue;
            }
            if entry.is_dir() {
                if !is_safe_relative_asset_path(rel_trimmed) {
                    return Err(anyhow!("不正なフォルダパスが含まれています: {}", rel_trimmed));
                }
                fs::create_dir_all(stage_dir.join(rel_trimmed))?;
                continue;
            }

            if !is_safe_relative_asset_path(rel_trimmed) {
                return Err(anyhow!("不正なファイルパスが含まれています: {}", rel_trimmed));
            }
            if is_forbidden_executable_path(rel_trimmed) {
                return Err(anyhow!(
                    "安全のため実行可能ファイルを含む MOD は導入できません: {}",
                    rel_trimmed
                ));
            }

            let out_path = stage_dir.join(rel_trimmed);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let out_file = File::create(&out_path)?;
            let mut writer = BufWriter::with_capacity(64 * 1024, out_file);
            let mut buf = [0u8; 16 * 1024];
            loop {
                let n = entry.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                total_written += n as u64;
                if total_written > MAX_ARCHIVE_TOTAL_BYTES {
                    return Err(anyhow!(
                        "展開サイズが上限（{} MB）を超えています",
                        MAX_ARCHIVE_TOTAL_BYTES / (1024 * 1024)
                    ));
                }
                writer.write_all(&buf[..n])?;
            }
            writer.flush()?;
        }

        // ステージング上で mod.json と参照アセット実在チェックを実行
        let empty_states = HashMap::new();
        let validated = parse_mod_folder(&stage_dir, "staged", &empty_states);
        if let Some(err) = validated.load_error {
            return Err(anyhow!("MOD マニフェストの検証に失敗しました:\n{}", err));
        }
        let mod_id = validated.id.clone();

        fs::create_dir_all(mods_dir).context("mods フォルダの作成に失敗しました")?;
        let target_dir = mods_dir.join(&mod_id);
        let existing_dir = find_mod_directory(mods_dir, &mod_id);
        // 先に既存 MOD を削除せず一時退避し、新フォルダ配置が成功してから削除（失敗時は自動ロールバック）
        let backup_dir = mods_dir.join(format!(".__replace_bak_{}_{}", mod_id, uuid::Uuid::new_v4()));
        let mut backed_up_origin: Option<PathBuf> = None;
        if let Some(ref existing) = existing_dir {
            if existing.exists() {
                fs::rename(existing, &backup_dir)?;
                backed_up_origin = Some(existing.clone());
            }
        } else if target_dir.exists() {
            fs::rename(&target_dir, &backup_dir)?;
            backed_up_origin = Some(target_dir.clone());
        }

        if let Err(e) = copy_dir_recursive(&stage_dir, &target_dir) {
            let _ = fs::remove_dir_all(&target_dir);
            if let Some(orig) = backed_up_origin {
                let _ = fs::rename(&backup_dir, orig);
            }
            return Err(e);
        }
        if backup_dir.exists() {
            let _ = fs::remove_dir_all(&backup_dir);
        }

        let now_ms = Utc::now().timestamp_millis();
        saved_states.insert(
            mod_id.clone(),
            ModSavedState {
                enabled: true,
                installed_at_ms: now_ms,
            },
        );
        let final_mod = parse_mod_folder(&target_dir, &mod_id, saved_states);
        Ok(final_mod)
    })();

    let _ = fs::remove_dir_all(&stage_dir);
    extract_result
}

/// 導入済み MOD をアンインストール（ディレクトリ削除）する
/// `mod_id_or_folder` にフォルダ名（重複エラー側など）が渡された場合はそのフォルダを優先解決する
pub fn uninstall_mod(
    mods_dir: &Path,
    mod_id_or_folder: &str,
    saved_states: &mut HashMap<String, ModSavedState>,
) -> Result<()> {
    let direct_folder = if is_safe_relative_asset_path(mod_id_or_folder)
        && !mod_id_or_folder.contains('/')
    {
        let p = mods_dir.join(mod_id_or_folder);
        if p.is_dir() {
            Some(p)
        } else {
            None
        }
    } else {
        None
    };

    let target = direct_folder
        .or_else(|| find_mod_directory(mods_dir, mod_id_or_folder))
        .ok_or_else(|| anyhow!("削除対象の MOD が見つかりません: {}", mod_id_or_folder))?;

    let canon_root = fs::canonicalize(mods_dir)?;
    let canon_target = fs::canonicalize(&target)?;
    if !canon_target.starts_with(&canon_root) || canon_target == canon_root {
        return Err(anyhow!("不正な削除パスです"));
    }

    fs::remove_dir_all(&canon_target).context("MOD フォルダの削除に失敗しました")?;
    // 同一 ID の正常側フォルダが他に残っていない場合のみ saved_states から除去
    if find_mod_directory(mods_dir, mod_id_or_folder).is_none() {
        saved_states.remove(mod_id_or_folder);
    }
    Ok(())
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let s = entry.path();
        let d = dst.join(entry.file_name());
        if s.is_dir() {
            copy_dir_recursive(&s, &d)?;
        } else if s.is_file() {
            fs::copy(&s, &d)?;
        }
    }
    Ok(())
}

/// OS のファイル選択ダイアログで `.stosupmod` / `.zip` ファイルを選択する
pub fn pick_mod_archive_dialog() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("導入する Stosup MOD パッケージ (.stosupmod / .zip) を選択")
        .add_filter("Stosup MOD パッケージ", &["stosupmod", "zip"])
        .add_filter("すべてのファイル", &["*"])
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

/// ローカルの MOD 配置フォルダを OS のファイラーで開く
pub fn open_mods_folder(mods_dir: &Path) -> Result<()> {
    fs::create_dir_all(mods_dir)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(mods_dir)
            .spawn()
            .context("エクスプローラーの起動に失敗しました")?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(mods_dir).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(mods_dir)
            .spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    fn create_test_zip(path: &Path, files: &[(&str, &str)]) {
        let f = File::create(path).unwrap();
        let mut zw = ZipWriter::new(f);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, content) in files {
            zw.start_file(*name, opts).unwrap();
            zw.write_all(content.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
    }

    #[test]
    fn test_mod_id_and_path_safety() {
        assert!(is_valid_mod_id("starlight-burst"));
        assert!(is_valid_mod_id("mod_01"));
        assert!(!is_valid_mod_id("A"));
        assert!(!is_valid_mod_id("Upper_Case"));
        assert!(!is_valid_mod_id("../escape"));
        assert!(!is_valid_mod_id("-leading-dash"));

        assert!(is_safe_relative_asset_path("effect.js"));
        assert!(is_safe_relative_asset_path("assets/star.png"));
        assert!(!is_safe_relative_asset_path("../secret.txt"));
        assert!(!is_safe_relative_asset_path("/etc/passwd"));
        assert!(!is_safe_relative_asset_path(".hidden/a.js"));
        assert!(!is_safe_relative_asset_path("a\\b.js"));
        assert!(is_forbidden_executable_path("payload.exe"));
        assert!(is_forbidden_executable_path("scripts/run.ps1"));
        assert!(!is_forbidden_executable_path("effect.js"));
    }

    #[test]
    fn test_import_scan_resolve_and_uninstall_mod_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        let zip_path = tmp.path().join("sample.stosupmod");

        let manifest = r#"{
            "schema_version": 1,
            "id": "demo-pack",
            "name": "デモ統合パック",
            "version": "1.0.0",
            "author": "tester",
            "description": "3種テスト",
            "reaction_effects": [
                {
                    "id": "spark",
                    "name": "スパーク",
                    "entry": "effect.js",
                    "css": "effect.css",
                    "params": [
                        { "key": "count", "label": "数", "kind": "number", "default_value": 20, "min": 1, "max": 100 }
                    ]
                }
            ],
            "overlay_themes": [
                {
                    "id": "neon_grid",
                    "name": "ネオングリッド",
                    "targets": ["comments", "superchat"],
                    "css": "theme.css"
                }
            ],
            "custom_overlays": [
                {
                    "id": "meter",
                    "name": "メーター",
                    "entry": "overlay.html",
                    "channels": ["comments", "superchat"]
                }
            ],
            "logic_scripts": [
                {
                    "id": "combo",
                    "name": "コンボ制御",
                    "entry": "logic.rhai",
                    "events": ["comment", "superchat"]
                }
            ]
        }"#;

        // 1階層フォルダ入り ZIP（GitHub Download ZIP 形式）でも自動認識することを確認
        create_test_zip(
            &zip_path,
            &[
                ("demo-pack-main/mod.json", manifest),
                ("demo-pack-main/effect.js", "console.log('ok');"),
                ("demo-pack-main/effect.css", ".fx { color: red; }"),
                ("demo-pack-main/theme.css", "body { color: cyan; }"),
                ("demo-pack-main/overlay.html", "<html><body>Hi</body></html>"),
                ("demo-pack-main/logic.rhai", "fn on_comment(ctx) { log(ctx.message); }"),
            ],
        );

        let mut states = HashMap::new();
        let imported = import_mod_archive(&mods_dir, &zip_path, &mut states).unwrap();
        assert_eq!(imported.id, "demo-pack");
        assert!(imported.enabled);
        assert!(imported.load_error.is_none());
        assert_eq!(imported.reaction_effects.len(), 1);
        assert_eq!(imported.overlay_themes.len(), 1);
        assert_eq!(
            imported.overlay_themes[0].full_theme_id,
            "mod:demo-pack:neon_grid"
        );
        assert_eq!(imported.custom_overlays.len(), 1);
        assert_eq!(
            imported.custom_overlays[0].overlay_path,
            "/overlay/mod/demo-pack/meter"
        );
        assert_eq!(imported.logic_scripts.len(), 1);
        assert_eq!(imported.logic_scripts[0].script_id, "combo");

        // スキャンおよびアセット解決
        let list = scan_installed_mods(&mods_dir, &states);
        assert_eq!(list.len(), 1);
        let asset = resolve_mod_asset_path(&mods_dir, "demo-pack", "theme.css").unwrap();
        assert!(asset.is_file());
        assert!(resolve_mod_asset_path(&mods_dir, "demo-pack", "../outside.txt").is_err());

        // アンインストール
        uninstall_mod(&mods_dir, "demo-pack", &mut states).unwrap();
        assert!(scan_installed_mods(&mods_dir, &states).is_empty());
    }

    #[test]
    fn test_reject_executable_inside_mod_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        let bad_zip = tmp.path().join("bad.zip");
        create_test_zip(
            &bad_zip,
            &[
                ("mod.json", r#"{"id":"bad-mod","name":"Bad","custom_overlays":[{"id":"o","name":"o","entry":"o.html"}]}"#),
                ("o.html", "<html></html>"),
                ("evil.exe", "MZ"),
            ],
        );
        let mut states = HashMap::new();
        let err = import_mod_archive(&mods_dir, &bad_zip, &mut states).unwrap_err();
        assert!(err.to_string().contains("実行可能ファイル"));
    }

    #[test]
    fn test_bundled_example_mods_load_cleanly() {
        let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("examples")
            .join("mods");
        assert!(examples_dir.is_dir(), "examples/mods が存在すること");
        let states = HashMap::new();
        let scanned = scan_installed_mods(&examples_dir, &states);
        assert_eq!(scanned.len(), 4, "4種類の実働サンプルMODがすべて検出されること");
        for m in &scanned {
            assert!(
                m.load_error.is_none(),
                "サンプル MOD '{}' にロードエラーがないこと: {:?}",
                m.id,
                m.load_error
            );
            assert!(m.enabled);
        }
        assert!(scanned.iter().any(|m| m.id == "starlight-burst-effect" && m.reaction_effects.len() == 1));
        assert!(scanned.iter().any(|m| m.id == "retro-arcade-theme" && m.overlay_themes.len() == 1));
        assert!(scanned.iter().any(|m| m.id == "hype-meter-overlay" && m.custom_overlays.len() == 1));
        assert!(scanned.iter().any(|m| {
            m.id == "combo-fever-script"
                && m.logic_scripts.len() == 1
                && m.reaction_effects.len() == 1
        }));
    }

    #[test]
    fn test_invalid_rhai_script_reports_load_error() {
        let tmp = tempfile::tempdir().unwrap();
        let mod_dir = tmp.path().join("broken-script");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("mod.json"),
            r#"{"schema_version":1,"id":"broken-script","name":"Broken","logic_scripts":[{"id":"s1","name":"S1","entry":"bad.rhai"}]}"#,
        )
        .unwrap();
        fs::write(mod_dir.join("bad.rhai"), "fn on_comment(ctx { broken").unwrap();
        let scanned = scan_installed_mods(tmp.path(), &HashMap::new());
        assert_eq!(scanned.len(), 1);
        assert!(!scanned[0].enabled);
        assert!(scanned[0]
            .load_error
            .as_deref()
            .unwrap_or("")
            .contains("構文エラー"));
    }
}
