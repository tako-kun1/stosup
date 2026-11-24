import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/user_avatar.dart';

/// ユーザー抽選システム ＆ コメント抽選システム画面
class LotteryScreen extends StatefulWidget {
  const LotteryScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<LotteryScreen> createState() => _LotteryScreenState();
}

class _LotteryScreenState extends State<LotteryScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabCtrl;

  // ユーザー抽選設定
  bool _currentSessionOnly = true;
  int _minWatchCount = 0;
  int _minCommentCount = 0;
  LotteryRepeatMode _userRepeatMode = LotteryRepeatMode.onceOnly;
  int _userWinnerCount = 1;

  // コメント抽選設定
  late TextEditingController _startMsgCtrl;
  late TextEditingController _endMsgCtrl;
  late TextEditingController _keywordCtrl;
  MatchMode _commentMatchMode = MatchMode.contains;
  bool _oneEntryPerUser = true;
  LotteryRepeatMode _commentRepeatMode = LotteryRepeatMode.onceOnly;
  int _commentWinnerCount = 1;
  bool _postToChat = true;

  @override
  void initState() {
    super.initState();
    _tabCtrl = TabController(length: 2, vsync: this);
    final cfg = widget.controller.commentLottery.config;
    _startMsgCtrl = TextEditingController(text: cfg.startMessage);
    _endMsgCtrl = TextEditingController(text: cfg.endMessage);
    _keywordCtrl = TextEditingController(text: cfg.targetKeyword);
    _commentMatchMode = cfg.matchMode;
    _oneEntryPerUser = cfg.oneEntryPerUser;
    _commentRepeatMode = cfg.repeatMode;
    _commentWinnerCount = cfg.winnerCount;
    _postToChat = cfg.postToChat;
  }

  @override
  void dispose() {
    _tabCtrl.dispose();
    _startMsgCtrl.dispose();
    _endMsgCtrl.dispose();
    _keywordCtrl.dispose();
    super.dispose();
  }

  CommentLotteryConfig _buildCommentConfig() {
    return CommentLotteryConfig(
      startMessage: _startMsgCtrl.text.trim(),
      endMessage: _endMsgCtrl.text.trim(),
      targetKeyword: _keywordCtrl.text.trim(),
      matchMode: _commentMatchMode,
      caseSensitive: false,
      oneEntryPerUser: _oneEntryPerUser,
      repeatMode: _commentRepeatMode,
      winnerCount: _commentWinnerCount,
      postToChat: _postToChat,
    );
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;

    return Column(
      children: [
        Container(
          color: AppTheme.surfaceDark,
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Text(
                    '抽選システム',
                    style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
                  ),
                  const Spacer(),
                  OutlinedButton.icon(
                    onPressed: ctrl.clearOverlayLottery,
                    icon: const Icon(Icons.layers_clear, size: 18),
                    label: const Text('OBS 抽選表示をクリア'),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              TabBar(
                controller: _tabCtrl,
                tabs: const [
                  Tab(
                    icon: Icon(Icons.people_alt_outlined),
                    text: 'ユーザー抽選（参加ユーザーから抽選）',
                  ),
                  Tab(
                    icon: Icon(Icons.mark_chat_unread_outlined),
                    text: 'コメント抽選（開始〜終了の間の特定ワード）',
                  ),
                ],
              ),
            ],
          ),
        ),
        Expanded(
          child: TabBarView(
            controller: _tabCtrl,
            children: [
              _buildUserLotteryTab(ctrl),
              _buildCommentLotteryTab(ctrl),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildUserLotteryTab(StosupController ctrl) {
    final wonSet = ctrl.userLotteryWonIds.toSet();
    final eligibleUsers = ctrl.users
        .where((u) => !u.isBanned)
        .where((u) => !_currentSessionOnly || u.inCurrentSession)
        .where((u) => u.watchCount >= BigInt.from(_minWatchCount))
        .where((u) => u.totalCommentCount >= BigInt.from(_minCommentCount))
        .where((u) =>
            _userRepeatMode == LotteryRepeatMode.allowMultiple ||
            !wonSet.contains(u.userId))
        .toList();

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 左: 条件設定カード
          SizedBox(
            width: 390,
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(18),
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('抽選設定',
                          style: TextStyle(
                              fontSize: 17, fontWeight: FontWeight.bold)),
                      const SizedBox(height: 12),
                      const Text('重複当選モード',
                          style: TextStyle(fontWeight: FontWeight.w600)),
                      const SizedBox(height: 8),
                      SizedBox(
                        width: double.infinity,
                        child: SegmentedButton<LotteryRepeatMode>(
                          segments: const [
                            ButtonSegment(
                              value: LotteryRepeatMode.onceOnly,
                              label: Text('1度のみ（重複なし）'),
                            ),
                            ButtonSegment(
                              value: LotteryRepeatMode.allowMultiple,
                              label: Text('複数回（重複あり）'),
                            ),
                          ],
                          selected: {_userRepeatMode},
                          onSelectionChanged: (s) =>
                              setState(() => _userRepeatMode = s.first),
                        ),
                      ),
                      const SizedBox(height: 14),
                      SwitchListTile(
                        contentPadding: EdgeInsets.zero,
                        value: _currentSessionOnly,
                        onChanged: (v) =>
                            setState(() => _currentSessionOnly = v),
                        title: const Text('現在の配信に参加したユーザーのみ'),
                        subtitle: const Text('OFFにすると過去配信の全登録ユーザーが対象'),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          const Text('当選人数: '),
                          const SizedBox(width: 8),
                          IconButton.outlined(
                            onPressed: _userWinnerCount > 1
                                ? () => setState(() => _userWinnerCount--)
                                : null,
                            icon: const Icon(Icons.remove, size: 16),
                          ),
                          Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 14),
                            child: Text(
                              '$_userWinnerCount 名',
                              style: const TextStyle(
                                  fontSize: 16, fontWeight: FontWeight.bold),
                            ),
                          ),
                          IconButton.outlined(
                            onPressed: _userWinnerCount < 20
                                ? () => setState(() => _userWinnerCount++)
                                : null,
                            icon: const Icon(Icons.add, size: 16),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          Expanded(
                            child: TextField(
                              keyboardType: TextInputType.number,
                              decoration: const InputDecoration(
                                labelText: '最小視聴回数',
                                hintText: '0',
                              ),
                              onChanged: (v) => setState(
                                  () => _minWatchCount = int.tryParse(v) ?? 0),
                            ),
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: TextField(
                              keyboardType: TextInputType.number,
                              decoration: const InputDecoration(
                                labelText: '最小コメント数',
                                hintText: '0',
                              ),
                              onChanged: (v) => setState(() =>
                                  _minCommentCount = int.tryParse(v) ?? 0),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: AppTheme.bgDark,
                          borderRadius: BorderRadius.circular(10),
                          border: Border.all(color: AppTheme.borderDark),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              '現在の抽選候補: ${eligibleUsers.length} 名（BANユーザー自動除外）',
                              style: const TextStyle(
                                  fontWeight: FontWeight.bold,
                                  color: AppTheme.accentSky),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              '既当選者数: ${ctrl.userLotteryWonIds.length} 名',
                              style: const TextStyle(
                                  fontSize: 12.5, color: Colors.white60),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 16),
                      SizedBox(
                        width: double.infinity,
                        height: 48,
                        child: FilledButton.icon(
                          onPressed: eligibleUsers.isEmpty
                              ? null
                              : () => ctrl.drawUserLottery(UserLotteryRequest(
                                    currentSessionOnly: _currentSessionOnly,
                                    minWatchCount: BigInt.from(_minWatchCount),
                                    minCommentCount:
                                        BigInt.from(_minCommentCount),
                                    repeatMode: _userRepeatMode,
                                    winnerCount: _userWinnerCount,
                                  )),
                          icon: const Icon(Icons.casino),
                          label: const Text('ユーザー抽選を実行！',
                              style: TextStyle(
                                  fontSize: 16, fontWeight: FontWeight.bold)),
                        ),
                      ),
                      const SizedBox(height: 10),
                      SizedBox(
                        width: double.infinity,
                        child: OutlinedButton.icon(
                          onPressed: ctrl.userLotteryWonIds.isEmpty
                              ? null
                              : ctrl.resetUserLotteryWinners,
                          icon: const Icon(Icons.refresh),
                          label: const Text('既当選リストをリセット'),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(width: 20),

          // 右: 当選者表示（アイコン・名前）＆ 抽選履歴
          Expanded(
            child: ListView(
              children: [
                _WinnerShowcaseCard(
                  title: '最新のユーザー抽選 当選者',
                  result: ctrl.lastUserLotteryResult,
                ),
                const SizedBox(height: 16),
                _LotteryHistoryCard(history: ctrl.lotteryHistory),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCommentLotteryTab(StosupController ctrl) {
    final cl = ctrl.commentLottery;

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 左: コメント抽選設定 ＆ 操作パネル
          SizedBox(
            width: 410,
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(18),
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          const Text('コメント抽選コントロール',
                              style: TextStyle(
                                  fontSize: 17, fontWeight: FontWeight.bold)),
                          const Spacer(),
                          Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 10, vertical: 4),
                            decoration: BoxDecoration(
                              color: (cl.isAccepting
                                      ? AppTheme.accentEmerald
                                      : Colors.white24)
                                  .withValues(alpha: 0.2),
                              borderRadius: BorderRadius.circular(999),
                              border: Border.all(
                                color: cl.isAccepting
                                    ? AppTheme.accentEmerald
                                    : Colors.white38,
                              ),
                            ),
                            child: Text(
                              cl.isAccepting ? '応募受付中' : '待機 / 終了',
                              style: TextStyle(
                                fontSize: 12,
                                fontWeight: FontWeight.bold,
                                color: cl.isAccepting
                                    ? AppTheme.accentEmerald
                                    : Colors.white70,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      TextField(
                        controller: _startMsgCtrl,
                        enabled: !cl.isAccepting,
                        decoration: const InputDecoration(
                          labelText: '開始コメント（チャットへ吐き出す文章）',
                        ),
                      ),
                      const SizedBox(height: 10),
                      TextField(
                        controller: _endMsgCtrl,
                        decoration: const InputDecoration(
                          labelText: '終了コメント（チャットへ吐き出す文章）',
                        ),
                      ),
                      const SizedBox(height: 10),
                      Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: _keywordCtrl,
                              enabled: !cl.isAccepting,
                              decoration: const InputDecoration(
                                labelText: '応募対象ワード（空欄なら全コメント）',
                                hintText: '例: 参加',
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          SizedBox(
                            width: 125,
                            child: DropdownButtonFormField<MatchMode>(
                              initialValue: _commentMatchMode,
                              dropdownColor: AppTheme.cardDark,
                              decoration:
                                  const InputDecoration(labelText: '一致方式'),
                              items: const [
                                DropdownMenuItem(
                                    value: MatchMode.contains,
                                    child: Text('部分一致')),
                                DropdownMenuItem(
                                    value: MatchMode.exact,
                                    child: Text('完全一致')),
                                DropdownMenuItem(
                                    value: MatchMode.regex,
                                    child: Text('正規表現')),
                              ],
                              onChanged: cl.isAccepting
                                  ? null
                                  : (v) {
                                      if (v != null) {
                                        setState(() => _commentMatchMode = v);
                                      }
                                    },
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),
                      SwitchListTile(
                        contentPadding: EdgeInsets.zero,
                        value: _postToChat,
                        onChanged: (v) => setState(() => _postToChat = v),
                        title: const Text('開始・終了コメントを配信チャットへ自動投稿'),
                        subtitle: const Text('YouTube OAuth 連携済みまたはデモ配信時に有効'),
                      ),
                      SwitchListTile(
                        contentPadding: EdgeInsets.zero,
                        value: _oneEntryPerUser,
                        onChanged: cl.isAccepting
                            ? null
                            : (v) => setState(() => _oneEntryPerUser = v),
                        title: const Text('1ユーザーにつき1応募のみ受付'),
                      ),
                      const SizedBox(height: 6),
                      SegmentedButton<LotteryRepeatMode>(
                        segments: const [
                          ButtonSegment(
                            value: LotteryRepeatMode.onceOnly,
                            label: Text('1度のみ'),
                          ),
                          ButtonSegment(
                            value: LotteryRepeatMode.allowMultiple,
                            label: Text('複数回OK'),
                          ),
                        ],
                        selected: {_commentRepeatMode},
                        onSelectionChanged: (s) =>
                            setState(() => _commentRepeatMode = s.first),
                      ),
                      const SizedBox(height: 10),
                      Row(
                        children: [
                          const Text('当選人数: '),
                          const SizedBox(width: 8),
                          IconButton.outlined(
                            onPressed: _commentWinnerCount > 1
                                ? () => setState(() => _commentWinnerCount--)
                                : null,
                            icon: const Icon(Icons.remove, size: 16),
                          ),
                          Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 12),
                            child: Text('$_commentWinnerCount 名',
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold)),
                          ),
                          IconButton.outlined(
                            onPressed: _commentWinnerCount < 20
                                ? () => setState(() => _commentWinnerCount++)
                                : null,
                            icon: const Icon(Icons.add, size: 16),
                          ),
                        ],
                      ),
                      if (cl.lastPostStatus != null) ...[
                        const SizedBox(height: 10),
                        Container(
                          width: double.infinity,
                          padding: const EdgeInsets.all(10),
                          decoration: BoxDecoration(
                            color: AppTheme.bgDark,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: AppTheme.borderDark),
                          ),
                          child: Text(
                            cl.lastPostStatus!,
                            style: const TextStyle(
                                fontSize: 12, color: AppTheme.accentAmber),
                          ),
                        ),
                      ],
                      const SizedBox(height: 14),
                      // ステップ操作ボタン
                      Row(
                        children: [
                          Expanded(
                            child: FilledButton.icon(
                              style: FilledButton.styleFrom(
                                backgroundColor: AppTheme.accentEmerald,
                                foregroundColor: Colors.black,
                              ),
                              onPressed: cl.isAccepting
                                  ? null
                                  : () => ctrl.startCommentLottery(
                                      _buildCommentConfig()),
                              icon: const Icon(Icons.play_arrow),
                              label: const Text('1. 受付開始'),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: FilledButton.icon(
                              style: FilledButton.styleFrom(
                                backgroundColor: AppTheme.accentRose,
                              ),
                              onPressed: cl.isAccepting
                                  ? ctrl.stopCommentLottery
                                  : null,
                              icon: const Icon(Icons.stop),
                              label: const Text('2. 受付終了'),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),
                      SizedBox(
                        width: double.infinity,
                        height: 46,
                        child: FilledButton.icon(
                          onPressed: cl.entries.isEmpty
                              ? null
                              : ctrl.drawCommentLottery,
                          icon: const Icon(Icons.casino),
                          label: Text(
                            '3. 応募コメント (${cl.entries.length}件) から抽選！',
                            style: const TextStyle(fontWeight: FontWeight.bold),
                          ),
                        ),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          Expanded(
                            child: OutlinedButton(
                              onPressed: () => ctrl.resetCommentLottery(
                                  clearWinnersOnly: true),
                              child: const Text('既当選リセット'),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: OutlinedButton(
                              onPressed: () => ctrl.resetCommentLottery(
                                  clearWinnersOnly: false),
                              child: const Text('応募を全クリア'),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(width: 20),

          // 右: 当選結果 ＆ 収集済み応募コメント一覧
          Expanded(
            child: Column(
              children: [
                _WinnerShowcaseCard(
                  title: 'コメント抽選 当選者',
                  result: cl.lastResult,
                ),
                const SizedBox(height: 14),
                Expanded(
                  child: Card(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Padding(
                          padding: const EdgeInsets.all(14),
                          child: Row(
                            children: [
                              const Icon(Icons.list_alt,
                                  color: AppTheme.accentSky, size: 18),
                              const SizedBox(width: 8),
                              Text(
                                '収集済み応募コメント (${cl.entries.length}件・BANユーザー除外済み)',
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold),
                              ),
                            ],
                          ),
                        ),
                        const Divider(height: 1),
                        Expanded(
                          child: cl.entries.isEmpty
                              ? const Center(
                                  child: Text(
                                    '「1. 受付開始」を押すと、指定ワードを含むコメントがここに自動で集まります。',
                                    style: TextStyle(color: Colors.white54),
                                  ),
                                )
                              : ListView.separated(
                                  itemCount: cl.entries.length,
                                  separatorBuilder: (_, _) =>
                                      const Divider(height: 1),
                                  itemBuilder: (_, idx) {
                                    final e = cl.entries[idx];
                                    return ListTile(
                                      leading: UserAvatar(
                                        name: e.userName,
                                        iconUrl: e.userIconUrl,
                                        radius: 18,
                                      ),
                                      title: Text(e.userName,
                                          style: const TextStyle(
                                              fontWeight: FontWeight.bold,
                                              fontSize: 13.5)),
                                      subtitle: Text(e.message),
                                    );
                                  },
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
    );
  }
}

class _WinnerShowcaseCard extends StatelessWidget {
  const _WinnerShowcaseCard({
    required this.title,
    required this.result,
  });

  final String title;
  final LotteryResult? result;

  @override
  Widget build(BuildContext context) {
    return Card(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(
          color: result != null ? AppTheme.accentAmber : AppTheme.borderDark,
          width: result != null ? 1.8 : 1,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.emoji_events, color: AppTheme.accentAmber),
                const SizedBox(width: 8),
                Text(title,
                    style: const TextStyle(
                        fontSize: 16, fontWeight: FontWeight.bold)),
                if (result != null) ...[
                  const Spacer(),
                  Text(
                    '母数: ${result!.candidateCount}件',
                    style:
                        const TextStyle(fontSize: 12, color: Colors.white60),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 14),
            if (result == null || result!.winners.isEmpty)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 20),
                child: Center(
                  child: Text(
                    '抽選を実行すると、当選者のアイコンと名前が大きく表示されます（OBSオーバーレイにも連動）。',
                    style: TextStyle(color: Colors.white54),
                  ),
                ),
              )
            else
              Wrap(
                spacing: 14,
                runSpacing: 14,
                children: result!.winners.map((w) {
                  return Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 18, vertical: 14),
                    decoration: BoxDecoration(
                      color: AppTheme.bgDark,
                      borderRadius: BorderRadius.circular(14),
                      border: Border.all(
                          color: AppTheme.accentAmber.withValues(alpha: 0.7)),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        UserAvatar(
                          name: w.userName,
                          iconUrl: w.userIconUrl,
                          radius: 28,
                          borderColor: AppTheme.accentAmber,
                        ),
                        const SizedBox(width: 14),
                        ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 320),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                w.userName,
                                style: const TextStyle(
                                  fontSize: 20,
                                  fontWeight: FontWeight.bold,
                                  color: AppTheme.accentAmber,
                                ),
                              ),
                              if (w.commentText != null &&
                                  w.commentText!.isNotEmpty) ...[
                                const SizedBox(height: 4),
                                Text(
                                  w.commentText!,
                                  style: const TextStyle(
                                      fontSize: 13.5, color: Colors.white),
                                ),
                              ],
                            ],
                          ),
                        ),
                      ],
                    ),
                  );
                }).toList(),
              ),
          ],
        ),
      ),
    );
  }
}

class _LotteryHistoryCard extends StatelessWidget {
  const _LotteryHistoryCard({required this.history});

  final List<LotteryResult> history;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('抽選履歴',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold)),
            const SizedBox(height: 10),
            if (history.isEmpty)
              const Text('まだ抽選履歴はありません。',
                  style: TextStyle(color: Colors.white54))
            else
              ...history.take(10).map((r) {
                final names = r.winners.map((w) => w.userName).join(', ');
                final dt = DateTime.fromMillisecondsSinceEpoch(r.drawnAtMs);
                final timeStr =
                    '${dt.month}/${dt.day} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
                return ListTile(
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  leading: Chip(
                    label: Text(
                      r.kind == 'comment' ? 'コメント' : 'ユーザー',
                      style: const TextStyle(fontSize: 11),
                    ),
                    visualDensity: VisualDensity.compact,
                  ),
                  title: Text(names,
                      style: const TextStyle(fontWeight: FontWeight.w600)),
                  trailing: Text(
                    timeStr,
                    style:
                        const TextStyle(fontSize: 12, color: Colors.white54),
                  ),
                );
              }),
          ],
        ),
      ),
    );
  }
}
