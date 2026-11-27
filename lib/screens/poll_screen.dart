import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';

class _EditablePollOption {
  _EditablePollOption({
    required this.id,
    required String label,
    required String keywordsText,
    required this.color,
  })  : labelCtrl = TextEditingController(text: label),
        keywordsCtrl = TextEditingController(text: keywordsText);

  final String id;
  final TextEditingController labelCtrl;
  final TextEditingController keywordsCtrl;
  String color;

  void dispose() {
    labelCtrl.dispose();
    keywordsCtrl.dispose();
  }
}

/// リアルタイム投票 / アンケート管理 ＆ OBS グラフ連携画面
class PollScreen extends StatefulWidget {
  const PollScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<PollScreen> createState() => _PollScreenState();
}

class _PollScreenState extends State<PollScreen> {
  static const List<String> _palette = [
    '#3B82F6',
    '#22C55E',
    '#F59E0B',
    '#EC4899',
    '#A855F7',
    '#06B6D4',
  ];

  late TextEditingController _questionCtrl;
  final List<_EditablePollOption> _options = [];
  late bool _allowVoteChange;
  late bool _excludeBanned;
  late bool _showPercentage;
  late bool _showVoteCount;
  String _lastSyncedConfigSig = '';

  String _configSignature(PollConfig cfg) {
    final opts = cfg.options
        .map((o) => '${o.id}|${o.label}|${o.keywords.join(",")}|${o.color}')
        .join(';');
    return '${cfg.question}|${cfg.allowVoteChange}|${cfg.excludeBanned}|${cfg.showPercentage}|${cfg.showVoteCount}|$opts';
  }

  void _syncFromConfig(PollConfig cfg) {
    _lastSyncedConfigSig = _configSignature(cfg);
    _questionCtrl.text = cfg.question;
    _allowVoteChange = cfg.allowVoteChange;
    _excludeBanned = cfg.excludeBanned;
    _showPercentage = cfg.showPercentage;
    _showVoteCount = cfg.showVoteCount;
    for (final o in _options) {
      o.dispose();
    }
    _options.clear();
    for (final opt in cfg.options) {
      _options.add(_EditablePollOption(
        id: opt.id,
        label: opt.label,
        keywordsText: opt.keywords.join(', '),
        color: opt.color,
      ));
    }
    if (_options.length < 2) {
      while (_options.length < 2) {
        final idx = _options.length + 1;
        _options.add(_EditablePollOption(
          id: 'opt-init-$idx',
          label: '選択肢 $idx',
          keywordsText: '$idx',
          color: _palette[(idx - 1) % _palette.length],
        ));
      }
    }
  }

  @override
  void initState() {
    super.initState();
    _questionCtrl = TextEditingController();
    _syncFromConfig(widget.controller.pollState.config);
  }

  @override
  void dispose() {
    _questionCtrl.dispose();
    for (final o in _options) {
      o.dispose();
    }
    super.dispose();
  }

  void _addOption() {
    if (_options.length >= 6) return;
    final idx = _options.length + 1;
    setState(() {
      _options.add(_EditablePollOption(
        id: 'opt-${DateTime.now().millisecondsSinceEpoch}-$idx',
        label: '選択肢 $idx',
        keywordsText: '$idx',
        color: _palette[(idx - 1) % _palette.length],
      ));
    });
  }

  void _removeOption(int index) {
    if (_options.length <= 2) return;
    setState(() {
      final removed = _options.removeAt(index);
      removed.dispose();
    });
  }

  PollConfig _buildConfig() {
    final existingCounts = {
      for (final o in widget.controller.pollState.config.options)
        o.id: o.voteCount,
    };
    return PollConfig(
      question: _questionCtrl.text.trim().isEmpty
          ? 'アンケート'
          : _questionCtrl.text.trim(),
      options: _options.asMap().entries.map((entry) {
        final idx = entry.key;
        final o = entry.value;
        final kws = o.keywordsCtrl.text
            .split(RegExp(r'[,、\n]'))
            .map((e) => e.trim())
            .where((e) => e.isNotEmpty)
            .toList();
        return PollOption(
          id: o.id,
          label: o.labelCtrl.text.trim().isEmpty
              ? '選択肢 ${idx + 1}'
              : o.labelCtrl.text.trim(),
          keywords: kws.isEmpty ? ['${idx + 1}'] : kws,
          color: o.color,
          voteCount: existingCounts[o.id] ?? BigInt.zero,
        );
      }).toList(),
      allowVoteChange: _allowVoteChange,
      excludeBanned: _excludeBanned,
      showPercentage: _showPercentage,
      showVoteCount: _showVoteCount,
    );
  }

  void _copyOverlayUrl() {
    final url = '${widget.controller.overlayBaseUrl}/overlay/poll';
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('ライブアンケートの OBS 用 URL をコピーしました')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final poll = ctrl.pollState;
    final sig = _configSignature(poll.config);
    if (sig != _lastSyncedConfigSig) {
      _syncFromConfig(poll.config);
    }
    final enabled = ctrl.featureFlags.pollEnabled;
    final total = poll.totalVotes.toInt();
    final maxVotes = poll.config.options.fold<int>(
      0,
      (prev, e) => e.voteCount.toInt() > prev ? e.voteCount.toInt() : prev,
    );

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
                      'ライブアンケート（リアルタイム投票）',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '視聴者が「1」「2」やキーワードをコメントするとリアルタイム集計し、OBS オーバーレイに棒グラフ表示します。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: 'ライブアンケート',
                enabled: enabled,
                onChanged: (v) => ctrl.setFeatureEnabled(
                  poll: v,
                  featureLabel: 'ライブアンケート',
                ),
              ),
              const SizedBox(width: 8),
              OutlinedButton.icon(
                onPressed: _copyOverlayUrl,
                icon: const Icon(Icons.copy, size: 15),
                label: const Text('OBS URL'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (!enabled)
            FeatureDisabledBanner(
              featureName: 'ライブアンケート',
              description:
                  'コメントからの投票集計と OBS ライブアンケート表示を一時停止しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                poll: true,
                featureLabel: 'ライブアンケート',
              ),
            ),

          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 左：投票設定・操作
                Expanded(
                  flex: 6,
                  child: SingleChildScrollView(
                    child: Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                const Expanded(
                                  child: Text(
                                    '質問 ＆ 選択肢の設定',
                                    style: TextStyle(
                                        fontSize: 15.5,
                                        fontWeight: FontWeight.bold),
                                  ),
                                ),
                                OutlinedButton.icon(
                                  onPressed: _options.length >= 6
                                      ? null
                                      : _addOption,
                                  icon: const Icon(Icons.add, size: 16),
                                  label: const Text('選択肢を追加'),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            TextField(
                              controller: _questionCtrl,
                              decoration: const InputDecoration(
                                labelText: 'アンケートの質問文',
                                hintText: '例: 次のゲームはどれにする？',
                              ),
                            ),
                            const SizedBox(height: 12),
                            ..._options.asMap().entries.map((entry) {
                              final idx = entry.key;
                              final opt = entry.value;
                              final c = AppTheme.parseCssHex(opt.color,
                                  fallback: p.accent);
                              return Padding(
                                padding: const EdgeInsets.only(bottom: 8),
                                child: Row(
                                  children: [
                                    InkWell(
                                      onTap: () {
                                        final cur = _palette.indexOf(opt.color);
                                        setState(() {
                                          opt.color = _palette[
                                              (cur + 1) % _palette.length];
                                        });
                                      },
                                      borderRadius: BorderRadius.circular(6),
                                      child: Container(
                                        width: 30,
                                        height: 30,
                                        alignment: Alignment.center,
                                        decoration: BoxDecoration(
                                          color: c,
                                          borderRadius:
                                              BorderRadius.circular(6),
                                        ),
                                        child: Text(
                                          '${idx + 1}',
                                          style: const TextStyle(
                                            color: Colors.white,
                                            fontWeight: FontWeight.bold,
                                          ),
                                        ),
                                      ),
                                    ),
                                    const SizedBox(width: 8),
                                    Expanded(
                                      flex: 3,
                                      child: TextField(
                                        controller: opt.labelCtrl,
                                        decoration: InputDecoration(
                                          labelText: '選択肢 ${idx + 1} の表示名',
                                        ),
                                      ),
                                    ),
                                    const SizedBox(width: 8),
                                    Expanded(
                                      flex: 2,
                                      child: TextField(
                                        controller: opt.keywordsCtrl,
                                        decoration: const InputDecoration(
                                          labelText: '反応ワード（カンマ区切り）',
                                          hintText: '例: 1, 雑談',
                                        ),
                                      ),
                                    ),
                                    const SizedBox(width: 4),
                                    IconButton(
                                      tooltip: '選択肢を削除',
                                      onPressed: _options.length <= 2
                                          ? null
                                          : () => _removeOption(idx),
                                      icon: Icon(
                                        Icons.close,
                                        size: 18,
                                        color: _options.length <= 2
                                            ? p.textMuted
                                            : p.danger,
                                      ),
                                    ),
                                  ],
                                ),
                              );
                            }),
                            const Divider(height: 20),
                            Wrap(
                              spacing: 12,
                              runSpacing: 4,
                              children: [
                                SizedBox(
                                  width: 240,
                                  child: SwitchListTile(
                                    dense: true,
                                    contentPadding: EdgeInsets.zero,
                                    title: const Text('投票の変更を許可する'),
                                    value: _allowVoteChange,
                                    onChanged: (v) =>
                                        setState(() => _allowVoteChange = v),
                                  ),
                                ),
                                SizedBox(
                                  width: 240,
                                  child: SwitchListTile(
                                    dense: true,
                                    contentPadding: EdgeInsets.zero,
                                    title: const Text('BAN 済みユーザーを除外'),
                                    value: _excludeBanned,
                                    onChanged: (v) =>
                                        setState(() => _excludeBanned = v),
                                  ),
                                ),
                                SizedBox(
                                  width: 240,
                                  child: SwitchListTile(
                                    dense: true,
                                    contentPadding: EdgeInsets.zero,
                                    title: const Text('割合（%）を表示'),
                                    value: _showPercentage,
                                    onChanged: (v) =>
                                        setState(() => _showPercentage = v),
                                  ),
                                ),
                                SizedBox(
                                  width: 240,
                                  child: SwitchListTile(
                                    dense: true,
                                    contentPadding: EdgeInsets.zero,
                                    title: const Text('票数を表示'),
                                    value: _showVoteCount,
                                    onChanged: (v) =>
                                        setState(() => _showVoteCount = v),
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            Wrap(
                              spacing: 8,
                              runSpacing: 8,
                              children: [
                                FilledButton.icon(
                                  onPressed: enabled
                                      ? () => ctrl.startPoll(_buildConfig())
                                      : null,
                                  icon: const Icon(Icons.play_arrow, size: 18),
                                  label: Text(
                                      poll.isActive ? '新しく投票を再開始' : '投票を開始'),
                                ),
                                if (poll.isActive)
                                  FilledButton.icon(
                                    style: FilledButton.styleFrom(
                                      backgroundColor: p.danger,
                                      foregroundColor: Colors.white,
                                    ),
                                    onPressed: ctrl.stopPoll,
                                    icon: const Icon(Icons.stop, size: 18),
                                    label: const Text('受付を締め切る'),
                                  ),
                                OutlinedButton.icon(
                                  onPressed: () =>
                                      ctrl.savePollConfig(_buildConfig()),
                                  icon: const Icon(Icons.save_outlined, size: 16),
                                  label: const Text('設定を保存'),
                                ),
                                OutlinedButton.icon(
                                  onPressed: ctrl.resetPoll,
                                  icon: const Icon(Icons.refresh, size: 16),
                                  label: const Text('票をリセット'),
                                ),
                                TextButton.icon(
                                  onPressed: ctrl.clearOverlayPoll,
                                  icon: const Icon(Icons.visibility_off_outlined,
                                      size: 16),
                                  label: const Text('OBS 表示を消す'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 14),

                // 右：リアルタイム集計結果
                Expanded(
                  flex: 5,
                  child: Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 8, vertical: 3),
                                decoration: BoxDecoration(
                                  color: (poll.isActive
                                          ? p.success
                                          : p.textMuted)
                                      .withValues(alpha: 0.16),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  poll.isActive ? '受付中' : '停止中',
                                  style: TextStyle(
                                    fontSize: 11.5,
                                    fontWeight: FontWeight.bold,
                                    color: poll.isActive
                                        ? p.success
                                        : p.textSecondary,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 10),
                              Expanded(
                                child: Text(
                                  poll.config.question,
                                  style: const TextStyle(
                                    fontSize: 16,
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                              ),
                              Text(
                                '合計 $total 票',
                                style: TextStyle(
                                  fontWeight: FontWeight.bold,
                                  color: p.accent,
                                ),
                              ),
                            ],
                          ),
                          const Divider(height: 22),
                          Expanded(
                            child: ListView.separated(
                              itemCount: poll.config.options.length,
                              separatorBuilder: (_, _) =>
                                  const SizedBox(height: 12),
                              itemBuilder: (context, idx) {
                                final opt = poll.config.options[idx];
                                final votes = opt.voteCount.toInt();
                                final ratio = total > 0 ? votes / total : 0.0;
                                final pct = (ratio * 100).round();
                                final color = AppTheme.parseCssHex(opt.color,
                                    fallback: p.accent);
                                final isLeader =
                                    maxVotes > 0 && votes == maxVotes;

                                return Container(
                                  padding: const EdgeInsets.all(12),
                                  decoration: BoxDecoration(
                                    color: p.subtleBg.withValues(alpha: 0.5),
                                    borderRadius: BorderRadius.circular(8),
                                    border: Border.all(
                                      color: isLeader ? color : p.border,
                                      width: isLeader ? 1.8 : 1,
                                    ),
                                  ),
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Row(
                                        children: [
                                          Container(
                                            width: 24,
                                            height: 24,
                                            alignment: Alignment.center,
                                            decoration: BoxDecoration(
                                              color: color,
                                              borderRadius:
                                                  BorderRadius.circular(5),
                                            ),
                                            child: Text(
                                              '${idx + 1}',
                                              style: const TextStyle(
                                                color: Colors.white,
                                                fontWeight: FontWeight.bold,
                                                fontSize: 12,
                                              ),
                                            ),
                                          ),
                                          const SizedBox(width: 8),
                                          Expanded(
                                            child: Text(
                                              opt.label,
                                              style: const TextStyle(
                                                fontWeight: FontWeight.bold,
                                                fontSize: 14.5,
                                              ),
                                            ),
                                          ),
                                          Text(
                                            '$pct% ($votes票)',
                                            style: TextStyle(
                                              fontWeight: FontWeight.w800,
                                              color: color,
                                              fontSize: 14,
                                            ),
                                          ),
                                        ],
                                      ),
                                      const SizedBox(height: 8),
                                      TweenAnimationBuilder<double>(
                                        tween: Tween<double>(end: ratio),
                                        duration:
                                            const Duration(milliseconds: 450),
                                        curve: Curves.easeOutCubic,
                                        builder: (context, animatedRatio, _) =>
                                            ClipRRect(
                                          borderRadius:
                                              BorderRadius.circular(5),
                                          child: LinearProgressIndicator(
                                            value: animatedRatio,
                                            minHeight: 10,
                                            backgroundColor: p.border,
                                            color: color,
                                          ),
                                        ),
                                      ),
                                      const SizedBox(height: 5),
                                      Text(
                                        '反応ワード: ${opt.keywords.join(", ")}',
                                        style: TextStyle(
                                          fontSize: 11.5,
                                          color: p.textSecondary,
                                        ),
                                      ),
                                    ],
                                  ),
                                );
                              },
                            ),
                          ),
                          const Divider(height: 18),
                          Row(
                            children: [
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                  color: p.accent.withValues(alpha: 0.16),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  poll.isActive ? '投票方法' : '案内',
                                  style: TextStyle(
                                    fontSize: 11,
                                    fontWeight: FontWeight.bold,
                                    color: p.accent,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  poll.isActive
                                      ? 'コメントに番号（1〜${poll.config.options.length}）または反応ワードを入力して投票できます（1人1票・${poll.config.allowVoteChange ? "再コメントで変更可" : "変更不可"}）'
                                      : '投票は停止中です。「投票を開始」を押すとコメントから自動集計が始まります。',
                                  style: TextStyle(
                                    fontSize: 12,
                                    color: p.textSecondary,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
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
