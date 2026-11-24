import 'package:material_ui/material_ui.dart';

import '../theme/app_theme.dart';

/// リリースノート用の軽量 Markdown 表示ウィジェット（外部依存ゼロ）
class MarkdownNotesView extends StatelessWidget {
  const MarkdownNotesView({super.key, required this.markdown});

  final String markdown;

  @override
  Widget build(BuildContext context) {
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
            style: const TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.bold,
              color: AppTheme.accentSky,
            ),
          ),
        ));
      } else if (line.startsWith('## ')) {
        widgets.add(Padding(
          padding: const EdgeInsets.only(top: 12, bottom: 6),
          child: Text(
            line.substring(3),
            style: const TextStyle(
              fontSize: 17,
              fontWeight: FontWeight.bold,
              color: Colors.white,
            ),
          ),
        ));
      } else if (line.startsWith('# ')) {
        widgets.add(Padding(
          padding: const EdgeInsets.only(top: 12, bottom: 6),
          child: Text(
            line.substring(2),
            style: const TextStyle(
              fontSize: 19,
              fontWeight: FontWeight.bold,
              color: Colors.white,
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
              const Text('•  ',
                  style: TextStyle(
                      color: AppTheme.accentSky, fontWeight: FontWeight.bold)),
              Expanded(child: _buildInlineRichText(content)),
            ],
          ),
        ));
      } else {
        widgets.add(Padding(
          padding: const EdgeInsets.symmetric(vertical: 2),
          child: _buildInlineRichText(line),
        ));
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: widgets,
    );
  }

  Widget _buildInlineRichText(String text) {
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
          style: const TextStyle(
            fontWeight: FontWeight.bold,
            color: Colors.white,
          ),
        ));
      } else if (token.startsWith('`') && token.endsWith('`')) {
        spans.add(TextSpan(
          text: token.substring(1, token.length - 1),
          style: const TextStyle(
            fontFamily: 'Consolas',
            backgroundColor: Color(0xFF0F172A),
            color: AppTheme.accentAmber,
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
        style: const TextStyle(
          fontSize: 13.5,
          height: 1.5,
          color: Color(0xFFE2E8F0),
        ),
        children: spans,
      ),
    );
  }
}
