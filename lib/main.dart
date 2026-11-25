import 'package:material_ui/material_ui.dart';

import 'screens/comments_screen.dart';
import 'screens/counters_screen.dart';
import 'screens/first_time_screen.dart';
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

class StosupApp extends StatefulWidget {
  const StosupApp({super.key, this.afterUpdate = false});

  final bool afterUpdate;

  @override
  State<StosupApp> createState() => _StosupAppState();
}

class _StosupAppState extends State<StosupApp> {
  final _controller = StosupController();

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onChanged);
    _controller.initialize(afterUpdate: widget.afterUpdate);
  }

  @override
  void dispose() {
    _controller.removeListener(_onChanged);
    _controller.dispose();
    super.dispose();
  }

  void _onChanged() {
    if (mounted) setState(() {});
  }

  ThemeMode _mapThemeMode(AppThemeMode mode) {
    switch (mode) {
      case AppThemeMode.light:
        return ThemeMode.light;
      case AppThemeMode.dark:
        return ThemeMode.dark;
      case AppThemeMode.system:
        return ThemeMode.system;
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Stosup - 配信者補助ソフトウェア',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.lightTheme(),
      darkTheme: AppTheme.darkTheme(),
      themeMode: _mapThemeMode(_controller.themeMode),
      home: StosupShell(controller: _controller),
    );
  }
}

class StosupShell extends StatefulWidget {
  const StosupShell({super.key, required this.controller});

  final StosupController controller;

  @override
  State<StosupShell> createState() => _StosupShellState();
}

class _StosupShellState extends State<StosupShell> {
  int _selectedIndex = 0;
  bool _startupNotesShown = false;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onStateChanged);
    widget.controller.onNotice = _showNotice;
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onStateChanged);
    super.dispose();
  }

  void _onStateChanged() {
    if (!mounted) return;
    setState(() {});

    // アップデート後の初回起動時に自動でリリースノートダイアログを表示
    if (!_startupNotesShown &&
        widget.controller.pendingStartupReleaseNotes != null) {
      _startupNotesShown = true;
      final notes = widget.controller.pendingStartupReleaseNotes!;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        showDialog<void>(
          context: context,
          barrierDismissible: false,
          builder: (_) => ReleaseNotesDialog(
            info: notes,
            onClose: () => widget.controller.dismissStartupReleaseNotes(),
          ),
        );
      });
    }
  }

  void _showNotice(String level, String message) {
    if (!mounted) return;
    final p = context.palette;
    Color accent = p.accent;
    if (level == 'error') accent = p.danger;
    if (level == 'success') accent = p.success;
    if (level == 'warning') accent = p.warning;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Container(
              width: 3,
              height: 18,
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
    final c = widget.controller;
    switch (_selectedIndex) {
      case 0:
        return CommentsScreen(controller: c);
      case 1:
        return HighlightRulesScreen(controller: c);
      case 2:
        return CountersScreen(controller: c);
      case 3:
        return FirstTimeScreen(controller: c);
      case 4:
        return LotteryScreen(controller: c);
      case 5:
        return UsersScreen(controller: c);
      case 6:
        return OverlayScreen(controller: c);
      case 7:
        return SettingsScreen(controller: c);
      default:
        return CommentsScreen(controller: c);
    }
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;
    final p = context.palette;

    if (ctrl.initError != null) {
      return Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.error_outline, size: 48, color: p.danger),
                const SizedBox(height: 14),
                const Text(
                  'Stosup バックエンドの初期化に失敗しました',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 8),
                SelectableText(
                  ctrl.initError!,
                  style: TextStyle(color: p.textSecondary),
                ),
              ],
            ),
          ),
        ),
      );
    }

    if (!ctrl.isInitialized) {
      return const Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator(strokeWidth: 2.5),
              SizedBox(height: 14),
              Text('Stosup を起動しています...'),
            ],
          ),
        ),
      );
    }

    final connected = ctrl.connectionState.status == ConnectionStatus.connected;
    final updateInfo = ctrl.availableUpdate;
    final flags = ctrl.featureFlags;
    final allLotteryOff =
        !flags.userLotteryEnabled && !flags.commentLotteryEnabled;
    final isDarkNow = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      body: Column(
        children: [
          // 新バージョン通知バー
          if (updateInfo != null && updateInfo.hasUpdate)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 7),
              decoration: BoxDecoration(
                color: p.accent.withValues(alpha: 0.12),
                border: Border(bottom: BorderSide(color: p.border)),
              ),
              child: Row(
                children: [
                  Icon(Icons.system_update_alt, color: p.accent, size: 17),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      '新しいバージョン Stosup v${updateInfo.latestVersion} が利用可能です（現在: v${updateInfo.currentVersion}）',
                      style: TextStyle(
                        color: p.textPrimary,
                        fontWeight: FontWeight.w600,
                        fontSize: 12.5,
                      ),
                    ),
                  ),
                  FilledButton(
                    style: FilledButton.styleFrom(
                      visualDensity: VisualDensity.compact,
                    ),
                    onPressed: () {
                      showDialog<void>(
                        context: context,
                        builder: (_) => UpdateDialog(
                          controller: ctrl,
                          info: updateInfo,
                        ),
                      );
                    },
                    child: const Text('詳細・更新'),
                  ),
                ],
              ),
            ),

          Expanded(
            child: Row(
              children: [
                // 左サイドバー（落ち着いたデスクトップツール調）
                Container(
                  width: 212,
                  decoration: BoxDecoration(
                    color: p.sidebarBg,
                    border: Border(right: BorderSide(color: p.border)),
                  ),
                  child: Column(
                    children: [
                      // ブランドヘッダー（統一ロゴ維持）
                      Padding(
                        padding: const EdgeInsets.fromLTRB(14, 16, 14, 14),
                        child: Row(
                          children: [
                            Container(
                              width: 34,
                              height: 34,
                              decoration: BoxDecoration(
                                gradient: const LinearGradient(
                                  colors: [
                                    Color(0xFF38BDF8),
                                    Color(0xFF818CF8),
                                  ],
                                ),
                                borderRadius: BorderRadius.circular(9),
                              ),
                              child: const Icon(Icons.podcasts,
                                  color: Colors.white, size: 20),
                            ),
                            const SizedBox(width: 10),
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  'Stosup',
                                  style: TextStyle(
                                    fontSize: 16,
                                    fontWeight: FontWeight.w800,
                                    color: p.textPrimary,
                                  ),
                                ),
                                Text(
                                  'v${ctrl.appVersion}',
                                  style: TextStyle(
                                      fontSize: 11, color: p.textMuted),
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
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                          children: [
                            _NavTile(
                              icon: Icons.chat_bubble_outline,
                              label: 'コメント表示',
                              selected: _selectedIndex == 0,
                              badge: !flags.commentDisplayEnabled
                                  ? 'OFF'
                                  : (connected ? 'LIVE' : null),
                              badgeColor: !flags.commentDisplayEnabled
                                  ? p.textMuted
                                  : p.success,
                              dimmed: !flags.commentDisplayEnabled,
                              onTap: () => setState(() => _selectedIndex = 0),
                            ),
                            _NavTile(
                              icon: Icons.palette_outlined,
                              label: 'ワード色反応',
                              selected: _selectedIndex == 1,
                              badge: !flags.wordHighlightEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.wordHighlightEnabled,
                              onTap: () => setState(() => _selectedIndex = 1),
                            ),
                            _NavTile(
                              icon: Icons.pin_outlined,
                              label: 'ワードカウント',
                              selected: _selectedIndex == 2,
                              badge: !flags.wordCounterEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.wordCounterEnabled,
                              onTap: () => setState(() => _selectedIndex = 2),
                            ),
                            _NavTile(
                              icon: Icons.waving_hand_outlined,
                              label: '初見カウンター',
                              selected: _selectedIndex == 3,
                              badge: !flags.firstTimeCounterEnabled
                                  ? 'OFF'
                                  : (ctrl.firstTimeState.count > BigInt.zero
                                      ? ctrl.firstTimeState.count.toString()
                                      : null),
                              badgeColor: !flags.firstTimeCounterEnabled
                                  ? p.textMuted
                                  : p.success,
                              dimmed: !flags.firstTimeCounterEnabled,
                              onTap: () => setState(() => _selectedIndex = 3),
                            ),
                            _NavTile(
                              icon: Icons.casino_outlined,
                              label: '抽選システム',
                              selected: _selectedIndex == 4,
                              badge: allLotteryOff
                                  ? 'OFF'
                                  : (ctrl.commentLottery.isAccepting
                                      ? '受付中'
                                      : null),
                              badgeColor:
                                  allLotteryOff ? p.textMuted : p.warning,
                              dimmed: allLotteryOff,
                              onTap: () => setState(() => _selectedIndex = 4),
                            ),
                            _NavTile(
                              icon: Icons.people_outline,
                              label: 'ユーザー / BAN',
                              selected: _selectedIndex == 5,
                              badge: !flags.userTrackingEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.userTrackingEnabled,
                              onTap: () => setState(() => _selectedIndex = 5),
                            ),
                            _NavTile(
                              icon: Icons.brush_outlined,
                              label: 'OBS・デザイン',
                              selected: _selectedIndex == 6,
                              badge: !flags.obsOverlayEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.obsOverlayEnabled,
                              onTap: () => setState(() => _selectedIndex = 6),
                            ),
                            const Padding(
                              padding: EdgeInsets.symmetric(vertical: 8),
                              child: Divider(height: 1),
                            ),
                            _NavTile(
                              icon: Icons.settings_outlined,
                              label: '設定・機能管理',
                              selected: _selectedIndex == 7,
                              badge:
                                  updateInfo?.hasUpdate == true ? 'NEW' : null,
                              badgeColor: p.accent,
                              onTap: () => setState(() => _selectedIndex = 7),
                            ),
                          ],
                        ),
                      ),
                      // 下部ステータス ＆ クイックライト/ダーク切替
                      Container(
                        margin: const EdgeInsets.all(8),
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 6),
                        decoration: BoxDecoration(
                          color: p.cardBg,
                          borderRadius: BorderRadius.circular(6),
                          border: Border.all(color: p.border),
                        ),
                        child: Row(
                          children: [
                            Container(
                              width: 7,
                              height: 7,
                              decoration: BoxDecoration(
                                color: flags.obsOverlayEnabled
                                    ? p.success
                                    : p.textMuted,
                                shape: BoxShape.circle,
                              ),
                            ),
                            const SizedBox(width: 7),
                            Expanded(
                              child: Text(
                                flags.obsOverlayEnabled
                                    ? 'OBS :${ctrl.overlayConfig.port}'
                                    : 'OBS 停止中',
                                style: TextStyle(
                                  fontFamily: 'Consolas',
                                  fontSize: 11.5,
                                  color: p.textSecondary,
                                ),
                              ),
                            ),
                            IconButton(
                              tooltip:
                                  isDarkNow ? 'ライトモードへ切替' : 'ダークモードへ切替',
                              visualDensity: VisualDensity.compact,
                              padding: EdgeInsets.zero,
                              constraints: const BoxConstraints(
                                minWidth: 28,
                                minHeight: 28,
                              ),
                              onPressed: () => ctrl.setThemeMode(
                                isDarkNow
                                    ? AppThemeMode.light
                                    : AppThemeMode.dark,
                              ),
                              icon: Icon(
                                isDarkNow
                                    ? Icons.light_mode_outlined
                                    : Icons.dark_mode_outlined,
                                size: 16,
                                color: p.textSecondary,
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
    this.dimmed = false,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;
  final String? badge;
  final Color? badgeColor;
  final bool dimmed;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Padding(
      padding: const EdgeInsets.only(bottom: 2),
      child: Material(
        color: selected ? p.cardBg : Colors.transparent,
        borderRadius: BorderRadius.circular(6),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(6),
              border: Border.all(
                color: selected ? p.border : Colors.transparent,
              ),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 18,
                  color: selected
                      ? p.accent
                      : (dimmed ? p.textMuted : p.textSecondary),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    label,
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight:
                          selected ? FontWeight.w700 : FontWeight.w500,
                      color: selected
                          ? p.textPrimary
                          : (dimmed ? p.textMuted : p.textSecondary),
                    ),
                  ),
                ),
                if (badge != null)
                  Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                    decoration: BoxDecoration(
                      color: (badgeColor ?? p.accent).withValues(alpha: 0.16),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      badge!,
                      style: TextStyle(
                        fontSize: 10,
                        fontWeight: FontWeight.w700,
                        color: badgeColor ?? p.accent,
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
