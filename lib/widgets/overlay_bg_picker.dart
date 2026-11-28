import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// 各 OBS オーバーレイ用の背景色切り替えウィジェット（透過 / クロマキー / 単色 / 自由カラー選択 / HEX指定）
class OverlayBgPicker extends StatefulWidget {
  const OverlayBgPicker({
    super.key,
    required this.value,
    required this.onChanged,
    this.title = 'オーバーレイ背景色（透過 / クロマキー / 単色切替）',
    this.compact = false,
  });

  /// "transparent" または "#RRGGBB" / "#RRGGBBAA"
  final String value;
  final ValueChanged<String> onChanged;
  final String title;
  final bool compact;

  static const List<({String label, String value, Color preview})> quickColors =
      [
    (label: '透過（OBS標準）', value: 'transparent', preview: Colors.transparent),
    (label: 'クロマキー緑', value: '#00FF00', preview: Color(0xFF00FF00)),
    (label: 'クロマキー青', value: '#0000FF', preview: Color(0xFF0000FF)),
    (label: 'ダーク', value: '#18181B', preview: Color(0xFF18181B)),
    (label: '半透明ダーク', value: '#18181BCC', preview: Color(0xCC18181B)),
    (label: 'ホワイト', value: '#FFFFFF', preview: Color(0xFFFFFFFF)),
    (label: 'ネイビー', value: '#0F172A', preview: Color(0xFF0F172A)),
  ];

  @override
  State<OverlayBgPicker> createState() => _OverlayBgPickerState();
}

class _OverlayBgPickerState extends State<OverlayBgPicker> {
  late TextEditingController _hexCtrl;

  @override
  void initState() {
    super.initState();
    _hexCtrl = TextEditingController(text: widget.value);
  }

  @override
  void didUpdateWidget(covariant OverlayBgPicker oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value && _hexCtrl.text != widget.value) {
      _hexCtrl.text = widget.value;
    }
  }

  @override
  void dispose() {
    _hexCtrl.dispose();
    super.dispose();
  }

  Color? _parsePreviewColor(String raw) {
    final trimmed = raw.trim();
    if (trimmed.isEmpty || trimmed.toLowerCase() == 'transparent') {
      return null;
    }
    if (!RegExp(r'^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$').hasMatch(trimmed)) {
      return null;
    }
    return AppTheme.parseCssHex(trimmed);
  }

  Future<void> _openColorDialog() async {
    final initial = _parsePreviewColor(widget.value) ?? const Color(0xFF18181B);
    final result = await showDialog<String>(
      context: context,
      builder: (_) => _CustomBgColorDialog(initialColor: initial),
    );
    if (!mounted || result == null) return;
    _hexCtrl.text = result;
    widget.onChanged(result);
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final currentLower = widget.value.trim().toLowerCase();
    final previewColor = _parsePreviewColor(widget.value);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          widget.title,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: widget.compact ? 12.5 : 13.5,
          ),
        ),
        SizedBox(height: widget.compact ? 6 : 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: OverlayBgPicker.quickColors.map((item) {
            final selected = item.value.toLowerCase() == currentLower;
            final isTransparent = item.value == 'transparent';
            return InkWell(
              onTap: () => widget.onChanged(item.value),
              borderRadius: BorderRadius.circular(6),
              child: Container(
                padding: EdgeInsets.symmetric(
                  horizontal: widget.compact ? 8 : 10,
                  vertical: widget.compact ? 5 : 6,
                ),
                decoration: BoxDecoration(
                  color: selected ? p.accent.withValues(alpha: 0.14) : p.cardBg,
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: selected ? p.accent : p.border,
                    width: selected ? 1.8 : 1,
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 15,
                      height: 15,
                      decoration: BoxDecoration(
                        color: isTransparent ? p.windowBg : item.preview,
                        borderRadius: BorderRadius.circular(3),
                        border: Border.all(color: p.border),
                      ),
                      child: isTransparent
                          ? Icon(Icons.grid_on, size: 11, color: p.textMuted)
                          : null,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      item.label,
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight:
                            selected ? FontWeight.bold : FontWeight.normal,
                        color: p.textPrimary,
                      ),
                    ),
                  ],
                ),
              ),
            );
          }).toList(),
        ),
        SizedBox(height: widget.compact ? 8 : 10),
        Wrap(
          spacing: 10,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            Container(
              width: 28,
              height: 28,
              decoration: BoxDecoration(
                color: previewColor ?? p.windowBg,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: p.border, width: 1.5),
              ),
              child: previewColor == null
                  ? Icon(Icons.grid_on, size: 16, color: p.textMuted)
                  : null,
            ),
            SizedBox(
              width: 230,
              child: TextField(
                controller: _hexCtrl,
                style: const TextStyle(fontFamily: 'Consolas', fontSize: 13),
                decoration: const InputDecoration(
                  labelText: '背景色コード（transparent / #RRGGBB）',
                  hintText: 'transparent または #18181B',
                  isDense: true,
                ),
                onChanged: (v) {
                  final trimmed = v.trim();
                  if (trimmed.toLowerCase() == 'transparent' ||
                      RegExp(r'^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$')
                          .hasMatch(trimmed)) {
                    widget.onChanged(trimmed);
                  }
                },
              ),
            ),
            OutlinedButton.icon(
              onPressed: _openColorDialog,
              icon: const Icon(Icons.palette_outlined, size: 16),
              label: const Text('カラー選択'),
            ),
            if (!widget.compact)
              Text(
                'URL末尾に ?bg=00FF00 等を付けて一時上書きも可能です',
                style: TextStyle(fontSize: 11.5, color: p.textSecondary),
              ),
          ],
        ),
      ],
    );
  }
}

class _CustomBgColorDialog extends StatefulWidget {
  const _CustomBgColorDialog({required this.initialColor});

  final Color initialColor;

  @override
  State<_CustomBgColorDialog> createState() => _CustomBgColorDialogState();
}

class _CustomBgColorDialogState extends State<_CustomBgColorDialog> {
  static const List<Color> _swatches = [
    Color(0xFF000000),
    Color(0xFF18181B),
    Color(0xFF27272A),
    Color(0xFFFFFFFF),
    Color(0xFF00FF00),
    Color(0xFF0000FF),
    Color(0xFFFF00FF),
    Color(0xFF0F172A),
    Color(0xFF1E3A8A),
    Color(0xFF065F46),
    Color(0xFF7C2D12),
    Color(0xFF581C87),
    Color(0xFF881337),
    Color(0xFF0EA5E9),
    Color(0xFF22C55E),
    Color(0xFFF59E0B),
    Color(0xFFEF4444),
    Color(0xFFEC4899),
  ];

  late HSVColor _hsv;
  late double _alpha;

  @override
  void initState() {
    super.initState();
    _hsv = HSVColor.fromColor(widget.initialColor.withValues(alpha: 1.0));
    _alpha = widget.initialColor.a;
  }

  String _toCssHex() {
    final a = (_alpha * 255).round().clamp(0, 255);
    return AppTheme.toCssHex(
      _hsv.toColor().withValues(alpha: _alpha),
      includeAlpha: a < 255,
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final preview = _hsv.toColor().withValues(alpha: _alpha);
    final hex = _toCssHex();

    return AlertDialog(
      title: const Text('オーバーレイ背景色を選択'),
      content: SizedBox(
        width: 400,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 48,
                    height: 48,
                    decoration: BoxDecoration(
                      color: preview,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: p.border, width: 1.5),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        '選択中の背景色',
                        style: TextStyle(
                            fontSize: 12, fontWeight: FontWeight.bold),
                      ),
                      Text(
                        hex,
                        style: const TextStyle(
                          fontFamily: 'Consolas',
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 14),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: _swatches.map((c) {
                  return InkWell(
                    onTap: () => setState(() {
                      _hsv = HSVColor.fromColor(c);
                      _alpha = 1.0;
                    }),
                    borderRadius: BorderRadius.circular(5),
                    child: Container(
                      width: 28,
                      height: 28,
                      decoration: BoxDecoration(
                        color: c,
                        borderRadius: BorderRadius.circular(5),
                        border: Border.all(color: p.border),
                      ),
                    ),
                  );
                }).toList(),
              ),
              const SizedBox(height: 14),
              Text('色相 (Hue): ${_hsv.hue.round()}°',
                  style: TextStyle(fontSize: 12, color: p.textSecondary)),
              Slider(
                value: _hsv.hue,
                min: 0,
                max: 360,
                onChanged: (v) => setState(() => _hsv = _hsv.withHue(v)),
              ),
              Text('彩度 (Saturation): ${(_hsv.saturation * 100).round()}%',
                  style: TextStyle(fontSize: 12, color: p.textSecondary)),
              Slider(
                value: _hsv.saturation,
                min: 0,
                max: 1,
                onChanged: (v) => setState(() => _hsv = _hsv.withSaturation(v)),
              ),
              Text('明度 (Brightness): ${(_hsv.value * 100).round()}%',
                  style: TextStyle(fontSize: 12, color: p.textSecondary)),
              Slider(
                value: _hsv.value,
                min: 0,
                max: 1,
                onChanged: (v) => setState(() => _hsv = _hsv.withValue(v)),
              ),
              Text('不透明度 (Alpha): ${(_alpha * 100).round()}%',
                  style: TextStyle(fontSize: 12, color: p.textSecondary)),
              Slider(
                value: _alpha,
                min: 0,
                max: 1,
                onChanged: (v) => setState(() => _alpha = v),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, 'transparent'),
          child: const Text('透過に戻す'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('キャンセル'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, hex),
          child: const Text('この色を適用'),
        ),
      ],
    );
  }
}
