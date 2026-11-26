import 'package:flutter/gestures.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:stosup/main.dart';
import 'package:stosup/screens/comments_screen.dart';
import 'package:stosup/screens/counters_screen.dart';
import 'package:stosup/screens/first_time_screen.dart';
import 'package:stosup/screens/highlight_rules_screen.dart';
import 'package:stosup/screens/lottery_screen.dart';
import 'package:stosup/screens/media_reaction_screen.dart';
import 'package:stosup/screens/overlay_screen.dart';
import 'package:stosup/screens/poll_screen.dart';
import 'package:stosup/screens/queue_screen.dart';
import 'package:stosup/screens/settings_screen.dart';
import 'package:stosup/screens/superchat_screen.dart';
import 'package:stosup/screens/users_screen.dart';
import 'package:stosup/src/rust/models.dart';
import 'package:stosup/state/stosup_controller.dart';
import 'package:stosup/theme/app_theme.dart';
import 'package:stosup/widgets/feature_toggle_banner.dart';
import 'package:stosup/widgets/license_agreement_screen.dart';
import 'package:stosup/widgets/markdown_notes_view.dart';
import 'package:stosup/widgets/overlay_bg_picker.dart';
import 'package:stosup/widgets/qr_code_view.dart';
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

  testWidgets('新機能 4 画面（スパチャ/投票/参加型キュー/ワード演出）が正常に描画される',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1440, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = StosupController();
    addTearDown(ctrl.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: SuperchatScreen(controller: ctrl)),
      ),
    );
    expect(find.textContaining('スーパーチャット / メンバーシップ'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: PollScreen(controller: ctrl)),
      ),
    );
    expect(find.text('リアルタイム投票 / アンケート'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: QueueScreen(controller: ctrl)),
      ),
    );
    expect(find.text('参加型キュー（順番待ちリスト）管理'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: MediaReactionScreen(controller: ctrl)),
      ),
    );
    expect(find.textContaining('WordParty 演出エディタ'), findsOneWidget);
    expect(find.text('順番待ち（1つずつ）'), findsOneWidget);
    expect(find.text('重ね表示（同時再生）'), findsOneWidget);
    expect(find.text('上書き（最新のみ）'), findsOneWidget);
    expect(find.text('おめでとうクラッカー'), findsOneWidget);
    expect(find.text('草シャワー落下'), findsOneWidget);
    expect(find.text('888 紙吹雪'), findsOneWidget);
    expect(find.textContaining('WordParty オーバーレイ背景色'), findsOneWidget);
    expect(find.text('カラー選択'), findsOneWidget);

    ctrl.appVersion = '0.4.0';
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: SettingsScreen(controller: ctrl)),
      ),
    );
    expect(find.text('ソフトウェア情報'), findsOneWidget);
    expect(find.text('Stosup：v0.4.0'), findsOneWidget);
    expect(find.text('アップデートチェック'), findsOneWidget);
    expect(find.text('リリースノートを表示'), findsOneWidget);
    expect(find.text('利用規約を表示'), findsOneWidget);
    expect(find.text('GitHub リポジトリを開く'), findsOneWidget);
    expect(find.text('ローカルネットワーク (LAN) 接続 ＆ Web リモコン'), findsOneWidget);
    expect(find.text('ローカルネットワーク内の他端末からの接続を許可する'), findsOneWidget);

    // LAN 公開 ON 状態（QR コード・LAN URL・ファイアウォール手順）の描画検証
    ctrl.remoteAccessStatus = const RemoteAccessStatus(
      config: RemoteAccessConfig(
        allowLanAccess: true,
        remoteControlEnabled: true,
        requirePin: true,
        pinCode: '123456',
        embedPinInQr: true,
      ),
      serverRunning: true,
      port: 18765,
      boundAddress: '0.0.0.0:18765',
      localIps: ['192.168.1.50'],
      localUrl: 'http://127.0.0.1:18765/remote',
      lanUrls: ['http://192.168.1.50:18765/remote'],
      primaryRemoteUrl: 'http://192.168.1.50:18765/remote',
      qrTargetUrl: 'http://192.168.1.50:18765/remote?pin=123456',
      qrCode: QrCodeMatrix(
        size: 3,
        modules: [true, false, true, false, true, false, true, false, true],
      ),
    );
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: SettingsScreen(controller: ctrl)),
      ),
    );
    expect(find.text('LAN 公開中 (0.0.0.0:18765)'), findsOneWidget);
    expect(find.text('http://192.168.1.50:18765/remote'), findsOneWidget);
    expect(find.text('http://127.0.0.1:18765/remote'), findsNothing);
    expect(find.text('許可コマンドをコピー'), findsOneWidget);
    expect(find.byType(QrCodeView), findsOneWidget);
  });

  testWidgets('LicenseAgreementScreen で規約同意・不同意操作が正しく機能する',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1280, 860);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = StosupController();
    addTearDown(ctrl.dispose);
    ctrl.licenseStatus = const LicenseAgreementStatus(
      title: 'Stosup ソフトウェア利用規約（使用許諾契約書）',
      markdownBody: '# Stosup 規約\n\n## 第1条（定義）\nテスト条文です。',
      revisionLabel: 'v1.0 (2026-11-26)',
      contentHash: 'abc123',
      source: 'bundled',
      acceptedHash: 'oldhash',
      acceptedRevision: 'v0.9',
      needsAcceptance: true,
      isUpdatedLicense: true,
    );

    var declined = false;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: LicenseAgreementScreen(
          controller: ctrl,
          onDecline: () => declined = true,
        ),
      ),
    );

    expect(find.textContaining('利用規約が更新されました'), findsOneWidget);
    expect(find.text('v1.0 (2026-11-26)'), findsOneWidget);
    expect(find.text('第1条（定義）'), findsOneWidget);

    // チェック前は「同意して Stosup を起動する」ボタンが無効
    final acceptBtnFinder = find.widgetWithText(
      FilledButton,
      '同意して Stosup を起動する',
    );
    expect(tester.widget<FilledButton>(acceptBtnFinder).onPressed, isNull);

    // チェックボックスを ON にするとボタンが有効化される
    await tester.tap(find.byType(CheckboxListTile));
    await tester.pump();
    expect(tester.widget<FilledButton>(acceptBtnFinder).onPressed, isNotNull);

    // 不同意ボタンで終了コールバックが呼ばれる
    await tester.tap(find.text('同意しない（アプリを終了）'));
    await tester.pump();
    expect(declined, isTrue);

    // 初期化済みコントローラーを渡した StosupApp は初回フレームから即座に規約画面を描画する
    ctrl.isInitialized = true;
    ctrl.themeMode = AppThemeMode.light;
    await tester.pumpWidget(StosupApp(controller: ctrl));
    expect(find.byType(LicenseAgreementScreen), findsOneWidget);
    expect(find.text('Stosup を起動しています...'), findsNothing);
  });

  testWidgets('最小ウィンドウサイズ(1120x720)で全12画面とOBSタブのホイールスクロールが崩れない',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1120, 720);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = StosupController();
    addTearDown(ctrl.dispose);
    ctrl.isInitialized = true;
    ctrl.appVersion = '0.4.3';
    ctrl.connectionState = const ConnectionStateDto(
      status: ConnectionStatus.connected,
      platform: PlatformKind.demo,
      mode: ConnectionMode.demo,
      streamTitle: 'テスト配信タイトル（長めのタイトルでも省略表示されて崩れない確認）',
      streamId: 'demo-1',
      sessionId: 'sess-1',
      canPostChat: true,
    );
    ctrl.mediaReactionRules = [
      MediaReactionRule(
        id: 'mr-1',
        name: 'おめでとうクラッカー',
        pattern: 'おめでとう\nおめ',
        matchMode: MatchMode.contains,
        caseSensitive: false,
        effectType: MediaEffectType.cracker,
        triggerType: MediaTriggerType.keyword,
        minSuperchatAmount: BigInt.zero,
        priority: 20,
        mediaKind: MediaKind.image,
        storedFileName: '',
        originalFileName: '',
        soundStoredFileName: '',
        soundOriginalFileName: '',
        soundVolume: 80,
        animation: MediaAnimationKind.popIn,
        position: MediaPosition.center,
        durationMs: 4000,
        widthPercent: 45,
        volume: 80,
        particleCount: 30,
        particleSize: 60,
        particleSpeed: 100,
        gravity: 100,
        particleText: '🎉,✨',
        ambientPreset: 'snow',
        cooldownSec: 2,
        excludeBanned: true,
        captionTemplate: '',
        enabled: true,
      ),
    ];
    ctrl.comments = [
      CommentItem(
        id: 'c-1',
        sessionId: 'sess-1',
        platform: PlatformKind.demo,
        userId: 'u-1',
        userName: '常連リスナー太郎',
        userIconUrl: '',
        userMemo: 'イラスト担当',
        isFirstTime: true,
        message: 'こんにちは！テストコメントです！',
        timestampMs: 1700000000000,
        isOwner: false,
        isModerator: true,
        isMember: true,
        isBanned: false,
        amountDisplay: '¥1,000',
      ),
    ];
    ctrl.monetizationItems = [
      MonetizationItem(
        id: 'm-1',
        sessionId: 'sess-1',
        platform: PlatformKind.demo,
        kind: MonetizationEventKind.superChat,
        userId: 'u-1',
        userName: '応援リスナー',
        userIconUrl: '',
        amountDisplay: '¥1,000',
        amountMicros: BigInt.from(1000000000),
        currency: 'JPY',
        tierColor: '#FFCA28',
        message: '応援しています！',
        timestampMs: 1700000000000,
        isChecked: false,
      ),
    ];
    ctrl.highlightRules = [
      const HighlightRule(
        id: 'h-1',
        name: '質問ハイライト',
        pattern: '質問',
        matchMode: MatchMode.contains,
        caseSensitive: false,
        bgColor: '#FEF08A',
        textColor: '#18181B',
        borderColor: '#EAB308',
        priority: 10,
        enabled: true,
      ),
    ];
    ctrl.wordCounters = [
      WordCounterConfig(
        id: 'wc-1',
        name: '草カウンター',
        keywords: const ['草', 'www'],
        matchMode: MatchMode.contains,
        caseSensitive: false,
        excludeBanned: true,
        accentColor: '#22C55E',
        enabled: true,
        totalCount: BigInt.from(12),
        uniqueUserCount: BigInt.from(5),
      ),
    ];

    final screens = <Widget>[
      CommentsScreen(controller: ctrl),
      SuperchatScreen(controller: ctrl),
      HighlightRulesScreen(controller: ctrl),
      MediaReactionScreen(controller: ctrl),
      CountersScreen(controller: ctrl),
      FirstTimeScreen(controller: ctrl),
      PollScreen(controller: ctrl),
      QueueScreen(controller: ctrl),
      LotteryScreen(controller: ctrl),
      UsersScreen(controller: ctrl),
      OverlayScreen(controller: ctrl),
      SettingsScreen(controller: ctrl),
    ];

    for (final screen in screens) {
      await tester.pumpWidget(
        MaterialApp(
          theme: AppTheme.darkTheme(),
          home: Scaffold(
            body: Row(
              children: [
                const SizedBox(width: 212),
                Expanded(child: screen),
              ],
            ),
          ),
        ),
      );
      await tester.pump();
    }

    // StosupShell 全体描画確認
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    // OBS オーバーレイ画面のスライドバー ＆ マウスホイール横スクロール確認
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: Row(
            children: [
              const SizedBox(width: 212),
              Expanded(child: OverlayScreen(controller: ctrl)),
            ],
          ),
        ),
      ),
    );
    await tester.pump();

    final tabTextFinder = find.text('コメント表示 / CSS');
    expect(tabTextFinder, findsOneWidget);
    final scrollableFinder = find.ancestor(
      of: tabTextFinder,
      matching: find.byType(SingleChildScrollView),
    );
    final beforeScroll =
        tester.widget<SingleChildScrollView>(scrollableFinder).controller!.offset;

    final center = tester.getCenter(tabTextFinder);
    final pointer = TestPointer(1, PointerDeviceKind.mouse);
    await tester.sendEventToBinding(pointer.hover(center));
    await tester.sendEventToBinding(pointer.scroll(const Offset(0, 120)));
    await tester.pump();

    final afterScroll =
        tester.widget<SingleChildScrollView>(scrollableFinder).controller!.offset;
    expect(afterScroll, greaterThan(beforeScroll));

    // タブ切替（コメント表示 / CSS）でも崩れないことを確認
    await tester.tap(tabTextFinder);
    await tester.pumpAndSettle();
    expect(find.text('コメント表示オーバーレイのデザイン'), findsOneWidget);

    // 抽選画面のコメント抽選タブ切替でも崩れないことを確認
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: Row(
            children: [
              const SizedBox(width: 212),
              Expanded(child: LotteryScreen(controller: ctrl)),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(find.text('コメント抽選（開始〜終了の間の特定ワード）'));
    await tester.pumpAndSettle();
    expect(find.text('1. 受付開始'), findsOneWidget);
  });
}
