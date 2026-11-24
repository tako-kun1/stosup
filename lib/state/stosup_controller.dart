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
    defaultBubbleBg: '#1E293BE6',
    defaultTextColor: '#F8FAFC',
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
      // バックグラウンドで GitHub 最新版チェック
      unawaited(checkUpdates(silent: true));
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

  // ==================== OBS オーバーレイ ====================

  String get overlayBaseUrl => 'http://127.0.0.1:${overlayConfig.port}';

  Future<void> saveOverlayConfig(OverlayConfig config) async {
    try {
      overlayConfig = await api.saveOverlayConfig(config: config);
      notifyListeners();
      onNotice?.call('success', 'OBS オーバーレイ設定を保存しました');
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
      installerName: info.installerName ?? 'Stosup-Setup.exe',
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
