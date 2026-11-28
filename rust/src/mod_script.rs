use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;

use chrono::Utc;
use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Scope, AST};

use crate::models::{
    CommentItem, InstalledMod, LotteryResult, ModScriptLogEntry, MonetizationEventKind,
};
use crate::mods::find_mod_directory;

/// 1 回のスクリプト実行で許可する最大操作数（無限ループ強制停止）
const MAX_OPERATIONS: u64 = 50_000;
/// 最大関数呼び出しネスト深度
const MAX_CALL_LEVELS: usize = 32;
/// 式の最大ネスト深度
const MAX_EXPR_DEPTH: usize = 48;
/// 関数内式の最大ネスト深度
const MAX_FUNC_EXPR_DEPTH: usize = 24;
/// 文字列最大サイズ（バイト）
const MAX_STRING_SIZE: usize = 64 * 1024;
/// 配列最大要素数
const MAX_ARRAY_SIZE: usize = 1024;
/// マップ最大キー数
const MAX_MAP_SIZE: usize = 512;
/// 1 回のスクリプト実行で発火できる最大アクション数（暴走防止）
const MAX_ACTIONS_PER_RUN: usize = 16;
/// 1 MOD あたり保持できる状態キー上限
const MAX_STATE_KEYS_PER_MOD: usize = 256;
/// 状態ストア 1 エントリあたりの最大値サイズ（JSON 換算 16 KB）
const MAX_STATE_VALUE_BYTES: usize = 16 * 1024;
/// `contains_any` の最大キーワード走査件数
const MAX_CONTAINS_ANY_KEYWORDS: usize = 128;
/// 実行ログのリングバッファ最大保持件数
const MAX_SCRIPT_LOGS: usize = 200;
/// 1 スクリプトソースファイルの最大読み込みサイズ（512 KB）
pub const MAX_SCRIPT_SOURCE_BYTES: usize = 512 * 1024;

/// スクリプト実行が Stosup 本体へ要求する安全なアクション
#[derive(Debug, Clone)]
pub enum ModScriptAction {
    /// MOD 独自のリアクション演出を発火
    TriggerModEffect {
        mod_id: String,
        effect_id: String,
        params_json: String,
        caption: String,
        duration_ms: Option<u32>,
    },
    /// 組み込みリアクション演出を発火
    TriggerBuiltinEffect {
        effect_type: String,
        caption: String,
        particle_text: String,
        particle_count: Option<u32>,
        duration_ms: Option<u32>,
    },
    /// 既存のリアクションステージルールを名前または ID で発火
    TriggerMediaRule { rule_name_or_id: String },
    /// リアクションステージの演出をクリア
    ClearMedia,
    /// 発火元のコメントをスポットテロップ（ピックアップ）へ表示
    PickupCurrentComment,
    /// スポットテロップを解除
    ClearPickup,
    /// OBS Studio のシーンを切り替え
    ObsSwitchScene { scene_name: String },
    /// OBS Studio のソース表示/非表示を切り替え
    ObsSetSourceVisible {
        scene_name: String,
        source_name: String,
        visible: bool,
    },
    /// OBS Studio のソースを一時表示し、指定ミリ秒後に自動で非表示へ戻す
    ObsPulseSource {
        scene_name: String,
        source_name: String,
        duration_ms: u64,
    },
    /// Stosup UI へのシステム通知バー表示
    Notify { level: String, message: String },
}

impl ModScriptAction {
    /// テスト実行時のサマリー表示向け日本語説明
    pub fn describe(&self) -> String {
        match self {
            ModScriptAction::TriggerModEffect {
                mod_id,
                effect_id,
                caption,
                ..
            } => {
                if caption.is_empty() {
                    format!("MOD演出発火 ({}:{})", mod_id, effect_id)
                } else {
                    format!("MOD演出発火 ({}:{} / 「{}」)", mod_id, effect_id, caption)
                }
            }
            ModScriptAction::TriggerBuiltinEffect {
                effect_type,
                caption,
                ..
            } => {
                if caption.is_empty() {
                    format!("標準演出発火 ({})", effect_type)
                } else {
                    format!("標準演出発火 ({} / 「{}」)", effect_type, caption)
                }
            }
            ModScriptAction::TriggerMediaRule { rule_name_or_id } => {
                format!("リアクションルール発火 ({})", rule_name_or_id)
            }
            ModScriptAction::ClearMedia => "リアクション演出をクリア".to_string(),
            ModScriptAction::PickupCurrentComment => "コメントをスポットテロップに表示".to_string(),
            ModScriptAction::ClearPickup => "スポットテロップを解除".to_string(),
            ModScriptAction::ObsSwitchScene { scene_name } => {
                format!("OBSシーン切替 (「{}」)", scene_name)
            }
            ModScriptAction::ObsSetSourceVisible {
                scene_name,
                source_name,
                visible,
            } => {
                let target = if scene_name.is_empty() {
                    source_name.clone()
                } else {
                    format!("{} / {}", scene_name, source_name)
                };
                format!(
                    "OBSソース{} ({})",
                    if *visible { "表示" } else { "非表示" },
                    target
                )
            }
            ModScriptAction::ObsPulseSource {
                scene_name,
                source_name,
                duration_ms,
            } => {
                let target = if scene_name.is_empty() {
                    source_name.clone()
                } else {
                    format!("{} / {}", scene_name, source_name)
                };
                format!(
                    "OBSソース一時表示 ({} / {:.1}秒)",
                    target,
                    (*duration_ms as f64) / 1000.0
                )
            }
            ModScriptAction::Notify { level, message } => {
                format!("アプリ通知 [{}]: {}", level, message)
            }
        }
    }
}

/// 実行中のスクリプト 1 本に紐づく一時コンテキスト（スレッドローカルに束縛）
struct ActiveScriptContext {
    mod_id: String,
    script_id: String,
    state: HashMap<String, Dynamic>,
    actions: Vec<ModScriptAction>,
    logs: Vec<ModScriptLogEntry>,
    action_limit_warned: bool,
}

impl ActiveScriptContext {
    fn new(mod_id: String, script_id: String, state: HashMap<String, Dynamic>) -> Self {
        Self {
            mod_id,
            script_id,
            state,
            actions: Vec::new(),
            logs: Vec::new(),
            action_limit_warned: false,
        }
    }

    fn push_log(&mut self, level: &str, message: impl Into<String>) {
        let mut msg = message.into();
        if msg.len() > 800 {
            // UTF-8 文字境界に合わせて安全に切り詰め
            let mut end = 800;
            while end > 0 && !msg.is_char_boundary(end) {
                end -= 1;
            }
            msg.truncate(end);
            msg.push('…');
        }
        self.logs.push(ModScriptLogEntry {
            timestamp_ms: Utc::now().timestamp_millis(),
            mod_id: self.mod_id.clone(),
            script_id: self.script_id.clone(),
            level: level.to_string(),
            message: msg,
        });
    }

    fn push_action(&mut self, action: ModScriptAction) {
        if self.actions.len() >= MAX_ACTIONS_PER_RUN {
            if !self.action_limit_warned {
                self.action_limit_warned = true;
                self.push_log(
                    "warn",
                    format!(
                        "1回の実行あたりのアクション上限（{}件）に達したため、以降の要求を無視しました",
                        MAX_ACTIONS_PER_RUN
                    ),
                );
            }
            return;
        }
        self.push_log("action", action.describe());
        self.actions.push(action);
    }
}

thread_local! {
    static ACTIVE_CTX: RefCell<Option<ActiveScriptContext>> = const { RefCell::new(None) };
}

/// パニック時を含めて確実にスレッドローカルコンテキストを回収する RAII ガード
struct ActiveContextGuard {}

impl ActiveContextGuard {
    fn enter(ctx: ActiveScriptContext) -> Self {
        ACTIVE_CTX.with(|cell| {
            *cell.borrow_mut() = Some(ctx);
        });
        Self {}
    }

    fn take(self) -> Option<ActiveScriptContext> {
        let taken = ACTIVE_CTX.with(|cell| cell.borrow_mut().take());
        std::mem::forget(self);
        taken
    }
}

impl Drop for ActiveContextGuard {
    fn drop(&mut self) {
        ACTIVE_CTX.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

fn with_active_ctx<R>(f: impl FnOnce(&mut ActiveScriptContext) -> R) -> Option<R> {
    ACTIVE_CTX.with(|cell| {
        let mut guard = cell.borrow_mut();
        guard.as_mut().map(f)
    })
}

fn map_to_json_string(params: Map) -> String {
    match rhai::serde::from_dynamic::<serde_json::Value>(&Dynamic::from(params)) {
        Ok(v) => v.to_string(),
        Err(_) => "{}".to_string(),
    }
}

/// サンドボックス化された Rhai エンジンを構築する（FS / OS / ネットワーク機能は一切公開しない）
fn build_sandboxed_engine() -> Engine {
    let mut engine = Engine::new();

    // 動的 eval() の実行を明示的に無効化（静的コンパイル済み AST のみ実行）
    engine.disable_symbol("eval");

    // サンドボックス実行上限（無限ループ・メモリ枯渇を完全防御）
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_FUNC_EXPR_DEPTH);
    engine.set_max_string_size(MAX_STRING_SIZE);
    engine.set_max_array_size(MAX_ARRAY_SIZE);
    engine.set_max_map_size(MAX_MAP_SIZE);

    // print() / debug() を実行ログビューアへ流す
    engine.on_print(|text| {
        with_active_ctx(|ctx| ctx.push_log("info", text.to_string()));
    });
    engine.on_debug(|text, source, pos| {
        let loc = match source {
            Some(s) if !pos.is_none() => format!("[{}:{}] {}", s, pos, text),
            _ if !pos.is_none() => format!("[{}] {}", pos, text),
            _ => text.to_string(),
        };
        with_active_ctx(|ctx| ctx.push_log("info", loc));
    });

    // ==================== 状態ストア（MOD 単位のインメモリ KV） ====================
    engine.register_fn("state_get", |key: ImmutableString| -> Dynamic {
        let k = key.trim();
        if k.is_empty() {
            return Dynamic::UNIT;
        }
        with_active_ctx(|ctx| ctx.state.get(k).cloned().unwrap_or(Dynamic::UNIT))
            .unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn(
        "state_get_or",
        |key: ImmutableString, default_val: Dynamic| -> Dynamic {
            let k = key.trim();
            if k.is_empty() {
                return default_val;
            }
            with_active_ctx(|ctx| ctx.state.get(k).cloned().unwrap_or(default_val.clone()))
                .unwrap_or(default_val)
        },
    );

    engine.register_fn("state_set", |key: ImmutableString, value: Dynamic| {
        with_active_ctx(|ctx| {
            let k = key.trim();
            if k.is_empty() || k.len() > 128 {
                return;
            }
            if !ctx.state.contains_key(k) && ctx.state.len() >= MAX_STATE_KEYS_PER_MOD {
                ctx.push_log(
                    "warn",
                    format!(
                        "状態ストアのキー数上限（{}件）に達したため '{}' を保存できませんでした",
                        MAX_STATE_KEYS_PER_MOD, k
                    ),
                );
                return;
            }
            let est_size = rhai::serde::from_dynamic::<serde_json::Value>(&value)
                .ok()
                .and_then(|v| serde_json::to_vec(&v).ok())
                .map(|b| b.len())
                .unwrap_or_else(|| value.to_string().len());
            if est_size > MAX_STATE_VALUE_BYTES {
                ctx.push_log(
                    "warn",
                    format!(
                        "状態ストアの値サイズが上限（{} KB）を超えたため '{}' を保存できませんでした",
                        MAX_STATE_VALUE_BYTES / 1024,
                        k
                    ),
                );
                return;
            }
            ctx.state.insert(k.to_string(), value);
        });
    });

    engine.register_fn("state_inc", |key: ImmutableString, delta: i64| -> i64 {
        with_active_ctx(|ctx| {
            let k = key.trim();
            if k.is_empty() || k.len() > 128 {
                return 0;
            }
            let current = ctx
                .state
                .get(k)
                .and_then(|v| v.as_int().ok())
                .unwrap_or(0);
            let next = current.saturating_add(delta);
            if ctx.state.contains_key(k) || ctx.state.len() < MAX_STATE_KEYS_PER_MOD {
                ctx.state.insert(k.to_string(), Dynamic::from_int(next));
            }
            next
        })
        .unwrap_or(0)
    });

    engine.register_fn("state_remove", |key: ImmutableString| {
        let k = key.trim();
        if k.is_empty() {
            return;
        }
        with_active_ctx(|ctx| {
            ctx.state.remove(k);
        });
    });

    engine.register_fn("state_clear", || {
        with_active_ctx(|ctx| {
            ctx.state.clear();
        });
    });

    // ==================== 時刻・文字列・判定ヘルパー ====================
    engine.register_fn("now_ms", || -> i64 { Utc::now().timestamp_millis() });

    engine.register_fn(
        "contains_any",
        |text: ImmutableString, keywords: Array| -> bool {
            let text_slice = if text.len() > 4096 {
                let mut end = 4096;
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                &text[..end]
            } else {
                text.as_str()
            };
            let lower = text_slice.to_lowercase();
            for item in keywords.into_iter().take(MAX_CONTAINS_ANY_KEYWORDS) {
                if let Ok(kw) = item.into_immutable_string() {
                    let kw_trimmed = kw.trim();
                    if !kw_trimmed.is_empty()
                        && kw_trimmed.len() <= 256
                        && lower.contains(&kw_trimmed.to_lowercase())
                    {
                        return true;
                    }
                }
            }
            false
        },
    );

    engine.register_fn("to_lower", |text: ImmutableString| -> String {
        text.to_lowercase()
    });

    engine.register_fn("str_len", |text: ImmutableString| -> i64 {
        text.chars().count() as i64
    });

    // ==================== ログ・通知 ====================
    engine.register_fn("log", |msg: Dynamic| {
        with_active_ctx(|ctx| ctx.push_log("info", msg.to_string()));
    });

    engine.register_fn("log_warn", |msg: Dynamic| {
        with_active_ctx(|ctx| ctx.push_log("warn", msg.to_string()));
    });

    engine.register_fn("notify", |message: ImmutableString| {
        let msg = message.trim().to_string();
        if msg.is_empty() {
            return;
        }
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::Notify {
                level: "info".to_string(),
                message: msg,
            });
        });
    });

    engine.register_fn(
        "notify",
        |level: ImmutableString, message: ImmutableString| {
            let msg = message.trim().to_string();
            if msg.is_empty() {
                return;
            }
            let lv = match level.trim().to_ascii_lowercase().as_str() {
                "warn" | "warning" => "warn",
                "error" => "error",
                "success" => "success",
                _ => "info",
            };
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::Notify {
                    level: lv.to_string(),
                    message: msg,
                });
            });
        },
    );

    // ==================== リアクション演出の発火 ====================
    engine.register_fn("trigger_mod_effect", |effect_id: ImmutableString| {
        with_active_ctx(|ctx| {
            let mod_id = ctx.mod_id.clone();
            ctx.push_action(ModScriptAction::TriggerModEffect {
                mod_id,
                effect_id: effect_id.trim().to_string(),
                params_json: "{}".to_string(),
                caption: String::new(),
                duration_ms: None,
            });
        });
    });

    engine.register_fn(
        "trigger_mod_effect",
        |effect_id: ImmutableString, params: Map| {
            let params_json = map_to_json_string(params);
            with_active_ctx(|ctx| {
                let mod_id = ctx.mod_id.clone();
                ctx.push_action(ModScriptAction::TriggerModEffect {
                    mod_id,
                    effect_id: effect_id.trim().to_string(),
                    params_json,
                    caption: String::new(),
                    duration_ms: None,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_mod_effect",
        |effect_id: ImmutableString, params: Map, caption: ImmutableString| {
            let params_json = map_to_json_string(params);
            with_active_ctx(|ctx| {
                let mod_id = ctx.mod_id.clone();
                ctx.push_action(ModScriptAction::TriggerModEffect {
                    mod_id,
                    effect_id: effect_id.trim().to_string(),
                    params_json,
                    caption: caption.trim().to_string(),
                    duration_ms: None,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_mod_effect",
        |effect_id: ImmutableString, params: Map, caption: ImmutableString, duration_ms: i64| {
            let params_json = map_to_json_string(params);
            let dur = if duration_ms > 0 {
                Some(duration_ms.clamp(500, 120_000) as u32)
            } else {
                None
            };
            with_active_ctx(|ctx| {
                let mod_id = ctx.mod_id.clone();
                ctx.push_action(ModScriptAction::TriggerModEffect {
                    mod_id,
                    effect_id: effect_id.trim().to_string(),
                    params_json,
                    caption: caption.trim().to_string(),
                    duration_ms: dur,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_other_mod_effect",
        |target_mod_id: ImmutableString,
         effect_id: ImmutableString,
         params: Map,
         caption: ImmutableString| {
            let params_json = map_to_json_string(params);
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::TriggerModEffect {
                    mod_id: target_mod_id.trim().to_string(),
                    effect_id: effect_id.trim().to_string(),
                    params_json,
                    caption: caption.trim().to_string(),
                    duration_ms: None,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_builtin_effect",
        |effect_type: ImmutableString, caption: ImmutableString| {
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::TriggerBuiltinEffect {
                    effect_type: effect_type.trim().to_ascii_lowercase(),
                    caption: caption.trim().to_string(),
                    particle_text: String::new(),
                    particle_count: None,
                    duration_ms: None,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_builtin_effect",
        |effect_type: ImmutableString,
         caption: ImmutableString,
         particle_text: ImmutableString| {
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::TriggerBuiltinEffect {
                    effect_type: effect_type.trim().to_ascii_lowercase(),
                    caption: caption.trim().to_string(),
                    particle_text: particle_text.trim().to_string(),
                    particle_count: None,
                    duration_ms: None,
                });
            });
        },
    );

    engine.register_fn(
        "trigger_builtin_effect",
        |effect_type: ImmutableString,
         caption: ImmutableString,
         particle_text: ImmutableString,
         particle_count: i64,
         duration_ms: i64| {
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::TriggerBuiltinEffect {
                    effect_type: effect_type.trim().to_ascii_lowercase(),
                    caption: caption.trim().to_string(),
                    particle_text: particle_text.trim().to_string(),
                    particle_count: if particle_count > 0 {
                        Some(particle_count.clamp(1, 200) as u32)
                    } else {
                        None
                    },
                    duration_ms: if duration_ms > 0 {
                        Some(duration_ms.clamp(500, 60_000) as u32)
                    } else {
                        None
                    },
                });
            });
        },
    );

    engine.register_fn("trigger_media_rule", |name_or_id: ImmutableString| {
        let target = name_or_id.trim().to_string();
        if target.is_empty() {
            return;
        }
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::TriggerMediaRule {
                rule_name_or_id: target,
            });
        });
    });

    engine.register_fn("clear_media", || {
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::ClearMedia);
        });
    });

    // ==================== スポットテロップ（ピックアップ） ====================
    engine.register_fn("pickup_current_comment", || {
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::PickupCurrentComment);
        });
    });

    engine.register_fn("clear_pickup", || {
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::ClearPickup);
        });
    });

    // ==================== OBS Studio 自動操作 ====================
    engine.register_fn("obs_set_scene", |scene_name: ImmutableString| {
        let sc = scene_name.trim().to_string();
        if sc.is_empty() {
            return;
        }
        with_active_ctx(|ctx| {
            ctx.push_action(ModScriptAction::ObsSwitchScene { scene_name: sc });
        });
    });

    engine.register_fn(
        "obs_set_source_visible",
        |scene_name: ImmutableString, source_name: ImmutableString, visible: bool| {
            let src = source_name.trim().to_string();
            if src.is_empty() {
                return;
            }
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::ObsSetSourceVisible {
                    scene_name: scene_name.trim().to_string(),
                    source_name: src,
                    visible,
                });
            });
        },
    );

    engine.register_fn(
        "obs_pulse_source",
        |scene_name: ImmutableString, source_name: ImmutableString, duration_sec: f64| {
            let src = source_name.trim().to_string();
            if src.is_empty() {
                return;
            }
            let safe_sec = if duration_sec.is_finite() {
                duration_sec.clamp(0.2, 60.0)
            } else {
                3.0
            };
            let ms = (safe_sec * 1000.0).round() as u64;
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::ObsPulseSource {
                    scene_name: scene_name.trim().to_string(),
                    source_name: src,
                    duration_ms: ms,
                });
            });
        },
    );

    engine.register_fn(
        "obs_pulse_source",
        |scene_name: ImmutableString, source_name: ImmutableString, duration_sec: i64| {
            let src = source_name.trim().to_string();
            if src.is_empty() {
                return;
            }
            let ms = ((duration_sec.clamp(1, 60) as u64) * 1000).clamp(200, 60_000);
            with_active_ctx(|ctx| {
                ctx.push_action(ModScriptAction::ObsPulseSource {
                    scene_name: scene_name.trim().to_string(),
                    source_name: src,
                    duration_ms: ms,
                });
            });
        },
    );

    engine
}

/// `.rhai` スクリプトの構文チェックを単体実行する（`mods.rs` スキャン時の即時エラー報告用）
pub fn validate_rhai_syntax(source: &str) -> Result<(), String> {
    let engine = build_sandboxed_engine();
    engine
        .compile(source)
        .map(|_| ())
        .map_err(|e| format!("構文エラー ({}): {}", e.position(), e.err_type()))
}

struct CompiledModScript {
    mod_id: String,
    script_id: String,
    events: Vec<String>,
    ast: AST,
    has_on_comment: bool,
    has_on_superchat: bool,
    has_on_first_time: bool,
    has_on_lottery: bool,
    has_on_test: bool,
}

impl CompiledModScript {
    fn has_fn(ast: &AST, name: &str) -> bool {
        ast.iter_functions().any(|f| f.name == name)
    }

    fn subscribes_to(&self, event_name: &str) -> bool {
        self.events.iter().any(|e| e == event_name)
    }
}

/// 導入済み MOD の全 Rhai ロジックスクリプトを事前コンパイル・保持・実行するランタイム
pub struct ModScriptRuntime {
    engine: Engine,
    scripts: Vec<CompiledModScript>,
    /// MOD ID ごとの独立インメモリ状態ストア
    mod_states: HashMap<String, HashMap<String, Dynamic>>,
    /// リアルタイムデバッグ用の実行ログリングバッファ
    logs: VecDeque<ModScriptLogEntry>,
}

impl Default for ModScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ModScriptRuntime {
    pub fn new() -> Self {
        Self {
            engine: build_sandboxed_engine(),
            scripts: Vec::new(),
            mod_states: HashMap::new(),
            logs: VecDeque::new(),
        }
    }

    pub fn has_any_scripts(&self) -> bool {
        !self.scripts.is_empty()
    }

    fn append_log(&mut self, entry: ModScriptLogEntry) {
        self.logs.push_back(entry);
        while self.logs.len() > MAX_SCRIPT_LOGS {
            self.logs.pop_front();
        }
    }

    pub fn list_logs(&self, limit: usize) -> Vec<ModScriptLogEntry> {
        let take = limit.clamp(1, MAX_SCRIPT_LOGS);
        self.logs.iter().rev().take(take).cloned().collect()
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    /// 有効な MOD 一覧からすべての `.rhai` スクリプトを再読み込み・事前コンパイルする
    pub fn reload_from_mods(&mut self, mods_dir: &Path, installed_mods: &[InstalledMod]) {
        self.scripts.clear();

        // 無効化または削除された MOD の状態ストアは掃除しつつ、有効継続中の MOD 状態は温存する
        let active_mod_ids: std::collections::HashSet<&str> = installed_mods
            .iter()
            .filter(|m| m.enabled && m.load_error.is_none() && !m.logic_scripts.is_empty())
            .map(|m| m.id.as_str())
            .collect();
        self.mod_states
            .retain(|k, _| active_mod_ids.contains(k.as_str()));

        for m in installed_mods {
            if !m.enabled || m.load_error.is_some() || m.logic_scripts.is_empty() {
                continue;
            }
            let Some(mod_dir) = find_mod_directory(mods_dir, &m.id) else {
                continue;
            };
            let Ok(canon_mod_dir) = mod_dir.canonicalize() else {
                continue;
            };
            for def in &m.logic_scripts {
                let script_path = mod_dir.join(&def.entry);
                let canon_script = match script_path.canonicalize() {
                    Ok(p) if p.starts_with(&canon_mod_dir) => p,
                    _ => {
                        self.append_log(ModScriptLogEntry {
                            timestamp_ms: Utc::now().timestamp_millis(),
                            mod_id: m.id.clone(),
                            script_id: def.script_id.clone(),
                            level: "error".to_string(),
                            message: "スクリプトパスが MOD フォルダ外を参照しているため読み込みを拒否しました"
                                .to_string(),
                        });
                        continue;
                    }
                };
                let source = match fs::read_to_string(&canon_script) {
                    Ok(s) => s,
                    Err(e) => {
                        self.append_log(ModScriptLogEntry {
                            timestamp_ms: Utc::now().timestamp_millis(),
                            mod_id: m.id.clone(),
                            script_id: def.script_id.clone(),
                            level: "error".to_string(),
                            message: format!("スクリプトの読み込みに失敗しました: {}", e),
                        });
                        continue;
                    }
                };
                if source.len() > MAX_SCRIPT_SOURCE_BYTES {
                    self.append_log(ModScriptLogEntry {
                        timestamp_ms: Utc::now().timestamp_millis(),
                        mod_id: m.id.clone(),
                        script_id: def.script_id.clone(),
                        level: "error".to_string(),
                        message: format!(
                            "スクリプトサイズ上限（{} KB）を超えています",
                            MAX_SCRIPT_SOURCE_BYTES / 1024
                        ),
                    });
                    continue;
                }
                match self.engine.compile(&source) {
                    Ok(ast) => {
                        let has_on_comment = CompiledModScript::has_fn(&ast, "on_comment");
                        let has_on_superchat = CompiledModScript::has_fn(&ast, "on_superchat");
                        let has_on_first_time = CompiledModScript::has_fn(&ast, "on_first_time");
                        let has_on_lottery = CompiledModScript::has_fn(&ast, "on_lottery");
                        let has_on_test = CompiledModScript::has_fn(&ast, "on_test");
                        self.scripts.push(CompiledModScript {
                            mod_id: m.id.clone(),
                            script_id: def.script_id.clone(),
                            events: def.events.clone(),
                            ast,
                            has_on_comment,
                            has_on_superchat,
                            has_on_first_time,
                            has_on_lottery,
                            has_on_test,
                        });
                    }
                    Err(e) => {
                        self.append_log(ModScriptLogEntry {
                            timestamp_ms: Utc::now().timestamp_millis(),
                            mod_id: m.id.clone(),
                            script_id: def.script_id.clone(),
                            level: "error".to_string(),
                            message: format!(
                                "コンパイル失敗 ({}): {}",
                                e.position(),
                                e.err_type()
                            ),
                        });
                    }
                }
            }
        }
    }

    fn invoke_script_fn(
        &mut self,
        script_idx: usize,
        fn_name: &str,
        ctx_map: Map,
    ) -> Vec<ModScriptAction> {
        let (mod_id, script_id, ast) = {
            let s = &self.scripts[script_idx];
            (s.mod_id.clone(), s.script_id.clone(), s.ast.clone())
        };

        let saved_state = self.mod_states.remove(&mod_id).unwrap_or_default();
        let active_ctx = ActiveScriptContext::new(mod_id.clone(), script_id.clone(), saved_state);
        let guard = ActiveContextGuard::enter(active_ctx);

        let mut scope = Scope::new();
        let call_res: Result<Dynamic, Box<EvalAltResult>> =
            self.engine.call_fn(&mut scope, &ast, fn_name, (ctx_map,));

        let finished = guard.take();
        let mut actions = Vec::new();
        if let Some(done) = finished {
            self.mod_states.insert(mod_id.clone(), done.state);
            for log in done.logs {
                self.append_log(log);
            }
            actions = done.actions;
        }

        if let Err(err) = call_res {
            let msg = match err.as_ref() {
                EvalAltResult::ErrorTooManyOperations(_) => {
                    format!(
                        "{}() が最大ステップ上限（{}）を超えたため安全のため強制停止しました（無限ループを確認してください）",
                        fn_name, MAX_OPERATIONS
                    )
                }
                other => format!("{}() 実行エラー: {}", fn_name, other),
            };
            self.append_log(ModScriptLogEntry {
                timestamp_ms: Utc::now().timestamp_millis(),
                mod_id,
                script_id,
                level: "error".to_string(),
                message: msg,
            });
        }

        actions
    }

    /// コメント受信イベント（comment / first_time / superchat）に対して全該当スクリプトを実行
    pub fn dispatch_comment(
        &mut self,
        item: &CommentItem,
        amount_micros: u64,
        is_first_time: bool,
    ) -> Vec<ModScriptAction> {
        if self.scripts.is_empty() {
            return Vec::new();
        }
        let mut all_actions = Vec::new();
        let is_monetization = item.event_kind.is_some();

        let script_count = self.scripts.len();
        for idx in 0..script_count {
            let (run_comment, run_first, run_sc) = {
                let s = &self.scripts[idx];
                (
                    s.has_on_comment && s.subscribes_to("comment"),
                    is_first_time && s.has_on_first_time && s.subscribes_to("first_time"),
                    is_monetization && s.has_on_superchat && s.subscribes_to("superchat"),
                )
            };

            if run_comment {
                let map = build_comment_ctx_map(item, amount_micros, is_first_time, "comment", false);
                all_actions.extend(self.invoke_script_fn(idx, "on_comment", map));
            }
            if run_first {
                let map =
                    build_comment_ctx_map(item, amount_micros, is_first_time, "first_time", false);
                all_actions.extend(self.invoke_script_fn(idx, "on_first_time", map));
            }
            if run_sc {
                let map =
                    build_comment_ctx_map(item, amount_micros, is_first_time, "superchat", false);
                all_actions.extend(self.invoke_script_fn(idx, "on_superchat", map));
            }
        }
        all_actions
    }

    /// 抽選当選イベントに対して全該当スクリプトを実行
    pub fn dispatch_lottery(&mut self, result: &LotteryResult) -> Vec<ModScriptAction> {
        if self.scripts.is_empty() {
            return Vec::new();
        }
        let mut all_actions = Vec::new();
        let script_count = self.scripts.len();
        for idx in 0..script_count {
            let should_run = {
                let s = &self.scripts[idx];
                s.has_on_lottery && s.subscribes_to("lottery")
            };
            if should_run {
                let map = build_lottery_ctx_map(result, false);
                all_actions.extend(self.invoke_script_fn(idx, "on_lottery", map));
            }
        }
        all_actions
    }

    /// 指定スクリプト 1 本を手動テスト実行する（`on_test` があれば優先、無ければ `on_comment` へフォールバック）
    pub fn test_script(
        &mut self,
        mod_id: &str,
        script_id: &str,
        sample_comment: &CommentItem,
        sample_amount_micros: u64,
    ) -> Result<Vec<ModScriptAction>, String> {
        let Some(idx) = self
            .scripts
            .iter()
            .position(|s| s.mod_id == mod_id && s.script_id == script_id)
        else {
            return Err(format!(
                "有効なロジックスクリプトが見つかりません（{}:{}）。MOD が有効かつエラーなしであることを確認してください。",
                mod_id, script_id
            ));
        };

        let (fn_name, event_label) = {
            let s = &self.scripts[idx];
            if s.has_on_test {
                ("on_test", "test")
            } else if s.has_on_comment {
                ("on_comment", "comment")
            } else if s.has_on_superchat {
                ("on_superchat", "superchat")
            } else if s.has_on_first_time {
                ("on_first_time", "first_time")
            } else if s.has_on_lottery {
                let sample_lottery = LotteryResult {
                    id: "test-lottery".to_string(),
                    kind: "comment".to_string(),
                    winners: vec![crate::models::LotteryWinner {
                        user_id: sample_comment.user_id.clone(),
                        user_name: sample_comment.user_name.clone(),
                        user_icon_url: sample_comment.user_icon_url.clone(),
                        comment_text: Some(sample_comment.message.clone()),
                        won_at_ms: sample_comment.timestamp_ms,
                    }],
                    candidate_count: 10,
                    drawn_at_ms: sample_comment.timestamp_ms,
                };
                let map = build_lottery_ctx_map(&sample_lottery, true);
                return Ok(self.invoke_script_fn(idx, "on_lottery", map));
            } else {
                return Err(
                    "実行可能なイベント関数（on_test / on_comment / on_superchat / on_first_time / on_lottery）が定義されていません"
                        .to_string(),
                );
            }
        };

        // フォールバック先イベント種別に合わせてダミーコンテキスト（スパチャ / 初見）を補完する
        let mut effective_comment = sample_comment.clone();
        let mut effective_amount_micros = sample_amount_micros;
        if fn_name == "on_superchat" && effective_comment.event_kind.is_none() {
            effective_comment.event_kind = Some(MonetizationEventKind::SuperChat);
            effective_comment.amount_display = Some("¥1,000".to_string());
            effective_amount_micros = 1_000_000_000;
        }
        if fn_name == "on_first_time" {
            effective_comment.is_first_time = true;
        }

        let map = build_comment_ctx_map(
            &effective_comment,
            effective_amount_micros,
            effective_comment.is_first_time,
            event_label,
            true,
        );
        Ok(self.invoke_script_fn(idx, fn_name, map))
    }
}

fn build_comment_ctx_map(
    item: &CommentItem,
    amount_micros: u64,
    is_first_time: bool,
    event_name: &str,
    is_test: bool,
) -> Map {
    let mut map = Map::new();
    let is_superchat = matches!(
        item.event_kind,
        Some(MonetizationEventKind::SuperChat | MonetizationEventKind::SuperSticker)
    );
    let is_membership = matches!(
        item.event_kind,
        Some(
            MonetizationEventKind::MembershipNew
                | MonetizationEventKind::MembershipMilestone
                | MonetizationEventKind::MembershipGift
        )
    );
    map.insert("event".into(), Dynamic::from(event_name.to_string()));
    map.insert("comment_id".into(), Dynamic::from(item.id.clone()));
    map.insert("user_id".into(), Dynamic::from(item.user_id.clone()));
    map.insert("user_name".into(), Dynamic::from(item.user_name.clone()));
    map.insert("message".into(), Dynamic::from(item.message.clone()));
    map.insert(
        "platform".into(),
        Dynamic::from(item.platform.as_str().to_string()),
    );
    map.insert("is_first_time".into(), Dynamic::from_bool(is_first_time));
    map.insert("is_owner".into(), Dynamic::from_bool(item.is_owner));
    map.insert("is_moderator".into(), Dynamic::from_bool(item.is_moderator));
    map.insert("is_member".into(), Dynamic::from_bool(item.is_member));
    map.insert("is_banned".into(), Dynamic::from_bool(item.is_banned));
    map.insert("is_superchat".into(), Dynamic::from_bool(is_superchat));
    map.insert("is_membership".into(), Dynamic::from_bool(is_membership));
    map.insert(
        "event_kind".into(),
        Dynamic::from(
            item.event_kind
                .map(|k| k.as_str())
                .unwrap_or("")
                .to_string(),
        ),
    );
    map.insert(
        "amount_jpy".into(),
        Dynamic::from_int((amount_micros / 1_000_000) as i64),
    );
    map.insert(
        "amount_display".into(),
        Dynamic::from(item.amount_display.clone().unwrap_or_default()),
    );
    map.insert(
        "timestamp_ms".into(),
        Dynamic::from_int(item.timestamp_ms),
    );
    map.insert("is_test".into(), Dynamic::from_bool(is_test));
    map
}

fn build_lottery_ctx_map(result: &LotteryResult, is_test: bool) -> Map {
    let mut map = Map::new();
    let mut winners_arr = Array::new();
    let mut winner_names = Array::new();
    for w in &result.winners {
        let mut wm = Map::new();
        wm.insert("user_id".into(), Dynamic::from(w.user_id.clone()));
        wm.insert("user_name".into(), Dynamic::from(w.user_name.clone()));
        wm.insert(
            "comment_text".into(),
            Dynamic::from(w.comment_text.clone().unwrap_or_default()),
        );
        winners_arr.push(Dynamic::from(wm));
        winner_names.push(Dynamic::from(w.user_name.clone()));
    }
    map.insert("event".into(), Dynamic::from("lottery".to_string()));
    map.insert("kind".into(), Dynamic::from(result.kind.clone()));
    map.insert(
        "candidate_count".into(),
        Dynamic::from_int(result.candidate_count as i64),
    );
    map.insert(
        "winner_count".into(),
        Dynamic::from_int(result.winners.len() as i64),
    );
    map.insert("winners".into(), Dynamic::from(winners_arr));
    map.insert("winner_names".into(), Dynamic::from(winner_names));
    map.insert("drawn_at_ms".into(), Dynamic::from_int(result.drawn_at_ms));
    map.insert("is_test".into(), Dynamic::from_bool(is_test));
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModLogicScriptDef, PlatformKind};

    fn make_comment(msg: &str) -> CommentItem {
        CommentItem {
            id: "c1".to_string(),
            session_id: "s1".to_string(),
            platform: PlatformKind::Demo,
            user_id: "u1".to_string(),
            user_name: "テスト視聴者".to_string(),
            user_icon_url: String::new(),
            user_memo: String::new(),
            is_first_time: false,
            message: msg.to_string(),
            message_runs: Vec::new(),
            timestamp_ms: Utc::now().timestamp_millis(),
            is_owner: false,
            is_moderator: false,
            is_member: false,
            is_banned: false,
            is_ng_masked: false,
            hidden_from_overlay: false,
            highlight_rule_id: None,
            bubble_bg_color: None,
            bubble_text_color: None,
            bubble_border_color: None,
            event_kind: None,
            amount_display: None,
            event_accent_color: None,
        }
    }

    fn setup_runtime_with_script(source: &str) -> (tempfile::TempDir, ModScriptRuntime) {
        let tmp = tempfile::tempdir().unwrap();
        let mod_dir = tmp.path().join("combo-test");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(
            mod_dir.join("mod.json"),
            r#"{"schema_version":1,"id":"combo-test","name":"Combo Test","logic_scripts":[{"id":"main","name":"Main","entry":"logic.rhai"}]}"#,
        )
        .unwrap();
        fs::write(mod_dir.join("logic.rhai"), source).unwrap();

        let installed = vec![InstalledMod {
            id: "combo-test".to_string(),
            name: "Combo Test".to_string(),
            version: "1.0.0".to_string(),
            author: "tester".to_string(),
            description: String::new(),
            min_stosup_version: String::new(),
            homepage: String::new(),
            license: String::new(),
            icon_url: String::new(),
            folder_name: "combo-test".to_string(),
            enabled: true,
            installed_at_ms: 0,
            load_error: None,
            reaction_effects: Vec::new(),
            overlay_themes: Vec::new(),
            custom_overlays: Vec::new(),
            logic_scripts: vec![ModLogicScriptDef {
                mod_id: "combo-test".to_string(),
                mod_name: "Combo Test".to_string(),
                script_id: "main".to_string(),
                name: "Main".to_string(),
                description: String::new(),
                entry: "logic.rhai".to_string(),
                events: vec![
                    "comment".to_string(),
                    "superchat".to_string(),
                    "first_time".to_string(),
                    "lottery".to_string(),
                ],
            }],
        }];

        let mut rt = ModScriptRuntime::new();
        rt.reload_from_mods(tmp.path(), &installed);
        (tmp, rt)
    }

    #[test]
    fn test_rhai_combo_state_and_dynamic_effect_trigger() {
        let script = r#"
            fn on_comment(ctx) {
                if contains_any(ctx.message, ["草", "www"]) {
                    let count = state_inc("kusa_combo", 1);
                    log(`コンボ現在値: ${count}`);
                    if count >= 3 {
                        trigger_mod_effect("combo_burst", #{ combo: count, label: "草フィーバー！" }, `${count}コンボ達成！`);
                        state_set("kusa_combo", 0);
                    }
                }
            }
        "#;
        let (_tmp, mut rt) = setup_runtime_with_script(script);

        assert!(rt.dispatch_comment(&make_comment("こんにちは"), 0, false).is_empty());
        assert!(rt.dispatch_comment(&make_comment("草"), 0, false).is_empty());
        assert!(rt.dispatch_comment(&make_comment("WWW"), 0, false).is_empty());

        let actions = rt.dispatch_comment(&make_comment("大草原"), 0, false);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            ModScriptAction::TriggerModEffect {
                mod_id,
                effect_id,
                params_json,
                caption,
                ..
            } => {
                assert_eq!(mod_id, "combo-test");
                assert_eq!(effect_id, "combo_burst");
                assert_eq!(caption, "3コンボ達成！");
                let v: serde_json::Value = serde_json::from_str(params_json).unwrap();
                assert_eq!(v["combo"], 3);
                assert_eq!(v["label"], "草フィーバー！");
            }
            other => panic!("期待しないアクション: {:?}", other),
        }

        // ログにコンボ記録とアクションが残っていること
        let logs = rt.list_logs(20);
        assert!(logs.iter().any(|l| l.message.contains("コンボ現在値: 3")));
    }

    #[test]
    fn test_rhai_sandbox_stops_infinite_loop_safely() {
        let script = r#"
            fn on_comment(ctx) {
                loop {
                    let x = 1 + 1;
                }
            }
        "#;
        let (_tmp, mut rt) = setup_runtime_with_script(script);
        let actions = rt.dispatch_comment(&make_comment("trigger"), 0, false);
        assert!(actions.is_empty());
        let logs = rt.list_logs(10);
        assert!(
            logs.iter()
                .any(|l| l.level == "error" && l.message.contains("強制停止")),
            "無限ループが上限で強制停止されエラーログが出ること: {:?}",
            logs
        );
    }

    #[test]
    fn test_rhai_syntax_validation() {
        assert!(validate_rhai_syntax("fn on_comment(ctx) { log(ctx.message); }").is_ok());
        assert!(validate_rhai_syntax("fn on_comment(ctx { broken").is_err());
        assert!(
            validate_rhai_syntax("fn on_comment(ctx) { eval(\"1 + 1\"); }").is_err(),
            "eval() はサンドボックスによりコンパイル時に拒否されること"
        );
    }

    #[test]
    fn test_rhai_safety_guards_and_test_fallbacks() {
        // state_get / state_set の前後空白トリム統一 & NaN ガード & eval 無効化
        let script = r#"
            fn on_superchat(ctx) {
                state_set("  key1  ", 42);
                let v = state_get("key1");
                if ctx.is_superchat && ctx.amount_jpy >= 1000 && v == 42 {
                    obs_pulse_source("", "Cam", 0.0 / 0.0);
                }
            }
        "#;
        let (_tmp, mut rt) = setup_runtime_with_script(script);
        let actions = rt
            .test_script("combo-test", "main", &make_comment("テスト"), 0)
            .unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            ModScriptAction::ObsPulseSource { duration_ms, .. } => {
                assert_eq!(*duration_ms, 3000);
            }
            other => panic!("期待しないアクション: {:?}", other),
        }
    }
}
