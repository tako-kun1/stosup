import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// 各機能画面のヘッダーに配置するコンパクトな ON/OFF トグルチップ
class FeatureToggleSwitch extends StatelessWidget {
  const FeatureToggleSwitch({
    super.key,
    required this.label,
    required this.enabled,
    required this.onChanged,
  });

  final String label;
  final bool enabled;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final color = enabled ? AppTheme.accentEmerald : Colors.white38;
    return Container(
      padding: const EdgeInsets.only(left: 12, right: 6, top: 2, bottom: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: 0.65)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            enabled ? Icons.power_settings_new : Icons.power_off_outlined,
            size: 16,
            color: color,
          ),
          const SizedBox(width: 6),
          Text(
            '$label: ${enabled ? "ON" : "OFF"}',
            style: TextStyle(
              fontSize: 12.5,
              fontWeight: FontWeight.bold,
              color: enabled ? AppTheme.accentEmerald : Colors.white70,
            ),
          ),
          const SizedBox(width: 4),
          Switch(
            value: enabled,
            onChanged: onChanged,
            materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
        ],
      ),
    );
  }
}

/// 機能が OFF のときに画面上部に表示する案内バナー（ワンクリックで ON に復帰可能）
class FeatureDisabledBanner extends StatelessWidget {
  const FeatureDisabledBanner({
    super.key,
    required this.featureName,
    required this.description,
    required this.onEnable,
  });

  final String featureName;
  final String description;
  final VoidCallback onEnable;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 14),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: AppTheme.accentAmber.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: AppTheme.accentAmber.withValues(alpha: 0.7)),
      ),
      child: Row(
        children: [
          const Icon(Icons.pause_circle_outline,
              color: AppTheme.accentAmber, size: 22),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '「$featureName」は現在オフになっています',
                  style: const TextStyle(
                    fontWeight: FontWeight.bold,
                    color: AppTheme.accentAmber,
                    fontSize: 13.5,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: const TextStyle(fontSize: 12.5, color: Colors.white70),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          FilledButton.icon(
            style: FilledButton.styleFrom(
              backgroundColor: AppTheme.accentEmerald,
              foregroundColor: Colors.black,
            ),
            onPressed: onEnable,
            icon: const Icon(Icons.power_settings_new, size: 17),
            label: const Text('オンにする'),
          ),
        ],
      ),
    );
  }
}
