import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

enum _UserFilter { all, currentSession, bannedOnly }

enum _UserSort { lastSeen, watchCount, commentCount, name }

/// コメント参加ユーザーリスト ＆ 視聴回数・コメント回数・BAN管理画面（ライト/ダーク両対応）
class UsersScreen extends StatefulWidget {
  const UsersScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<UsersScreen> createState() => _UsersScreenState();
}

class _UsersScreenState extends State<UsersScreen> {
  final _searchCtrl = TextEditingController();
  _UserFilter _filter = _UserFilter.all;
  _UserSort _sort = _UserSort.lastSeen;

  StosupController? _cachedController;
  List<UserRecord>? _cachedSource;
  int _cachedSourceLen = -1;
  int _cachedUsersRev = -1;
  String _cachedQuery = '';
  _UserFilter? _cachedFilter;
  _UserSort? _cachedSort;
  List<UserRecord> _cachedFiltered = const [];
  int _cachedInSessionCount = 0;
  int _cachedBannedCount = 0;

  @override
  void dispose() {
    _searchCtrl.dispose();
    super.dispose();
  }

  Future<void> _toggleBan(UserRecord u) async {
    if (u.isBanned) {
      await widget.controller.setUserBan(u.userId, u.platform, false, '');
      return;
    }
    final p = context.palette;
    final reasonCtrl = TextEditingController(text: u.banReason);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('${u.displayName} をアプリ内BANしますか？'),
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
                labelText: 'BAN 理由（任意）',
                hintText: '例: 荒らし・スパム行為',
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('キャンセル')),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: p.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('BANする'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await widget.controller
          .setUserBan(u.userId, u.platform, true, reasonCtrl.text.trim());
    }
    reasonCtrl.dispose();
  }

  Future<void> _editMemo(UserRecord u) async {
    final memoCtrl = TextEditingController(text: u.memo);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('${u.displayName} のメモ'),
        content: TextField(
          controller: memoCtrl,
          maxLines: 3,
          decoration: const InputDecoration(
            labelText: '配信者用メモ',
            hintText: '例: 常連さん、イラストを描いてくれた人',
          ),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('キャンセル')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('保存')),
        ],
      ),
    );
    if (ok == true) {
      await widget.controller
          .setUserMemo(u.userId, u.platform, memoCtrl.text.trim());
    }
    memoCtrl.dispose();
  }

  Future<void> _confirmDeleteUser(UserRecord u) async {
    final p = context.palette;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('「${u.displayName}」をリストから削除しますか？'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'このユーザーの視聴回数（${u.watchCount}回）・累計コメント数（${u.totalCommentCount}回）・名前メモ・初見検知履歴${u.isBanned ? "・BAN設定" : ""}が完全に削除されます。',
              style: TextStyle(fontSize: 13, color: p.textSecondary),
            ),
            const SizedBox(height: 8),
            Text(
              '削除後にもう一度コメントした場合は、未登録ユーザーとして新たに記録されます。',
              style: TextStyle(fontSize: 12.5, color: p.textSecondary),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton.icon(
            style: FilledButton.styleFrom(
              backgroundColor: p.danger,
              foregroundColor: Colors.white,
            ),
            onPressed: () => Navigator.pop(ctx, true),
            icon: const Icon(Icons.delete_outline, size: 16),
            label: const Text('削除する'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await widget.controller.deleteUser(
        u.userId,
        u.platform,
        displayName: u.displayName,
      );
    }
  }

  Future<void> _confirmResetUsers(int totalCount, int bannedCount) async {
    final keepBanned = await showDialog<bool>(
      context: context,
      builder: (ctx) => _ResetUsersDialog(
        totalCount: totalCount,
        bannedCount: bannedCount,
      ),
    );
    if (keepBanned != null) {
      await widget.controller.resetUsers(keepBanned: keepBanned);
    }
  }

  void _ensureFilteredCache(List<UserRecord> allUsers, String query) {
    final usersRev = widget.controller.topicRevision(StosupTopic.users);
    if (identical(widget.controller, _cachedController) &&
        identical(allUsers, _cachedSource) &&
        allUsers.length == _cachedSourceLen &&
        usersRev == _cachedUsersRev &&
        query == _cachedQuery &&
        _filter == _cachedFilter &&
        _sort == _cachedSort) {
      return;
    }

    var inSession = 0;
    var banned = 0;
    final filtered = <UserRecord>[];
    for (final u in allUsers) {
      if (u.inCurrentSession) inSession++;
      if (u.isBanned) banned++;

      if (_filter == _UserFilter.currentSession && !u.inCurrentSession) {
        continue;
      }
      if (_filter == _UserFilter.bannedOnly && !u.isBanned) {
        continue;
      }
      if (query.isNotEmpty) {
        final hit = u.displayName.toLowerCase().contains(query) ||
            u.userId.toLowerCase().contains(query) ||
            u.memo.toLowerCase().contains(query);
        if (!hit) continue;
      }
      filtered.add(u);
    }

    filtered.sort((a, b) {
      switch (_sort) {
        case _UserSort.lastSeen:
          return b.lastSeenMs.compareTo(a.lastSeenMs);
        case _UserSort.watchCount:
          return b.watchCount.compareTo(a.watchCount);
        case _UserSort.commentCount:
          return b.totalCommentCount.compareTo(a.totalCommentCount);
        case _UserSort.name:
          return a.displayName.compareTo(b.displayName);
      }
    });

    _cachedController = widget.controller;
    _cachedSource = allUsers;
    _cachedSourceLen = allUsers.length;
    _cachedUsersRev = usersRev;
    _cachedQuery = query;
    _cachedFilter = _filter;
    _cachedSort = _sort;
    _cachedFiltered = filtered;
    _cachedInSessionCount = inSession;
    _cachedBannedCount = banned;
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final allUsers = widget.controller.users;
    final query = _searchCtrl.text.trim().toLowerCase();
    _ensureFilteredCache(allUsers, query);

    final filtered = _cachedFiltered;
    final inSessionCount = _cachedInSessionCount;
    final bannedCount = _cachedBannedCount;
    final trackingEnabled = widget.controller.featureFlags.userTrackingEnabled;

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'リスナー名簿 ＆ BAN 管理',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '1配信につき1回でもコメントすると視聴回数が +1 されます。BANしたユーザーはドローボックス等に一切参加できなくなります。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: '自動記録',
                enabled: trackingEnabled,
                onChanged: (v) => widget.controller.setFeatureEnabled(
                  userTracking: v,
                  featureLabel: 'リスナー名簿（自動記録）',
                ),
              ),
              const SizedBox(width: 10),
              _StatBadge('総ユーザー', '${allUsers.length}人', p.accent),
              const SizedBox(width: 6),
              _StatBadge('今回参加', '$inSessionCount人', p.success),
              const SizedBox(width: 6),
              _StatBadge('BAN済み', '$bannedCount人', p.danger),
            ],
          ),
          const SizedBox(height: 14),
          if (!trackingEnabled)
            FeatureDisabledBanner(
              featureName: 'リスナー名簿（自動記録）',
              description:
                  'コメント受信時の視聴回数・コメント回数の自動集計を停止しています（既存リストの閲覧および BAN 管理は引き続き利用できます）。',
              onEnable: () => widget.controller.setFeatureEnabled(
                userTracking: true,
                featureLabel: 'リスナー名簿（自動記録）',
              ),
            ),

          // 検索・フィルタ・並び替えバー
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _searchCtrl,
                  onChanged: (_) => setState(() {}),
                  decoration: const InputDecoration(
                    prefixIcon: Icon(Icons.search, size: 19),
                    hintText: 'ユーザー名・ID・メモで検索...',
                  ),
                ),
              ),
              const SizedBox(width: 10),
              SegmentedButton<_UserFilter>(
                segments: const [
                  ButtonSegment(value: _UserFilter.all, label: Text('全員')),
                  ButtonSegment(
                      value: _UserFilter.currentSession, label: Text('今回配信')),
                  ButtonSegment(
                      value: _UserFilter.bannedOnly, label: Text('BANのみ')),
                ],
                selected: {_filter},
                onSelectionChanged: (s) => setState(() => _filter = s.first),
              ),
              const SizedBox(width: 10),
              DropdownButtonHideUnderline(
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: p.inputBg,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: p.border),
                  ),
                  child: DropdownButton<_UserSort>(
                    value: _sort,
                    dropdownColor: p.cardBg,
                    style: TextStyle(color: p.textPrimary, fontSize: 13),
                    items: const [
                      DropdownMenuItem(
                          value: _UserSort.lastSeen, child: Text('最終コメント順')),
                      DropdownMenuItem(
                          value: _UserSort.watchCount,
                          child: Text('視聴回数が多い順')),
                      DropdownMenuItem(
                          value: _UserSort.commentCount,
                          child: Text('コメント回数が多い順')),
                      DropdownMenuItem(
                          value: _UserSort.name, child: Text('名前順')),
                    ],
                    onChanged: (v) {
                      if (v != null) setState(() => _sort = v);
                    },
                  ),
                ),
              ),
              const SizedBox(width: 10),
              OutlinedButton.icon(
                style: OutlinedButton.styleFrom(
                  foregroundColor: p.danger,
                  side: BorderSide(
                    color: allUsers.isEmpty
                        ? p.border
                        : p.danger.withValues(alpha: 0.6),
                  ),
                  padding:
                      const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
                ),
                onPressed: allUsers.isEmpty
                    ? null
                    : () => _confirmResetUsers(allUsers.length, bannedCount),
                icon: const Icon(Icons.delete_sweep_outlined, size: 17),
                label: const Text('一括リセット'),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // ユーザーリスト
          Expanded(
            child: Card(
              child: filtered.isEmpty
                  ? Center(
                      child: Text(
                        '条件に一致するユーザーがいません。配信に接続してコメントを受信すると自動で記録されます。',
                        style: TextStyle(color: p.textSecondary),
                      ),
                    )
                  : ListView.separated(
                      itemCount: filtered.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, idx) {
                        final u = filtered[idx];
                        return ListTile(
                          key: ValueKey('${u.platform.name}:${u.userId}'),
                          contentPadding: const EdgeInsets.symmetric(
                              horizontal: 14, vertical: 4),
                          leading: UserAvatar(
                            name: u.displayName,
                            iconUrl: u.iconUrl,
                            radius: 19,
                            borderColor: u.isBanned ? p.danger : null,
                          ),
                          title: Row(
                            children: [
                              Text(
                                u.displayName,
                                style: TextStyle(
                                  fontWeight: FontWeight.bold,
                                  color: u.isBanned ? p.danger : p.textPrimary,
                                ),
                              ),
                              const SizedBox(width: 8),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 6, vertical: 1),
                                decoration: BoxDecoration(
                                  color: p.subtleBg,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  u.platform == PlatformKind.youTube
                                      ? 'YouTube'
                                      : 'Demo',
                                  style: TextStyle(
                                      fontSize: 11, color: p.textSecondary),
                                ),
                              ),
                              if (u.inCurrentSession) ...[
                                const SizedBox(width: 6),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 6, vertical: 1),
                                  decoration: BoxDecoration(
                                    color: p.success.withValues(alpha: 0.14),
                                    borderRadius: BorderRadius.circular(4),
                                  ),
                                  child: Text(
                                    '今回参加',
                                    style: TextStyle(
                                        fontSize: 11,
                                        color: p.success,
                                        fontWeight: FontWeight.bold),
                                  ),
                                ),
                              ],
                              if (u.isBanned) ...[
                                const SizedBox(width: 6),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 6, vertical: 1),
                                  decoration: BoxDecoration(
                                    color: p.danger,
                                    borderRadius: BorderRadius.circular(4),
                                  ),
                                  child: Text(
                                    u.banReason.isEmpty
                                        ? 'BAN済み'
                                        : 'BAN: ${u.banReason}',
                                    style: const TextStyle(
                                        fontSize: 11,
                                        color: Colors.white,
                                        fontWeight: FontWeight.bold),
                                  ),
                                ),
                              ],
                            ],
                          ),
                          subtitle: Padding(
                            padding: const EdgeInsets.only(top: 3),
                            child: Text(
                              '視聴回数: ${u.watchCount}回   |   累計コメント: ${u.totalCommentCount}回'
                              '${u.inCurrentSession ? " (今回 ${u.sessionCommentCount}回)" : ""}'
                              '${u.memo.isNotEmpty ? "   |   メモ: ${u.memo}" : ""}',
                              style: TextStyle(
                                  fontSize: 12.5, color: p.textSecondary),
                            ),
                          ),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              IconButton(
                                tooltip: 'メモを編集',
                                onPressed: () => _editMemo(u),
                                icon: const Icon(Icons.note_alt_outlined,
                                    size: 18),
                              ),
                              const SizedBox(width: 4),
                              u.isBanned
                                  ? OutlinedButton.icon(
                                      onPressed: () => _toggleBan(u),
                                      icon: const Icon(Icons.undo, size: 15),
                                      label: const Text('BAN解除'),
                                    )
                                  : FilledButton.icon(
                                      style: FilledButton.styleFrom(
                                        backgroundColor: p.danger,
                                        foregroundColor: Colors.white,
                                      ),
                                      onPressed: () => _toggleBan(u),
                                      icon: const Icon(Icons.block, size: 15),
                                      label: const Text('BAN'),
                                    ),
                              const SizedBox(width: 4),
                              IconButton(
                                tooltip: 'リストから削除',
                                onPressed: () => _confirmDeleteUser(u),
                                icon: Icon(
                                  Icons.delete_outline,
                                  size: 19,
                                  color: p.danger,
                                ),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ResetUsersDialog extends StatefulWidget {
  const _ResetUsersDialog({
    required this.totalCount,
    required this.bannedCount,
  });

  final int totalCount;
  final int bannedCount;

  @override
  State<_ResetUsersDialog> createState() => _ResetUsersDialogState();
}

class _ResetUsersDialogState extends State<_ResetUsersDialog> {
  bool _keepBanned = true;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final targetCount = _keepBanned
        ? (widget.totalCount - widget.bannedCount)
        : widget.totalCount;

    return AlertDialog(
      title: const Text('リスナー名簿を一括リセットしますか？'),
      content: SizedBox(
        width: 460,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '登録済みユーザー（全 ${widget.totalCount} 人 / うち BAN 済み ${widget.bannedCount} 人）の視聴回数・累計コメント数・名前メモ・初見検知履歴を削除します。この操作は取り消せません。',
              style: TextStyle(fontSize: 13, color: p.textSecondary),
            ),
            const SizedBox(height: 12),
            Material(
              color: p.subtleBg,
              borderRadius: BorderRadius.circular(8),
              child: CheckboxListTile(
                value: _keepBanned,
                onChanged: (v) => setState(() => _keepBanned = v ?? true),
                controlAffinity: ListTileControlAffinity.leading,
                contentPadding:
                    const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
                title: const Text(
                  'BAN 済みユーザーは残す（推奨）',
                  style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.w600),
                ),
                subtitle: Text(
                  widget.bannedCount > 0
                      ? 'チェックを外すと BAN 済みの ${widget.bannedCount} 人も完全に削除されます。'
                      : '現在 BAN 済みのユーザーはいません。',
                  style: TextStyle(fontSize: 12, color: p.textSecondary),
                ),
              ),
            ),
            if (targetCount == 0) ...[
              const SizedBox(height: 10),
              Text(
                '※ 削除対象となる通常ユーザーが 0 人です。BAN 済みユーザーも含めて削除する場合は上のチェックを外してください。',
                style: TextStyle(fontSize: 12, color: p.warning),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, null),
          child: const Text('キャンセル'),
        ),
        FilledButton.icon(
          style: FilledButton.styleFrom(
            backgroundColor: p.danger,
            foregroundColor: Colors.white,
          ),
          onPressed: targetCount == 0
              ? null
              : () => Navigator.pop(context, _keepBanned),
          icon: const Icon(Icons.delete_sweep_outlined, size: 17),
          label: Text('リセットを実行（$targetCount人を削除）'),
        ),
      ],
    );
  }
}

class _StatBadge extends StatelessWidget {
  const _StatBadge(this.label, this.value, this.color);
  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 6),
      decoration: BoxDecoration(
        color: p.cardBg,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: p.border),
      ),
      child: Column(
        children: [
          Text(label, style: TextStyle(fontSize: 10.5, color: p.textSecondary)),
          Text(
            value,
            style: TextStyle(
                fontSize: 14.5, fontWeight: FontWeight.bold, color: color),
          ),
        ],
      ),
    );
  }
}
