import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

enum _MonetizationFilter { all, unread, checked, superchat, membership }

/// スーパーチャット / メンバーシップ専用表示 ＆ 読み忘れチェック画面
class SuperchatScreen extends StatefulWidget {
  const SuperchatScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<SuperchatScreen> createState() => _SuperchatScreenState();
}

class _SuperchatScreenState extends State<SuperchatScreen> {
  _MonetizationFilter _filter = _MonetizationFilter.all;

  String _formatTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    final s = dt.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  String _kindLabel(MonetizationEventKind kind) {
    switch (kind) {
      case MonetizationEventKind.superChat:
        return 'スーパーチャット';
      case MonetizationEventKind.superSticker:
        return 'スーパーステッカー';
      case MonetizationEventKind.membershipNew:
        return '新規メンバー加入';
      case MonetizationEventKind.membershipMilestone:
        return 'メンバー継続';
      case MonetizationEventKind.membershipGift:
        return 'メンバーシップギフト';
    }
  }

  bool _isSuperchatKind(MonetizationEventKind kind) =>
      kind == MonetizationEventKind.superChat ||
      kind == MonetizationEventKind.superSticker;

  void _copyUrl(String url, String label) {
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$label の OBS 用 URL をコピーしました')),
    );
  }

  CommentItem _toCommentItem(MonetizationItem it) {
    return CommentItem(
      id: it.id,
      sessionId: it.sessionId,
      platform: it.platform,
      userId: it.userId,
      userName: it.userName,
      userIconUrl: it.userIconUrl,
      userMemo: '',
      isFirstTime: false,
      message: it.message,
      timestampMs: it.timestampMs,
      isOwner: false,
      isModerator: false,
      isMember: !_isSuperchatKind(it.kind),
      isBanned: false,
      eventKind: it.kind,
      amountDisplay: it.amountDisplay,
      eventAccentColor: it.tierColor,
    );
  }

  Future<void> _openTestDialog() async {
    await showDialog<void>(
      context: context,
      builder: (_) => _TestMonetizationDialog(
        onSend: (kind, name, amount, msg) =>
            widget.controller.sendDemoMonetization(kind, name, amount, msg),
      ),
    );
  }

  Future<void> _confirmClear({required bool checkedOnly}) async {
    final p = context.palette;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(checkedOnly ? 'お礼済みの履歴をクリアしますか？' : 'すべての履歴をクリアしますか？'),
        content: Text(
          checkedOnly
              ? 'お礼済みチェックが付いた履歴のみ削除します（未読は残ります）。'
              : '未読を含め、記録されているスパチャ・メンバー履歴をすべて削除します。',
          style: TextStyle(color: p.textSecondary, fontSize: 13),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: p.danger,
              foregroundColor: Colors.white,
            ),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('削除する'),
          ),
        ],
      ),
    );
    if (ok == true) {
      await widget.controller.clearMonetizationEvents(checkedOnly: checkedOnly);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final enabled = ctrl.featureFlags.superchatEnabled;
    final all = ctrl.monetizationItems;
    final unreadCount = all.where((e) => !e.isChecked).length;
    final checkedCount = all.length - unreadCount;
    final overlayUrl = '${ctrl.overlayBaseUrl}/overlay/superchat';

    final filtered = all.where((e) {
      switch (_filter) {
        case _MonetizationFilter.all:
          return true;
        case _MonetizationFilter.unread:
          return !e.isChecked;
        case _MonetizationFilter.checked:
          return e.isChecked;
        case _MonetizationFilter.superchat:
          return _isSuperchatKind(e.kind);
        case _MonetizationFilter.membership:
          return !_isSuperchatKind(e.kind);
      }
    }).toList();

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
                      'スーパーチャット / メンバーシップ ＆ 読み忘れチェック',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '届いたスパチャ・ステッカー・メンバー加入を一覧管理し、お礼済みチェックや OBS ピックアップ表示ができます。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              FeatureToggleSwitch(
                label: 'スパチャ管理',
                enabled: enabled,
                onChanged: (v) => ctrl.setFeatureEnabled(
                  superchat: v,
                  featureLabel: 'スパチャ / メンバーシップ管理',
                ),
              ),
              const SizedBox(width: 10),
              _BadgeBox('未読', '$unreadCount件', unreadCount > 0 ? p.warning : p.textMuted),
              const SizedBox(width: 6),
              _BadgeBox('お礼済み', '$checkedCount件', p.success),
            ],
          ),
          const SizedBox(height: 12),
          if (!enabled)
            FeatureDisabledBanner(
              featureName: 'スパチャ / メンバーシップ管理',
              description:
                  '新規スパチャ・メンバー加入の専用記録と OBS スパチャオーバーレイを一時停止しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                superchat: true,
                featureLabel: 'スパチャ / メンバーシップ管理',
              ),
            ),

          // 操作バー
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SegmentedButton<_MonetizationFilter>(
                segments: const [
                  ButtonSegment(value: _MonetizationFilter.all, label: Text('すべて')),
                  ButtonSegment(value: _MonetizationFilter.unread, label: Text('未読のみ')),
                  ButtonSegment(value: _MonetizationFilter.checked, label: Text('お礼済み')),
                  ButtonSegment(value: _MonetizationFilter.superchat, label: Text('スパチャ')),
                  ButtonSegment(value: _MonetizationFilter.membership, label: Text('メンバー')),
                ],
                selected: {_filter},
                onSelectionChanged: (s) => setState(() => _filter = s.first),
              ),
              FilledButton.icon(
                onPressed: unreadCount == 0 ? null : ctrl.markAllMonetizationChecked,
                icon: const Icon(Icons.done_all, size: 16),
                label: const Text('すべてお礼済み'),
              ),
              OutlinedButton.icon(
                onPressed: _openTestDialog,
                icon: const Icon(Icons.science_outlined, size: 16),
                label: const Text('テスト送信'),
              ),
              OutlinedButton.icon(
                onPressed: () => _copyUrl(overlayUrl, 'スパチャ・メンバー表示'),
                icon: const Icon(Icons.copy, size: 15),
                label: const Text('OBS URL'),
              ),
              OutlinedButton.icon(
                onPressed: () =>
                    _copyUrl('$overlayUrl?unread_only=1', '未読のみスパチャ表示'),
                icon: const Icon(Icons.copy_all, size: 15),
                label: const Text('未読のみURL'),
              ),
              if (checkedCount > 0)
                TextButton.icon(
                  onPressed: () => _confirmClear(checkedOnly: true),
                  icon: const Icon(Icons.cleaning_services_outlined, size: 16),
                  label: const Text('お礼済みを削除'),
                ),
              if (all.isNotEmpty)
                TextButton.icon(
                  style: TextButton.styleFrom(foregroundColor: p.danger),
                  onPressed: () => _confirmClear(checkedOnly: false),
                  icon: const Icon(Icons.delete_sweep_outlined, size: 16),
                  label: const Text('全履歴クリア'),
                ),
            ],
          ),
          const SizedBox(height: 12),

          // リスト
          Expanded(
            child: Card(
              child: filtered.isEmpty
                  ? Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.volunteer_activism_outlined,
                              size: 44, color: p.textMuted),
                          const SizedBox(height: 10),
                          Text(
                            '該当するスパチャ・メンバーシップ履歴がありません',
                            style: TextStyle(color: p.textSecondary, fontSize: 14.5),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            'YouTube Live 接続時に自動記録されるほか、「テスト送信」から動作・OBS 表示を確認できます。',
                            style: TextStyle(color: p.textMuted, fontSize: 12.5),
                          ),
                        ],
                      ),
                    )
                  : ListView.separated(
                      padding: const EdgeInsets.all(12),
                      itemCount: filtered.length,
                      separatorBuilder: (_, _) => const SizedBox(height: 8),
                      itemBuilder: (context, idx) {
                        final it = filtered[idx];
                        final tier = AppTheme.parseCssHex(it.tierColor,
                            fallback: p.warning);
                        final isPicked =
                            ctrl.pickedComment.comment?.id == it.id;

                        return Opacity(
                          opacity: it.isChecked ? 0.62 : 1.0,
                          child: Container(
                            decoration: BoxDecoration(
                              color: p.cardBg,
                              borderRadius: BorderRadius.circular(8),
                              border: Border.all(
                                color: it.isChecked ? p.border : tier,
                                width: it.isChecked ? 1 : 1.8,
                              ),
                            ),
                            child: IntrinsicHeight(
                              child: Row(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Container(
                                    width: 6,
                                    decoration: BoxDecoration(
                                      color: tier,
                                      borderRadius: const BorderRadius.only(
                                        topLeft: Radius.circular(7),
                                        bottomLeft: Radius.circular(7),
                                      ),
                                    ),
                                  ),
                                  Expanded(
                                    child: Padding(
                                      padding: const EdgeInsets.symmetric(
                                          horizontal: 12, vertical: 10),
                                      child: Row(
                                        crossAxisAlignment:
                                            CrossAxisAlignment.start,
                                        children: [
                                          Checkbox(
                                            value: it.isChecked,
                                            onChanged: (v) =>
                                                ctrl.setMonetizationChecked(
                                                    it.id, v ?? false),
                                          ),
                                          const SizedBox(width: 4),
                                          UserAvatar(
                                            name: it.userName,
                                            iconUrl: it.userIconUrl,
                                            radius: 19,
                                            borderColor: tier,
                                          ),
                                          const SizedBox(width: 10),
                                          Expanded(
                                            child: Column(
                                              crossAxisAlignment:
                                                  CrossAxisAlignment.start,
                                              children: [
                                                Wrap(
                                                  spacing: 8,
                                                  runSpacing: 4,
                                                  crossAxisAlignment:
                                                      WrapCrossAlignment.center,
                                                  children: [
                                                    Text(
                                                      it.userName,
                                                      style: const TextStyle(
                                                        fontWeight:
                                                            FontWeight.bold,
                                                        fontSize: 14,
                                                      ),
                                                    ),
                                                    Container(
                                                      padding: const EdgeInsets
                                                          .symmetric(
                                                          horizontal: 7,
                                                          vertical: 2),
                                                      decoration: BoxDecoration(
                                                        color: tier,
                                                        borderRadius:
                                                            BorderRadius.circular(
                                                                4),
                                                      ),
                                                      child: Text(
                                                        it.amountDisplay,
                                                        style: const TextStyle(
                                                          color: Colors.black,
                                                          fontWeight:
                                                              FontWeight.w800,
                                                          fontSize: 12,
                                                        ),
                                                      ),
                                                    ),
                                                    Text(
                                                      _kindLabel(it.kind),
                                                      style: TextStyle(
                                                        fontSize: 11.5,
                                                        color: p.textSecondary,
                                                      ),
                                                    ),
                                                    Text(
                                                      _formatTime(
                                                          it.timestampMs),
                                                      style: TextStyle(
                                                        fontSize: 11,
                                                        color: p.textMuted,
                                                      ),
                                                    ),
                                                    if (it.isChecked)
                                                      Container(
                                                        padding:
                                                            const EdgeInsets
                                                                .symmetric(
                                                                horizontal: 6,
                                                                vertical: 1),
                                                        decoration:
                                                            BoxDecoration(
                                                          color: p.success
                                                              .withValues(
                                                                  alpha: 0.16),
                                                          borderRadius:
                                                              BorderRadius
                                                                  .circular(4),
                                                        ),
                                                        child: Text(
                                                          'お礼済み',
                                                          style: TextStyle(
                                                            fontSize: 10.5,
                                                            color: p.success,
                                                            fontWeight:
                                                                FontWeight.bold,
                                                          ),
                                                        ),
                                                      ),
                                                  ],
                                                ),
                                                if (it.message.isNotEmpty) ...[
                                                  const SizedBox(height: 5),
                                                  Text(
                                                    it.message,
                                                    style: TextStyle(
                                                      fontSize: 14,
                                                      color: p.textPrimary,
                                                      height: 1.4,
                                                    ),
                                                  ),
                                                ],
                                              ],
                                            ),
                                          ),
                                          const SizedBox(width: 8),
                                          if (ctrl.featureFlags.pickupEnabled)
                                            IconButton(
                                              tooltip: isPicked
                                                  ? 'ピックアップ解除'
                                                  : 'OBS にピックアップ表示',
                                              onPressed: () =>
                                                  ctrl.togglePickComment(
                                                      _toCommentItem(it)),
                                              icon: Icon(
                                                isPicked
                                                    ? Icons.push_pin
                                                    : Icons.push_pin_outlined,
                                                size: 18,
                                                color: isPicked
                                                    ? p.accent
                                                    : p.textSecondary,
                                              ),
                                            ),
                                          IconButton(
                                            tooltip: '履歴から削除',
                                            onPressed: () => ctrl
                                                .deleteMonetizationEvent(it.id),
                                            icon: Icon(
                                              Icons.delete_outline,
                                              size: 18,
                                              color: p.danger,
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
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

class _BadgeBox extends StatelessWidget {
  const _BadgeBox(this.label, this.value, this.color);
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

class _TestMonetizationDialog extends StatefulWidget {
  const _TestMonetizationDialog({required this.onSend});

  final Future<void> Function(
    MonetizationEventKind kind,
    String userName,
    String amountDisplay,
    String message,
  ) onSend;

  @override
  State<_TestMonetizationDialog> createState() =>
      _TestMonetizationDialogState();
}

class _TestMonetizationDialogState extends State<_TestMonetizationDialog> {
  MonetizationEventKind _kind = MonetizationEventKind.superChat;
  final _nameCtrl = TextEditingController(text: '応援リスナー');
  final _amountCtrl = TextEditingController(text: '¥1,000');
  final _msgCtrl = TextEditingController(text: 'いつも楽しい配信ありがとうございます！');

  @override
  void dispose() {
    _nameCtrl.dispose();
    _amountCtrl.dispose();
    _msgCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('スパチャ / メンバーシップ テスト送信'),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            DropdownButtonFormField<MonetizationEventKind>(
              initialValue: _kind,
              isExpanded: true,
              decoration: const InputDecoration(labelText: '種別'),
              items: const [
                DropdownMenuItem(
                  value: MonetizationEventKind.superChat,
                  child: Text('スーパーチャット'),
                ),
                DropdownMenuItem(
                  value: MonetizationEventKind.superSticker,
                  child: Text('スーパーステッカー'),
                ),
                DropdownMenuItem(
                  value: MonetizationEventKind.membershipNew,
                  child: Text('新規メンバー加入'),
                ),
                DropdownMenuItem(
                  value: MonetizationEventKind.membershipMilestone,
                  child: Text('メンバー継続マイルストーン'),
                ),
                DropdownMenuItem(
                  value: MonetizationEventKind.membershipGift,
                  child: Text('メンバーシップギフト'),
                ),
              ],
              onChanged: (v) {
                if (v == null) return;
                setState(() {
                  _kind = v;
                  if (v == MonetizationEventKind.superChat) {
                    _amountCtrl.text = '¥1,000';
                  } else if (v == MonetizationEventKind.superSticker) {
                    _amountCtrl.text = '¥500';
                  } else if (v == MonetizationEventKind.membershipNew) {
                    _amountCtrl.text = '新規メンバー加入';
                  } else if (v == MonetizationEventKind.membershipMilestone) {
                    _amountCtrl.text = 'メンバー 6ヶ月継続';
                  } else {
                    _amountCtrl.text = 'メンバーシップギフト 5件';
                  }
                });
              },
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(labelText: '視聴者名'),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _amountCtrl,
              decoration: const InputDecoration(labelText: '金額 / ラベル表示'),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _msgCtrl,
              maxLines: 2,
              decoration: const InputDecoration(labelText: 'メッセージ'),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('キャンセル'),
        ),
        FilledButton.icon(
          onPressed: () async {
            final navigator = Navigator.of(context);
            await widget.onSend(
              _kind,
              _nameCtrl.text.trim(),
              _amountCtrl.text.trim(),
              _msgCtrl.text.trim(),
            );
            navigator.pop();
          },
          icon: const Icon(Icons.send, size: 16),
          label: const Text('テスト送信'),
        ),
      ],
    );
  }
}
