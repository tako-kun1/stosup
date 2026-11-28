# Stosup MOD API & オーバーレイリファレンス

このドキュメントは、Stosup の演出 MOD から利用できる **JavaScript SDK (`window.StosupMod`)**、**WebSocket リアルタイムイベント仕様**、および **全 9 種 OBS オーバーレイの DOM・CSS セレクタ一覧** をまとめた技術リファレンスです。

> 導入・作成手順の概要は [MOD_DEVELOPMENT.md](./MOD_DEVELOPMENT.md)、Stosup 本体のサンドボックス上で動く **Rhai ロジックスクリプト（`.rhai`）の API 仕様** は [MOD_SCRIPT_GUIDE.md](./MOD_SCRIPT_GUIDE.md) を参照してください。

---

## 1. `window.StosupMod` SDK リファレンス

`/overlay/stosup-mod-sdk.js` により、リアクションステージ（`/overlay/media`）・テーマフック・独立カスタムオーバーレイ（`/overlay/mod/{mod_id}/{overlay_id}`）のすべてでグローバル変数 `window.StosupMod` が利用できます。

### 1.1 プロパティ & 登録 API

#### `StosupMod.sdkVersion: string`
現在の SDK バージョン（例: `"1.0.0"`）。

#### `StosupMod.registerEffect(effectId, handler)`
リアクションステージ用の独自演出ハンドラを登録します。
- `effectId` (`string`): `mod.json` の `reaction_effects[].id` と一致する識別子。
- `handler` (`(ctx: ModEffectContext) => void | Promise<void>`): 演出発動時に呼ばれる関数。

#### `StosupMod.registerThemeHook(themeId, hooks)`
オーバーレイ追加テーマ用の任意 JS フックを登録します。
- `themeId` (`string`): `mod.json` の `overlay_themes[].id` と一致する識別子。
- `hooks` (`object`):
  - `mount?(info)`: テーマ適用直後に 1 回呼ばれます。
  - `update?(info)`: 設定更新・再読み込み時に呼ばれます。
  - `unmount?(info)`: 別テーマへの切替時や MOD 無効化時に呼ばれます。
- `info` オブジェクト: `{ overlayKey: string, modId: string, themeId: string, baseUrl: string }`

#### `StosupMod.connect(options?)`
独立カスタムオーバーレイ向けの WebSocket クライアントを作成します（自動再接続およびプレビューモード自動判定内蔵）。
- `options.channels` (`string[]`, 既定 `['all']`): 購読チャンネル一覧。単一指定（例 `['comments']`）時はそのチャンネル専用で軽量購読、複数指定時は `all` で接続します。
  - 指定可能なチャンネル: `all`, `comments`, `superchat`, `pickup`, `poll`, `queue`, `media`, `lottery`, `counter`, `first_time`
- `options.reconnectDelayMs` (`number`, 既定 `2000`): 切断時の再接続間隔（ミリ秒）。
- **戻り値 (`StosupModClient`)**:
  - `client.on(eventName, callback)` — イベント購読（メソッドチェーン可、`'*'` で全イベント購読）
  - `client.off(eventName, callback)` — 購読解除
  - `client.close()` — 切断（再接続も停止）
  - `client.isPreview: boolean` — `/overlay/preview/mod/...` で開かれている場合 `true`
  - `client.lastSnapshot: object | null` — 最後に受信した初期 `snapshot` ペイロード

---

### 1.2 `StosupMod.utils` ユーティリティ

| メソッド | 説明 |
|---|---|
| `utils.renderMessageRuns(container, fallbackText, runs)` | YouTube カスタム絵文字・メンバースタンプ混在の `message_runs` を XSS 安全に DOM 描画します。 |
| `utils.fallbackAvatar(name)` | アイコン URL が無い視聴者向けの SVG アバター Data URL を返します。 |
| `utils.escapeHtml(text)` | HTML 特殊文字（`& < > " '`）を安全にエスケープします。 |
| `utils.playAudio(url, volume0to100)` | Web Audio API（フォールバック付き）で音声・効果音を再生します。 |
| `utils.assetUrl(modId, relativePath)` | `/mods/{modId}/{relativePath}` 形式の URL を生成します。 |
| `utils.getQueryParams()` | 現在の URL クエリパラメータを `{ key: value }` の連想配列で返します。 |
| `utils.isPreviewMode()` | プレビュー専用 URL で表示中かどうかを返します。 |

---

## 2. リアクション演出 `ModEffectContext` (`ctx`) リファレンス

`StosupMod.registerEffect(effectId, (ctx) => { ... })` の引数 `ctx` には以下のプロパティ・メソッドが渡されます。

| プロパティ / メソッド | 型 | 説明 |
|---|---|---|
| `ctx.stage` | `HTMLDivElement` | この発火専用の全画面オーバーレイ DOM コンテナ（終了時に自動削除）。 |
| `ctx.createCanvas()` | `() => { canvas, ctx, width, height }` | ビューポート等倍（最大 2x DPR）の専用 2D Canvas を `ctx.stage` 内へ生成して返します。 |
| `ctx.trigger` | `MediaTriggerEvent` | 発火元ルール名・発動者名・テロップ・再生時間・素材 URL 等。 |
| `ctx.params` | `Record<string, any>` | `mod.json` の `params` スキーマで型変換・デフォルト補完されたユーザー設定値。 |
| `ctx.width`, `ctx.height` | `number` | 現在のビューポート幅・高さ（px）。 |
| `ctx.mod` | `{ id, effectId, name, version, baseUrl, assetUrl(rel) }` | MOD 自身のメタ情報および同梱アセット URL 解決ヘルパー。 |
| `ctx.audio` | `{ play(url, vol0to100), masterVolume }` | リアクションステージ本体の Web Audio エンジンと連動した音声再生。 |
| `ctx.showCaption(text?)` | `(text?: string) => void` | Stosup 標準テロップバナーを表示します（引数省略時は `ctx.trigger.caption`）。 |
| `ctx.onCleanup(fn)` | `(fn: () => void) => void` | 演出終了・タイムアウト・「全演出クリア」時に必ず実行される後始末コールバックを登録します。 |
| `ctx.done()` | `() => void` | 演出の完了を通知し、DOM 解放およびキュー再生モードの次演出へ進めます。 |

### `ctx.trigger` (`MediaTriggerEvent`) の主なフィールド
```json
{
  "event_id": "uuid",
  "rule_id": "rule-uuid",
  "rule_name": "スパチャ流星演出",
  "effect_type": "Mod",
  "media_url": "/media/xxxx.png",
  "media_kind": "Image",
  "sound_url": "/media/se.mp3",
  "sound_volume": 85,
  "animation": "PopIn",
  "position": "Center",
  "duration_ms": 4200,
  "width_percent": 45,
  "volume": 80,
  "caption": "たこやき太郎 さんありがとう！",
  "triggered_by": "たこやき太郎",
  "triggered_at_ms": 1764300000000,
  "mod_id": "starlight-burst-effect",
  "mod_effect_id": "starlight_burst",
  "mod_params_json": "{\"starCount\":32}"
}
```

---

## 3. WebSocket リアルタイムイベント仕様（`StosupMod.connect`）

独立カスタムオーバーレイから `client.on(eventName, handler)` で受信できるイベント一覧です。

| イベント名 | タイミング | ペイロード概要 |
|---|---|---|
| `open` | WebSocket 接続確立時 | `{ channel, isPreview }` |
| `close` | WebSocket 切断時 | `{}` |
| `snapshot` | 接続直後の初期状態同期 | `{ config, comments, counters, last_lottery, first_time, monetization_items, poll_state, picked_comment, queue_state, media_config, mods, mods_revision, ... }` |
| `comment` | 新規コメント受信時 | `CommentItem`（`id`, `user_id`, `user_name`, `user_icon_url`, `user_memo`, `message`, `message_runs`, `is_first_time`, `is_owner`, `is_moderator`, `is_member`, `event_kind`, `amount_display`, `event_accent_color`, `timestamp_ms` 等） |
| `superchat` | スパチャ・メンバー一覧更新時 | `{ items: MonetizationItem[], enabled: boolean }` |
| `pickup` | スポットテロップ切替・解除時 | `{ picked: { comment: CommentItem \| null, picked_at_ms }, enabled: boolean }` |
| `poll` | ライブアンケート開始・票更新・締切時 | `{ poll: PollState, enabled: boolean }` |
| `clear_poll` | アンケート表示クリア時 | `{}` |
| `queue` | エントリーレーン更新時 | `{ queue: QueueState, enabled: boolean }` |
| `counters` | コールカウンター値更新時 | `WordCounterConfig[]` |
| `first_time` | ウェルカムカウンター更新時 | `{ first_time: FirstTimeCounterState, enabled: boolean }` |
| `lottery_result` | 抽選実行時 | `LotteryResult`（`kind`, `winners`, `candidate_count`, `drawn_at_ms`） |
| `clear_lottery` | 抽選表示クリア時 | `{}` |
| `media_trigger` | リアクション演出発火時 | `MediaTriggerEvent` |
| `media_config` | リアクション全体設定変更時 | `{ media_config, enabled }` |
| `clear_media` | 全演出クリア押下時 | `{}` |
| `config_updated` | オーバーレイ設定・デザイン保存時 | `OverlayConfig` |
| `mods_updated` | MOD の再読み込み・ON/OFF 変更時 | `{ mods: InstalledMod[], revision: number }` |

### 補助 HTTP エンドポイント
- `GET /api/mods/runtime`: 現在有効な MOD 一覧とリビジョン番号を JSON で取得します。
- `GET /mods/{mod_id}/{relative_path}`: 有効な MOD 配下の静的ファイル（CSS / JS / 画像 / 音声 / 動画 / フォント等）を取得します。

---

## 4. 全 9 種 OBS オーバーレイの CSS セレクタ・変数一覧

`overlay_themes` の CSS を作成する際は、`body[data-theme="mod:{mod_id}:{theme_id}"]` 配下にルールを書くと他のテーマと干渉せず安全にスタイリングできます。

### 全オーバーレイ共通の CSS 変数・属性
- `--page-bg`: 背景色（既定 `transparent`）
- `--font-family`: 選択されたシステムフォント
- `--accent`: アクセントカラー
- `body[data-theme="mod:{mod_id}:{theme_id}"]`
- `body[data-mod-id="{mod_id}"][data-mod-theme="{theme_id}"]`
- `body.has-custom-font`: ユーザーがフォントを選択している場合に付与

### 各オーバーレイ固有の主要 DOM セレクタ

| 対象 (`targets` キー) | オーバーレイ名 | 主なセレクタ |
|---|---|---|
| `comments` | トークモニター | `#container`, `.comment-row`, `.avatar`, `.bubble`, `.bubble.monetization`, `.header`, `.author`, `.badge`, `.badge-owner`, `.badge-mod`, `.badge-member`, `.badge-first`, `.badge-monetization`, `.timestamp`, `.message`, `.stosup-emoji` |
| `superchat` | エールボックス | `#list`, `.sc-card`, `.sc-header`, `.sc-avatar`, `.sc-author`, `.sc-amount`, `.sc-badge`, `.sc-message` |
| `pickup` | スポットテロップ | `#stage`, `.pickup-card`, `.pickup-head`, `.pickup-avatar`, `.pickup-author`, `.pickup-badge`, `.pickup-message` |
| `poll` | ライブアンケート | `#poll-box`, `.poll-header`, `.poll-title`, `.poll-status`, `.poll-options`, `.poll-option`, `.poll-bar-track`, `.poll-bar-fill`, `.poll-label`, `.poll-votes`, `.poll-footer` |
| `queue` | エントリーレーン | `#queue-box`, `.queue-header`, `.queue-section-title`, `.queue-item`, `.queue-item.active`, `.queue-item.waiting`, `.queue-rank`, `.queue-avatar`, `.queue-name`, `.queue-meta` |
| `media` | リアクションステージ | `#fx-canvas`, `#stage`, `.media-slot`, `.media-frame`, `.media-el`, `.media-caption` |
| `lottery` | ドローボックス | `#stage`, `.lottery-card`, `.lottery-title`, `.winner-card`, `.winner-avatar`, `.winner-name`, `.winner-comment` |
| `counter` | コールカウンター | `#counters`, `.counter-card`, `.counter-name`, `.counter-value`, `.counter-sub` |
| `first_time` | ウェルカムカウンター | `#box`, `.ft-card`, `.ft-label`, `.ft-count`, `.ft-users`, `.ft-user-row`, `.ft-avatar`, `.ft-name` |

> **デザインのコツ**: GUI ビジュアルスタイルエディタ（文字サイズ・袋文字・エッジ発光・カード背景透明度など）は MOD テーマ CSS の **後から** 適用されるため、テーマ CSS 側では `!important` を多用せず通常の詳細度で指定しておくと、ユーザーが GUI から微調整した際に綺麗に反映されます。
