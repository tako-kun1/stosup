import 'package:material_ui/material_ui.dart';

/// Stosup 配信者向けダークテーマ ＆ カラーユーティリティ
class AppTheme {
  static const Color bgDark = Color(0xFF0B1120);
  static const Color surfaceDark = Color(0xFF111827);
  static const Color cardDark = Color(0xFF1E293B);
  static const Color borderDark = Color(0xFF334155);
  static const Color accentSky = Color(0xFF38BDF8);
  static const Color accentIndigo = Color(0xFF818CF8);
  static const Color accentEmerald = Color(0xFF34D399);
  static const Color accentAmber = Color(0xFFFBBF24);
  static const Color accentRose = Color(0xFFFB7185);

  static ThemeData darkTheme() {
    final colorScheme = ColorScheme.fromSeed(
      seedColor: accentSky,
      brightness: Brightness.dark,
      surface: surfaceDark,
    ).copyWith(
      primary: accentSky,
      secondary: accentIndigo,
      surface: surfaceDark,
      surfaceContainerHighest: cardDark,
      error: accentRose,
    );

    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: bgDark,
      fontFamily: 'Yu Gothic UI',
      cardTheme: CardThemeData(
        color: cardDark,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(14),
          side: const BorderSide(color: borderDark, width: 1),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: const Color(0xFF0F172A),
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: const BorderSide(color: borderDark),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: const BorderSide(color: borderDark),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: const BorderSide(color: accentSky, width: 1.5),
        ),
      ),
      dividerTheme: const DividerThemeData(
        color: borderDark,
        thickness: 1,
      ),
      snackBarTheme: SnackBarThemeData(
        backgroundColor: cardDark,
        contentTextStyle: const TextStyle(color: Colors.white),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: const BorderSide(color: borderDark),
        ),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }

  /// `#RRGGBB` または `#RRGGBBAA` (CSS形式) を Flutter の Color に変換する
  static Color parseCssHex(String? hex, {Color fallback = cardDark}) {
    if (hex == null) return fallback;
    final cleaned = hex.trim().replaceFirst('#', '');
    try {
      if (cleaned.length == 6) {
        final rgb = int.parse(cleaned, radix: 16);
        return Color(0xFF000000 | rgb);
      } else if (cleaned.length == 8) {
        final r = int.parse(cleaned.substring(0, 2), radix: 16);
        final g = int.parse(cleaned.substring(2, 4), radix: 16);
        final b = int.parse(cleaned.substring(4, 6), radix: 16);
        final a = int.parse(cleaned.substring(6, 8), radix: 16);
        return Color.fromARGB(a, r, g, b);
      }
    } catch (_) {
      return fallback;
    }
    return fallback;
  }

  /// Flutter の Color を CSS形式 `#RRGGBBAA` へ変換する
  static String toCssHex(Color color, {bool includeAlpha = true}) {
    final r = (color.r * 255).round().toRadixString(16).padLeft(2, '0');
    final g = (color.g * 255).round().toRadixString(16).padLeft(2, '0');
    final b = (color.b * 255).round().toRadixString(16).padLeft(2, '0');
    if (!includeAlpha) {
      return '#${r.toUpperCase()}${g.toUpperCase()}${b.toUpperCase()}';
    }
    final a = (color.a * 255).round().toRadixString(16).padLeft(2, '0');
    return '#${r.toUpperCase()}${g.toUpperCase()}${b.toUpperCase()}${a.toUpperCase()}';
  }
}
