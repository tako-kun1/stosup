import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/overlay_bg_picker.dart';

/// リアクションステージ（特定ワード反応アニメーション・動画演出）エディタ画面
/// 固定表示・クラッカー・落下物・紙吹雪・投てき・環境演出・効果音(SE)・イベント連動に対応
class MediaReactionScreen extends StatefulWidget {
  const MediaReactionScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<MediaReactionScreen> createState() => _MediaReactionScreenState();
}

class _MediaReactionScreenState extends State<MediaReactionScreen> {
  late MediaPlaybackMode _playbackMode;
  late int _masterVolume;
  late int _maxConcurrent;
  late bool _allowMultipleMatch;
  late MediaReactionConfig _lastSyncedConfig;

  @override
  void initState() {
    super.initState();
    _syncConfigFromController(widget.controller.mediaReactionConfig);
  }

  void _syncConfigFromController(MediaReactionConfig cfg) {
    _lastSyncedConfig = cfg;
    _playbackMode = cfg.playbackMode;
    _masterVolume = cfg.masterVolume;
    _maxConcurrent = cfg.maxConcurrent;
    _allowMultipleMatch = cfg.allowMultipleMatch;
  }

  static String effectLabel(MediaEffectType t) {
    switch (t) {
      case MediaEffectType.fixed:
        return '固定表示';
      case MediaEffectType.cracker:
        return 'クラッカー';
      case MediaEffectType.drop:
        return '落下物';
      case MediaEffectType.confetti:
        return '紙吹雪';
      case MediaEffectType.throw_:
        return '投てき';
      case MediaEffectType.ambient:
        return '環境演出';
      case MediaEffectType.mod:
        return 'カスタム MOD';
    }
  }

  static IconData effectIcon(MediaEffectType t) {
    switch (t) {
      case MediaEffectType.fixed:
        return Icons.smart_display_outlined;
      case MediaEffectType.cracker:
        return Icons.celebration_outlined;
      case MediaEffectType.drop:
        return Icons.arrow_downward_rounded;
      case MediaEffectType.confetti:
        return Icons.auto_awesome;
      case MediaEffectType.throw_:
        return Icons.sports_baseball_outlined;
      case MediaEffectType.ambient:
        return Icons.ac_unit;
      case MediaEffectType.mod:
        return Icons.extension_outlined;
    }
  }

  static String triggerLabel(MediaTriggerType t, BigInt minAmount) {
    switch (t) {
      case MediaTriggerType.keyword:
        return 'キーワード';
      case MediaTriggerType.anyComment:
        return '全コメント';
      case MediaTriggerType.superChat:
        return minAmount > BigInt.zero ? 'スパチャ ¥$minAmount〜' : 'スパチャ全般';
      case MediaTriggerType.membership:
        return 'メンバーシップ';
      case MediaTriggerType.firstTime:
        return '初見さん';
    }
  }

  static String ambientLabel(String preset) {
    switch (preset) {
      case 'sakura':
        return '桜吹雪';
      case 'rain':
        return '雨';
      case 'sparkle':
        return 'キラキラ';
      case 'hearts':
        return 'ハート';
      case 'embers':
        return '火の粉';
      default:
        return '雪';
    }
  }

  void _copyOverlayUrl() {
    final url = '${widget.controller.overlayBaseUrl}/overlay/media';
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('リアクションステージの OBS 用 URL をコピーしました')),
    );
  }

  Future<void> _openEditor({MediaReactionRule? initial}) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _ReactionStageRuleEditorDialog(
        controller: widget.controller,
        initial: initial,
      ),
    );
  }

  Future<void> _addQuickPreset(String key) async {
    late MediaReactionRule rule;
    switch (key) {
      case 'cracker':
        rule = MediaReactionRule(
          id: '',
          name: 'おめでとうクラッカー',
          pattern: 'おめでとう\nおめ\n祝',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.cracker,
          triggerType: MediaTriggerType.keyword,
          minSuperchatAmount: BigInt.zero,
          priority: 20,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 85,
          animation: MediaAnimationKind.popIn,
          position: MediaPosition.center,
          durationMs: 4000,
          widthPercent: 45,
          volume: 80,
          particleCount: 36,
          particleSize: 56,
          particleSpeed: 105,
          gravity: 100,
          particleText: '🎉,🎊,✨',
          ambientPreset: 'snow',
          cooldownSec: 2,
          excludeBanned: true,
          captionTemplate: '',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
      case 'kusa':
        rule = MediaReactionRule(
          id: '',
          name: '草シャワー（落下物）',
          pattern: '草\nwww\nワロタ',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.drop,
          triggerType: MediaTriggerType.keyword,
          minSuperchatAmount: BigInt.zero,
          priority: 10,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 80,
          animation: MediaAnimationKind.fade,
          position: MediaPosition.center,
          durationMs: 4500,
          widthPercent: 45,
          volume: 80,
          particleCount: 24,
          particleSize: 60,
          particleSpeed: 100,
          gravity: 100,
          particleText: '草,🌿,w',
          ambientPreset: 'snow',
          cooldownSec: 2,
          excludeBanned: true,
          captionTemplate: '',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
      case 'confetti':
        rule = MediaReactionRule(
          id: '',
          name: '888 紙吹雪',
          pattern: '888\nパチパチ\n👏',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.confetti,
          triggerType: MediaTriggerType.keyword,
          minSuperchatAmount: BigInt.zero,
          priority: 15,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 80,
          animation: MediaAnimationKind.fade,
          position: MediaPosition.center,
          durationMs: 5000,
          widthPercent: 45,
          volume: 80,
          particleCount: 70,
          particleSize: 28,
          particleSpeed: 100,
          gravity: 90,
          particleText: '',
          ambientPreset: 'snow',
          cooldownSec: 3,
          excludeBanned: true,
          captionTemplate: '',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
      case 'throw':
        rule = MediaReactionRule(
          id: '',
          name: 'ナイス投てき',
          pattern: 'ナイス\n神プレイ\nすご',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.throw_,
          triggerType: MediaTriggerType.keyword,
          minSuperchatAmount: BigInt.zero,
          priority: 12,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 80,
          animation: MediaAnimationKind.popIn,
          position: MediaPosition.center,
          durationMs: 3200,
          widthPercent: 45,
          volume: 80,
          particleCount: 6,
          particleSize: 88,
          particleSpeed: 105,
          gravity: 100,
          particleText: '🎯,⭐,👍',
          ambientPreset: 'snow',
          cooldownSec: 2,
          excludeBanned: true,
          captionTemplate: '',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
      case 'superchat_sakura':
        rule = MediaReactionRule(
          id: '',
          name: 'スパチャ桜吹雪',
          pattern: '',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.ambient,
          triggerType: MediaTriggerType.superChat,
          minSuperchatAmount: BigInt.zero,
          priority: 30,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 85,
          animation: MediaAnimationKind.popIn,
          position: MediaPosition.topRight,
          durationMs: 6500,
          widthPercent: 45,
          volume: 80,
          particleCount: 40,
          particleSize: 32,
          particleSpeed: 100,
          gravity: 100,
          particleText: '',
          ambientPreset: 'sakura',
          cooldownSec: 1,
          excludeBanned: true,
          captionTemplate: '{user} さんスパチャありがとう！',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
      default:
        rule = MediaReactionRule(
          id: '',
          name: '雪の環境演出',
          pattern: '雪\n寒い\n冬',
          matchMode: MatchMode.contains,
          caseSensitive: false,
          effectType: MediaEffectType.ambient,
          triggerType: MediaTriggerType.keyword,
          minSuperchatAmount: BigInt.zero,
          priority: 5,
          mediaKind: MediaKind.image,
          storedFileName: '',
          originalFileName: '',
          soundStoredFileName: '',
          soundOriginalFileName: '',
          soundVolume: 80,
          animation: MediaAnimationKind.fade,
          position: MediaPosition.center,
          durationMs: 6500,
          widthPercent: 45,
          volume: 80,
          particleCount: 36,
          particleSize: 28,
          particleSpeed: 95,
          gravity: 100,
          particleText: '',
          ambientPreset: 'snow',
          cooldownSec: 5,
          excludeBanned: true,
          captionTemplate: '',
          enabled: true,
          modId: '',
          modEffectId: '',
          modParamsJson: '{}',
        );
    }
    await widget.controller.saveMediaReactionRule(rule);
  }

  MediaReactionRule _copyRule(
    MediaReactionRule r, {
    bool? enabled,
    int? priority,
  }) {
    return MediaReactionRule(
      id: r.id,
      name: r.name,
      pattern: r.pattern,
      matchMode: r.matchMode,
      caseSensitive: r.caseSensitive,
      effectType: r.effectType,
      triggerType: r.triggerType,
      minSuperchatAmount: r.minSuperchatAmount,
      priority: priority ?? r.priority,
      mediaKind: r.mediaKind,
      storedFileName: r.storedFileName,
      originalFileName: r.originalFileName,
      soundStoredFileName: r.soundStoredFileName,
      soundOriginalFileName: r.soundOriginalFileName,
      soundVolume: r.soundVolume,
      animation: r.animation,
      position: r.position,
      durationMs: r.durationMs,
      widthPercent: r.widthPercent,
      volume: r.volume,
      particleCount: r.particleCount,
      particleSize: r.particleSize,
      particleSpeed: r.particleSpeed,
      gravity: r.gravity,
      particleText: r.particleText,
      ambientPreset: r.ambientPreset,
      cooldownSec: r.cooldownSec,
      excludeBanned: r.excludeBanned,
      captionTemplate: r.captionTemplate,
      enabled: enabled ?? r.enabled,
      modId: r.modId,
      modEffectId: r.modEffectId,
      modParamsJson: r.modParamsJson,
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    if (ctrl.mediaReactionConfig != _lastSyncedConfig) {
      _syncConfigFromController(ctrl.mediaReactionConfig);
    }
    final enabled = ctrl.featureFlags.mediaReactionEnabled;
    final rules = ctrl.mediaReactionRules;
    final overlayUrl = '${ctrl.overlayBaseUrl}/overlay/media';

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'リアクションステージ（コメント連動アニメーション・動画演出）',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      'コメント・スパチャ・メンバー・初見に反応して、固定表示（画像/GIF/透過WebM/MP4）・クラッカー・落下物・紙吹雪・投てき・環境演出・効果音を再生します。',
                      style: TextStyle(color: p.textSecondary, fontSize: 13),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              FeatureToggleSwitch(
                label: 'リアクションステージ',
                enabled: enabled,
                onChanged: (v) => ctrl.setFeatureEnabled(
                  mediaReaction: v,
                  featureLabel: 'リアクションステージ',
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              FilledButton.icon(
                onPressed: () => _openEditor(),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('演出を追加'),
              ),
              OutlinedButton.icon(
                onPressed: _copyOverlayUrl,
                icon: const Icon(Icons.copy, size: 15),
                label: const Text('OBS URL'),
              ),
              OutlinedButton.icon(
                onPressed: ctrl.featureFlags.obsOverlayEnabled
                    ? () => ctrl.openUrl(overlayUrl)
                    : null,
                icon: const Icon(Icons.open_in_new, size: 15),
                label: const Text('ブラウザで確認'),
              ),
              OutlinedButton.icon(
                onPressed: ctrl.clearOverlayMedia,
                icon: const Icon(Icons.stop_circle_outlined, size: 16),
                label: const Text('全停止'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (!enabled)
            FeatureDisabledBanner(
              featureName: 'リアクションステージ',
              description:
                  'コメントやイベントによるパーティクル・画像・動画・効果音の自動発動を一時停止しています。',
              onEnable: () => ctrl.setFeatureEnabled(
                mediaReaction: true,
                featureLabel: 'リアクションステージ',
              ),
            ),

          // 全体再生方式・複数判定・マスター音量・背景色設定カード
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Expanded(
                        child: Text(
                          '全体設定（固定表示の再生方式・複数判定・マスター音量）',
                          style: TextStyle(
                              fontWeight: FontWeight.bold, fontSize: 14),
                        ),
                      ),
                      const SizedBox(width: 10),
                      FilledButton.icon(
                        onPressed: () => ctrl.saveMediaReactionConfig(
                          MediaReactionConfig(
                            playbackMode: _playbackMode,
                            masterVolume: _masterVolume,
                            maxConcurrent: _maxConcurrent,
                            allowMultipleMatch: _allowMultipleMatch,
                          ),
                        ),
                        icon: const Icon(Icons.save_outlined, size: 16),
                        label: const Text('全体設定を保存'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '※OBSで効果音を配信に載せる場合は「OBSを介して音声を制御する」を有効にしてください',
                    style: TextStyle(fontSize: 11.5, color: p.textSecondary),
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 14,
                    runSpacing: 10,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      SegmentedButton<MediaPlaybackMode>(
                        segments: const [
                          ButtonSegment(
                            value: MediaPlaybackMode.queue,
                            icon: Icon(Icons.queue, size: 16),
                            label: Text('順番待ち（1つずつ）'),
                          ),
                          ButtonSegment(
                            value: MediaPlaybackMode.overlay,
                            icon: Icon(Icons.layers_outlined, size: 16),
                            label: Text('重ね表示（同時再生）'),
                          ),
                          ButtonSegment(
                            value: MediaPlaybackMode.replace,
                            icon: Icon(Icons.bolt, size: 16),
                            label: Text('上書き（最新のみ）'),
                          ),
                        ],
                        selected: {_playbackMode},
                        onSelectionChanged: (s) =>
                            setState(() => _playbackMode = s.first),
                      ),
                      SizedBox(
                        width: 215,
                        child: CheckboxListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          controlAffinity: ListTileControlAffinity.leading,
                          value: _allowMultipleMatch,
                          onChanged: (v) =>
                              setState(() => _allowMultipleMatch = v ?? true),
                          title: const Text('1コメントで複数同時発動',
                              style: TextStyle(fontSize: 12.5)),
                        ),
                      ),
                      SizedBox(
                        width: 220,
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('マスター音量: $_masterVolume%',
                                style: TextStyle(
                                    fontSize: 12, color: p.textSecondary)),
                            Slider(
                              value: _masterVolume.toDouble().clamp(0, 100),
                              min: 0,
                              max: 100,
                              divisions: 20,
                              onChanged: (v) =>
                                  setState(() => _masterVolume = v.round()),
                            ),
                          ],
                        ),
                      ),
                      if (_playbackMode == MediaPlaybackMode.overlay)
                        SizedBox(
                          width: 130,
                          child: DropdownButtonFormField<int>(
                            key: ValueKey(_maxConcurrent.clamp(1, 12)),
                            initialValue: _maxConcurrent.clamp(1, 12),
                            isExpanded: true,
                            decoration:
                                const InputDecoration(labelText: '固定同時上限'),
                            items: List.generate(
                              12,
                              (i) => DropdownMenuItem(
                                value: i + 1,
                                child: Text('${i + 1} 個'),
                              ),
                            ),
                            onChanged: (v) {
                              if (v != null) setState(() => _maxConcurrent = v);
                            },
                          ),
                        ),
                    ],
                  ),
                  const Divider(height: 18),
                  OverlayBgPicker(
                    title: 'リアクションステージ オーバーレイ背景色（選択すると OBS ＆ 全端末へ即時反映）',
                    compact: true,
                    value: ctrl.overlayConfig.mediaBgColor,
                    onChanged: ctrl.setMediaOverlayBgColor,
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),

          // ワンクリック定番テンプレートバー
          Wrap(
            spacing: 6,
            runSpacing: 6,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text(
                '定番テンプレート追加:',
                style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    color: p.textSecondary),
              ),
              ActionChip(
                avatar: const Icon(Icons.celebration_outlined, size: 15),
                label: const Text('おめでとうクラッカー'),
                onPressed: () => _addQuickPreset('cracker'),
              ),
              ActionChip(
                avatar: const Icon(Icons.arrow_downward_rounded, size: 15),
                label: const Text('草シャワー落下'),
                onPressed: () => _addQuickPreset('kusa'),
              ),
              ActionChip(
                avatar: const Icon(Icons.auto_awesome, size: 15),
                label: const Text('888 紙吹雪'),
                onPressed: () => _addQuickPreset('confetti'),
              ),
              ActionChip(
                avatar: const Icon(Icons.sports_baseball_outlined, size: 15),
                label: const Text('ナイス投てき'),
                onPressed: () => _addQuickPreset('throw'),
              ),
              ActionChip(
                avatar: const Icon(Icons.ac_unit, size: 15),
                label: const Text('雪の環境演出'),
                onPressed: () => _addQuickPreset('snow'),
              ),
              ActionChip(
                avatar: const Icon(Icons.volunteer_activism_outlined, size: 15),
                label: const Text('スパチャ桜吹雪'),
                onPressed: () => _addQuickPreset('superchat_sakura'),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // ルール一覧
          Card(
            child: rules.isEmpty
                ? Padding(
                    padding: const EdgeInsets.symmetric(
                        vertical: 40, horizontal: 20),
                    child: Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.celebration_outlined,
                              size: 46, color: p.textMuted),
                          const SizedBox(height: 10),
                          Text(
                            '演出ルールがまだありません',
                            style: TextStyle(
                                fontSize: 15, color: p.textSecondary),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            '上の「定番テンプレート追加」からワンクリックで追加するか、「演出を追加」からオリジナルの画像・動画・効果音を設定できます。',
                            textAlign: TextAlign.center,
                            style:
                                TextStyle(fontSize: 12.5, color: p.textMuted),
                          ),
                        ],
                      ),
                    ),
                  )
                : ListView.separated(
                    shrinkWrap: true,
                    physics: const NeverScrollableScrollPhysics(),
                    padding: const EdgeInsets.all(12),
                    itemCount: rules.length,
                    separatorBuilder: (_, _) => const SizedBox(height: 8),
                    itemBuilder: (context, idx) {
                      final r = rules[idx];
                      final patternPreview = r.pattern
                          .split('\n')
                          .map((e) => e.trim())
                          .where((e) => e.isNotEmpty)
                          .join(' / ');
                      final assetSummary = r.effectType == MediaEffectType.mod
                          ? 'MOD: ${r.modId} / ${r.modEffectId}'
                          : (r.originalFileName.isNotEmpty
                              ? '素材: ${r.originalFileName}'
                              : (r.effectType == MediaEffectType.ambient
                                  ? '環境: ${ambientLabel(r.ambientPreset)}'
                                  : (r.particleText.isNotEmpty
                                      ? '文字/絵文字: ${r.particleText}'
                                      : '標準パーティクル')));
                      final seSummary = r.soundOriginalFileName.isNotEmpty
                          ? '  |  SE: ${r.soundOriginalFileName}'
                          : '';

                      return Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 14, vertical: 10),
                        decoration: BoxDecoration(
                          color: p.cardBg,
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(
                            color: r.enabled ? p.border : p.subtleBg,
                          ),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Container(
                                  width: 40,
                                  height: 40,
                                  decoration: BoxDecoration(
                                    color: p.accent.withValues(alpha: 0.14),
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                  child: Icon(
                                    effectIcon(r.effectType),
                                    color: p.accent,
                                  ),
                                ),
                                const SizedBox(width: 12),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Wrap(
                                        spacing: 6,
                                        runSpacing: 4,
                                        crossAxisAlignment:
                                            WrapCrossAlignment.center,
                                        children: [
                                          Text(
                                            r.name,
                                            style: const TextStyle(
                                              fontWeight: FontWeight.bold,
                                              fontSize: 14.5,
                                            ),
                                          ),
                                          _ChipTag(effectLabel(r.effectType),
                                              p.accent),
                                          _ChipTag(
                                            triggerLabel(r.triggerType,
                                                r.minSuperchatAmount),
                                            p.success,
                                          ),
                                          if (r.triggerType ==
                                                  MediaTriggerType.keyword &&
                                              patternPreview.isNotEmpty)
                                            _ChipTag('反応: $patternPreview',
                                                p.warning),
                                          Text(
                                            '優先:${r.priority}',
                                            style: TextStyle(
                                                fontSize: 11,
                                                color: p.textMuted),
                                          ),
                                        ],
                                      ),
                                      const SizedBox(height: 4),
                                        Text(
                                        '$assetSummary$seSummary  |  '
                                        '${r.effectType == MediaEffectType.mod ? "カスタムMOD演出" : (r.effectType == MediaEffectType.fixed ? "幅${r.widthPercent}%" : "個数${r.particleCount} / サイズ${r.particleSize}px")}  |  '
                                        '時間: ${r.durationMs == 0 ? "動画終了まで" : "${(r.durationMs / 1000).toStringAsFixed(1)}秒"}  |  CD: ${r.cooldownSec}秒',
                                        style: TextStyle(
                                            fontSize: 12,
                                            color: p.textSecondary),
                                      ),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 8),
                                Switch(
                                  value: r.enabled,
                                  onChanged: (v) => ctrl.saveMediaReactionRule(
                                    _copyRule(r, enabled: v),
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 6),
                            Align(
                              alignment: Alignment.centerRight,
                              child: Wrap(
                                spacing: 4,
                                runSpacing: 4,
                                crossAxisAlignment: WrapCrossAlignment.center,
                                children: [
                                  IconButton(
                                    tooltip: '優先度を上げる',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () => ctrl.saveMediaReactionRule(
                                      _copyRule(r, priority: r.priority + 5),
                                    ),
                                    icon: const Icon(Icons.arrow_upward,
                                        size: 16),
                                  ),
                                  IconButton(
                                    tooltip: '優先度を下げる',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () => ctrl.saveMediaReactionRule(
                                      _copyRule(r, priority: r.priority - 5),
                                    ),
                                    icon: const Icon(Icons.arrow_downward,
                                        size: 16),
                                  ),
                                  OutlinedButton.icon(
                                    style: OutlinedButton.styleFrom(
                                      visualDensity: VisualDensity.compact,
                                    ),
                                    onPressed: () =>
                                        ctrl.testMediaReactionRule(r.id),
                                    icon: const Icon(Icons.play_arrow, size: 16),
                                    label: const Text('テスト再生'),
                                  ),
                                  IconButton(
                                    tooltip: '編集',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () => _openEditor(initial: r),
                                    icon: const Icon(Icons.edit_outlined,
                                        size: 18),
                                  ),
                                  IconButton(
                                    tooltip: '削除',
                                    visualDensity: VisualDensity.compact,
                                    onPressed: () =>
                                        ctrl.deleteMediaReactionRule(r.id),
                                    icon: Icon(Icons.delete_outline,
                                        size: 18, color: p.danger),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _ChipTag extends StatelessWidget {
  const _ChipTag(this.label, this.color);
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          color: color,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _ReactionStageRuleEditorDialog extends StatefulWidget {
  const _ReactionStageRuleEditorDialog({
    required this.controller,
    this.initial,
  });

  final StosupController controller;
  final MediaReactionRule? initial;

  @override
  State<_ReactionStageRuleEditorDialog> createState() =>
      _ReactionStageRuleEditorDialogState();
}

class _ReactionStageRuleEditorDialogState
    extends State<_ReactionStageRuleEditorDialog> {
  late TextEditingController _nameCtrl;
  late TextEditingController _patternCtrl;
  late TextEditingController _minScCtrl;
  late TextEditingController _priorityCtrl;
  late TextEditingController _captionCtrl;
  late TextEditingController _durationSecCtrl;
  late TextEditingController _cooldownCtrl;
  late TextEditingController _particleTextCtrl;

  late MediaEffectType _effectType;
  late MediaTriggerType _triggerType;
  late MatchMode _matchMode;
  late bool _caseSensitive;
  late MediaKind _mediaKind;
  late String _storedFileName;
  late String _originalFileName;
  late String _soundStoredFileName;
  late String _soundOriginalFileName;
  late int _soundVolume;
  late MediaAnimationKind _animation;
  late MediaPosition _position;
  late int _widthPercent;
  late int _volume;
  late int _particleCount;
  late int _particleSize;
  late int _particleSpeed;
  late int _gravity;
  late String _ambientPreset;
  late bool _excludeBanned;
  late bool _enabled;
  late String _modId;
  late String _modEffectId;
  late Map<String, dynamic> _modParams;
  bool _importingMedia = false;
  bool _importingSound = false;

  @override
  void initState() {
    super.initState();
    final r = widget.initial;
    _nameCtrl = TextEditingController(text: r?.name ?? '');
    _patternCtrl = TextEditingController(text: r?.pattern ?? '');
    _minScCtrl =
        TextEditingController(text: (r?.minSuperchatAmount ?? BigInt.zero).toString());
    _priorityCtrl = TextEditingController(text: (r?.priority ?? 10).toString());
    _captionCtrl = TextEditingController(text: r?.captionTemplate ?? '');
    _durationSecCtrl = TextEditingController(
      text: r == null
          ? '4.0'
          : (r.durationMs == 0 ? '0' : (r.durationMs / 1000).toString()),
    );
    _cooldownCtrl = TextEditingController(text: (r?.cooldownSec ?? 3).toString());
    _particleTextCtrl = TextEditingController(text: r?.particleText ?? '');

    _effectType = r?.effectType ?? MediaEffectType.cracker;
    _triggerType = r?.triggerType ?? MediaTriggerType.keyword;
    _matchMode = r?.matchMode ?? MatchMode.contains;
    _caseSensitive = r?.caseSensitive ?? false;
    _mediaKind = r?.mediaKind ?? MediaKind.image;
    _storedFileName = r?.storedFileName ?? '';
    _originalFileName = r?.originalFileName ?? '';
    _soundStoredFileName = r?.soundStoredFileName ?? '';
    _soundOriginalFileName = r?.soundOriginalFileName ?? '';
    _soundVolume = r?.soundVolume ?? 85;
    _animation = r?.animation ?? MediaAnimationKind.popIn;
    _position = r?.position ?? MediaPosition.center;
    _widthPercent = r?.widthPercent ?? 45;
    _volume = r?.volume ?? 80;
    _particleCount = r?.particleCount ?? 28;
    _particleSize = r?.particleSize ?? 64;
    _particleSpeed = r?.particleSpeed ?? 100;
    _gravity = r?.gravity ?? 100;
    _ambientPreset = r?.ambientPreset ?? 'snow';
    _excludeBanned = r?.excludeBanned ?? true;
    _enabled = r?.enabled ?? true;
    _modId = r?.modId ?? '';
    _modEffectId = r?.modEffectId ?? '';
    _modParams = {};
    if (r != null && r.modParamsJson.trim().isNotEmpty) {
      try {
        final decoded = jsonDecode(r.modParamsJson);
        if (decoded is Map<String, dynamic>) {
          _modParams = Map<String, dynamic>.from(decoded);
        }
      } catch (_) {}
    }
    _ensureModSelectionDefault();
  }

  void _ensureModSelectionDefault() {
    final available = widget.controller.enabledModReactionEffects;
    if (available.isEmpty) return;
    final hasMatch = available.any(
      (e) => e.modId == _modId && e.effectId == _modEffectId,
    );
    if (!hasMatch && _modId.isEmpty && _modEffectId.isEmpty) {
      final first = available.first;
      _modId = first.modId;
      _modEffectId = first.effectId;
      _seedModParamDefaults(first, overwrite: false);
    }
  }

  void _seedModParamDefaults(
    ModReactionEffectDef ef, {
    required bool overwrite,
  }) {
    for (final param in ef.params) {
      if (!overwrite && _modParams.containsKey(param.key)) continue;
      switch (param.kind) {
        case ModParamKind.number:
          _modParams[param.key] = double.tryParse(param.defaultValue) ?? 0;
        case ModParamKind.bool:
          _modParams[param.key] =
              param.defaultValue.toLowerCase() == 'true' ||
                  param.defaultValue == '1';
        case ModParamKind.string:
        case ModParamKind.color:
        case ModParamKind.select:
          _modParams[param.key] = param.defaultValue;
      }
    }
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _patternCtrl.dispose();
    _minScCtrl.dispose();
    _priorityCtrl.dispose();
    _captionCtrl.dispose();
    _durationSecCtrl.dispose();
    _cooldownCtrl.dispose();
    _particleTextCtrl.dispose();
    super.dispose();
  }

  Future<void> _pickAndImportMedia() async {
    setState(() => _importingMedia = true);
    try {
      final path = await widget.controller.pickMediaFileDialog();
      if (path == null || path.isEmpty) return;
      final imported = await widget.controller.importMediaFile(path);
      if (imported != null && mounted) {
        setState(() {
          _storedFileName = imported.storedFileName;
          _originalFileName = imported.originalFileName;
          _mediaKind = imported.mediaKind;
          if (_nameCtrl.text.trim().isEmpty) {
            _nameCtrl.text = imported.originalFileName;
          }
          if (imported.mediaKind == MediaKind.video) {
            _effectType = MediaEffectType.fixed;
            if (widget.initial == null) _durationSecCtrl.text = '0';
          }
        });
      }
    } finally {
      if (mounted) setState(() => _importingMedia = false);
    }
  }

  Future<void> _pickAndImportSound() async {
    setState(() => _importingSound = true);
    try {
      final path = await widget.controller.pickSoundFileDialog();
      if (path == null || path.isEmpty) return;
      final imported = await widget.controller.importMediaFile(path);
      if (imported != null && mounted) {
        setState(() {
          _soundStoredFileName = imported.storedFileName;
          _soundOriginalFileName = imported.originalFileName;
        });
      }
    } finally {
      if (mounted) setState(() => _importingSound = false);
    }
  }

  Future<void> _save() async {
    if (_effectType == MediaEffectType.mod &&
        (_modId.trim().isEmpty || _modEffectId.trim().isEmpty)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('カスタム MOD 演出を選択してください（未導入の場合は「MOD 管理」画面から導入できます）'),
        ),
      );
      return;
    }
    if (_effectType == MediaEffectType.fixed &&
        _storedFileName.isEmpty &&
        _soundStoredFileName.isEmpty &&
        _captionCtrl.text.trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('固定表示では画像・動画・音声ファイルまたはテロップを設定してください'),
        ),
      );
      return;
    }

    final parsedDur = double.tryParse(_durationSecCtrl.text.trim());
    final durSec = (parsedDur != null && parsedDur.isFinite)
        ? parsedDur.clamp(0.0, 600.0)
        : 4.0;
    final durMs = durSec <= 0 ? 0 : (durSec * 1000).round();
    final cd = (int.tryParse(_cooldownCtrl.text.trim()) ?? 3).clamp(0, 3600);
    final prio = int.tryParse(_priorityCtrl.text.trim()) ?? 10;
    final minSc = BigInt.tryParse(_minScCtrl.text.trim()) ?? BigInt.zero;

    final fallbackName = _patternCtrl.text
            .split('\n')
            .map((e) => e.trim())
            .firstWhere((e) => e.isNotEmpty, orElse: () => '')
            .isNotEmpty
        ? _patternCtrl.text
            .split('\n')
            .map((e) => e.trim())
            .firstWhere((e) => e.isNotEmpty)
        : MediaReactionScreenStateHelper.defaultRuleTitle(_effectType);

    final rule = MediaReactionRule(
      id: widget.initial?.id ?? '',
      name: _nameCtrl.text.trim().isEmpty ? fallbackName : _nameCtrl.text.trim(),
      pattern: _patternCtrl.text.trim(),
      matchMode: _matchMode,
      caseSensitive: _caseSensitive,
      effectType: _effectType,
      triggerType: _triggerType,
      minSuperchatAmount: minSc < BigInt.zero ? BigInt.zero : minSc,
      priority: prio,
      mediaKind: _mediaKind,
      storedFileName: _storedFileName,
      originalFileName: _originalFileName,
      soundStoredFileName: _soundStoredFileName,
      soundOriginalFileName: _soundOriginalFileName,
      soundVolume: _soundVolume,
      animation: _animation,
      position: _position,
      durationMs: durMs,
      widthPercent: _widthPercent,
      volume: _volume,
      particleCount: _particleCount,
      particleSize: _particleSize,
      particleSpeed: _particleSpeed,
      gravity: _gravity,
      particleText: _particleTextCtrl.text.trim(),
      ambientPreset: _ambientPreset,
      cooldownSec: cd,
      excludeBanned: _excludeBanned,
      captionTemplate: _captionCtrl.text.trim(),
      enabled: _enabled,
      modId: _effectType == MediaEffectType.mod ? _modId : '',
      modEffectId: _effectType == MediaEffectType.mod ? _modEffectId : '',
      modParamsJson: _effectType == MediaEffectType.mod
          ? jsonEncode(_modParams)
          : '{}',
    );
    final nav = Navigator.of(context);
    await widget.controller.saveMediaReactionRule(rule);
    if (!mounted) return;
    nav.pop();
  }

  Widget _buildModEffectSelectorSection(StosupPalette p) {
    final available = widget.controller.enabledModReactionEffects;
    if (available.isEmpty) {
      return Container(
        width: double.infinity,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: p.subtleBg,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: p.border),
        ),
        child: Text(
          '有効なリアクション演出 MOD がまだ導入されていません。\n左メニューの「MOD 管理」画面から .stosupmod / .zip を導入してください。',
          style: TextStyle(fontSize: 12.5, color: p.textSecondary),
        ),
      );
    }

    final currentKey = '$_modId:$_modEffectId';
    final selectedDef = available.cast<ModReactionEffectDef?>().firstWhere(
          (e) => e != null && '${e.modId}:${e.effectId}' == currentKey,
          orElse: () => null,
        );

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: p.subtleBg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: p.accent.withValues(alpha: 0.4)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          DropdownButtonFormField<String>(
            key: ValueKey('mod_sel_$currentKey'),
            initialValue: selectedDef != null ? currentKey : null,
            isExpanded: true,
            decoration: const InputDecoration(
              labelText: '使用するカスタム MOD 演出',
            ),
            items: available
                .map(
                  (e) => DropdownMenuItem<String>(
                    value: '${e.modId}:${e.effectId}',
                    child: Text('${e.name}（${e.modName}）'),
                  ),
                )
                .toList(),
            onChanged: (val) {
              if (val == null) return;
              final found = available.firstWhere(
                (e) => '${e.modId}:${e.effectId}' == val,
              );
              setState(() {
                _modId = found.modId;
                _modEffectId = found.effectId;
                _modParams = {};
                _seedModParamDefaults(found, overwrite: true);
                if (widget.initial == null) {
                  _durationSecCtrl.text =
                      (found.defaultDurationMs / 1000).toString();
                  if (_nameCtrl.text.trim().isEmpty) {
                    _nameCtrl.text = found.name;
                  }
                }
              });
            },
          ),
          if (selectedDef != null && selectedDef.description.isNotEmpty) ...[
            const SizedBox(height: 6),
            Text(
              selectedDef.description,
              style: TextStyle(fontSize: 12, color: p.textSecondary),
            ),
          ],
          if (selectedDef != null && selectedDef.params.isNotEmpty) ...[
            const Divider(height: 20),
            const Text(
              'MOD カスタムパラメータ',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12.5),
            ),
            const SizedBox(height: 8),
            ...selectedDef.params.map((param) => _buildModParamField(p, param)),
          ],
        ],
      ),
    );
  }

  Widget _buildModParamField(StosupPalette p, ModParamSchema param) {
    final cur = _modParams[param.key];
    switch (param.kind) {
      case ModParamKind.bool:
        final bVal = cur is bool
            ? cur
            : (cur?.toString().toLowerCase() == 'true' ||
                (cur == null && param.defaultValue.toLowerCase() == 'true'));
        return SwitchListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          title: Text(param.label, style: const TextStyle(fontSize: 13)),
          subtitle: param.description.isNotEmpty
              ? Text(param.description,
                  style: TextStyle(fontSize: 11, color: p.textSecondary))
              : null,
          value: bVal,
          onChanged: (v) => setState(() => _modParams[param.key] = v),
        );
      case ModParamKind.select:
        final opts = param.options.isEmpty ? [param.defaultValue] : param.options;
        final sVal = cur?.toString() ?? param.defaultValue;
        final effective = opts.contains(sVal) ? sVal : opts.first;
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: DropdownButtonFormField<String>(
            key: ValueKey('mod_param_${_modId}_${_modEffectId}_${param.key}_$effective'),
            initialValue: effective,
            isExpanded: true,
            decoration: InputDecoration(
              labelText: param.label,
              helperText: param.description.isEmpty ? null : param.description,
            ),
            items: opts
                .map((o) => DropdownMenuItem(value: o, child: Text(o)))
                .toList(),
            onChanged: (v) {
              if (v != null) setState(() => _modParams[param.key] = v);
            },
          ),
        );
      case ModParamKind.number:
        final rawNum = cur is num
            ? cur.toDouble()
            : (double.tryParse(cur?.toString() ?? param.defaultValue) ?? 0);
        final numVal = rawNum.isFinite ? rawNum : 0.0;
        if (param.min != null && param.max != null && param.max! > param.min!) {
          final minV = param.min!;
          final maxV = param.max!;
          final clamped = numVal.clamp(minV, maxV);
          final step = (param.step != null && param.step! > 0)
              ? param.step!
              : ((maxV - minV) >= 20 ? 1.0 : (maxV - minV) / 20.0);
          final divs = ((maxV - minV) / step).round().clamp(1, 200);
          final display = clamped == clamped.roundToDouble()
              ? clamped.round().toString()
              : clamped.toStringAsFixed(1);
          return Padding(
            padding: const EdgeInsets.only(bottom: 6),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${param.label}: $display${param.description.isNotEmpty ? "（${param.description}）" : ""}',
                  style: TextStyle(fontSize: 12, color: p.textSecondary),
                ),
                Slider(
                  value: clamped,
                  min: minV,
                  max: maxV,
                  divisions: divs,
                  onChanged: (v) => setState(() => _modParams[param.key] = v),
                ),
              ],
            ),
          );
        }
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: TextFormField(
            key: ValueKey('mod_param_${_modId}_${_modEffectId}_${param.key}'),
            initialValue: numVal.toString(),
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: param.label,
              helperText: param.description.isEmpty ? null : param.description,
            ),
            onChanged: (v) {
              final parsed = double.tryParse(v.trim());
              if (parsed != null && parsed.isFinite) {
                _modParams[param.key] = parsed;
              }
            },
          ),
        );
      case ModParamKind.color:
        final cVal = (cur?.toString() ?? param.defaultValue).trim();
        final previewColor = AppTheme.parseCssHex(
          cVal.isEmpty ? '#38BDF8' : cVal,
        );
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Row(
            children: [
              Container(
                width: 26,
                height: 26,
                decoration: BoxDecoration(
                  color: previewColor,
                  borderRadius: BorderRadius.circular(5),
                  border: Border.all(color: p.border),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: TextFormField(
                  key: ValueKey(
                      'mod_param_${_modId}_${_modEffectId}_${param.key}'),
                  initialValue: cVal,
                  decoration: InputDecoration(
                    labelText: '${param.label}（HEX 色コード）',
                    hintText: '#38BDF8',
                    helperText:
                        param.description.isEmpty ? null : param.description,
                  ),
                  onChanged: (v) => setState(() => _modParams[param.key] = v.trim()),
                ),
              ),
            ],
          ),
        );
      case ModParamKind.string:
        final strVal = cur?.toString() ?? param.defaultValue;
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: TextFormField(
            key: ValueKey('mod_param_${_modId}_${_modEffectId}_${param.key}'),
            initialValue: strVal,
            decoration: InputDecoration(
              labelText: param.label,
              helperText: param.description.isEmpty ? null : param.description,
            ),
            onChanged: (v) => _modParams[param.key] = v,
          ),
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final isFixed = _effectType == MediaEffectType.fixed;
    final isAmbient = _effectType == MediaEffectType.ambient;
    final isMod = _effectType == MediaEffectType.mod;

    return AlertDialog(
      title: Text(widget.initial == null
          ? 'リアクションステージ演出を追加'
          : 'リアクションステージ演出を編集'),
      content: SizedBox(
        width: 640,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 1. 演出タイプ選択
              const Text('1. 演出の種類',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
              const SizedBox(height: 6),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: MediaEffectType.values.map((t) {
                  final sel = _effectType == t;
                  return ChoiceChip(
                    avatar: Icon(
                      _MediaReactionScreenState.effectIcon(t),
                      size: 16,
                      color: sel ? p.accent : p.textSecondary,
                    ),
                    label: Text(_MediaReactionScreenState.effectLabel(t)),
                    selected: sel,
                    onSelected: (_) => setState(() => _effectType = t),
                  );
                }).toList(),
              ),
              const SizedBox(height: 14),

              // 2. 名前・優先順位・トリガー条件
              Row(
                children: [
                  Expanded(
                    flex: 3,
                    child: TextField(
                      controller: _nameCtrl,
                      decoration: const InputDecoration(
                        labelText: '演出名',
                        hintText: '例: おめでとうクラッカー',
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    flex: 2,
                    child: DropdownButtonFormField<MediaTriggerType>(
                      initialValue: _triggerType,
                      isExpanded: true,
                      decoration: const InputDecoration(labelText: '発動トリガー'),
                      items: const [
                        DropdownMenuItem(
                            value: MediaTriggerType.keyword,
                            child: Text('キーワード一致')),
                        DropdownMenuItem(
                            value: MediaTriggerType.anyComment,
                            child: Text('すべてのコメント')),
                        DropdownMenuItem(
                            value: MediaTriggerType.superChat,
                            child: Text('スーパーチャット')),
                        DropdownMenuItem(
                            value: MediaTriggerType.membership,
                            child: Text('メンバーシップ')),
                        DropdownMenuItem(
                            value: MediaTriggerType.firstTime,
                            child: Text('初見さん検知')),
                      ],
                      onChanged: (v) {
                        if (v != null) setState(() => _triggerType = v);
                      },
                    ),
                  ),
                  const SizedBox(width: 10),
                  SizedBox(
                    width: 90,
                    child: TextField(
                      controller: _priorityCtrl,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(labelText: '優先度'),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              if (_triggerType == MediaTriggerType.keyword) ...[
                TextField(
                  controller: _patternCtrl,
                  maxLines: 3,
                  decoration: const InputDecoration(
                    labelText: '反応ワード（1行につき1パターン / 空欄ですべてのコメントに反応）',
                    hintText: '888\nパチパチ\n👏',
                  ),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<MatchMode>(
                        initialValue: _matchMode,
                        isExpanded: true,
                        decoration: const InputDecoration(labelText: '一致方式'),
                        items: const [
                          DropdownMenuItem(
                              value: MatchMode.contains, child: Text('部分一致')),
                          DropdownMenuItem(
                              value: MatchMode.exact, child: Text('完全一致')),
                          DropdownMenuItem(
                              value: MatchMode.regex, child: Text('正規表現')),
                        ],
                        onChanged: (v) {
                          if (v != null) setState(() => _matchMode = v);
                        },
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: CheckboxListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        controlAffinity: ListTileControlAffinity.leading,
                        value: _caseSensitive,
                        onChanged: (v) =>
                            setState(() => _caseSensitive = v ?? false),
                        title: const Text('大文字・小文字を区別'),
                      ),
                    ),
                  ],
                ),
              ],

              if (_triggerType == MediaTriggerType.superChat) ...[
                TextField(
                  controller: _minScCtrl,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    labelText: '発動する最低スパチャ金額（0 = すべてのスパチャで発動）',
                    hintText: '例: 1000',
                  ),
                ),
              ],
              const SizedBox(height: 12),

              // 3. 画像・動画・効果音素材
              const Text('2. 素材ファイル ＆ 効果音 (SE)',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
              const SizedBox(height: 6),
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: p.subtleBg,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: p.border),
                ),
                child: Column(
                  children: [
                    Row(
                      children: [
                        Icon(
                          _mediaKind == MediaKind.video
                              ? Icons.videocam_outlined
                              : (_mediaKind == MediaKind.audio
                                  ? Icons.audiotrack_outlined
                                  : Icons.image_outlined),
                          color: p.accent,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                _originalFileName.isEmpty
                                    ? (isFixed
                                        ? '画像・GIF・透過WebM/MP4・音声（任意）'
                                        : 'カスタム画像（未選択時は絵文字/標準シェイプ）')
                                    : _originalFileName,
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold, fontSize: 13),
                              ),
                              Text(
                                'PNG / JPG / GIF / WebP / SVG / WebM / MP4 / MP3 / WAV',
                                style: TextStyle(
                                    fontSize: 11, color: p.textSecondary),
                              ),
                            ],
                          ),
                        ),
                        if (_storedFileName.isNotEmpty)
                          IconButton(
                            tooltip: '素材をクリア',
                            onPressed: () => setState(() {
                              _storedFileName = '';
                              _originalFileName = '';
                            }),
                            icon: const Icon(Icons.close, size: 17),
                          ),
                        FilledButton.icon(
                          onPressed:
                              _importingMedia ? null : _pickAndImportMedia,
                          icon: _importingMedia
                              ? const SizedBox(
                                  width: 14,
                                  height: 14,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Icon(Icons.folder_open, size: 16),
                          label: const Text('素材を選択'),
                        ),
                      ],
                    ),
                    const Divider(height: 16),
                    Row(
                      children: [
                        Icon(Icons.volume_up_outlined, color: p.success),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                _soundOriginalFileName.isEmpty
                                    ? '同時再生する効果音 (SE) なし'
                                    : 'SE: $_soundOriginalFileName（音量 $_soundVolume%）',
                                style: const TextStyle(
                                    fontWeight: FontWeight.w600, fontSize: 13),
                              ),
                              Text(
                                '演出発動と同時に MP3 / WAV / OGG を鳴らせます',
                                style: TextStyle(
                                    fontSize: 11, color: p.textSecondary),
                              ),
                            ],
                          ),
                        ),
                        if (_soundStoredFileName.isNotEmpty)
                          IconButton(
                            tooltip: '効果音を解除',
                            onPressed: () => setState(() {
                              _soundStoredFileName = '';
                              _soundOriginalFileName = '';
                            }),
                            icon: const Icon(Icons.close, size: 17),
                          ),
                        OutlinedButton.icon(
                          onPressed:
                              _importingSound ? null : _pickAndImportSound,
                          icon: _importingSound
                              ? const SizedBox(
                                  width: 14,
                                  height: 14,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Icon(Icons.music_note, size: 16),
                          label: const Text('効果音を選択'),
                        ),
                      ],
                    ),
                    if (_soundStoredFileName.isNotEmpty)
                      Slider(
                        value: _soundVolume.toDouble().clamp(0, 100),
                        min: 0,
                        max: 100,
                        divisions: 20,
                        label: 'SE音量 $_soundVolume%',
                        onChanged: (v) =>
                            setState(() => _soundVolume = v.round()),
                      ),
                  ],
                ),
              ),
              const SizedBox(height: 12),

              // 4. 演出別パラメータ
              if (isMod) ...[
                _buildModEffectSelectorSection(p),
                const SizedBox(height: 10),
              ] else if (isAmbient) ...[
                DropdownButtonFormField<String>(
                  initialValue: _ambientPreset,
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: '環境演出プリセット'),
                  items: const [
                    DropdownMenuItem(value: 'snow', child: Text('雪（ふわふわ降雪）')),
                    DropdownMenuItem(value: 'sakura', child: Text('桜吹雪（花びら）')),
                    DropdownMenuItem(value: 'rain', child: Text('雨（斜め雨筋）')),
                    DropdownMenuItem(
                        value: 'sparkle', child: Text('キラキラ（光の星粒）')),
                    DropdownMenuItem(
                        value: 'hearts', child: Text('ハート（下から上昇）')),
                    DropdownMenuItem(
                        value: 'embers', child: Text('火の粉（熱い光粒）')),
                  ],
                  onChanged: (v) {
                    if (v != null) setState(() => _ambientPreset = v);
                  },
                ),
                const SizedBox(height: 10),
              ],

              if (!isFixed && !isAmbient && !isMod) ...[
                TextField(
                  controller: _particleTextCtrl,
                  decoration: const InputDecoration(
                    labelText: '画像未指定時に飛ばす絵文字・文字（カンマ区切り）',
                    hintText: '例: 🎉, 🎊, ✨  または  草, 🌿',
                  ),
                ),
                const SizedBox(height: 10),
              ],

              if (isMod) ...[
                const SizedBox.shrink(),
              ] else if (isFixed) ...[
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<MediaAnimationKind>(
                        initialValue: _animation,
                        isExpanded: true,
                        decoration:
                            const InputDecoration(labelText: '登場アニメーション'),
                        items: const [
                          DropdownMenuItem(
                              value: MediaAnimationKind.popIn,
                              child: Text('ポップイン')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.fade,
                              child: Text('フェード')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.slideUp,
                              child: Text('下からスライド')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.slideLeft,
                              child: Text('右からスライド')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.bounce,
                              child: Text('バウンス')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.zoomSpin,
                              child: Text('ズーム回転')),
                          DropdownMenuItem(
                              value: MediaAnimationKind.none,
                              child: Text('なし')),
                        ],
                        onChanged: (v) {
                          if (v != null) setState(() => _animation = v);
                        },
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: DropdownButtonFormField<MediaPosition>(
                        initialValue: _position,
                        isExpanded: true,
                        decoration: const InputDecoration(labelText: '表示位置'),
                        items: const [
                          DropdownMenuItem(
                              value: MediaPosition.center, child: Text('画面中央')),
                          DropdownMenuItem(
                              value: MediaPosition.topLeft, child: Text('左上')),
                          DropdownMenuItem(
                              value: MediaPosition.topRight, child: Text('右上')),
                          DropdownMenuItem(
                              value: MediaPosition.bottomLeft,
                              child: Text('左下')),
                          DropdownMenuItem(
                              value: MediaPosition.bottomRight,
                              child: Text('右下')),
                          DropdownMenuItem(
                              value: MediaPosition.fullScreen,
                              child: Text('全画面')),
                        ],
                        onChanged: (v) {
                          if (v != null) setState(() => _position = v);
                        },
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('表示幅: $_widthPercent%',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _widthPercent.toDouble().clamp(15, 100),
                            min: 15,
                            max: 100,
                            divisions: 17,
                            onChanged: (v) =>
                                setState(() => _widthPercent = v.round()),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('動画・音声音量: $_volume%',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _volume.toDouble().clamp(0, 100),
                            min: 0,
                            max: 100,
                            divisions: 20,
                            onChanged: (v) =>
                                setState(() => _volume = v.round()),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ] else ...[
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('個数 / 密度: $_particleCount',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _particleCount.toDouble().clamp(1, 120),
                            min: 1,
                            max: 120,
                            divisions: 39,
                            onChanged: (v) =>
                                setState(() => _particleCount = v.round()),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('サイズ: ${_particleSize}px',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _particleSize.toDouble().clamp(20, 220),
                            min: 20,
                            max: 220,
                            divisions: 20,
                            onChanged: (v) =>
                                setState(() => _particleSize = v.round()),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('速度: $_particleSpeed%',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _particleSpeed.toDouble().clamp(30, 250),
                            min: 30,
                            max: 250,
                            divisions: 22,
                            onChanged: (v) =>
                                setState(() => _particleSpeed = v.round()),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('重力: $_gravity%',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary)),
                          Slider(
                            value: _gravity.toDouble().clamp(0, 200),
                            min: 0,
                            max: 200,
                            divisions: 20,
                            onChanged: (v) =>
                                setState(() => _gravity = v.round()),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ],

              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _durationSecCtrl,
                      keyboardType:
                          const TextInputType.numberWithOptions(decimal: true),
                      decoration: const InputDecoration(
                        labelText: '演出時間（秒）',
                        hintText: '動画は 0 で最後まで再生',
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: TextField(
                      controller: _cooldownCtrl,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(
                        labelText: '連打防止クールダウン（秒）',
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              TextField(
                controller: _captionCtrl,
                decoration: const InputDecoration(
                  labelText: '同時表示テロップ（任意・空欄で非表示）',
                  hintText: '例: {user} さんありがとう！',
                ),
              ),
              const SizedBox(height: 6),
              CheckboxListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                controlAffinity: ListTileControlAffinity.leading,
                value: _excludeBanned,
                onChanged: (v) => setState(() => _excludeBanned = v ?? true),
                title: const Text('BAN 済みユーザーのコメントでは発動しない'),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('キャンセル'),
        ),
        FilledButton.icon(
          onPressed: _save,
          icon: const Icon(Icons.save_outlined, size: 16),
          label: const Text('保存する'),
        ),
      ],
    );
  }
}

class MediaReactionScreenStateHelper {
  static String defaultRuleTitle(MediaEffectType t) =>
      '${_MediaReactionScreenState.effectLabel(t)}演出';
}
