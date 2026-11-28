# Stosup 演出 MOD 開発公式ガイド

Stosup の演出まわり（リアクションステージ・全 9 種 OBS オーバーレイ・独立カスタムオーバーレイ・イベント連動ロジック）を拡張するためのユーザー作成「**MOD**」の制作・テスト・配布方法を解説します。

> **特別なビルド環境やコンパイル作業は一切不要です！**  
> 画面演出は **HTML / CSS / JavaScript**、コメント反応やコンボ・OBS 自動操作などの裏側ロジックは組み込みスクリプト言語 **Rhai (`.rhai`)** で記述できます。テキストエディタだけで作成し、「再読み込み」ボタン 1 つで即座に反映できます。

> **対象バージョン**: Stosup v0.6.0 以降（`schema_version: 1` / `StosupMod SDK 1.0.0` / Rhai ロジックスクリプト対応）  
> **ロジックスクリプト開発ガイド（`.rhai`）**: [MOD_SCRIPT_GUIDE.md](./MOD_SCRIPT_GUIDE.md)  
> **フロントエンド API リファレンス**: JS SDK・WebSocket イベント・DOM セレクタの詳細は [MOD_API_REFERENCE.md](./MOD_API_REFERENCE.md) を参照してください。  
> **実働サンプル集**: そのまま導入して動かせる 4 種類のサンプルが [`examples/mods/`](../examples/mods/README.md) に用意されています。

---

## 1. Stosup MOD で作れる 4 種類の拡張機能

1 つの MOD パッケージ（フォルダまたは `.stosupmod` / `.zip` ファイル）には、以下の 4 種類のコンポーネントを **1 つだけでも、複数組み合わせても** 収録できます。

| 種別 (`mod.json` キー) | 使用言語 | できること | 利用される場所 |
|---|---|---|---|
| **① リアクション独自演出** (`reaction_effects`) | JavaScript / CSS | コメント・スパチャ・メンバーシップ・初見などに反応して動く独自エフェクト（Canvas / WebGL / DOM / CSS / 画像・効果音）。Stosup GUI 上で編集できるカスタムパラメータも定義可能。 | リアクションステージ画面のルール編集 → 演出の種類「カスタム MOD」 |
| **② オーバーレイ追加テーマ** (`overlay_themes`) | CSS / 任意 JS | トークモニターやエールボックスなど既存 9 種類の OBS オーバーレイへ、オリジナルのデザインテーマ（CSS ＋ 任意 JS フック ＋ 同梱アセット）を追加。 | オーバーレイスタジオ画面 → 各オーバーレイの「ベースデザイン」一覧 |
| **③ 独立カスタムオーバーレイ** (`custom_overlays`) | HTML / CSS / JS | ゼロから作る完全自由な OBS オーバーレイ（ハイプメーター、ミニゲーム、RPG 風ステータス表示など）。Stosup の全リアルタイムイベントを購読可能。 | MOD 管理画面・オーバーレイスタジオ画面から OBS URL (`/overlay/mod/{mod_id}/{overlay_id}`) をコピー |
| **④ ロジックスクリプト** (`logic_scripts`) | **Rhai (`.rhai`)** | Stosup 本体の安全なサンドボックス上で動作するイベント処理スクリプト。**コンボ集計・状態管理・金額／権限別分岐・動的演出発火・スポットテロップ・OBS シーン／ソース自動操作** を記述可能。 | MOD 管理画面で自動ロード・テスト実行・実行ログ確認（詳細は [MOD_SCRIPT_GUIDE.md](./MOD_SCRIPT_GUIDE.md)） |

---

## 2. クイックスタート（開発の進め方）

### ステップ 1: MOD 配置フォルダを開く
1. Stosup を起動し、左メニューから **「MOD 管理（拡張演出）」** を開きます。
2. 右上の **「MOD フォルダを開く」** ボタンをクリックします（既定の配置先は `%APPDATA%\Stosup\mods\` です）。

### ステップ 2: MOD フォルダと `mod.json` を作成する
`mods` フォルダ直下に任意のフォルダ（例: `my-first-mod`）を作り、`mod.json` と演出ファイルを配置します。

```text
%APPDATA%\Stosup\mods\my-first-mod\
├── mod.json        ← 必須マニフェスト
├── effect.js       ← 例: リアクション演出スクリプト
├── effect.css      ← 任意: 演出用スタイル
└── assets\         ← 任意: 画像・効果音・フォント等
```

### ステップ 3: 再読み込みして本番非干渉プレビューで確認する
1. Stosup の **「MOD 管理」** で **「再読み込み」** をクリックします。
   - `mod.json` に構文ミスや参照ファイル漏れがある場合は、該当カードに赤枠で具体的なエラー内容が表示されます。
2. 動作確認には本番データへ一切影響しない **プレビュー環境** を使えます：
   - リアクション演出: MOD カード内の **「プレビュー再生」** ボタン（`/overlay/preview/media` へ発火）
   - オーバーレイテーマ: オーバーレイスタジオでテーマを選択して「保存して即時反映」→「ブラウザで実寸プレビュー」
   - 独自オーバーレイ: MOD カード内の **「プレビューを開く」** ボタン（`/overlay/preview/mod/{mod_id}/{overlay_id}`）
3. ファイルを書き換えた後は、**「再読み込み」** を押すとキャッシュバスター（`?v=...`）が更新され、接続中の OBS / ブラウザへ即座に反映されます。

---

## 3. `mod.json` マニフェスト仕様（schema_version: 1）

`mod.json` は UTF-8（BOM なし推奨）の JSON ファイルとして MOD フォルダ直下に配置します。

### 3.1 トップレベルフィールド

```json
{
  "schema_version": 1,
  "id": "starlight-burst-effect",
  "name": "スターライト・バースト演出",
  "version": "1.0.0",
  "author": "あなたの名前",
  "description": "コメントやスパチャに反応して流星群と光の輪を画面に広げるリアクション演出です。",
  "min_stosup_version": "0.6.0",
  "homepage": "https://example.com/my-mod",
  "license": "MIT",
  "icon": "icon.png",
  "reaction_effects": [],
  "overlay_themes": [],
  "custom_overlays": [],
  "logic_scripts": []
}
```

| フィールド | 必須 | 説明 |
|---|---|---|
| `schema_version` | 推奨 (`1`) | マニフェスト形式バージョン。現在は `1` を指定します。 |
| `id` | **必須** | 一意の MOD ID。**2〜64 文字の半角英小文字・数字・`-`・`_`**（先頭は英小文字または数字）。例: `my-awesome-fx` |
| `name` | **必須** | アプリ画面に表示される MOD 名称。 |
| `version` | 任意 | MOD のバージョン表記（例: `"1.0.0"`）。省略時は `"1.0.0"`。 |
| `author` | 任意 | 制作者名・サークル名。 |
| `description` | 任意 | MOD の簡潔な説明。 |
| `min_stosup_version` | 任意 | 推奨・最低対応 Stosup バージョン表記。 |
| `homepage` | 任意 | 配布ページやドキュメント URL。 |
| `license` | 任意 | MOD のライセンス表記（例: `"MIT"`, `"CC-BY-4.0"`）。 |
| `icon` | 任意 | MOD 一覧に表示するアイコン画像の相対パス（例: `"icon.png"`）。 |
| `reaction_effects` | 条件必須 | リアクション独自演出の配列。 |
| `overlay_themes` | 条件必須 | オーバーレイ追加テーマの配列。 |
| `custom_overlays` | 条件必須 | 独立カスタムオーバーレイの配列。 |
| `logic_scripts` | 条件必須 | Rhai ロジックスクリプト（`.rhai`）の配列。 |

※ `reaction_effects` / `overlay_themes` / `custom_overlays` / `logic_scripts` のうち **いずれか 1 つ以上** の定義が必要です。

---

### 3.2 `reaction_effects`（リアクション独自演出）

```json
{
  "id": "starlight_burst",
  "name": "スターライト・バースト",
  "description": "流星群と光のリングが画面を彩る演出",
  "entry": "effect.js",
  "css": "effect.css",
  "default_duration_ms": 4200,
  "params": [
    {
      "key": "starCount",
      "label": "流星の本数",
      "kind": "number",
      "default_value": 24,
      "min": 6,
      "max": 80,
      "step": 2,
      "description": "1回の発動で流れる星の本数"
    },
    {
      "key": "primaryColor",
      "label": "メインカラー",
      "kind": "color",
      "default_value": "#38BDF8"
    },
    {
      "key": "burstMode",
      "label": "演出パターン",
      "kind": "select",
      "default_value": "meteor",
      "options": ["meteor", "firework", "aurora"]
    },
    {
      "key": "showFlash",
      "label": "フラッシュ演出",
      "kind": "bool",
      "default_value": true
    },
    {
      "key": "bannerText",
      "label": "サブテキスト",
      "kind": "string",
      "default_value": "STARLIGHT!"
    }
  ]
}
```

#### パラメータ `kind` の種類（Stosup GUI で自動生成されます）
- `"string"`: テキスト入力欄
- `"number"`: 数値入力 / `min` と `max` を両方指定するとスライダーを自動表示
- `"color"`: カラープレビュー付き HEX カラー入力（例: `#38BDF8`）
- `"bool"`: ON / OFF スイッチ
- `"select"`: `options` 配列から選ぶドロップダウン

#### `effect.js` の基本テンプレート

```javascript
(function () {
  'use strict';

  window.StosupMod.registerEffect('starlight_burst', function (ctx) {
    const { canvas, ctx: c, width, height } = ctx.createCanvas();
    const duration = ctx.trigger.duration_ms || 4000;
    const count = Number(ctx.params.starCount) || 24;
    const color = ctx.params.primaryColor || '#38BDF8';

    // テロップが設定されていれば Stosup 標準バナーを表示
    if (ctx.trigger.caption) {
      ctx.showCaption();
    }

    let rafId = 0;
    const startTime = performance.now();

    function frame(now) {
      const elapsed = now - startTime;
      const t = Math.min(1, elapsed / duration);
      c.clearRect(0, 0, width, height);

      // ここに Canvas 描画処理を記述
      c.fillStyle = color;
      c.globalAlpha = 1 - t;
      for (let i = 0; i < count; i++) {
        const x = ((i + 0.5) / count) * width;
        const y = t * height;
        c.beginPath();
        c.arc(x, y, 6, 0, Math.PI * 2);
        c.fill();
      }

      if (t < 1) {
        rafId = requestAnimationFrame(frame);
      } else {
        ctx.done();
      }
    }

    // 「全演出クリア」や中断時に必ず rAF・タイマーを停止する
    ctx.onCleanup(function () {
      if (rafId) cancelAnimationFrame(rafId);
    });

    rafId = requestAnimationFrame(frame);
  });
})();
```

---

### 3.3 `overlay_themes`（9 種オーバーレイ追加テーマ）

```json
{
  "id": "retro_arcade",
  "name": "レトロ・アーケード",
  "description": "アーケード筐体風のドット枠とハイコントラスト配色",
  "targets": ["comments", "superchat", "pickup", "counter", "first_time", "poll", "queue", "lottery", "media"],
  "css": "theme.css",
  "script": "theme.js",
  "preview_bg": "#111322",
  "preview_fg": "#F8FAFC",
  "preview_border": "#F43F5E"
}
```

- `targets` に指定できる値: `comments`, `superchat`, `pickup`, `poll`, `queue`, `media`, `lottery`, `counter`, `first_time`（省略時は全 9 オーバーレイ対象）。
- テーマ適用時、対象オーバーレイの `<body>` には以下の属性が付与されます：
  - `data-theme="mod:{mod_id}:{theme_id}"`
  - `data-mod-id="{mod_id}"`
  - `data-mod-theme="{theme_id}"`
- CSS は Stosup 標準のベース CSS より後、かつ GUI ビジュアルスタイル／ユーザーカスタム CSS より前に読み込まれるため、配信者がさらに文字サイズや縁取りを GUI から調整しても自然に両立します。
- 任意で `script` を指定すると、`StosupMod.registerThemeHook(themeId, { mount, update, unmount })` で動的な DOM 装飾や背景パーティクルを追加できます。

---

### 3.4 `custom_overlays`（独立カスタムオーバーレイ）

```json
{
  "id": "hype_meter",
  "name": "ハイプメーター（盛り上がりゲージ）",
  "description": "コメント速度とスパチャに反応してゲージが上昇する独自オーバーレイ",
  "entry": "overlay.html",
  "channels": ["comments", "superchat"],
  "recommended_width": 640,
  "recommended_height": 220
}
```

Stosup は `/overlay/mod/{mod_id}/{overlay_id}`（およびプレビュー用 `/overlay/preview/mod/{mod_id}/{overlay_id}`）を配信する際、HTML の `<head>` へ以下を自動的に注入します（手動で書いても重複しません）：

1. `<base href="/mods/{mod_id}/" />` — MOD 内相対パス（`style.css`, `assets/bg.png` 等）をそのまま利用可能
2. `<script src="/overlay/stosup-overlay.js"></script>`
3. `<script src="/overlay/stosup-mod-sdk.js"></script>`

HTML 内からは以下のように数行で Stosup のリアルタイムイベントを受信できます：

```html
<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8" />
  <title>サンプル独自オーバーレイ</title>
  <link rel="stylesheet" href="overlay.css" />
</head>
<body>
  <div id="count">0</div>
  <script>
    const client = window.StosupMod.connect({ channels: ['comments', 'superchat'] });
    let total = 0;

    client.on('snapshot', (snap) => {
      total = (snap.comments || []).length;
      document.getElementById('count').textContent = String(total);
    });

    client.on('comment', (comment) => {
      total++;
      document.getElementById('count').textContent = String(total);
    });
  </script>
</body>
</html>
```

---

### 3.5 `logic_scripts`（Rhai ロジックスクリプト `.rhai`）

**コンパイル作業やビルド環境なしで**、コメント・スパチャ・初見・抽選イベントに応じた裏側の条件判定・コンボ状態管理・動的演出発火・スポットテロップ・OBS 自動操作を記述できます。

```json
{
  "id": "combo_controller",
  "name": "コンボ＆スパチャ自動演出ロジック",
  "description": "15秒以内の連続コメントを集計し、コンボ数連動の演出を発火します",
  "entry": "logic.rhai",
  "events": ["comment", "superchat", "first_time", "lottery"]
}
```

```rhai
fn on_comment(ctx) {
    if contains_any(ctx.message, ["草", "www"]) {
        let combo = state_inc("kusa_combo", 1);
        if combo >= 3 {
            trigger_mod_effect("combo_burst", #{ combo: combo }, `${combo} コンボ達成！`);
            state_set("kusa_combo", 0);
        }
    }
}
```

- `logic_scripts` は Stosup 本体の安全なサンドボックス上で動作し、ファイルシステムや OS・認証トークンへは一切アクセスできません。無限ループも最大操作数上限で自動停止します。
- 構文・全フック関数・組み込み API 関数・実践レシピの詳細は **[ロジックスクリプト開発ガイド（MOD_SCRIPT_GUIDE.md）](./MOD_SCRIPT_GUIDE.md)** をご覧ください。

---

## 4. パッケージングと配布（`.stosupmod` / `.zip`）

完成した MOD を他の配信者へ配布するには、MOD フォルダを ZIP 圧縮します。拡張子は **`.zip`** のままでも、Stosup 専用拡張子 **`.stosupmod`** に変更しても導入できます。

### 対応する ZIP 構造（どちらも自動認識）
- **パターン A（ルート直下に `mod.json`）**:
  ```text
  my-mod.stosupmod
  ├── mod.json
  ├── effect.js
  └── assets/...
  ```
- **パターン B（1 階層フォルダ入り / GitHub の Download ZIP 等）**:
  ```text
  my-mod.zip
  └── my-mod-main/
      ├── mod.json
      ├── effect.js
      └── assets/...
  ```

### パッケージ検証・セキュリティ制限
Stosup は導入時に以下を自動検証します：
- **実行可能ファイル禁止**: `.exe`, `.dll`, `.bat`, `.cmd`, `.ps1`, `.vbs`, `.msi`, `.sh`, `.jar` 等を含むアーカイブは拒否されます。
- **サイズ・件数上限**: 展開後合計 128 MB 以下、ファイル数 1,000 件以下。
- **安全な相対パスのみ許可**: `..` を含むパス、絶対パス、`.` で始まる隠しファイル（`.git` 等）は配信・展開されません。
- **外部リソース利用**: Google Fonts や CDN ライブラリ（Three.js, GSAP 等）の読み込みは制作の自由度を優先し許可されていますが、オフライン環境や配信中の安定性を高めるため、可能な限り必要なライブラリ・素材は MOD フォルダ内に同梱することを推奨します。

### 配布前チェックリスト
- [ ] `mod.json` の `id` が他の MOD と衝突しにくい固有の名前になっている
- [ ] `version` / `author` / `description` / `license` を記載した
- [ ] すべての画像・音声・フォント・ライブラリに再配布可能な権利がある
- [ ] リアクション演出で `ctx.onCleanup(...)` に `cancelAnimationFrame` / `clearTimeout` / `clearInterval` を登録し、終了時に `ctx.done()` を呼んでいる
- [ ] オーバーレイテーマの `unmount` で追加した DOM やタイマーを確実に片付けている
- [ ] 本番非干渉プレビュー環境（`/overlay/preview/...`）でエラーなく動作することを確認した
- [ ] Stosup の利用規約（[License.md](../License.md)）の MOD 制作・配布条項を確認した

---

## 5. トラブルシューティング

- **MOD 管理画面に赤いエラーが表示される**:
  - `mod.json` の末尾カンマや括弧閉じ忘れ、あるいは `entry` / `css` / `icon` に記載した相対パスのスペルミスを確認してください。
- **リアクション演出が動かない**:
  - `effect.js` 内の `StosupMod.registerEffect('<id>', ...)` の第 1 引数が `mod.json` の `reaction_effects[].id` と完全一致しているか確認してください。
  - ブラウザで `/overlay/preview/media` を開き、開発者ツール（F12 → Console）で JS エラーを確認できます。
- **ファイルを編集したのに反映されない**:
  - Stosup の **「MOD 管理」→「再読み込み」** を押してください。接続中の全オーバーレイへ新しいリビジョン番号が送信され、`.rhai` ロジックスクリプトも即座に再コンパイルされます。独自オーバーレイ HTML 本体を変更した場合は OBS ソースの「現在のページのキャッシュを更新」またはブラウザ再読み込みを行ってください。
- **ロジックスクリプト（`.rhai`）が動かない・エラーが出る**:
  - 構文エラーがある場合は MOD カードに赤枠で行・列番号が表示されます。
  - 実行時の動作や `log(...)` の出力、発火したアクションは MOD 管理画面下部の **「スクリプト実行ログ（デバッグコンソール）」** で確認できます。各スクリプト行の **「テスト実行」** ボタンから本番に影響せず動作確認できます。
