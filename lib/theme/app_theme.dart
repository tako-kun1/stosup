import 'package:material_ui/material_ui.dart';

/// ライトモード・ダークモード両対応のセマンティックカラーパレット
/// ニュートラルなデスクトップツール調（Windows 11 / OBS / VS Code 系）で落ち着いた視認性を重視
@immutable
class StosupPalette extends ThemeExtension<StosupPalette> {
  const StosupPalette({
    required this.windowBg,
    required this.sidebarBg,
    required this.cardBg,
    required this.inputBg,
    required this.border,
    required this.subtleBg,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.accent,
    required this.success,
    required this.warning,
    required this.danger,
  });

  final Color windowBg;
  final Color sidebarBg;
  final Color cardBg;
  final Color inputBg;
  final Color border;
  final Color subtleBg;
  final Color textPrimary;
  final Color textSecondary;
  final Color textMuted;
  final Color accent;
  final Color success;
  final Color warning;
  final Color danger;

  static const StosupPalette dark = StosupPalette(
    windowBg: Color(0xFF18181B),
    sidebarBg: Color(0xFF1E1E22),
    cardBg: Color(0xFF232328),
    inputBg: Color(0xFF161619),
    border: Color(0xFF32323A),
    subtleBg: Color(0xFF2B2B32),
    textPrimary: Color(0xFFF4F4F5),
    textSecondary: Color(0xFFA1A1AA),
    textMuted: Color(0xFF71717A),
    accent: Color(0xFF3B82F6),
    success: Color(0xFF22C55E),
    warning: Color(0xFFEAB308),
    danger: Color(0xFFEF4444),
  );

  static const StosupPalette light = StosupPalette(
    windowBg: Color(0xFFF4F5F7),
    sidebarBg: Color(0xFFEBEEF2),
    cardBg: Color(0xFFFFFFFF),
    inputBg: Color(0xFFF9FAFB),
    border: Color(0xFFD6DAE1),
    subtleBg: Color(0xFFE5E7EB),
    textPrimary: Color(0xFF18181B),
    textSecondary: Color(0xFF52525B),
    textMuted: Color(0xFF71717A),
    accent: Color(0xFF2563EB),
    success: Color(0xFF15803D),
    warning: Color(0xFFB45309),
    danger: Color(0xFFDC2626),
  );

  @override
  StosupPalette copyWith({
    Color? windowBg,
    Color? sidebarBg,
    Color? cardBg,
    Color? inputBg,
    Color? border,
    Color? subtleBg,
    Color? textPrimary,
    Color? textSecondary,
    Color? textMuted,
    Color? accent,
    Color? success,
    Color? warning,
    Color? danger,
  }) {
    return StosupPalette(
      windowBg: windowBg ?? this.windowBg,
      sidebarBg: sidebarBg ?? this.sidebarBg,
      cardBg: cardBg ?? this.cardBg,
      inputBg: inputBg ?? this.inputBg,
      border: border ?? this.border,
      subtleBg: subtleBg ?? this.subtleBg,
      textPrimary: textPrimary ?? this.textPrimary,
      textSecondary: textSecondary ?? this.textSecondary,
      textMuted: textMuted ?? this.textMuted,
      accent: accent ?? this.accent,
      success: success ?? this.success,
      warning: warning ?? this.warning,
      danger: danger ?? this.danger,
    );
  }

  @override
  StosupPalette lerp(ThemeExtension<StosupPalette>? other, double t) {
    if (other is! StosupPalette) return this;
    return StosupPalette(
      windowBg: Color.lerp(windowBg, other.windowBg, t)!,
      sidebarBg: Color.lerp(sidebarBg, other.sidebarBg, t)!,
      cardBg: Color.lerp(cardBg, other.cardBg, t)!,
      inputBg: Color.lerp(inputBg, other.inputBg, t)!,
      border: Color.lerp(border, other.border, t)!,
      subtleBg: Color.lerp(subtleBg, other.subtleBg, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textSecondary: Color.lerp(textSecondary, other.textSecondary, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      accent: Color.lerp(accent, other.accent, t)!,
      success: Color.lerp(success, other.success, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
    );
  }
}

extension StosupThemeX on BuildContext {
  StosupPalette get palette =>
      Theme.of(this).extension<StosupPalette>() ?? StosupPalette.dark;
}

/// Stosup テーマ定義 ＆ カラーユーティリティ
class AppTheme {
  static ThemeData darkTheme() => _buildTheme(Brightness.dark, StosupPalette.dark);

  static ThemeData lightTheme() =>
      _buildTheme(Brightness.light, StosupPalette.light);

  static ThemeData _buildTheme(Brightness brightness, StosupPalette p) {
    final isDark = brightness == Brightness.dark;
    final colorScheme = ColorScheme(
      brightness: brightness,
      primary: p.accent,
      onPrimary: Colors.white,
      secondary: p.accent,
      onSecondary: Colors.white,
      error: p.danger,
      onError: Colors.white,
      surface: p.cardBg,
      onSurface: p.textPrimary,
      surfaceContainerHighest: p.subtleBg,
      onSurfaceVariant: p.textSecondary,
      outline: p.border,
      outlineVariant: p.border,
    );

    final btnShape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(6),
    );

    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: p.windowBg,
      fontFamily: 'Yu Gothic UI',
      extensions: [p],
      cardTheme: CardThemeData(
        color: p.cardBg,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
          side: BorderSide(color: p.border, width: 1),
        ),
      ),
      dialogTheme: DialogThemeData(
        backgroundColor: p.cardBg,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(color: p.border),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          shape: btnShape,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          shape: btnShape,
          side: BorderSide(color: p.border),
          foregroundColor: p.textPrimary,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(shape: btnShape),
      ),
      segmentedButtonTheme: SegmentedButtonThemeData(
        style: ButtonStyle(
          shape: WidgetStatePropertyAll(btnShape),
          side: WidgetStatePropertyAll(BorderSide(color: p.border)),
        ),
      ),
      chipTheme: ChipThemeData(
        backgroundColor: p.subtleBg,
        side: BorderSide(color: p.border),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        labelStyle: TextStyle(color: p.textPrimary, fontSize: 12),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: p.inputBg,
        isDense: true,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
        labelStyle: TextStyle(color: p.textSecondary),
        hintStyle: TextStyle(color: p.textMuted),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: p.border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: p.border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: p.accent, width: 1.5),
        ),
      ),
      dividerTheme: DividerThemeData(
        color: p.border,
        thickness: 1,
        space: 1,
      ),
      tabBarTheme: TabBarThemeData(
        labelColor: p.textPrimary,
        unselectedLabelColor: p.textSecondary,
        indicatorColor: p.accent,
        dividerColor: p.border,
      ),
      snackBarTheme: SnackBarThemeData(
        backgroundColor: isDark ? const Color(0xFF27272A) : const Color(0xFF18181B),
        contentTextStyle: const TextStyle(color: Colors.white),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
          side: BorderSide(color: p.border),
        ),
        behavior: SnackBarBehavior.floating,
      ),
    );
  }

  /// `#RRGGBB` または `#RRGGBBAA` (CSS形式) を Flutter の Color に変換する
  static Color parseCssHex(String? hex, {Color fallback = const Color(0xFF232328)}) {
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
