import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';

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

  Future<void> _openEditor(BuildContext context, [HighlightRule? existing]) async {
    final nameCtrl = TextEditingController(text: existing?.name ?? '');
    final patternCtrl = TextEditingController(text: existing?.pattern ?? '');
    final priorityCtrl =
        TextEditingController(text: (existing?.priority ?? 10).toString());
    var matchMode = existing?.matchMode ?? MatchMode.contains;
    var caseSensitive = existing?.caseSensitive ?? false;
    var bgColor = existing?.bgColor ?? '#065F46E6';
    var textColor = existing?.textColor ?? '#F8FAFC';
    var borderColor = existing?.borderColor ?? '#34D399';
    var enabled = existing?.enabled ?? true;

    final saved = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setState) {
          final bg = AppTheme.parseCssHex(bgColor);
          final fg = AppTheme.parseCssHex(textColor, fallback: Colors.white);
          final bd = AppTheme.parseCssHex(borderColor, fallback: Colors.white38);

          return AlertDialog(
            backgroundColor: AppTheme.surfaceDark,
            title: Text(existing == null ? 'ワード反応ルールを追加' : 'ワード反応ルールを編集'),
            content: SizedBox(
              width: 560,
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // ライブプレビュー
                    const Text('吹き出しプレビュー',
                        style: TextStyle(fontSize: 12, color: Colors.white60)),
                    const SizedBox(height: 6),
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(
                          horizontal: 16, vertical: 12),
                      decoration: BoxDecoration(
                        color: bg,
                        borderRadius: const BorderRadius.only(
                          topLeft: Radius.circular(4),
                          topRight: Radius.circular(14),
                          bottomLeft: Radius.circular(14),
                          bottomRight: Radius.circular(14),
                        ),
                        border: Border.all(color: bd, width: 2),
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
                            style: TextStyle(color: fg, fontSize: 14.5),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 16),
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
                            dropdownColor: AppTheme.cardDark,
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
                        const SizedBox(width: 12),
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
                      contentPadding: EdgeInsets.zero,
                      value: caseSensitive,
                      onChanged: (v) => setState(() => caseSensitive = v),
                      title: const Text('英字の大文字・小文字を区別する'),
                    ),
                    const Divider(),
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
                        '#F8FAFC',
                        '#ECFDF5',
                        '#FFF7ED',
                        '#FEF9C3',
                        '#FCE7F3',
                        '#0F172A',
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
    final rules = controller.highlightRules;

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '特定ワード反応（吹き出し色変更）',
                      style:
                          TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
                    ),
                    SizedBox(height: 4),
                    Text(
                      '設定したワードを含むコメントの吹き出し背景色・文字色・枠線色をアプリ内と OBS オーバーレイで自動変更します。',
                      style: TextStyle(color: Colors.white60),
                    ),
                  ],
                ),
              ),
              FilledButton.icon(
                onPressed: () => _openEditor(context),
                icon: const Icon(Icons.add),
                label: const Text('新規ルール追加'),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Expanded(
            child: rules.isEmpty
                ? const Center(
                    child: Text(
                      'ルールが登録されていません。右上の「新規ルール追加」から作成できます。',
                      style: TextStyle(color: Colors.white54),
                    ),
                  )
                : ListView.builder(
                    itemCount: rules.length,
                    itemBuilder: (context, idx) {
                      final r = rules[idx];
                      final bg = AppTheme.parseCssHex(r.bgColor);
                      final fg =
                          AppTheme.parseCssHex(r.textColor, fallback: Colors.white);
                      final bd = AppTheme.parseCssHex(r.borderColor,
                          fallback: Colors.white38);

                      return Card(
                        margin: const EdgeInsets.only(bottom: 12),
                        child: Padding(
                          padding: const EdgeInsets.all(14),
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
                              const SizedBox(width: 12),
                              Expanded(
                                flex: 2,
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      r.name,
                                      style: const TextStyle(
                                          fontWeight: FontWeight.bold,
                                          fontSize: 15),
                                    ),
                                    const SizedBox(height: 4),
                                    Wrap(
                                      spacing: 8,
                                      children: [
                                        Chip(
                                          label: Text('ワード: ${r.pattern}',
                                              style: const TextStyle(
                                                  fontSize: 12)),
                                          visualDensity: VisualDensity.compact,
                                        ),
                                        Chip(
                                          label: Text(_modeLabel(r.matchMode),
                                              style: const TextStyle(
                                                  fontSize: 12)),
                                          visualDensity: VisualDensity.compact,
                                        ),
                                        Chip(
                                          label: Text('優先度: ${r.priority}',
                                              style: const TextStyle(
                                                  fontSize: 12)),
                                          visualDensity: VisualDensity.compact,
                                        ),
                                      ],
                                    ),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 12),
                              Expanded(
                                flex: 2,
                                child: Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 14, vertical: 10),
                                  decoration: BoxDecoration(
                                    color: bg,
                                    borderRadius: BorderRadius.circular(12),
                                    border: Border.all(color: bd, width: 1.8),
                                  ),
                                  child: Text(
                                    'プレビュー: 「${r.pattern}」を含むコメント',
                                    style: TextStyle(color: fg, fontSize: 13),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 12),
                              IconButton(
                                tooltip: '編集',
                                onPressed: () => _openEditor(context, r),
                                icon: const Icon(Icons.edit_outlined),
                              ),
                              IconButton(
                                tooltip: '削除',
                                onPressed: () =>
                                    controller.deleteHighlightRule(r.id),
                                icon: const Icon(Icons.delete_outline,
                                    color: AppTheme.accentRose),
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
