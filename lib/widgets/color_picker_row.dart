import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// プリセットカラー選択 ＋ HEX直接入力できる軽量カラーピッカー行
class ColorPickerRow extends StatelessWidget {
  const ColorPickerRow({
    super.key,
    required this.label,
    required this.hexValue,
    required this.onChanged,
    this.presets = defaultPresets,
  });

  final String label;
  final String hexValue;
  final ValueChanged<String> onChanged;
  final List<String> presets;

  static const List<String> defaultPresets = [
    '#065F46E6',
    '#1E3A8AE6',
    '#7C2D12E6',
    '#701A75E6',
    '#881337E6',
    '#3F3F46E6',
    '#1E293BE6',
    '#38BDF8',
    '#34D399',
    '#FBBF24',
    '#FB923C',
    '#F472B6',
    '#A78BFA',
    '#F8FAFC',
  ];

  @override
  Widget build(BuildContext context) {
    final current = AppTheme.parseCssHex(hexValue);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(label, style: const TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(width: 10),
            Container(
              width: 22,
              height: 22,
              decoration: BoxDecoration(
                color: current,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: Colors.white38),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              hexValue,
              style: const TextStyle(
                fontFamily: 'Consolas',
                fontSize: 12,
                color: Colors.white70,
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 6,
          runSpacing: 6,
          children: presets.map((hex) {
            final color = AppTheme.parseCssHex(hex);
            final selected = hex.toUpperCase() == hexValue.toUpperCase();
            return InkWell(
              onTap: () => onChanged(hex),
              borderRadius: BorderRadius.circular(6),
              child: Container(
                width: 28,
                height: 28,
                decoration: BoxDecoration(
                  color: color,
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(
                    color: selected ? Colors.white : Colors.white24,
                    width: selected ? 2.5 : 1,
                  ),
                ),
                child: selected
                    ? const Icon(Icons.check, size: 15, color: Colors.white)
                    : null,
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}
