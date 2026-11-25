import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:stosup/screens/users_screen.dart';
import 'package:stosup/src/rust/models.dart';
import 'package:stosup/state/stosup_controller.dart';
import 'package:stosup/theme/app_theme.dart';
import 'package:stosup/widgets/feature_toggle_banner.dart';
import 'package:stosup/widgets/markdown_notes_view.dart';
import 'package:stosup/widgets/overlay_bg_picker.dart';
import 'package:stosup/widgets/system_font_picker.dart';
import 'package:stosup/widgets/user_avatar.dart';

class _FakeUsersController extends StosupController {
  final List<String> deletedIds = [];
  final List<bool> resetKeepBannedCalls = [];

  @override
  Future<void> deleteUser(
    String userId,
    PlatformKind platform, {
    String? displayName,
  }) async {
    deletedIds.add(userId);
    users = users
        .where((u) => !(u.userId == userId && u.platform == platform))
        .toList();
    notifyListeners();
  }

  @override
  Future<void> resetUsers({required bool keepBanned}) async {
    resetKeepBannedCalls.add(keepBanned);
    users = keepBanned ? users.where((u) => u.isBanned).toList() : [];
    notifyListeners();
  }
}

UserRecord _sampleUser(
  String id,
  String name, {
  bool banned = false,
  int lastSeenMs = 1000,
}) {
  return UserRecord(
    userId: id,
    platform: PlatformKind.youTube,
    displayName: name,
    iconUrl: '',
    watchCount: BigInt.from(2),
    totalCommentCount: BigInt.from(5),
    sessionCommentCount: BigInt.from(1),
    inCurrentSession: true,
    isBanned: banned,
    banReason: banned ? 'スパム' : '',
    memo: '',
    firstSeenMs: 500,
    lastSeenMs: lastSeenMs,
  );
}

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

  testWidgets('UsersScreen で個別削除と一括リセット（BAN保持切替）ができる',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = _FakeUsersController()
      ..users = [
        _sampleUser('u1', '太郎', lastSeenMs: 3000),
        _sampleUser('u2', '花子', lastSeenMs: 2000),
        _sampleUser('u3', '荒らし', banned: true, lastSeenMs: 1000),
      ];
    addTearDown(ctrl.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: ListenableBuilder(
            listenable: ctrl,
            builder: (_, _) => UsersScreen(controller: ctrl),
          ),
        ),
      ),
    );

    // 1. 個別削除（先頭行＝太郎を削除）
    expect(find.text('太郎'), findsOneWidget);
    await tester.tap(find.byTooltip('リストから削除').first);
    await tester.pumpAndSettle();
    expect(find.text('「太郎」をリストから削除しますか？'), findsOneWidget);

    await tester.tap(find.text('削除する'));
    await tester.pumpAndSettle();
    expect(ctrl.deletedIds, ['u1']);
    expect(find.text('太郎'), findsNothing);
    expect(find.text('花子'), findsOneWidget);

    // 2. 一括リセット（デフォルトは BAN ユーザーを残す＝1人削除）
    await tester.tap(find.text('一括リセット'));
    await tester.pumpAndSettle();
    expect(find.text('ユーザーリストを一括リセットしますか？'), findsOneWidget);
    expect(find.text('リセットを実行（1人を削除）'), findsOneWidget);

    // チェックを外すと BAN ユーザー含む 2 人が削除対象に変わる
    await tester.tap(find.text('BAN 済みユーザーは残す（推奨）'));
    await tester.pumpAndSettle();
    expect(find.text('リセットを実行（2人を削除）'), findsOneWidget);

    // 再度チェックを ON に戻して実行すると通常ユーザーのみ削除される
    await tester.tap(find.text('BAN 済みユーザーは残す（推奨）'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('リセットを実行（1人を削除）'));
    await tester.pumpAndSettle();

    expect(ctrl.resetKeepBannedCalls, [true]);
    expect(find.text('花子'), findsNothing);
    expect(find.text('荒らし'), findsOneWidget);
  });
}
