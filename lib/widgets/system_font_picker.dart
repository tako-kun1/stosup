import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// OS にインストールされたシステムフォントを検索・プレビュー選択できるウィジェット
class SystemFontPicker extends StatefulWidget {
  const SystemFontPicker({
    super.key,
    required this.value,
    required this.availableFonts,
    required this.onChanged,
    required this.onRefreshFonts,
  });

  /// 空文字 = プリセット標準フォント
  final String value;
  final List<String> availableFonts;
  final ValueChanged<String> onChanged;
  final VoidCallback onRefreshFonts;

  @override
  State<SystemFontPicker> createState() => _SystemFontPickerState();
}

class _SystemFontPickerState extends State<SystemFontPicker> {
  late TextEditingController _manualCtrl;

  @override
  void initState() {
    super.initState();
    _manualCtrl = TextEditingController(text: widget.value);
  }

  @override
  void didUpdateWidget(covariant SystemFontPicker oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value && _manualCtrl.text != widget.value) {
      _manualCtrl.text = widget.value;
    }
  }

  @override
  void dispose() {
    _manualCtrl.dispose();
    super.dispose();
  }

  Future<void> _openFontPickerDialog() async {
    final picked = await showDialog<String>(
      context: context,
      builder: (_) => _FontPickerDialog(
        currentFont: widget.value,
        availableFonts: widget.availableFonts,
      ),
    );
    if (picked != null) {
      widget.onChanged(picked);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final hasCustom = widget.value.trim().isNotEmpty;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Expanded(
              child: Text(
                'システムフォント（OS にインストール済みのフォントを使用）',
                style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5),
              ),
            ),
            IconButton(
              tooltip: 'インストール済みフォント一覧を再読み込み',
              visualDensity: VisualDensity.compact,
              onPressed: widget.onRefreshFonts,
              icon: const Icon(Icons.refresh, size: 18),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _manualCtrl,
                decoration: InputDecoration(
                  labelText: 'フォントファミリー名（空欄でプリセット標準）',
                  hintText: '例: Meiryo / BIZ UDPゴシック / 游明朝',
                  suffixIcon: hasCustom
                      ? IconButton(
                          tooltip: 'プリセット標準に戻す',
                          onPressed: () => widget.onChanged(''),
                          icon: const Icon(Icons.clear, size: 17),
                        )
                      : null,
                ),
                onChanged: (v) => widget.onChanged(v.trim()),
              ),
            ),
            const SizedBox(width: 10),
            FilledButton.icon(
              onPressed: _openFontPickerDialog,
              icon: const Icon(Icons.font_download_outlined, size: 17),
              label: Text('一覧から選ぶ (${widget.availableFonts.length})'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: p.windowBg,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(color: p.border),
          ),
          child: Row(
            children: [
              Text(
                '書体サンプル:',
                style: TextStyle(fontSize: 11.5, color: p.textSecondary),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  '初見です！配信応援してます 123 ABC',
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontFamily: hasCustom ? widget.value : null,
                    fontSize: 15,
                    color: p.textPrimary,
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Text(
                hasCustom ? widget.value : 'プリセット標準',
                style: TextStyle(
                  fontSize: 11.5,
                  color: p.textMuted,
                  fontFamily: 'Consolas',
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _FontPickerDialog extends StatefulWidget {
  const _FontPickerDialog({
    required this.currentFont,
    required this.availableFonts,
  });

  final String currentFont;
  final List<String> availableFonts;

  @override
  State<_FontPickerDialog> createState() => _FontPickerDialogState();
}

class _FontPickerDialogState extends State<_FontPickerDialog> {
  final _searchCtrl = TextEditingController();

  @override
  void dispose() {
    _searchCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final q = _searchCtrl.text.trim().toLowerCase();
    final filtered = widget.availableFonts
        .where((f) => q.isEmpty || f.toLowerCase().contains(q))
        .toList();

    return AlertDialog(
      title: Row(
        children: [
          const Expanded(
            child: Text(
              'OS のシステムフォントを選択',
              style: TextStyle(fontSize: 16.5, fontWeight: FontWeight.bold),
            ),
          ),
          Text(
            '${filtered.length} / ${widget.availableFonts.length} 件',
            style: TextStyle(fontSize: 12, color: p.textSecondary),
          ),
        ],
      ),
      content: SizedBox(
        width: 520,
        height: 440,
        child: Column(
          children: [
            TextField(
              controller: _searchCtrl,
              autofocus: true,
              onChanged: (_) => setState(() {}),
              decoration: const InputDecoration(
                prefixIcon: Icon(Icons.search, size: 18),
                hintText: 'フォント名で検索（例: メイリオ, 游ゴシック, BIZ, Noto, Consolas）',
              ),
            ),
            const SizedBox(height: 10),
            Expanded(
              child: Material(
                color: p.windowBg,
                clipBehavior: Clip.antiAlias,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(6),
                  side: BorderSide(color: p.border),
                ),
                child: ListView.separated(
                  itemCount: filtered.length + 1,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (_, idx) {
                    if (idx == 0) {
                      final selected = widget.currentFont.isEmpty;
                      return ListTile(
                        dense: true,
                        selected: selected,
                        leading: Icon(
                          selected ? Icons.check_circle : Icons.restart_alt,
                          size: 18,
                          color: selected ? p.accent : p.textSecondary,
                        ),
                        title: const Text(
                          'プリセット標準フォント（指定なし）',
                          style: TextStyle(fontWeight: FontWeight.w600),
                        ),
                        onTap: () => Navigator.pop(context, ''),
                      );
                    }
                    final fontName = filtered[idx - 1];
                    final selected = fontName == widget.currentFont;
                    return ListTile(
                      dense: true,
                      selected: selected,
                      leading: Icon(
                        selected
                            ? Icons.check_circle
                            : Icons.font_download_outlined,
                        size: 18,
                        color: selected ? p.accent : p.textSecondary,
                      ),
                      title: Text(
                        fontName,
                        style: TextStyle(
                          fontWeight:
                              selected ? FontWeight.bold : FontWeight.w500,
                        ),
                      ),
                      subtitle: Text(
                        'あいうえお 配信テスト ABC 123',
                        style: TextStyle(
                          fontFamily: fontName,
                          fontSize: 13.5,
                          color: p.textSecondary,
                        ),
                      ),
                      onTap: () => Navigator.pop(context, fontName),
                    );
                  },
                ),
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
      ],
    );
  }
}
