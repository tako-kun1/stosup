import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

/// 初見カウンター画面（配信ごとの0リセット、未登録/初見ワード検知、オプションのアイコン＆名前表示）
class FirstTimeScreen extends StatefulWidget {
  const FirstTimeScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<FirstTimeScreen> createState() => _FirstTimeScreenState();
}

class _FirstTimeScreenState extends State<FirstTimeScreen> {
  late TextEditingController _keywordsCtrl;
  late bool _detectUnregistered;
  late bool _detectKeyword;
  late bool _autoResetOnNewStream;
  late bool _showProfile;
  late bool _excludeBanned;
  late String _accentColor;

  @override
  void initState() {
    super.initState();
    final cfg = widget.controller.firstTimeState.config;
    _keywordsCtrl = TextEditingController(text: cfg.keywords.join(', '));
    _detectUnregistered = cfg.detectUnregistered;
    _detectKeyword = cfg.detectKeyword;
    _autoResetOnNewStream = cfg.autoResetOnNewStream;
    _showProfile = cfg.showFirstTimerProfile;
    _excludeBanned = cfg.excludeBanned;
    _accentColor = cfg.accentColor;
  }

  @override
  void dispose() {
    _keywordsCtrl.dispose();
    super.dispose();
  }

  Future<void> _saveConfig() async {
    final kws = _keywordsCtrl.text
        .split(RegExp(r'[,、\n]'))
        .map((e) => e.trim())
        .where((e) => e.isNotEmpty)
        .toList();
    await widget.controller.saveFirstTimeConfig(FirstTimeCounterConfig(
      detectUnregistered: _detectUnregistered,
      detectKeyword: _detectKeyword,
      keywords: kws.isEmpty ? const ['初見'] : kws,
      autoResetOnNewStream: _autoResetOnNewStream,
      showFirstTimerProfile: _showProfile,
      excludeBanned: _excludeBanned,
      accentColor: _accentColor,
    ));
  }

  Future<void> _confirmResetAllHistory() async {
    final p = context.palette;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('全ユーザーの初見検知履歴をリセットしますか？'),
        content: Text(
          'これまでの「初見検知済み」フラグを全ユーザーから消去します。\n'
          '通常、配信ごとにカウントだけ 0 にしたい場合は「配信カウントを 0 にリセット」をお使いください。',
          style: TextStyle(color: p.textSecondary, fontSize: 13),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: p.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('履歴も完全リセット'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await widget.controller.resetFirstTimeAllHistory();
    }
  }

  String _formatTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final ft = ctrl.firstTimeState;
    final enabled = ctrl.featureFlags.firstTimeCounterEnabled;
    final accent = AppTheme.parseCssHex(_accentColor, fallback: p.success);
    final overlayUrl = '${ctrl.overlayBaseUrl}/overlay/first_time';

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      '初見カウンター',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '未登録ユーザーの初回コメント、または初見ワード発言を 1 アカウントにつき 1 度だけ検知してカウントします（配信ごとにカウントを 0 へリセット可能）。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: '初見カウンター',
                enabled: enabled,
                onChanged: (v) => ctrl.setFeatureEnabled(
                  firstTimeCounter: v,
                  featureLabel: '初見カウンター',
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),
          if (!enabled)
            FeatureDisabledBanner(
              featureName: '初見カウンター',
              description: '初見ユーザーの自動検知と OBS 初見カウンター表示を停止しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                firstTimeCounter: true,
                featureLabel: '初見カウンター',
              ),
            ),

          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 左: カウント表示 ＆ 検知設定
                SizedBox(
                  width: 440,
                  child: ListView(
                    children: [
                      // 現在のカウントカード
                      Card(
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                          side: BorderSide(
                            color: enabled
                                ? accent.withValues(alpha: 0.65)
                                : p.border,
                            width: enabled ? 1.5 : 1,
                          ),
                        ),
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  const Text('今回の配信の初見さん',
                                      style: TextStyle(
                                          fontWeight: FontWeight.bold,
                                          fontSize: 14.5)),
                                  const Spacer(),
                                  Text(
                                    '累計検知: ${ft.totalDetectedAllTime}人',
                                    style: TextStyle(
                                        fontSize: 12, color: p.textSecondary),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              Row(
                                crossAxisAlignment: CrossAxisAlignment.center,
                                children: [
                                  Text(
                                    ft.count.toString(),
                                    style: TextStyle(
                                      fontSize: 52,
                                      fontWeight: FontWeight.w900,
                                      color: accent,
                                    ),
                                  ),
                                  const SizedBox(width: 8),
                                  Text('人',
                                      style: TextStyle(
                                          fontSize: 18,
                                          color: p.textSecondary)),
                                  const Spacer(),
                                  IconButton.outlined(
                                    tooltip: '-1',
                                    onPressed: () =>
                                        ctrl.adjustFirstTimeCount(-1),
                                    icon: const Icon(Icons.remove, size: 17),
                                  ),
                                  const SizedBox(width: 6),
                                  IconButton.outlined(
                                    tooltip: '+1',
                                    onPressed: () =>
                                        ctrl.adjustFirstTimeCount(1),
                                    icon: const Icon(Icons.add, size: 17),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 10),
                              Wrap(
                                spacing: 8,
                                runSpacing: 8,
                                children: [
                                  FilledButton.icon(
                                    onPressed: ctrl.resetFirstTimeSessionCount,
                                    icon: const Icon(Icons.restart_alt,
                                        size: 17),
                                    label: const Text('配信カウントを 0 にリセット'),
                                  ),
                                  OutlinedButton.icon(
                                    onPressed: () {
                                      Clipboard.setData(
                                          ClipboardData(text: overlayUrl));
                                      ScaffoldMessenger.of(context)
                                          .showSnackBar(
                                        const SnackBar(
                                          content: Text(
                                              '初見カウンター用 OBS URL をコピーしました'),
                                        ),
                                      );
                                    },
                                    icon: const Icon(Icons.link, size: 16),
                                    label: const Text('OBS URLコピー'),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 12),

                      // 設定カード
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text('検知ルール ＆ 表示オプション',
                                  style: TextStyle(
                                      fontWeight: FontWeight.bold,
                                      fontSize: 15)),
                              const SizedBox(height: 8),
                              SwitchListTile(
                                dense: true,
                                contentPadding: EdgeInsets.zero,
                                value: _detectUnregistered,
                                onChanged: (v) =>
                                    setState(() => _detectUnregistered = v),
                                title: const Text('未登録ユーザーの初回コメントで検知'),
                                subtitle:
                                    const Text('ユーザーリストにまだ登録されていない人を自動検知'),
                              ),
                              SwitchListTile(
                                dense: true,
                                contentPadding: EdgeInsets.zero,
                                value: _detectKeyword,
                                onChanged: (v) =>
                                    setState(() => _detectKeyword = v),
                                title: const Text('初見ワードの発言で検知'),
                                subtitle: const Text(
                                    'まだ初見カウントされていない人が下のワードを言うと検知'),
                              ),
                              if (_detectKeyword) ...[
                                const SizedBox(height: 6),
                                TextField(
                                  controller: _keywordsCtrl,
                                  decoration: const InputDecoration(
                                    labelText: '初見判定ワード（カンマ区切り）',
                                    hintText: '初見, しょけん',
                                  ),
                                ),
                              ],
                              const Divider(height: 20),
                              SwitchListTile(
                                dense: true,
                                contentPadding: EdgeInsets.zero,
                                value: _showProfile,
                                onChanged: (v) =>
                                    setState(() => _showProfile = v),
                                title: const Text('初見の人のアイコンと名前を表示（オプション）'),
                                subtitle: const Text(
                                    'デフォルトはオフ。オンにすると右欄および OBS オーバーレイに表示します'),
                              ),
                              SwitchListTile(
                                dense: true,
                                contentPadding: EdgeInsets.zero,
                                value: _autoResetOnNewStream,
                                onChanged: (v) =>
                                    setState(() => _autoResetOnNewStream = v),
                                title: const Text('新しい配信接続時に自動でカウントを 0 にする'),
                              ),
                              SwitchListTile(
                                dense: true,
                                contentPadding: EdgeInsets.zero,
                                value: _excludeBanned,
                                onChanged: (v) =>
                                    setState(() => _excludeBanned = v),
                                title: const Text('BAN 済みユーザーを除外する'),
                              ),
                              const Divider(height: 18),
                              ColorPickerRow(
                                label: 'アクセントカラー',
                                hexValue: _accentColor,
                                presets: const [
                                  '#16A34A',
                                  '#2563EB',
                                  '#D97706',
                                  '#DB2777',
                                  '#7C3AED',
                                  '#EA580C',
                                ],
                                onChanged: (v) =>
                                    setState(() => _accentColor = v),
                              ),
                              const SizedBox(height: 14),
                              Row(
                                children: [
                                  TextButton(
                                    style: TextButton.styleFrom(
                                        foregroundColor: p.danger),
                                    onPressed: _confirmResetAllHistory,
                                    child: const Text('全ユーザーの履歴リセット'),
                                  ),
                                  const Spacer(),
                                  FilledButton.icon(
                                    onPressed: _saveConfig,
                                    icon: const Icon(Icons.save_outlined,
                                        size: 17),
                                    label: const Text('設定を保存'),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 16),

                // 右: オプション（初見の人のアイコンと名前表示）
                Expanded(
                  child: Card(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Padding(
                          padding: const EdgeInsets.all(14),
                          child: Row(
                            children: [
                              const Text(
                                '初見さんのアイコン・名前表示',
                                style: TextStyle(
                                    fontWeight: FontWeight.bold, fontSize: 14.5),
                              ),
                              const Spacer(),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 8, vertical: 2),
                                decoration: BoxDecoration(
                                  color: (ft.config.showFirstTimerProfile
                                          ? p.success
                                          : p.textMuted)
                                      .withValues(alpha: 0.14),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  ft.config.showFirstTimerProfile
                                      ? '表示オプション: ON'
                                      : '表示オプション: OFF（デフォルト）',
                                  style: TextStyle(
                                    fontSize: 11.5,
                                    fontWeight: FontWeight.bold,
                                    color: ft.config.showFirstTimerProfile
                                        ? p.success
                                        : p.textSecondary,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                        const Divider(height: 1),
                        Expanded(
                          child: !ft.config.showFirstTimerProfile
                              ? Center(
                                  child: Padding(
                                    padding: const EdgeInsets.all(24),
                                    child: Column(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        Icon(Icons.visibility_off_outlined,
                                            size: 40, color: p.textMuted),
                                        const SizedBox(height: 10),
                                        const Text(
                                          '初見ユーザーのアイコン・名前表示は現在オフです',
                                          style: TextStyle(
                                              fontWeight: FontWeight.bold),
                                        ),
                                        const SizedBox(height: 4),
                                        Text(
                                          '左側の「初見の人のアイコンと名前を表示（オプション）」をオンにして保存すると、ここに初見さんの一覧と OBS オーバーレイ表示が有効になります。',
                                          textAlign: TextAlign.center,
                                          style: TextStyle(
                                              fontSize: 12.5,
                                              color: p.textSecondary),
                                        ),
                                      ],
                                    ),
                                  ),
                                )
                              : ft.recentUsers.isEmpty
                                  ? Center(
                                      child: Text(
                                        '今回の配信でまだ初見さんは検知されていません。',
                                        style:
                                            TextStyle(color: p.textSecondary),
                                      ),
                                    )
                                  : ListView.separated(
                                      itemCount: ft.recentUsers.length,
                                      separatorBuilder: (_, _) =>
                                          const Divider(height: 1),
                                      itemBuilder: (_, idx) {
                                        final u = ft.recentUsers[idx];
                                        return ListTile(
                                          leading: UserAvatar(
                                            name: u.userName,
                                            iconUrl: u.userIconUrl,
                                            radius: 19,
                                            borderColor: accent,
                                          ),
                                          title: Text(
                                            u.userName,
                                            style: const TextStyle(
                                                fontWeight: FontWeight.bold),
                                          ),
                                          subtitle: Text(
                                            '検知理由: ${u.triggerReason}',
                                            style: TextStyle(
                                                fontSize: 12,
                                                color: p.textSecondary),
                                          ),
                                          trailing: Text(
                                            _formatTime(u.detectedAtMs),
                                            style: TextStyle(
                                                fontSize: 12,
                                                color: p.textSecondary),
                                          ),
                                        );
                                      },
                                    ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
