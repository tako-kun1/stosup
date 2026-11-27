import 'package:material_ui/material_ui.dart';

/// ユーザーアイコンを表示する共通ウィジェット（ネットワーク画像・フォールバック両対応）
class UserAvatar extends StatelessWidget {
  const UserAvatar({
    super.key,
    required this.name,
    required this.iconUrl,
    this.radius = 18,
    this.borderColor,
  });

  final String name;
  final String iconUrl;
  final double radius;
  final Color? borderColor;

  static const List<Color> _palette = [
    Color(0xFF0284C7),
    Color(0xFF7C3AED),
    Color(0xFF059669),
    Color(0xFFD97706),
    Color(0xFFDB2777),
    Color(0xFF2563EB),
    Color(0xFF0D9488),
  ];

  Color _pickColor() {
    var hash = 0;
    for (final unit in name.codeUnits) {
      hash = (hash * 31 + unit) & 0x7FFFFFFF;
    }
    return _palette[hash % _palette.length];
  }

  String _initial() {
    final t = name.trim();
    if (t.isEmpty) return '?';
    return t.characters.first;
  }

  bool _isHttpUrl(String url) {
    final lower = url.trim().toLowerCase();
    return lower.startsWith('http://') || lower.startsWith('https://');
  }

  @override
  Widget build(BuildContext context) {
    final size = radius * 2;
    Widget inner;
    if (_isHttpUrl(iconUrl)) {
      final dpr = MediaQuery.maybeDevicePixelRatioOf(context) ?? 1.0;
      final decodePx = (size * dpr).round().clamp(16, 256);
      inner = ClipOval(
        child: Image.network(
          iconUrl,
          width: size,
          height: size,
          cacheWidth: decodePx,
          cacheHeight: decodePx,
          gaplessPlayback: true,
          fit: BoxFit.cover,
          errorBuilder: (_, _, _) => _buildFallback(size),
        ),
      );
    } else {
      inner = _buildFallback(size);
    }

    if (borderColor == null) return inner;
    return Container(
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        border: Border.all(color: borderColor!, width: 2),
      ),
      child: inner,
    );
  }

  Widget _buildFallback(double size) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: _pickColor(),
      ),
      alignment: Alignment.center,
      child: Text(
        _initial(),
        style: TextStyle(
          color: Colors.white,
          fontWeight: FontWeight.bold,
          fontSize: radius * 0.85,
        ),
      ),
    );
  }
}
