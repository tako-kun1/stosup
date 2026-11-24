import 'package:material_ui/material_ui.dart';

import 'screens/comments_screen.dart';
import 'screens/counters_screen.dart';
import 'screens/highlight_rules_screen.dart';
import 'screens/lottery_screen.dart';
import 'screens/overlay_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/users_screen.dart';
import 'src/rust/models.dart';
import 'state/stosup_controller.dart';
import 'theme/app_theme.dart';
import 'widgets/release_notes_dialog.dart';
import 'widgets/update_dialog.dart';

void main(List<String> args) {
  WidgetsFlutterBinding.ensureInitialized();
  final afterUpdate = args.contains('--after-update');
  runApp(StosupApp(afterUpdate: afterUpdate));
}

class StosupApp extends StatelessWidget {
  const StosupApp({super.key, this.afterUpdate = false});

  final bool afterUpdate;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Stosup - 配信者補助ソフトウェア',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.darkTheme(),
      home: StosupShell(afterUpdate: afterUpdate),
    );
  }
}

class StosupShell extends StatefulWidget {
  const StosupShell({super.key, required this.afterUpdate});

  final bool afterUpdate;

  @override
  State<StosupShell> createState() => _StosupShellState();
}

class _StosupShellState extends State<StosupShell> {
  final _controller = StosupController();
  int _selectedIndex = 0;
  bool _startupNotesShown = false;

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onStateChanged);
    _controller.onNotice = _showNotice;
    _controller.initialize(afterUpdate: widget.afterUpdate);
  }

  @override
  void dispose() {
    _controller.removeListener(_onStateChanged);
    _controller.dispose();
    super.dispose();
  }

  void _onStateChanged() {
    if (!mounted) return;
    setState(() {});

    // アップデート後の初回起動時に自動でリリースノートダイアログを表示
    if (!_startupNotesShown &&
        _controller.pendingStartupReleaseNotes != null) {
      _startupNotesShown = true;
      final notes = _controller.pendingStartupReleaseNotes!;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        showDialog<void>(
          context: context,
          barrierDismissible: false,
          builder: (_) => ReleaseNotesDialog(
            info: notes,
            onClose: () => _controller.dismissStartupReleaseNotes(),
          ),
        );
      });
    }
  }

  void _showNotice(String level, String message) {
    if (!mounted) return;
    Color accent = AppTheme.accentSky;
    if (level == 'error') accent = AppTheme.accentRose;
    if (level == 'success') accent = AppTheme.accentEmerald;
    if (level == 'warning') accent = AppTheme.accentAmber;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Container(
              width: 4,
              height: 20,
              decoration: BoxDecoration(
                color: accent,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(child: Text(message)),
          ],
        ),
      ),
    );
  }

  Widget _buildBody() {
    switch (_selectedIndex) {
      case 0:
        return CommentsScreen(controller: _controller);
      case 1:
        return HighlightRulesScreen(controller: _controller);
      case 2:
        return CountersScreen(controller: _controller);
      case 3:
        return LotteryScreen(controller: _controller);
      case 4:
        return UsersScreen(controller: _controller);
      case 5:
        return OverlayScreen(controller: _controller);
      case 6:
        return SettingsScreen(controller: _controller);
      default:
        return CommentsScreen(controller: _controller);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_controller.initError != null) {
      return Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.error_outline,
                    size: 56, color: AppTheme.accentRose),
                const SizedBox(height: 16),
                const Text(
                  'Stosup バックエンドの初期化に失敗しました',
                  style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 8),
                SelectableText(
                  _controller.initError!,
                  style: const TextStyle(color: Colors.white70),
                ),
              ],
            ),
          ),
        ),
      );
    }

    if (!_controller.isInitialized) {
      return const Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator(),
              SizedBox(height: 16),
              Text('Stosup を起動しています...'),
            ],
          ),
        ),
      );
    }

    final connected =
        _controller.connectionState.status == ConnectionStatus.connected;
    final updateInfo = _controller.availableUpdate;

    return Scaffold(
      body: Column(
        children: [
          // 新バージョン通知バナー
          if (updateInfo != null && updateInfo.hasUpdate)
            Container(
              width: double.infinity,
              padding:
                  const EdgeInsets.symmetric(horizontal: 18, vertical: 8),
              decoration: BoxDecoration(
                color: AppTheme.accentEmerald.withValues(alpha: 0.16),
                border: const Border(
                  bottom: BorderSide(color: AppTheme.accentEmerald, width: 1),
                ),
              ),
              child: Row(
                children: [
                  const Icon(Icons.system_update_alt,
                      color: AppTheme.accentEmerald, size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      '新しいバージョン Stosup v${updateInfo.latestVersion} が公開されています！（現在: v${updateInfo.currentVersion}）',
                      style: const TextStyle(
                        color: AppTheme.accentEmerald,
                        fontWeight: FontWeight.bold,
                        fontSize: 13,
                      ),
                    ),
                  ),
                  FilledButton(
                    style: FilledButton.styleFrom(
                      backgroundColor: AppTheme.accentEmerald,
                      foregroundColor: Colors.black,
                      visualDensity: VisualDensity.compact,
                    ),
                    onPressed: () {
                      showDialog<void>(
                        context: context,
                        builder: (_) => UpdateDialog(
                          controller: _controller,
                          info: updateInfo,
                        ),
                      );
                    },
                    child: const Text('詳細・アップデート'),
                  ),
                ],
              ),
            ),

          Expanded(
            child: Row(
              children: [
                // 左ナビゲーションサイドバー
                Container(
                  width: 215,
                  decoration: const BoxDecoration(
                    color: AppTheme.surfaceDark,
                    border:
                        Border(right: BorderSide(color: AppTheme.borderDark)),
                  ),
                  child: Column(
                    children: [
                      // ブランドヘッダー
                      Padding(
                        padding: const EdgeInsets.fromLTRB(16, 18, 16, 14),
                        child: Row(
                          children: [
                            Container(
                              width: 36,
                              height: 36,
                              decoration: BoxDecoration(
                                gradient: const LinearGradient(
                                  colors: [
                                    AppTheme.accentSky,
                                    AppTheme.accentIndigo
                                  ],
                                ),
                                borderRadius: BorderRadius.circular(10),
                              ),
                              child: const Icon(Icons.podcasts,
                                  color: Colors.white, size: 22),
                            ),
                            const SizedBox(width: 10),
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                const Text(
                                  'Stosup',
                                  style: TextStyle(
                                    fontSize: 18,
                                    fontWeight: FontWeight.w900,
                                    letterSpacing: 0.5,
                                  ),
                                ),
                                Text(
                                  'v${_controller.appVersion}',
                                  style: const TextStyle(
                                      fontSize: 11, color: Colors.white54),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                      const Divider(height: 1),
                      const SizedBox(height: 8),
                      Expanded(
                        child: ListView(
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                          children: [
                            _NavTile(
                              icon: Icons.chat_bubble_outline,
                              label: 'コメント表示',
                              selected: _selectedIndex == 0,
                              badge: connected ? 'LIVE' : null,
                              badgeColor: AppTheme.accentEmerald,
                              onTap: () => setState(() => _selectedIndex = 0),
                            ),
                            _NavTile(
                              icon: Icons.palette_outlined,
                              label: 'ワード色反応',
                              selected: _selectedIndex == 1,
                              onTap: () => setState(() => _selectedIndex = 1),
                            ),
                            _NavTile(
                              icon: Icons.pin_outlined,
                              label: 'ワードカウント',
                              selected: _selectedIndex == 2,
                              onTap: () => setState(() => _selectedIndex = 2),
                            ),
                            _NavTile(
                              icon: Icons.casino_outlined,
                              label: '抽選システム',
                              selected: _selectedIndex == 3,
                              badge: _controller.commentLottery.isAccepting
                                  ? '受付中'
                                  : null,
                              badgeColor: AppTheme.accentAmber,
                              onTap: () => setState(() => _selectedIndex = 3),
                            ),
                            _NavTile(
                              icon: Icons.people_outline,
                              label: 'ユーザー / BAN',
                              selected: _selectedIndex == 4,
                              onTap: () => setState(() => _selectedIndex = 4),
                            ),
                            _NavTile(
                              icon: Icons.layers_outlined,
                              label: 'OBS オーバーレイ',
                              selected: _selectedIndex == 5,
                              onTap: () => setState(() => _selectedIndex = 5),
                            ),
                            const Divider(height: 20),
                            _NavTile(
                              icon: Icons.settings_outlined,
                              label: '設定・更新',
                              selected: _selectedIndex == 6,
                              badge: updateInfo?.hasUpdate == true ? 'NEW' : null,
                              badgeColor: AppTheme.accentEmerald,
                              onTap: () => setState(() => _selectedIndex = 6),
                            ),
                          ],
                        ),
                      ),
                      // 下部 OBS サーバー稼働表示
                      Container(
                        margin: const EdgeInsets.all(10),
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 8),
                        decoration: BoxDecoration(
                          color: AppTheme.bgDark,
                          borderRadius: BorderRadius.circular(10),
                          border: Border.all(color: AppTheme.borderDark),
                        ),
                        child: Row(
                          children: [
                            const Icon(Icons.circle,
                                size: 8, color: AppTheme.accentEmerald),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'OBS :${_controller.overlayConfig.port}',
                                style: const TextStyle(
                                  fontFamily: 'Consolas',
                                  fontSize: 12,
                                  color: Colors.white70,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),

                // メイン画面
                Expanded(child: _buildBody()),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _NavTile extends StatelessWidget {
  const _NavTile({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.badge,
    this.badgeColor,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;
  final String? badge;
  final Color? badgeColor;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Material(
        color: selected
            ? AppTheme.accentSky.withValues(alpha: 0.16)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(10),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(10),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 20,
                  color: selected ? AppTheme.accentSky : Colors.white70,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    label,
                    style: TextStyle(
                      fontSize: 13.5,
                      fontWeight:
                          selected ? FontWeight.bold : FontWeight.normal,
                      color: selected ? Colors.white : Colors.white70,
                    ),
                  ),
                ),
                if (badge != null)
                  Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: badgeColor ?? AppTheme.accentSky,
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(
                      badge!,
                      style: const TextStyle(
                        fontSize: 10,
                        fontWeight: FontWeight.bold,
                        color: Colors.black,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
