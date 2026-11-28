# Stosup 演出 MOD サンプル集

このフォルダには、Stosup の 4 種類の演出 MOD をそのまま導入・改造して動かせる実働サンプルを収録しています。

## 収録サンプル一覧

| フォルダ | MOD の種別 | 内容 |
|---|---|---|
| [`starlight-burst-effect/`](./starlight-burst-effect/) | **リアクション独自演出** (`reaction_effects`) | 流星群・花火・オーロラの 3 パターンを切り替えられる 60fps Canvas 演出。全 5 種類のカスタムパラメータ型（数値スライダー・色・選択・ON/OFF・文字）の実装例です。 |
| [`retro-arcade-theme/`](./retro-arcade-theme/) | **オーバーレイ追加テーマ** (`overlay_themes`) | 全 9 種類の OBS オーバーレイをアーケード筐体風デザインにする CSS ＋ 任意 JS フック（`theme.js`）の実装例です。 |
| [`hype-meter-overlay/`](./hype-meter-overlay/) | **独立カスタムオーバーレイ** (`custom_overlays`) | `StosupMod.connect()` でコメント速度・スパチャ・演出発火を受信し、盛り上がりゲージ＆FEVER 演出を表示する独立 OBS 画面の実装例です。 |
| [`combo-fever-script/`](./combo-fever-script/) | **ロジックスクリプト ＋ 演出連携** (`logic_scripts` + `reaction_effects`) | 軽量スクリプト言語 **Rhai (`.rhai`)** で 15 秒コンボ集計・スパチャ金額別分岐・初見歓迎を記述し、自 MOD の専用 Canvas 演出を動的パラメータで自動発火する統合実装例です。 |

## 導入して試す手順

### 方法 A: 開発者向け（フォルダを直接コピーして試す）
1. Stosup の左メニューから **「MOD 管理」** を開き、**「MOD フォルダを開く」** をクリックします。
2. 開いたフォルダ（`%APPDATA%\Stosup\mods\`）へ、試したいサンプルのフォルダ（例: `combo-fever-script`）を丸ごとコピーします。
3. Stosup の **「MOD 管理」** 画面で **「再読み込み」** をクリックすると即座に認識されます。

### 方法 B: パッケージ導入（ZIP / `.stosupmod` として導入する）
1. 任意のサンプルフォルダを ZIP 圧縮します（拡張子は `.zip` のままでも `.stosupmod` に変更しても導入可能です）。
2. Stosup の **「MOD 管理」** → **「MOD を導入 (.stosupmod / .zip)」** から選択すると導入されます。

## 詳しい作り方
- 総合ガイド: [`docs/MOD_DEVELOPMENT.md`](../../docs/MOD_DEVELOPMENT.md)
- ロジックスクリプト（`.rhai`）開発ガイド: [`docs/MOD_SCRIPT_GUIDE.md`](../../docs/MOD_SCRIPT_GUIDE.md)
- API・イベント・DOM リファレンス: [`docs/MOD_API_REFERENCE.md`](../../docs/MOD_API_REFERENCE.md)
