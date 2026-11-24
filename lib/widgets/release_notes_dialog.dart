import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../theme/app_theme.dart';
import 'markdown_notes_view.dart';

/// アップデート後の初回起動時（および手動閲覧時）に表示するリリースノートモーダル
class ReleaseNotesDialog extends StatelessWidget {
  const ReleaseNotesDialog({
    super.key,
    required this.info,
    required this.onClose,
  });

  final ReleaseNotesInfo info;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AppTheme.surfaceDark,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(18),
        side: const BorderSide(color: AppTheme.accentSky, width: 1.5),
      ),
      title: Row(
        children: [
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: AppTheme.accentSky.withValues(alpha: 0.16),
              borderRadius: BorderRadius.circular(10),
            ),
            child: const Icon(Icons.auto_awesome, color: AppTheme.accentSky),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  info.title,
                  style: const TextStyle(
                      fontSize: 18, fontWeight: FontWeight.bold),
                ),
                if (info.previousVersion != null &&
                    info.previousVersion != info.version)
                  Text(
                    'v${info.previousVersion} → v${info.version} にアップデートされました',
                    style: const TextStyle(
                      fontSize: 12.5,
                      color: AppTheme.accentEmerald,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
      content: SizedBox(
        width: 580,
        height: 380,
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: AppTheme.bgDark,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: AppTheme.borderDark),
          ),
          child: SingleChildScrollView(
            child: MarkdownNotesView(markdown: info.markdownBody),
          ),
        ),
      ),
      actions: [
        FilledButton.icon(
          onPressed: () {
            onClose();
            Navigator.of(context).pop();
          },
          icon: const Icon(Icons.check),
          label: const Text('確認しました'),
        ),
      ],
    );
  }
}
