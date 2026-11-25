import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';

/// 特定ワードカウンター画面（アカウント毎にn回まで / 回数上限なし）
class CountersScreen extends StatelessWidget {
  const CountersScreen({super.key, required this.controller});

  final StosupController controller;

  Future<void> _openEditor(BuildContext context,
      [WordCounterConfig? existing]) async {
    final p = context.palette;
    final nameCtrl = TextEditingController(text: existing?.name ?? '');
    final keywordsCtrl =
        TextEditingController(text: existing?.keywords.join(', ') ?? '');
    final maxPerAccCtrl = TextEditingController(
      text: (existing?.maxPerAccount ?? 1).toString(),
    );
    var hasLimit = existing?.maxPerAccount != null;
    var matchMode = existing?.matchMode ?? MatchMode.contains;
    var caseSensitive = existing?.caseSensitive ?? false;
    var excludeBanned = existing?.excludeBanned ?? true;
    var accentColor = existing?.accentColor ?? '#2563EB';
    var enabled = existing?.enabled ?? true;

    final saved = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setState) => AlertDialog(
          title: Text(existing == null ? 'ワードカウンターを作成' : 'ワードカウンターを編集'),
          content: SizedBox(
            width: 500,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  TextField(
                    controller: nameCtrl,
                    decoration: const InputDecoration(
                      labelText: 'カウンター名',
                      hintText: '例: 草カウンター / 参加表明カウント',
                    ),
                  ),
                  const SizedBox(height: 10),
                  TextField(
                    controller: keywordsCtrl,
                    decoration: const InputDecoration(
                      labelText: '対象ワード（カンマ区切りで複数指定可）',
                      hintText: '例: 草, www, ﾜﾛﾀ',
                    ),
                  ),
                  const SizedBox(height: 10),
                  DropdownButtonFormField<MatchMode>(
                    initialValue: matchMode,
                    dropdownColor: p.cardBg,
                    decoration: const InputDecoration(labelText: '一致方式'),
                    items: const [
                      DropdownMenuItem(
                          value: MatchMode.contains, child: Text('部分一致')),
                      DropdownMenuItem(
                          value: MatchMode.exact, child: Text('完全一致')),
                      DropdownMenuItem(
                          value: MatchMode.regex, child: Text('正規表現')),
                    ],
                    onChanged: (v) {
                      if (v != null) setState(() => matchMode = v);
                    },
                  ),
                  const SizedBox(height: 12),
                  const Text(
                    'アカウント毎のカウント回数制限',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 6),
                  SegmentedButton<bool>(
                    segments: const [
                      ButtonSegment(
                        value: false,
                        icon: Icon(Icons.all_inclusive, size: 17),
                        label: Text('回数上限なし'),
                      ),
                      ButtonSegment(
                        value: true,
                        icon: Icon(Icons.person_outline, size: 17),
                        label: Text('アカウント毎に n 回まで'),
                      ),
                    ],
                    selected: {hasLimit},
                    onSelectionChanged: (s) =>
                        setState(() => hasLimit = s.first),
                  ),
                  if (hasLimit) ...[
                    const SizedBox(height: 10),
                    TextField(
                      controller: maxPerAccCtrl,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(
                        labelText: '1アカウントあたりの最大カウント数 (n)',
                        hintText: '1',
                      ),
                    ),
                  ],
                  const SizedBox(height: 6),
                  SwitchListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    value: excludeBanned,
                    onChanged: (v) => setState(() => excludeBanned = v),
                    title: const Text('BAN 済みユーザーをカウントから除外する'),
                  ),
                  SwitchListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    value: caseSensitive,
                    onChanged: (v) => setState(() => caseSensitive = v),
                    title: const Text('英字の大文字・小文字を区別する'),
                  ),
                  const Divider(height: 18),
                  ColorPickerRow(
                    label: 'アクセントカラー',
                    hexValue: accentColor,
                    presets: const [
                      '#2563EB',
                      '#16A34A',
                      '#D97706',
                      '#EA580C',
                      '#DB2777',
                      '#7C3AED',
                      '#DC2626',
                    ],
                    onChanged: (v) => setState(() => accentColor = v),
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('キャンセル'),
            ),
            FilledButton(
              onPressed: () {
                if (keywordsCtrl.text.trim().isEmpty) return;
                Navigator.pop(ctx, true);
              },
              child: const Text('保存'),
            ),
          ],
        ),
      ),
    );

    if (saved == true) {
      final kws = keywordsCtrl.text
          .split(RegExp(r'[,、\n]'))
          .map((e) => e.trim())
          .where((e) => e.isNotEmpty)
          .toList();
      final maxVal = hasLimit
          ? (int.tryParse(maxPerAccCtrl.text.trim()) ?? 1).clamp(1, 999999)
          : null;

      await controller.saveWordCounter(WordCounterConfig(
        id: existing?.id ?? '',
        name: nameCtrl.text.trim().isEmpty ? kws.first : nameCtrl.text.trim(),
        keywords: kws,
        matchMode: matchMode,
        caseSensitive: caseSensitive,
        maxPerAccount: maxVal,
        excludeBanned: excludeBanned,
        accentColor: accentColor,
        enabled: enabled,
        totalCount: existing?.totalCount ?? BigInt.zero,
        uniqueUserCount: existing?.uniqueUserCount ?? BigInt.zero,
      ));
    }

    nameCtrl.dispose();
    keywordsCtrl.dispose();
    maxPerAccCtrl.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final counters = controller.wordCounters;
    final featureEnabled = controller.featureFlags.wordCounterEnabled;

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
                      '特定ワードカウンター',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '指定したワードを含むコメントを自動集計します。「アカウント毎に n 回まで」と「上限なし」をカウンターごとに選択できます。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: 'ワードカウント',
                enabled: featureEnabled,
                onChanged: (v) => controller.setFeatureEnabled(
                  wordCounter: v,
                  featureLabel: '特定ワードカウント機能',
                ),
              ),
              const SizedBox(width: 10),
              FilledButton.icon(
                onPressed: () => _openEditor(context),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('新規カウンター作成'),
              ),
            ],
          ),
          const SizedBox(height: 14),
          if (!featureEnabled)
            FeatureDisabledBanner(
              featureName: '特定ワードカウント機能',
              description:
                  'コメント受信時の自動カウント加算および OBS カウンター表示を一括停止しています。',
              onEnable: () => controller.setFeatureEnabled(
                wordCounter: true,
                featureLabel: '特定ワードカウント機能',
              ),
            ),
          Expanded(
            child: counters.isEmpty
                ? Center(
                    child: Text(
                      'カウンターがありません。「新規カウンター作成」から追加してください。',
                      style: TextStyle(color: p.textSecondary),
                    ),
                  )
                : GridView.builder(
                    gridDelegate:
                        const SliverGridDelegateWithMaxCrossAxisExtent(
                      maxCrossAxisExtent: 410,
                      mainAxisExtent: 235,
                      crossAxisSpacing: 12,
                      mainAxisSpacing: 12,
                    ),
                    itemCount: counters.length,
                    itemBuilder: (context, idx) {
                      final c = counters[idx];
                      final accent = AppTheme.parseCssHex(c.accentColor,
                          fallback: p.accent);
                      final singleUrl =
                          '${controller.overlayBaseUrl}/overlay/counter?id=${c.id}';

                      return Card(
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                          side: BorderSide(
                            color: c.enabled
                                ? accent.withValues(alpha: 0.65)
                                : p.border,
                            width: c.enabled ? 1.5 : 1,
                          ),
                        ),
                        child: Padding(
                          padding: const EdgeInsets.all(14),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  Expanded(
                                    child: Text(
                                      c.name,
                                      style: const TextStyle(
                                          fontWeight: FontWeight.bold,
                                          fontSize: 15),
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                  Switch(
                                    value: c.enabled,
                                    onChanged: (v) {
                                      controller
                                          .saveWordCounter(WordCounterConfig(
                                        id: c.id,
                                        name: c.name,
                                        keywords: c.keywords,
                                        matchMode: c.matchMode,
                                        caseSensitive: c.caseSensitive,
                                        maxPerAccount: c.maxPerAccount,
                                        excludeBanned: c.excludeBanned,
                                        accentColor: c.accentColor,
                                        enabled: v,
                                        totalCount: c.totalCount,
                                        uniqueUserCount: c.uniqueUserCount,
                                      ));
                                    },
                                  ),
                                ],
                              ),
                              Row(
                                crossAxisAlignment: CrossAxisAlignment.baseline,
                                textBaseline: TextBaseline.alphabetic,
                                children: [
                                  Text(
                                    c.totalCount.toString(),
                                    style: TextStyle(
                                      fontSize: 40,
                                      fontWeight: FontWeight.w800,
                                      color: accent,
                                    ),
                                  ),
                                  const SizedBox(width: 12),
                                  Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Container(
                                        padding: const EdgeInsets.symmetric(
                                            horizontal: 7, vertical: 2),
                                        decoration: BoxDecoration(
                                          color: accent.withValues(alpha: 0.14),
                                          borderRadius:
                                              BorderRadius.circular(4),
                                        ),
                                        child: Text(
                                          c.maxPerAccount != null
                                              ? '1アカウント ${c.maxPerAccount}回まで'
                                              : '回数上限なし',
                                          style: TextStyle(
                                              color: accent,
                                              fontSize: 11,
                                              fontWeight: FontWeight.bold),
                                        ),
                                      ),
                                      const SizedBox(height: 3),
                                      Text(
                                        '参加アカウント: ${c.uniqueUserCount}人',
                                        style: TextStyle(
                                            fontSize: 12,
                                            color: p.textSecondary),
                                      ),
                                    ],
                                  ),
                                ],
                              ),
                              const SizedBox(height: 4),
                              Text(
                                '対象ワード: ${c.keywords.join(", ")}',
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                    fontSize: 12, color: p.textSecondary),
                              ),
                              const Spacer(),
                              Row(
                                children: [
                                  IconButton.outlined(
                                    tooltip: '-1',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () =>
                                        controller.adjustWordCounter(c.id, -1),
                                    icon: const Icon(Icons.remove, size: 15),
                                  ),
                                  const SizedBox(width: 4),
                                  IconButton.outlined(
                                    tooltip: '+1',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () =>
                                        controller.adjustWordCounter(c.id, 1),
                                    icon: const Icon(Icons.add, size: 15),
                                  ),
                                  const SizedBox(width: 6),
                                  OutlinedButton(
                                    style: OutlinedButton.styleFrom(
                                      visualDensity: VisualDensity.compact,
                                    ),
                                    onPressed: () =>
                                        controller.resetWordCounter(c.id),
                                    child: const Text('リセット'),
                                  ),
                                  const Spacer(),
                                  IconButton(
                                    tooltip: 'このカウンター単体のOBS URLをコピー',
                                    onPressed: () {
                                      Clipboard.setData(
                                          ClipboardData(text: singleUrl));
                                      ScaffoldMessenger.of(context)
                                          .showSnackBar(
                                        const SnackBar(
                                          content: Text(
                                              '単体カウンター用 OBS URL をコピーしました'),
                                        ),
                                      );
                                    },
                                    icon: const Icon(Icons.link, size: 18),
                                  ),
                                  IconButton(
                                    tooltip: '編集',
                                    onPressed: () => _openEditor(context, c),
                                    icon: const Icon(Icons.edit_outlined,
                                        size: 18),
                                  ),
                                  IconButton(
                                    tooltip: '削除',
                                    onPressed: () =>
                                        controller.deleteWordCounter(c.id),
                                    icon: Icon(Icons.delete_outline,
                                        size: 18, color: p.danger),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}
