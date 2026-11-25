import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// 各 OBS オーバーレイ用の背景色切り替えウィジェット（透過 / クロマキー / 単色 / HEX指定）
class OverlayBgPicker extends StatefulWidget {
  const OverlayBgPicker({
    super.key,
    required this.value,
    required this.onChanged,
  });

  /// "transparent" または "#RRGGBB" / "#RRGGBBAA"
  final String value;
  final ValueChanged<String> onChanged;

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

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final currentLower = widget.value.trim().toLowerCase();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'オーバーレイ背景色（透過 / クロマキー / 単色切替）',
          style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5),
        ),
        const SizedBox(height: 8),
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
                padding:
                    const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
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
                      width: 16,
                      height: 16,
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
        const SizedBox(height: 10),
        Row(
          children: [
            SizedBox(
              width: 240,
              child: TextField(
                controller: _hexCtrl,
                style: const TextStyle(fontFamily: 'Consolas', fontSize: 13),
                decoration: const InputDecoration(
                  labelText: '任意の背景色（transparent / #RRGGBB / #RRGGBBAA）',
                  hintText: 'transparent または #18181B',
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
            const SizedBox(width: 10),
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
