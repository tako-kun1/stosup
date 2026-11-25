import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

enum _UserFilter { all, currentSession, bannedOnly }

enum _UserSort { lastSeen, watchCount, commentCount, name }

/// コメント参加ユーザーリスト ＆ 視聴回数・コメント回数・BAN管理画面
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
    final reasonCtrl = TextEditingController(text: u.banReason);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: AppTheme.surfaceDark,
        title: Text('${u.displayName} をアプリ内BANしますか？'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'BANされたユーザーはユーザー抽選・コメント抽選・ワードカウントに一切参加できなくなります。',
              style: TextStyle(fontSize: 13, color: Colors.white70),
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
            style: FilledButton.styleFrom(backgroundColor: AppTheme.accentRose),
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
        backgroundColor: AppTheme.surfaceDark,
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
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'コメント参加ユーザーリスト ＆ BAN 管理',
                      style:
                          TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
                    ),
                    SizedBox(height: 4),
                    Text(
                      '1配信につき1回でもコメントすると視聴回数が +1 されます。BANしたユーザーは抽選システム等に一切参加できなくなります。',
                      style: TextStyle(color: Colors.white60),
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
              const SizedBox(width: 12),
              _StatBadge('総ユーザー', '${allUsers.length}人', AppTheme.accentSky),
              const SizedBox(width: 8),
              _StatBadge('今回参加', '$inSessionCount人', AppTheme.accentEmerald),
              const SizedBox(width: 8),
              _StatBadge('BAN済み', '$bannedCount人', AppTheme.accentRose),
            ],
          ),
          const SizedBox(height: 16),
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
                    prefixIcon: Icon(Icons.search),
                    hintText: 'ユーザー名・ID・メモで検索...',
                  ),
                ),
              ),
              const SizedBox(width: 12),
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
              const SizedBox(width: 12),
              DropdownButtonHideUnderline(
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  decoration: BoxDecoration(
                    color: const Color(0xFF0F172A),
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: AppTheme.borderDark),
                  ),
                  child: DropdownButton<_UserSort>(
                    value: _sort,
                    dropdownColor: AppTheme.cardDark,
                    items: const [
                      DropdownMenuItem(
                          value: _UserSort.lastSeen, child: Text('最終コメント順')),
                      DropdownMenuItem(
                          value: _UserSort.watchCount, child: Text('視聴回数が多い順')),
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
          const SizedBox(height: 14),

          // ユーザーリスト
          Expanded(
            child: Card(
              child: filtered.isEmpty
                  ? const Center(
                      child: Text(
                        '条件に一致するユーザーがいません。配信に接続してコメントを受信すると自動で記録されます。',
                        style: TextStyle(color: Colors.white54),
                      ),
                    )
                  : ListView.separated(
                      itemCount: filtered.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, idx) {
                        final u = filtered[idx];
                        return ListTile(
                          contentPadding: const EdgeInsets.symmetric(
                              horizontal: 16, vertical: 6),
                          leading: UserAvatar(
                            name: u.displayName,
                            iconUrl: u.iconUrl,
                            radius: 21,
                            borderColor:
                                u.isBanned ? AppTheme.accentRose : null,
                          ),
                          title: Row(
                            children: [
                              Text(
                                u.displayName,
                                style: TextStyle(
                                  fontWeight: FontWeight.bold,
                                  color: u.isBanned
                                      ? AppTheme.accentRose
                                      : Colors.white,
                                ),
                              ),
                              const SizedBox(width: 8),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 7, vertical: 2),
                                decoration: BoxDecoration(
                                  color: Colors.white10,
                                  borderRadius: BorderRadius.circular(6),
                                ),
                                child: Text(
                                  u.platform == PlatformKind.youTube
                                      ? 'YouTube'
                                      : 'Demo',
                                  style: const TextStyle(
                                      fontSize: 11, color: Colors.white70),
                                ),
                              ),
                              if (u.inCurrentSession) ...[
                                const SizedBox(width: 6),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 7, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: AppTheme.accentEmerald
                                        .withValues(alpha: 0.2),
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: const Text(
                                    '今回参加',
                                    style: TextStyle(
                                        fontSize: 11,
                                        color: AppTheme.accentEmerald,
                                        fontWeight: FontWeight.bold),
                                  ),
                                ),
                              ],
                              if (u.isBanned) ...[
                                const SizedBox(width: 6),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 7, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: AppTheme.accentRose,
                                    borderRadius: BorderRadius.circular(6),
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
                            padding: const EdgeInsets.only(top: 4),
                            child: Text(
                              '視聴回数: ${u.watchCount}回   |   累計コメント: ${u.totalCommentCount}回'
                              '${u.inCurrentSession ? " (今回 ${u.sessionCommentCount}回)" : ""}'
                              '${u.memo.isNotEmpty ? "   |   メモ: ${u.memo}" : ""}',
                              style: const TextStyle(
                                  fontSize: 12.5, color: Colors.white70),
                            ),
                          ),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              IconButton(
                                tooltip: 'メモを編集',
                                onPressed: () => _editMemo(u),
                                icon: const Icon(Icons.note_alt_outlined,
                                    size: 19),
                              ),
                              const SizedBox(width: 4),
                              u.isBanned
                                  ? OutlinedButton.icon(
                                      onPressed: () => _toggleBan(u),
                                      icon: const Icon(Icons.undo, size: 16),
                                      label: const Text('BAN解除'),
                                    )
                                  : FilledButton.icon(
                                      style: FilledButton.styleFrom(
                                        backgroundColor: AppTheme.accentRose
                                            .withValues(alpha: 0.85),
                                      ),
                                      onPressed: () => _toggleBan(u),
                                      icon: const Icon(Icons.block, size: 16),
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
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.5)),
      ),
      child: Column(
        children: [
          Text(label,
              style: const TextStyle(fontSize: 11, color: Colors.white70)),
          Text(
            value,
            style: TextStyle(
                fontSize: 16, fontWeight: FontWeight.bold, color: color),
          ),
        ],
      ),
    );
  }
}
