import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// リリースノート用の軽量 Markdown 表示ウィジェット（ライト/ダーク両対応）
class MarkdownNotesView extends StatelessWidget {
  const MarkdownNotesView({super.key, required this.markdown});

  final String markdown;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final lines = markdown.replaceAll('\r\n', '\n').split('\n');
    final widgets = <Widget>[];

    for (final rawLine in lines) {
      final line = rawLine.trimRight();
      if (line.trim().isEmpty) {
        widgets.add(const SizedBox(height: 6));
      } else if (line.startsWith('### ')) {
        widgets.add(Padding(
          padding: const EdgeInsets.only(top: 10, bottom: 4),
          child: Text(
            line.substring(4),
            style: TextStyle(
              fontSize: 14.5,
              fontWeight: FontWeight.bold,
              color: p.accent,
            ),
          ),
        ));
      } else if (line.startsWith('## ')) {
        widgets.add(Padding(
          padding: const EdgeInsets.only(top: 12, bottom: 6),
          child: Text(
            line.substring(3),
            style: TextStyle(
              fontSize: 16.5,
              fontWeight: FontWeight.bold,
              color: p.textPrimary,
            ),
          ),
        ));
      } else if (line.startsWith('# ')) {
        widgets.add(Padding(
          padding: const EdgeInsets.only(top: 12, bottom: 6),
          child: Text(
            line.substring(2),
            style: TextStyle(
              fontSize: 18.5,
              fontWeight: FontWeight.bold,
              color: p.textPrimary,
            ),
          ),
        ));
      } else if (line.trim() == '---') {
        widgets.add(const Padding(
          padding: EdgeInsets.symmetric(vertical: 8),
          child: Divider(),
        ));
      } else if (line.trimLeft().startsWith('- ') ||
          line.trimLeft().startsWith('* ')) {
        final content = line.trimLeft().substring(2);
        widgets.add(Padding(
          padding: const EdgeInsets.only(left: 8, top: 3, bottom: 3),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('•  ',
                  style:
                      TextStyle(color: p.accent, fontWeight: FontWeight.bold)),
              Expanded(child: _buildInlineRichText(content, p)),
            ],
          ),
        ));
      } else {
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 2),
          child: _buildInlineRichText(line, p),
        ));
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
  }

  Widget _buildInlineRichText(String text, StosupPalette p) {
    final spans = <InlineSpan>[];
    final pattern = RegExp(r'(\*\*.*?\*\*|`[^`]+`)');
    var lastEnd = 0;

    for (final match in pattern.allMatches(text)) {
      if (match.start > lastEnd) {
        spans.add(TextSpan(text: text.substring(lastEnd, match.start)));
      }
      final token = match.group(0)!;
      if (token.startsWith('**') && token.endsWith('**')) {
        spans.add(TextSpan(
          text: token.substring(2, token.length - 2),
          style: TextStyle(
            fontWeight: FontWeight.bold,
            color: p.textPrimary,
          ),
        ));
      } else if (token.startsWith('`') && token.endsWith('`')) {
        spans.add(TextSpan(
          text: token.substring(1, token.length - 1),
          style: TextStyle(
            fontFamily: 'Consolas',
            backgroundColor: p.subtleBg,
            color: p.accent,
          ),
        ));
      }
      lastEnd = match.end;
    }

    if (lastEnd < text.length) {
      spans.add(TextSpan(text: text.substring(lastEnd)));
    }

    return Text.rich(
      TextSpan(
        style: TextStyle(
          fontSize: 13.5,
          height: 1.5,
          color: p.textSecondary,
        ),
        children: spans,
      ),
    );
  }
}
