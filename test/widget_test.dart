import 'package:flutter/gestures.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:stosup/main.dart';
import 'package:stosup/screens/automation_screen.dart';
import 'package:stosup/screens/comments_screen.dart';
import 'package:stosup/screens/counters_screen.dart';
import 'package:stosup/screens/first_time_screen.dart';
import 'package:stosup/screens/highlight_rules_screen.dart';
import 'package:stosup/screens/lottery_screen.dart';
import 'package:stosup/screens/media_reaction_screen.dart';
import 'package:stosup/screens/mods_screen.dart';
import 'package:stosup/screens/overlay_screen.dart';
import 'package:stosup/screens/poll_screen.dart';
import 'package:stosup/screens/queue_screen.dart';
import 'package:stosup/screens/report_screen.dart';
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
import 'package:stosup/widgets/rich_comment_text.dart';
import 'package:stosup/widgets/system_font_picker.dart';
import 'package:stosup/widgets/update_dialog.dart';
import 'package:stosup/widgets/user_avatar.dart';

class _TestNoFfiController extends StosupController {
  int checkUpdatesCallCount = 0;

  @override
  Future<void> refreshStreamSessions() async {}

  @override
  Future<OnAirReportDto?> loadOnAirReport(String sessionId) async =>
      currentReport;

  @override
  Future<void> updateFeatureFlags(
    FeatureFlags next, {
    String? noticeMessage,
  }) async {
    final prevIncludePre = featureFlags.includePrereleaseUpdates;
    featureFlags = next;
    if (!featureFlags.includePrereleaseUpdates &&
        availableUpdate?.isPrerelease == true) {
      availableUpdate = null;
    }
    notifyListeners();
    if (noticeMessage != null) {
      onNotice?.call('info', noticeMessage);
    }
    if (!prevIncludePre && featureFlags.includePrereleaseUpdates) {
      await checkUpdates(silent: true);
    }
  }

  @override
  Future<UpdateInfo?> checkUpdates({bool silent = false}) async {
    checkUpdatesCallCount++;
    return availableUpdate;
  }
}

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
    expect(find.text('リスナー名簿を一括リセットしますか？'), findsOneWidget);
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
    expect(find.text('エールボックス（スパチャ・メンバー ＆ お礼チェック）'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: PollScreen(controller: ctrl)),
      ),
    );
    expect(find.text('ライブアンケート（リアルタイム投票）'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: QueueScreen(controller: ctrl)),
      ),
    );
    expect(find.text('エントリーレーン（参加型順番待ち管理）'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: MediaReactionScreen(controller: ctrl)),
      ),
    );
    expect(find.text('リアクションステージ（コメント連動アニメーション・動画演出）'), findsOneWidget);
    expect(find.text('順番待ち（1つずつ）'), findsOneWidget);
    expect(find.text('重ね表示（同時再生）'), findsOneWidget);
    expect(find.text('上書き（最新のみ）'), findsOneWidget);
    expect(find.text('おめでとうクラッカー'), findsOneWidget);
    expect(find.text('草シャワー落下'), findsOneWidget);
    expect(find.text('888 紙吹雪'), findsOneWidget);
    expect(find.textContaining('リアクションステージ オーバーレイ背景色'), findsOneWidget);
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
    expect(find.textContaining('ポケットスイッチャー'), findsWidgets);
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

  testWidgets('最小ウィンドウサイズ(1120x720)で全14画面とOBSタブのホイールスクロール・スタイルエディタが崩れない',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1120, 720);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = _TestNoFfiController();
    addTearDown(ctrl.dispose);
    ctrl.isInitialized = true;
    ctrl.appVersion = '0.5.0';
    ctrl.connectionState = const ConnectionStateDto(
      status: ConnectionStatus.connected,
      platform: PlatformKind.demo,
      mode: ConnectionMode.demo,
      streamTitle: 'テスト配信タイトル（長めのタイトルでも省略表示されて崩れない確認）',
      streamId: 'demo-1',
      sessionId: 'sess-1',
      canPostChat: true,
      reconnectAttempt: 0,
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
        modId: '',
        modEffectId: '',
        modParamsJson: '{}',
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
        message: 'こんにちは！テストコメントです！ :stosup_star:',
        messageRuns: const [
          MessageRun(
            kind: MessageRunKind.text,
            text: 'こんにちは！テストコメントです！ ',
            isCustomEmoji: false,
          ),
          MessageRun(
            kind: MessageRunKind.emoji,
            text: ':stosup_star:',
            emojiUrl:
                "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 48 48'><text>★</text></svg>",
            alt: ':stosup_star:',
            isCustomEmoji: true,
          ),
        ],
        timestampMs: 1700000000000,
        isOwner: false,
        isModerator: true,
        isMember: true,
        isBanned: false,
        isNgMasked: false,
        hiddenFromOverlay: false,
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
        messageRuns: const [],
        timestampMs: 1700000000000,
        isChecked: false,
      ),
    ];
    ctrl.streamSessions = [
      StreamSessionSummary(
        sessionId: 'sess-1',
        platform: PlatformKind.demo,
        streamId: 'demo-1',
        title: 'テスト配信',
        startedAtMs: 1700000000000,
        lastCommentAtMs: 1700000180000,
        totalComments: BigInt.from(24),
        uniqueUsers: BigInt.from(10),
        firstTimeUsers: BigInt.from(3),
        monetizationCount: BigInt.from(2),
      ),
    ];
    ctrl.currentReport = OnAirReportDto(
      session: ctrl.streamSessions.first,
      durationMinutes: 4,
      avgCommentsPerMinTenths: 60,
      maxCommentsPerMin: BigInt.from(12),
      memberCommentCount: BigInt.from(8),
      monetizationTotals: const ['¥2,000'],
      buckets: [
        ReportMinuteBucket(
          minuteOffset: 0,
          timestampMs: 1700000000000,
          commentCount: BigInt.from(4),
          monetizationCount: BigInt.zero,
          firstTimeCount: BigInt.from(1),
          sampleMessages: const ['太郎: こんちゃ'],
        ),
        ReportMinuteBucket(
          minuteOffset: 1,
          timestampMs: 1700000060000,
          commentCount: BigInt.from(12),
          monetizationCount: BigInt.from(1),
          firstTimeCount: BigInt.from(2),
          sampleMessages: const ['花子: ナイス！'],
        ),
      ],
      peaks: [
        ReportPeakItem(
          minuteOffset: 1,
          elapsedLabel: '00:01:00',
          timestampMs: 1700000060000,
          commentCount: BigInt.from(12),
          sampleMessages: const ['花子: ナイス！'],
        ),
      ],
      topWords: [
        ReportWordRank(word: '草', count: BigInt.from(9)),
      ],
      topUsers: [
        ReportUserRank(
          userId: 'u-1',
          userName: '常連リスナー太郎',
          commentCount: BigInt.from(7),
          isMember: true,
        ),
      ],
    );
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
      AutomationScreen(controller: ctrl),
      ReportScreen(controller: ctrl),
      SettingsScreen(controller: ctrl),
    ];

    for (final screen in screens) {
      await tester.pumpWidget(
        MaterialApp(
          key: ValueKey(screen.runtimeType),
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
        key: const ValueKey('shell'),
        theme: AppTheme.darkTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    // OBS オーバーレイ画面のスライドバー ＆ マウスホイール横スクロール確認
    await tester.pumpWidget(
      MaterialApp(
        key: const ValueKey('overlay-scroll'),
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

    final tabTextFinder = find.text('トークモニター / CSS');
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

    // タブ切替（トークモニター / CSS）でも崩れず、新5プリセットとビジュアルスタイルエディタが表示されることを確認
    await tester.tap(tabTextFinder);
    await tester.pumpAndSettle();
    expect(find.text('トークモニターオーバーレイのデザイン'), findsOneWidget);
    expect(find.text('プレビューへテスト送信'), findsOneWidget);
    expect(find.text('ブラウザで実寸プレビュー'), findsOneWidget);
    expect(find.textContaining('/overlay/preview/comments'), findsOneWidget);
    expect(find.text('オンエア・テロップ'), findsOneWidget);
    expect(find.text('クリーン・マガジン'), findsOneWidget);
    expect(find.text('タクティカル・HUD'), findsOneWidget);
    expect(find.text('エッジ・グロウ'), findsOneWidget);
    await tester.tap(find.text('エッジ・グロウ'));
    await tester.pumpAndSettle();
    expect(find.text('ビジュアルスタイル・テキスト装飾エディタ（CSS 不要）'), findsOneWidget);
    // A/B/C 各カテゴリの折りたたみドロップダウンが閉じ状態で存在し、展開でパラメータが表示される
    final catA = find.text('A. テキスト・タイポグラフィ');
    final catB = find.text('B. 袋文字（縁取り・二重フチ）・エッジ発光・文字シャドウ');
    final catC = find.text('C. ボックス装飾 ＆ 出現モーション');
    expect(catA, findsOneWidget);
    expect(catB, findsOneWidget);
    expect(catC, findsOneWidget);
    expect(find.text('文字の太さ (Weight)'), findsNothing);
    await tester.ensureVisible(catA);
    await tester.tap(catA);
    await tester.pumpAndSettle();
    expect(find.text('文字の太さ (Weight)'), findsOneWidget);
    await tester.ensureVisible(catB);
    await tester.tap(catB);
    await tester.pumpAndSettle();
    expect(find.text('テキストエッジ発光'), findsOneWidget);
    expect(find.text('発光の広がり'), findsNothing);
    // エッジ発光スイッチ ON で関連パラメータが表示されることを確認
    final glowSwitch = find.text('テキストエッジ発光');
    await tester.ensureVisible(glowSwitch);
    await tester.tap(glowSwitch);
    await tester.pumpAndSettle();
    expect(find.text('発光の広がり'), findsOneWidget);
    expect(find.text('発光強度'), findsOneWidget);
    expect(find.text('エッジ芯の太さ'), findsOneWidget);
    expect(find.text('文字ドロップシャドウ'), findsOneWidget);
    await tester.ensureVisible(catC);
    await tester.tap(catC);
    await tester.pumpAndSettle();
    expect(find.text('出現モーション'), findsOneWidget);

    // 抽選画面のコメント抽選タブ切替でも崩れないことを確認
    await tester.pumpWidget(
      MaterialApp(
        key: const ValueKey('lottery-tab'),
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

  testWidgets('RichCommentText がカスタム絵文字・スタンプをインライン表示できる', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: const Scaffold(
          body: RichCommentText(
            message: 'テスト :star: :gg:',
            runs: [
              MessageRun(
                kind: MessageRunKind.text,
                text: 'テスト ',
                isCustomEmoji: false,
              ),
              MessageRun(
                kind: MessageRunKind.emoji,
                text: ':star:',
                emojiUrl:
                    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg'><rect fill='%23eab308'/><text>★</text></svg>",
                alt: ':star:',
                isCustomEmoji: true,
              ),
              MessageRun(
                kind: MessageRunKind.emoji,
                text: ':gg:',
                emojiUrl:
                    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg'><rect fill='%2310b981'/><text>GG</text></svg>",
                alt: ':gg:',
                isCustomEmoji: true,
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text('★'), findsOneWidget);
    expect(find.text('GG'), findsOneWidget);
  });

  testWidgets('背の高いウィンドウでもスクロール画面の上部に不要な空白が空かない', (tester) async {
    tester.view.physicalSize = const Size(1440, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    final ctrl = StosupController();
    ctrl.isInitialized = true;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.lightTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('連携・オートメーション'));
    await tester.pumpAndSettle();

    final titleFinder =
        find.text('連携・オートメーション（OBS WebSocket / ホットキー / Stream Deck）');
    expect(titleFinder, findsOneWidget);
    final topLeft = tester.getTopLeft(titleFinder);
    expect(topLeft.dy, lessThan(40));
  });

  testWidgets('高頻度コメント受信がフレーム単位で統合され無関係トピックへ波及しない',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    final ctrl = _TestNoFfiController()..isInitialized = true;
    addTearDown(ctrl.dispose);

    var notifyCount = 0;
    ctrl.addListener(() => notifyCount++);

    final initialShellRev = ctrl.topicRevision(StosupTopic.shell);
    final initialUsersRev = ctrl.topicRevision(StosupTopic.users);
    final initialOverlayRev = ctrl.topicRevision(StosupTopic.overlay);
    final initialCommentsRev = ctrl.topicRevision(StosupTopic.comments);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    // 20 件のコメントをバースト注入
    for (var i = 0; i < 20; i++) {
      ctrl.handleEventForTest(
        AppEvent(
          kind: AppEventKind.commentReceived,
          comment: CommentItem(
            id: 'burst-$i',
            sessionId: 's1',
            platform: PlatformKind.demo,
            userId: 'u-$i',
            userName: '視聴者$i',
            userIconUrl: '',
            userMemo: '',
            isFirstTime: false,
            message: 'バーストコメント #$i',
            messageRuns: const [],
            timestampMs: 1700000000000 + i * 10,
            isOwner: false,
            isModerator: false,
            isMember: false,
            isBanned: false,
            isNgMasked: false,
            hiddenFromOverlay: false,
          ),
        ),
      );
    }

    // 16ms 経過前は通知を保留して統合し、経過後に 1 回だけ通知される
    expect(notifyCount, 0);
    await tester.pump(const Duration(milliseconds: 25));
    expect(notifyCount, 1);
    expect(ctrl.comments.length, 20);

    // コメントトピックのみ更新され、シェル・ユーザー・オーバーレイは再ビルド対象外のまま
    expect(ctrl.topicRevision(StosupTopic.comments), initialCommentsRev + 1);
    expect(ctrl.topicRevision(StosupTopic.shell), initialShellRev);
    expect(ctrl.topicRevision(StosupTopic.users), initialUsersRev);
    expect(ctrl.topicRevision(StosupTopic.overlay), initialOverlayRev);

    // カウンター更新イベントは comments と counters のみに限定される
    final prevCountersRev = ctrl.topicRevision(StosupTopic.counters);
    ctrl.handleEventForTest(
      const AppEvent(
        kind: AppEventKind.countersUpdated,
        counters: [],
      ),
    );
    expect(ctrl.topicRevision(StosupTopic.counters), prevCountersRev + 1);
    expect(ctrl.topicRevision(StosupTopic.shell), initialShellRev);
    expect(ctrl.topicRevision(StosupTopic.overlay), initialOverlayRev);

    // NG ワード更新イベントは CommentsScreen の「NG防御 (件数)」バッジへ即座に反映される（トピック漏れ回帰防止）
    expect(find.text('NG防御 (0)'), findsOneWidget);
    final prevShellForNg = ctrl.topicRevision(StosupTopic.shell);
    ctrl.handleEventForTest(
      const AppEvent(
        kind: AppEventKind.ngWordRulesUpdated,
        ngWordRules: [
          NgWordRule(
            id: 'ng-1',
            pattern: 'ネタバレ',
            matchMode: MatchMode.contains,
            caseSensitive: false,
            action: NgWordAction.hideOnOverlay,
            replacement: '***',
            enabled: true,
          ),
          NgWordRule(
            id: 'ng-2',
            pattern: '無効ルール',
            matchMode: MatchMode.contains,
            caseSensitive: false,
            action: NgWordAction.maskText,
            replacement: '***',
            enabled: false,
          ),
        ],
      ),
    );
    await tester.pump();
    expect(find.text('NG防御 (1)'), findsOneWidget);
    expect(ctrl.topicRevision(StosupTopic.shell), prevShellForNg);

    // 従来通りの直接 notifyListeners() 呼び出しは後方互換として全トピックを更新する
    ctrl.notifyListeners();
    expect(ctrl.topicRevision(StosupTopic.shell), initialShellRev + 1);
    expect(ctrl.topicRevision(StosupTopic.users), initialUsersRev + 1);
    expect(ctrl.topicRevision(StosupTopic.overlay), initialOverlayRev + 1);
  });

  testWidgets('LotteryScreen の候補抽出キャッシュが既当選更新イベントで正しく無効化される',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final ctrl = _TestNoFfiController()
      ..isInitialized = true
      ..users = [
        _sampleUser('u1', '太郎', lastSeenMs: 3000),
        _sampleUser('u2', '花子', lastSeenMs: 2000),
      ];
    addTearDown(ctrl.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('ドローボックス'));
    await tester.pumpAndSettle();
    expect(find.textContaining('2 名'), findsWidgets);

    // Web リモコン経由の当選イベントを模擬して既当選 ID を更新すると候補が 1 名へ即時反映される
    ctrl.handleEventForTest(
      const AppEvent(
        kind: AppEventKind.userLotteryWinnersUpdated,
        userLotteryWonIds: ['u1'],
      ),
    );
    await tester.pump();
    expect(find.textContaining('1 名'), findsWidgets);
  });

  testWidgets('pre バージョンは自動更新せず、詳細画面から手動でソフト内更新できる',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    const preInfo = UpdateInfo(
      currentVersion: '0.5.0',
      latestVersion: '0.5.1-pre',
      hasUpdate: true,
      isPrerelease: true,
      releaseName: 'Stosup v0.5.1-pre',
      releaseNotes: '- 試験的機能の追加',
      publishedAt: '2026-11-28T00:00:00Z',
      installerUrl:
          'https://github.com/tako-kun1/stosup/releases/download/v0.5.1-pre/Stosup-v0.5.1-pre-x64.msi',
      installerName: 'Stosup-v0.5.1-pre-x64.msi',
      htmlUrl: 'https://github.com/tako-kun1/stosup/releases/tag/v0.5.1-pre',
    );
    const stableInfo = UpdateInfo(
      currentVersion: '0.5.0',
      latestVersion: '0.5.1',
      hasUpdate: true,
      isPrerelease: false,
      releaseName: 'Stosup v0.5.1',
      releaseNotes: '- 正式リリース',
      publishedAt: '2026-11-28T00:00:00Z',
      installerUrl:
          'https://github.com/tako-kun1/stosup/releases/download/v0.5.1/Stosup-v0.5.1-x64.msi',
      installerName: 'Stosup-v0.5.1-x64.msi',
      htmlUrl: 'https://github.com/tako-kun1/stosup/releases/tag/v0.5.1',
    );

    final ctrl = _TestNoFfiController()
      ..isInitialized = true
      ..availableUpdate = preInfo;
    addTearDown(ctrl.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: StosupShell(controller: ctrl),
      ),
    );
    await tester.pump();

    // 上部通知バー: pre 版は自動更新対象外のアナウンス + 詳細を見るのみ
    expect(find.textContaining('0.5.1-pre'), findsOneWidget);
    expect(find.textContaining('自動更新対象外'), findsOneWidget);
    expect(find.text('詳細を見る'), findsOneWidget);
    expect(find.text('詳細・更新'), findsNothing);
    expect(find.text('PRE'), findsOneWidget);

    // ダイアログを開くと自動更新は行わない旨の注意表示と、ソフト内手動更新ボタン（サイレント既定 OFF）が表示される
    await tester.tap(find.text('詳細を見る'));
    await tester.pumpAndSettle();
    expect(find.textContaining('先行版（pre）のお知らせ'), findsOneWidget);
    expect(find.textContaining('自動更新を行いません'), findsOneWidget);
    expect(find.text('手動でダウンロードして更新'), findsOneWidget);
    expect(find.text('GitHub で開く'), findsOneWidget);
    final preSilentCheckbox = tester.widget<CheckboxListTile>(
      find.widgetWithText(
          CheckboxListTile, 'サイレント更新（完了後に自動で再起動してリリースノートを表示）'),
    );
    expect(preSilentCheckbox.value, isFalse);

    // 前のダイアログを閉じてから、インストーラー URL が無い pre 版の表示を検証
    await tester.tap(find.text('閉じる'));
    await tester.pumpAndSettle();

    // インストーラー URL が無い pre 版の場合は GitHub リリースページ案内にフォールバックする
    const preNoAssetInfo = UpdateInfo(
      currentVersion: '0.5.1',
      latestVersion: '0.6.0-pre01',
      hasUpdate: true,
      isPrerelease: true,
      releaseName: 'Stosup v0.6.0-pre01',
      releaseNotes: '- 試験的機能',
      publishedAt: '2026-11-28T00:00:00Z',
      htmlUrl: 'https://github.com/tako-kun1/stosup/releases/tag/v0.6.0-pre01',
    );
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: UpdateDialog(
            key: const ValueKey('pre-no-asset'),
            controller: ctrl,
            info: preNoAssetInfo,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('手動でダウンロードして更新'), findsNothing);
    expect(find.text('GitHub リリースページで確認'), findsOneWidget);

    // 正式版では従来どおり自動更新 UI（サイレント既定 ON）が表示される
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: UpdateDialog(
            key: const ValueKey('stable'),
            controller: ctrl,
            info: stableInfo,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('アップデート利用可能'), findsOneWidget);
    expect(find.text('ダウンロードして更新'), findsOneWidget);
    final stableSilentCheckbox = tester.widget<CheckboxListTile>(
      find.widgetWithText(
          CheckboxListTile, 'サイレント更新（完了後に自動で再起動してリリースノートを表示）'),
    );
    expect(stableSilentCheckbox.value, isTrue);
  });

  testWidgets('設定画面の先行版（pre）更新アナウンススイッチは既定 OFF で切替・連動が動作する',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    final ctrl = _TestNoFfiController()..isInitialized = true;
    addTearDown(ctrl.dispose);

    // デフォルトでは OFF
    expect(ctrl.featureFlags.includePrereleaseUpdates, isFalse);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(
          body: ListenableBuilder(
            listenable: ctrl,
            builder: (_, _) => SettingsScreen(controller: ctrl),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final switchFinder = find.widgetWithText(
      SwitchListTile,
      '先行版（pre）の更新アナウンスを含める',
    );
    expect(switchFinder, findsOneWidget);
    expect(tester.widget<SwitchListTile>(switchFinder).value, isFalse);

    // スイッチが見える位置までスクロールしてから ON へ切替
    await tester.ensureVisible(switchFinder);
    await tester.pumpAndSettle();
    await tester.tap(switchFinder);
    await tester.pumpAndSettle();

    expect(ctrl.featureFlags.includePrereleaseUpdates, isTrue);
    expect(tester.widget<SwitchListTile>(switchFinder).value, isTrue);
    expect(ctrl.checkUpdatesCallCount, 1);

    // pre 版アナウンス表示中に OFF へ戻すとアナウンスも即座に消去される
    ctrl.availableUpdate = const UpdateInfo(
      currentVersion: '0.5.0',
      latestVersion: '0.5.1-pre',
      hasUpdate: true,
      isPrerelease: true,
      releaseName: 'Stosup v0.5.1-pre',
      releaseNotes: '- 試験的機能',
      publishedAt: '2026-11-28T00:00:00Z',
      htmlUrl: 'https://github.com/tako-kun1/stosup/releases/tag/v0.5.1-pre',
    );
    await tester.tap(switchFinder);
    await tester.pumpAndSettle();

    expect(ctrl.featureFlags.includePrereleaseUpdates, isFalse);
    expect(tester.widget<SwitchListTile>(switchFinder).value, isFalse);
    expect(ctrl.availableUpdate, isNull);

    // 「すべて ON」一括操作でも先行版アナウンス設定は勝手に ON にならない
    await ctrl.setAllFeaturesEnabled(true);
    await tester.pumpAndSettle();
    expect(ctrl.featureFlags.includePrereleaseUpdates, isFalse);
  });

  testWidgets('MOD管理画面とリアクション・オーバーレイスタジオのMOD連動UIが正常動作する',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    final ctrl = _TestNoFfiController()
      ..isInitialized = true
      ..modsFolderPath = r'C:\Users\demo\AppData\Roaming\Stosup\mods'
      ..installedMods = const [
        InstalledMod(
          id: 'starlight-pack',
          name: 'スターライト演出パック',
          version: '1.0.0',
          author: 'ModAuthor',
          description: '流星演出・レトロテーマ・盛り上がりゲージの統合サンプル',
          minStosupVersion: '0.5.1',
          homepage: '',
          license: 'MIT',
          iconUrl: '',
          folderName: 'starlight-pack',
          enabled: true,
          installedAtMs: 1700000000000,
          loadError: null,
          reactionEffects: [
            ModReactionEffectDef(
              modId: 'starlight-pack',
              modName: 'スターライト演出パック',
              effectId: 'starlight_burst',
              name: 'スターライト・バースト',
              description: '流星群と光の輪',
              entry: 'effect.js',
              css: '',
              defaultDurationMs: 4200,
              params: [
                ModParamSchema(
                  key: 'starCount',
                  label: '流星の本数',
                  kind: ModParamKind.number,
                  defaultValue: '24',
                  min: 6,
                  max: 80,
                  step: 2,
                  options: [],
                  description: '星の数',
                ),
                ModParamSchema(
                  key: 'primaryColor',
                  label: 'メイン発光色',
                  kind: ModParamKind.color,
                  defaultValue: '#38BDF8',
                  options: [],
                  description: '',
                ),
              ],
            ),
          ],
          overlayThemes: [
            ModOverlayThemeDef(
              modId: 'starlight-pack',
              modName: 'スターライト演出パック',
              themeId: 'retro_arcade',
              fullThemeId: 'mod:starlight-pack:retro_arcade',
              name: 'レトロ・アーケード',
              description: 'アーケード筐体風デザイン',
              targets: ['comments', 'superchat'],
              css: 'theme.css',
              script: '',
              previewBg: '#111322',
              previewFg: '#F8FAFC',
              previewBorder: '#F43F5E',
            ),
          ],
          customOverlays: [
            ModCustomOverlayDef(
              modId: 'starlight-pack',
              modName: 'スターライト演出パック',
              overlayId: 'hype_meter',
              name: 'ハイプメーター',
              description: '盛り上がりゲージ',
              entry: 'overlay.html',
              channels: ['comments', 'superchat'],
              recommendedWidth: 640,
              recommendedHeight: 220,
              overlayPath: '/overlay/mod/starlight-pack/hype_meter',
              previewPath: '/overlay/preview/mod/starlight-pack/hype_meter',
            ),
          ],
          logicScripts: [
            ModLogicScriptDef(
              modId: 'starlight-pack',
              modName: 'スターライト演出パック',
              scriptId: 'combo_fever',
              name: 'コンボ・フィーバー制御',
              description: '草コメント3連打でスターライト演出を自動発火',
              entry: 'logic.rhai',
              events: ['comment', 'superchat'],
            ),
          ],
        ),
      ]
      ..modScriptLogs = const [
        ModScriptLogEntry(
          timestampMs: 1700000000000,
          modId: 'starlight-pack',
          scriptId: 'combo_fever',
          level: 'action',
          message: 'MOD演出発火 (starlight-pack:starlight_burst / 「3コンボ達成！」)',
        ),
      ];
    addTearDown(ctrl.dispose);

    // 1. MOD 管理画面で 4 種類すべてのコンポーネントとスクリプト実行ログが表示される
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: ModsScreen(controller: ctrl)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('MOD 管理（演出・テーマ・独自オーバーレイ・ロジックスクリプト）'), findsOneWidget);
    expect(find.text('スターライト演出パック'), findsOneWidget);
    expect(find.textContaining('スターライト・バースト'), findsOneWidget);
    expect(find.textContaining('レトロ・アーケード'), findsOneWidget);
    expect(find.textContaining('ハイプメーター'), findsOneWidget);
    expect(find.textContaining('コンボ・フィーバー制御'), findsOneWidget);
    expect(find.text('テスト実行'), findsOneWidget);
    expect(find.textContaining('スクリプト実行ログ（デバッグコンソール）'), findsOneWidget);
    expect(find.textContaining('3コンボ達成！'), findsOneWidget);
    expect(find.text('OBS URL コピー'), findsOneWidget);

    // 2. オーバーレイスタジオの「トークモニター」タブに MOD 追加テーマが表示され、URL タブに独自オーバーレイも表示される
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: OverlayScreen(controller: ctrl)),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('MOD 独自オーバーレイ'), findsOneWidget);
    expect(find.textContaining('ハイプメーター'), findsOneWidget);
    await tester.tap(find.text('トークモニター / CSS'));
    await tester.pumpAndSettle();
    expect(find.text('レトロ・アーケード'), findsOneWidget);
    expect(find.text('MOD: スターライト演出パック'), findsOneWidget);

    // 3. リアクションステージのルール追加ダイアログで「カスタム MOD」を選ぶと MOD パラメータ入力欄が生成される
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: MediaReactionScreen(controller: ctrl)),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('演出を追加'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('カスタム MOD'));
    await tester.pumpAndSettle();
    expect(find.text('MOD カスタムパラメータ'), findsOneWidget);
    expect(find.textContaining('流星の本数'), findsOneWidget);
    expect(find.textContaining('メイン発光色'), findsOneWidget);
  });

  testWidgets(
      'OverlayScreen でプリセット標準に戻す→保存→同一設定イベント再到着後も再編集が巻き戻らず保存でき、外部ポート変更も同期される',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final savedConfigs = <OverlayConfig>[];
    final ctrl = _OverlaySaveCaptureController(savedConfigs);
    addTearDown(ctrl.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.darkTheme(),
        home: Scaffold(body: OverlayScreen(controller: ctrl)),
      ),
    );
    await tester.pumpAndSettle();

    // トークモニター / CSS タブを開く
    await tester.tap(find.text('トークモニター / CSS'));
    await tester.pumpAndSettle();

    // 1. レシピ適用 → プリセット標準に戻す → 保存して即時反映
    await tester.ensureVisible(find.text('バラエティ二重フチ'));
    await tester.tap(find.text('バラエティ二重フチ'));
    await tester.pumpAndSettle();
    expect(find.text('カスタム有効'), findsOneWidget);

    await tester.ensureVisible(find.text('プリセット標準に戻す'));
    await tester.tap(find.text('プリセット標準に戻す'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('保存して即時反映'));
    await tester.pumpAndSettle();
    expect(savedConfigs.length, 1);
    expect(savedConfigs.last.visualStyles['comments']?.enabled, isFalse);

    // 2. 直後に別レシピを適用し、そこへ Rust 側から直前保存と同一内容（Map 参照違い）のイベントが遅れて届いても巻き戻らないこと
    await tester.ensureVisible(find.text('テレビ字幕（白＋黒フチ）'));
    await tester.tap(find.text('テレビ字幕（白＋黒フチ）'));
    await tester.pumpAndSettle();
    expect(find.text('カスタム有効'), findsOneWidget);

    final dupSameAsFirstSave = OverlayConfig(
      port: savedConfigs.first.port,
      commentFontSize: savedConfigs.first.commentFontSize,
      commentMaxItems: savedConfigs.first.commentMaxItems,
      showIcons: savedConfigs.first.showIcons,
      showTimestamps: savedConfigs.first.showTimestamps,
      defaultBubbleBg: savedConfigs.first.defaultBubbleBg,
      defaultTextColor: savedConfigs.first.defaultTextColor,
      commentsTheme: savedConfigs.first.commentsTheme,
      lotteryTheme: savedConfigs.first.lotteryTheme,
      counterTheme: savedConfigs.first.counterTheme,
      firstTimeTheme: savedConfigs.first.firstTimeTheme,
      superchatTheme: savedConfigs.first.superchatTheme,
      pollTheme: savedConfigs.first.pollTheme,
      pickupTheme: savedConfigs.first.pickupTheme,
      queueTheme: savedConfigs.first.queueTheme,
      mediaTheme: savedConfigs.first.mediaTheme,
      commentsBgColor: savedConfigs.first.commentsBgColor,
      lotteryBgColor: savedConfigs.first.lotteryBgColor,
      counterBgColor: savedConfigs.first.counterBgColor,
      firstTimeBgColor: savedConfigs.first.firstTimeBgColor,
      superchatBgColor: savedConfigs.first.superchatBgColor,
      pollBgColor: savedConfigs.first.pollBgColor,
      pickupBgColor: savedConfigs.first.pickupBgColor,
      queueBgColor: savedConfigs.first.queueBgColor,
      mediaBgColor: savedConfigs.first.mediaBgColor,
      commentsFontFamily: savedConfigs.first.commentsFontFamily,
      lotteryFontFamily: savedConfigs.first.lotteryFontFamily,
      counterFontFamily: savedConfigs.first.counterFontFamily,
      firstTimeFontFamily: savedConfigs.first.firstTimeFontFamily,
      superchatFontFamily: savedConfigs.first.superchatFontFamily,
      pollFontFamily: savedConfigs.first.pollFontFamily,
      pickupFontFamily: savedConfigs.first.pickupFontFamily,
      queueFontFamily: savedConfigs.first.queueFontFamily,
      mediaFontFamily: savedConfigs.first.mediaFontFamily,
      commentsCustomCss: savedConfigs.first.commentsCustomCss,
      lotteryCustomCss: savedConfigs.first.lotteryCustomCss,
      counterCustomCss: savedConfigs.first.counterCustomCss,
      firstTimeCustomCss: savedConfigs.first.firstTimeCustomCss,
      superchatCustomCss: savedConfigs.first.superchatCustomCss,
      pollCustomCss: savedConfigs.first.pollCustomCss,
      pickupCustomCss: savedConfigs.first.pickupCustomCss,
      queueCustomCss: savedConfigs.first.queueCustomCss,
      mediaCustomCss: savedConfigs.first.mediaCustomCss,
      visualStyles:
          Map<String, OverlayVisualStyle>.from(savedConfigs.first.visualStyles),
    );
    ctrl.handleEventForTest(
      AppEvent(
        kind: AppEventKind.overlayConfigUpdated,
        overlayConfig: dupSameAsFirstSave,
      ),
    );
    await tester.pumpAndSettle();

    // 巻き戻らず「カスタム有効」が維持されており、保存すると enabled == true が送信される
    expect(find.text('カスタム有効'), findsOneWidget);
    await tester.tap(find.text('保存して即時反映'));
    await tester.pumpAndSettle();
    expect(savedConfigs.length, 2);
    expect(savedConfigs.last.visualStyles['comments']?.enabled, isTrue);

    // 3. 外部イベントでポートが変わった場合は即座に同期され、次の保存で古いポートを送り返さない
    final portChangedCfg = OverlayConfig(
      port: 29876,
      commentFontSize: savedConfigs.last.commentFontSize,
      commentMaxItems: savedConfigs.last.commentMaxItems,
      showIcons: savedConfigs.last.showIcons,
      showTimestamps: savedConfigs.last.showTimestamps,
      defaultBubbleBg: savedConfigs.last.defaultBubbleBg,
      defaultTextColor: savedConfigs.last.defaultTextColor,
      commentsTheme: savedConfigs.last.commentsTheme,
      lotteryTheme: savedConfigs.last.lotteryTheme,
      counterTheme: savedConfigs.last.counterTheme,
      firstTimeTheme: savedConfigs.last.firstTimeTheme,
      superchatTheme: savedConfigs.last.superchatTheme,
      pollTheme: savedConfigs.last.pollTheme,
      pickupTheme: savedConfigs.last.pickupTheme,
      queueTheme: savedConfigs.last.queueTheme,
      mediaTheme: savedConfigs.last.mediaTheme,
      commentsBgColor: savedConfigs.last.commentsBgColor,
      lotteryBgColor: savedConfigs.last.lotteryBgColor,
      counterBgColor: savedConfigs.last.counterBgColor,
      firstTimeBgColor: savedConfigs.last.firstTimeBgColor,
      superchatBgColor: savedConfigs.last.superchatBgColor,
      pollBgColor: savedConfigs.last.pollBgColor,
      pickupBgColor: savedConfigs.last.pickupBgColor,
      queueBgColor: savedConfigs.last.queueBgColor,
      mediaBgColor: savedConfigs.last.mediaBgColor,
      commentsFontFamily: savedConfigs.last.commentsFontFamily,
      lotteryFontFamily: savedConfigs.last.lotteryFontFamily,
      counterFontFamily: savedConfigs.last.counterFontFamily,
      firstTimeFontFamily: savedConfigs.last.firstTimeFontFamily,
      superchatFontFamily: savedConfigs.last.superchatFontFamily,
      pollFontFamily: savedConfigs.last.pollFontFamily,
      pickupFontFamily: savedConfigs.last.pickupFontFamily,
      queueFontFamily: savedConfigs.last.queueFontFamily,
      mediaFontFamily: savedConfigs.last.mediaFontFamily,
      commentsCustomCss: savedConfigs.last.commentsCustomCss,
      lotteryCustomCss: savedConfigs.last.lotteryCustomCss,
      counterCustomCss: savedConfigs.last.counterCustomCss,
      firstTimeCustomCss: savedConfigs.last.firstTimeCustomCss,
      superchatCustomCss: savedConfigs.last.superchatCustomCss,
      pollCustomCss: savedConfigs.last.pollCustomCss,
      pickupCustomCss: savedConfigs.last.pickupCustomCss,
      queueCustomCss: savedConfigs.last.queueCustomCss,
      mediaCustomCss: savedConfigs.last.mediaCustomCss,
      visualStyles:
          Map<String, OverlayVisualStyle>.from(savedConfigs.last.visualStyles),
    );
    ctrl.handleEventForTest(
      AppEvent(
        kind: AppEventKind.overlayConfigUpdated,
        overlayConfig: portChangedCfg,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('保存して即時反映'));
    await tester.pumpAndSettle();
    expect(savedConfigs.last.port, 29876);
  });
}

class _OverlaySaveCaptureController extends StosupController {
  _OverlaySaveCaptureController(this.savedConfigs);

  final List<OverlayConfig> savedConfigs;

  @override
  Future<void> saveOverlayConfig(OverlayConfig config) async {
    savedConfigs.add(config);
    overlayConfig = config;
    notifyListeners();
  }
}
