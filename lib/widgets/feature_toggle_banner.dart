import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// 各機能画面のヘッダーに配置するコンパクトな ON/OFF トグル（ライト/ダーク両対応）
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
    final p = context.palette;
    final statusColor = enabled ? p.success : p.textMuted;
    return Container(
      padding: const EdgeInsets.only(left: 10, right: 6, top: 2, bottom: 2),
      decoration: BoxDecoration(
        color: p.cardBg,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(
          color: enabled ? p.success.withValues(alpha: 0.5) : p.border,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(
              color: statusColor,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 7),
          Text(
            '$label: ${enabled ? "ON" : "OFF"}',
            style: TextStyle(
              fontSize: 12.5,
              fontWeight: FontWeight.w600,
              color: enabled ? p.textPrimary : p.textSecondary,
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

/// 機能が OFF のときに画面上部に表示する案内バナー（ライト/ダーク両対応）
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
    final p = context.palette;
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 14),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: p.warning.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: p.warning.withValues(alpha: 0.45)),
      ),
      child: Row(
        children: [
          Icon(Icons.pause_circle_outline, color: p.warning, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '「$featureName」は現在オフになっています',
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color: p.textPrimary,
                    fontSize: 13,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                ),
              ],
            ),
          ),
          const SizedBox(width: 10),
          FilledButton(
            onPressed: onEnable,
            child: const Text('オンにする'),
          ),
        ],
      ),
    );
  }
}
