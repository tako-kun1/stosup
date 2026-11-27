import 'dart:async';
import 'dart:io' show exit;

import 'package:flutter/foundation.dart';

import '../src/rust/api.dart' as api;
import '../src/rust/frb_generated.dart';
import '../src/rust/models.dart';

/// UI 再ビルドのスコープを画面・役割ごとに分離するための通知トピック
enum StosupTopic {
  shell,
  comments,
  superchat,
  highlight,
  media,
  counters,
  firstTime,
  poll,
  queue,
  lottery,
  users,
  overlay,
  automation,
  report,
  settings,
}

/// Stosup アプリ全体の状態を管理するコントローラー
class StosupController extends ChangeNotifier {
  final List<int> _topicRevisions =
      List<int>.filled(StosupTopic.values.length, 0);

  /// 指定トピックの変更リビジョンを取得（不変ならその領域の UI 再ビルドを省略できる）
  int topicRevision(StosupTopic topic) => _topicRevisions[topic.index];

  void _notifyTopics(List<StosupTopic> topics) {
    for (final t in topics) {
      _topicRevisions[t.index]++;
    }
    super.notifyListeners();
  }

  /// 既存の notifyListeners() 呼び出しやテストサブクラスとの完全な後方互換性を維持
  @override
  void notifyListeners() {
    for (var i = 0; i < _topicRevisions.length; i++) {
      _topicRevisions[i]++;
    }
    super.notifyListeners();
  }
  bool isInitialized = false;
  String? initError;
  String appVersion = '0.1.0';
  LicenseAgreementStatus? licenseStatus;
  bool get needsLicenseAcceptance => licenseStatus?.needsAcceptance ?? false;
  bool _postLicenseStartupDone = false;

  ConnectionStateDto connectionState = const ConnectionStateDto(
    status: ConnectionStatus.disconnected,
    platform: PlatformKind.youTube,
    mode: ConnectionMode.youTubeInnerTube,
    streamTitle: '',
    streamId: '',
    sessionId: '',
    canPostChat: false,
    reconnectAttempt: 0,
  );

  List<CommentItem> comments = [];
  List<HighlightRule> highlightRules = [];
  List<WordCounterConfig> wordCounters = [];
  List<UserRecord> users = [];
  List<String> userLotteryWonIds = [];
  LotteryResult? lastUserLotteryResult;
  List<LotteryResult> lotteryHistory = [];
  List<ConnectionHistoryItem> connectionHistory = [];
  List<SavedChannelItem> savedChannels = [];
  List<NgWordRule> ngWordRules = [];
  ObsWsConfig obsWsConfig = const ObsWsConfig(
    enabled: false,
    host: '127.0.0.1',
    port: 4455,
    password: '',
    autoConnect: false,
    resetFirstTimeOnStreamStart: false,
    resetCountersOnStreamStart: false,
    rules: [],
  );
  ObsWsStatus obsWsStatus = const ObsWsStatus(
    connected: false,
    connecting: false,
    obsVersion: '',
    currentScene: '',
    scenes: [],
    sceneSources: {},
    isStreaming: false,
  );
  List<HotkeyBinding> hotkeyBindings = [];
  List<StreamProfileSummary> streamProfiles = [];
  List<StreamSessionSummary> streamSessions = [];
  OnAirReportDto? currentReport;

  CommentLotteryState commentLottery = const CommentLotteryState(
    isAccepting: false,
    config: CommentLotteryConfig(
      startMessage: '【抽選開始】「参加」とコメントして応募してください！',
      endMessage: '【抽選終了】応募を締め切りました！',
      targetKeyword: '参加',
      matchMode: MatchMode.contains,
      caseSensitive: false,
      oneEntryPerUser: true,
      repeatMode: LotteryRepeatMode.onceOnly,
      winnerCount: 1,
      postToChat: true,
    ),
    entries: [],
    alreadyWonUserIds: [],
  );

  OverlayConfig overlayConfig = const OverlayConfig(
    port: 18765,
    commentFontSize: 18,
    commentMaxItems: 30,
    showIcons: true,
    showTimestamps: false,
    defaultBubbleBg: '#202124E6',
    defaultTextColor: '#F4F4F5',
    commentsTheme: 'broadcast',
    lotteryTheme: 'broadcast',
    counterTheme: 'broadcast',
    firstTimeTheme: 'broadcast',
    superchatTheme: 'broadcast',
    pollTheme: 'broadcast',
    pickupTheme: 'broadcast',
    queueTheme: 'broadcast',
    mediaTheme: 'broadcast',
    commentsBgColor: 'transparent',
    lotteryBgColor: 'transparent',
    counterBgColor: 'transparent',
    firstTimeBgColor: 'transparent',
    superchatBgColor: 'transparent',
    pollBgColor: 'transparent',
    pickupBgColor: 'transparent',
    queueBgColor: 'transparent',
    mediaBgColor: 'transparent',
    commentsFontFamily: '',
    lotteryFontFamily: '',
    counterFontFamily: '',
    firstTimeFontFamily: '',
    superchatFontFamily: '',
    pollFontFamily: '',
    pickupFontFamily: '',
    queueFontFamily: '',
    mediaFontFamily: '',
    commentsCustomCss: '',
    lotteryCustomCss: '',
    counterCustomCss: '',
    firstTimeCustomCss: '',
    superchatCustomCss: '',
    pollCustomCss: '',
    pickupCustomCss: '',
    queueCustomCss: '',
    mediaCustomCss: '',
    visualStyles: {},
  );

  List<String> systemFonts = [];

  AppThemeMode themeMode = AppThemeMode.dark;

  List<MonetizationItem> monetizationItems = [];

  PollState pollState = PollState(
    isActive: false,
    config: PollConfig(
      question: '次の企画はどれがいい？',
      options: [
        PollOption(
          id: 'opt-1',
          label: '雑談トーク',
          keywords: const ['1', '雑談'],
          color: '#3B82F6',
          voteCount: BigInt.zero,
        ),
        PollOption(
          id: 'opt-2',
          label: '参加型ゲーム',
          keywords: const ['2', 'ゲーム'],
          color: '#22C55E',
          voteCount: BigInt.zero,
        ),
      ],
      allowVoteChange: true,
      excludeBanned: true,
      showPercentage: true,
      showVoteCount: true,
    ),
    totalVotes: BigInt.zero,
  );

  PickedCommentState pickedComment = const PickedCommentState();

  QueueState queueState = const QueueState(
    config: QueueConfig(
      isAccepting: false,
      maxActiveSlots: 3,
      joinKeywords: ['参加希望', '!join'],
      leaveKeywords: ['辞退', 'キャンセル', '!leave'],
      sortMode: QueueSortMode.firstCome,
      excludeBanned: true,
      allowRejoinAfterComplete: true,
    ),
    entries: [],
  );

  List<MediaReactionRule> mediaReactionRules = [];

  MediaReactionConfig mediaReactionConfig = const MediaReactionConfig(
    playbackMode: MediaPlaybackMode.overlay,
    masterVolume: 80,
    maxConcurrent: 8,
    allowMultipleMatch: true,
  );

  MediaTriggerEvent? lastMediaTrigger;

  RemoteAccessStatus remoteAccessStatus = const RemoteAccessStatus(
    config: RemoteAccessConfig(
      allowLanAccess: true,
      remoteControlEnabled: true,
      requirePin: true,
      pinCode: '123456',
      embedPinInQr: true,
    ),
    serverRunning: false,
    port: 18765,
    boundAddress: '0.0.0.0:18765',
    localIps: [],
    localUrl: 'http://127.0.0.1:18765/remote',
    lanUrls: [],
    primaryRemoteUrl: 'http://127.0.0.1:18765/remote',
    qrTargetUrl: 'http://127.0.0.1:18765/remote',
  );

  int get unreadMonetizationCount =>
      monetizationItems.where((e) => !e.isChecked).length;

  FirstTimeCounterState firstTimeState = FirstTimeCounterState(
    count: BigInt.zero,
    totalDetectedAllTime: BigInt.zero,
    recentUsers: const [],
    config: const FirstTimeCounterConfig(
      detectUnregistered: true,
      detectKeyword: true,
      keywords: ['初見', 'しょけん'],
      autoResetOnNewStream: true,
      showFirstTimerProfile: false,
      excludeBanned: true,
      accentColor: '#16A34A',
    ),
  );

  FeatureFlags featureFlags = const FeatureFlags(
    commentDisplayEnabled: true,
    wordHighlightEnabled: true,
    wordCounterEnabled: true,
    userLotteryEnabled: true,
    commentLotteryEnabled: true,
    userTrackingEnabled: true,
    userMemoDisplayEnabled: true,
    firstTimeCounterEnabled: true,
    superchatEnabled: true,
    pollEnabled: true,
    pickupEnabled: true,
    queueEnabled: true,
    mediaReactionEnabled: true,
    ngWordFilterEnabled: true,
    obsOverlayEnabled: true,
    autoUpdateCheckEnabled: true,
  );

  YouTubeOAuthStatus oauthStatus = const YouTubeOAuthStatus(
    clientId: '',
    hasClientSecret: false,
    isAuthenticated: false,
  );

  UpdateInfo? availableUpdate;
  ReleaseNotesInfo? pendingStartupReleaseNotes;

  bool isConnecting = false;
  bool isOAuthInProgress = false;
  bool isCheckingUpdate = false;

  /// UI 側でトーストやエラー通知を表示するためのコールバック
  void Function(String level, String message)? onNotice;

  StreamSubscription<AppEvent>? _eventSub;
  Timer? _usersRefreshDebounce;
  Timer? _commentNotifyTimer;
  bool _usersViewActive = false;
  bool _usersDirty = false;
  bool _isDisposed = false;
  // 起動後の遅延非同期ロードがユーザー操作やリアルタイム更新による最新状態を古い値で上書きしないための世代番号
  int _usersEpoch = 0;
  int _lotteryWonEpoch = 0;
  int _lotteryHistoryEpoch = 0;
  int _fontsEpoch = 0;
  int _remoteEpoch = 0;

  /// ユーザーリスト / ユーザー抽選画面を表示中かどうかを切り替え、非表示時の不要な全件同期を抑止する
  void setUsersViewActive(bool active) {
    if (_usersViewActive == active) return;
    _usersViewActive = active;
    if (active && _usersDirty) {
      _usersDirty = false;
      _usersRefreshDebounce?.cancel();
      unawaited(refreshUsersNow());
    }
  }

  Future<void> refreshUsersNow() async {
    try {
      final fetched = await api.listUsers();
      if (_isDisposed) return;
      users = fetched;
      _usersEpoch++;
      _usersDirty = false;
      _notifyTopics(const [StosupTopic.lottery, StosupTopic.users]);
    } catch (_) {}
  }

  Future<void>? _initFuture;

  Future<void> initialize({required bool afterUpdate}) {
    if (isInitialized) return Future<void>.value();
    _initFuture ??= _initializeInternal(afterUpdate: afterUpdate);
    return _initFuture!;
  }

  Future<void> _initializeInternal({required bool afterUpdate}) async {
    try {
      await RustLib.init();
      await api.initApp(afterUpdate: afterUpdate);
      appVersion = api.getAppVersion();

      // 初回ウィンドウ表示前はローカル高速データとローカル規約判定のみ完了させ、
      // オンライン通信待機や重い一覧取得（フォント列挙・ipconfig・全ユーザー）による起動遅延を撤廃する
      await Future.wait([
        _refreshStartupCore(),
        _loadInitialLicenseStatusLocal(),
      ]);

      _eventSub?.cancel();
      _eventSub = api.subscribeEvents().listen(_handleEvent);

      isInitialized = true;
      notifyListeners();

      // 初回描画後に重い補助データとオンライン規約更新チェックを非同期で完了させる
      unawaited(_loadDeferredStartupData());
      unawaited(_checkOnlineLicenseInBackground());

      // 利用規約に同意済みの場合のみ通常の起動後処理（リリースノート・更新確認）を実行
      if (!needsLicenseAcceptance) {
        _runPostLicenseStartup();
      }
    } catch (e) {
      initError = e.toString();
      notifyListeners();
    }
  }

  Future<void> _loadInitialLicenseStatusLocal() async {
    licenseStatus = await api.checkLicenseStatus(checkOnline: false);
  }

  /// 起動表示をブロックせずに GitHub 上の最新利用規約を確認し、改定があれば再同意画面へ切替える
  Future<void> _checkOnlineLicenseInBackground() async {
    try {
      final prevHash = licenseStatus?.contentHash;
      final prevNeeds = needsLicenseAcceptance;
      final next = await api.checkLicenseStatus(checkOnline: true);
      if (_isDisposed) return;
      licenseStatus = next;
      if (next.contentHash != prevHash || needsLicenseAcceptance != prevNeeds) {
        _notifyTopics(const [StosupTopic.shell, StosupTopic.settings]);
      }
    } catch (_) {}
  }

  /// 初回フレーム描画に必要な軽量ローカル状態のみを起動時に読み込む
  Future<void> _refreshStartupCore() async {
    final results = await Future.wait([
      api.getConnectionState(),
      api.getRecentComments(),
      api.listHighlightRules(),
      api.listWordCounters(),
      api.getCommentLotteryState(),
      api.getOverlayConfig(),
      api.getYoutubeOauthStatus(),
      api.getFeatureFlags(),
      api.getThemeMode(),
      api.getFirstTimeState(),
      api.listMonetizationEvents(limit: 150),
      api.getPollState(),
      api.getPickedComment(),
      api.getQueueState(),
      api.listMediaReactionRules(),
      api.getMediaReactionConfig(),
      api.listConnectionHistory(),
      api.listSavedChannels(),
      api.listNgWordRules(),
      api.getObsWsConfig(),
      api.getObsWsStatus(),
      api.listHotkeyBindings(),
      api.listStreamProfiles(),
    ]);

    connectionState = results[0] as ConnectionStateDto;
    comments = List<CommentItem>.from(results[1] as List<CommentItem>);
    highlightRules = results[2] as List<HighlightRule>;
    wordCounters = results[3] as List<WordCounterConfig>;
    commentLottery = results[4] as CommentLotteryState;
    overlayConfig = results[5] as OverlayConfig;
    oauthStatus = results[6] as YouTubeOAuthStatus;
    featureFlags = results[7] as FeatureFlags;
    themeMode = results[8] as AppThemeMode;
    firstTimeState = results[9] as FirstTimeCounterState;
    monetizationItems = results[10] as List<MonetizationItem>;
    pollState = results[11] as PollState;
    pickedComment = results[12] as PickedCommentState;
    queueState = results[13] as QueueState;
    mediaReactionRules = results[14] as List<MediaReactionRule>;
    mediaReactionConfig = results[15] as MediaReactionConfig;
    connectionHistory = results[16] as List<ConnectionHistoryItem>;
    savedChannels = results[17] as List<SavedChannelItem>;
    ngWordRules = results[18] as List<NgWordRule>;
    obsWsConfig = results[19] as ObsWsConfig;
    obsWsStatus = results[20] as ObsWsStatus;
    hotkeyBindings = results[21] as List<HotkeyBinding>;
    streamProfiles = results[22] as List<StreamProfileSummary>;
  }

  /// 起動直後のバックグラウンドで重い補助データ（フォント列挙・ipconfig・全ユーザー・抽選履歴）を個別に非同期ロード
  /// 各取得を独立実行して一部失敗の連鎖と遅い処理の道連れを防ぎ、取得中に更新された最新状態は世代番号で保護する
  Future<void> _loadDeferredStartupData() async {
    final usersEpoch = _usersEpoch;
    final wonEpoch = _lotteryWonEpoch;
    final historyEpoch = _lotteryHistoryEpoch;
    final fontsEpoch = _fontsEpoch;
    final remoteEpoch = _remoteEpoch;

    unawaited(() async {
      try {
        final fetched = await api.listUsers();
        if (_isDisposed || _usersEpoch != usersEpoch) return;
        users = fetched;
        _usersEpoch++;
        _notifyTopics(const [StosupTopic.lottery, StosupTopic.users]);
      } catch (_) {}
    }());

    unawaited(() async {
      try {
        final fetched = await api.getUserLotteryWonIds();
        if (_isDisposed || _lotteryWonEpoch != wonEpoch) return;
        userLotteryWonIds = fetched;
        _lotteryWonEpoch++;
        _notifyTopics(const [StosupTopic.lottery]);
      } catch (_) {}
    }());

    unawaited(() async {
      try {
        final fetched = await api.listLotteryHistory(limit: 30);
        if (_isDisposed || _lotteryHistoryEpoch != historyEpoch) return;
        lotteryHistory = fetched;
        _lotteryHistoryEpoch++;
        _notifyTopics(const [StosupTopic.lottery]);
      } catch (_) {}
    }());

    unawaited(() async {
      try {
        final fetched = await api.listSystemFonts();
        if (_isDisposed || _fontsEpoch != fontsEpoch) return;
        systemFonts = fetched;
        _fontsEpoch++;
        _notifyTopics(const [StosupTopic.overlay]);
      } catch (_) {}
    }());

    unawaited(() async {
      try {
        final fetched = await api.getRemoteAccessStatus();
        if (_isDisposed || _remoteEpoch != remoteEpoch) return;
        remoteAccessStatus = fetched;
        _remoteEpoch++;
        _notifyTopics(const [StosupTopic.shell, StosupTopic.settings]);
      } catch (_) {}
    }());
  }

  void _runPostLicenseStartup() {
    if (_postLicenseStartupDone) return;
    _postLicenseStartupDone = true;
    // アップデート後初回起動時のリリースノート判定
    unawaited(_checkStartupReleaseNotes());
    // 自動アップデート確認機能が ON の場合のみバックグラウンドで最新版チェック
    if (featureFlags.autoUpdateCheckEnabled) {
      unawaited(checkUpdates(silent: true));
    }
  }

  /// 利用規約の最新状態を確認する（GitHub 上の最新 License.md のオンライン取得に対応）
  Future<void> refreshLicenseStatus({
    bool checkOnline = true,
    bool notifyResult = false,
  }) async {
    try {
      final prevHash = licenseStatus?.contentHash;
      licenseStatus = await api.checkLicenseStatus(checkOnline: checkOnline);
      notifyListeners();
      if (notifyResult) {
        if (needsLicenseAcceptance) {
          onNotice?.call(
            'warning',
            '利用規約が更新されています。内容をご確認のうえ再度同意してください。',
          );
        } else if (prevHash != null &&
            prevHash == licenseStatus?.contentHash) {
          onNotice?.call(
            'info',
            '利用規約は最新版（${licenseStatus?.revisionLabel ?? ""}）に同意済みです',
          );
        }
      }
    } catch (e) {
      onNotice?.call('error', '利用規約の確認に失敗しました: $e');
    }
  }

  /// 現在の利用規約への同意を記録して通常起動へ進む
  Future<void> acceptLicenseAgreement() async {
    final status = licenseStatus;
    if (status == null) return;
    try {
      licenseStatus = await api.acceptLicenseAgreement(
        contentHash: status.contentHash,
        revisionLabel: status.revisionLabel,
      );
      notifyListeners();
      _runPostLicenseStartup();
      onNotice?.call('success', '利用規約（${status.revisionLabel}）に同意しました');
    } catch (e) {
      onNotice?.call('error', '利用規約の同意保存に失敗しました: $e');
    }
  }

  /// 利用規約に不同意の場合、同意を保存せず即座にプロセスを終了（タスクキル）する
  void declineLicenseAndExit() {
    try {
      api.terminateAppProcess();
    } catch (_) {}
    exit(0);
  }

  Future<void> refreshAll({bool notify = true}) async {
    final shouldFetchFonts = systemFonts.isEmpty;
    final results = await Future.wait([
      api.getConnectionState(),
      api.getRecentComments(),
      api.listHighlightRules(),
      api.listWordCounters(),
      api.listUsers(),
      api.getCommentLotteryState(),
      api.getUserLotteryWonIds(),
      api.listLotteryHistory(limit: 30),
      api.getOverlayConfig(),
      api.getYoutubeOauthStatus(),
      api.getFeatureFlags(),
      api.getThemeMode(),
      api.getFirstTimeState(),
      shouldFetchFonts ? api.listSystemFonts() : Future<List<String>>.value(systemFonts),
      api.listMonetizationEvents(limit: 150),
      api.getPollState(),
      api.getPickedComment(),
      api.getQueueState(),
      api.listMediaReactionRules(),
      api.getMediaReactionConfig(),
      api.getRemoteAccessStatus(),
      api.listConnectionHistory(),
      api.listSavedChannels(),
      api.listNgWordRules(),
      api.getObsWsConfig(),
      api.getObsWsStatus(),
      api.listHotkeyBindings(),
      api.listStreamProfiles(),
      api.listStreamSessions(limit: 40),
    ]);

    connectionState = results[0] as ConnectionStateDto;
    comments = List<CommentItem>.from(results[1] as List<CommentItem>);
    highlightRules = results[2] as List<HighlightRule>;
    wordCounters = results[3] as List<WordCounterConfig>;
    users = results[4] as List<UserRecord>;
    _usersEpoch++;
    commentLottery = results[5] as CommentLotteryState;
    userLotteryWonIds = results[6] as List<String>;
    _lotteryWonEpoch++;
    lotteryHistory = results[7] as List<LotteryResult>;
    _lotteryHistoryEpoch++;
    overlayConfig = results[8] as OverlayConfig;
    oauthStatus = results[9] as YouTubeOAuthStatus;
    featureFlags = results[10] as FeatureFlags;
    themeMode = results[11] as AppThemeMode;
    firstTimeState = results[12] as FirstTimeCounterState;
    systemFonts = results[13] as List<String>;
    if (shouldFetchFonts) _fontsEpoch++;
    monetizationItems = results[14] as List<MonetizationItem>;
    pollState = results[15] as PollState;
    pickedComment = results[16] as PickedCommentState;
    queueState = results[17] as QueueState;
    mediaReactionRules = results[18] as List<MediaReactionRule>;
    mediaReactionConfig = results[19] as MediaReactionConfig;
    remoteAccessStatus = results[20] as RemoteAccessStatus;
    _remoteEpoch++;
    connectionHistory = results[21] as List<ConnectionHistoryItem>;
    savedChannels = results[22] as List<SavedChannelItem>;
    ngWordRules = results[23] as List<NgWordRule>;
    obsWsConfig = results[24] as ObsWsConfig;
    obsWsStatus = results[25] as ObsWsStatus;
    hotkeyBindings = results[26] as List<HotkeyBinding>;
    streamProfiles = results[27] as List<StreamProfileSummary>;
    streamSessions = results[28] as List<StreamSessionSummary>;
    if (notify) {
      notifyListeners();
    }
  }

  void _scheduleCommentNotify() {
    // バースト受信したコメント通知を最大 16ms（約 60fps）間隔に統合し、1 フレーム内の多重再ビルドを防ぐ
    if (_commentNotifyTimer != null) return;
    _commentNotifyTimer = Timer(const Duration(milliseconds: 16), () {
      _commentNotifyTimer = null;
      if (!_isDisposed) {
        _notifyTopics(const [StosupTopic.comments]);
      }
    });
  }

  /// テスト用エントリーポイント：イベント受信時のトピック通知挙動を検証する
  @visibleForTesting
  void handleEventForTest(AppEvent ev) => _handleEvent(ev);

  void _handleEvent(AppEvent ev) {
    switch (ev.kind) {
      case AppEventKind.commentReceived:
        if (ev.comment != null) {
          comments.add(ev.comment!);
          if (comments.length > 400) {
            comments.removeRange(0, comments.length - 400);
          }
          _scheduleCommentNotify();
        }
      case AppEventKind.commentsReplaced:
        if (ev.comments != null) {
          _commentNotifyTimer?.cancel();
          _commentNotifyTimer = null;
          comments = List<CommentItem>.from(ev.comments!);
          _notifyTopics(const [StosupTopic.comments]);
        }
      case AppEventKind.connectionChanged:
        if (ev.connection != null) {
          connectionState = ev.connection!;
          isConnecting = connectionState.status == ConnectionStatus.connecting;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.comments]);
        }
      case AppEventKind.countersUpdated:
        if (ev.counters != null) {
          wordCounters = ev.counters!;
          _notifyTopics(const [StosupTopic.comments, StosupTopic.counters]);
        }
      case AppEventKind.firstTimeUpdated:
        if (ev.firstTimeState != null) {
          firstTimeState = ev.firstTimeState!;
          _notifyTopics(const [
            StosupTopic.shell,
            StosupTopic.comments,
            StosupTopic.firstTime,
          ]);
        }
      case AppEventKind.usersUpdated:
        _scheduleUsersRefresh();
      case AppEventKind.commentLotteryUpdated:
        if (ev.commentLottery != null) {
          commentLottery = ev.commentLottery!;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.lottery]);
        }
      case AppEventKind.lotteryDrawn:
        if (ev.lotteryResult != null) {
          final res = ev.lotteryResult!;
          if (res.kind == 'user') {
            lastUserLotteryResult = res;
          }
          lotteryHistory = [res, ...lotteryHistory.take(29)];
          _lotteryHistoryEpoch++;
          _notifyTopics(const [StosupTopic.lottery]);
        }
      case AppEventKind.monetizationUpdated:
        if (ev.monetizationItems != null) {
          monetizationItems = ev.monetizationItems!;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.superchat]);
        }
      case AppEventKind.pollUpdated:
        if (ev.pollState != null) {
          pollState = ev.pollState!;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.poll]);
        }
      case AppEventKind.pickedCommentUpdated:
        if (ev.pickedComment != null) {
          pickedComment = ev.pickedComment!;
          _notifyTopics(const [StosupTopic.comments, StosupTopic.superchat]);
        }
      case AppEventKind.queueUpdated:
        if (ev.queueState != null) {
          queueState = ev.queueState!;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.queue]);
        }
      case AppEventKind.mediaTriggered:
        if (ev.mediaTrigger != null) {
          lastMediaTrigger = ev.mediaTrigger!;
          _notifyTopics(const [StosupTopic.media]);
        }
      case AppEventKind.mediaRulesUpdated:
        if (ev.mediaRules != null) {
          mediaReactionRules = List<MediaReactionRule>.from(ev.mediaRules!);
          _notifyTopics(const [StosupTopic.media, StosupTopic.automation]);
        }
      case AppEventKind.mediaConfigUpdated:
        if (ev.mediaConfig != null) {
          mediaReactionConfig = ev.mediaConfig!;
          _notifyTopics(const [StosupTopic.media]);
        }
      case AppEventKind.highlightRulesUpdated:
        if (ev.highlightRules != null) {
          highlightRules = List<HighlightRule>.from(ev.highlightRules!);
          _notifyTopics(const [StosupTopic.highlight]);
        }
      case AppEventKind.ngWordRulesUpdated:
        if (ev.ngWordRules != null) {
          ngWordRules = List<NgWordRule>.from(ev.ngWordRules!);
          // CommentsScreen 上の「NG防御 (件数)」バッジとダイアログへ即時反映する
          _notifyTopics(const [StosupTopic.comments, StosupTopic.highlight]);
        }
      case AppEventKind.overlayConfigUpdated:
        if (ev.overlayConfig != null) {
          overlayConfig = ev.overlayConfig!;
          _notifyTopics(const [
            StosupTopic.shell,
            StosupTopic.overlay,
            StosupTopic.automation,
            StosupTopic.media,
            StosupTopic.counters,
            StosupTopic.firstTime,
            StosupTopic.superchat,
            StosupTopic.poll,
            StosupTopic.queue,
            StosupTopic.lottery,
          ]);
        }
      case AppEventKind.userLotteryWinnersUpdated:
        if (ev.userLotteryWonIds != null) {
          userLotteryWonIds = List<String>.from(ev.userLotteryWonIds!);
          _lotteryWonEpoch++;
          _notifyTopics(const [StosupTopic.lottery]);
        }
      case AppEventKind.featureFlagsUpdated:
        if (ev.featureFlags != null) {
          featureFlags = ev.featureFlags!;
          notifyListeners();
        }
      case AppEventKind.obsWsStatusChanged:
        if (ev.obsWsStatus != null) {
          obsWsStatus = ev.obsWsStatus!;
          _notifyTopics(const [StosupTopic.shell, StosupTopic.automation]);
        }
      case AppEventKind.profilesUpdated:
        if (ev.profiles != null) {
          streamProfiles = List<StreamProfileSummary>.from(ev.profiles!);
          _notifyTopics(const [
            StosupTopic.shell,
            StosupTopic.automation,
            StosupTopic.settings,
          ]);
        }
      case AppEventKind.connectionHistoryUpdated:
        unawaited(refreshConnectionHistory());
      case AppEventKind.systemNotice:
        if (ev.noticeMessage != null) {
          onNotice?.call(ev.noticeLevel ?? 'info', ev.noticeMessage!);
        }
    }
  }

  void _scheduleUsersRefresh() {
    // ユーザーリスト／抽選画面を非表示中は FFI 全件取得を保留し、画面を開いた瞬間に同期する
    if (!_usersViewActive) {
      _usersDirty = true;
      return;
    }
    _usersRefreshDebounce?.cancel();
    _usersRefreshDebounce = Timer(const Duration(milliseconds: 400), () async {
      await refreshUsersNow();
    });
  }

  Future<void> _checkStartupReleaseNotes() async {
    try {
      final notes = await api.getStartupReleaseNotes();
      if (notes.shouldShowOnStartup) {
        pendingStartupReleaseNotes = notes;
        notifyListeners();
      }
    } catch (_) {}
  }

  Future<void> dismissStartupReleaseNotes() async {
    pendingStartupReleaseNotes = null;
    notifyListeners();
    try {
      await api.markReleaseNotesSeen();
    } catch (_) {}
  }

  Future<ReleaseNotesInfo> fetchCurrentReleaseNotes() async {
    return api.getCurrentReleaseNotes();
  }

  // ==================== 配信接続・コメント ====================

  Future<void> connectYouTube(String urlOrId, ConnectionMode mode) async {
    isConnecting = true;
    notifyListeners();
    try {
      connectionState = await api.connectYoutube(
        urlOrVideoId: urlOrId,
        mode: mode,
      );
      users = await api.listUsers();
      _usersEpoch++;
      onNotice?.call('success', 'YouTube Live に接続しました: ${connectionState.streamTitle}');
    } catch (e) {
      onNotice?.call('error', e.toString());
    } finally {
      isConnecting = false;
      notifyListeners();
    }
  }

  Future<void> connectDemo({
    String sessionName = '',
    bool autoGenerate = true,
  }) async {
    isConnecting = true;
    notifyListeners();
    try {
      connectionState = await api.connectDemo(
        sessionName: sessionName,
        autoGenerate: autoGenerate,
      );
      users = await api.listUsers();
      _usersEpoch++;
      onNotice?.call('success', 'デモ配信モードを開始しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    } finally {
      isConnecting = false;
      notifyListeners();
    }
  }

  Future<void> disconnect() async {
    try {
      await api.disconnectStream();
      onNotice?.call('info', '配信から切断しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearComments() async {
    _commentNotifyTimer?.cancel();
    _commentNotifyTimer = null;
    await api.clearRecentComments();
    comments = [];
    _notifyTopics(const [StosupTopic.comments]);
  }

  Future<void> sendDemoComment(
      String userId, String userName, String message) async {
    try {
      await api.sendDemoComment(
        userId: userId,
        userName: userName,
        message: message,
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> postChat(String text) async {
    try {
      final res = await api.postChatMessage(text: text);
      onNotice?.call('success', res);
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ワード反応ルール ====================

  Future<void> saveHighlightRule(HighlightRule rule) async {
    try {
      highlightRules = await api.saveHighlightRule(rule: rule);
      notifyListeners();
      onNotice?.call('success', 'ハイライトサインのルールを保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteHighlightRule(String id) async {
    try {
      highlightRules = await api.deleteHighlightRule(id: id);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ワードカウンター ====================

  Future<void> saveWordCounter(WordCounterConfig counter) async {
    try {
      wordCounters = await api.saveWordCounter(counter: counter);
      notifyListeners();
      onNotice?.call('success', 'コールカウンターを保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteWordCounter(String id) async {
    try {
      wordCounters = await api.deleteWordCounter(id: id);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetWordCounter(String id) async {
    try {
      wordCounters = await api.resetWordCounter(id: id);
      notifyListeners();
      onNotice?.call('info', 'カウンターをリセットしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> adjustWordCounter(String id, int delta) async {
    try {
      wordCounters = await api.adjustWordCounter(id: id, delta: delta);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ユーザー & BAN ====================

  Future<void> setUserBan(
    String userId,
    PlatformKind platform,
    bool banned,
    String reason,
  ) async {
    try {
      users = await api.setUserBan(
        userId: userId,
        platform: platform,
        banned: banned,
        reason: reason,
      );
      _usersEpoch++;
      notifyListeners();
      onNotice?.call(
        banned ? 'warning' : 'info',
        banned ? 'ユーザーをアプリ内BANしました（抽選・カウント対象外）' : 'ユーザーのBANを解除しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> setUserMemo(
      String userId, PlatformKind platform, String memo) async {
    try {
      users = await api.setUserMemo(
        userId: userId,
        platform: platform,
        memo: memo,
      );
      _usersEpoch++;
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteUser(
    String userId,
    PlatformKind platform, {
    String? displayName,
  }) async {
    try {
      users = await api.deleteUser(userId: userId, platform: platform);
      _usersEpoch++;
      userLotteryWonIds = await api.getUserLotteryWonIds();
      _lotteryWonEpoch++;
      firstTimeState = await api.getFirstTimeState();
      notifyListeners();
      onNotice?.call(
        'info',
        displayName != null
            ? '「$displayName」をリスナー名簿から削除しました'
            : 'リスナー名簿から削除しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetUsers({required bool keepBanned}) async {
    try {
      users = await api.resetUsers(keepBanned: keepBanned);
      _usersEpoch++;
      userLotteryWonIds = await api.getUserLotteryWonIds();
      _lotteryWonEpoch++;
      firstTimeState = await api.getFirstTimeState();
      notifyListeners();
      onNotice?.call(
        'warning',
        keepBanned
            ? 'リスナー名簿を一括リセットしました（BAN済みユーザーは維持）'
            : 'BAN済みユーザーを含め、リスナー名簿をすべてリセットしました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ユーザー抽選 ====================

  Future<LotteryResult?> drawUserLottery(UserLotteryRequest req) async {
    try {
      final res = await api.drawUserLottery(req: req);
      lastUserLotteryResult = res;
      userLotteryWonIds = await api.getUserLotteryWonIds();
      _lotteryWonEpoch++;
      notifyListeners();
      return res;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<void> resetUserLotteryWinners() async {
    await api.resetUserLotteryWinners();
    userLotteryWonIds = [];
    _lotteryWonEpoch++;
    notifyListeners();
    onNotice?.call('info', 'ユーザー抽選の既当選リストをリセットしました');
  }

  Future<void> clearOverlayLottery() async {
    await api.clearOverlayLottery();
    onNotice?.call('info', 'OBS ドローボックスの表示をクリアしました');
  }

  // ==================== コメント抽選 ====================

  Future<void> startCommentLottery(CommentLotteryConfig config) async {
    try {
      commentLottery = await api.startCommentLottery(config: config);
      notifyListeners();
      onNotice?.call(
        'success',
        commentLottery.lastPostStatus ?? 'コメント抽選の受付を開始しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> stopCommentLottery() async {
    try {
      commentLottery = await api.stopCommentLottery();
      notifyListeners();
      onNotice?.call(
        'info',
        commentLottery.lastPostStatus ?? 'コメント抽選の受付を終了しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<LotteryResult?> drawCommentLottery() async {
    try {
      final res = await api.drawCommentLottery();
      commentLottery = await api.getCommentLotteryState();
      notifyListeners();
      return res;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<void> resetCommentLottery({required bool clearWinnersOnly}) async {
    try {
      commentLottery =
          await api.resetCommentLottery(clearWinnersOnly: clearWinnersOnly);
      notifyListeners();
      onNotice?.call(
        'info',
        clearWinnersOnly ? '既当選リストをリセットしました' : 'コメント抽選をリセットしました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 各機能の有効/無効（ON/OFF） ====================

  Future<void> updateFeatureFlags(
    FeatureFlags next, {
    String? noticeMessage,
  }) async {
    try {
      final prevObsEnabled = featureFlags.obsOverlayEnabled;
      final prevPort = overlayConfig.port;
      featureFlags = await api.saveFeatureFlags(flags: next);
      // OBS サーバー起動/停止が絡む場合のみポート・LAN 状態を再同期（通常トグル時の ipconfig 起動を回避）
      if (featureFlags.obsOverlayEnabled != prevObsEnabled) {
        overlayConfig = await api.getOverlayConfig();
        remoteAccessStatus = await api.getRemoteAccessStatus();
        _remoteEpoch++;
      } else if (overlayConfig.port != prevPort) {
        remoteAccessStatus = await api.getRemoteAccessStatus();
        _remoteEpoch++;
      }
      notifyListeners();
      if (noticeMessage != null) {
        onNotice?.call('info', noticeMessage);
      }
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> setFeatureEnabled({
    bool? commentDisplay,
    bool? wordHighlight,
    bool? wordCounter,
    bool? userLottery,
    bool? commentLottery,
    bool? userTracking,
    bool? userMemoDisplay,
    bool? firstTimeCounter,
    bool? superchat,
    bool? poll,
    bool? pickup,
    bool? queue,
    bool? mediaReaction,
    bool? ngWordFilter,
    bool? obsOverlay,
    bool? autoUpdateCheck,
    String? featureLabel,
  }) async {
    final next = FeatureFlags(
      commentDisplayEnabled:
          commentDisplay ?? featureFlags.commentDisplayEnabled,
      wordHighlightEnabled: wordHighlight ?? featureFlags.wordHighlightEnabled,
      wordCounterEnabled: wordCounter ?? featureFlags.wordCounterEnabled,
      userLotteryEnabled: userLottery ?? featureFlags.userLotteryEnabled,
      commentLotteryEnabled:
          commentLottery ?? featureFlags.commentLotteryEnabled,
      userTrackingEnabled: userTracking ?? featureFlags.userTrackingEnabled,
      userMemoDisplayEnabled:
          userMemoDisplay ?? featureFlags.userMemoDisplayEnabled,
      firstTimeCounterEnabled:
          firstTimeCounter ?? featureFlags.firstTimeCounterEnabled,
      superchatEnabled: superchat ?? featureFlags.superchatEnabled,
      pollEnabled: poll ?? featureFlags.pollEnabled,
      pickupEnabled: pickup ?? featureFlags.pickupEnabled,
      queueEnabled: queue ?? featureFlags.queueEnabled,
      mediaReactionEnabled:
          mediaReaction ?? featureFlags.mediaReactionEnabled,
      ngWordFilterEnabled: ngWordFilter ?? featureFlags.ngWordFilterEnabled,
      obsOverlayEnabled: obsOverlay ?? featureFlags.obsOverlayEnabled,
      autoUpdateCheckEnabled:
          autoUpdateCheck ?? featureFlags.autoUpdateCheckEnabled,
    );
    final switchedValue = commentDisplay ??
        wordHighlight ??
        wordCounter ??
        userLottery ??
        commentLottery ??
        userTracking ??
        userMemoDisplay ??
        firstTimeCounter ??
        superchat ??
        poll ??
        pickup ??
        queue ??
        mediaReaction ??
        ngWordFilter ??
        obsOverlay ??
        autoUpdateCheck;
    final msg = (featureLabel != null && switchedValue != null)
        ? '「$featureLabel」を ${switchedValue ? "オン" : "オフ"} にしました'
        : null;
    await updateFeatureFlags(next, noticeMessage: msg);
  }

  Future<void> setAllFeaturesEnabled(bool enabled) async {
    final next = FeatureFlags(
      commentDisplayEnabled: enabled,
      wordHighlightEnabled: enabled,
      wordCounterEnabled: enabled,
      userLotteryEnabled: enabled,
      commentLotteryEnabled: enabled,
      userTrackingEnabled: enabled,
      userMemoDisplayEnabled: enabled,
      firstTimeCounterEnabled: enabled,
      superchatEnabled: enabled,
      pollEnabled: enabled,
      pickupEnabled: enabled,
      queueEnabled: enabled,
      mediaReactionEnabled: enabled,
      ngWordFilterEnabled: enabled,
      obsOverlayEnabled: enabled,
      autoUpdateCheckEnabled: enabled,
    );
    await updateFeatureFlags(
      next,
      noticeMessage: enabled ? 'すべての機能をオンにしました' : 'すべての機能をオフにしました',
    );
  }

  // ==================== スーパーチャット / メンバーシップ ====================

  Future<void> setMonetizationChecked(String id, bool checked) async {
    try {
      monetizationItems =
          await api.setMonetizationChecked(id: id, checked: checked);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> markAllMonetizationChecked() async {
    try {
      monetizationItems = await api.markAllMonetizationChecked();
      notifyListeners();
      onNotice?.call('success', 'すべての未読スパチャ・メンバーをお礼済みにしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteMonetizationEvent(String id) async {
    try {
      monetizationItems = await api.deleteMonetizationEvent(id: id);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearMonetizationEvents({required bool checkedOnly}) async {
    try {
      monetizationItems =
          await api.clearMonetizationEvents(checkedOnly: checkedOnly);
      notifyListeners();
      onNotice?.call(
        'info',
        checkedOnly ? 'お礼済みの履歴をクリアしました' : 'スパチャ・メンバー履歴をすべてクリアしました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> sendDemoMonetization(
    MonetizationEventKind kind,
    String userName,
    String amountDisplay,
    String message,
  ) async {
    try {
      monetizationItems = await api.sendDemoMonetization(
        kind: kind,
        userName: userName,
        amountDisplay: amountDisplay,
        message: message,
      );
      notifyListeners();
      onNotice?.call('success', 'テスト支援イベントを送信しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== リアルタイム投票 / アンケート ====================

  Future<void> savePollConfig(PollConfig config) async {
    try {
      pollState = await api.savePollConfig(config: config);
      notifyListeners();
      onNotice?.call('success', '投票設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> startPoll(PollConfig config) async {
    try {
      pollState = await api.startPoll(config: config);
      notifyListeners();
      onNotice?.call('success', 'ライブアンケートの受付を開始しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> stopPoll() async {
    try {
      pollState = await api.stopPoll();
      notifyListeners();
      onNotice?.call('info', '投票の受付を締め切りました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetPoll() async {
    try {
      pollState = await api.resetPoll();
      notifyListeners();
      onNotice?.call('info', '投票の集計をリセットしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearOverlayPoll() async {
    try {
      await api.clearOverlayPoll();
      onNotice?.call('info', 'OBS ライブアンケートの表示をクリアしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== コメントピックアップ（ピン留め表示） ====================

  Future<void> togglePickComment(CommentItem comment) async {
    if (pickedComment.comment?.id == comment.id) {
      await clearPickedComment();
      return;
    }
    try {
      pickedComment = await api.pickComment(comment: comment);
      notifyListeners();
      onNotice?.call('success', '${comment.userName} さんのコメントをスポットテロップに表示しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearPickedComment() async {
    try {
      pickedComment = await api.clearPickedComment();
      notifyListeners();
      onNotice?.call('info', 'スポットテロップ表示を解除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 参加型キュー（順番待ちリスト） ====================

  Future<void> saveQueueConfig(QueueConfig config) async {
    try {
      queueState = await api.saveQueueConfig(config: config);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueAddManual(String userName, String note) async {
    try {
      queueState = await api.queueAddManual(userName: userName, note: note);
      notifyListeners();
      onNotice?.call('success', '待機列へ手動追加しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueSetStatus(String userId, QueueEntryStatus status) async {
    try {
      queueState = await api.queueSetStatus(userId: userId, status: status);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queuePromoteNext() async {
    try {
      queueState = await api.queuePromoteNext();
      notifyListeners();
      onNotice?.call('info', '空き枠へ待機ユーザーを繰り上げました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueRotateActive() async {
    try {
      queueState = await api.queueRotateActive();
      notifyListeners();
      onNotice?.call('success', '現在の参加者を交代し、次の待機者を繰り上げました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueMoveEntry(String userId, int delta) async {
    try {
      queueState = await api.queueMoveEntry(userId: userId, delta: delta);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueRemoveEntry(String userId) async {
    try {
      queueState = await api.queueRemoveEntry(userId: userId);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> queueReset({required bool resetPlayCounts}) async {
    try {
      queueState = await api.queueReset(resetPlayCounts: resetPlayCounts);
      notifyListeners();
      onNotice?.call('warning', 'エントリーレーンをリセットしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 特定ワード反応メディアアニメーション ====================

  Future<String?> pickMediaFileDialog() async {
    try {
      return await api.pickMediaFileDialog();
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<String?> pickSoundFileDialog() async {
    try {
      return await api.pickSoundFileDialog();
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<ImportedMediaFile?> importMediaFile(String sourcePath) async {
    try {
      final res = await api.importMediaFile(sourcePath: sourcePath);
      onNotice?.call('success', 'メディアをアプリ内へ保存しました: ${res.originalFileName}');
      return res;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<void> saveMediaReactionRule(MediaReactionRule rule) async {
    try {
      mediaReactionRules = await api.saveMediaReactionRule(rule: rule);
      notifyListeners();
      onNotice?.call('success', 'リアクションステージの演出ルールを保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteMediaReactionRule(String id) async {
    try {
      mediaReactionRules = await api.deleteMediaReactionRule(id: id);
      notifyListeners();
      onNotice?.call('info', 'アニメーションルールを削除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> saveMediaReactionConfig(MediaReactionConfig config) async {
    try {
      mediaReactionConfig = await api.saveMediaReactionConfig(config: config);
      notifyListeners();
      onNotice?.call('success', 'アニメーション再生方式・音量設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> testMediaReactionRule(String id) async {
    try {
      await api.testMediaReactionRule(id: id);
      onNotice?.call('info', 'OBS オーバーレイへテスト再生を送信しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearOverlayMedia() async {
    try {
      await api.clearOverlayMedia();
      onNotice?.call('info', 'リアクションステージの演出をすべて停止しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  /// リアクションステージオーバーレイ (/overlay/media) の背景色のみを即時更新・全端末へ同期する
  Future<void> setMediaOverlayBgColor(String bgColor) async {
    final c = overlayConfig;
    final next = OverlayConfig(
      port: c.port,
      commentFontSize: c.commentFontSize,
      commentMaxItems: c.commentMaxItems,
      showIcons: c.showIcons,
      showTimestamps: c.showTimestamps,
      defaultBubbleBg: c.defaultBubbleBg,
      defaultTextColor: c.defaultTextColor,
      commentsTheme: c.commentsTheme,
      lotteryTheme: c.lotteryTheme,
      counterTheme: c.counterTheme,
      firstTimeTheme: c.firstTimeTheme,
      superchatTheme: c.superchatTheme,
      pollTheme: c.pollTheme,
      pickupTheme: c.pickupTheme,
      queueTheme: c.queueTheme,
      mediaTheme: c.mediaTheme,
      commentsBgColor: c.commentsBgColor,
      lotteryBgColor: c.lotteryBgColor,
      counterBgColor: c.counterBgColor,
      firstTimeBgColor: c.firstTimeBgColor,
      superchatBgColor: c.superchatBgColor,
      pollBgColor: c.pollBgColor,
      pickupBgColor: c.pickupBgColor,
      queueBgColor: c.queueBgColor,
      mediaBgColor: bgColor,
      commentsFontFamily: c.commentsFontFamily,
      lotteryFontFamily: c.lotteryFontFamily,
      counterFontFamily: c.counterFontFamily,
      firstTimeFontFamily: c.firstTimeFontFamily,
      superchatFontFamily: c.superchatFontFamily,
      pollFontFamily: c.pollFontFamily,
      pickupFontFamily: c.pickupFontFamily,
      queueFontFamily: c.queueFontFamily,
      mediaFontFamily: c.mediaFontFamily,
      commentsCustomCss: c.commentsCustomCss,
      lotteryCustomCss: c.lotteryCustomCss,
      counterCustomCss: c.counterCustomCss,
      firstTimeCustomCss: c.firstTimeCustomCss,
      superchatCustomCss: c.superchatCustomCss,
      pollCustomCss: c.pollCustomCss,
      pickupCustomCss: c.pickupCustomCss,
      queueCustomCss: c.queueCustomCss,
      mediaCustomCss: c.mediaCustomCss,
      visualStyles: c.visualStyles,
    );
    try {
      overlayConfig = await api.saveOverlayConfig(config: next);
      notifyListeners();
      onNotice?.call(
        'success',
        bgColor == 'transparent'
            ? 'リアクションステージの背景色を透過に戻しました'
            : 'リアクションステージの背景色を $bgColor に変更しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 初見カウンター ====================

  Future<void> saveFirstTimeConfig(FirstTimeCounterConfig config) async {
    try {
      firstTimeState = await api.saveFirstTimeConfig(config: config);
      notifyListeners();
      onNotice?.call('success', 'ウェルカムカウンター設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetFirstTimeSessionCount() async {
    try {
      firstTimeState = await api.resetFirstTimeSessionCount();
      notifyListeners();
      onNotice?.call('info', '配信の初見カウントを 0 にリセットしました（過去検知履歴は維持）');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetFirstTimeAllHistory() async {
    try {
      firstTimeState = await api.resetFirstTimeAllHistory();
      notifyListeners();
      onNotice?.call('warning', '全ユーザーの初見検知履歴をリセットしました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> adjustFirstTimeCount(int delta) async {
    try {
      firstTimeState = await api.adjustFirstTimeCount(delta: delta);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 外観テーマ（ライト / ダーク / システム） ====================

  Future<void> setThemeMode(AppThemeMode mode) async {
    try {
      themeMode = await api.saveThemeMode(mode: mode);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== OBS オーバーレイ ====================

  String get overlayBaseUrl => 'http://127.0.0.1:${overlayConfig.port}';

  Future<void> saveOverlayConfig(OverlayConfig config) async {
    try {
      final prevPort = overlayConfig.port;
      overlayConfig = await api.saveOverlayConfig(config: config);
      // ポート番号が変わった時だけ LAN/QR 情報を再生成（デザイン・CSS 保存時の ipconfig 起動を回避）
      if (overlayConfig.port != prevPort) {
        remoteAccessStatus = await api.getRemoteAccessStatus();
        _remoteEpoch++;
      }
      notifyListeners();
      onNotice?.call('success', 'オーバーレイスタジオ設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ローカルネットワーク接続・Web リモコン ====================

  Future<void> refreshRemoteAccessStatus() async {
    try {
      remoteAccessStatus = await api.getRemoteAccessStatus();
      _remoteEpoch++;
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> saveRemoteAccessConfig(
    RemoteAccessConfig config, {
    String? notice,
  }) async {
    try {
      remoteAccessStatus = await api.saveRemoteAccessConfig(config: config);
      _remoteEpoch++;
      overlayConfig = await api.getOverlayConfig();
      notifyListeners();
      onNotice?.call(
        'success',
        notice ??
            (config.allowLanAccess
                ? 'ローカルネットワークからの接続を許可しました（${remoteAccessStatus.boundAddress}）'
                : 'ローカルネットワークからの接続を無効にしました（同一 PC のみ許可）'),
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> regenerateRemotePin() async {
    try {
      remoteAccessStatus = await api.regenerateRemotePin();
      _remoteEpoch++;
      notifyListeners();
      onNotice?.call(
        'info',
        'アクセス PIN を再生成しました（新しい PIN: ${remoteAccessStatus.config.pinCode}）',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> refreshSystemFonts() async {
    try {
      systemFonts = await api.listSystemFonts();
      _fontsEpoch++;
      notifyListeners();
      onNotice?.call('info', 'システムフォント一覧を再読み込みしました（${systemFonts.length}種類）');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> openUrl(String url) async {
    try {
      await api.openExternalUrl(url: url);
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== YouTube OAuth ====================

  Future<void> saveOAuthClient(String clientId, String clientSecret) async {
    try {
      oauthStatus = await api.saveYoutubeOauthClient(
        clientId: clientId,
        clientSecret: clientSecret,
      );
      notifyListeners();
      onNotice?.call('success', 'OAuth クライアント設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> startOAuthLogin() async {
    isOAuthInProgress = true;
    notifyListeners();
    try {
      oauthStatus = await api.startYoutubeOauthLogin();
      onNotice?.call(
        'success',
        'YouTube アカウント（${oauthStatus.accountName ?? "連携済み"}）と連携しました！',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    } finally {
      isOAuthInProgress = false;
      notifyListeners();
    }
  }

  Future<void> disconnectOAuth() async {
    try {
      oauthStatus = await api.disconnectYoutubeOauth();
      notifyListeners();
      onNotice?.call('info', 'YouTube アカウント連携を解除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 自動アップデート ====================

  Future<UpdateInfo?> checkUpdates({bool silent = false}) async {
    isCheckingUpdate = true;
    notifyListeners();
    try {
      final info = await api.checkForUpdates();
      if (info.hasUpdate) {
        availableUpdate = info;
      } else if (!silent) {
        availableUpdate = null;
        onNotice?.call(
            'success', '現在のバージョン (v${info.currentVersion}) は最新です');
      }
      return info;
    } catch (e) {
      if (!silent) {
        onNotice?.call('error', e.toString());
      }
      return null;
    } finally {
      isCheckingUpdate = false;
      notifyListeners();
    }
  }

  Stream<UpdateDownloadProgress> downloadUpdate(UpdateInfo info) {
    return api.downloadUpdateInstaller(
      installerUrl: info.installerUrl!,
      installerName: info.installerName ?? 'Stosup-x64.msi',
      checksumUrl: info.checksumUrl,
    );
  }

  Future<void> launchUpdateInstaller(String path, {bool silent = true}) async {
    await api.launchUpdateInstaller(installerPath: path, silent: silent);
  }

  // ==================== 接続履歴・チャンネル自動検出 ====================

  Future<void> refreshConnectionHistory() async {
    try {
      connectionHistory = await api.listConnectionHistory();
      if (!_isDisposed) {
        _notifyTopics(const [StosupTopic.comments]);
      }
    } catch (_) {}
  }

  Future<ResolvedYouTubeLive?> resolveYouTubeLive(String input) async {
    try {
      return await api.resolveYoutubeLive(input: input);
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<void> toggleConnectionHistoryFavorite(String id, bool favorite) async {
    try {
      connectionHistory = await api.toggleConnectionHistoryFavorite(
        id: id,
        favorite: favorite,
      );
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteConnectionHistory(String id) async {
    try {
      connectionHistory = await api.deleteConnectionHistory(id: id);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> saveSavedChannel(SavedChannelItem item) async {
    try {
      savedChannels = await api.saveSavedChannel(item: item);
      notifyListeners();
      onNotice?.call('success', '自動検出用チャンネル「${item.label}」を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteSavedChannel(String id) async {
    try {
      savedChannels = await api.deleteSavedChannel(id: id);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== NG ワード防御 ====================

  Future<void> saveNgWordRule(NgWordRule rule) async {
    try {
      ngWordRules = await api.saveNgWordRule(rule: rule);
      notifyListeners();
      onNotice?.call('success', 'NG ワードルールを保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteNgWordRule(String id) async {
    try {
      ngWordRules = await api.deleteNgWordRule(id: id);
      notifyListeners();
      onNotice?.call('info', 'NG ワードルールを削除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== OBS WebSocket 連携 ====================

  Future<void> saveObsWsConfig(ObsWsConfig config) async {
    try {
      obsWsConfig = await api.saveObsWsConfig(config: config);
      notifyListeners();
      onNotice?.call('success', 'OBS WebSocket 連携設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> connectObsWs() async {
    try {
      obsWsStatus = await api.connectObsWs();
      notifyListeners();
      onNotice?.call('success', 'OBS WebSocket に接続しました (${obsWsStatus.obsVersion})');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> disconnectObsWs() async {
    try {
      obsWsStatus = await api.disconnectObsWs();
      notifyListeners();
      onNotice?.call('info', 'OBS WebSocket から切断しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> refreshObsScenes() async {
    try {
      await api.refreshObsScenes();
      onNotice?.call('info', 'OBS シーン・ソース一覧を更新しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> testObsRule(String ruleId) async {
    try {
      await api.testObsRule(ruleId: ruleId);
      onNotice?.call('success', 'OBS 自動連動ルールをテスト実行しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== グローバルホットキー / Stream Deck ====================

  Future<void> saveHotkeyBindings(List<HotkeyBinding> bindings) async {
    try {
      hotkeyBindings = await api.saveHotkeyBindings(bindings: bindings);
      notifyListeners();
      onNotice?.call('success', 'ホットキー・オートメーション設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> executeAutomationAction(
    AutomationActionKind action, {
    String targetId = '',
    String secondaryTarget = '',
  }) async {
    try {
      final msg = await api.executeAutomationAction(
        action: action,
        targetId: targetId,
        secondaryTarget: secondaryTarget,
      );
      onNotice?.call('success', msg);
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== 配信プロファイル切替 ====================

  Future<void> saveCurrentAsProfile({
    String? id,
    required String name,
    String description = '',
  }) async {
    try {
      streamProfiles = await api.saveCurrentAsProfile(
        id: id,
        name: name,
        description: description,
      );
      notifyListeners();
      onNotice?.call('success', '配信プロファイル「$name」を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> createTemplateProfile(String templateId, String name) async {
    try {
      streamProfiles = await api.createTemplateProfile(
        templateId: templateId,
        name: name,
      );
      notifyListeners();
      onNotice?.call('success', 'テンプレートプロファイル「$name」を作成しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> applyStreamProfile(String id) async {
    try {
      streamProfiles = await api.applyStreamProfile(id: id);
      await refreshAll();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> deleteStreamProfile(String id) async {
    try {
      streamProfiles = await api.deleteStreamProfile(id: id);
      notifyListeners();
      onNotice?.call('info', '配信プロファイルを削除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== バックアップ / 引っ越し (ZIP) ====================

  Future<BackupExportResult?> exportBackupWithDialog() async {
    try {
      final path = await api.pickBackupSavePath();
      if (path == null || path.isEmpty) return null;
      final res = await api.exportBackupZip(destPath: path);
      onNotice?.call(
        'success',
        'バックアップを書き出しました（素材 ${res.mediaFileCount} 件）: ${res.filePath}',
      );
      return res;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<BackupImportResult?> importBackupWithDialog() async {
    try {
      final path = await api.pickBackupOpenPath();
      if (path == null || path.isEmpty) return null;
      final res = await api.importBackupZip(zipPath: path);
      await refreshAll();
      onNotice?.call(
        'success',
        'バックアップから復元しました（素材 ${res.mediaFileCount} 件 / 退避ファイル保存済み）',
      );
      return res;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  // ==================== オンエアレポート ====================

  Future<void> refreshStreamSessions() async {
    try {
      streamSessions = await api.listStreamSessions(limit: 50);
      notifyListeners();
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<OnAirReportDto?> loadOnAirReport(String sessionId) async {
    try {
      currentReport = await api.getOnAirReport(sessionId: sessionId);
      notifyListeners();
      return currentReport;
    } catch (e) {
      onNotice?.call('error', e.toString());
      return null;
    }
  }

  Future<void> deleteStreamSession(String sessionId) async {
    try {
      streamSessions = await api.deleteStreamSession(sessionId: sessionId);
      if (currentReport?.session.sessionId == sessionId) {
        currentReport = null;
      }
      notifyListeners();
      onNotice?.call('info', '配信セッション履歴を削除しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> exportReportCsv(String sessionId) async {
    try {
      final saved = await api.exportReportCsvWithDialog(sessionId: sessionId);
      if (saved != null) {
        onNotice?.call('success', 'CSV レポートを書き出しました: $saved');
      }
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> exportReportJson(String sessionId) async {
    try {
      final saved = await api.exportReportJsonWithDialog(sessionId: sessionId);
      if (saved != null) {
        onNotice?.call('success', 'JSON レポートを書き出しました: $saved');
      }
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== オーバーレイテスト発火 ====================

  Future<void> sendOverlayTestEvent(String target) async {
    try {
      final msg = await api.sendOverlayTestEvent(target: target);
      onNotice?.call('info', msg);
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _eventSub?.cancel();
    _usersRefreshDebounce?.cancel();
    _commentNotifyTimer?.cancel();
    super.dispose();
  }
}
