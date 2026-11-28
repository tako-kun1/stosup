# Stosup MOD ロジックスクリプト開発ガイド（`.rhai`）

> **特別な開発環境やコンパイル作業は一切不要です。**  
> テキストエディタで `.rhai` ファイルを書いて `%APPDATA%\Stosup\mods\` に置き、MOD 管理画面で **「再読み込み」** を押すだけで即座に動作します。

このガイドでは、Stosup 本体に組み込まれた軽量スクリプト言語 **[Rhai](https://rhai.rs/)** を使って、コメントやスーパーチャット・初見・抽選に応じた **独自の条件判定・コンボ／状態管理・動的演出発火・OBS 自動操作** を記述する方法を解説します。

- 総合ガイド（`mod.json` / 4 種類の MOD 構造 / 配布方法）: [`MOD_DEVELOPMENT.md`](./MOD_DEVELOPMENT.md)
- フロントエンド JS SDK・WebSocket・DOM 仕様: [`MOD_API_REFERENCE.md`](./MOD_API_REFERENCE.md)
- そのまま動かせるサンプル MOD: [`examples/mods/combo-fever-script/`](../examples/mods/combo-fever-script/)

---

## 1. ロジックスクリプト（`.rhai`）でできること

フロントエンドの JavaScript（`reaction_effects` / `overlay_themes` / `custom_overlays`）が **「画面の見え方・アニメーション」** を担当するのに対し、ロジックスクリプト（`logic_scripts`）は **Stosup 本体の裏側で動く「ルール・状態・連動ロジック」** を担当します。

1 つの MOD パッケージの中に `logic_scripts` と `reaction_effects` を同梱すれば、次のような高度な演出が **シンプルなスクリプトだけで** 実現できます。

- **コンボ／ゲージ集計演出**: 「15 秒以内に『草 / www』が 3 回続いたら、コンボ数に応じたド派手な専用 Canvas 演出を発火」
- **金額・権限別の分岐演出**: 「¥1,000 以上なら通常演出、¥5,000 以上なら豪華演出＋OBS ソースを 5 秒間点灯、モデレーターの特定コマンドでテロップ表示」
- **初見さん歓迎オートメーション**: 「初見さんが来たら専用演出を鳴らし、配信内の累計初見数が 10 人ごとにお祝い演出を追加発火」
- **抽選当選の演出強化**: 「抽選で当選者が決まった瞬間にクラッカー演出と OBS シーン演出を連動」

---

## 2. 最小構成のサンプル（30 秒で動く）

### `mod.json`
```json
{
  "schema_version": 1,
  "id": "my-first-script",
  "name": "はじめてのロジックスクリプト",
  "version": "1.0.0",
  "author": "あなたの名前",
  "description": "「乾杯」コメントに反応して標準クラッカー演出を発火します。",
  "logic_scripts": [
    {
      "id": "cheers",
      "name": "乾杯リアクション",
      "description": "「乾杯」または「かんぱい」に反応",
      "entry": "logic.rhai",
      "events": ["comment"]
    }
  ]
}
```

### `logic.rhai`
```rhai
fn on_comment(ctx) {
    if ctx.is_banned {
        return;
    }
    if contains_any(ctx.message, ["乾杯", "かんぱい", "cheers"]) {
        let total = state_inc("cheers_count", 1);
        log(`${ctx.user_name} さんが乾杯しました（本日 ${total} 回目）`);
        trigger_builtin_effect(
            "cracker",
            `${ctx.user_name} さんと乾杯！ (${total}回目)`,
            "🍻,🥂,🎉,✨"
        );
    }
}

// MOD 管理画面の「テスト実行」ボタンを押したときに呼ばれます
fn on_test(ctx) {
    log("テスト実行を開始します");
    trigger_builtin_effect("confetti", "乾杯テスト成功！", "🍻,✨,🎉");
}
```

---

## 3. `mod.json` の `logic_scripts` フィールド仕様

```json
"logic_scripts": [
  {
    "id": "combo_controller",
    "name": "コンボ・フィーバー制御",
    "description": "連続リアクションを検知して演出パラメータを動的制御します",
    "entry": "logic.rhai",
    "events": ["comment", "superchat", "first_time", "lottery"]
  }
]
```

| フィールド | 必須 | 説明 |
|---|---|---|
| `id` | **必須** | スクリプト ID（2〜64 文字の英小文字・数字・`-`・`_`）。 |
| `name` | **必須** | MOD 管理画面に表示される名称。 |
| `description` | 任意 | スクリプトの説明。 |
| `entry` | **必須** | `.rhai` ファイルの MOD フォルダからの相対パス（上限 512 KB）。 |
| `events` | 任意 | 購読するイベント配列。省略時は全 4 種 (`["comment", "superchat", "first_time", "lottery"]`) を購読します。 |

※ MOD 読み込み時に自動で構文チェックが行われ、文法エラーがある場合は MOD カードに **エラー行・列番号付き** で詳細が表示されます。

---

## 4. イベントフック関数一覧

スクリプト内に以下の関数を定義しておくと、対応するイベント発生時に Stosup が自動呼び出しします（使わない関数は省略して構いません）。

| 関数名 | 対応 `events` 値 | 呼ばれるタイミング |
|---|---|---|
| `fn on_comment(ctx)` | `"comment"` | 通常コメント・スパチャ・初見を含む全コメント受信時 |
| `fn on_superchat(ctx)` | `"superchat"` | スーパーチャット・スーパーステッカー・メンバーシップ受信時 |
| `fn on_first_time(ctx)` | `"first_time"` | 初見視聴者が検知されたとき |
| `fn on_lottery(ctx)` | `"lottery"` | ユーザー抽選またはコメント抽選で当選者が確定したとき |
| `fn on_test(ctx)` | （常時有効） | MOD 管理画面から「テスト実行」を押したとき（未定義の場合はサンプルデータで `on_comment` 等へ自動フォールバック） |

### 4.1 `ctx` のプロパティ（コメント系: `on_comment` / `on_superchat` / `on_first_time` / `on_test`）

| プロパティ | 型 | 内容 |
|---|---|---|
| `ctx.event` | 文字列 | `"comment"` / `"superchat"` / `"first_time"` / `"test"` |
| `ctx.comment_id` | 文字列 | コメント ID |
| `ctx.user_id` | 文字列 | 視聴者アカウント ID |
| `ctx.user_name` | 文字列 | 視聴者表示名 |
| `ctx.message` | 文字列 | コメント本文 |
| `ctx.platform` | 文字列 | `"youtube"` / `"twitch"` / `"demo"` |
| `ctx.is_first_time` | 真偽値 | 初見視聴者かどうか |
| `ctx.is_owner` | 真偽値 | 配信者（チャンネル所有者）かどうか |
| `ctx.is_moderator` | 真偽値 | モデレーターかどうか |
| `ctx.is_member` | 真偽値 | メンバーシップ加入者かどうか |
| `ctx.is_banned` | 真偽値 | アプリ内 BAN 対象かどうか |
| `ctx.is_superchat` | 真偽値 | スーパーチャット／スーパーステッカーかどうか |
| `ctx.is_membership` | 真偽値 | メンバーシップ加入・継続・ギフトかどうか |
| `ctx.event_kind` | 文字列 | `""` / `"super_chat"` / `"super_sticker"` / `"membership_new"` / `"membership_milestone"` / `"membership_gift"` |
| `ctx.amount_jpy` | 整数 | スパチャ等の金額（円相当の整数。通常コメントは `0`） |
| `ctx.amount_display` | 文字列 | 表示用金額文字列（例: `"¥1,000"`） |
| `ctx.timestamp_ms` | 整数 | 受信時刻（Unix ミリ秒） |
| `ctx.is_test` | 真偽値 | MOD 管理画面からのテスト実行時は `true` |

### 4.2 `ctx` のプロパティ（抽選系: `on_lottery`）

| プロパティ | 型 | 内容 |
|---|---|---|
| `ctx.event` | 文字列 | `"lottery"` |
| `ctx.kind` | 文字列 | `"user"`（ユーザー抽選）または `"comment"`（コメント抽選） |
| `ctx.candidate_count` | 整数 | 応募・候補人数 |
| `ctx.winner_count` | 整数 | 当選人数 |
| `ctx.winners` | 配列 | 当選者マップの配列（各要素に `user_id`, `user_name`, `comment_text`） |
| `ctx.winner_names` | 配列 | 当選者表示名の文字列配列 |
| `ctx.drawn_at_ms` | 整数 | 抽選時刻（Unix ミリ秒） |
| `ctx.is_test` | 真偽値 | テスト実行時は `true` |

---

## 5. 組み込み API 関数リファレンス

### 5.1 状態ストア（MOD 単位・配信中インメモリ保持）

同じ MOD 内の全スクリプト間で共有され、他 MOD とは完全に分離された Key-Value ストアです（上限 256 キー／MOD）。コンボ数・クールダウン時刻・ユーザーごとの記録などに利用できます。

| 関数 | 説明 |
|---|---|
| `state_get(key)` | キーの値を取得（未設定時は `()`） |
| `state_get_or(key, default_val)` | キーの値を取得（未設定時は `default_val` を返す） |
| `state_set(key, value)` | キーへ任意の値（数値・文字列・配列・マップ等）を保存 |
| `state_inc(key, delta)` | 整数値へ `delta` を加算して保存し、新しい整数値を返す（未設定時は `0 + delta`） |
| `state_remove(key)` | 指定キーを削除 |
| `state_clear()` | この MOD の状態キーをすべて削除 |

### 5.2 リアクション演出の発火

| 関数 | 説明 |
|---|---|
| `trigger_mod_effect(effect_id)` | 自 MOD 内の `reaction_effects` を既定設定で発火 |
| `trigger_mod_effect(effect_id, params_map)` | 動的パラメータ（例: `#{ combo: 5, color: "#FF6B6B" }`）を渡して発火 |
| `trigger_mod_effect(effect_id, params_map, caption)` | テロップ文言付きで発火 |
| `trigger_mod_effect(effect_id, params_map, caption, duration_ms)` | 再生時間（ミリ秒: 500〜120000）も指定して発火 |
| `trigger_other_mod_effect(mod_id, effect_id, params_map, caption)` | 他の導入済み有効 MOD の演出を発火 |
| `trigger_builtin_effect(effect_type, caption)` | Stosup 標準演出を発火（`effect_type`: `"confetti"` / `"cracker"` / `"drop"` / `"throw"` / `"ambient"` / `"fixed"`） |
| `trigger_builtin_effect(effect_type, caption, particle_text)` | 絵文字・文字パーティクル指定付き（例: `"草,w,🌿"`）で標準演出を発火 |
| `trigger_builtin_effect(effect_type, caption, particle_text, particle_count, duration_ms)` | 個数（1〜200）・再生時間（500〜60000ms）も指定して発火 |
| `trigger_media_rule(name_or_id)` | 配信者がリアクションステージ画面で作成済みの演出ルールを名前または ID で発火 |
| `clear_media()` | 現在再生中のリアクション演出を即時クリア |

### 5.3 スポットテロップ（コメントピックアップ）

| 関数 | 説明 |
|---|---|
| `pickup_current_comment()` | 発火元コメントをスポットテロップ（ピックアップ）へ表示 |
| `clear_pickup()` | スポットテロップの表示を解除 |

### 5.4 OBS Studio 自動操作（OBS WebSocket 接続時のみ実行）

※ MOD 管理画面からの **プレビューテスト実行時は本番 OBS を誤操作しないよう自動でスキップ** されます。

| 関数 | 説明 |
|---|---|
| `obs_set_scene(scene_name)` | 指定した OBS シーンへ切り替え |
| `obs_set_source_visible(scene_name, source_name, visible)` | 指定シーンのソース表示/非表示を切り替え（`scene_name` に `""` を渡すと現在のアクティブシーンを対象） |
| `obs_pulse_source(scene_name, source_name, duration_sec)` | 指定ソースを表示し、`duration_sec` 秒（0.2〜60 秒）後に自動で非表示へ戻す |

### 5.5 時刻・文字列ヘルパー・ログ・通知

| 関数 | 説明 |
|---|---|
| `now_ms()` | 現在の Unix 時刻（ミリ秒整数） |
| `contains_any(text, keywords_array)` | `text` に配列内のキーワードのいずれかが含まれるか（大文字小文字を区別しない） |
| `to_lower(text)` | 文字列を小文字化 |
| `str_len(text)` | 文字数（Unicode 文字単位）を返す |
| `log(value)` / `print(value)` | MOD 管理画面の「スクリプト実行ログ」へ `info` ログを出力 |
| `log_warn(value)` | 実行ログへ `warn` ログを出力 |
| `notify(message)` / `notify(level, message)` | Stosup アプリ画面にシステム通知を表示（`level`: `"info"` / `"success"` / `"warn"` / `"error"`） |

---

## 6. Rhai 構文クイックチートシート（JavaScript との違い）

Rhai は JavaScript によく似たとても簡潔なスクリプト言語です。

```rhai
// 変数（let のみ。再代入も可能）
let count = 0;
count += 1;

// 文字列補間はバッククォート `${変数}`
let msg = `現在 ${count} コンボです！`;

// オブジェクト（マップ）は #{ ... } と先頭に # を付ける点だけ JS と異なります
let params = #{
    combo: count,
    accentColor: "#22C55E",
    isFever: count >= 5,
};

// 配列
let words = ["草", "www", "ワロタ"];

// 条件分岐
if count >= 5 {
    log("フィーバー！");
} else if count >= 3 {
    log("コンボ中！");
} else {
    log("通常");
}

// 関数定義（イベントフック以外の自作ヘルパー関数も自由に書けます）
fn calc_tier(amount) {
    if amount >= 10000 { return "red"; }
    if amount >= 5000  { return "magenta"; }
    if amount >= 1000  { return "yellow"; }
    return "blue";
}
```

---

## 7. 実践レシピ集

### レシピ A: クールダウン付きモデレーター専用コマンド
```rhai
fn on_comment(ctx) {
    if !(ctx.is_owner || ctx.is_moderator) {
        return;
    }
    if ctx.message == "!break" {
        obs_set_scene("休憩画面");
        notify("info", `${ctx.user_name} が休憩シーンへ切り替えました`);
    } else if ctx.message == "!clear" {
        clear_media();
        clear_pickup();
    }
}
```

### レシピ B: 10 秒クールダウン付きキーワード反応
```rhai
fn on_comment(ctx) {
    if ctx.is_banned { return; }
    if !contains_any(ctx.message, ["ナイス", "gg", "うま"]) { return; }

    let now = now_ms();
    let last = state_get_or("last_gg_ms", 0);
    if now - last < 10_000 {
        return; // 10 秒以内の連打は無視
    }
    state_set("last_gg_ms", now);

    trigger_builtin_effect("confetti", "NICE PLAY!!", "👏,🔥,✨,GG");
}
```

### レシピ C: スパチャ金額別の豪華演出 ＋ OBS ソース一時点灯
```rhai
fn on_superchat(ctx) {
    if !ctx.is_superchat { return; }

    if ctx.amount_jpy >= 5000 {
        obs_pulse_source("", "豪華スポットライト", 6.0);
        trigger_builtin_effect(
            "cracker",
            `${ctx.user_name} さん ${ctx.amount_display} ありがとうございます！`,
            "💎,👑,🎉,✨",
            80,
            6000
        );
    }
}
```

---

## 8. サンドボックス制限とデバッグ手順

Stosup のスクリプト実行環境は、配信者の PC と配信画面を守るため以下の安全制限のもとで動作します。

| 項目 | 上限・方針 |
|---|---|
| 1 回の実行あたりの最大ステップ数 | `50,000` ops（無限ループは自動で強制停止し、ログへエラー出力） |
| 1 回の実行あたりの最大アクション数 | `16` 件（過剰連発を防止） |
| 状態ストア上限 | 1 MOD あたり最大 `256` キー |
| 関数ネスト深度 | 最大 `32` 階層 |
| ファイルシステム / OS コマンド / DB / 認証トークン | **完全不可**（アクセス API 自体が存在しません） |

### デバッグのコツ
1. `.rhai` ファイルを編集したら、MOD 管理画面の **「再読み込み」** をクリックします（構文エラーがあれば即座にカードに表示されます）。
2. 各スクリプト行の **「テスト実行（プレビュー）」** をクリックすると、本番環境を汚さずにプレビュー環境（`/overlay/preview/media` 等）へ演出を発火できます。
3. MOD 管理画面下部の **「スクリプト実行ログ（デバッグコンソール）」** に `log(...)` の出力・発火したアクション・実行時エラーが時系列でリアルタイム表示されます。
