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
    final p = context.palette;
    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.article_outlined, color: p.accent),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  info.title,
                  style: const TextStyle(
                      fontSize: 17, fontWeight: FontWeight.bold),
                ),
                if (info.previousVersion != null &&
                    info.previousVersion != info.version)
                  Text(
                    'v${info.previousVersion} → v${info.version} にアップデートされました',
                    style: TextStyle(fontSize: 12.5, color: p.success),
                  ),
              ],
            ),
          ),
        ],
      ),
      content: SizedBox(
        width: 580,
        height: 360,
        child: Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: p.windowBg,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(color: p.border),
          ),
          child: SingleChildScrollView(
            child: MarkdownNotesView(markdown: info.markdownBody),
          ),
        ),
      ),
      actions: [
        FilledButton(
          onPressed: () {
            onClose();
            Navigator.of(context).pop();
          },
          child: const Text('閉じる'),
        ),
      ],
    );
  }
}
