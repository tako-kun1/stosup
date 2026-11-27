import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';

/// 特定ワード反応によるコメント吹き出し背景色の変更ルール設定画面
class HighlightRulesScreen extends StatelessWidget {
  const HighlightRulesScreen({super.key, required this.controller});

  final StosupController controller;

  String _modeLabel(MatchMode mode) {
    switch (mode) {
      case MatchMode.contains:
        return '部分一致';
      case MatchMode.exact:
        return '完全一致';
      case MatchMode.regex:
        return '正規表現';
    }
  }

  Future<void> _openEditor(BuildContext context,
      [HighlightRule? existing]) async {
    final p = context.palette;
    final nameCtrl = TextEditingController(text: existing?.name ?? '');
    final patternCtrl = TextEditingController(text: existing?.pattern ?? '');
    final priorityCtrl =
        TextEditingController(text: (existing?.priority ?? 10).toString());
    var matchMode = existing?.matchMode ?? MatchMode.contains;
    var caseSensitive = existing?.caseSensitive ?? false;
    var bgColor = existing?.bgColor ?? '#065F46E6';
    var textColor = existing?.textColor ?? '#F4F4F5';
    var borderColor = existing?.borderColor ?? '#16A34A';
    var enabled = existing?.enabled ?? true;

    final saved = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setState) {
          final bg = AppTheme.parseCssHex(bgColor);
          final fg = AppTheme.parseCssHex(textColor, fallback: Colors.white);
          final bd = AppTheme.parseCssHex(borderColor, fallback: p.border);

          return AlertDialog(
            title: Text(existing == null ? 'ハイライトサインを追加' : 'ハイライトサインを編集'),
            content: SizedBox(
              width: 540,
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('吹き出しプレビュー',
                        style: TextStyle(fontSize: 12, color: p.textSecondary)),
                    const SizedBox(height: 6),
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(
                          horizontal: 14, vertical: 10),
                      decoration: BoxDecoration(
                        color: bg,
                        borderRadius: const BorderRadius.only(
                          topLeft: Radius.circular(3),
                          topRight: Radius.circular(8),
                          bottomLeft: Radius.circular(8),
                          bottomRight: Radius.circular(8),
                        ),
                        border: Border.all(color: bd, width: 1.8),
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'サンプル視聴者',
                            style: TextStyle(
                              color: fg.withValues(alpha: 0.85),
                              fontSize: 12,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          const SizedBox(height: 3),
                          Text(
                            patternCtrl.text.isEmpty
                                ? '特定のワードを含むコメントの表示イメージです！'
                                : '「${patternCtrl.text}」に反応したコメントはこのように表示されます！',
                            style: TextStyle(color: fg, fontSize: 14),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 14),
                    TextField(
                      controller: nameCtrl,
                      decoration: const InputDecoration(
                        labelText: 'ルール名',
                        hintText: '例: 初見さん歓迎 / お祝いコメント',
                      ),
                    ),
                    const SizedBox(height: 10),
                    TextField(
                      controller: patternCtrl,
                      onChanged: (_) => setState(() {}),
                      decoration: const InputDecoration(
                        labelText: '反応するワード / パターン',
                        hintText: '例: 初見',
                      ),
                    ),
                    const SizedBox(height: 10),
                    Row(
                      children: [
                        Expanded(
                          child: DropdownButtonFormField<MatchMode>(
                            initialValue: matchMode,
                            dropdownColor: p.cardBg,
                            decoration:
                                const InputDecoration(labelText: '一致方式'),
                            items: const [
                              DropdownMenuItem(
                                  value: MatchMode.contains,
                                  child: Text('部分一致')),
                              DropdownMenuItem(
                                  value: MatchMode.exact, child: Text('完全一致')),
                              DropdownMenuItem(
                                  value: MatchMode.regex, child: Text('正規表現')),
                            ],
                            onChanged: (v) {
                              if (v != null) setState(() => matchMode = v);
                            },
                          ),
                        ),
                        const SizedBox(width: 10),
                        SizedBox(
                          width: 130,
                          child: TextField(
                            controller: priorityCtrl,
                            keyboardType: TextInputType.number,
                            decoration: const InputDecoration(
                              labelText: '優先度（大が優先）',
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    SwitchListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      value: caseSensitive,
                      onChanged: (v) => setState(() => caseSensitive = v),
                      title: const Text('英字の大文字・小文字を区別する'),
                    ),
                    const Divider(height: 20),
                    ColorPickerRow(
                      label: '吹き出し背景色',
                      hexValue: bgColor,
                      onChanged: (v) => setState(() => bgColor = v),
                    ),
                    const SizedBox(height: 12),
                    ColorPickerRow(
                      label: '文字色',
                      hexValue: textColor,
                      presets: const [
                        '#F4F4F5',
                        '#ECFDF5',
                        '#FFF7ED',
                        '#FEF9C3',
                        '#FCE7F3',
                        '#18181B',
                      ],
                      onChanged: (v) => setState(() => textColor = v),
                    ),
                    const SizedBox(height: 12),
                    ColorPickerRow(
                      label: '枠線色',
                      hexValue: borderColor,
                      onChanged: (v) => setState(() => borderColor = v),
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
                  if (patternCtrl.text.trim().isEmpty) return;
                  Navigator.pop(ctx, true);
                },
                child: const Text('保存'),
              ),
            ],
          );
        },
      ),
    );

    if (saved == true) {
      final rule = HighlightRule(
        id: existing?.id ?? '',
        name: nameCtrl.text.trim().isEmpty
            ? patternCtrl.text.trim()
            : nameCtrl.text.trim(),
        pattern: patternCtrl.text.trim(),
        matchMode: matchMode,
        caseSensitive: caseSensitive,
        bgColor: bgColor,
        textColor: textColor,
        borderColor: borderColor,
        priority: int.tryParse(priorityCtrl.text.trim()) ?? 0,
        enabled: enabled,
      );
      await controller.saveHighlightRule(rule);
    }

    nameCtrl.dispose();
    patternCtrl.dispose();
    priorityCtrl.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final rules = controller.highlightRules;
    final featureEnabled = controller.featureFlags.wordHighlightEnabled;

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
                      'ハイライトサイン（吹き出し色反応）',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '設定したワードを含むコメントの吹き出し背景色・文字色・枠線色をアプリ内と OBS オーバーレイで自動変更します。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: 'ハイライトサイン',
                enabled: featureEnabled,
                onChanged: (v) => controller.setFeatureEnabled(
                  wordHighlight: v,
                  featureLabel: 'ハイライトサイン',
                ),
              ),
              const SizedBox(width: 10),
              FilledButton.icon(
                onPressed: () => _openEditor(context),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('新規ルール追加'),
              ),
            ],
          ),
          const SizedBox(height: 14),
          if (!featureEnabled)
            FeatureDisabledBanner(
              featureName: 'ハイライトサイン',
              description:
                  '個別ルールが有効でも、すべてのコメントを通常の吹き出し色で表示します。',
              onEnable: () => controller.setFeatureEnabled(
                wordHighlight: true,
                featureLabel: 'ハイライトサイン',
              ),
            ),
          Expanded(
            child: rules.isEmpty
                ? Center(
                    child: Text(
                      'ルールが登録されていません。「新規ルール追加」から作成できます。',
                      style: TextStyle(color: p.textSecondary),
                    ),
                  )
                : ListView.builder(
                    itemCount: rules.length,
                    itemBuilder: (context, idx) {
                      final r = rules[idx];
                      final bg = AppTheme.parseCssHex(r.bgColor);
                      final fg = AppTheme.parseCssHex(r.textColor,
                          fallback: Colors.white);
                      final bd = AppTheme.parseCssHex(r.borderColor,
                          fallback: p.border);

                      return Padding(
                        padding: const EdgeInsets.only(bottom: 10),
                        child: Card(
                          child: Padding(
                            padding: const EdgeInsets.all(12),
                            child: Row(
                              children: [
                                Switch(
                                  value: r.enabled,
                                  onChanged: (v) {
                                    controller.saveHighlightRule(HighlightRule(
                                      id: r.id,
                                      name: r.name,
                                      pattern: r.pattern,
                                      matchMode: r.matchMode,
                                      caseSensitive: r.caseSensitive,
                                      bgColor: r.bgColor,
                                      textColor: r.textColor,
                                      borderColor: r.borderColor,
                                      priority: r.priority,
                                      enabled: v,
                                    ));
                                  },
                                ),
                                const SizedBox(width: 10),
                                Expanded(
                                  flex: 2,
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Text(
                                        r.name,
                                        style: const TextStyle(
                                            fontWeight: FontWeight.bold,
                                            fontSize: 14.5),
                                      ),
                                      const SizedBox(height: 4),
                                      Wrap(
                                        spacing: 6,
                                        children: [
                                          Chip(
                                            label: Text('ワード: ${r.pattern}'),
                                            visualDensity:
                                                VisualDensity.compact,
                                          ),
                                          Chip(
                                            label:
                                                Text(_modeLabel(r.matchMode)),
                                            visualDensity:
                                                VisualDensity.compact,
                                          ),
                                          Chip(
                                            label: Text('優先度: ${r.priority}'),
                                            visualDensity:
                                                VisualDensity.compact,
                                          ),
                                        ],
                                      ),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 10),
                                Expanded(
                                  flex: 2,
                                  child: Container(
                                    padding: const EdgeInsets.symmetric(
                                        horizontal: 12, vertical: 9),
                                    decoration: BoxDecoration(
                                      color: bg,
                                      borderRadius: BorderRadius.circular(8),
                                      border:
                                          Border.all(color: bd, width: 1.5),
                                    ),
                                    child: Text(
                                      'プレビュー: 「${r.pattern}」を含むコメント',
                                      style: TextStyle(color: fg, fontSize: 13),
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ),
                                const SizedBox(width: 10),
                                IconButton(
                                  tooltip: '編集',
                                  onPressed: () => _openEditor(context, r),
                                  icon: const Icon(Icons.edit_outlined,
                                      size: 19),
                                ),
                                IconButton(
                                  tooltip: '削除',
                                  onPressed: () =>
                                      controller.deleteHighlightRule(r.id),
                                  icon: Icon(Icons.delete_outline,
                                      size: 19, color: p.danger),
                                ),
                              ],
                            ),
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
