import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

/// コメント表示 ＆ 配信接続画面（ライト/ダーク両対応）
class CommentsScreen extends StatefulWidget {
  const CommentsScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<CommentsScreen> createState() => _CommentsScreenState();
}

class _CommentsScreenState extends State<CommentsScreen> {
  final _urlCtrl = TextEditingController();
  final _demoUserCtrl = TextEditingController(text: 'テスト視聴者A');
  final _demoMsgCtrl = TextEditingController();
  final _postChatCtrl = TextEditingController();
  final _scrollCtrl = ScrollController();

  ConnectionMode _selectedMode = ConnectionMode.youTubeInnerTube;
  bool _autoScroll = true;
  int _lastCommentCount = 0;

  @override
  void dispose() {
    _urlCtrl.dispose();
    _demoUserCtrl.dispose();
    _demoMsgCtrl.dispose();
    _postChatCtrl.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  void _maybeScrollToBottom(int count) {
    if (_autoScroll && count != _lastCommentCount) {
      _lastCommentCount = count;
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
              'BANされたユーザーはユーザー抽選・コメント抽選・ワードカウントに一切参加できなくなります。',
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
    if (ok == true) {
      await widget.controller.setUserBan(
        c.userId,
        c.platform,
        true,
        reasonCtrl.text.trim(),
      );
    }
    reasonCtrl.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final conn = ctrl.connectionState;
    final connected = conn.status == ConnectionStatus.connected;
    _maybeScrollToBottom(ctrl.comments.length);

    return Column(
      children: [
        // 配信接続バー
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: p.sidebarBg,
            border: Border(bottom: BorderSide(color: p.border)),
          ),
          child: Column(
            children: [
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _urlCtrl,
                      decoration: InputDecoration(
                        prefixIcon:
                            Icon(Icons.live_tv, color: p.danger, size: 19),
                        labelText: 'YouTube Live 配信 URL または 動画 ID',
                        hintText:
                            '例: https://www.youtube.com/watch?v=XXXXXXXXXXX',
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
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
                  const SizedBox(width: 8),
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
                  const SizedBox(width: 6),
                  OutlinedButton.icon(
                    onPressed: ctrl.isConnecting
                        ? null
                        : () => ctrl.connectDemo(autoGenerate: true),
                    icon: const Icon(Icons.science_outlined, size: 17),
                    label: const Text('デモ配信'),
                  ),
                  if (connected) ...[
                    const SizedBox(width: 6),
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
                ],
              ),
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
                      child: Row(
                        children: [
                          Icon(Icons.chat_bubble_outline,
                              size: 17, color: p.textSecondary),
                          const SizedBox(width: 8),
                          Text(
                            'コメント一覧 (${ctrl.comments.length}件)',
                            style: const TextStyle(
                                fontWeight: FontWeight.bold, fontSize: 14),
                          ),
                          const SizedBox(width: 12),
                          FeatureToggleSwitch(
                            label: 'コメント表示',
                            enabled: ctrl.featureFlags.commentDisplayEnabled,
                            onChanged: (v) => ctrl.setFeatureEnabled(
                              commentDisplay: v,
                              featureLabel: 'コメント表示機能',
                            ),
                          ),
                          const Spacer(),
                          Text('自動スクロール',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Switch(
                            value: _autoScroll,
                            onChanged: (v) => setState(() => _autoScroll = v),
                          ),
                          const SizedBox(width: 6),
                          TextButton.icon(
                            onPressed: ctrl.comments.isEmpty
                                ? null
                                : ctrl.clearComments,
                            icon: const Icon(Icons.delete_sweep, size: 17),
                            label: const Text('クリア'),
                          ),
                        ],
                      ),
                    ),
                    const Divider(height: 1),
                    if (!ctrl.featureFlags.commentDisplayEnabled)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(14, 12, 14, 0),
                        child: FeatureDisabledBanner(
                          featureName: 'コメント表示機能',
                          description:
                              '新規コメントのアプリ内表示および OBS コメントオーバーレイ配信を一時停止しています。',
                          onEnable: () => ctrl.setFeatureEnabled(
                            commentDisplay: true,
                            featureLabel: 'コメント表示機能',
                          ),
                        ),
                      ),
                    Expanded(
                      child: ctrl.comments.isEmpty
                          ? Center(
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Icon(Icons.forum_outlined,
                                      size: 44, color: p.textMuted),
                                  const SizedBox(height: 10),
                                  Text(
                                    'コメントがまだありません',
                                    style: TextStyle(
                                        fontSize: 15, color: p.textSecondary),
                                  ),
                                  const SizedBox(height: 4),
                                  Text(
                                    'YouTube Live に接続するか「デモ配信」ボタンを押すとコメントが表示されます。',
                                    style: TextStyle(
                                        fontSize: 12.5, color: p.textMuted),
                                  ),
                                ],
                              ),
                            )
                          : ListView.builder(
                              controller: _scrollCtrl,
                              padding: const EdgeInsets.all(12),
                              itemCount: ctrl.comments.length,
                              itemBuilder: (context, idx) {
                                final c = ctrl.comments[idx];
                                return _CommentBubbleCard(
                                  comment: c,
                                  timeLabel: _formatTime(c.timestampMs),
                                  onBanTap: () => _confirmBan(c),
                                  onUnbanTap: () => ctrl.setUserBan(
                                      c.userId, c.platform, false, ''),
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
                    Row(
                      children: [
                        const Text(
                          '稼働中ワードカウンター',
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
    required this.timeLabel,
    required this.onBanTap,
    required this.onUnbanTap,
  });

  final CommentItem comment;
  final String timeLabel;
  final VoidCallback onBanTap;
  final VoidCallback onUnbanTap;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final isHighlighted = comment.bubbleBgColor != null;
    final bg = isHighlighted
        ? AppTheme.parseCssHex(comment.bubbleBgColor, fallback: p.cardBg)
        : p.cardBg;
    final fg = comment.bubbleTextColor != null
        ? AppTheme.parseCssHex(comment.bubbleTextColor, fallback: p.textPrimary)
        : p.textPrimary;
    final border = comment.bubbleBorderColor != null
        ? AppTheme.parseCssHex(comment.bubbleBorderColor, fallback: p.border)
        : p.border;

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
                    width: isHighlighted ? 1.5 : 1.0,
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          comment.userName,
                          style: TextStyle(
                            color: fg.withValues(alpha: 0.88),
                            fontWeight: FontWeight.bold,
                            fontSize: 12,
                          ),
                        ),
                        if (comment.isOwner) ...[
                          const SizedBox(width: 6),
                          _MiniBadge('配信者', p.warning, Colors.black),
                        ],
                        if (comment.isModerator) ...[
                          const SizedBox(width: 6),
                          _MiniBadge('MOD', p.accent, Colors.white),
                        ],
                        if (comment.isMember) ...[
                          const SizedBox(width: 6),
                          _MiniBadge('メンバー', p.success, Colors.white),
                        ],
                        if (comment.isBanned) ...[
                          const SizedBox(width: 6),
                          _MiniBadge('BAN', p.danger, Colors.white),
                        ],
                        const Spacer(),
                        Text(
                          timeLabel,
                          style: TextStyle(
                            color: fg.withValues(alpha: 0.55),
                            fontSize: 11,
                          ),
                        ),
                        const SizedBox(width: 4),
                        PopupMenuButton<String>(
                          icon: Icon(Icons.more_vert,
                              size: 15, color: fg.withValues(alpha: 0.65)),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                          onSelected: (v) {
                            if (v == 'ban') onBanTap();
                            if (v == 'unban') onUnbanTap();
                          },
                          itemBuilder: (_) => [
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
                    Text(
                      comment.message,
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
