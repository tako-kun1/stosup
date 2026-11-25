import 'dart:async';

import 'package:flutter/foundation.dart';

import '../src/rust/api.dart' as api;
import '../src/rust/frb_generated.dart';
import '../src/rust/models.dart';

/// Stosup アプリ全体の状態を管理するコントローラー
class StosupController extends ChangeNotifier {
  bool isInitialized = false;
  String? initError;
  String appVersion = '0.1.0';

  ConnectionStateDto connectionState = const ConnectionStateDto(
    status: ConnectionStatus.disconnected,
    platform: PlatformKind.youTube,
    mode: ConnectionMode.youTubeInnerTube,
    streamTitle: '',
    streamId: '',
    sessionId: '',
    canPostChat: false,
  );

  List<CommentItem> comments = [];
  List<HighlightRule> highlightRules = [];
  List<WordCounterConfig> wordCounters = [];
  List<UserRecord> users = [];
  List<String> userLotteryWonIds = [];
  LotteryResult? lastUserLotteryResult;
  List<LotteryResult> lotteryHistory = [];

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
    commentsTheme: 'default',
    lotteryTheme: 'default',
    counterTheme: 'default',
    firstTimeTheme: 'default',
    superchatTheme: 'default',
    pollTheme: 'default',
    pickupTheme: 'default',
    queueTheme: 'default',
    mediaTheme: 'default',
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
      allowLanAccess: false,
      remoteControlEnabled: true,
      requirePin: true,
      pinCode: '123456',
      embedPinInQr: true,
    ),
    serverRunning: false,
    port: 18765,
    boundAddress: '127.0.0.1:18765',
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

  Future<void> initialize({required bool afterUpdate}) async {
    try {
      await RustLib.init();
      await api.initApp(afterUpdate: afterUpdate);
      appVersion = api.getAppVersion();

      await refreshAll();

      _eventSub?.cancel();
      _eventSub = api.subscribeEvents().listen(_handleEvent);

      isInitialized = true;
      notifyListeners();

      // アップデート後初回起動時のリリースノート判定
      unawaited(_checkStartupReleaseNotes());
      // 自動アップデート確認機能が ON の場合のみバックグラウンドで最新版チェック
      if (featureFlags.autoUpdateCheckEnabled) {
        unawaited(checkUpdates(silent: true));
      }
    } catch (e) {
      initError = e.toString();
      notifyListeners();
    }
  }

  Future<void> refreshAll() async {
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
      api.listSystemFonts(),
      api.listMonetizationEvents(limit: 150),
      api.getPollState(),
      api.getPickedComment(),
      api.getQueueState(),
      api.listMediaReactionRules(),
      api.getMediaReactionConfig(),
      api.getRemoteAccessStatus(),
    ]);

    connectionState = results[0] as ConnectionStateDto;
    comments = List<CommentItem>.from(results[1] as List<CommentItem>);
    highlightRules = results[2] as List<HighlightRule>;
    wordCounters = results[3] as List<WordCounterConfig>;
    users = results[4] as List<UserRecord>;
    commentLottery = results[5] as CommentLotteryState;
    userLotteryWonIds = results[6] as List<String>;
    lotteryHistory = results[7] as List<LotteryResult>;
    overlayConfig = results[8] as OverlayConfig;
    oauthStatus = results[9] as YouTubeOAuthStatus;
    featureFlags = results[10] as FeatureFlags;
    themeMode = results[11] as AppThemeMode;
    firstTimeState = results[12] as FirstTimeCounterState;
    systemFonts = results[13] as List<String>;
    monetizationItems = results[14] as List<MonetizationItem>;
    pollState = results[15] as PollState;
    pickedComment = results[16] as PickedCommentState;
    queueState = results[17] as QueueState;
    mediaReactionRules = results[18] as List<MediaReactionRule>;
    mediaReactionConfig = results[19] as MediaReactionConfig;
    remoteAccessStatus = results[20] as RemoteAccessStatus;
    notifyListeners();
  }

  void _handleEvent(AppEvent ev) {
    switch (ev.kind) {
      case AppEventKind.commentReceived:
        if (ev.comment != null) {
          comments = [...comments, ev.comment!];
          if (comments.length > 400) {
            comments = comments.sublist(comments.length - 400);
          }
          notifyListeners();
        }
      case AppEventKind.commentsReplaced:
        if (ev.comments != null) {
          comments = List<CommentItem>.from(ev.comments!);
          notifyListeners();
        }
      case AppEventKind.connectionChanged:
        if (ev.connection != null) {
          connectionState = ev.connection!;
          isConnecting = connectionState.status == ConnectionStatus.connecting;
          notifyListeners();
        }
      case AppEventKind.countersUpdated:
        if (ev.counters != null) {
          wordCounters = ev.counters!;
          notifyListeners();
        }
      case AppEventKind.firstTimeUpdated:
        if (ev.firstTimeState != null) {
          firstTimeState = ev.firstTimeState!;
          notifyListeners();
        }
      case AppEventKind.usersUpdated:
        _scheduleUsersRefresh();
      case AppEventKind.commentLotteryUpdated:
        if (ev.commentLottery != null) {
          commentLottery = ev.commentLottery!;
          notifyListeners();
        }
      case AppEventKind.lotteryDrawn:
        if (ev.lotteryResult != null) {
          final res = ev.lotteryResult!;
          if (res.kind == 'user') {
            lastUserLotteryResult = res;
          }
          lotteryHistory = [res, ...lotteryHistory.take(29)];
          notifyListeners();
        }
      case AppEventKind.monetizationUpdated:
        if (ev.monetizationItems != null) {
          monetizationItems = ev.monetizationItems!;
          notifyListeners();
        }
      case AppEventKind.pollUpdated:
        if (ev.pollState != null) {
          pollState = ev.pollState!;
          notifyListeners();
        }
      case AppEventKind.pickedCommentUpdated:
        if (ev.pickedComment != null) {
          pickedComment = ev.pickedComment!;
          notifyListeners();
        }
      case AppEventKind.queueUpdated:
        if (ev.queueState != null) {
          queueState = ev.queueState!;
          notifyListeners();
        }
      case AppEventKind.mediaTriggered:
        if (ev.mediaTrigger != null) {
          lastMediaTrigger = ev.mediaTrigger!;
          notifyListeners();
        }
      case AppEventKind.mediaRulesUpdated:
        if (ev.mediaRules != null) {
          mediaReactionRules = List<MediaReactionRule>.from(ev.mediaRules!);
          notifyListeners();
        }
      case AppEventKind.mediaConfigUpdated:
        if (ev.mediaConfig != null) {
          mediaReactionConfig = ev.mediaConfig!;
          notifyListeners();
        }
      case AppEventKind.highlightRulesUpdated:
        if (ev.highlightRules != null) {
          highlightRules = List<HighlightRule>.from(ev.highlightRules!);
          notifyListeners();
        }
      case AppEventKind.overlayConfigUpdated:
        if (ev.overlayConfig != null) {
          overlayConfig = ev.overlayConfig!;
          notifyListeners();
        }
      case AppEventKind.userLotteryWinnersUpdated:
        if (ev.userLotteryWonIds != null) {
          userLotteryWonIds = List<String>.from(ev.userLotteryWonIds!);
          notifyListeners();
        }
      case AppEventKind.featureFlagsUpdated:
        if (ev.featureFlags != null) {
          featureFlags = ev.featureFlags!;
          notifyListeners();
        }
      case AppEventKind.systemNotice:
        if (ev.noticeMessage != null) {
          onNotice?.call(ev.noticeLevel ?? 'info', ev.noticeMessage!);
        }
    }
  }

  void _scheduleUsersRefresh() {
    _usersRefreshDebounce?.cancel();
    _usersRefreshDebounce = Timer(const Duration(milliseconds: 250), () async {
      try {
        users = await api.listUsers();
        notifyListeners();
      } catch (_) {}
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
    await api.clearRecentComments();
    comments = [];
    notifyListeners();
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
      onNotice?.call('success', 'ワード反応ルールを保存しました');
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
      onNotice?.call('success', 'ワードカウンターを保存しました');
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
      userLotteryWonIds = await api.getUserLotteryWonIds();
      firstTimeState = await api.getFirstTimeState();
      notifyListeners();
      onNotice?.call(
        'info',
        displayName != null
            ? '「$displayName」をユーザーリストから削除しました'
            : 'ユーザーをリストから削除しました',
      );
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> resetUsers({required bool keepBanned}) async {
    try {
      users = await api.resetUsers(keepBanned: keepBanned);
      userLotteryWonIds = await api.getUserLotteryWonIds();
      firstTimeState = await api.getFirstTimeState();
      notifyListeners();
      onNotice?.call(
        'warning',
        keepBanned
            ? 'ユーザーリストを一括リセットしました（BAN済みユーザーは維持）'
            : 'BAN済みユーザーを含め、ユーザーリストをすべてリセットしました',
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
    notifyListeners();
    onNotice?.call('info', 'ユーザー抽選の既当選リストをリセットしました');
  }

  Future<void> clearOverlayLottery() async {
    await api.clearOverlayLottery();
    onNotice?.call('info', 'OBS 抽選オーバーレイの表示をクリアしました');
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
      featureFlags = await api.saveFeatureFlags(flags: next);
      // OBSサーバー再起動時にポートが変わる可能性があるため同期
      overlayConfig = await api.getOverlayConfig();
      remoteAccessStatus = await api.getRemoteAccessStatus();
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
      onNotice?.call('success', 'リアルタイム投票の受付を開始しました');
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
      onNotice?.call('info', 'OBS 投票オーバーレイの表示をクリアしました');
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
      onNotice?.call('success', '${comment.userName} さんのコメントをピックアップ表示しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  Future<void> clearPickedComment() async {
    try {
      pickedComment = await api.clearPickedComment();
      notifyListeners();
      onNotice?.call('info', 'ピックアップ表示を解除しました');
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
      onNotice?.call('warning', '参加型キューをリセットしました');
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
      onNotice?.call('success', 'ワード反応アニメーションルールを保存しました');
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
      onNotice?.call('info', '再生中のメディア演出をすべて停止しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  /// WordParty オーバーレイ (/overlay/media) の背景色のみを即時更新・全端末へ同期する
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
    );
    try {
      overlayConfig = await api.saveOverlayConfig(config: next);
      notifyListeners();
      onNotice?.call(
        'success',
        bgColor == 'transparent'
            ? 'WordParty の背景色を透過に戻しました'
            : 'WordParty の背景色を $bgColor に変更しました',
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
      onNotice?.call('success', '初見カウンター設定を保存しました');
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
      overlayConfig = await api.saveOverlayConfig(config: config);
      remoteAccessStatus = await api.getRemoteAccessStatus();
      notifyListeners();
      onNotice?.call('success', 'OBS オーバーレイ設定を保存しました');
    } catch (e) {
      onNotice?.call('error', e.toString());
    }
  }

  // ==================== ローカルネットワーク接続・Web リモコン ====================

  Future<void> refreshRemoteAccessStatus() async {
    try {
      remoteAccessStatus = await api.getRemoteAccessStatus();
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

  @override
  void dispose() {
    _eventSub?.cancel();
    _usersRefreshDebounce?.cancel();
    super.dispose();
  }
}
