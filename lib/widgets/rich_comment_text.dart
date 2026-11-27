import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';

/// YouTube カスタム絵文字・メンバースタンプの画像インライン表示に対応したコメント本文ウィジェット
class RichCommentText extends StatelessWidget {
  const RichCommentText({
    super.key,
    required this.message,
    required this.runs,
    this.style,
    this.maxLines,
    this.overflow = TextOverflow.clip,
    this.emojiSize = 22,
  });

  final String message;
  final List<MessageRun> runs;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow overflow;
  final double emojiSize;

  static final RegExp _svgFillHexRe =
      RegExp(r'''fill=['"](?:%23|#)([0-9a-fA-F]{6})['"]''');
  static final RegExp _svgTextRawRe =
      RegExp(r'<text[^>]*>([^<]{1,24})</text>', caseSensitive: false);
  static final RegExp _svgTextDecodedRe =
      RegExp(r'<text[^>]*>([^<]{1,8})</text>', caseSensitive: false);

  @override
  Widget build(BuildContext context) {
    final hasEmoji = runs.any(
      (r) =>
          r.kind == MessageRunKind.emoji &&
          r.emojiUrl != null &&
          r.emojiUrl!.isNotEmpty,
    );
    if (runs.isEmpty || !hasEmoji) {
      return Text(
        message,
        style: style,
        maxLines: maxLines,
        overflow: overflow,
      );
    }

    final dpr = MediaQuery.maybeDevicePixelRatioOf(context) ?? 1.0;
    final decodePx = (emojiSize * dpr).round().clamp(16, 128);
    final spans = <InlineSpan>[];
    for (final run in runs) {
      if (run.kind == MessageRunKind.emoji &&
          run.emojiUrl != null &&
          run.emojiUrl!.isNotEmpty) {
        final url = run.emojiUrl!;
        final fallback = run.alt ?? run.text;
        spans.add(
          WidgetSpan(
            alignment: PlaceholderAlignment.middle,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 1.5),
              child: Tooltip(
                message: fallback,
                child: _buildEmojiImage(url, fallback, decodePx),
              ),
            ),
          ),
        );
      } else if (run.text.isNotEmpty) {
        spans.add(TextSpan(text: run.text));
      }
    }

    return Text.rich(
      TextSpan(style: style, children: spans),
      maxLines: maxLines,
      overflow: overflow,
    );
  }

  Widget _buildEmojiImage(String url, String fallback, int decodePx) {
    // テスト・デモ用のインライン SVG Data URI は簡易バッジとして美しく描画
    if (url.startsWith('data:image/svg+xml')) {
      final decoded = _extractSvgBadgeLabel(url, fallback);
      final badgeColor = _extractSvgBadgeColor(url) ?? const Color(0xFFCA8A04);
      return DecoratedBox(
        decoration: BoxDecoration(
          color: badgeColor,
          borderRadius: BorderRadius.circular(5),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 3),
          child: Text(
            decoded,
            style: TextStyle(
              color: Colors.white,
              fontSize: (emojiSize * 0.56).clamp(10, 14),
              fontWeight: FontWeight.w800,
              height: 1.0,
            ),
          ),
        ),
      );
    }

    if (!url.startsWith('https://') && !url.startsWith('http://')) {
      return Text(fallback, style: style);
    }

    return Image.network(
      url,
      width: emojiSize,
      height: emojiSize,
      cacheWidth: decodePx,
      cacheHeight: decodePx,
      gaplessPlayback: true,
      fit: BoxFit.contain,
      filterQuality: FilterQuality.medium,
      errorBuilder: (_, _, _) => Text(fallback, style: style),
    );
  }

  Color? _extractSvgBadgeColor(String dataUri) {
    final match = _svgFillHexRe.firstMatch(dataUri);
    final hex = match?.group(1);
    if (hex == null) return null;
    final value = int.tryParse(hex, radix: 16);
    if (value == null) return null;
    return Color(0xFF000000 | value);
  }

  String _extractSvgBadgeLabel(String dataUri, String fallback) {
    // 非 ASCII 文字（★ 等）を含む Data URI に Uri.decodeFull を直接適用すると例外になるため、
    // まず生の SVG から <text>...</text> を抽出し、%XX を含む場合のみ安全にデコードする。
    final rawMatch = _svgTextRawRe.firstMatch(dataUri);
    if (rawMatch != null && rawMatch.group(1) != null) {
      var label = rawMatch.group(1)!.trim();
      if (label.contains('%')) {
        try {
          label = Uri.decodeComponent(label).trim();
        } catch (_) {}
      }
      if (label.isNotEmpty) {
        return label;
      }
    }
    try {
      final decoded = Uri.decodeFull(dataUri);
      final match = _svgTextDecodedRe.firstMatch(decoded);
      if (match != null && match.group(1) != null) {
        final label = match.group(1)!.trim();
        if (label.isNotEmpty) return label;
      }
    } catch (_) {}
    final cleaned = fallback.replaceAll(':', '');
    if (cleaned.isEmpty) return '★';
    final bytes = utf8.encode(cleaned);
    if (bytes.length <= 4) return cleaned;
    return cleaned.substring(0, 2).toUpperCase();
  }
}
