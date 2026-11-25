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
              'BANされたユーザーはユーザー抽選・コメント抽選・ワードカウントに一切参加できなくなります。',
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

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final allUsers = widget.controller.users;
    final query = _searchCtrl.text.trim().toLowerCase();

    final filtered = allUsers.where((u) {
      if (_filter == _UserFilter.currentSession && !u.inCurrentSession) {
        return false;
      }
      if (_filter == _UserFilter.bannedOnly && !u.isBanned) {
        return false;
      }
      if (query.isNotEmpty) {
        return u.displayName.toLowerCase().contains(query) ||
            u.userId.toLowerCase().contains(query) ||
            u.memo.toLowerCase().contains(query);
      }
      return true;
    }).toList();

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

    final inSessionCount = allUsers.where((u) => u.inCurrentSession).length;
    final bannedCount = allUsers.where((u) => u.isBanned).length;
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
                      'コメント参加ユーザーリスト ＆ BAN 管理',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '1配信につき1回でもコメントすると視聴回数が +1 されます。BANしたユーザーは抽選システム等に一切参加できなくなります。',
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
                  featureLabel: 'ユーザーリスト自動記録',
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
              featureName: 'ユーザーリスト自動記録',
              description:
                  'コメント受信時の視聴回数・コメント回数の自動集計を停止しています（既存リストの閲覧および BAN 管理は引き続き利用できます）。',
              onEnable: () => widget.controller.setFeatureEnabled(
                userTracking: true,
                featureLabel: 'ユーザーリスト自動記録',
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
