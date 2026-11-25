import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';

class _PresetMeta {
  const _PresetMeta({
    required this.id,
    required this.name,
    required this.desc,
    required this.bg,
    required this.fg,
    required this.border,
  });

  final String id;
  final String name;
  final String desc;
  final Color bg;
  final Color fg;
  final Color border;
}

/// OBS オーバーレイ ＆ 各機能デザイン（プリセット選択 ＋ カスタム CSS 編集）画面
class OverlayScreen extends StatefulWidget {
  const OverlayScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<OverlayScreen> createState() => _OverlayScreenState();
}

class _OverlayScreenState extends State<OverlayScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabCtrl;
  late TextEditingController _portCtrl;
  late TextEditingController _commentsCssCtrl;
  late TextEditingController _lotteryCssCtrl;
  late TextEditingController _counterCssCtrl;

  late int _fontSize;
  late int _maxItems;
  late bool _showIcons;
  late bool _showTimestamps;
  late String _defaultBg;
  late String _defaultText;
  late String _commentsTheme;
  late String _lotteryTheme;
  late String _counterTheme;

  static const List<_PresetMeta> _commentPresets = [
    _PresetMeta(
      id: 'default',
      name: 'スタンダード',
      desc: '落ち着いたダーク吹き出し。視認性重視。',
      bg: Color(0xFF202124),
      fg: Color(0xFFF4F4F5),
      border: Color(0xFF52525B),
    ),
    _PresetMeta(
      id: 'minimal',
      name: 'ミニマル字幕風',
      desc: '背景控えめ＆左ラインで配信画面を邪魔しない。',
      bg: Color(0x990F0F12),
      fg: Color(0xFFFFFFFF),
      border: Color(0xFFA1A1AA),
    ),
    _PresetMeta(
      id: 'pop',
      name: 'ポップ・コミック',
      desc: '白ベース＋太黒枠の明るく元気な吹き出し。',
      bg: Color(0xFFFFFFFF),
      fg: Color(0xFF18181B),
      border: Color(0xFF18181B),
    ),
    _PresetMeta(
      id: 'neon',
      name: 'サイバー・ネオン',
      desc: '発光ボーダー付き。ゲーム配信に相性抜群。',
      bg: Color(0xFF0A0C16),
      fg: Color(0xFFECFEFF),
      border: Color(0xFF22D3EE),
    ),
    _PresetMeta(
      id: 'japanese',
      name: '和モダン',
      desc: '墨色＋金線＋赤アクセントの上品な明朝調。',
      bg: Color(0xFF1C1917),
      fg: Color(0xFFFAF5EB),
      border: Color(0xFFD4AF37),
    ),
  ];

  static const List<_PresetMeta> _lotteryPresets = [
    _PresetMeta(
      id: 'default',
      name: 'クラシック',
      desc: '落ち着いたダークカード＋ゴールド枠。',
      bg: Color(0xFF18181B),
      fg: Color(0xFFFDE047),
      border: Color(0xFFEAB308),
    ),
    _PresetMeta(
      id: 'festive',
      name: 'お祭り・派手演出',
      desc: '赤金の華やかなグラデーションでお祝い向け。',
      bg: Color(0xFF7F1D1D),
      fg: Color(0xFFFEF08A),
      border: Color(0xFFFACC15),
    ),
    _PresetMeta(
      id: 'minimal',
      name: 'シンプル・下部テロップ',
      desc: '画面下部に横長コンパクト表示で邪魔しない。',
      bg: Color(0xE018181B),
      fg: Color(0xFFFFFFFF),
      border: Color(0xFF3B82F6),
    ),
    _PresetMeta(
      id: 'neon',
      name: 'サイバー・ステージ',
      desc: 'ネオン発光フレームのゲーム大会向け演出。',
      bg: Color(0xFF090B14),
      fg: Color(0xFFF0ABFC),
      border: Color(0xFFE879F9),
    ),
    _PresetMeta(
      id: 'cute',
      name: 'パステル・ふんわり',
      desc: 'やわらかい白＆ピンク系の可愛らしいカード。',
      bg: Color(0xFFFFFBF5),
      fg: Color(0xFFDB2777),
      border: Color(0xFFF9A8D4),
    ),
  ];

  static const List<_PresetMeta> _counterPresets = [
    _PresetMeta(
      id: 'default',
      name: 'スタンダード',
      desc: 'バランス良く読みやすい標準カード表示。',
      bg: Color(0xFF18181B),
      fg: Color(0xFFF4F4F5),
      border: Color(0xFF3B82F6),
    ),
    _PresetMeta(
      id: 'compact',
      name: 'コンパクト・バッジ',
      desc: '項目名と数字だけを横並びにした省スペース型。',
      bg: Color(0xFF202124),
      fg: Color(0xFFF4F4F5),
      border: Color(0xFF22C55E),
    ),
    _PresetMeta(
      id: 'scoreboard',
      name: 'スコアボード',
      desc: '等幅ビッグ数字の対戦・計測・企画向けデザイン。',
      bg: Color(0xFF111214),
      fg: Color(0xFFFFFFFF),
      border: Color(0xFFEAB308),
    ),
    _PresetMeta(
      id: 'neon',
      name: 'ネオン・フレーム',
      desc: '発光数字＆発光ボーダーのゲーム配信向け。',
      bg: Color(0xFF090B14),
      fg: Color(0xFF67E8F9),
      border: Color(0xFF22D3EE),
    ),
    _PresetMeta(
      id: 'pastel',
      name: 'パステル・やわらか',
      desc: '明るい白ベース＆丸角の雑談配信向け。',
      bg: Color(0xFFFFFFFF),
      fg: Color(0xFF27272A),
      border: Color(0xFFF472B6),
    ),
  ];

  @override
  void initState() {
    super.initState();
    _tabCtrl = TabController(length: 4, vsync: this);
    final cfg = widget.controller.overlayConfig;
    _portCtrl = TextEditingController(text: cfg.port.toString());
    _commentsCssCtrl = TextEditingController(text: cfg.commentsCustomCss);
    _lotteryCssCtrl = TextEditingController(text: cfg.lotteryCustomCss);
    _counterCssCtrl = TextEditingController(text: cfg.counterCustomCss);
    _fontSize = cfg.commentFontSize;
    _maxItems = cfg.commentMaxItems;
    _showIcons = cfg.showIcons;
    _showTimestamps = cfg.showTimestamps;
    _defaultBg = cfg.defaultBubbleBg;
    _defaultText = cfg.defaultTextColor;
    _commentsTheme = cfg.commentsTheme;
    _lotteryTheme = cfg.lotteryTheme;
    _counterTheme = cfg.counterTheme;
  }

  @override
  void dispose() {
    _tabCtrl.dispose();
    _portCtrl.dispose();
    _commentsCssCtrl.dispose();
    _lotteryCssCtrl.dispose();
    _counterCssCtrl.dispose();
    super.dispose();
  }

  void _copy(String url, String label) {
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$label の OBS 用 URL をコピーしました')),
    );
  }

  Future<void> _save() async {
    final port = int.tryParse(_portCtrl.text.trim()) ??
        widget.controller.overlayConfig.port;
    await widget.controller.saveOverlayConfig(OverlayConfig(
      port: port.clamp(1024, 65535),
      commentFontSize: _fontSize,
      commentMaxItems: _maxItems,
      showIcons: _showIcons,
      showTimestamps: _showTimestamps,
      defaultBubbleBg: _defaultBg,
      defaultTextColor: _defaultText,
      commentsTheme: _commentsTheme,
      lotteryTheme: _lotteryTheme,
      counterTheme: _counterTheme,
      commentsCustomCss: _commentsCssCtrl.text,
      lotteryCustomCss: _lotteryCssCtrl.text,
      counterCustomCss: _counterCssCtrl.text,
    ));
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final obsEnabled = widget.controller.featureFlags.obsOverlayEnabled;

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
                  const Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'OBS オーバーレイ ＆ デザインカスタマイズ',
                          style: TextStyle(
                              fontSize: 20, fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                  FeatureToggleSwitch(
                    label: 'OBS サーバー',
                    enabled: obsEnabled,
                    onChanged: (v) => widget.controller.setFeatureEnabled(
                      obsOverlay: v,
                      featureLabel: 'OBS オーバーレイサーバー',
                    ),
                  ),
                  const SizedBox(width: 10),
                  FilledButton.icon(
                    onPressed: _save,
                    icon: const Icon(Icons.save_outlined, size: 17),
                    label: const Text('保存して即時反映'),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              TabBar(
                controller: _tabCtrl,
                isScrollable: true,
                tabAlignment: TabAlignment.start,
                tabs: const [
                  Tab(text: 'OBS URL・基本設定'),
                  Tab(text: 'コメント表示デザイン / CSS'),
                  Tab(text: '抽選画面デザイン / CSS'),
                  Tab(text: 'カウンターデザイン / CSS'),
                ],
              ),
            ],
          ),
        ),
        Expanded(
          child: TabBarView(
            controller: _tabCtrl,
            children: [
              _buildGeneralTab(obsEnabled),
              _buildDesignTab(
                title: 'コメント表示オーバーレイのデザイン',
                presets: _commentPresets,
                selectedId: _commentsTheme,
                onSelect: (id) => setState(() => _commentsTheme = id),
                cssController: _commentsCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/comments',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.comment-row（行全体） / .avatar（アイコン） / .bubble（吹き出し） / .bubble.highlighted（ワード反応時） / .author（名前） / .message（本文）',
                sampleCss:
                    '/* 例: 吹き出しを角張らせて文字に縁取りを付ける */\n.bubble {\n  border-radius: 4px;\n  padding: 12px 18px;\n}\n.message {\n  font-weight: 700;\n  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.75);\n}',
                previewBuilder: (meta) => _buildCommentMiniPreview(meta),
              ),
              _buildDesignTab(
                title: '抽選演出・当選者オーバーレイのデザイン',
                presets: _lotteryPresets,
                selectedId: _lotteryTheme,
                onSelect: (id) => setState(() => _lotteryTheme = id),
                cssController: _lotteryCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/lottery',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.banner（上部見出し） / .winner-card（当選カード） / .winner-avatar（アイコン） / .winner-name（当選者名） / .winner-comment（応募コメント） / .rolling（抽選中表示）',
                sampleCss:
                    '/* 例: 当選者名をさらに大きく強調する */\n.winner-card {\n  min-width: 360px;\n}\n.winner-name {\n  font-size: 34px;\n  letter-spacing: 0.04em;\n}',
                previewBuilder: (meta) => _buildLotteryMiniPreview(meta),
              ),
              _buildDesignTab(
                title: 'ワードカウンターオーバーレイのデザイン',
                presets: _counterPresets,
                selectedId: _counterTheme,
                onSelect: (id) => setState(() => _counterTheme = id),
                cssController: _counterCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/counter',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '#counters（全体コンテナ） / .card（カウンターカード） / .label（カウンター名） / .mode-tag（回数制限タグ） / .value（数値） / .sub（補足テキスト）',
                sampleCss:
                    '/* 例: カウンターを縦並びにし、数字を強調する */\n#counters {\n  flex-direction: column;\n}\n.value {\n  font-size: 54px;\n}',
                previewBuilder: (meta) => _buildCounterMiniPreview(meta),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildGeneralTab(bool obsEnabled) {
    final p = context.palette;
    final base = widget.controller.overlayBaseUrl;
    final commentsUrl = '$base/overlay/comments';
    final lotteryUrl = '$base/overlay/lottery';
    final counterUrl = '$base/overlay/counter';

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (!obsEnabled)
            FeatureDisabledBanner(
              featureName: 'OBS オーバーレイサーバー',
              description:
                  'ローカル HTTP/WebSocket サーバーを停止してポートを解放しています。OBS で表示する際はオンにしてください。',
              onEnable: () => widget.controller.setFeatureEnabled(
                obsOverlay: true,
                featureLabel: 'OBS オーバーレイサーバー',
              ),
            ),
          _OverlayUrlCard(
            icon: Icons.chat_bubble_outline,
            title: 'コメント表示オーバーレイ',
            description: '推奨サイズ: 幅 480 × 高さ 720。末尾に ?theme=pop 等を付けて個別上書きも可能。',
            url: commentsUrl,
            onCopy: () => _copy(commentsUrl, 'コメント表示'),
            onOpen: obsEnabled ? () => widget.controller.openUrl(commentsUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.emoji_events_outlined,
            title: '抽選演出・当選者オーバーレイ',
            description: '推奨サイズ: 幅 960 × 高さ 540。当選者のアイコン・名前・応募コメントを表示。',
            url: lotteryUrl,
            onCopy: () => _copy(lotteryUrl, '抽選結果表示'),
            onOpen: obsEnabled ? () => widget.controller.openUrl(lotteryUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.pin_outlined,
            title: '特定ワードカウンターオーバーレイ',
            description: '推奨サイズ: 幅 600 × 高さ 240。単体 URL はカウンター画面からコピー可能。',
            url: counterUrl,
            onCopy: () => _copy(counterUrl, 'ワードカウンター'),
            onOpen: obsEnabled ? () => widget.controller.openUrl(counterUrl) : null,
          ),
          const SizedBox(height: 18),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '共通サーバー ＆ コメント基本設定',
                    style:
                        TextStyle(fontSize: 15.5, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 14),
                  Row(
                    children: [
                      SizedBox(
                        width: 170,
                        child: TextField(
                          controller: _portCtrl,
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            labelText: 'サーバーポート',
                            hintText: '18765',
                          ),
                        ),
                      ),
                      const SizedBox(width: 24),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('コメント文字サイズ: ${_fontSize}px',
                                style: TextStyle(color: p.textSecondary)),
                            Slider(
                              value: _fontSize.toDouble(),
                              min: 12,
                              max: 36,
                              divisions: 24,
                              onChanged: (v) =>
                                  setState(() => _fontSize = v.round()),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 20),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('最大保持コメント数: $_maxItems件',
                                style: TextStyle(color: p.textSecondary)),
                            Slider(
                              value: _maxItems.toDouble(),
                              min: 5,
                              max: 100,
                              divisions: 19,
                              onChanged: (v) =>
                                  setState(() => _maxItems = v.round()),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: SwitchListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          value: _showIcons,
                          onChanged: (v) => setState(() => _showIcons = v),
                          title: const Text('ユーザーアイコンを表示する'),
                        ),
                      ),
                      Expanded(
                        child: SwitchListTile(
                          dense: true,
                          contentPadding: EdgeInsets.zero,
                          value: _showTimestamps,
                          onChanged: (v) => setState(() => _showTimestamps = v),
                          title: const Text('時刻を表示する'),
                        ),
                      ),
                    ],
                  ),
                  const Divider(height: 22),
                  ColorPickerRow(
                    label: 'スタンダード時の吹き出し背景色',
                    hexValue: _defaultBg,
                    onChanged: (v) => setState(() => _defaultBg = v),
                  ),
                  const SizedBox(height: 12),
                  ColorPickerRow(
                    label: 'スタンダード時の文字色',
                    hexValue: _defaultText,
                    presets: const [
                      '#F4F4F5',
                      '#E4E4E7',
                      '#FEF9C3',
                      '#ECFDF5',
                      '#18181B',
                    ],
                    onChanged: (v) => setState(() => _defaultText = v),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDesignTab({
    required String title,
    required List<_PresetMeta> presets,
    required String selectedId,
    required ValueChanged<String> onSelect,
    required TextEditingController cssController,
    required String previewUrl,
    required bool obsEnabled,
    required String selectorsHelp,
    required String sampleCss,
    required Widget Function(_PresetMeta meta) previewBuilder,
  }) {
    final p = context.palette;
    final current = presets.firstWhere(
      (e) => e.id == selectedId,
      orElse: () => presets.first,
    );

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  title,
                  style: const TextStyle(
                      fontSize: 16.5, fontWeight: FontWeight.bold),
                ),
              ),
              OutlinedButton.icon(
                onPressed: obsEnabled
                    ? () => widget.controller.openUrl(previewUrl)
                    : null,
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text('ブラウザで実寸プレビュー'),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // プリセット選択グリッド
          const Text('1. プリセットデザインを選択',
              style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5)),
          const SizedBox(height: 8),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: presets.map((meta) {
              final selected = meta.id == selectedId;
              return InkWell(
                onTap: () => onSelect(meta.id),
                borderRadius: BorderRadius.circular(8),
                child: Container(
                  width: 215,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: p.cardBg,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: selected ? p.accent : p.border,
                      width: selected ? 2 : 1,
                    ),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            width: 18,
                            height: 18,
                            decoration: BoxDecoration(
                              color: meta.bg,
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(color: meta.border, width: 2),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              meta.name,
                              style: const TextStyle(
                                  fontWeight: FontWeight.bold, fontSize: 13.5),
                            ),
                          ),
                          if (selected)
                            Icon(Icons.check_circle,
                                size: 16, color: p.accent),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        meta.desc,
                        style: TextStyle(
                            fontSize: 11.5, color: p.textSecondary),
                      ),
                    ],
                  ),
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 16),

          // イメージプレビュー
          const Text('選択中プリセットのイメージ',
              style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5)),
          const SizedBox(height: 8),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(18),
            decoration: BoxDecoration(
              color: const Color(0xFF1F2429),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: p.border),
            ),
            child: previewBuilder(current),
          ),
          const SizedBox(height: 18),

          // カスタム CSS エディタ
          Row(
            children: [
              const Expanded(
                child: Text(
                  '2. ユーザーカスタム CSS（上級者向け・プリセットの上書き調整）',
                  style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5),
                ),
              ),
              TextButton.icon(
                onPressed: () {
                  setState(() {
                    if (cssController.text.trim().isEmpty) {
                      cssController.text = sampleCss;
                    } else {
                      cssController.text = '${cssController.text}\n\n$sampleCss';
                    }
                  });
                },
                icon: const Icon(Icons.code, size: 16),
                label: const Text('サンプル CSS を挿入'),
              ),
              const SizedBox(width: 6),
              TextButton.icon(
                onPressed: () => setState(() => cssController.clear()),
                icon: const Icon(Icons.clear, size: 16),
                label: const Text('CSS をクリア'),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: p.subtleBg.withValues(alpha: 0.6),
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: p.border),
            ),
            child: SelectableText(
              '主な CSS セレクタ: $selectorsHelp',
              style: TextStyle(
                fontFamily: 'Consolas',
                fontSize: 11.5,
                color: p.textSecondary,
              ),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: cssController,
            maxLines: 9,
            style: const TextStyle(
              fontFamily: 'Consolas',
              fontSize: 13,
              height: 1.45,
            ),
            decoration: const InputDecoration(
              hintText: 'ここにカスタム CSS を記述し、右上の「保存して即時反映」を押すと OBS にリアルタイム適用されます...',
            ),
          ),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: _save,
              icon: const Icon(Icons.save_outlined, size: 17),
              label: const Text('このデザイン・CSS を保存して OBS へ即時反映'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCommentMiniPreview(_PresetMeta meta) {
    final radius = meta.id == 'pop'
        ? 16.0
        : (meta.id == 'japanese' || meta.id == 'minimal' ? 4.0 : 10.0);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        CircleAvatar(
          radius: 18,
          backgroundColor: meta.border,
          child: const Text('視',
              style:
                  TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
        ),
        const SizedBox(width: 10),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: meta.bg,
            borderRadius: BorderRadius.circular(radius),
            border: Border.all(
              color: meta.border,
              width: meta.id == 'pop' ? 2.5 : 1.5,
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'サンプル視聴者',
                style: TextStyle(
                  color: meta.fg.withValues(alpha: 0.8),
                  fontWeight: FontWeight.bold,
                  fontSize: 12,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                'プリセット「${meta.name}」でのコメント表示イメージです！',
                style: TextStyle(color: meta.fg, fontSize: 14),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildLotteryMiniPreview(_PresetMeta meta) {
    return Center(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 14),
        decoration: BoxDecoration(
          color: meta.bg,
          borderRadius: BorderRadius.circular(meta.id == 'cute' ? 20 : 10),
          border: Border.all(color: meta.border, width: 2),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircleAvatar(
              radius: 22,
              backgroundColor: meta.border,
              child: const Text('当',
                  style: TextStyle(
                      color: Colors.white, fontWeight: FontWeight.bold)),
            ),
            const SizedBox(width: 14),
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '当選者サンプル',
                  style: TextStyle(
                    color: meta.fg,
                    fontSize: 18,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Text(
                  '参加します！当たれ〜！',
                  style: TextStyle(
                    color: meta.fg.withValues(alpha: 0.78),
                    fontSize: 12.5,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildCounterMiniPreview(_PresetMeta meta) {
    final isCompact = meta.id == 'compact';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
      decoration: BoxDecoration(
        color: meta.bg,
        borderRadius: BorderRadius.circular(isCompact
            ? 999
            : (meta.id == 'pastel' ? 18 : 8)),
        border: Border.all(color: meta.border, width: 2),
      ),
      child: isCompact
          ? Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('草カウンター',
                    style: TextStyle(
                        color: meta.fg, fontWeight: FontWeight.bold)),
                const SizedBox(width: 12),
                Text('128',
                    style: TextStyle(
                        color: meta.border,
                        fontSize: 24,
                        fontWeight: FontWeight.w900)),
              ],
            )
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('草カウンター',
                    style: TextStyle(
                        color: meta.fg.withValues(alpha: 0.85),
                        fontSize: 12.5,
                        fontWeight: FontWeight.bold)),
                Text(
                  '128',
                  style: TextStyle(
                    color: meta.border,
                    fontSize: 36,
                    fontWeight: FontWeight.w900,
                    fontFamily: meta.id == 'scoreboard' ? 'Consolas' : null,
                  ),
                ),
                Text('対象: 草, www / 参加 42人',
                    style: TextStyle(
                        color: meta.fg.withValues(alpha: 0.6), fontSize: 11)),
              ],
            ),
    );
  }
}

class _OverlayUrlCard extends StatelessWidget {
  const _OverlayUrlCard({
    required this.icon,
    required this.title,
    required this.description,
    required this.url,
    required this.onCopy,
    required this.onOpen,
  });

  final IconData icon;
  final String title;
  final String description;
  final String url;
  final VoidCallback onCopy;
  final VoidCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: p.subtleBg,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(icon, color: p.accent, size: 22),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title,
                      style: const TextStyle(
                          fontWeight: FontWeight.bold, fontSize: 14.5)),
                  const SizedBox(height: 2),
                  Text(description,
                      style: TextStyle(fontSize: 12, color: p.textSecondary)),
                  const SizedBox(height: 4),
                  SelectableText(
                    url,
                    style: TextStyle(
                      fontFamily: 'Consolas',
                      color: p.accent,
                      fontSize: 12.5,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 10),
            FilledButton.icon(
              onPressed: onCopy,
              icon: const Icon(Icons.copy, size: 15),
              label: const Text('コピー'),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: onOpen,
              icon: const Icon(Icons.open_in_new, size: 15),
              label: const Text('確認'),
            ),
          ],
        ),
      ),
    );
  }
}
