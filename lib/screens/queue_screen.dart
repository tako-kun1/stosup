import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

/// 参加型キュー（順番待ちリスト）管理 ＆ OBS オーバーレイ連携画面
class QueueScreen extends StatefulWidget {
  const QueueScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<QueueScreen> createState() => _QueueScreenState();
}

class _QueueScreenState extends State<QueueScreen> {
  late TextEditingController _joinKwCtrl;
  late TextEditingController _leaveKwCtrl;
  late int _maxSlots;
  late QueueSortMode _sortMode;
  late bool _excludeBanned;
  late bool _allowRejoin;

  @override
  void initState() {
    super.initState();
    final cfg = widget.controller.queueState.config;
    _joinKwCtrl = TextEditingController(text: cfg.joinKeywords.join(', '));
    _leaveKwCtrl = TextEditingController(text: cfg.leaveKeywords.join(', '));
    _maxSlots = cfg.maxActiveSlots;
    _sortMode = cfg.sortMode;
    _excludeBanned = cfg.excludeBanned;
    _allowRejoin = cfg.allowRejoinAfterComplete;
  }

  @override
  void dispose() {
    _joinKwCtrl.dispose();
    _leaveKwCtrl.dispose();
    super.dispose();
  }

  QueueConfig _buildConfig({bool? isAccepting}) {
    final joinKws = _joinKwCtrl.text
        .split(RegExp(r'[,、\n]'))
        .map((e) => e.trim())
        .where((e) => e.isNotEmpty)
        .toList();
    final leaveKws = _leaveKwCtrl.text
        .split(RegExp(r'[,、\n]'))
        .map((e) => e.trim())
        .where((e) => e.isNotEmpty)
        .toList();
    return QueueConfig(
      isAccepting: isAccepting ?? widget.controller.queueState.config.isAccepting,
      maxActiveSlots: _maxSlots,
      joinKeywords: joinKws.isEmpty ? const ['参加希望', '!join'] : joinKws,
      leaveKeywords: leaveKws.isEmpty ? const ['辞退', 'キャンセル'] : leaveKws,
      sortMode: _sortMode,
      excludeBanned: _excludeBanned,
      allowRejoinAfterComplete: _allowRejoin,
    );
  }

  void _copyOverlayUrl() {
    final url = '${widget.controller.overlayBaseUrl}/overlay/queue';
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('参加型キューの OBS 用 URL をコピーしました')),
    );
  }

  Future<void> _openManualAddDialog() async {
    await showDialog<void>(
      context: context,
      builder: (_) => _ManualAddQueueDialog(
        onAdd: (name, note) => widget.controller.queueAddManual(name, note),
      ),
    );
  }

  Future<void> _confirmReset() async {
    final resetCounts = await showDialog<bool>(
      context: context,
      builder: (_) => const _ResetQueueDialog(),
    );
    if (resetCounts != null) {
      await widget.controller.queueReset(resetPlayCounts: resetCounts);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final enabled = ctrl.featureFlags.queueEnabled;
    final qs = ctrl.queueState;
    final active =
        qs.entries.where((e) => e.status == QueueEntryStatus.active).toList();
    final waiting =
        qs.entries.where((e) => e.status == QueueEntryStatus.waiting).toList();
    final completed = qs.entries
        .where((e) => e.status == QueueEntryStatus.completed)
        .toList();

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
                      '参加型キュー（順番待ちリスト）管理',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '視聴者の「参加希望」「辞退」コメントで待機列を自動管理し、初見優先・参加回数カウント・一括交代ができます。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: '参加型キュー',
                enabled: enabled,
                onChanged: (v) => ctrl.setFeatureEnabled(
                  queue: v,
                  featureLabel: '参加型キュー管理',
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
              featureName: '参加型キュー管理',
              description:
                  'コメントからの自動登録および OBS 参加型キューオーバーレイを一時停止しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                queue: true,
                featureLabel: '参加型キュー管理',
              ),
            ),

          // 受付・設定コントロールカード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Wrap(
                    spacing: 10,
                    runSpacing: 8,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      FilledButton.icon(
                        style: FilledButton.styleFrom(
                          backgroundColor: qs.config.isAccepting
                              ? p.danger
                              : p.success,
                          foregroundColor: Colors.white,
                        ),
                        onPressed: enabled
                            ? () => ctrl.saveQueueConfig(
                                _buildConfig(isAccepting: !qs.config.isAccepting))
                            : null,
                        icon: Icon(
                          qs.config.isAccepting
                              ? Icons.pause_circle_outline
                              : Icons.play_circle_outline,
                          size: 17,
                        ),
                        label: Text(qs.config.isAccepting
                            ? 'コメント自動受付を停止'
                            : 'コメント自動受付を開始'),
                      ),
                      FilledButton.icon(
                        onPressed: waiting.isEmpty ? null : ctrl.queuePromoteNext,
                        icon: const Icon(Icons.arrow_upward, size: 16),
                        label: const Text('空き枠へ繰り上げ'),
                      ),
                      FilledButton.tonalIcon(
                        onPressed: (active.isEmpty && waiting.isEmpty)
                            ? null
                            : ctrl.queueRotateActive,
                        icon: const Icon(Icons.swap_horiz, size: 17),
                        label: const Text('参加者を交代（全員完了→次へ）'),
                      ),
                      OutlinedButton.icon(
                        onPressed: _openManualAddDialog,
                        icon: const Icon(Icons.person_add_alt, size: 16),
                        label: const Text('手動追加'),
                      ),
                      TextButton.icon(
                        style: TextButton.styleFrom(foregroundColor: p.danger),
                        onPressed: qs.entries.isEmpty ? null : _confirmReset,
                        icon: const Icon(Icons.delete_sweep_outlined, size: 16),
                        label: const Text('リセット'),
                      ),
                    ],
                  ),
                  const Divider(height: 20),
                  Row(
                    children: [
                      SizedBox(
                        width: 130,
                        child: DropdownButtonFormField<int>(
                          initialValue: _maxSlots,
                          isExpanded: true,
                          decoration: const InputDecoration(labelText: '同時参加枠'),
                          items: List.generate(
                            16,
                            (i) => DropdownMenuItem(
                              value: i + 1,
                              child: Text('${i + 1} 人'),
                            ),
                          ),
                          onChanged: (v) {
                            if (v != null) setState(() => _maxSlots = v);
                          },
                        ),
                      ),
                      const SizedBox(width: 10),
                      SizedBox(
                        width: 250,
                        child: DropdownButtonFormField<QueueSortMode>(
                          initialValue: _sortMode,
                          isExpanded: true,
                          decoration: const InputDecoration(labelText: '待機列の優先順'),
                          items: const [
                            DropdownMenuItem(
                              value: QueueSortMode.firstCome,
                              child: Text('先着順',
                                  overflow: TextOverflow.ellipsis),
                            ),
                            DropdownMenuItem(
                              value: QueueSortMode.fewestPlaysFirst,
                              child: Text('参加回数が少ない人優先',
                                  overflow: TextOverflow.ellipsis),
                            ),
                            DropdownMenuItem(
                              value: QueueSortMode.firstTimerFirst,
                              child: Text('初見さん優先 → 回数少ない順',
                                  overflow: TextOverflow.ellipsis),
                            ),
                          ],
                          onChanged: (v) {
                            if (v != null) setState(() => _sortMode = v);
                          },
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: TextField(
                          controller: _joinKwCtrl,
                          decoration: const InputDecoration(
                            labelText: '参加キーワード（カンマ区切り）',
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: TextField(
                          controller: _leaveKwCtrl,
                          decoration: const InputDecoration(
                            labelText: '辞退キーワード（カンマ区切り）',
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      OutlinedButton.icon(
                        onPressed: () => ctrl.saveQueueConfig(_buildConfig()),
                        icon: const Icon(Icons.save_outlined, size: 16),
                        label: const Text('設定適用'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      SizedBox(
                        width: 240,
                        child: CheckboxListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          controlAffinity: ListTileControlAffinity.leading,
                          value: _allowRejoin,
                          onChanged: (v) =>
                              setState(() => _allowRejoin = v ?? true),
                          title: const Text('完了後の再参加を許可',
                              style: TextStyle(fontSize: 12.5)),
                        ),
                      ),
                      SizedBox(
                        width: 240,
                        child: CheckboxListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          controlAffinity: ListTileControlAffinity.leading,
                          value: _excludeBanned,
                          onChanged: (v) =>
                              setState(() => _excludeBanned = v ?? true),
                          title: const Text('BAN 済みユーザーを除外',
                              style: TextStyle(fontSize: 12.5)),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),

          // 参加中 & 待機中リスト
          Expanded(
            child: Row(
              children: [
                // 参加中リスト
                Expanded(
                  child: Card(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Padding(
                          padding: const EdgeInsets.fromLTRB(14, 12, 14, 8),
                          child: Row(
                            children: [
                              Icon(Icons.sports_esports,
                                  color: p.success, size: 18),
                              const SizedBox(width: 8),
                              Text(
                                '参加中 (${active.length} / ${qs.config.maxActiveSlots})',
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold, fontSize: 15),
                              ),
                            ],
                          ),
                        ),
                        const Divider(height: 1),
                        Expanded(
                          child: active.isEmpty
                              ? Center(
                                  child: Text(
                                    '参加中のメンバーはいません',
                                    style: TextStyle(color: p.textSecondary),
                                  ),
                                )
                              : ListView.separated(
                                  itemCount: active.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (context, idx) {
                                    final e = active[idx];
                                    return ListTile(
                                      leading: UserAvatar(
                                        name: e.userName,
                                        iconUrl: e.userIconUrl,
                                        radius: 17,
                                        borderColor: p.success,
                                      ),
                                      title: Row(
                                        children: [
                                          Expanded(
                                            child: Text(
                                              e.userName,
                                              style: const TextStyle(
                                                  fontWeight: FontWeight.bold),
                                              overflow: TextOverflow.ellipsis,
                                            ),
                                          ),
                                          if (e.isFirstTime)
                                            _Tag('初見', p.success),
                                          const SizedBox(width: 4),
                                          _Tag('参加${e.playCount}回', p.accent),
                                        ],
                                      ),
                                      subtitle: e.note.isNotEmpty
                                          ? Text(e.note,
                                              style: TextStyle(
                                                  fontSize: 12,
                                                  color: p.textSecondary))
                                          : null,
                                      trailing: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          FilledButton.tonal(
                                            onPressed: () => ctrl.queueSetStatus(
                                              e.userId,
                                              QueueEntryStatus.completed,
                                            ),
                                            child: const Text('完了'),
                                          ),
                                          IconButton(
                                            tooltip: '待機列へ戻す',
                                            onPressed: () => ctrl.queueSetStatus(
                                              e.userId,
                                              QueueEntryStatus.waiting,
                                            ),
                                            icon: const Icon(Icons.undo,
                                                size: 17),
                                          ),
                                          IconButton(
                                            tooltip: '削除',
                                            onPressed: () =>
                                                ctrl.queueRemoveEntry(e.userId),
                                            icon: Icon(Icons.close,
                                                size: 17, color: p.danger),
                                          ),
                                        ],
                                      ),
                                    );
                                  },
                                ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 12),

                // 待機中リスト（+ 完了済み数表示）
                Expanded(
                  child: Card(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Padding(
                          padding: const EdgeInsets.fromLTRB(14, 12, 14, 8),
                          child: Row(
                            children: [
                              Icon(Icons.format_list_numbered,
                                  color: p.accent, size: 18),
                              const SizedBox(width: 8),
                              Text(
                                '順番待ちリスト (${waiting.length}人)',
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold, fontSize: 15),
                              ),
                              const Spacer(),
                              if (completed.isNotEmpty)
                                Text(
                                  '参加済み: ${completed.length}人',
                                  style: TextStyle(
                                      fontSize: 12, color: p.textSecondary),
                                ),
                            ],
                          ),
                        ),
                        const Divider(height: 1),
                        Expanded(
                          child: waiting.isEmpty
                              ? Center(
                                  child: Text(
                                    '待機中のユーザーはいません',
                                    style: TextStyle(color: p.textSecondary),
                                  ),
                                )
                              : ListView.separated(
                                  itemCount: waiting.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (context, idx) {
                                    final e = waiting[idx];
                                    return ListTile(
                                      leading: CircleAvatar(
                                        radius: 15,
                                        backgroundColor: p.subtleBg,
                                        child: Text(
                                          '${idx + 1}',
                                          style: TextStyle(
                                            fontWeight: FontWeight.bold,
                                            color: p.textPrimary,
                                            fontSize: 12.5,
                                          ),
                                        ),
                                      ),
                                      title: Row(
                                        children: [
                                          UserAvatar(
                                            name: e.userName,
                                            iconUrl: e.userIconUrl,
                                            radius: 13,
                                          ),
                                          const SizedBox(width: 8),
                                          Expanded(
                                            child: Text(
                                              e.userName,
                                              style: const TextStyle(
                                                  fontWeight: FontWeight.w600),
                                              overflow: TextOverflow.ellipsis,
                                            ),
                                          ),
                                          if (e.isFirstTime)
                                            _Tag('初見', p.success),
                                          const SizedBox(width: 4),
                                          _Tag('${e.playCount}回', p.textSecondary),
                                        ],
                                      ),
                                      subtitle: e.note.isNotEmpty
                                          ? Text(e.note,
                                              style: TextStyle(
                                                  fontSize: 12,
                                                  color: p.textSecondary))
                                          : null,
                                      trailing: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          IconButton(
                                            tooltip: '上へ移動',
                                            onPressed: idx == 0
                                                ? null
                                                : () => ctrl.queueMoveEntry(
                                                    e.userId, -1),
                                            icon: const Icon(
                                                Icons.keyboard_arrow_up,
                                                size: 18),
                                          ),
                                          IconButton(
                                            tooltip: '下へ移動',
                                            onPressed: idx == waiting.length - 1
                                                ? null
                                                : () => ctrl.queueMoveEntry(
                                                    e.userId, 1),
                                            icon: const Icon(
                                                Icons.keyboard_arrow_down,
                                                size: 18),
                                          ),
                                          FilledButton(
                                            style: FilledButton.styleFrom(
                                              visualDensity:
                                                  VisualDensity.compact,
                                            ),
                                            onPressed: () => ctrl.queueSetStatus(
                                              e.userId,
                                              QueueEntryStatus.active,
                                            ),
                                            child: const Text('参加へ'),
                                          ),
                                          IconButton(
                                            tooltip: '削除',
                                            onPressed: () =>
                                                ctrl.queueRemoveEntry(e.userId),
                                            icon: Icon(Icons.close,
                                                size: 17, color: p.danger),
                                          ),
                                        ],
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

class _Tag extends StatelessWidget {
  const _Tag(this.text, this.color);
  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.16),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: TextStyle(
          fontSize: 10.5,
          fontWeight: FontWeight.bold,
          color: color,
        ),
      ),
    );
  }
}

class _ManualAddQueueDialog extends StatefulWidget {
  const _ManualAddQueueDialog({required this.onAdd});

  final Future<void> Function(String name, String note) onAdd;

  @override
  State<_ManualAddQueueDialog> createState() => _ManualAddQueueDialogState();
}

class _ManualAddQueueDialogState extends State<_ManualAddQueueDialog> {
  final _nameCtrl = TextEditingController();
  final _noteCtrl = TextEditingController();

  @override
  void dispose() {
    _nameCtrl.dispose();
    _noteCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('順番待ちリストへ手動追加'),
      content: SizedBox(
        width: 380,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: _nameCtrl,
              autofocus: true,
              decoration: const InputDecoration(
                labelText: '参加者名',
                hintText: '例: たこやき太郎',
              ),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _noteCtrl,
              decoration: const InputDecoration(
                labelText: 'メモ / ゲームID（任意）',
                hintText: '例: フレンドコード・キャラ名など',
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('キャンセル'),
        ),
        FilledButton(
          onPressed: () async {
            if (_nameCtrl.text.trim().isEmpty) return;
            final nav = Navigator.of(context);
            await widget.onAdd(_nameCtrl.text.trim(), _noteCtrl.text.trim());
            nav.pop();
          },
          child: const Text('追加する'),
        ),
      ],
    );
  }
}

class _ResetQueueDialog extends StatefulWidget {
  const _ResetQueueDialog();

  @override
  State<_ResetQueueDialog> createState() => _ResetQueueDialogState();
}

class _ResetQueueDialogState extends State<_ResetQueueDialog> {
  bool _resetPlayCounts = false;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return AlertDialog(
      title: const Text('参加型キューをリセットしますか？'),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '現在の参加中・待機中・完了済みリストをすべてクリアします。',
              style: TextStyle(fontSize: 13, color: p.textSecondary),
            ),
            const SizedBox(height: 10),
            Material(
              color: p.subtleBg,
              borderRadius: BorderRadius.circular(8),
              child: CheckboxListTile(
                value: _resetPlayCounts,
                onChanged: (v) => setState(() => _resetPlayCounts = v ?? false),
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('ユーザー毎の参加回数履歴も 0 に戻す'),
                subtitle: const Text('チェックを外すと次回並んだ際もこれまでの参加回数が引き継がれます'),
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, null),
          child: const Text('キャンセル'),
        ),
        FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: p.danger,
            foregroundColor: Colors.white,
          ),
          onPressed: () => Navigator.pop(context, _resetPlayCounts),
          child: const Text('リセットを実行'),
        ),
      ],
    );
  }
}
