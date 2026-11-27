import 'package:material_ui/material_ui.dart';

import 'screens/automation_screen.dart';
import 'screens/comments_screen.dart';
import 'screens/counters_screen.dart';
import 'screens/first_time_screen.dart';
import 'screens/highlight_rules_screen.dart';
import 'screens/lottery_screen.dart';
import 'screens/media_reaction_screen.dart';
import 'screens/overlay_screen.dart';
import 'screens/poll_screen.dart';
import 'screens/queue_screen.dart';
import 'screens/report_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/superchat_screen.dart';
import 'screens/users_screen.dart';
import 'src/rust/models.dart';
import 'state/stosup_controller.dart';
import 'theme/app_theme.dart';
import 'widgets/license_agreement_screen.dart';
import 'widgets/release_notes_dialog.dart';
import 'widgets/update_dialog.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  final afterUpdate = args.contains('--after-update');

  // ウィンドウ初回表示 (Win32 Show) より前にテーマ設定と利用規約判定を完了させ、
  // 起動時の白黒フラッシュ・ローディング経由のちらつきを防止する
  final controller = StosupController();
  await controller.initialize(afterUpdate: afterUpdate);

  runApp(StosupApp(afterUpdate: afterUpdate, controller: controller));
}

class StosupApp extends StatefulWidget {
  const StosupApp({
    super.key,
    this.afterUpdate = false,
    this.controller,
  });

  final bool afterUpdate;
  final StosupController? controller;

  @override
  State<StosupApp> createState() => _StosupAppState();
}

class _StosupAppState extends State<StosupApp> {
  late final StosupController _controller;
  late final bool _ownsController;
  late AppThemeMode _lastThemeMode;

  @override
  void initState() {
    super.initState();
    _ownsController = widget.controller == null;
    _controller = widget.controller ?? StosupController();
    _lastThemeMode = _controller.themeMode;
    _controller.addListener(_onChanged);
    if (!_controller.isInitialized && _controller.initError == null) {
      _controller.initialize(afterUpdate: widget.afterUpdate);
    }
  }

  @override
  void dispose() {
    _controller.removeListener(_onChanged);
    if (_ownsController) {
      _controller.dispose();
    }
    super.dispose();
  }

  void _onChanged() {
    if (!mounted) return;
    // MaterialApp 全体の再ビルドはテーマモードが変わった時のみに限定
    if (_controller.themeMode != _lastThemeMode) {
      _lastThemeMode = _controller.themeMode;
      setState(() {});
    }
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
      // テーマ切替時のクロスフェード明滅（白⇔黒フラッシュ）を無効化
      themeAnimationDuration: Duration.zero,
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
  late int _lastShellRevision;

  @override
  void initState() {
    super.initState();
    _lastShellRevision = widget.controller.topicRevision(StosupTopic.shell);
    widget.controller.addListener(_onStateChanged);
    widget.controller.onNotice = _showNotice;
    widget.controller.setUsersViewActive(
      _selectedIndex == 8 || _selectedIndex == 9,
    );
  }

  void _selectTab(int idx) {
    if (_selectedIndex == idx) return;
    setState(() => _selectedIndex = idx);
    widget.controller.setUsersViewActive(idx == 8 || idx == 9);
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onStateChanged);
    super.dispose();
  }

  void _onStateChanged() {
    if (!mounted) return;
    // サイドバーやシェル全体に影響するトピック（shell）が変わった時だけシェルを再ビルドし、
    // コメント受信等でサイドバー全体や他画面が巻き込み再ビルドされるのを完全に防止する
    final nextShellRev = widget.controller.topicRevision(StosupTopic.shell);
    if (nextShellRev != _lastShellRevision) {
      _lastShellRevision = nextShellRev;
      setState(() {});
    }

    // アップデート後の初回起動時に自動でリリースノートダイアログを表示（規約同意完了後のみ）
    if (!widget.controller.needsLicenseAcceptance &&
        !_startupNotesShown &&
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

  StosupTopic _topicForIndex(int idx) {
    switch (idx) {
      case 0:
        return StosupTopic.comments;
      case 1:
        return StosupTopic.superchat;
      case 2:
        return StosupTopic.highlight;
      case 3:
        return StosupTopic.media;
      case 4:
        return StosupTopic.counters;
      case 5:
        return StosupTopic.firstTime;
      case 6:
        return StosupTopic.poll;
      case 7:
        return StosupTopic.queue;
      case 8:
        return StosupTopic.lottery;
      case 9:
        return StosupTopic.users;
      case 10:
        return StosupTopic.overlay;
      case 11:
        return StosupTopic.automation;
      case 12:
        return StosupTopic.report;
      case 13:
        return StosupTopic.settings;
      default:
        return StosupTopic.comments;
    }
  }

  Widget _buildBody() {
    final c = widget.controller;
    return _TopicScopedScreen(
      controller: c,
      topic: _topicForIndex(_selectedIndex),
      builder: () {
        switch (_selectedIndex) {
          case 0:
            return CommentsScreen(controller: c);
          case 1:
            return SuperchatScreen(controller: c);
          case 2:
            return HighlightRulesScreen(controller: c);
          case 3:
            return MediaReactionScreen(controller: c);
          case 4:
            return CountersScreen(controller: c);
          case 5:
            return FirstTimeScreen(controller: c);
          case 6:
            return PollScreen(controller: c);
          case 7:
            return QueueScreen(controller: c);
          case 8:
            return LotteryScreen(controller: c);
          case 9:
            return UsersScreen(controller: c);
          case 10:
            return OverlayScreen(controller: c);
          case 11:
            return AutomationScreen(controller: c);
          case 12:
            return ReportScreen(controller: c);
          case 13:
            return SettingsScreen(controller: c);
          default:
            return CommentsScreen(controller: c);
        }
      },
    );
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

    // 利用規約に未同意、または規約が更新された場合は同意ボタンを押すまでメイン画面へ進ませない
    if (ctrl.needsLicenseAcceptance) {
      return LicenseAgreementScreen(controller: ctrl);
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
              crossAxisAlignment: CrossAxisAlignment.stretch,
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
                              label: 'トークモニター',
                              selected: _selectedIndex == 0,
                              badge: !flags.commentDisplayEnabled
                                  ? 'OFF'
                                  : (connected ? 'LIVE' : null),
                              badgeColor: !flags.commentDisplayEnabled
                                  ? p.textMuted
                                  : p.success,
                              dimmed: !flags.commentDisplayEnabled,
                              onTap: () => _selectTab(0),
                            ),
                            _NavTile(
                              icon: Icons.volunteer_activism_outlined,
                              label: 'エールボックス',
                              selected: _selectedIndex == 1,
                              badge: !flags.superchatEnabled
                                  ? 'OFF'
                                  : (ctrl.unreadMonetizationCount > 0
                                      ? '未読${ctrl.unreadMonetizationCount}'
                                      : null),
                              badgeColor: !flags.superchatEnabled
                                  ? p.textMuted
                                  : p.warning,
                              dimmed: !flags.superchatEnabled,
                              onTap: () => _selectTab(1),
                            ),
                            _NavTile(
                              icon: Icons.palette_outlined,
                              label: 'ハイライトサイン',
                              selected: _selectedIndex == 2,
                              badge: !flags.wordHighlightEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.wordHighlightEnabled,
                              onTap: () => _selectTab(2),
                            ),
                            _NavTile(
                              icon: Icons.movie_filter_outlined,
                              label: 'リアクションステージ',
                              selected: _selectedIndex == 3,
                              badge: !flags.mediaReactionEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.mediaReactionEnabled,
                              onTap: () => _selectTab(3),
                            ),
                            _NavTile(
                              icon: Icons.pin_outlined,
                              label: 'コールカウンター',
                              selected: _selectedIndex == 4,
                              badge: !flags.wordCounterEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.wordCounterEnabled,
                              onTap: () => _selectTab(4),
                            ),
                            _NavTile(
                              icon: Icons.waving_hand_outlined,
                              label: 'ウェルカムカウンター',
                              selected: _selectedIndex == 5,
                              badge: !flags.firstTimeCounterEnabled
                                  ? 'OFF'
                                  : (ctrl.firstTimeState.count > BigInt.zero
                                      ? ctrl.firstTimeState.count.toString()
                                      : null),
                              badgeColor: !flags.firstTimeCounterEnabled
                                  ? p.textMuted
                                  : p.success,
                              dimmed: !flags.firstTimeCounterEnabled,
                              onTap: () => _selectTab(5),
                            ),
                            _NavTile(
                              icon: Icons.poll_outlined,
                              label: 'ライブアンケート',
                              selected: _selectedIndex == 6,
                              badge: !flags.pollEnabled
                                  ? 'OFF'
                                  : (ctrl.pollState.isActive ? '受付中' : null),
                              badgeColor: !flags.pollEnabled
                                  ? p.textMuted
                                  : p.success,
                              dimmed: !flags.pollEnabled,
                              onTap: () => _selectTab(6),
                            ),
                            _NavTile(
                              icon: Icons.queue_play_next,
                              label: 'エントリーレーン',
                              selected: _selectedIndex == 7,
                              badge: !flags.queueEnabled
                                  ? 'OFF'
                                  : (ctrl.queueState.config.isAccepting
                                      ? '受付中'
                                      : null),
                              badgeColor: !flags.queueEnabled
                                  ? p.textMuted
                                  : p.accent,
                              dimmed: !flags.queueEnabled,
                              onTap: () => _selectTab(7),
                            ),
                            _NavTile(
                              icon: Icons.casino_outlined,
                              label: 'ドローボックス',
                              selected: _selectedIndex == 8,
                              badge: allLotteryOff
                                  ? 'OFF'
                                  : (ctrl.commentLottery.isAccepting
                                      ? '受付中'
                                      : null),
                              badgeColor:
                                  allLotteryOff ? p.textMuted : p.warning,
                              dimmed: allLotteryOff,
                              onTap: () => _selectTab(8),
                            ),
                            _NavTile(
                              icon: Icons.people_outline,
                              label: 'リスナー名簿',
                              selected: _selectedIndex == 9,
                              badge: !flags.userTrackingEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.userTrackingEnabled,
                              onTap: () => _selectTab(9),
                            ),
                            _NavTile(
                              icon: Icons.brush_outlined,
                              label: 'オーバーレイスタジオ',
                              selected: _selectedIndex == 10,
                              badge: !flags.obsOverlayEnabled ? 'OFF' : null,
                              badgeColor: p.textMuted,
                              dimmed: !flags.obsOverlayEnabled,
                              onTap: () => _selectTab(10),
                            ),
                            _NavTile(
                              icon: Icons.auto_mode_outlined,
                              label: '連携・オートメーション',
                              selected: _selectedIndex == 11,
                              badge: ctrl.obsWsStatus.connected ? 'OBS' : null,
                              badgeColor: p.success,
                              onTap: () => _selectTab(11),
                            ),
                            _NavTile(
                              icon: Icons.analytics_outlined,
                              label: 'オンエアレポート',
                              selected: _selectedIndex == 12,
                              onTap: () => _selectTab(12),
                            ),
                            const Padding(
                              padding: EdgeInsets.symmetric(vertical: 8),
                              child: Divider(height: 1),
                            ),
                            _NavTile(
                              icon: Icons.settings_outlined,
                              label: '設定・機能管理',
                              selected: _selectedIndex == 13,
                              badge:
                                  updateInfo?.hasUpdate == true ? 'NEW' : null,
                              badgeColor: p.accent,
                              onTap: () => _selectTab(13),
                            ),
                          ],
                        ),
                      ),
                      // クイック配信プロファイル切替バー
                      if (ctrl.streamProfiles.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.fromLTRB(8, 4, 8, 0),
                          child: Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 8, vertical: 2),
                            decoration: BoxDecoration(
                              color: p.cardBg,
                              borderRadius: BorderRadius.circular(6),
                              border: Border.all(color: p.border),
                            ),
                            child: DropdownButtonHideUnderline(
                              child: DropdownButton<String>(
                                isExpanded: true,
                                isDense: true,
                                value: ctrl.streamProfiles
                                        .any((e) => e.isActive)
                                    ? ctrl.streamProfiles
                                        .firstWhere((e) => e.isActive)
                                        .id
                                    : null,
                                hint: Text(
                                  'プロファイル切替...',
                                  style: TextStyle(
                                      fontSize: 11.5, color: p.textSecondary),
                                ),
                                style: TextStyle(
                                    fontSize: 12, color: p.textPrimary),
                                dropdownColor: p.cardBg,
                                items: ctrl.streamProfiles
                                    .map(
                                      (pr) => DropdownMenuItem(
                                        value: pr.id,
                                        child: Text(
                                          pr.name,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ),
                                    )
                                    .toList(),
                                onChanged: (id) {
                                  if (id != null) {
                                    ctrl.applyStreamProfile(id);
                                  }
                                },
                              ),
                            ),
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
                                ctrl.remoteAccessStatus.serverRunning
                                    ? (ctrl.remoteAccessStatus.config
                                            .allowLanAccess
                                        ? 'LAN :${ctrl.overlayConfig.port}'
                                        : 'OBS :${ctrl.overlayConfig.port}')
                                    : (flags.obsOverlayEnabled
                                        ? 'OBS :${ctrl.overlayConfig.port}'
                                        : 'OBS 停止中'),
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

/// 対象トピックのリビジョンが変化したときだけ画面本体を再ビルドするスコープ境界
class _TopicScopedScreen extends StatefulWidget {
  const _TopicScopedScreen({
    required this.controller,
    required this.topic,
    required this.builder,
  });

  final StosupController controller;
  final StosupTopic topic;
  final Widget Function() builder;

  @override
  State<_TopicScopedScreen> createState() => _TopicScopedScreenState();
}

class _TopicScopedScreenState extends State<_TopicScopedScreen> {
  late int _lastRev;

  @override
  void initState() {
    super.initState();
    _lastRev = widget.controller.topicRevision(widget.topic);
    widget.controller.addListener(_onControllerChanged);
  }

  @override
  void didUpdateWidget(covariant _TopicScopedScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_onControllerChanged);
      widget.controller.addListener(_onControllerChanged);
    }
    if (oldWidget.topic != widget.topic ||
        oldWidget.controller != widget.controller) {
      _lastRev = widget.controller.topicRevision(widget.topic);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onControllerChanged);
    super.dispose();
  }

  void _onControllerChanged() {
    if (!mounted) return;
    final nextRev = widget.controller.topicRevision(widget.topic);
    if (nextRev != _lastRev) {
      _lastRev = nextRev;
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) => widget.builder();
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
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 12.5,
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
