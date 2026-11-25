import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/release_notes_dialog.dart';
import '../widgets/update_dialog.dart';

/// 設定画面（外観テーマ切替 / 各機能 ON・OFF 管理 / YouTube OAuth 連携 / 自動アップデート）
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late TextEditingController _clientIdCtrl;
  final _clientSecretCtrl = TextEditingController();
  bool _obscureSecret = true;

  @override
  void initState() {
    super.initState();
    _clientIdCtrl =
        TextEditingController(text: widget.controller.oauthStatus.clientId);
  }

  @override
  void dispose() {
    _clientIdCtrl.dispose();
    _clientSecretCtrl.dispose();
    super.dispose();
  }

  Future<void> _showCurrentReleaseNotes() async {
    final notes = await widget.controller.fetchCurrentReleaseNotes();
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      builder: (_) => ReleaseNotesDialog(
        info: notes,
        onClose: () {},
      ),
    );
  }

  Future<void> _checkUpdatesNow() async {
    final info = await widget.controller.checkUpdates(silent: false);
    if (!mounted || info == null) return;
    if (info.hasUpdate) {
      await showDialog<void>(
        context: context,
        builder: (_) => UpdateDialog(
          controller: widget.controller,
          info: info,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;
    final p = context.palette;
    final oauth = ctrl.oauthStatus;
    final flags = ctrl.featureFlags;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '設定・外観・機能管理',
            style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 16),

          // 外観テーマ（ライト / ダーク / システム）切替カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.contrast, color: p.accent, size: 20),
                      const SizedBox(width: 8),
                      const Text(
                        '外観テーマ',
                        style: TextStyle(
                            fontSize: 15.5, fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    'ソフトウェア本体の配色をライトモード / ダークモード / Windows のシステム設定から選択できます。',
                    style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                  ),
                  const SizedBox(height: 12),
                  SegmentedButton<AppThemeMode>(
                    segments: const [
                      ButtonSegment(
                        value: AppThemeMode.light,
                        icon: Icon(Icons.light_mode_outlined, size: 17),
                        label: Text('ライトモード'),
                      ),
                      ButtonSegment(
                        value: AppThemeMode.dark,
                        icon: Icon(Icons.dark_mode_outlined, size: 17),
                        label: Text('ダークモード'),
                      ),
                      ButtonSegment(
                        value: AppThemeMode.system,
                        icon: Icon(Icons.desktop_windows_outlined, size: 17),
                        label: Text('システムに従う'),
                      ),
                    ],
                    selected: {ctrl.themeMode},
                    onSelectionChanged: (s) => ctrl.setThemeMode(s.first),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // 各機能の ON/OFF 一括管理カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.tune, color: p.accent, size: 20),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          '各機能の有効 / 無効（ON / OFF 管理）',
                          style: TextStyle(
                              fontSize: 15.5, fontWeight: FontWeight.bold),
                        ),
                      ),
                      OutlinedButton(
                        onPressed: () => ctrl.setAllFeaturesEnabled(true),
                        child: const Text('すべて ON'),
                      ),
                      const SizedBox(width: 8),
                      OutlinedButton(
                        onPressed: () => ctrl.setAllFeaturesEnabled(false),
                        child: const Text('すべて OFF'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    '配信スタイルに合わせて使用する機能だけをオンにできます。各機能画面の右上スイッチとも連動し、自動で保存されます。',
                    style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                  ),
                  const SizedBox(height: 10),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.chat_bubble_outline, size: 20),
                    title: const Text('コメント表示機能'),
                    subtitle: const Text('アプリ内コメント一覧および OBS コメント表示への反映'),
                    value: flags.commentDisplayEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      commentDisplay: v,
                      featureLabel: 'コメント表示機能',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.palette_outlined, size: 20),
                    title: const Text('特定ワード反応（吹き出し色変更）'),
                    subtitle: const Text('指定ワードを含むコメントの吹き出し背景色・文字色の変更'),
                    value: flags.wordHighlightEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      wordHighlight: v,
                      featureLabel: '特定ワード反応（吹き出し色変更）',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.pin_outlined, size: 20),
                    title: const Text('特定ワードカウント機能'),
                    subtitle: const Text('アカウント毎上限あり / 上限なしの自動ワードカウント'),
                    value: flags.wordCounterEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      wordCounter: v,
                      featureLabel: '特定ワードカウント機能',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.people_alt_outlined, size: 20),
                    title: const Text('ユーザー抽選システム'),
                    subtitle: const Text('参加ユーザーリストからの抽選（1度のみ / 複数回）'),
                    value: flags.userLotteryEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      userLottery: v,
                      featureLabel: 'ユーザー抽選システム',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary:
                        const Icon(Icons.mark_chat_unread_outlined, size: 20),
                    title: const Text('コメント抽選システム'),
                    subtitle: const Text('開始〜終了コメント間の特定ワード収集と抽選'),
                    value: flags.commentLotteryEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      commentLottery: v,
                      featureLabel: 'コメント抽選システム',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.people_outline, size: 20),
                    title: const Text('参加ユーザーリスト自動記録'),
                    subtitle: const Text('1配信1コメントで視聴回数+1・累計コメント数の自動記録'),
                    value: flags.userTrackingEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      userTracking: v,
                      featureLabel: 'ユーザーリスト自動記録',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.note_alt_outlined, size: 20),
                    title: const Text('名前メモ表示（コメント欄上）'),
                    subtitle: const Text('ユーザーに設定したメモをコメント欄の名前の横に表示'),
                    value: flags.userMemoDisplayEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      userMemoDisplay: v,
                      featureLabel: '名前メモ表示',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.waving_hand_outlined, size: 20),
                    title: const Text('初見カウンター'),
                    subtitle: const Text('未登録ユーザーまたは初見ワード発言を1アカウント1度だけ検知'),
                    value: flags.firstTimeCounterEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      firstTimeCounter: v,
                      featureLabel: '初見カウンター',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary:
                        const Icon(Icons.volunteer_activism_outlined, size: 20),
                    title: const Text('スパチャ / メンバーシップ管理'),
                    subtitle:
                        const Text('スーパーチャット・メンバー加入の専用一覧と読み忘れチェック管理'),
                    value: flags.superchatEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      superchat: v,
                      featureLabel: 'スパチャ / メンバーシップ管理',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.poll_outlined, size: 20),
                    title: const Text('リアルタイム投票 / アンケート'),
                    subtitle: const Text('コメントからリアルタイム投票を集計し OBS グラフ表示'),
                    value: flags.pollEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      poll: v,
                      featureLabel: 'リアルタイム投票 / アンケート',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.push_pin_outlined, size: 20),
                    title: const Text('コメントピックアップ（ピン留め表示）'),
                    subtitle: const Text('選択したコメントを大きく OBS オーバーレイへピン留め表示'),
                    value: flags.pickupEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      pickup: v,
                      featureLabel: 'コメントピックアップ表示',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.queue_play_next, size: 20),
                    title: const Text('参加型キュー（順番待ちリスト）'),
                    subtitle: const Text('参加希望・辞退コメントからの自動待機列管理と交代・優先枠'),
                    value: flags.queueEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      queue: v,
                      featureLabel: '参加型キュー管理',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.movie_filter_outlined, size: 20),
                    title: const Text('特定ワード反応アニメーション・動画演出'),
                    subtitle:
                        const Text('指定ワード発言時にユーザー登録の画像・GIF・動画を OBS 上で再生'),
                    value: flags.mediaReactionEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      mediaReaction: v,
                      featureLabel: '特定ワード反応アニメーション',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.brush_outlined, size: 20),
                    title: const Text('OBS オーバーレイローカルサーバー'),
                    subtitle: const Text('OBS ブラウザソース用の HTTP / WebSocket サーバー稼働'),
                    value: flags.obsOverlayEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      obsOverlay: v,
                      featureLabel: 'OBS オーバーレイサーバー',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.system_update_alt, size: 20),
                    title: const Text('起動時の自動アップデート確認'),
                    subtitle: const Text('アプリ起動時に GitHub Releases の最新版を自動チェック'),
                    value: flags.autoUpdateCheckEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      autoUpdateCheck: v,
                      featureLabel: '起動時の自動アップデート確認',
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // YouTube OAuth 2.0 連携カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.account_circle_outlined,
                          color: p.danger, size: 21),
                      const SizedBox(width: 8),
                      const Text(
                        'YouTube アカウント連携 (OAuth 2.0)',
                        style: TextStyle(
                            fontSize: 15.5, fontWeight: FontWeight.bold),
                      ),
                      const Spacer(),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 8, vertical: 3),
                        decoration: BoxDecoration(
                          color: (oauth.isAuthenticated ? p.success : p.textMuted)
                              .withValues(alpha: 0.14),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          oauth.isAuthenticated
                              ? '連携済み: ${oauth.accountName ?? "YouTubeアカウント"}'
                              : '未連携',
                          style: TextStyle(
                            color: oauth.isAuthenticated
                                ? p.success
                                : p.textSecondary,
                            fontWeight: FontWeight.w600,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    'コメント抽選の開始・終了コメントを YouTube チャットへ自動投稿する場合や、公式 Data API v3 モードを利用する場合に設定します。',
                    style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _clientIdCtrl,
                    decoration: const InputDecoration(
                      labelText: 'Google OAuth クライアント ID（デスクトップアプリ用）',
                      hintText:
                          'xxxxxxxxxxxx-xxxxxxxxxxxxxxxx.apps.googleusercontent.com',
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: _clientSecretCtrl,
                    obscureText: _obscureSecret,
                    decoration: InputDecoration(
                      labelText: oauth.hasClientSecret
                          ? 'Google OAuth クライアント シークレット（設定済み・変更時のみ入力）'
                          : 'Google OAuth クライアント シークレット',
                      suffixIcon: IconButton(
                        onPressed: () =>
                            setState(() => _obscureSecret = !_obscureSecret),
                        icon: Icon(_obscureSecret
                            ? Icons.visibility_outlined
                            : Icons.visibility_off_outlined),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      OutlinedButton.icon(
                        onPressed: () => ctrl.saveOAuthClient(
                          _clientIdCtrl.text.trim(),
                          _clientSecretCtrl.text.trim(),
                        ),
                        icon: const Icon(Icons.save_outlined, size: 17),
                        label: const Text('クライアント設定を保存'),
                      ),
                      FilledButton.icon(
                        onPressed: ctrl.isOAuthInProgress
                            ? null
                            : () async {
                                if (_clientIdCtrl.text.trim().isNotEmpty) {
                                  await ctrl.saveOAuthClient(
                                    _clientIdCtrl.text.trim(),
                                    _clientSecretCtrl.text.trim(),
                                  );
                                }
                                await ctrl.startOAuthLogin();
                              },
                        icon: ctrl.isOAuthInProgress
                            ? const SizedBox(
                                width: 15,
                                height: 15,
                                child:
                                    CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.login, size: 17),
                        label: Text(ctrl.isOAuthInProgress
                            ? 'ブラウザで認証待機中...'
                            : 'Google アカウントで連携'),
                      ),
                      if (oauth.isAuthenticated)
                        TextButton.icon(
                          style:
                              TextButton.styleFrom(foregroundColor: p.danger),
                          onPressed: ctrl.disconnectOAuth,
                          icon: const Icon(Icons.logout, size: 17),
                          label: const Text('連携を解除'),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // ソフトウェア情報カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.info_outline, color: p.accent, size: 20),
                      const SizedBox(width: 8),
                      const Text(
                        'ソフトウェア情報',
                        style: TextStyle(
                            fontSize: 15.5, fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Text(
                    'Stosup：v${ctrl.appVersion}',
                    style: TextStyle(
                      fontSize: 17,
                      fontWeight: FontWeight.bold,
                      color: p.textPrimary,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      FilledButton.icon(
                        onPressed:
                            ctrl.isCheckingUpdate ? null : _checkUpdatesNow,
                        icon: ctrl.isCheckingUpdate
                            ? const SizedBox(
                                width: 15,
                                height: 15,
                                child:
                                    CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.refresh, size: 17),
                        label: const Text('アップデートチェック'),
                      ),
                      OutlinedButton.icon(
                        onPressed: _showCurrentReleaseNotes,
                        icon: const Icon(Icons.article_outlined, size: 17),
                        label: const Text('リリースノートを表示'),
                      ),
                      OutlinedButton.icon(
                        onPressed: () => ctrl
                            .openUrl('https://github.com/tako-kun1/stosup'),
                        icon: const Icon(Icons.open_in_new, size: 17),
                        label: const Text('GitHub リポジトリを開く'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // 対応プラットフォーム状況カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '対応配信プラットフォーム',
                    style:
                        TextStyle(fontSize: 15, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      Chip(
                        avatar: Icon(Icons.check_circle,
                            color: p.success, size: 16),
                        label: const Text('YouTube Live（対応済み）'),
                      ),
                      Chip(
                        avatar:
                            Icon(Icons.science, color: p.accent, size: 16),
                        label: const Text('デモ配信モード（対応済み）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: p.textMuted, size: 16),
                        label: const Text('Twitch（今後対応予定）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: p.textMuted, size: 16),
                        label: const Text('ニコニコ生放送（今後対応予定）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: p.textMuted, size: 16),
                        label: const Text('ツイキャス（今後対応予定）'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
