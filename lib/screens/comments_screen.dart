import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/rich_comment_text.dart';
import '../widgets/user_avatar.dart';

enum _CommentFilter {
  all,
  firstTime,
  member,
  monetization,
  highlighted,
  question,
  ngFlagged,
}

/// コメント表示 ＆ 配信接続画面（ライト/ダーク両対応）
class CommentsScreen extends StatefulWidget {
  const CommentsScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<CommentsScreen> createState() => _CommentsScreenState();
}

class _CommentsScreenState extends State<CommentsScreen> {
  final _urlCtrl = TextEditingController();
  final _searchCtrl = TextEditingController();
  final _demoUserCtrl = TextEditingController(text: 'テスト視聴者A');
  final _demoMsgCtrl = TextEditingController();
  final _postChatCtrl = TextEditingController();
  final _scrollCtrl = ScrollController();

  ConnectionMode _selectedMode = ConnectionMode.youTubeInnerTube;
  _CommentFilter _filter = _CommentFilter.all;
  String _searchQuery = '';
  bool _autoScroll = true;
  int _lastCommentCount = 0;
  String? _lastCommentId;

  // フィルタ／検索適用時のみメモ化し、同一コメント内容での再ビルド時に全件再走査を回避
  StosupController? _cachedFilterCtrl;
  int _cachedCommentsRev = -1;
  int _cachedSourceLen = -1;
  String? _cachedSourceLastId;
  _CommentFilter? _cachedFilterMode;
  String _cachedFilterQuery = '';
  List<CommentItem> _cachedFilteredComments = const [];

  @override
  void dispose() {
    _urlCtrl.dispose();
    _searchCtrl.dispose();
    _demoUserCtrl.dispose();
    _demoMsgCtrl.dispose();
    _postChatCtrl.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  void _maybeScrollToBottom(List<CommentItem> comments) {
    final count = comments.length;
    final lastId = comments.isNotEmpty ? comments.last.id : null;
    if (_autoScroll && (count != _lastCommentCount || lastId != _lastCommentId)) {
      _lastCommentCount = count;
      _lastCommentId = lastId;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollCtrl.hasClients) {
          _scrollCtrl.animateTo(
            _scrollCtrl.position.maxScrollExtent,
            duration: const Duration(milliseconds: 180),
            curve: Curves.easeOut,
          );
        }
      });
    }
  }

  String _formatTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    final s = dt.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  Future<void> _confirmBan(CommentItem c) async {
    final p = context.palette;
    final reasonCtrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('${c.userName} をアプリ内BANしますか？'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'BANされたユーザーはドローボックス・コールカウンター等に一切参加できなくなります。',
              style: TextStyle(fontSize: 13, color: p.textSecondary),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: reasonCtrl,
              decoration: const InputDecoration(
                labelText: 'BAN 理由メモ（任意）',
                hintText: '例: 荒らし行為',
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: p.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('BANする'),
          ),
        ],
      ),
    );
    final reason = reasonCtrl.text.trim();
    WidgetsBinding.instance.addPostFrameCallback((_) => reasonCtrl.dispose());
    if (ok == true && mounted) {
      await widget.controller.setUserBan(
        c.userId,
        c.platform,
        true,
        reason,
      );
    }
  }

  Future<void> _editUserMemo(CommentItem c) async {
    final memoCtrl = TextEditingController(text: c.userMemo);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('${c.userName} の名前メモ'),
        content: TextField(
          controller: memoCtrl,
          maxLines: 2,
          decoration: const InputDecoration(
            labelText: 'コメント欄上に表示するメモ',
            hintText: '例: 常連さん / イラスト担当 / 前回当選者',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    final memo = memoCtrl.text.trim();
    WidgetsBinding.instance.addPostFrameCallback((_) => memoCtrl.dispose());
    if (ok == true && mounted) {
      await widget.controller.setUserMemo(c.userId, c.platform, memo);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final conn = ctrl.connectionState;
    final connected = conn.status == ConnectionStatus.connected ||
        conn.status == ConnectionStatus.reconnecting;
    final filteredComments = _applyFilter(ctrl.comments);
    _maybeScrollToBottom(filteredComments);

    return Column(
      children: [
        // 配信接続バー（URL/動画ID/チャンネル自動検出・接続履歴・お気に入りチャンネル対応）
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: p.sidebarBg,
            border: Border(bottom: BorderSide(color: p.border)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  SizedBox(
                    width: 340,
                    child: TextField(
                      controller: _urlCtrl,
                      decoration: InputDecoration(
                        prefixIcon:
                            Icon(Icons.live_tv, color: p.danger, size: 19),
                        labelText: 'YouTube 配信URL / 動画ID / @ハンドル',
                        hintText: '例: @channel や youtube.com/watch?v=...',
                      ),
                      onSubmitted: (_) {
                        if (!ctrl.isConnecting &&
                            _urlCtrl.text.trim().isNotEmpty) {
                          ctrl.connectYouTube(
                              _urlCtrl.text.trim(), _selectedMode);
                        }
                      },
                    ),
                  ),
                  DropdownButtonHideUnderline(
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      decoration: BoxDecoration(
                        color: p.inputBg,
                        borderRadius: BorderRadius.circular(6),
                        border: Border.all(color: p.border),
                      ),
                      child: DropdownButton<ConnectionMode>(
                        value: _selectedMode,
                        dropdownColor: p.cardBg,
                        style: TextStyle(color: p.textPrimary, fontSize: 13),
                        items: const [
                          DropdownMenuItem(
                            value: ConnectionMode.youTubeInnerTube,
                            child: Text('InnerTube（クォータ消費なし）'),
                          ),
                          DropdownMenuItem(
                            value: ConnectionMode.youTubeDataApi,
                            child: Text('YouTube Data API v3（公式）'),
                          ),
                        ],
                        onChanged: (v) {
                          if (v != null) setState(() => _selectedMode = v);
                        },
                      ),
                    ),
                  ),
                  FilledButton.icon(
                    onPressed: ctrl.isConnecting
                        ? null
                        : () => ctrl.connectYouTube(
                              _urlCtrl.text.trim(),
                              _selectedMode,
                            ),
                    icon: ctrl.isConnecting
                        ? const SizedBox(
                            width: 15,
                            height: 15,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.play_arrow, size: 18),
                    label: const Text('接続'),
                  ),
                  OutlinedButton.icon(
                    onPressed: () => _openConnectionHistoryDialog(ctrl),
                    icon: const Icon(Icons.history, size: 16),
                    label: Text(
                      ctrl.connectionHistory.isEmpty
                          ? '履歴・自動検出'
                          : '履歴 (${ctrl.connectionHistory.length})',
                    ),
                  ),
                  OutlinedButton.icon(
                    onPressed: ctrl.isConnecting
                        ? null
                        : () => ctrl.connectDemo(autoGenerate: true),
                    icon: const Icon(Icons.science_outlined, size: 17),
                    label: const Text('デモ配信'),
                  ),
                  if (connected)
                    OutlinedButton.icon(
                      style: OutlinedButton.styleFrom(
                        foregroundColor: p.danger,
                        side: BorderSide(color: p.danger),
                      ),
                      onPressed: ctrl.disconnect,
                      icon: const Icon(Icons.stop, size: 17),
                      label: const Text('切断'),
                    ),
                ],
              ),
              if (ctrl.savedChannels.isNotEmpty) ...[
                const SizedBox(height: 6),
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    Text(
                      '自動検出:',
                      style: TextStyle(
                        fontSize: 11.5,
                        color: p.textSecondary,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    ...ctrl.savedChannels.map(
                      (ch) => ActionChip(
                        visualDensity: VisualDensity.compact,
                        avatar: Icon(Icons.radar, size: 14, color: p.accent),
                        label: Text(ch.label, style: const TextStyle(fontSize: 11.5)),
                        onPressed: ctrl.isConnecting
                            ? null
                            : () {
                                _urlCtrl.text = ch.channelInput;
                                ctrl.connectYouTube(
                                    ch.channelInput, _selectedMode);
                              },
                      ),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: 8),
              Row(
                children: [
                  _StatusChip(state: conn),
                  const SizedBox(width: 10),
                  if (conn.streamTitle.isNotEmpty)
                    Expanded(
                      child: Text(
                        conn.streamTitle,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                            fontWeight: FontWeight.w600, color: p.textPrimary),
                      ),
                    )
                  else
                    Expanded(
                      child: Text(
                        '未接続です。YouTube URL を入力するか「デモ配信」で動作を確認できます。',
                        style: TextStyle(color: p.textSecondary, fontSize: 12.5),
                      ),
                    ),
                  if (connected)
                    Chip(
                      visualDensity: VisualDensity.compact,
                      avatar: Icon(
                        conn.canPostChat
                            ? Icons.check_circle_outline
                            : Icons.info_outline,
                        size: 15,
                        color: conn.canPostChat ? p.success : p.warning,
                      ),
                      label: Text(
                        conn.canPostChat
                            ? 'チャット投稿: 可能'
                            : 'チャット投稿: OAuth未連携',
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),

        // メイン領域
        Expanded(
          child: Row(
            children: [
              // コメント一覧
              Expanded(
                flex: 3,
                child: Column(
                  children: [
                    Padding(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 14, vertical: 8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Icon(Icons.chat_bubble_outline,
                                  size: 17, color: p.textSecondary),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  (_searchQuery.isEmpty &&
                                          _filter == _CommentFilter.all)
                                      ? 'コメント一覧 (${ctrl.comments.length}件)'
                                      : 'コメント一覧 (表示 ${filteredComments.length} / ${ctrl.comments.length}件)',
                                  overflow: TextOverflow.ellipsis,
                                  style: const TextStyle(
                                      fontWeight: FontWeight.bold,
                                      fontSize: 14),
                                ),
                              ),
                              OutlinedButton.icon(
                                style: OutlinedButton.styleFrom(
                                  visualDensity: VisualDensity.compact,
                                ),
                                onPressed: () => _openNgWordDialog(ctrl),
                                icon: Icon(
                                  Icons.shield_outlined,
                                  size: 15,
                                  color: ctrl.featureFlags.ngWordFilterEnabled
                                      ? p.accent
                                      : p.textMuted,
                                ),
                                label: Text(
                                  'NG防御 (${ctrl.ngWordRules.where((r) => r.enabled).length})',
                                  style: const TextStyle(fontSize: 12),
                                ),
                              ),
                              const SizedBox(width: 8),
                              Text('自動スクロール',
                                  style: TextStyle(
                                      fontSize: 12, color: p.textSecondary)),
                              Switch(
                                value: _autoScroll,
                                onChanged: (v) =>
                                    setState(() => _autoScroll = v),
                              ),
                              const SizedBox(width: 4),
                              TextButton.icon(
                                onPressed: ctrl.comments.isEmpty
                                    ? null
                                    : ctrl.clearComments,
                                icon: const Icon(Icons.delete_sweep, size: 17),
                                label: const Text('クリア'),
                              ),
                            ],
                          ),
                          const SizedBox(height: 6),
                          Wrap(
                            spacing: 8,
                            runSpacing: 6,
                            children: [
                              FeatureToggleSwitch(
                                label: 'トークモニター',
                                enabled: ctrl.featureFlags.commentDisplayEnabled,
                                onChanged: (v) => ctrl.setFeatureEnabled(
                                  commentDisplay: v,
                                  featureLabel: 'トークモニター',
                                ),
                              ),
                              FeatureToggleSwitch(
                                label: '名前メモ',
                                enabled:
                                    ctrl.featureFlags.userMemoDisplayEnabled,
                                onChanged: (v) => ctrl.setFeatureEnabled(
                                  userMemoDisplay: v,
                                  featureLabel: '名前メモ表示',
                                ),
                              ),
                              FeatureToggleSwitch(
                                label: 'スポットテロップ',
                                enabled: ctrl.featureFlags.pickupEnabled,
                                onChanged: (v) => ctrl.setFeatureEnabled(
                                  pickup: v,
                                  featureLabel: 'スポットテロップ',
                                ),
                              ),
                              FeatureToggleSwitch(
                                label: 'NGワード防御',
                                enabled: ctrl.featureFlags.ngWordFilterEnabled,
                                onChanged: (v) => ctrl.setFeatureEnabled(
                                  ngWordFilter: v,
                                  featureLabel: 'NGワード防御',
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 8),
                          // 検索 & フィルタバー
                          Row(
                            children: [
                              SizedBox(
                                width: 220,
                                child: TextField(
                                  controller: _searchCtrl,
                                  decoration: InputDecoration(
                                    isDense: true,
                                    prefixIcon: const Icon(Icons.search, size: 16),
                                    hintText: '本文・名前・メモを検索',
                                    suffixIcon: _searchQuery.isEmpty
                                        ? null
                                        : IconButton(
                                            icon: const Icon(Icons.close, size: 15),
                                            onPressed: () {
                                              _searchCtrl.clear();
                                              setState(() => _searchQuery = '');
                                            },
                                          ),
                                  ),
                                  onChanged: (v) => setState(
                                      () => _searchQuery = v.trim().toLowerCase()),
                                ),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: SingleChildScrollView(
                                  scrollDirection: Axis.horizontal,
                                  child: Row(
                                    children: [
                                      _buildFilterChip(_CommentFilter.all, 'すべて'),
                                      _buildFilterChip(_CommentFilter.firstTime, '初見'),
                                      _buildFilterChip(_CommentFilter.member, 'メンバー'),
                                      _buildFilterChip(_CommentFilter.monetization, 'エール'),
                                      _buildFilterChip(_CommentFilter.highlighted, 'ハイライト'),
                                      _buildFilterChip(_CommentFilter.question, '質問(?)'),
                                      _buildFilterChip(_CommentFilter.ngFlagged, 'NG該当'),
                                    ],
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                    const Divider(height: 1),
                    if (ctrl.pickedComment.comment != null)
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.symmetric(
                            horizontal: 14, vertical: 8),
                        decoration: BoxDecoration(
                          color: p.accent.withValues(alpha: 0.12),
                          border: Border(bottom: BorderSide(color: p.border)),
                        ),
                        child: Row(
                          children: [
                            Icon(Icons.push_pin, size: 16, color: p.accent),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'スポットテロップ表示中: ${ctrl.pickedComment.comment!.userName}「${ctrl.pickedComment.comment!.message}」',
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontSize: 12.5,
                                  fontWeight: FontWeight.w600,
                                  color: p.textPrimary,
                                ),
                              ),
                            ),
                            const SizedBox(width: 8),
                            OutlinedButton.icon(
                              style: OutlinedButton.styleFrom(
                                visualDensity: VisualDensity.compact,
                              ),
                              onPressed: ctrl.clearPickedComment,
                              icon: const Icon(Icons.close, size: 14),
                              label: const Text('解除'),
                            ),
                          ],
                        ),
                      ),
                    if (!ctrl.featureFlags.commentDisplayEnabled)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(14, 12, 14, 0),
                        child: FeatureDisabledBanner(
                          featureName: 'トークモニター',
                          description:
                              '新規コメントのアプリ内表示および OBS トークモニター配信を一時停止しています。',
                          onEnable: () => ctrl.setFeatureEnabled(
                            commentDisplay: true,
                            featureLabel: 'トークモニター',
                          ),
                        ),
                      ),
                    Expanded(
                      child: filteredComments.isEmpty
                          ? Center(
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Icon(Icons.forum_outlined,
                                      size: 44, color: p.textMuted),
                                  const SizedBox(height: 10),
                                  Text(
                                    ctrl.comments.isEmpty
                                        ? 'コメントがまだありません'
                                        : '検索・フィルタ条件に一致するコメントがありません',
                                    style: TextStyle(
                                        fontSize: 15, color: p.textSecondary),
                                  ),
                                  const SizedBox(height: 4),
                                  Text(
                                    ctrl.comments.isEmpty
                                        ? 'YouTube 配信 URL や @ハンドルを入力するか「デモ配信」ボタンを押すとコメントが表示されます。'
                                        : 'キーワードやフィルタを「すべて」に戻すと全コメントが表示されます。',
                                    style: TextStyle(
                                        fontSize: 12.5, color: p.textMuted),
                                  ),
                                ],
                              ),
                            )
                          : ListView.builder(
                              controller: _scrollCtrl,
                              padding: const EdgeInsets.all(12),
                              itemCount: filteredComments.length,
                              itemBuilder: (context, idx) {
                                final c = filteredComments[idx];
                                final isPicked =
                                    ctrl.pickedComment.comment?.id == c.id;
                                return RepaintBoundary(
                                  key: ValueKey(c.id),
                                  child: _CommentBubbleCard(
                                    comment: c,
                                    showMemo: ctrl
                                        .featureFlags.userMemoDisplayEnabled,
                                    pickupEnabled:
                                        ctrl.featureFlags.pickupEnabled,
                                    isPicked: isPicked,
                                    timeLabel: _formatTime(c.timestampMs),
                                    onPickToggle: () =>
                                        ctrl.togglePickComment(c),
                                    onMemoTap: () => _editUserMemo(c),
                                    onBanTap: () => _confirmBan(c),
                                    onUnbanTap: () => ctrl.setUserBan(
                                        c.userId, c.platform, false, ''),
                                  ),
                                );
                              },
                            ),
                    ),
                  ],
                ),
              ),

              const VerticalDivider(width: 1),

              // 右サイドパネル
              SizedBox(
                width: 310,
                child: ListView(
                  padding: const EdgeInsets.all(12),
                  children: [
                    // 初見カウンター速報
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 12, vertical: 10),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      const Text('本日の初見さん',
                                          style: TextStyle(
                                              fontWeight: FontWeight.bold,
                                              fontSize: 13)),
                                      Text(
                                        ctrl.featureFlags
                                                .firstTimeCounterEnabled
                                            ? '累計検知 ${ctrl.firstTimeState.totalDetectedAllTime}人'
                                            : 'ウェルカムカウンター OFF',
                                        style: TextStyle(
                                            fontSize: 11,
                                            color: p.textSecondary),
                                      ),
                                    ],
                                  ),
                                ),
                                Text(
                                  ctrl.firstTimeState.count.toString(),
                                  style: TextStyle(
                                    fontSize: 26,
                                    fontWeight: FontWeight.w800,
                                    color: AppTheme.parseCssHex(
                                      ctrl.firstTimeState.config.accentColor,
                                      fallback: p.success,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                            if (ctrl.firstTimeState.config
                                    .showFirstTimerProfile &&
                                ctrl.firstTimeState.recentUsers.isNotEmpty) ...[
                              const Divider(height: 14),
                              ...ctrl.firstTimeState.recentUsers.take(3).map(
                                    (u) => Padding(
                                      padding: const EdgeInsets.only(bottom: 4),
                                      child: Row(
                                        children: [
                                          UserAvatar(
                                            name: u.userName,
                                            iconUrl: u.userIconUrl,
                                            radius: 10,
                                          ),
                                          const SizedBox(width: 6),
                                          Expanded(
                                            child: Text(
                                              u.userName,
                                              overflow: TextOverflow.ellipsis,
                                              style:
                                                  const TextStyle(fontSize: 12),
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                  ),
                            ],
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        const Text(
                          '稼働中コールカウンター',
                          style: TextStyle(
                              fontWeight: FontWeight.bold, fontSize: 13.5),
                        ),
                        if (!ctrl.featureFlags.wordCounterEnabled) ...[
                          const SizedBox(width: 8),
                          Text(
                            '(機能OFF)',
                            style: TextStyle(fontSize: 11.5, color: p.warning),
                          ),
                        ],
                      ],
                    ),
                    const SizedBox(height: 8),
                    if (ctrl.wordCounters.where((e) => e.enabled).isEmpty)
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(12),
                          child: Text('有効なカウンターがありません',
                              style: TextStyle(color: p.textSecondary)),
                        ),
                      )
                    else
                      ...ctrl.wordCounters.where((e) => e.enabled).map((wc) {
                        final accent = AppTheme.parseCssHex(wc.accentColor,
                            fallback: p.accent);
                        return Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: Card(
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 12, vertical: 9),
                              child: Row(
                                children: [
                                  Expanded(
                                    child: Column(
                                      crossAxisAlignment:
                                          CrossAxisAlignment.start,
                                      children: [
                                        Text(wc.name,
                                            style: const TextStyle(
                                                fontWeight: FontWeight.w600,
                                                fontSize: 13)),
                                        Text(
                                          wc.maxPerAccount != null
                                              ? '1人${wc.maxPerAccount}回まで'
                                              : '上限なし',
                                          style: TextStyle(
                                              fontSize: 11,
                                              color: p.textSecondary),
                                        ),
                                      ],
                                    ),
                                  ),
                                  Text(
                                    wc.totalCount.toString(),
                                    style: TextStyle(
                                      fontSize: 24,
                                      fontWeight: FontWeight.w800,
                                      color: accent,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        );
                      }),
                    const SizedBox(height: 12),

                    // 配信チャットへ直接投稿
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('配信チャットへ投稿',
                                style: TextStyle(
                                    fontWeight: FontWeight.bold,
                                    fontSize: 13.5)),
                            const SizedBox(height: 8),
                            TextField(
                              controller: _postChatCtrl,
                              maxLines: 2,
                              decoration: const InputDecoration(
                                hintText: '配信チャットに送るメッセージ...',
                              ),
                            ),
                            const SizedBox(height: 8),
                            SizedBox(
                              width: double.infinity,
                              child: FilledButton.icon(
                                onPressed: connected && conn.canPostChat
                                    ? () {
                                        final t = _postChatCtrl.text.trim();
                                        if (t.isNotEmpty) {
                                          ctrl.postChat(t);
                                          _postChatCtrl.clear();
                                        }
                                      }
                                    : null,
                                icon: const Icon(Icons.send, size: 15),
                                label: const Text('チャット送信'),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 10),

                    // テストコメント手動送信
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('テストコメント手動送信',
                                style: TextStyle(
                                    fontWeight: FontWeight.bold,
                                    fontSize: 13.5)),
                            const SizedBox(height: 4),
                            Text(
                              '接続中に任意のコメントを流して色変化・カウント・抽選をテストできます。',
                              style: TextStyle(
                                  fontSize: 11.5, color: p.textSecondary),
                            ),
                            const SizedBox(height: 8),
                            TextField(
                              controller: _demoUserCtrl,
                              decoration:
                                  const InputDecoration(labelText: '視聴者名'),
                            ),
                            const SizedBox(height: 8),
                            TextField(
                              controller: _demoMsgCtrl,
                              decoration: const InputDecoration(
                                labelText: 'コメント本文',
                                hintText: '例: 初見です！参加します！草',
                              ),
                              onSubmitted: (_) => _sendTestComment(ctrl),
                            ),
                            const SizedBox(height: 8),
                            SizedBox(
                              width: double.infinity,
                              child: OutlinedButton.icon(
                                onPressed: connected
                                    ? () => _sendTestComment(ctrl)
                                    : null,
                                icon: const Icon(Icons.add_comment, size: 15),
                                label: const Text('テストコメント送信'),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  List<CommentItem> _applyFilter(List<CommentItem> source) {
    // デフォルト（フィルタなし・検索語なし）は毎ビルドの 400 件コピーを回避してゼロコピー返却
    if (_filter == _CommentFilter.all && _searchQuery.isEmpty) {
      return source;
    }
    final rev = widget.controller.topicRevision(StosupTopic.comments);
    final lastId = source.isNotEmpty ? source.last.id : null;
    if (identical(_cachedFilterCtrl, widget.controller) &&
        _cachedCommentsRev == rev &&
        _cachedSourceLen == source.length &&
        _cachedSourceLastId == lastId &&
        _cachedFilterMode == _filter &&
        _cachedFilterQuery == _searchQuery) {
      return _cachedFilteredComments;
    }
    final result = source.where((c) {
      if (_searchQuery.isNotEmpty) {
        final hit = c.message.toLowerCase().contains(_searchQuery) ||
            c.userName.toLowerCase().contains(_searchQuery) ||
            c.userMemo.toLowerCase().contains(_searchQuery);
        if (!hit) return false;
      }
      switch (_filter) {
        case _CommentFilter.all:
          return true;
        case _CommentFilter.firstTime:
          return c.isFirstTime;
        case _CommentFilter.member:
          return c.isMember;
        case _CommentFilter.monetization:
          return c.eventKind != null;
        case _CommentFilter.highlighted:
          return c.bubbleBgColor != null;
        case _CommentFilter.question:
          return c.message.contains('?') ||
              c.message.contains('？') ||
              c.message.contains('質問');
        case _CommentFilter.ngFlagged:
          return c.isNgMasked || c.hiddenFromOverlay;
      }
    }).toList();
    _cachedFilterCtrl = widget.controller;
    _cachedCommentsRev = rev;
    _cachedSourceLen = source.length;
    _cachedSourceLastId = lastId;
    _cachedFilterMode = _filter;
    _cachedFilterQuery = _searchQuery;
    _cachedFilteredComments = result;
    return result;
  }

  Widget _buildFilterChip(_CommentFilter target, String label) {
    final selected = _filter == target;
    return Padding(
      padding: const EdgeInsets.only(right: 6),
      child: ChoiceChip(
        visualDensity: VisualDensity.compact,
        label: Text(label, style: const TextStyle(fontSize: 11.5)),
        selected: selected,
        onSelected: (_) => setState(() => _filter = target),
      ),
    );
  }

  Future<void> _openConnectionHistoryDialog(StosupController ctrl) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _ConnectionHistoryDialog(
        controller: ctrl,
        onSelectUrl: (url, mode) {
          _urlCtrl.text = url;
          setState(() => _selectedMode = mode);
          ctrl.connectYouTube(url, mode);
        },
      ),
    );
  }

  Future<void> _openNgWordDialog(StosupController ctrl) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _NgWordManagerDialog(controller: ctrl),
    );
  }

  void _sendTestComment(StosupController ctrl) {
    final msg = _demoMsgCtrl.text.trim();
    if (msg.isEmpty) return;
    final name = _demoUserCtrl.text.trim();
    ctrl.sendDemoComment('', name, msg);
    _demoMsgCtrl.clear();
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.state});
  final ConnectionStateDto state;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    late Color color;
    late String label;
    switch (state.status) {
      case ConnectionStatus.connected:
        color = p.success;
        label = state.platform == PlatformKind.demo ? 'デモ配信中' : 'YouTube 接続中';
      case ConnectionStatus.connecting:
        color = p.warning;
        label = '接続中...';
      case ConnectionStatus.reconnecting:
        color = p.warning;
        label = '自動再接続中 (${state.reconnectAttempt}/10)';
      case ConnectionStatus.error:
        color = p.danger;
        label = '接続エラー';
      case ConnectionStatus.disconnected:
        color = p.textMuted;
        label = '未接続';
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(5),
        border: Border.all(color: color.withValues(alpha: 0.6)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: TextStyle(
              color: color,
              fontWeight: FontWeight.w700,
              fontSize: 12,
            ),
          ),
        ],
      ),
    );
  }
}

class _CommentBubbleCard extends StatelessWidget {
  const _CommentBubbleCard({
    required this.comment,
    required this.showMemo,
    required this.pickupEnabled,
    required this.isPicked,
    required this.timeLabel,
    required this.onPickToggle,
    required this.onMemoTap,
    required this.onBanTap,
    required this.onUnbanTap,
  });

  final CommentItem comment;
  final bool showMemo;
  final bool pickupEnabled;
  final bool isPicked;
  final String timeLabel;
  final VoidCallback onPickToggle;
  final VoidCallback onMemoTap;
  final VoidCallback onBanTap;
  final VoidCallback onUnbanTap;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final isMonetization = comment.eventKind != null;
    final eventAccent = isMonetization
        ? AppTheme.parseCssHex(comment.eventAccentColor, fallback: p.warning)
        : null;
    final isHighlighted = comment.bubbleBgColor != null;
    final bg = isHighlighted
        ? AppTheme.parseCssHex(comment.bubbleBgColor, fallback: p.cardBg)
        : (eventAccent != null
            ? eventAccent.withValues(alpha: 0.14)
            : p.cardBg);
    final fg = comment.bubbleTextColor != null
        ? AppTheme.parseCssHex(comment.bubbleTextColor, fallback: p.textPrimary)
        : p.textPrimary;
    final border = comment.bubbleBorderColor != null
        ? AppTheme.parseCssHex(comment.bubbleBorderColor, fallback: p.border)
        : (isPicked
            ? p.accent
            : (eventAccent ?? p.border));

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Opacity(
        opacity: comment.isBanned ? 0.45 : 1.0,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            UserAvatar(
              name: comment.userName,
              iconUrl: comment.userIconUrl,
              radius: 17,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: bg,
                  borderRadius: const BorderRadius.only(
                    topLeft: Radius.circular(3),
                    topRight: Radius.circular(8),
                    bottomLeft: Radius.circular(8),
                    bottomRight: Radius.circular(8),
                  ),
                  border: Border.all(
                    color: comment.isBanned ? p.danger : border,
                    width:
                        (isHighlighted || isPicked || isMonetization) ? 1.8 : 1.0,
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Wrap(
                            spacing: 6,
                            runSpacing: 4,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              Text(
                                comment.userName,
                                style: TextStyle(
                                  color: fg.withValues(alpha: 0.9),
                                  fontWeight: FontWeight.bold,
                                  fontSize: 12.5,
                                ),
                              ),
                              if (showMemo && comment.userMemo.isNotEmpty)
                                InkWell(
                                  onTap: onMemoTap,
                                  borderRadius: BorderRadius.circular(4),
                                  child: Container(
                                    padding: const EdgeInsets.symmetric(
                                        horizontal: 6, vertical: 1),
                                    decoration: BoxDecoration(
                                      color: p.accent.withValues(alpha: 0.16),
                                      borderRadius: BorderRadius.circular(4),
                                      border: Border.all(
                                        color: p.accent.withValues(alpha: 0.45),
                                      ),
                                    ),
                                    child: Row(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        Icon(Icons.note_outlined,
                                            size: 11, color: p.accent),
                                        const SizedBox(width: 3),
                                        Text(
                                          comment.userMemo,
                                          style: TextStyle(
                                            color: fg,
                                            fontSize: 11,
                                            fontWeight: FontWeight.w600,
                                          ),
                                        ),
                                      ],
                                    ),
                                  ),
                                ),
                              if (isMonetization &&
                                  (comment.amountDisplay?.isNotEmpty ?? false))
                                _MiniBadge(
                                  comment.amountDisplay!,
                                  eventAccent ?? p.warning,
                                  Colors.black,
                                ),
                              if (comment.isFirstTime)
                                _MiniBadge('初見', p.success, Colors.white),
                              if (comment.isOwner)
                                _MiniBadge('配信者', p.warning, Colors.black),
                              if (comment.isModerator)
                                _MiniBadge('MOD', p.accent, Colors.white),
                              if (comment.isMember)
                                _MiniBadge('メンバー', p.success, Colors.white),
                              if (comment.isBanned)
                                _MiniBadge('BAN', p.danger, Colors.white),
                              if (comment.hiddenFromOverlay)
                                _MiniBadge('OBS非表示', p.warning, Colors.black),
                              if (comment.isNgMasked)
                                _MiniBadge('NG伏せ字', p.textMuted, Colors.white),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          timeLabel,
                          style: TextStyle(
                            color: fg.withValues(alpha: 0.55),
                            fontSize: 11,
                          ),
                        ),
                        if (pickupEnabled) ...[
                          const SizedBox(width: 2),
                          IconButton(
                            tooltip: isPicked
                                ? 'スポットテロップを解除'
                                : 'スポットテロップに表示',
                            visualDensity: VisualDensity.compact,
                            padding: EdgeInsets.zero,
                            constraints: const BoxConstraints(
                              minWidth: 26,
                              minHeight: 26,
                            ),
                            onPressed: onPickToggle,
                            icon: Icon(
                              isPicked
                                  ? Icons.push_pin
                                  : Icons.push_pin_outlined,
                              size: 15,
                              color: isPicked
                                  ? p.accent
                                  : fg.withValues(alpha: 0.6),
                            ),
                          ),
                        ],
                        const SizedBox(width: 2),
                        PopupMenuButton<String>(
                          icon: Icon(Icons.more_vert,
                              size: 15, color: fg.withValues(alpha: 0.65)),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                          onSelected: (v) {
                            if (v == 'memo') onMemoTap();
                            if (v == 'ban') onBanTap();
                            if (v == 'unban') onUnbanTap();
                          },
                          itemBuilder: (_) => [
                            const PopupMenuItem(
                              value: 'memo',
                              child: Text('名前メモを編集'),
                            ),
                            if (!comment.isBanned)
                              const PopupMenuItem(
                                value: 'ban',
                                child: Text('このユーザーをBAN（抽選・カウント除外）'),
                              )
                            else
                              const PopupMenuItem(
                                value: 'unban',
                                child: Text('BANを解除'),
                              ),
                          ],
                        ),
                      ],
                    ),
                    const SizedBox(height: 3),
                    RichCommentText(
                      message: comment.message,
                      runs: comment.messageRuns,
                      emojiSize: 22,
                      style: TextStyle(
                        color: fg,
                        fontSize: 14,
                        height: 1.4,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MiniBadge extends StatelessWidget {
  const _MiniBadge(this.text, this.bg, this.fg);
  final String text;
  final Color bg;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: TextStyle(color: fg, fontSize: 10, fontWeight: FontWeight.bold),
      ),
    );
  }
}

/// 接続履歴 ＆ お気に入りチャンネル自動検出ダイアログ
class _ConnectionHistoryDialog extends StatefulWidget {
  const _ConnectionHistoryDialog({
    required this.controller,
    required this.onSelectUrl,
  });

  final StosupController controller;
  final void Function(String url, ConnectionMode mode) onSelectUrl;

  @override
  State<_ConnectionHistoryDialog> createState() =>
      _ConnectionHistoryDialogState();
}

class _ConnectionHistoryDialogState extends State<_ConnectionHistoryDialog> {
  final _chLabelCtrl = TextEditingController();
  final _chInputCtrl = TextEditingController();
  late List<ConnectionHistoryItem> _lastHistory;
  late List<SavedChannelItem> _lastSavedChannels;

  @override
  void initState() {
    super.initState();
    _lastHistory = widget.controller.connectionHistory;
    _lastSavedChannels = widget.controller.savedChannels;
    widget.controller.addListener(_onControllerChanged);
  }

  void _onControllerChanged() {
    if (!mounted) return;
    // コメント受信ごとの不要なダイアログ再ビルドを防ぎ、履歴・チャンネル更新時のみ再描画する
    if (!identical(_lastHistory, widget.controller.connectionHistory) ||
        !identical(_lastSavedChannels, widget.controller.savedChannels)) {
      _lastHistory = widget.controller.connectionHistory;
      _lastSavedChannels = widget.controller.savedChannels;
      setState(() {});
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onControllerChanged);
    _chLabelCtrl.dispose();
    _chInputCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    return AlertDialog(
          title: const Text('接続履歴 ＆ チャンネル自動検出'),
          content: SizedBox(
            width: 640,
            height: 460,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  '自動検出用チャンネル（@ハンドル / チャンネルURL）',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13.5),
                ),
                const SizedBox(height: 6),
                Row(
                  children: [
                    SizedBox(
                      width: 150,
                      child: TextField(
                        controller: _chLabelCtrl,
                        decoration: const InputDecoration(
                          isDense: true,
                          labelText: '表示名',
                          hintText: '自分のチャンネル',
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: TextField(
                        controller: _chInputCtrl,
                        decoration: const InputDecoration(
                          isDense: true,
                          labelText: '@ハンドル または チャンネルURL',
                          hintText: '例: @mychannel',
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    FilledButton.icon(
                      onPressed: () async {
                        final input = _chInputCtrl.text.trim();
                        if (input.isEmpty) return;
                        await ctrl.saveSavedChannel(
                          SavedChannelItem(
                            id: '',
                            label: _chLabelCtrl.text.trim().isEmpty
                                ? input
                                : _chLabelCtrl.text.trim(),
                            channelInput: input,
                            lastVideoId: '',
                            lastCheckedMs: 0,
                          ),
                        );
                        _chLabelCtrl.clear();
                        _chInputCtrl.clear();
                      },
                      icon: const Icon(Icons.add, size: 16),
                      label: const Text('登録'),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                if (ctrl.savedChannels.isNotEmpty)
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: ctrl.savedChannels
                        .map(
                          (ch) => InputChip(
                            avatar: Icon(Icons.radar, size: 15, color: p.accent),
                            label: Text('${ch.label} (${ch.channelInput})'),
                            onPressed: () {
                              Navigator.pop(context);
                              widget.onSelectUrl(
                                ch.channelInput,
                                ConnectionMode.youTubeInnerTube,
                              );
                            },
                            onDeleted: () => ctrl.deleteSavedChannel(ch.id),
                          ),
                        )
                        .toList(),
                  ),
                const Divider(height: 22),
                const Text(
                  '過去の接続履歴（クリックで再接続）',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13.5),
                ),
                const SizedBox(height: 6),
                Expanded(
                  child: ctrl.connectionHistory.isEmpty
                      ? Center(
                          child: Text(
                            'まだ接続履歴がありません',
                            style: TextStyle(color: p.textSecondary),
                          ),
                        )
                      : ListView.separated(
                          itemCount: ctrl.connectionHistory.length,
                          separatorBuilder: (_, _) => const Divider(height: 1),
                          itemBuilder: (_, idx) {
                            final h = ctrl.connectionHistory[idx];
                            return ListTile(
                              dense: true,
                              leading: IconButton(
                                icon: Icon(
                                  h.isFavorite ? Icons.star : Icons.star_border,
                                  color: h.isFavorite ? p.warning : p.textMuted,
                                  size: 19,
                                ),
                                onPressed: () =>
                                    ctrl.toggleConnectionHistoryFavorite(
                                        h.id, !h.isFavorite),
                              ),
                              title: Text(
                                h.streamTitle.isEmpty
                                    ? h.streamId
                                    : h.streamTitle,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                              ),
                              subtitle: Text(
                                '${h.streamId} • ${h.mode == ConnectionMode.youTubeDataApi ? "Data API" : "InnerTube"}',
                                style: TextStyle(
                                    fontSize: 11.5, color: p.textSecondary),
                              ),
                              trailing: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  FilledButton.tonal(
                                    onPressed: () {
                                      Navigator.pop(context);
                                      widget.onSelectUrl(h.streamId, h.mode);
                                    },
                                    child: const Text('接続'),
                                  ),
                                  IconButton(
                                    icon: Icon(Icons.delete_outline,
                                        size: 18, color: p.textMuted),
                                    onPressed: () =>
                                        ctrl.deleteConnectionHistory(h.id),
                                  ),
                                ],
                              ),
                            );
                          },
                        ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('閉じる'),
            ),
          ],
        );
  }
}

/// NG ワード防御ルール管理ダイアログ
class _NgWordManagerDialog extends StatefulWidget {
  const _NgWordManagerDialog({required this.controller});
  final StosupController controller;

  @override
  State<_NgWordManagerDialog> createState() => _NgWordManagerDialogState();
}

class _NgWordManagerDialogState extends State<_NgWordManagerDialog> {
  final _patternCtrl = TextEditingController();
  final _replCtrl = TextEditingController(text: '***');
  MatchMode _mode = MatchMode.contains;
  NgWordAction _action = NgWordAction.hideOnOverlay;
  late List<NgWordRule> _lastNgRules;

  @override
  void initState() {
    super.initState();
    _lastNgRules = widget.controller.ngWordRules;
    widget.controller.addListener(_onControllerChanged);
  }

  void _onControllerChanged() {
    if (!mounted) return;
    // コメント受信ごとの不要なダイアログ再ビルドを防ぎ、NG ルール更新時のみ再描画する
    if (!identical(_lastNgRules, widget.controller.ngWordRules)) {
      _lastNgRules = widget.controller.ngWordRules;
      setState(() {});
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onControllerChanged);
    _patternCtrl.dispose();
    _replCtrl.dispose();
    super.dispose();
  }

  String _actionLabel(NgWordAction a) {
    switch (a) {
      case NgWordAction.hideEverywhere:
        return '完全非表示（アプリ・OBS・集計から除外）';
      case NgWordAction.hideOnOverlay:
        return 'OBS のみ非表示（アプリ内で確認可）';
      case NgWordAction.maskText:
        return '伏せ字マスク置換';
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    return AlertDialog(
          title: const Text('NG ワード防御ルール管理'),
          content: SizedBox(
            width: 660,
            height: 480,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '荒らしコメントやネタバレを自動で遮断・伏せ字化し、OBS 配信画面への映り込みを防止します。',
                  style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    SizedBox(
                      width: 190,
                      child: TextField(
                        controller: _patternCtrl,
                        decoration: const InputDecoration(
                          isDense: true,
                          labelText: 'NG ワード / 正規表現',
                          hintText: '例: ネタバレ',
                        ),
                      ),
                    ),
                    DropdownButton<MatchMode>(
                      value: _mode,
                      items: const [
                        DropdownMenuItem(
                            value: MatchMode.contains, child: Text('部分一致')),
                        DropdownMenuItem(
                            value: MatchMode.exact, child: Text('完全一致')),
                        DropdownMenuItem(
                            value: MatchMode.regex, child: Text('正規表現')),
                      ],
                      onChanged: (v) {
                        if (v != null) setState(() => _mode = v);
                      },
                    ),
                    DropdownButton<NgWordAction>(
                      value: _action,
                      items: const [
                        DropdownMenuItem(
                          value: NgWordAction.hideOnOverlay,
                          child: Text('OBS非表示'),
                        ),
                        DropdownMenuItem(
                          value: NgWordAction.maskText,
                          child: Text('伏せ字置換'),
                        ),
                        DropdownMenuItem(
                          value: NgWordAction.hideEverywhere,
                          child: Text('完全非表示'),
                        ),
                      ],
                      onChanged: (v) {
                        if (v != null) setState(() => _action = v);
                      },
                    ),
                    if (_action == NgWordAction.maskText)
                      SizedBox(
                        width: 90,
                        child: TextField(
                          controller: _replCtrl,
                          decoration: const InputDecoration(
                            isDense: true,
                            labelText: '置換文字',
                          ),
                        ),
                      ),
                    FilledButton.icon(
                      onPressed: () async {
                        final pat = _patternCtrl.text.trim();
                        if (pat.isEmpty) return;
                        await ctrl.saveNgWordRule(
                          NgWordRule(
                            id: '',
                            pattern: pat,
                            matchMode: _mode,
                            caseSensitive: false,
                            action: _action,
                            replacement: _replCtrl.text.trim().isEmpty
                                ? '***'
                                : _replCtrl.text.trim(),
                            enabled: true,
                          ),
                        );
                        _patternCtrl.clear();
                      },
                      icon: const Icon(Icons.add, size: 16),
                      label: const Text('追加'),
                    ),
                  ],
                ),
                const Divider(height: 20),
                Expanded(
                  child: ctrl.ngWordRules.isEmpty
                      ? Center(
                          child: Text(
                            'NG ワードルールはまだ登録されていません',
                            style: TextStyle(color: p.textSecondary),
                          ),
                        )
                      : ListView.separated(
                          itemCount: ctrl.ngWordRules.length,
                          separatorBuilder: (_, _) => const Divider(height: 1),
                          itemBuilder: (_, idx) {
                            final r = ctrl.ngWordRules[idx];
                            return ListTile(
                              dense: true,
                              leading: Switch(
                                value: r.enabled,
                                onChanged: (v) => ctrl.saveNgWordRule(
                                  NgWordRule(
                                    id: r.id,
                                    pattern: r.pattern,
                                    matchMode: r.matchMode,
                                    caseSensitive: r.caseSensitive,
                                    action: r.action,
                                    replacement: r.replacement,
                                    enabled: v,
                                  ),
                                ),
                              ),
                              title: Text(
                                r.pattern,
                                style: const TextStyle(fontWeight: FontWeight.bold),
                              ),
                              subtitle: Text(
                                '${_actionLabel(r.action)}${r.action == NgWordAction.maskText ? " (→ ${r.replacement})" : ""}',
                                style: TextStyle(
                                    fontSize: 11.5, color: p.textSecondary),
                              ),
                              trailing: IconButton(
                                icon: Icon(Icons.delete_outline,
                                    color: p.danger, size: 19),
                                onPressed: () => ctrl.deleteNgWordRule(r.id),
                              ),
                            );
                          },
                        ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('閉じる'),
            ),
          ],
        );
  }
}
