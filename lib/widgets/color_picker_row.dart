import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// プリセットカラー選択 ＋ HEX 表示するカラーピッカー行（ライト/ダーク両対応）
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
    '#202124E6',
    '#065F46E6',
    '#1E3A8AE6',
    '#7C2D12E6',
    '#701A75E6',
    '#881337E6',
    '#3F3F46E6',
    '#2563EB',
    '#16A34A',
    '#D97706',
    '#EA580C',
    '#DB2777',
    '#7C3AED',
    '#F4F4F5',
  ];

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final current = AppTheme.parseCssHex(hexValue);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(label, style: const TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(width: 10),
            Container(
              width: 20,
              height: 20,
              decoration: BoxDecoration(
                color: current,
                borderRadius: BorderRadius.circular(4),
                border: Border.all(color: p.border),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              hexValue,
              style: TextStyle(
                fontFamily: 'Consolas',
                fontSize: 12,
                color: p.textSecondary,
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
            final isLightSwatch = color.computeLuminance() > 0.6;
            return InkWell(
              onTap: () => onChanged(hex),
              borderRadius: BorderRadius.circular(5),
              child: Container(
                width: 26,
                height: 26,
                decoration: BoxDecoration(
                  color: color,
                  borderRadius: BorderRadius.circular(5),
                  border: Border.all(
                    color: selected ? p.accent : p.border,
                    width: selected ? 2.2 : 1,
                  ),
                ),
                child: selected
                    ? Icon(
                        Icons.check,
                        size: 14,
                        color: isLightSwatch ? Colors.black : Colors.white,
                      )
                    : null,
              ),
            );
          }).toList(),
        ),
      ],
    );
  }
}
