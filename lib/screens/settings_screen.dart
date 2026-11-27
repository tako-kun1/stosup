import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/license_agreement_screen.dart';
import '../widgets/qr_code_view.dart';
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
  late TextEditingController _pinCtrl;
  String _lastSyncedPin = '';
  bool _obscureSecret = true;

  @override
  void initState() {
    super.initState();
    _clientIdCtrl =
        TextEditingController(text: widget.controller.oauthStatus.clientId);
    _lastSyncedPin = widget.controller.remoteAccessStatus.config.pinCode;
    _pinCtrl = TextEditingController(text: _lastSyncedPin);
  }

  @override
  void dispose() {
    _clientIdCtrl.dispose();
    _clientSecretCtrl.dispose();
    _pinCtrl.dispose();
    super.dispose();
  }

  void _copyText(String text, String label) {
    Clipboard.setData(ClipboardData(text: text));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$label をコピーしました')),
    );
  }

  Future<void> _updateRemoteConfig(
    RemoteAccessConfig next, {
    String? notice,
  }) async {
    await widget.controller.saveRemoteAccessConfig(next, notice: notice);
    if (mounted) {
      _pinCtrl.text = widget.controller.remoteAccessStatus.config.pinCode;
    }
  }

  Future<void> _openSaveProfileDialog(StosupController ctrl) async {
    final nameCtrl = TextEditingController();
    final descCtrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('現在の設定を配信プロファイルとして保存'),
        content: SizedBox(
          width: 440,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: nameCtrl,
                decoration: const InputDecoration(
                  labelText: 'プロファイル名',
                  hintText: '例: 雑談枠 / Apex参加型 / 歌枠',
                ),
              ),
              const SizedBox(height: 10),
              TextField(
                controller: descCtrl,
                decoration: const InputDecoration(
                  labelText: 'メモ（任意）',
                  hintText: '例: クリーン・マガジン配色、エントリーレーン OFF',
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    if (ok == true && nameCtrl.text.trim().isNotEmpty) {
      await ctrl.saveCurrentAsProfile(
        name: nameCtrl.text.trim(),
        description: descCtrl.text.trim(),
      );
    }
    nameCtrl.dispose();
    descCtrl.dispose();
  }

  Widget _buildProfilesAndBackupCard(
    BuildContext context,
    StosupController ctrl,
    StosupPalette p,
  ) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.layers_outlined, color: p.accent, size: 20),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    '配信プロファイル切替 ＆ バックアップ / PC 引っ越し',
                    style:
                        TextStyle(fontSize: 15.5, fontWeight: FontWeight.bold),
                  ),
                ),
                FilledButton.icon(
                  onPressed: () => _openSaveProfileDialog(ctrl),
                  icon: const Icon(Icons.add, size: 16),
                  label: const Text('現在の設定を保存'),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Text(
              '企画ごとに「機能 ON/OFF・オーバーレイデザイン・ルール有効状態」を丸ごと保存し、サイドバーやここから 1 クリックで切り替えられます。',
              style: TextStyle(fontSize: 12.5, color: p.textSecondary),
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Text(
                  'テンプレート作成:',
                  style: TextStyle(
                      fontSize: 12,
                      color: p.textSecondary,
                      fontWeight: FontWeight.w600),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('標準フルセット'),
                  onPressed: () =>
                      ctrl.createTemplateProfile('standard', '標準フルセット'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('雑談・ラジオ枠'),
                  onPressed: () =>
                      ctrl.createTemplateProfile('talk', '雑談・ラジオ枠'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('ゲーム参加型枠'),
                  onPressed: () =>
                      ctrl.createTemplateProfile('game_queue', 'ゲーム参加型枠'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('歌枠・バラエティ'),
                  onPressed: () =>
                      ctrl.createTemplateProfile('singing', '歌枠・バラエティ'),
                ),
              ],
            ),
            const SizedBox(height: 10),
            if (ctrl.streamProfiles.isEmpty)
              Text(
                'プロファイルはまだありません。「現在の設定を保存」または上のテンプレートから作成できます。',
                style: TextStyle(fontSize: 12.5, color: p.textMuted),
              )
            else
              ...ctrl.streamProfiles.map(
                (pr) => ListTile(
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  leading: Icon(
                    pr.isActive
                        ? Icons.check_circle
                        : Icons.radio_button_unchecked,
                    color: pr.isActive ? p.success : p.textMuted,
                    size: 19,
                  ),
                  title: Text(
                    pr.name,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  subtitle: pr.description.isEmpty
                      ? null
                      : Text(
                          pr.description,
                          style:
                              TextStyle(fontSize: 11.5, color: p.textSecondary),
                        ),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (!pr.isActive)
                        FilledButton.tonal(
                          onPressed: () => ctrl.applyStreamProfile(pr.id),
                          child: const Text('切り替え'),
                        )
                      else
                        Chip(
                          visualDensity: VisualDensity.compact,
                          label: Text('適用中',
                              style:
                                  TextStyle(fontSize: 11, color: p.success)),
                        ),
                      const SizedBox(width: 4),
                      IconButton(
                        tooltip: '現在の設定でこのプロファイルを上書き',
                        icon: Icon(Icons.save_outlined,
                            size: 18, color: p.textSecondary),
                        onPressed: () => ctrl.saveCurrentAsProfile(
                          id: pr.id,
                          name: pr.name,
                          description: pr.description,
                        ),
                      ),
                      IconButton(
                        tooltip: 'プロファイルを削除',
                        icon: Icon(Icons.delete_outline,
                            size: 18, color: p.danger),
                        onPressed: () => ctrl.deleteStreamProfile(pr.id),
                      ),
                    ],
                  ),
                ),
              ),
            const Divider(height: 22),
            Row(
              children: [
                Icon(Icons.archive_outlined, color: p.accent, size: 19),
                const SizedBox(width: 8),
                const Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'フルバックアップ ＆ PC 引っ越し（ZIP 書き出し / 復元）',
                        style: TextStyle(
                            fontSize: 13.5, fontWeight: FontWeight.bold),
                      ),
                      Text(
                        '設定・リスナー名簿・全ルール・リアクション素材ファイルを 1 つの ZIP にまとめて保存／復元できます（復元時は現在のデータを自動退避）。',
                        style: TextStyle(fontSize: 11.5),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 10,
              runSpacing: 8,
              children: [
                FilledButton.icon(
                  onPressed: ctrl.exportBackupWithDialog,
                  icon: const Icon(Icons.download_outlined, size: 16),
                  label: const Text('ZIP バックアップを書き出す'),
                ),
                OutlinedButton.icon(
                  onPressed: () async {
                    final ok = await showDialog<bool>(
                      context: context,
                      builder: (ctx) => AlertDialog(
                        title: const Text('バックアップ ZIP から復元しますか？'),
                        content: const Text(
                          '現在のデータベースと設定をバックアップの内容で置き換えます。\n復元前に現在のデータは自動バックアップとして安全に退避されます。',
                        ),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(ctx, false),
                            child: const Text('キャンセル'),
                          ),
                          FilledButton(
                            onPressed: () => Navigator.pop(ctx, true),
                            child: const Text('ファイルを選択して復元'),
                          ),
                        ],
                      ),
                    );
                    if (ok == true) {
                      await ctrl.importBackupWithDialog();
                    }
                  },
                  icon: const Icon(Icons.upload_file_outlined, size: 16),
                  label: const Text('ZIP から復元（引っ越し）'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
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

  Future<void> _showLicenseViewer() async {
    if (widget.controller.licenseStatus == null) {
      await widget.controller.refreshLicenseStatus(checkOnline: false);
    }
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      builder: (_) => LicenseViewerDialog(controller: widget.controller),
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

          // ローカルネットワーク (LAN) 接続 ＆ Web リモコン設定カード
          _buildRemoteAccessCard(context, ctrl, p),
          const SizedBox(height: 14),

          // 配信プロファイル切替 ＆ バックアップ / 引っ越しカード
          _buildProfilesAndBackupCard(context, ctrl, p),
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
                    title: const Text('トークモニター（コメント表示）'),
                    subtitle: const Text('アプリ内コメント一覧および OBS トークモニターへの反映'),
                    value: flags.commentDisplayEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      commentDisplay: v,
                      featureLabel: 'トークモニター',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.palette_outlined, size: 20),
                    title: const Text('ハイライトサイン（吹き出し色反応）'),
                    subtitle: const Text('指定ワードを含むコメントの吹き出し背景色・文字色の変更'),
                    value: flags.wordHighlightEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      wordHighlight: v,
                      featureLabel: 'ハイライトサイン',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.pin_outlined, size: 20),
                    title: const Text('コールカウンター（特定ワード集計）'),
                    subtitle: const Text('アカウント毎上限あり / 上限なしの自動ワードカウント'),
                    value: flags.wordCounterEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      wordCounter: v,
                      featureLabel: 'コールカウンター',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.people_alt_outlined, size: 20),
                    title: const Text('ドローボックス：ユーザー抽選'),
                    subtitle: const Text('リスナー名簿からの抽選（1度のみ / 複数回）'),
                    value: flags.userLotteryEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      userLottery: v,
                      featureLabel: 'ドローボックス（ユーザー抽選）',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary:
                        const Icon(Icons.mark_chat_unread_outlined, size: 20),
                    title: const Text('ドローボックス：コメント抽選'),
                    subtitle: const Text('開始〜終了コメント間の特定ワード収集と抽選'),
                    value: flags.commentLotteryEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      commentLottery: v,
                      featureLabel: 'ドローボックス（コメント抽選）',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.people_outline, size: 20),
                    title: const Text('リスナー名簿（自動記録）'),
                    subtitle: const Text('1配信1コメントで視聴回数+1・累計コメント数の自動記録'),
                    value: flags.userTrackingEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      userTracking: v,
                      featureLabel: 'リスナー名簿（自動記録）',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.note_alt_outlined, size: 20),
                    title: const Text('名前メモ表示（トークモニター上）'),
                    subtitle: const Text('リスナー名簿のメモをコメント欄の名前の横に表示'),
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
                    title: const Text('ウェルカムカウンター（初見さん検知）'),
                    subtitle: const Text('未登録ユーザーまたは初見ワード発言を1アカウント1度だけ検知'),
                    value: flags.firstTimeCounterEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      firstTimeCounter: v,
                      featureLabel: 'ウェルカムカウンター',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary:
                        const Icon(Icons.volunteer_activism_outlined, size: 20),
                    title: const Text('エールボックス（スパチャ・メンバー管理）'),
                    subtitle:
                        const Text('スーパーチャット・メンバー加入の専用一覧と読み忘れチェック管理'),
                    value: flags.superchatEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      superchat: v,
                      featureLabel: 'エールボックス',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.poll_outlined, size: 20),
                    title: const Text('ライブアンケート（リアルタイム投票）'),
                    subtitle: const Text('コメントからリアルタイム投票を集計し OBS グラフ表示'),
                    value: flags.pollEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      poll: v,
                      featureLabel: 'ライブアンケート',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.push_pin_outlined, size: 20),
                    title: const Text('スポットテロップ（ピン留め表示）'),
                    subtitle: const Text('選択したコメントを大きく OBS オーバーレイへピン留め表示'),
                    value: flags.pickupEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      pickup: v,
                      featureLabel: 'スポットテロップ',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.queue_play_next, size: 20),
                    title: const Text('エントリーレーン（参加型順番待ち）'),
                    subtitle: const Text('参加希望・辞退コメントからの自動待機列管理と交代・優先枠'),
                    value: flags.queueEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      queue: v,
                      featureLabel: 'エントリーレーン',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.movie_filter_outlined, size: 20),
                    title: const Text('リアクションステージ（アニメーション・動画演出）'),
                    subtitle:
                        const Text('指定ワードやスパチャ等に反応してパーティクル・画像・動画・効果音を再生'),
                    value: flags.mediaReactionEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      mediaReaction: v,
                      featureLabel: 'リアクションステージ',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.shield_outlined, size: 20),
                    title: const Text('NG ワード防御（荒らし・ネタバレ自動遮断）'),
                    subtitle:
                        const Text('指定ワードの完全非表示・OBS のみ非表示・伏せ字マスク置換'),
                    value: flags.ngWordFilterEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      ngWordFilter: v,
                      featureLabel: 'NGワード防御',
                    ),
                  ),
                  const Divider(height: 1),
                  SwitchListTile(
                    dense: true,
                    secondary: const Icon(Icons.brush_outlined, size: 20),
                    title: const Text('オーバーレイスタジオ（OBS サーバー）'),
                    subtitle: const Text('OBS ブラウザソース用の HTTP / WebSocket 配信稼働'),
                    value: flags.obsOverlayEnabled,
                    onChanged: (v) => ctrl.setFeatureEnabled(
                      obsOverlay: v,
                      featureLabel: 'オーバーレイスタジオ（OBS サーバー）',
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
                        onPressed: _showLicenseViewer,
                        icon: const Icon(Icons.gavel_rounded, size: 17),
                        label: const Text('利用規約を表示'),
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

  Widget _buildRemoteAccessCard(
    BuildContext context,
    StosupController ctrl,
    StosupPalette p,
  ) {
    final status = ctrl.remoteAccessStatus;
    final cfg = status.config;
    if (cfg.pinCode != _lastSyncedPin) {
      _lastSyncedPin = cfg.pinCode;
      _pinCtrl.text = cfg.pinCode;
    }
    final firewallCmd =
        'New-NetFirewallRule -DisplayName "Stosup LAN Remote" -Direction Inbound -Protocol TCP -LocalPort ${status.port} -Action Allow';

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.devices_outlined, color: p.accent, size: 20),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    'ポケットスイッチャー（LAN 接続 ＆ スマホ・別端末リモコン）',
                    style:
                        TextStyle(fontSize: 15.5, fontWeight: FontWeight.bold),
                  ),
                ),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: (cfg.allowLanAccess && status.serverRunning
                            ? p.success
                            : p.textMuted)
                        .withValues(alpha: 0.16),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    !status.serverRunning
                        ? 'サーバー停止中'
                        : (cfg.allowLanAccess
                            ? 'LAN 公開中 (${status.boundAddress})'
                            : '同一 PC のみ (${status.boundAddress})'),
                    style: TextStyle(
                      fontSize: 11.5,
                      fontWeight: FontWeight.bold,
                      color: cfg.allowLanAccess && status.serverRunning
                          ? p.success
                          : p.textSecondary,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Text(
              '同じ Wi-Fi / ローカルネットワーク内のスマホ・タブレット・サブ PC のブラウザから、スポットテロップ・エールボックス・ライブアンケート・エントリーレーン・リアクションステージ・ドローボックスなどをリアルタイム操作できます（外部インターネット IP からの接続は常にブロックされます）。',
              style: TextStyle(fontSize: 12.5, color: p.textSecondary),
            ),
            const SizedBox(height: 10),
            const Divider(height: 1),
            SwitchListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              secondary: const Icon(Icons.wifi, size: 20),
              title: const Text('ローカルネットワーク内の他端末からの接続を許可する'),
              subtitle: const Text(
                'ON にすると LAN 内のスマホ・タブレットや 2 PC 配信の別 PC から 192.168.x.x 経由でアクセスできます（PIN 認証で保護）',
              ),
              value: cfg.allowLanAccess,
              onChanged: (v) => _updateRemoteConfig(
                RemoteAccessConfig(
                  allowLanAccess: v,
                  remoteControlEnabled: cfg.remoteControlEnabled,
                  requirePin: cfg.requirePin,
                  pinCode: cfg.pinCode,
                  embedPinInQr: cfg.embedPinInQr,
                ),
              ),
            ),
            const Divider(height: 1),
            SwitchListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              secondary: const Icon(Icons.touch_app_outlined, size: 20),
              title: const Text('ポケットスイッチャー操作画面 (/remote) を有効にする'),
              subtitle: const Text(
                'OFF にすると Web 操作を遮断し、オーバーレイスタジオの表示のみを許可します',
              ),
              value: cfg.remoteControlEnabled,
              onChanged: (v) => _updateRemoteConfig(
                RemoteAccessConfig(
                  allowLanAccess: cfg.allowLanAccess,
                  remoteControlEnabled: v,
                  requirePin: cfg.requirePin,
                  pinCode: cfg.pinCode,
                  embedPinInQr: cfg.embedPinInQr,
                ),
                notice: v
                    ? 'ポケットスイッチャー操作画面を有効にしました'
                    : 'ポケットスイッチャー操作画面を無効にしました',
              ),
            ),
            const Divider(height: 1),
            SwitchListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              secondary: const Icon(Icons.lock_outline, size: 20),
              title: const Text('アクセス PIN コード認証を必須にする（推奨）'),
              subtitle: const Text(
                '同じ LAN 内の意図しない端末から誤操作されるのを防ぎます',
              ),
              value: cfg.requirePin,
              onChanged: (v) => _updateRemoteConfig(
                RemoteAccessConfig(
                  allowLanAccess: cfg.allowLanAccess,
                  remoteControlEnabled: cfg.remoteControlEnabled,
                  requirePin: v,
                  pinCode: cfg.pinCode,
                  embedPinInQr: cfg.embedPinInQr,
                ),
                notice: v ? 'アクセス PIN 認証を有効にしました' : 'アクセス PIN 認証を無効にしました',
              ),
            ),
            if (cfg.requirePin) ...[
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  SizedBox(
                    width: 210,
                    child: TextField(
                      controller: _pinCtrl,
                      decoration: const InputDecoration(
                        labelText: 'アクセス PIN コード',
                        hintText: '6桁の数字など',
                      ),
                    ),
                  ),
                  OutlinedButton.icon(
                    onPressed: () => _updateRemoteConfig(
                      RemoteAccessConfig(
                        allowLanAccess: cfg.allowLanAccess,
                        remoteControlEnabled: cfg.remoteControlEnabled,
                        requirePin: cfg.requirePin,
                        pinCode: _pinCtrl.text.trim(),
                        embedPinInQr: cfg.embedPinInQr,
                      ),
                      notice: 'アクセス PIN を更新しました',
                    ),
                    icon: const Icon(Icons.save_outlined, size: 16),
                    label: const Text('PIN を保存'),
                  ),
                  OutlinedButton.icon(
                    onPressed: () async {
                      await ctrl.regenerateRemotePin();
                      if (mounted) {
                        _lastSyncedPin =
                            ctrl.remoteAccessStatus.config.pinCode;
                        _pinCtrl.text = _lastSyncedPin;
                      }
                    },
                    icon: const Icon(Icons.casino_outlined, size: 16),
                    label: const Text('PIN 再生成'),
                  ),
                  SizedBox(
                    width: 340,
                    child: CheckboxListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      controlAffinity: ListTileControlAffinity.leading,
                      value: cfg.embedPinInQr,
                      onChanged: (v) => _updateRemoteConfig(
                        RemoteAccessConfig(
                          allowLanAccess: cfg.allowLanAccess,
                          remoteControlEnabled: cfg.remoteControlEnabled,
                          requirePin: cfg.requirePin,
                          pinCode: cfg.pinCode,
                          embedPinInQr: v ?? true,
                        ),
                        notice: 'QR コード設定を更新しました',
                      ),
                      title: const Text(
                        'QR コードに PIN を含める（スキャンで即ログイン）',
                        style: TextStyle(fontSize: 12.5),
                      ),
                    ),
                  ),
                ],
              ),
            ],
            const SizedBox(height: 14),

            // 接続 URL ＆ QR コード表示パネル
            Container(
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: p.subtleBg.withValues(alpha: 0.55),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: p.border),
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (status.qrCode != null) ...[
                    Column(
                      children: [
                        QrCodeView(matrix: status.qrCode!, size: 146),
                        const SizedBox(height: 6),
                        Text(
                          status.lanUrls.isNotEmpty ? 'スマホで読み取り' : 'LAN IP 未検出',
                          style:
                              TextStyle(fontSize: 11, color: p.textSecondary),
                        ),
                      ],
                    ),
                    const SizedBox(width: 16),
                  ],
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            const Text(
                              'ポケットスイッチャー接続 URL（LAN IP）',
                              style: TextStyle(
                                  fontWeight: FontWeight.bold, fontSize: 13.5),
                            ),
                            const Spacer(),
                            TextButton.icon(
                              onPressed: ctrl.refreshRemoteAccessStatus,
                              icon: const Icon(Icons.refresh, size: 15),
                              label: const Text('IP を再検出'),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        if (status.lanUrls.isNotEmpty) ...[
                          ...status.lanUrls.map(
                            (url) => Padding(
                              padding: const EdgeInsets.only(bottom: 6),
                              child: Row(
                                children: [
                                  Expanded(
                                    child: SelectableText(
                                      url,
                                      style: TextStyle(
                                        fontFamily: 'Consolas',
                                        color: p.accent,
                                        fontSize: 13.5,
                                        fontWeight: FontWeight.w600,
                                      ),
                                    ),
                                  ),
                                  const SizedBox(width: 8),
                                  OutlinedButton.icon(
                                    style: OutlinedButton.styleFrom(
                                      visualDensity: VisualDensity.compact,
                                    ),
                                    onPressed: () =>
                                        _copyText(url, 'ポケットスイッチャー URL'),
                                    icon: const Icon(Icons.copy, size: 14),
                                    label: const Text('コピー'),
                                  ),
                                ],
                              ),
                            ),
                          ),
                          if (!cfg.allowLanAccess) ...[
                            Padding(
                              padding: const EdgeInsets.only(bottom: 8),
                              child: Text(
                                '※現在「他端末からの接続を許可」が OFF です。LAN IP（${status.localIps.isNotEmpty ? status.localIps.first : status.lanUrls.first}）で接続するには ON にしてください。',
                                style:
                                    TextStyle(fontSize: 12, color: p.warning),
                              ),
                            ),
                          ],
                          const SizedBox(height: 2),
                          Wrap(
                            spacing: 8,
                            runSpacing: 8,
                            children: [
                              if (!cfg.allowLanAccess)
                                FilledButton.icon(
                                  style: FilledButton.styleFrom(
                                    visualDensity: VisualDensity.compact,
                                  ),
                                  onPressed: () => _updateRemoteConfig(
                                    RemoteAccessConfig(
                                      allowLanAccess: true,
                                      remoteControlEnabled:
                                          cfg.remoteControlEnabled,
                                      requirePin: cfg.requirePin,
                                      pinCode: cfg.pinCode,
                                      embedPinInQr: cfg.embedPinInQr,
                                    ),
                                  ),
                                  icon: const Icon(Icons.wifi, size: 14),
                                  label: const Text('LAN 接続を ON にする'),
                                ),
                              FilledButton.icon(
                                style: FilledButton.styleFrom(
                                  visualDensity: VisualDensity.compact,
                                ),
                                onPressed: status.serverRunning &&
                                        cfg.remoteControlEnabled &&
                                        cfg.allowLanAccess
                                    ? () => ctrl.openUrl(status.qrTargetUrl)
                                    : null,
                                icon: const Icon(Icons.open_in_new, size: 14),
                                label: const Text('ブラウザで開く'),
                              ),
                            ],
                          ),
                        ] else ...[
                          Text(
                            'LAN IPv4 アドレス（192.168.x.x 等）を検出できませんでした（Wi-Fi / LAN 接続を確認して「IP を再検出」を押してください）。',
                            style: TextStyle(fontSize: 12, color: p.warning),
                          ),
                          const SizedBox(height: 6),
                          Row(
                            children: [
                              Expanded(
                                child: SelectableText(
                                  status.localUrl,
                                  style: TextStyle(
                                    fontFamily: 'Consolas',
                                    color: p.textSecondary,
                                    fontSize: 12.5,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 8),
                              FilledButton.icon(
                                style: FilledButton.styleFrom(
                                  visualDensity: VisualDensity.compact,
                                ),
                                onPressed: status.serverRunning &&
                                        cfg.remoteControlEnabled
                                    ? () => ctrl.openUrl(status.qrTargetUrl)
                                    : null,
                                icon: const Icon(Icons.open_in_new, size: 14),
                                label: const Text('ブラウザで開く'),
                              ),
                            ],
                          ),
                        ],
                        if (cfg.allowLanAccess) ...[
                          const Divider(height: 18),
                          Row(
                            children: [
                              Icon(Icons.security_outlined,
                                  size: 15, color: p.textSecondary),
                              const SizedBox(width: 6),
                              Expanded(
                                child: Text(
                                  'スマホ等から接続できない場合は Windows ファイアウォールで TCP ${status.port} 番ポートを許可してください。',
                                  style: TextStyle(
                                    fontSize: 11.5,
                                    color: p.textSecondary,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 6),
                              TextButton.icon(
                                onPressed: () => _copyText(
                                  firewallCmd,
                                  'ファイアウォール許可 PowerShell コマンド',
                                ),
                                icon: const Icon(Icons.terminal, size: 14),
                                label: const Text('許可コマンドをコピー'),
                              ),
                            ],
                          ),
                        ],
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
