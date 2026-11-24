import 'package:material_ui/material_ui.dart';

import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/release_notes_dialog.dart';
import '../widgets/update_dialog.dart';

/// 設定画面（YouTube OAuth 2.0 連携 / GitHub 自動アップデート / リリースノート）
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
    final oauth = ctrl.oauthStatus;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '設定・連携・アップデート',
            style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 16),

          // YouTube OAuth 2.0 連携カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.account_circle,
                          color: AppTheme.accentRose, size: 24),
                      const SizedBox(width: 8),
                      const Text(
                        'YouTube アカウント連携 (OAuth 2.0)',
                        style: TextStyle(
                            fontSize: 17, fontWeight: FontWeight.bold),
                      ),
                      const Spacer(),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: (oauth.isAuthenticated
                                  ? AppTheme.accentEmerald
                                  : Colors.white24)
                              .withValues(alpha: 0.2),
                          borderRadius: BorderRadius.circular(999),
                        ),
                        child: Text(
                          oauth.isAuthenticated
                              ? '連携済み: ${oauth.accountName ?? "YouTubeアカウント"}'
                              : '未連携',
                          style: TextStyle(
                            color: oauth.isAuthenticated
                                ? AppTheme.accentEmerald
                                : Colors.white60,
                            fontWeight: FontWeight.bold,
                            fontSize: 12.5,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'コメント抽選の「開始コメント」「終了コメント」を YouTube ライブチャットへ自動投稿する場合、または公式 Data API v3 モードを利用する場合に設定します。',
                    style: TextStyle(fontSize: 13, color: Colors.white60),
                  ),
                  const SizedBox(height: 14),
                  TextField(
                    controller: _clientIdCtrl,
                    decoration: const InputDecoration(
                      labelText: 'Google OAuth クライアント ID（デスクトップアプリ用）',
                      hintText: 'xxxxxxxxxxxx-xxxxxxxxxxxxxxxx.apps.googleusercontent.com',
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
                  const SizedBox(height: 14),
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: [
                      OutlinedButton.icon(
                        onPressed: () => ctrl.saveOAuthClient(
                          _clientIdCtrl.text.trim(),
                          _clientSecretCtrl.text.trim(),
                        ),
                        icon: const Icon(Icons.save_outlined),
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
                                width: 16,
                                height: 16,
                                child:
                                    CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.login),
                        label: Text(ctrl.isOAuthInProgress
                            ? 'ブラウザで認証待機中...'
                            : 'Google アカウントで連携'),
                      ),
                      if (oauth.isAuthenticated)
                        TextButton.icon(
                          style: TextButton.styleFrom(
                              foregroundColor: AppTheme.accentRose),
                          onPressed: ctrl.disconnectOAuth,
                          icon: const Icon(Icons.logout),
                          label: const Text('連携を解除'),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // 自動アップデート & バージョン情報カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Row(
                    children: [
                      Icon(Icons.system_update_alt,
                          color: AppTheme.accentSky, size: 24),
                      SizedBox(width: 8),
                      Text(
                        'ソフトウェア情報 ＆ GitHub 自動アップデート',
                        style: TextStyle(
                            fontSize: 17, fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Text(
                        'Stosup v${ctrl.appVersion}',
                        style: const TextStyle(
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                          color: AppTheme.accentSky,
                        ),
                      ),
                      const SizedBox(width: 12),
                      const Chip(
                        label: Text('Windows 64bit / Flutter + Rust'),
                        visualDensity: VisualDensity.compact,
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    '更新元リポジトリ: https://github.com/tako-kun1/stosup',
                    style: TextStyle(fontSize: 13, color: Colors.white60),
                  ),
                  const SizedBox(height: 14),
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: [
                      FilledButton.icon(
                        onPressed:
                            ctrl.isCheckingUpdate ? null : _checkUpdatesNow,
                        icon: ctrl.isCheckingUpdate
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child:
                                    CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Icon(Icons.refresh),
                        label: const Text('GitHub でアップデートを確認'),
                      ),
                      OutlinedButton.icon(
                        onPressed: _showCurrentReleaseNotes,
                        icon: const Icon(Icons.article_outlined),
                        label: const Text('現在のリリースノートを見る'),
                      ),
                      OutlinedButton.icon(
                        onPressed: () => ctrl.openUrl(
                            'https://github.com/tako-kun1/stosup/releases'),
                        icon: const Icon(Icons.open_in_new),
                        label: const Text('GitHub Releases を開く'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // プラットフォーム拡張ロードマップカード
          const Card(
            child: Padding(
              padding: EdgeInsets.all(18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.extension,
                          color: AppTheme.accentIndigo, size: 22),
                      SizedBox(width: 8),
                      Text(
                        '対応配信プラットフォーム状況',
                        style: TextStyle(
                            fontSize: 16, fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                  SizedBox(height: 10),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      Chip(
                        avatar: Icon(Icons.check_circle,
                            color: AppTheme.accentEmerald, size: 18),
                        label: Text('YouTube Live（対応済み）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.science,
                            color: AppTheme.accentSky, size: 18),
                        label: Text('デモ配信モード（対応済み）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: Colors.white54, size: 18),
                        label: Text('Twitch（今後対応予定）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: Colors.white54, size: 18),
                        label: Text('ニコニコ生放送（今後対応予定）'),
                      ),
                      Chip(
                        avatar: Icon(Icons.schedule,
                            color: Colors.white54, size: 18),
                        label: Text('ツイキャス（今後対応予定）'),
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
