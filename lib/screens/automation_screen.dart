import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';

/// OBS WebSocket v5 自動連動 ＆ グローバルホットキー / Stream Deck 連携画面
class AutomationScreen extends StatefulWidget {
  const AutomationScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<AutomationScreen> createState() => _AutomationScreenState();
}

class _AutomationScreenState extends State<AutomationScreen> {
  late TextEditingController _hostCtrl;
  late TextEditingController _portCtrl;
  late TextEditingController _passCtrl;
  bool _obscurePass = true;
  late bool _obsEnabled;
  late bool _autoConnect;
  late bool _resetFtOnStart;
  late bool _resetCntOnStart;
  late List<ObsAutomationRule> _rules;
  late ObsWsConfig _lastSyncedCfg;

  @override
  void initState() {
    super.initState();
    final cfg = widget.controller.obsWsConfig;
    _lastSyncedCfg = cfg;
    _hostCtrl = TextEditingController(text: cfg.host);
    _portCtrl = TextEditingController(text: cfg.port.toString());
    _passCtrl = TextEditingController(text: cfg.password);
    _obsEnabled = cfg.enabled;
    _autoConnect = cfg.autoConnect;
    _resetFtOnStart = cfg.resetFirstTimeOnStreamStart;
    _resetCntOnStart = cfg.resetCountersOnStreamStart;
    _rules = List<ObsAutomationRule>.from(cfg.rules);
  }

  @override
  void dispose() {
    _hostCtrl.dispose();
    _portCtrl.dispose();
    _passCtrl.dispose();
    super.dispose();
  }

  void _syncIfNeeded(ObsWsConfig latest) {
    if (latest == _lastSyncedCfg) return;
    _lastSyncedCfg = latest;
    _hostCtrl.text = latest.host;
    _portCtrl.text = latest.port.toString();
    _passCtrl.text = latest.password;
    _obsEnabled = latest.enabled;
    _autoConnect = latest.autoConnect;
    _resetFtOnStart = latest.resetFirstTimeOnStreamStart;
    _resetCntOnStart = latest.resetCountersOnStreamStart;
    _rules = List<ObsAutomationRule>.from(latest.rules);
  }

  Future<void> _saveObsConfig() async {
    final port = int.tryParse(_portCtrl.text.trim()) ?? 4455;
    final next = ObsWsConfig(
      enabled: _obsEnabled,
      host: _hostCtrl.text.trim().isEmpty ? '127.0.0.1' : _hostCtrl.text.trim(),
      port: port.clamp(1, 65535),
      password: _passCtrl.text,
      autoConnect: _autoConnect,
      resetFirstTimeOnStreamStart: _resetFtOnStart,
      resetCountersOnStreamStart: _resetCntOnStart,
      rules: List<ObsAutomationRule>.from(_rules),
    );
    _lastSyncedCfg = next;
    await widget.controller.saveObsWsConfig(next);
  }

  void _copyUrl(String url, String label) {
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$label 用トリガー URL をコピーしました')),
    );
  }

  String _triggerLabel(ObsTriggerType t) {
    switch (t) {
      case ObsTriggerType.superChatMinAmount:
        return 'スパチャ（金額指定）';
      case ObsTriggerType.membership:
        return 'メンバー加入/継続';
      case ObsTriggerType.keyword:
        return '特定ワード';
      case ObsTriggerType.firstTime:
        return '初見さん検知';
      case ObsTriggerType.lotteryWinner:
        return '抽選当選時';
      case ObsTriggerType.manual:
        return '手動 / テストのみ';
    }
  }

  String _obsActionLabel(ObsActionType a) {
    switch (a) {
      case ObsActionType.switchScene:
        return 'シーン切替';
      case ObsActionType.setSceneItemVisible:
        return 'ソース表示/非表示';
      case ObsActionType.toggleSceneItem:
        return 'ソース表示トグル';
    }
  }

  String _hotkeyActionLabel(AutomationActionKind a) {
    switch (a) {
      case AutomationActionKind.clearPickup:
        return 'スポットテロップ解除';
      case AutomationActionKind.pickLatestComment:
        return '最新コメントをスポット表示';
      case AutomationActionKind.triggerMediaRule:
        return 'リアクションステージ発動';
      case AutomationActionKind.clearMedia:
        return 'リアクション全消去';
      case AutomationActionKind.endPoll:
        return 'ライブアンケート締切';
      case AutomationActionKind.advanceQueue:
        return 'エントリーレーン繰り上げ';
      case AutomationActionKind.resetFirstTime:
        return 'ウェルカムカウンターリセット';
      case AutomationActionKind.toggleFeature:
        return '機能の ON/OFF 切替';
      case AutomationActionKind.obsSwitchScene:
        return 'OBS シーン切替';
      case AutomationActionKind.obsToggleSource:
        return 'OBS ソース表示トグル';
    }
  }

  String _formatShortcut(HotkeyBinding b) {
    final parts = <String>[];
    if (b.ctrl) parts.add('Ctrl');
    if (b.alt) parts.add('Alt');
    if (b.shift) parts.add('Shift');
    if (b.key.isNotEmpty) parts.add(b.key);
    return parts.isEmpty ? 'キー未設定（Stream Deck 専用）' : parts.join(' + ');
  }

  Future<void> _openAddObsRuleDialog() async {
    final status = widget.controller.obsWsStatus;
    final nameCtrl = TextEditingController();
    final patCtrl = TextEditingController();
    final amtCtrl = TextEditingController(text: '1000');
    final sceneCtrl = TextEditingController(
      text: status.scenes.isNotEmpty ? status.scenes.first : '',
    );
    final sourceCtrl = TextEditingController();
    final revertCtrl = TextEditingController(text: '5000');
    var trigger = ObsTriggerType.superChatMinAmount;
    var action = ObsActionType.setSceneItemVisible;
    var setVisible = true;

    final saved = await showDialog<ObsAutomationRule>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDlg) {
          final sources = status.sceneSources[sceneCtrl.text] ?? const [];
          return AlertDialog(
            title: const Text('OBS 自動連動ルールを追加'),
            content: SizedBox(
              width: 520,
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    TextField(
                      controller: nameCtrl,
                      decoration: const InputDecoration(
                        labelText: 'ルール名',
                        hintText: '例: 高額スパチャ時に感謝フレームを5秒表示',
                      ),
                    ),
                    const SizedBox(height: 10),
                    DropdownButtonFormField<ObsTriggerType>(
                      initialValue: trigger,
                      decoration: const InputDecoration(labelText: '発火トリガー'),
                      items: ObsTriggerType.values
                          .map((t) => DropdownMenuItem(
                              value: t, child: Text(_triggerLabel(t))))
                          .toList(),
                      onChanged: (v) {
                        if (v != null) setDlg(() => trigger = v);
                      },
                    ),
                    const SizedBox(height: 10),
                    if (trigger == ObsTriggerType.superChatMinAmount)
                      TextField(
                        controller: amtCtrl,
                        keyboardType: TextInputType.number,
                        decoration:
                            const InputDecoration(labelText: '最小金額 (円相当)'),
                      ),
                    if (trigger == ObsTriggerType.keyword)
                      TextField(
                        controller: patCtrl,
                        decoration:
                            const InputDecoration(labelText: '反応するキーワード'),
                      ),
                    const SizedBox(height: 10),
                    DropdownButtonFormField<ObsActionType>(
                      initialValue: action,
                      decoration: const InputDecoration(labelText: 'OBS アクション'),
                      items: ObsActionType.values
                          .map((a) => DropdownMenuItem(
                              value: a, child: Text(_obsActionLabel(a))))
                          .toList(),
                      onChanged: (v) {
                        if (v != null) setDlg(() => action = v);
                      },
                    ),
                    const SizedBox(height: 10),
                    if (status.scenes.isNotEmpty)
                      DropdownButtonFormField<String>(
                        initialValue: status.scenes.contains(sceneCtrl.text)
                            ? sceneCtrl.text
                            : status.scenes.first,
                        decoration:
                            const InputDecoration(labelText: '対象シーン（OBSから取得）'),
                        items: status.scenes
                            .map((s) =>
                                DropdownMenuItem(value: s, child: Text(s)))
                            .toList(),
                        onChanged: (v) {
                          if (v != null) {
                            setDlg(() => sceneCtrl.text = v);
                          }
                        },
                      )
                    else
                      TextField(
                        controller: sceneCtrl,
                        decoration: const InputDecoration(
                          labelText: '対象シーン名',
                          hintText: 'OBS に接続すると候補が表示されます',
                        ),
                      ),
                    if (action != ObsActionType.switchScene) ...[
                      const SizedBox(height: 10),
                      if (sources.isNotEmpty)
                        DropdownButtonFormField<String>(
                          initialValue: sources.contains(sourceCtrl.text)
                              ? sourceCtrl.text
                              : sources.first,
                          decoration: const InputDecoration(
                              labelText: '対象ソース（OBSから取得）'),
                          items: sources
                              .map((s) =>
                                  DropdownMenuItem(value: s, child: Text(s)))
                              .toList(),
                          onChanged: (v) {
                            if (v != null) {
                              setDlg(() => sourceCtrl.text = v);
                            }
                          },
                        )
                      else
                        TextField(
                          controller: sourceCtrl,
                          decoration: const InputDecoration(
                            labelText: '対象ソース名',
                            hintText: '例: ありがとう画像',
                          ),
                        ),
                      if (action == ObsActionType.setSceneItemVisible)
                        SwitchListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          title: const Text('表示状態にする（OFF で非表示）'),
                          value: setVisible,
                          onChanged: (v) => setDlg(() => setVisible = v),
                        ),
                      const SizedBox(height: 8),
                      TextField(
                        controller: revertCtrl,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(
                          labelText: '自動復帰までの時間 (ミリ秒 / 0で復帰しない)',
                          hintText: '例: 5000 (5秒後に元へ戻す)',
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx),
                child: const Text('キャンセル'),
              ),
              FilledButton(
                onPressed: () {
                  final scene = sceneCtrl.text.trim();
                  final source = sourceCtrl.text.trim().isEmpty &&
                          sources.isNotEmpty
                      ? sources.first
                      : sourceCtrl.text.trim();
                  Navigator.pop(
                    ctx,
                    ObsAutomationRule(
                      id: 'obs-${DateTime.now().microsecondsSinceEpoch}',
                      name: nameCtrl.text.trim().isEmpty
                          ? '${_triggerLabel(trigger)} → ${_obsActionLabel(action)}'
                          : nameCtrl.text.trim(),
                      enabled: true,
                      triggerType: trigger,
                      pattern: patCtrl.text.trim(),
                      minAmountJpy:
                          BigInt.from(int.tryParse(amtCtrl.text.trim()) ?? 0),
                      actionType: action,
                      targetScene: scene,
                      targetSource: source,
                      setVisible: setVisible,
                      autoRevertMs:
                          (int.tryParse(revertCtrl.text.trim()) ?? 0)
                              .clamp(0, 600000),
                    ),
                  );
                },
                child: const Text('追加'),
              ),
            ],
          );
        },
      ),
    );
    nameCtrl.dispose();
    patCtrl.dispose();
    amtCtrl.dispose();
    sceneCtrl.dispose();
    sourceCtrl.dispose();
    revertCtrl.dispose();

    if (saved != null) {
      setState(() => _rules.add(saved));
      await _saveObsConfig();
    }
  }

  Future<void> _openAddHotkeyDialog() async {
    final ctrl = widget.controller;
    final nameCtrl = TextEditingController();
    final targetCtrl = TextEditingController();
    final secondaryCtrl = TextEditingController();
    var ctrlKey = true;
    var altKey = true;
    var shiftKey = false;
    var keyStr = 'F9';
    var action = AutomationActionKind.clearPickup;

    const keyChoices = [
      '',
      'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12',
      '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
      'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
      'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
      'SPACE', 'ESCAPE',
    ];

    final added = await showDialog<HotkeyBinding>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDlg) => AlertDialog(
          title: const Text('ホットキー / Stream Deck アクションを追加'),
          content: SizedBox(
            width: 520,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  TextField(
                    controller: nameCtrl,
                    decoration: const InputDecoration(
                      labelText: 'アクション名',
                      hintText: '例: テロップ即解除',
                    ),
                  ),
                  const SizedBox(height: 10),
                  DropdownButtonFormField<AutomationActionKind>(
                    initialValue: action,
                    decoration: const InputDecoration(labelText: '実行するアクション'),
                    items: AutomationActionKind.values
                        .map((a) => DropdownMenuItem(
                            value: a, child: Text(_hotkeyActionLabel(a))))
                        .toList(),
                    onChanged: (v) {
                      if (v != null) setDlg(() => action = v);
                    },
                  ),
                  const SizedBox(height: 10),
                  if (action == AutomationActionKind.triggerMediaRule)
                    DropdownButtonFormField<String>(
                      initialValue: ctrl.mediaReactionRules
                              .any((r) => r.id == targetCtrl.text)
                          ? targetCtrl.text
                          : (ctrl.mediaReactionRules.isNotEmpty
                              ? ctrl.mediaReactionRules.first.id
                              : null),
                      decoration:
                          const InputDecoration(labelText: '発動するリアクション演出'),
                      items: ctrl.mediaReactionRules
                          .map((r) => DropdownMenuItem(
                              value: r.id, child: Text(r.name)))
                          .toList(),
                      onChanged: (v) {
                        if (v != null) setDlg(() => targetCtrl.text = v);
                      },
                    ),
                  if (action == AutomationActionKind.toggleFeature)
                    DropdownButtonFormField<String>(
                      initialValue: targetCtrl.text.isEmpty
                          ? 'comment_display'
                          : targetCtrl.text,
                      decoration: const InputDecoration(labelText: '切り替える機能'),
                      items: const [
                        DropdownMenuItem(
                            value: 'comment_display', child: Text('トークモニター')),
                        DropdownMenuItem(
                            value: 'pickup', child: Text('スポットテロップ')),
                        DropdownMenuItem(
                            value: 'media_reaction', child: Text('リアクションステージ')),
                        DropdownMenuItem(
                            value: 'ng_word_filter', child: Text('NGワード防御')),
                        DropdownMenuItem(
                            value: 'obs_overlay', child: Text('オーバーレイスタジオ')),
                      ],
                      onChanged: (v) {
                        if (v != null) setDlg(() => targetCtrl.text = v);
                      },
                    ),
                  if (action == AutomationActionKind.obsSwitchScene ||
                      action == AutomationActionKind.obsToggleSource) ...[
                    TextField(
                      controller: targetCtrl,
                      decoration: const InputDecoration(labelText: 'OBS シーン名'),
                    ),
                    if (action == AutomationActionKind.obsToggleSource) ...[
                      const SizedBox(height: 8),
                      TextField(
                        controller: secondaryCtrl,
                        decoration:
                            const InputDecoration(labelText: 'OBS ソース名'),
                      ),
                    ],
                  ],
                  const SizedBox(height: 12),
                  const Text('グローバルショートカットキー（空欄なら Stream Deck 専用）',
                      style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold)),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 8,
                    runSpacing: 6,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      FilterChip(
                        label: const Text('Ctrl'),
                        selected: ctrlKey,
                        onSelected: (v) => setDlg(() => ctrlKey = v),
                      ),
                      FilterChip(
                        label: const Text('Alt'),
                        selected: altKey,
                        onSelected: (v) => setDlg(() => altKey = v),
                      ),
                      FilterChip(
                        label: const Text('Shift'),
                        selected: shiftKey,
                        onSelected: (v) => setDlg(() => shiftKey = v),
                      ),
                      SizedBox(
                        width: 140,
                        child: DropdownButtonFormField<String>(
                          initialValue: keyStr,
                          decoration: const InputDecoration(
                              isDense: true, labelText: 'キー'),
                          items: keyChoices
                              .map((k) => DropdownMenuItem(
                                    value: k,
                                    child: Text(k.isEmpty ? 'なし' : k),
                                  ))
                              .toList(),
                          onChanged: (v) {
                            if (v != null) setDlg(() => keyStr = v);
                          },
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('キャンセル'),
            ),
            FilledButton(
              onPressed: () {
                var tid = targetCtrl.text.trim();
                if (action == AutomationActionKind.triggerMediaRule &&
                    tid.isEmpty &&
                    ctrl.mediaReactionRules.isNotEmpty) {
                  tid = ctrl.mediaReactionRules.first.id;
                }
                if (action == AutomationActionKind.toggleFeature &&
                    tid.isEmpty) {
                  tid = 'comment_display';
                }
                Navigator.pop(
                  ctx,
                  HotkeyBinding(
                    id: 'hk-${DateTime.now().microsecondsSinceEpoch}',
                    name: nameCtrl.text.trim().isEmpty
                        ? _hotkeyActionLabel(action)
                        : nameCtrl.text.trim(),
                    enabled: true,
                    ctrl: ctrlKey,
                    alt: altKey,
                    shift: shiftKey,
                    key: keyStr,
                    action: action,
                    targetId: tid,
                    secondaryTarget: secondaryCtrl.text.trim(),
                  ),
                );
              },
              child: const Text('追加'),
            ),
          ],
        ),
      ),
    );
    nameCtrl.dispose();
    targetCtrl.dispose();
    secondaryCtrl.dispose();

    if (added != null) {
      final next = [...ctrl.hotkeyBindings, added];
      await ctrl.saveHotkeyBindings(next);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    _syncIfNeeded(ctrl.obsWsConfig);
    final obsSt = ctrl.obsWsStatus;
    final baseUrl = ctrl.overlayBaseUrl;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '連携・オートメーション（OBS WebSocket / ホットキー / Stream Deck）',
            style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 4),
          Text(
            'OBS Studio 本体の自動制御や、ゲーム中でも操作できるグローバルホットキー・Stream Deck 連携を一元管理します。',
            style: TextStyle(fontSize: 13, color: p.textSecondary),
          ),
          const SizedBox(height: 16),

          // 1. OBS WebSocket v5 接続カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.cable_outlined, color: p.accent, size: 20),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'OBS WebSocket v5 連携',
                          style: TextStyle(
                              fontSize: 15.5, fontWeight: FontWeight.bold),
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: (obsSt.connected ? p.success : p.textMuted)
                              .withValues(alpha: 0.14),
                          borderRadius: BorderRadius.circular(5),
                          border: Border.all(
                            color: obsSt.connected ? p.success : p.border,
                          ),
                        ),
                        child: Text(
                          obsSt.connected
                              ? '接続中 (${obsSt.obsVersion}) • シーン: ${obsSt.currentScene}'
                              : (obsSt.connecting ? '接続中...' : '未接続'),
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.bold,
                            color:
                                obsSt.connected ? p.success : p.textSecondary,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    'OBS Studio メニューの「ツール」→「WebSocket サーバー設定」でサーバーを有効にし、ポート番号とパスワードを入力してください。',
                    style: TextStyle(fontSize: 12, color: p.textSecondary),
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      SizedBox(
                        width: 170,
                        child: TextField(
                          controller: _hostCtrl,
                          decoration: const InputDecoration(
                            isDense: true,
                            labelText: 'ホスト',
                          ),
                        ),
                      ),
                      SizedBox(
                        width: 105,
                        child: TextField(
                          controller: _portCtrl,
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            isDense: true,
                            labelText: 'ポート',
                          ),
                        ),
                      ),
                      SizedBox(
                        width: 220,
                        child: TextField(
                          controller: _passCtrl,
                          obscureText: _obscurePass,
                          decoration: InputDecoration(
                            isDense: true,
                            labelText: 'パスワード（任意）',
                            suffixIcon: IconButton(
                              icon: Icon(
                                _obscurePass
                                    ? Icons.visibility_outlined
                                    : Icons.visibility_off_outlined,
                                size: 17,
                              ),
                              onPressed: () =>
                                  setState(() => _obscurePass = !_obscurePass),
                            ),
                          ),
                        ),
                      ),
                      FilledButton.icon(
                        onPressed: () async {
                          setState(() => _obsEnabled = true);
                          await _saveObsConfig();
                          await ctrl.connectObsWs();
                        },
                        icon: const Icon(Icons.link, size: 16),
                        label: const Text('保存して接続'),
                      ),
                      if (obsSt.connected) ...[
                        OutlinedButton.icon(
                          onPressed: ctrl.refreshObsScenes,
                          icon: const Icon(Icons.refresh, size: 16),
                          label: const Text('シーン再取得'),
                        ),
                        OutlinedButton.icon(
                          onPressed: ctrl.disconnectObsWs,
                          icon: const Icon(Icons.link_off, size: 16),
                          label: const Text('切断'),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 12,
                    runSpacing: 2,
                    children: [
                      SizedBox(
                        width: 230,
                        child: CheckboxListTile(
                          dense: true,
                          controlAffinity: ListTileControlAffinity.leading,
                          contentPadding: EdgeInsets.zero,
                          title: const Text('起動時に OBS へ自動接続',
                              style: TextStyle(fontSize: 12.5)),
                          value: _autoConnect,
                          onChanged: (v) async {
                            setState(() => _autoConnect = v ?? false);
                            await _saveObsConfig();
                          },
                        ),
                      ),
                      SizedBox(
                        width: 360,
                        child: CheckboxListTile(
                          dense: true,
                          controlAffinity: ListTileControlAffinity.leading,
                          contentPadding: EdgeInsets.zero,
                          title: const Text('OBS 配信開始時にウェルカムカウンター自動リセット',
                              style: TextStyle(fontSize: 12.5)),
                          value: _resetFtOnStart,
                          onChanged: (v) async {
                            setState(() => _resetFtOnStart = v ?? false);
                            await _saveObsConfig();
                          },
                        ),
                      ),
                      SizedBox(
                        width: 350,
                        child: CheckboxListTile(
                          dense: true,
                          controlAffinity: ListTileControlAffinity.leading,
                          contentPadding: EdgeInsets.zero,
                          title: const Text('OBS 配信開始時に全コールカウンター自動リセット',
                              style: TextStyle(fontSize: 12.5)),
                          value: _resetCntOnStart,
                          onChanged: (v) async {
                            setState(() => _resetCntOnStart = v ?? false);
                            await _saveObsConfig();
                          },
                        ),
                      ),
                    ],
                  ),
                  const Divider(height: 22),
                  Row(
                    children: [
                      const Expanded(
                        child: Text(
                          'OBS 自動連動ルール（スパチャ / ワード / 抽選 → シーン・ソース自動切替）',
                          style: TextStyle(
                              fontWeight: FontWeight.bold, fontSize: 13.5),
                        ),
                      ),
                      FilledButton.tonalIcon(
                        onPressed: _openAddObsRuleDialog,
                        icon: const Icon(Icons.add, size: 16),
                        label: const Text('ルールを追加'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  if (_rules.isEmpty)
                    Text(
                      '自動連動ルールはまだ登録されていません。',
                      style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                    )
                  else
                    ..._rules.asMap().entries.map((entry) {
                      final i = entry.key;
                      final r = entry.value;
                      return ListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        leading: Switch(
                          value: r.enabled,
                          onChanged: (v) async {
                            setState(() {
                              _rules[i] = ObsAutomationRule(
                                id: r.id,
                                name: r.name,
                                enabled: v,
                                triggerType: r.triggerType,
                                pattern: r.pattern,
                                minAmountJpy: r.minAmountJpy,
                                actionType: r.actionType,
                                targetScene: r.targetScene,
                                targetSource: r.targetSource,
                                setVisible: r.setVisible,
                                autoRevertMs: r.autoRevertMs,
                              );
                            });
                            await _saveObsConfig();
                          },
                        ),
                        title: Text(r.name,
                            style: const TextStyle(fontWeight: FontWeight.bold)),
                        subtitle: Text(
                          '${_triggerLabel(r.triggerType)} → ${_obsActionLabel(r.actionType)} (シーン: ${r.targetScene}${r.targetSource.isNotEmpty ? " / ソース: ${r.targetSource}" : ""}${r.autoRevertMs > 0 ? " / ${(r.autoRevertMs / 1000).toStringAsFixed(1)}秒後自動復帰" : ""})',
                          style:
                              TextStyle(fontSize: 11.5, color: p.textSecondary),
                        ),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            OutlinedButton(
                              onPressed: obsSt.connected
                                  ? () => ctrl.testObsRule(r.id)
                                  : null,
                              child: const Text('テスト'),
                            ),
                            IconButton(
                              icon: Icon(Icons.delete_outline,
                                  color: p.danger, size: 19),
                              onPressed: () async {
                                setState(() => _rules.removeAt(i));
                                await _saveObsConfig();
                              },
                            ),
                          ],
                        ),
                      );
                    }),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // 2. グローバルホットキー ＆ Stream Deck HTTP トリガー
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.keyboard_outlined, color: p.accent, size: 20),
                      const SizedBox(width: 8),
                      const Expanded(
                        child: Text(
                          'グローバルホットキー ＆ Stream Deck トリガー',
                          style: TextStyle(
                              fontSize: 15.5, fontWeight: FontWeight.bold),
                        ),
                      ),
                      FilledButton.icon(
                        onPressed: _openAddHotkeyDialog,
                        icon: const Icon(Icons.add, size: 16),
                        label: const Text('アクションを追加'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    'ゲーム等をフルスクリーン表示中でも、登録したショートカットキーや Stream Deck の「Web サイト（バックグラウンドで開く）」から Stosup を一発操作できます。',
                    style: TextStyle(fontSize: 12, color: p.textSecondary),
                  ),
                  const SizedBox(height: 10),
                  if (ctrl.hotkeyBindings.isEmpty)
                    Text(
                      'ホットキーはまだ登録されていません。「アクションを追加」から作成してください。',
                      style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                    )
                  else
                    ...ctrl.hotkeyBindings.asMap().entries.map((entry) {
                      final idx = entry.key;
                      final b = entry.value;
                      final triggerUrl =
                          '$baseUrl/api/trigger/hotkey?id=${Uri.encodeComponent(b.id)}';
                      return ListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        leading: Switch(
                          value: b.enabled,
                          onChanged: (v) {
                            final next =
                                List<HotkeyBinding>.from(ctrl.hotkeyBindings);
                            next[idx] = HotkeyBinding(
                              id: b.id,
                              name: b.name,
                              enabled: v,
                              ctrl: b.ctrl,
                              alt: b.alt,
                              shift: b.shift,
                              key: b.key,
                              action: b.action,
                              targetId: b.targetId,
                              secondaryTarget: b.secondaryTarget,
                            );
                            ctrl.saveHotkeyBindings(next);
                          },
                        ),
                        title: Row(
                          children: [
                            Text(b.name,
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold)),
                            const SizedBox(width: 8),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 7, vertical: 2),
                              decoration: BoxDecoration(
                                color: p.subtleBg,
                                borderRadius: BorderRadius.circular(4),
                                border: Border.all(color: p.border),
                              ),
                              child: Text(
                                _formatShortcut(b),
                                style: const TextStyle(
                                    fontFamily: 'Consolas', fontSize: 11),
                              ),
                            ),
                          ],
                        ),
                        subtitle: Text(
                          '${_hotkeyActionLabel(b.action)}${b.targetId.isNotEmpty ? " (${b.targetId})" : ""}',
                          style:
                              TextStyle(fontSize: 11.5, color: p.textSecondary),
                        ),
                        trailing: Wrap(
                          spacing: 6,
                          children: [
                            OutlinedButton.icon(
                              style: OutlinedButton.styleFrom(
                                  visualDensity: VisualDensity.compact),
                              onPressed: () =>
                                  _copyUrl(triggerUrl, 'Stream Deck'),
                              icon: const Icon(Icons.copy, size: 14),
                              label: const Text('Stream Deck URL',
                                  style: TextStyle(fontSize: 11.5)),
                            ),
                            OutlinedButton(
                              style: OutlinedButton.styleFrom(
                                  visualDensity: VisualDensity.compact),
                              onPressed: () => ctrl.executeAutomationAction(
                                b.action,
                                targetId: b.targetId,
                                secondaryTarget: b.secondaryTarget,
                              ),
                              child: const Text('実行',
                                  style: TextStyle(fontSize: 11.5)),
                            ),
                            IconButton(
                              icon: Icon(Icons.delete_outline,
                                  color: p.danger, size: 19),
                              onPressed: () {
                                final next = List<HotkeyBinding>.from(
                                    ctrl.hotkeyBindings)
                                  ..removeAt(idx);
                                ctrl.saveHotkeyBindings(next);
                              },
                            ),
                          ],
                        ),
                      );
                    }),
                  const Divider(height: 20),
                  const Text('定番 Stream Deck 用クイック URL（そのまま貼って使えます）',
                      style: TextStyle(
                          fontWeight: FontWeight.bold, fontSize: 12.5)),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 8,
                    runSpacing: 6,
                    children: [
                      _quickTriggerBtn('スポットテロップ解除',
                          '$baseUrl/api/trigger/clear_pickup'),
                      _quickTriggerBtn('最新コメントをピン留め',
                          '$baseUrl/api/trigger/pick_latest'),
                      _quickTriggerBtn(
                          'リアクション全消去', '$baseUrl/api/trigger/clear_media'),
                      _quickTriggerBtn(
                          'ライブアンケート締切', '$baseUrl/api/trigger/end_poll'),
                      _quickTriggerBtn('エントリーレーン繰り上げ',
                          '$baseUrl/api/trigger/advance_queue'),
                      _quickTriggerBtn('初見カウントリセット',
                          '$baseUrl/api/trigger/reset_first_time'),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _quickTriggerBtn(String label, String url) {
    return OutlinedButton.icon(
      style: OutlinedButton.styleFrom(visualDensity: VisualDensity.compact),
      onPressed: () => _copyUrl(url, label),
      icon: const Icon(Icons.link, size: 14),
      label: Text(label, style: const TextStyle(fontSize: 11.5)),
    );
  }
}
