import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/user_avatar.dart';

/// ユーザー抽選システム ＆ コメント抽選システム画面（ライト/ダーク両対応）
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
    final p = context.palette;
    final ctrl = widget.controller;

    return Column(
      children: [
        Container(
          color: p.sidebarBg,
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Text(
                    '抽選システム',
                    style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                  ),
                  const Spacer(),
                  OutlinedButton.icon(
                    onPressed: ctrl.clearOverlayLottery,
                    icon: const Icon(Icons.layers_clear, size: 17),
                    label: const Text('OBS 抽選表示をクリア'),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              TabBar(
                controller: _tabCtrl,
                tabs: const [
                  Tab(text: 'ユーザー抽選（参加ユーザーから抽選）'),
                  Tab(text: 'コメント抽選（開始〜終了の間の特定ワード）'),
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
    final p = context.palette;
    final featureEnabled = ctrl.featureFlags.userLotteryEnabled;
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
      child: Column(
        children: [
          if (!featureEnabled)
            FeatureDisabledBanner(
              featureName: 'ユーザー抽選システム',
              description: 'ユーザー抽選の実行を無効化しています。利用するときにオンへ切り替えてください。',
              onEnable: () => ctrl.setFeatureEnabled(
                userLottery: true,
                featureLabel: 'ユーザー抽選システム',
              ),
            ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 左: 条件設定カード
                SizedBox(
                  width: 390,
                  child: Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                const Text('抽選設定',
                                    style: TextStyle(
                                        fontSize: 15.5,
                                        fontWeight: FontWeight.bold)),
                                const Spacer(),
                                FeatureToggleSwitch(
                                  label: 'ユーザー抽選',
                                  enabled: featureEnabled,
                                  onChanged: (v) => ctrl.setFeatureEnabled(
                                    userLottery: v,
                                    featureLabel: 'ユーザー抽選システム',
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            const Text('重複当選モード',
                                style: TextStyle(
                                    fontWeight: FontWeight.w600, fontSize: 13)),
                            const SizedBox(height: 6),
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
                            const SizedBox(height: 10),
                            SwitchListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              value: _currentSessionOnly,
                              onChanged: (v) =>
                                  setState(() => _currentSessionOnly = v),
                              title: const Text('現在の配信に参加したユーザーのみ'),
                              subtitle:
                                  const Text('OFFにすると過去配信の全登録ユーザーが対象'),
                            ),
                            const SizedBox(height: 8),
                            Row(
                              children: [
                                const Text('当選人数: '),
                                const SizedBox(width: 8),
                                IconButton.outlined(
                                  visualDensity: VisualDensity.compact,
                                  onPressed: _userWinnerCount > 1
                                      ? () =>
                                          setState(() => _userWinnerCount--)
                                      : null,
                                  icon: const Icon(Icons.remove, size: 15),
                                ),
                                Padding(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 12),
                                  child: Text(
                                    '$_userWinnerCount 名',
                                    style: const TextStyle(
                                        fontSize: 15,
                                        fontWeight: FontWeight.bold),
                                  ),
                                ),
                                IconButton.outlined(
                                  visualDensity: VisualDensity.compact,
                                  onPressed: _userWinnerCount < 20
                                      ? () =>
                                          setState(() => _userWinnerCount++)
                                      : null,
                                  icon: const Icon(Icons.add, size: 15),
                                ),
                              ],
                            ),
                            const SizedBox(height: 10),
                            Row(
                              children: [
                                Expanded(
                                  child: TextField(
                                    keyboardType: TextInputType.number,
                                    decoration: const InputDecoration(
                                      labelText: '最小視聴回数',
                                      hintText: '0',
                                    ),
                                    onChanged: (v) => setState(() =>
                                        _minWatchCount = int.tryParse(v) ?? 0),
                                  ),
                                ),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: TextField(
                                    keyboardType: TextInputType.number,
                                    decoration: const InputDecoration(
                                      labelText: '最小コメント数',
                                      hintText: '0',
                                    ),
                                    onChanged: (v) => setState(() =>
                                        _minCommentCount =
                                            int.tryParse(v) ?? 0),
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 14),
                            Container(
                              width: double.infinity,
                              padding: const EdgeInsets.all(10),
                              decoration: BoxDecoration(
                                color: p.windowBg,
                                borderRadius: BorderRadius.circular(6),
                                border: Border.all(color: p.border),
                              ),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    '現在の抽選候補: ${eligibleUsers.length} 名（BAN自動除外）',
                                    style: TextStyle(
                                        fontWeight: FontWeight.bold,
                                        fontSize: 13,
                                        color: p.accent),
                                  ),
                                  const SizedBox(height: 3),
                                  Text(
                                    '既当選者数: ${ctrl.userLotteryWonIds.length} 名',
                                    style: TextStyle(
                                        fontSize: 12, color: p.textSecondary),
                                  ),
                                ],
                              ),
                            ),
                            const SizedBox(height: 14),
                            SizedBox(
                              width: double.infinity,
                              height: 44,
                              child: FilledButton.icon(
                                onPressed:
                                    (!featureEnabled || eligibleUsers.isEmpty)
                                        ? null
                                        : () => ctrl.drawUserLottery(
                                                UserLotteryRequest(
                                              currentSessionOnly:
                                                  _currentSessionOnly,
                                              minWatchCount:
                                                  BigInt.from(_minWatchCount),
                                              minCommentCount: BigInt.from(
                                                  _minCommentCount),
                                              repeatMode: _userRepeatMode,
                                              winnerCount: _userWinnerCount,
                                            )),
                                icon: const Icon(Icons.casino, size: 18),
                                label: const Text('ユーザー抽選を実行',
                                    style: TextStyle(
                                        fontSize: 14.5,
                                        fontWeight: FontWeight.bold)),
                              ),
                            ),
                            const SizedBox(height: 8),
                            SizedBox(
                              width: double.infinity,
                              child: OutlinedButton.icon(
                                onPressed: ctrl.userLotteryWonIds.isEmpty
                                    ? null
                                    : ctrl.resetUserLotteryWinners,
                                icon: const Icon(Icons.refresh, size: 17),
                                label: const Text('既当選リストをリセット'),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 16),

                // 右: 当選者表示 ＆ 履歴
                Expanded(
                  child: ListView(
                    children: [
                      _WinnerShowcaseCard(
                        title: '最新のユーザー抽選 当選者',
                        result: ctrl.lastUserLotteryResult,
                      ),
                      const SizedBox(height: 14),
                      _LotteryHistoryCard(history: ctrl.lotteryHistory),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCommentLotteryTab(StosupController ctrl) {
    final p = context.palette;
    final cl = ctrl.commentLottery;
    final featureEnabled = ctrl.featureFlags.commentLotteryEnabled;

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        children: [
          if (!featureEnabled)
            FeatureDisabledBanner(
              featureName: 'コメント抽選システム',
              description: 'コメント抽選の受付開始・応募コメント自動収集・抽選実行を無効化しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                commentLottery: true,
                featureLabel: 'コメント抽選システム',
              ),
            ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 左: コメント抽選設定 ＆ 操作パネル
                SizedBox(
                  width: 405,
                  child: Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                const Expanded(
                                  child: Text('コメント抽選',
                                      style: TextStyle(
                                          fontSize: 15.5,
                                          fontWeight: FontWeight.bold)),
                                ),
                                FeatureToggleSwitch(
                                  label: 'コメント抽選',
                                  enabled: featureEnabled,
                                  onChanged: (v) => ctrl.setFeatureEnabled(
                                    commentLottery: v,
                                    featureLabel: 'コメント抽選システム',
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 8),
                            Align(
                              alignment: Alignment.centerRight,
                              child: Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 8, vertical: 3),
                                decoration: BoxDecoration(
                                  color: (cl.isAccepting
                                          ? p.success
                                          : p.textMuted)
                                      .withValues(alpha: 0.14),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  cl.isAccepting ? '応募受付中' : '待機 / 終了',
                                  style: TextStyle(
                                    fontSize: 11.5,
                                    fontWeight: FontWeight.bold,
                                    color: cl.isAccepting
                                        ? p.success
                                        : p.textSecondary,
                                  ),
                                ),
                              ),
                            ),
                            const SizedBox(height: 10),
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
                                  width: 120,
                                  child: DropdownButtonFormField<MatchMode>(
                                    initialValue: _commentMatchMode,
                                    isExpanded: true,
                                    dropdownColor: p.cardBg,
                                    decoration: const InputDecoration(
                                        labelText: '一致方式'),
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
                                              setState(
                                                  () => _commentMatchMode = v);
                                            }
                                          },
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 8),
                            SwitchListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              value: _postToChat,
                              onChanged: (v) => setState(() => _postToChat = v),
                              title: const Text('開始・終了コメントをチャットへ自動投稿'),
                            ),
                            SwitchListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              value: _oneEntryPerUser,
                              onChanged: cl.isAccepting
                                  ? null
                                  : (v) =>
                                      setState(() => _oneEntryPerUser = v),
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
                            const SizedBox(height: 8),
                            Row(
                              children: [
                                const Text('当選人数: '),
                                const SizedBox(width: 8),
                                IconButton.outlined(
                                  visualDensity: VisualDensity.compact,
                                  onPressed: _commentWinnerCount > 1
                                      ? () => setState(
                                          () => _commentWinnerCount--)
                                      : null,
                                  icon: const Icon(Icons.remove, size: 15),
                                ),
                                Padding(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 10),
                                  child: Text('$_commentWinnerCount 名',
                                      style: const TextStyle(
                                          fontWeight: FontWeight.bold)),
                                ),
                                IconButton.outlined(
                                  visualDensity: VisualDensity.compact,
                                  onPressed: _commentWinnerCount < 20
                                      ? () => setState(
                                          () => _commentWinnerCount++)
                                      : null,
                                  icon: const Icon(Icons.add, size: 15),
                                ),
                              ],
                            ),
                            if (cl.lastPostStatus != null) ...[
                              const SizedBox(height: 8),
                              Container(
                                width: double.infinity,
                                padding: const EdgeInsets.all(8),
                                decoration: BoxDecoration(
                                  color: p.windowBg,
                                  borderRadius: BorderRadius.circular(6),
                                  border: Border.all(color: p.border),
                                ),
                                child: Text(
                                  cl.lastPostStatus!,
                                  style: TextStyle(
                                      fontSize: 12, color: p.textSecondary),
                                ),
                              ),
                            ],
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Expanded(
                                  child: FilledButton.icon(
                                    style: FilledButton.styleFrom(
                                      backgroundColor: p.success,
                                      foregroundColor: Colors.white,
                                    ),
                                    onPressed:
                                        (!featureEnabled || cl.isAccepting)
                                            ? null
                                            : () => ctrl.startCommentLottery(
                                                _buildCommentConfig()),
                                    icon: const Icon(Icons.play_arrow, size: 17),
                                    label: const Text('1. 受付開始'),
                                  ),
                                ),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: FilledButton.icon(
                                    style: FilledButton.styleFrom(
                                      backgroundColor: p.danger,
                                      foregroundColor: Colors.white,
                                    ),
                                    onPressed: cl.isAccepting
                                        ? ctrl.stopCommentLottery
                                        : null,
                                    icon: const Icon(Icons.stop, size: 17),
                                    label: const Text('2. 受付終了'),
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 8),
                            SizedBox(
                              width: double.infinity,
                              height: 44,
                              child: FilledButton.icon(
                                onPressed:
                                    (!featureEnabled || cl.entries.isEmpty)
                                        ? null
                                        : ctrl.drawCommentLottery,
                                icon: const Icon(Icons.casino, size: 18),
                                label: Text(
                                  '3. 応募コメント (${cl.entries.length}件) から抽選',
                                  style: const TextStyle(
                                      fontWeight: FontWeight.bold),
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
                const SizedBox(width: 16),

                // 右: 当選結果 ＆ 収集済み応募一覧
                Expanded(
                  child: Column(
                    children: [
                      _WinnerShowcaseCard(
                        title: 'コメント抽選 当選者',
                        result: cl.lastResult,
                      ),
                      const SizedBox(height: 12),
                      Expanded(
                        child: Card(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Padding(
                                padding: const EdgeInsets.all(12),
                                child: Text(
                                  '収集済み応募コメント (${cl.entries.length}件・BAN除外済み)',
                                  style: const TextStyle(
                                      fontWeight: FontWeight.bold,
                                      fontSize: 13.5),
                                ),
                              ),
                              const Divider(height: 1),
                              Expanded(
                                child: cl.entries.isEmpty
                                    ? Center(
                                        child: Text(
                                          '「1. 受付開始」を押すと指定ワードを含むコメントがここに集まります。',
                                          style:
                                              TextStyle(color: p.textSecondary),
                                        ),
                                      )
                                    : ListView.separated(
                                        itemCount: cl.entries.length,
                                        separatorBuilder: (_, _) =>
                                            const Divider(height: 1),
                                        itemBuilder: (_, idx) {
                                          final e = cl.entries[idx];
                                          return ListTile(
                                            dense: true,
                                            leading: UserAvatar(
                                              name: e.userName,
                                              iconUrl: e.userIconUrl,
                                              radius: 16,
                                            ),
                                            title: Text(e.userName,
                                                style: const TextStyle(
                                                    fontWeight: FontWeight.bold,
                                                    fontSize: 13)),
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
    final p = context.palette;
    return Card(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(
          color: result != null ? p.warning : p.border,
          width: result != null ? 1.5 : 1,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.emoji_events_outlined, color: p.warning, size: 20),
                const SizedBox(width: 8),
                Text(title,
                    style: const TextStyle(
                        fontSize: 15, fontWeight: FontWeight.bold)),
                if (result != null) ...[
                  const Spacer(),
                  Text(
                    '母数: ${result!.candidateCount}件',
                    style: TextStyle(fontSize: 12, color: p.textSecondary),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 12),
            if (result == null || result!.winners.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 16),
                child: Center(
                  child: Text(
                    '抽選を実行すると、当選者のアイコンと名前が表示されます（OBS オーバーレイ連動）。',
                    style: TextStyle(color: p.textSecondary, fontSize: 13),
                  ),
                ),
              )
            else
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: result!.winners.map((w) {
                  return Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 14, vertical: 10),
                    decoration: BoxDecoration(
                      color: p.windowBg,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: p.warning.withValues(alpha: 0.7)),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        UserAvatar(
                          name: w.userName,
                          iconUrl: w.userIconUrl,
                          radius: 24,
                          borderColor: p.warning,
                        ),
                        const SizedBox(width: 12),
                        ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 320),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                w.userName,
                                style: TextStyle(
                                  fontSize: 18,
                                  fontWeight: FontWeight.bold,
                                  color: p.textPrimary,
                                ),
                              ),
                              if (w.commentText != null &&
                                  w.commentText!.isNotEmpty) ...[
                                const SizedBox(height: 3),
                                Text(
                                  w.commentText!,
                                  style: TextStyle(
                                      fontSize: 13, color: p.textSecondary),
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
    final p = context.palette;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('抽選履歴',
                style: TextStyle(fontSize: 14.5, fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            if (history.isEmpty)
              Text('まだ抽選履歴はありません。',
                  style: TextStyle(color: p.textSecondary))
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
                    label: Text(r.kind == 'comment' ? 'コメント' : 'ユーザー'),
                    visualDensity: VisualDensity.compact,
                  ),
                  title: Text(names,
                      style: const TextStyle(fontWeight: FontWeight.w600)),
                  trailing: Text(
                    timeStr,
                    style: TextStyle(fontSize: 12, color: p.textSecondary),
                  ),
                );
              }),
          ],
        ),
      ),
    );
  }
}
