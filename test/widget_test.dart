import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:stosup/theme/app_theme.dart';
import 'package:stosup/widgets/feature_toggle_banner.dart';
import 'package:stosup/widgets/markdown_notes_view.dart';
import 'package:stosup/widgets/overlay_bg_picker.dart';
import 'package:stosup/widgets/system_font_picker.dart';
import 'package:stosup/widgets/user_avatar.dart';

void main() {
  test('AppTheme CSS HEX カラー変換が正しく動作する', () {
    final c6 = AppTheme.parseCssHex('#38BDF8');
    expect((c6.r * 255).round(), 0x38);
    expect((c6.g * 255).round(), 0xBD);
    expect((c6.b * 255).round(), 0xF8);
    expect((c6.a * 255).round(), 0xFF);

    final c8 = AppTheme.parseCssHex('#065F4680');
    expect((c8.r * 255).round(), 0x06);
    expect((c8.g * 255).round(), 0x5F);
    expect((c8.b * 255).round(), 0x46);
    expect((c8.a * 255).round(), 0x80);

    expect(AppTheme.toCssHex(c6, includeAlpha: false), '#38BDF8');
  });

  testWidgets('MarkdownNotesView が見出しと箇条書きを表示する',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: const Scaffold(
          body: MarkdownNotesView(
            markdown: '### 新機能\n- **YouTube Live 連携**: コメント取得に対応\n- `OBS` オーバーレイ対応',
          ),
        ),
      ),
    );

    expect(find.text('新機能'), findsOneWidget);
    expect(find.textContaining('YouTube Live 連携'), findsOneWidget);
    expect(find.textContaining('OBS'), findsOneWidget);
  });

  testWidgets('UserAvatar が非HTTPのURL時にイニシャルを表示する',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: const Scaffold(
          body: UserAvatar(name: 'たこやき太郎', iconUrl: ''),
        ),
      ),
    );

    expect(find.text('た'), findsOneWidget);
  });

  testWidgets('FeatureToggleSwitch と FeatureDisabledBanner が ON/OFF 操作に応答する',
      (WidgetTester tester) async {
    var enabled = false;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StatefulBuilder(
          builder: (context, setState) => Scaffold(
            body: Column(
              children: [
                FeatureToggleSwitch(
                  label: 'ワードカウント',
                  enabled: enabled,
                  onChanged: (v) => setState(() => enabled = v),
                ),
                if (!enabled)
                  FeatureDisabledBanner(
                    featureName: 'ワードカウント',
                    description: 'テスト説明',
                    onEnable: () => setState(() => enabled = true),
                  ),
              ],
            ),
          ),
        ),
      ),
    );

    expect(find.text('ワードカウント: OFF'), findsOneWidget);
    expect(find.text('「ワードカウント」は現在オフになっています'), findsOneWidget);

    await tester.tap(find.text('オンにする'));
    await tester.pumpAndSettle();

    expect(find.text('ワードカウント: ON'), findsOneWidget);
    expect(find.text('「ワードカウント」は現在オフになっています'), findsNothing);
  });

  testWidgets('ライトテーマとダークテーマの両方で StosupPalette が取得できる',
      (WidgetTester tester) async {
    StosupPalette? lightPalette;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.lightTheme(),
        home: Builder(
          builder: (ctx) {
            lightPalette = ctx.palette;
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    expect(lightPalette?.windowBg, StosupPalette.light.windowBg);

    StosupPalette? darkPalette;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Builder(
          builder: (ctx) {
            darkPalette = ctx.palette;
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(darkPalette?.windowBg, StosupPalette.dark.windowBg);
  });

  testWidgets('OverlayBgPicker が背景色の切り替えイベントを通知する',
      (WidgetTester tester) async {
    var selected = 'transparent';
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StatefulBuilder(
          builder: (context, setState) => Scaffold(
            body: OverlayBgPicker(
              value: selected,
              onChanged: (v) => setState(() => selected = v),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('クロマキー緑'));
    await tester.pumpAndSettle();
    expect(selected, '#00FF00');
  });

  testWidgets('SystemFontPicker がシステムフォント一覧から検索・選択できる',
      (WidgetTester tester) async {
    var selectedFont = '';
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StatefulBuilder(
          builder: (context, setState) => Scaffold(
            body: SystemFontPicker(
              value: selectedFont,
              availableFonts: const ['Meiryo', 'Yu Mincho', 'BIZ UDPゴシック'],
              onChanged: (v) => setState(() => selectedFont = v),
              onRefreshFonts: () {},
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('一覧から選ぶ (3)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('BIZ UDPゴシック'));
    await tester.pumpAndSettle();
    expect(selectedFont, 'BIZ UDPゴシック');
  });
}
