import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/overlay_bg_picker.dart';
import '../widgets/system_font_picker.dart';

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

/// OBS オーバーレイ ＆ 各機能デザイン（プリセット・背景色切替・カスタム CSS 編集）画面
class OverlayScreen extends StatefulWidget {
  const OverlayScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<OverlayScreen> createState() => _OverlayScreenState();
}

class _OverlayScreenState extends State<OverlayScreen>
    with SingleTickerProviderStateMixin {
  static const List<String> _overlayTabLabels = [
    'OBS URL・基本設定',
    'コメント表示 / CSS',
    'スパチャ / CSS',
    'ピックアップ / CSS',
    '投票グラフ / CSS',
    '参加型キュー / CSS',
    'ワード演出 / CSS',
    '抽選画面 / CSS',
    'ワードカウント / CSS',
    '初見カウンター / CSS',
  ];

  late TabController _tabCtrl;
  final ScrollController _tabScrollCtrl = ScrollController();
  late TextEditingController _portCtrl;
  late TextEditingController _commentsCssCtrl;
  late TextEditingController _lotteryCssCtrl;
  late TextEditingController _counterCssCtrl;
  late TextEditingController _firstTimeCssCtrl;
  late TextEditingController _superchatCssCtrl;
  late TextEditingController _pollCssCtrl;
  late TextEditingController _pickupCssCtrl;
  late TextEditingController _queueCssCtrl;
  late TextEditingController _mediaCssCtrl;

  late int _fontSize;
  late int _maxItems;
  late bool _showIcons;
  late bool _showTimestamps;
  late String _defaultBg;
  late String _defaultText;
  late String _commentsTheme;
  late String _lotteryTheme;
  late String _counterTheme;
  late String _firstTimeTheme;
  late String _superchatTheme;
  late String _pollTheme;
  late String _pickupTheme;
  late String _queueTheme;
  late String _mediaTheme;
  late String _commentsBgColor;
  late String _lotteryBgColor;
  late String _counterBgColor;
  late String _firstTimeBgColor;
  late String _superchatBgColor;
  late String _pollBgColor;
  late String _pickupBgColor;
  late String _queueBgColor;
  late String _mediaBgColor;
  late String _commentsFont;
  late String _lotteryFont;
  late String _counterFont;
  late String _firstTimeFont;
  late String _superchatFont;
  late String _pollFont;
  late String _pickupFont;
  late String _queueFont;
  late String _mediaFont;
  late OverlayConfig _lastSyncedOverlayConfig;

  static const List<_PresetMeta> _richOverlayPresets = [
    _PresetMeta(
      id: 'default',
      name: 'スタンダード',
      desc: '落ち着いたダークカード。どんな配信画面にも馴染む。',
      bg: Color(0xFF18181B),
      fg: Color(0xFFF4F4F5),
      border: Color(0xFF3B82F6),
    ),
    _PresetMeta(
      id: 'minimal',
      name: 'ミニマル字幕風',
      desc: '背景控えめ＆左アクセントラインで画面を邪魔しない。',
      bg: Color(0xB80F0F12),
      fg: Color(0xFFFFFFFF),
      border: Color(0xFFA1A1AA),
    ),
    _PresetMeta(
      id: 'pop',
      name: 'ポップ・コミック',
      desc: '白ベース＋太黒枠の明るく視認性の高いカード。',
      bg: Color(0xFFFFFFFF),
      fg: Color(0xFF18181B),
      border: Color(0xFF18181B),
    ),
    _PresetMeta(
      id: 'neon',
      name: 'サイバー・ネオン',
      desc: '発光フレーム付きのゲーム・大会配信向けデザイン。',
      bg: Color(0xFF090B14),
      fg: Color(0xFFECFEFF),
      border: Color(0xFF22D3EE),
    ),
    _PresetMeta(
      id: 'pastel',
      name: 'パステル・ふんわり',
      desc: 'やわらかい白＆ピンク系の可愛らしいデザイン。',
      bg: Color(0xFFFFFBF5),
      fg: Color(0xFF27272A),
      border: Color(0xFFF472B6),
    ),
  ];

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
    _tabCtrl = TabController(length: _overlayTabLabels.length, vsync: this);
    _tabCtrl.addListener(_onTabChanged);
    final cfg = widget.controller.overlayConfig;
    _lastSyncedOverlayConfig = cfg;
    _portCtrl = TextEditingController(text: cfg.port.toString());
    _commentsCssCtrl = TextEditingController(text: cfg.commentsCustomCss);
    _lotteryCssCtrl = TextEditingController(text: cfg.lotteryCustomCss);
    _counterCssCtrl = TextEditingController(text: cfg.counterCustomCss);
    _firstTimeCssCtrl = TextEditingController(text: cfg.firstTimeCustomCss);
    _superchatCssCtrl = TextEditingController(text: cfg.superchatCustomCss);
    _pollCssCtrl = TextEditingController(text: cfg.pollCustomCss);
    _pickupCssCtrl = TextEditingController(text: cfg.pickupCustomCss);
    _queueCssCtrl = TextEditingController(text: cfg.queueCustomCss);
    _mediaCssCtrl = TextEditingController(text: cfg.mediaCustomCss);
    _fontSize = cfg.commentFontSize;
    _maxItems = cfg.commentMaxItems;
    _showIcons = cfg.showIcons;
    _showTimestamps = cfg.showTimestamps;
    _defaultBg = cfg.defaultBubbleBg;
    _defaultText = cfg.defaultTextColor;
    _commentsTheme = cfg.commentsTheme;
    _lotteryTheme = cfg.lotteryTheme;
    _counterTheme = cfg.counterTheme;
    _firstTimeTheme = cfg.firstTimeTheme;
    _superchatTheme = cfg.superchatTheme;
    _pollTheme = cfg.pollTheme;
    _pickupTheme = cfg.pickupTheme;
    _queueTheme = cfg.queueTheme;
    _mediaTheme = cfg.mediaTheme;
    _commentsBgColor = cfg.commentsBgColor;
    _lotteryBgColor = cfg.lotteryBgColor;
    _counterBgColor = cfg.counterBgColor;
    _firstTimeBgColor = cfg.firstTimeBgColor;
    _superchatBgColor = cfg.superchatBgColor;
    _pollBgColor = cfg.pollBgColor;
    _pickupBgColor = cfg.pickupBgColor;
    _queueBgColor = cfg.queueBgColor;
    _mediaBgColor = cfg.mediaBgColor;
    _commentsFont = cfg.commentsFontFamily;
    _lotteryFont = cfg.lotteryFontFamily;
    _counterFont = cfg.counterFontFamily;
    _firstTimeFont = cfg.firstTimeFontFamily;
    _superchatFont = cfg.superchatFontFamily;
    _pollFont = cfg.pollFontFamily;
    _pickupFont = cfg.pickupFontFamily;
    _queueFont = cfg.queueFontFamily;
    _mediaFont = cfg.mediaFontFamily;
  }

  void _onTabChanged() {
    if (mounted) setState(() {});
  }

  void _scrollTabsBy(double delta) {
    if (!_tabScrollCtrl.hasClients) return;
    final pos = _tabScrollCtrl.position;
    final target =
        (_tabScrollCtrl.offset + delta).clamp(pos.minScrollExtent, pos.maxScrollExtent);
    _tabScrollCtrl.jumpTo(target);
  }

  void _animateScrollTabsBy(double delta) {
    if (!_tabScrollCtrl.hasClients) return;
    final pos = _tabScrollCtrl.position;
    final target =
        (_tabScrollCtrl.offset + delta).clamp(pos.minScrollExtent, pos.maxScrollExtent);
    _tabScrollCtrl.animateTo(
      target,
      duration: const Duration(milliseconds: 180),
      curve: Curves.easeOut,
    );
  }

  @override
  void dispose() {
    _tabCtrl.removeListener(_onTabChanged);
    _tabCtrl.dispose();
    _tabScrollCtrl.dispose();
    _portCtrl.dispose();
    _commentsCssCtrl.dispose();
    _lotteryCssCtrl.dispose();
    _counterCssCtrl.dispose();
    _firstTimeCssCtrl.dispose();
    _superchatCssCtrl.dispose();
    _pollCssCtrl.dispose();
    _pickupCssCtrl.dispose();
    _queueCssCtrl.dispose();
    _mediaCssCtrl.dispose();
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
      firstTimeTheme: _firstTimeTheme,
      superchatTheme: _superchatTheme,
      pollTheme: _pollTheme,
      pickupTheme: _pickupTheme,
      queueTheme: _queueTheme,
      mediaTheme: _mediaTheme,
      commentsBgColor: _commentsBgColor,
      lotteryBgColor: _lotteryBgColor,
      counterBgColor: _counterBgColor,
      firstTimeBgColor: _firstTimeBgColor,
      superchatBgColor: _superchatBgColor,
      pollBgColor: _pollBgColor,
      pickupBgColor: _pickupBgColor,
      queueBgColor: _queueBgColor,
      mediaBgColor: _mediaBgColor,
      commentsFontFamily: _commentsFont,
      lotteryFontFamily: _lotteryFont,
      counterFontFamily: _counterFont,
      firstTimeFontFamily: _firstTimeFont,
      superchatFontFamily: _superchatFont,
      pollFontFamily: _pollFont,
      pickupFontFamily: _pickupFont,
      queueFontFamily: _queueFont,
      mediaFontFamily: _mediaFont,
      commentsCustomCss: _commentsCssCtrl.text,
      lotteryCustomCss: _lotteryCssCtrl.text,
      counterCustomCss: _counterCssCtrl.text,
      firstTimeCustomCss: _firstTimeCssCtrl.text,
      superchatCustomCss: _superchatCssCtrl.text,
      pollCustomCss: _pollCssCtrl.text,
      pickupCustomCss: _pickupCssCtrl.text,
      queueCustomCss: _queueCssCtrl.text,
      mediaCustomCss: _mediaCssCtrl.text,
    ));
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final latestCfg = widget.controller.overlayConfig;
    if (latestCfg != _lastSyncedOverlayConfig) {
      _lastSyncedOverlayConfig = latestCfg;
      _mediaBgColor = latestCfg.mediaBgColor;
      _commentsBgColor = latestCfg.commentsBgColor;
      _lotteryBgColor = latestCfg.lotteryBgColor;
      _counterBgColor = latestCfg.counterBgColor;
      _firstTimeBgColor = latestCfg.firstTimeBgColor;
      _superchatBgColor = latestCfg.superchatBgColor;
      _pollBgColor = latestCfg.pollBgColor;
      _pickupBgColor = latestCfg.pickupBgColor;
      _queueBgColor = latestCfg.queueBgColor;
    }
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
                    child: Text(
                      'OBS オーバーレイ ＆ デザインカスタマイズ',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
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
              // タブ下の水平スライドバー ＆ マウススクロールホイール対応タブバー
              Listener(
                onPointerSignal: (event) {
                  if (event is PointerScrollEvent) {
                    final delta = event.scrollDelta.dy != 0
                        ? event.scrollDelta.dy
                        : event.scrollDelta.dx;
                    _scrollTabsBy(delta);
                  }
                },
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    IconButton(
                      tooltip: '左へスクロール',
                      visualDensity: VisualDensity.compact,
                      padding: EdgeInsets.zero,
                      constraints: const BoxConstraints(
                        minWidth: 28,
                        minHeight: 36,
                      ),
                      onPressed: () => _animateScrollTabsBy(-220),
                      icon: Icon(Icons.chevron_left,
                          size: 20, color: p.textSecondary),
                    ),
                    Expanded(
                      child: Scrollbar(
                        controller: _tabScrollCtrl,
                        thumbVisibility: true,
                        trackVisibility: true,
                        interactive: true,
                        thickness: 7,
                        radius: const Radius.circular(4),
                        child: SingleChildScrollView(
                          controller: _tabScrollCtrl,
                          scrollDirection: Axis.horizontal,
                          padding: const EdgeInsets.only(bottom: 12),
                          child: Row(
                            children: List.generate(
                              _overlayTabLabels.length,
                              (idx) {
                                final selected = _tabCtrl.index == idx;
                                return Padding(
                                  padding: const EdgeInsets.only(right: 4),
                                  child: InkWell(
                                    onTap: () => _tabCtrl.animateTo(idx),
                                    borderRadius: const BorderRadius.vertical(
                                      top: Radius.circular(6),
                                    ),
                                    child: Container(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 13,
                                        vertical: 9,
                                      ),
                                      decoration: BoxDecoration(
                                        color: selected
                                            ? p.accent.withValues(alpha: 0.12)
                                            : Colors.transparent,
                                        borderRadius:
                                            const BorderRadius.vertical(
                                          top: Radius.circular(6),
                                        ),
                                        border: Border(
                                          bottom: BorderSide(
                                            color: selected
                                                ? p.accent
                                                : Colors.transparent,
                                            width: 2.5,
                                          ),
                                        ),
                                      ),
                                      child: Text(
                                        _overlayTabLabels[idx],
                                        style: TextStyle(
                                          fontSize: 13,
                                          fontWeight: selected
                                              ? FontWeight.bold
                                              : FontWeight.w500,
                                          color: selected
                                              ? p.accent
                                              : p.textSecondary,
                                        ),
                                      ),
                                    ),
                                  ),
                                );
                              },
                            ),
                          ),
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: '右へスクロール',
                      visualDensity: VisualDensity.compact,
                      padding: EdgeInsets.zero,
                      constraints: const BoxConstraints(
                        minWidth: 28,
                        minHeight: 36,
                      ),
                      onPressed: () => _animateScrollTabsBy(220),
                      icon: Icon(Icons.chevron_right,
                          size: 20, color: p.textSecondary),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
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
                bgColor: _commentsBgColor,
                onBgChanged: (bg) => setState(() => _commentsBgColor = bg),
                fontFamily: _commentsFont,
                onFontChanged: (f) => setState(() => _commentsFont = f),
                cssController: _commentsCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/comments',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.comment-row / .avatar / .bubble / .bubble.highlighted / .bubble.monetization / .author / .message',
                sampleCss:
                    '/* 例: 吹き出しを角張らせて文字に縁取りを付ける */\n.bubble {\n  border-radius: 4px;\n  padding: 12px 18px;\n}\n.message {\n  font-weight: 700;\n  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.75);\n}',
                previewBuilder: (meta, font) =>
                    _buildCommentMiniPreview(meta, font),
              ),
              _buildDesignTab(
                title: 'スパチャ・メンバーシップオーバーレイのデザイン',
                presets: _richOverlayPresets,
                selectedId: _superchatTheme,
                onSelect: (id) => setState(() => _superchatTheme = id),
                bgColor: _superchatBgColor,
                onBgChanged: (bg) => setState(() => _superchatBgColor = bg),
                fontFamily: _superchatFont,
                onFontChanged: (f) => setState(() => _superchatFont = f),
                cssController: _superchatCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/superchat',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.sc-card / .sc-header / .sc-avatar / .sc-name / .sc-amount / .sc-body',
                sampleCss:
                    '/* 例: 金額文字をさらに大きく表示 */\n.sc-amount {\n  font-size: 24px;\n}\n.sc-card {\n  border-radius: 14px;\n}',
                previewBuilder: (meta, font) => _buildGenericMiniPreview(
                  meta,
                  '応援リスナー  ¥1,000',
                  'いつも配信ありがとう！応援してます！',
                  font,
                ),
              ),
              _buildDesignTab(
                title: 'コメントピックアップオーバーレイのデザイン',
                presets: _richOverlayPresets,
                selectedId: _pickupTheme,
                onSelect: (id) => setState(() => _pickupTheme = id),
                bgColor: _pickupBgColor,
                onBgChanged: (bg) => setState(() => _pickupBgColor = bg),
                fontFamily: _pickupFont,
                onFontChanged: (f) => setState(() => _pickupFont = f),
                cssController: _pickupCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/pickup',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.pickup-card / .pickup-head / .pickup-avatar / .pickup-author / .pickup-badge / .pickup-message',
                sampleCss:
                    '/* 例: ピックアップ本文を大きく強調 */\n.pickup-message {\n  font-size: 30px;\n  line-height: 1.55;\n}',
                previewBuilder: (meta, font) => _buildGenericMiniPreview(
                  meta,
                  '質問リスナー（ピックアップ中）',
                  '次のコラボ企画はいつ頃を予定していますか？',
                  font,
                ),
              ),
              _buildDesignTab(
                title: 'リアルタイム投票グラフオーバーレイのデザイン',
                presets: _richOverlayPresets,
                selectedId: _pollTheme,
                onSelect: (id) => setState(() => _pollTheme = id),
                bgColor: _pollBgColor,
                onBgChanged: (bg) => setState(() => _pollBgColor = bg),
                fontFamily: _pollFont,
                onFontChanged: (f) => setState(() => _pollFont = f),
                cssController: _pollCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/poll',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.poll-card / .poll-question / .poll-opt / .poll-opt.leader / .poll-bar / .poll-stats / .poll-footer',
                sampleCss:
                    '/* 例: 1位の選択肢を目立たせる */\n.poll-opt.leader {\n  border-width: 2.5px;\n}',
                previewBuilder: (meta, font) => _buildGenericMiniPreview(
                  meta,
                  '投票受付中: 次の企画はどれがいい？',
                  '1. 雑談トーク (45%)  /  2. 参加型ゲーム (55%)',
                  font,
                ),
              ),
              _buildDesignTab(
                title: '参加型キューオーバーレイのデザイン',
                presets: _richOverlayPresets,
                selectedId: _queueTheme,
                onSelect: (id) => setState(() => _queueTheme = id),
                bgColor: _queueBgColor,
                onBgChanged: (bg) => setState(() => _queueBgColor = bg),
                fontFamily: _queueFont,
                onFontChanged: (f) => setState(() => _queueFont = f),
                cssController: _queueCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/queue',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.q-card / .q-section-title / .q-item / .q-item.active / .q-rank / .q-name / .q-tag',
                sampleCss:
                    '/* 例: 順番待ちボードの幅を広げる */\n.q-card {\n  max-width: 500px;\n}',
                previewBuilder: (meta, font) => _buildGenericMiniPreview(
                  meta,
                  '参加中 (2/3)  ・  待機中 (4人)',
                  '1. たこやき太郎 [初見]   2. さくらもち [1回]',
                  font,
                ),
              ),
              _buildDesignTab(
                title: '特定ワード反応アニメーションオーバーレイのデザイン',
                presets: _richOverlayPresets,
                selectedId: _mediaTheme,
                onSelect: (id) => setState(() => _mediaTheme = id),
                bgColor: _mediaBgColor,
                onBgChanged: (bg) => setState(() => _mediaBgColor = bg),
                fontFamily: _mediaFont,
                onFontChanged: (f) => setState(() => _mediaFont = f),
                cssController: _mediaCssCtrl,
                previewUrl: '${widget.controller.overlayBaseUrl}/overlay/media',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.media-slot / .media-frame / .media-el / .media-caption',
                sampleCss:
                    '/* 例: テロップの文字サイズを大きくする */\n.media-caption {\n  font-size: 26px;\n  padding: 8px 22px;\n}',
                previewBuilder: (meta, font) => _buildGenericMiniPreview(
                  meta,
                  'ワード反応演出（画像 / GIF / 動画）',
                  '指定ワードでアニメーション＋テロップを表示します',
                  font,
                ),
              ),
              _buildDesignTab(
                title: '抽選演出・当選者オーバーレイのデザイン',
                presets: _lotteryPresets,
                selectedId: _lotteryTheme,
                onSelect: (id) => setState(() => _lotteryTheme = id),
                bgColor: _lotteryBgColor,
                onBgChanged: (bg) => setState(() => _lotteryBgColor = bg),
                fontFamily: _lotteryFont,
                onFontChanged: (f) => setState(() => _lotteryFont = f),
                cssController: _lotteryCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/lottery',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.banner / .winner-card / .winner-avatar / .winner-name / .winner-comment / .rolling',
                sampleCss:
                    '/* 例: 当選者名をさらに大きく強調する */\n.winner-card {\n  min-width: 360px;\n}\n.winner-name {\n  font-size: 34px;\n  letter-spacing: 0.04em;\n}',
                previewBuilder: (meta, font) =>
                    _buildLotteryMiniPreview(meta, font),
              ),
              _buildDesignTab(
                title: 'ワードカウンターオーバーレイのデザイン',
                presets: _counterPresets,
                selectedId: _counterTheme,
                onSelect: (id) => setState(() => _counterTheme = id),
                bgColor: _counterBgColor,
                onBgChanged: (bg) => setState(() => _counterBgColor = bg),
                fontFamily: _counterFont,
                onFontChanged: (f) => setState(() => _counterFont = f),
                cssController: _counterCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/counter',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '#counters / .card / .label / .mode-tag / .value / .sub',
                sampleCss:
                    '/* 例: カウンターを縦並びにし、数字を強調する */\n#counters {\n  flex-direction: column;\n}\n.value {\n  font-size: 54px;\n}',
                previewBuilder: (meta, font) =>
                    _buildCounterMiniPreview(meta, '草カウンター', '128', font),
              ),
              _buildDesignTab(
                title: '初見カウンターオーバーレイのデザイン',
                presets: _counterPresets,
                selectedId: _firstTimeTheme,
                onSelect: (id) => setState(() => _firstTimeTheme = id),
                bgColor: _firstTimeBgColor,
                onBgChanged: (bg) => setState(() => _firstTimeBgColor = bg),
                fontFamily: _firstTimeFont,
                onFontChanged: (f) => setState(() => _firstTimeFont = f),
                cssController: _firstTimeCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/first_time',
                obsEnabled: obsEnabled,
                selectorsHelp:
                    '.ft-card / .ft-title / .ft-count / .ft-profiles / .ft-user / .ft-avatar / .ft-user-name',
                sampleCss:
                    '/* 例: 初見カウント数字をさらに大きく表示する */\n.ft-count {\n  font-size: 56px;\n}\n.ft-user {\n  border-radius: 999px;\n}',
                previewBuilder: (meta, font) =>
                    _buildCounterMiniPreview(meta, '本日の初見さん', '5', font),
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
    final superchatUrl = '$base/overlay/superchat';
    final pickupUrl = '$base/overlay/pickup';
    final pollUrl = '$base/overlay/poll';
    final queueUrl = '$base/overlay/queue';
    final mediaUrl = '$base/overlay/media';
    final lotteryUrl = '$base/overlay/lottery';
    final counterUrl = '$base/overlay/counter';
    final firstTimeUrl = '$base/overlay/first_time';

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
            description:
                '推奨サイズ: 幅 480 × 高さ 720。背景色・プリセット・カスタムCSSは「コメント表示 / CSS」タブで調整できます。',
            url: commentsUrl,
            onCopy: () => _copy(commentsUrl, 'コメント表示'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(commentsUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.volunteer_activism_outlined,
            title: 'スーパーチャット・メンバーシップオーバーレイ',
            description:
                '推奨サイズ: 幅 480 × 高さ 640。未読のみ表示する場合は末尾に ?unread_only=1 を付けられます。',
            url: superchatUrl,
            onCopy: () => _copy(superchatUrl, 'スパチャ・メンバー表示'),
            onOpen: obsEnabled
                ? () => widget.controller.openUrl(superchatUrl)
                : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.push_pin_outlined,
            title: 'コメントピックアップ（ピン留め表示）オーバーレイ',
            description:
                '推奨サイズ: 幅 900 × 高さ 260。コメント画面・スパチャ画面でピン留めしたコメントを大きく表示します。',
            url: pickupUrl,
            onCopy: () => _copy(pickupUrl, 'コメントピックアップ'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(pickupUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.poll_outlined,
            title: 'リアルタイム投票・アンケートオーバーレイ',
            description:
                '推奨サイズ: 幅 680 × 高さ 420。視聴者コメントから集計したリアルタイム棒グラフを表示します。',
            url: pollUrl,
            onCopy: () => _copy(pollUrl, 'リアルタイム投票'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(pollUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.queue_play_next,
            title: '参加型キュー（順番待ちリスト）オーバーレイ',
            description:
                '推奨サイズ: 幅 440 × 高さ 600。現在の参加中メンバーと順番待ちリストを表示します。',
            url: queueUrl,
            onCopy: () => _copy(queueUrl, '参加型キュー'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(queueUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.movie_filter_outlined,
            title: '特定ワード反応アニメーション・動画演出オーバーレイ',
            description:
                '推奨サイズ: 幅 1920 × 高さ 1080（配信キャンバスと同サイズ推奨）。画像・GIF・動画を再生します。',
            url: mediaUrl,
            onCopy: () => _copy(mediaUrl, 'ワード反応演出'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(mediaUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.emoji_events_outlined,
            title: '抽選演出・当選者オーバーレイ',
            description:
                '推奨サイズ: 幅 960 × 高さ 540。当選者のアイコン・名前・応募コメントを表示します。',
            url: lotteryUrl,
            onCopy: () => _copy(lotteryUrl, '抽選結果表示'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(lotteryUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.pin_outlined,
            title: '特定ワードカウンターオーバーレイ',
            description:
                '推奨サイズ: 幅 600 × 高さ 240。単体 URL はワードカウント画面からコピーできます。',
            url: counterUrl,
            onCopy: () => _copy(counterUrl, 'ワードカウンター'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(counterUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.waving_hand_outlined,
            title: '初見カウンターオーバーレイ',
            description:
                '推奨サイズ: 幅 380 × 高さ 320。オプションON時は初見さんのアイコンと名前も表示されます。',
            url: firstTimeUrl,
            onCopy: () => _copy(firstTimeUrl, '初見カウンター'),
            onOpen: obsEnabled
                ? () => widget.controller.openUrl(firstTimeUrl)
                : null,
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
    required String bgColor,
    required ValueChanged<String> onBgChanged,
    required String fontFamily,
    required ValueChanged<String> onFontChanged,
    required TextEditingController cssController,
    required String previewUrl,
    required bool obsEnabled,
    required String selectorsHelp,
    required String sampleCss,
    required Widget Function(_PresetMeta meta, String? font) previewBuilder,
  }) {
    final p = context.palette;
    final current = presets.firstWhere(
      (e) => e.id == selectedId,
      orElse: () => presets.first,
    );
    final effectiveFont = fontFamily.trim().isEmpty ? null : fontFamily.trim();
    final isTransparent = bgColor.trim().toLowerCase() == 'transparent';
    final previewCanvasColor = isTransparent
        ? const Color(0xFF1F2429)
        : AppTheme.parseCssHex(bgColor, fallback: const Color(0xFF1F2429));

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

          // 1. プリセット選択
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
                        style: TextStyle(fontSize: 11.5, color: p.textSecondary),
                      ),
                    ],
                  ),
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 16),

          // 2. 背景色の切り替え
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: OverlayBgPicker(
                value: bgColor,
                onChanged: onBgChanged,
              ),
            ),
          ),
          const SizedBox(height: 12),

          // 3. システムフォントの選択
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: SystemFontPicker(
                value: fontFamily,
                availableFonts: widget.controller.systemFonts,
                onChanged: onFontChanged,
                onRefreshFonts: widget.controller.refreshSystemFonts,
              ),
            ),
          ),
          const SizedBox(height: 16),

          // イメージプレビュー（背景色・フォント連動）
          Wrap(
            spacing: 10,
            runSpacing: 4,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              const Text('プレビュー',
                  style:
                      TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5)),
              Text(
                '${isTransparent ? "背景: 透過" : "背景色: $bgColor"}  /  フォント: ${effectiveFont ?? "プリセット標準"}',
                style: TextStyle(
                    fontFamily: 'Consolas',
                    fontSize: 11.5,
                    color: p.textSecondary),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(18),
            decoration: BoxDecoration(
              color: previewCanvasColor,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: p.border),
            ),
            child: previewBuilder(current, effectiveFont),
          ),
          const SizedBox(height: 18),

          // 4. カスタム CSS エディタ
          Row(
            children: [
              const Expanded(
                child: Text(
                  '4. ユーザーカスタム CSS（プリセットの上書き調整）',
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
            maxLines: 8,
            style: const TextStyle(
              fontFamily: 'Consolas',
              fontSize: 13,
              height: 1.45,
            ),
            decoration: const InputDecoration(
              hintText:
                  'ここにカスタム CSS を記述し、「保存して即時反映」を押すと OBS にリアルタイム適用されます...',
            ),
          ),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: _save,
              icon: const Icon(Icons.save_outlined, size: 17),
              label: const Text('デザイン・背景色・CSS を保存して OBS へ即時反映'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCommentMiniPreview(_PresetMeta meta, String? font) {
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
                  fontFamily: font,
                  color: meta.fg.withValues(alpha: 0.8),
                  fontWeight: FontWeight.bold,
                  fontSize: 12,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                'プリセット「${meta.name}」でのコメント表示イメージです！',
                style: TextStyle(fontFamily: font, color: meta.fg, fontSize: 14),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildLotteryMiniPreview(_PresetMeta meta, String? font) {
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
                    fontFamily: font,
                    color: meta.fg,
                    fontSize: 18,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Text(
                  '参加します！当たれ〜！',
                  style: TextStyle(
                    fontFamily: font,
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

  Widget _buildGenericMiniPreview(
      _PresetMeta meta, String heading, String body, String? font) {
    final radius = meta.id == 'pastel'
        ? 18.0
        : (meta.id == 'pop' ? 14.0 : (meta.id == 'minimal' ? 4.0 : 10.0));
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
      decoration: BoxDecoration(
        color: meta.bg,
        borderRadius: BorderRadius.circular(radius),
        border: Border.all(
          color: meta.border,
          width: meta.id == 'pop' ? 2.5 : 1.8,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            heading,
            style: TextStyle(
              fontFamily: font,
              color: meta.border,
              fontWeight: FontWeight.w800,
              fontSize: 13.5,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            body,
            style: TextStyle(
              fontFamily: font,
              color: meta.fg,
              fontWeight: FontWeight.w600,
              fontSize: 14.5,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCounterMiniPreview(
      _PresetMeta meta, String title, String value, String? font) {
    final isCompact = meta.id == 'compact';
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
      decoration: BoxDecoration(
        color: meta.bg,
        borderRadius: BorderRadius.circular(
            isCompact ? 999 : (meta.id == 'pastel' ? 18 : 8)),
        border: Border.all(color: meta.border, width: 2),
      ),
      child: isCompact
          ? Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(title,
                    style: TextStyle(
                        fontFamily: font,
                        color: meta.fg,
                        fontWeight: FontWeight.bold)),
                const SizedBox(width: 12),
                Text(value,
                    style: TextStyle(
                        fontFamily: font,
                        color: meta.border,
                        fontSize: 24,
                        fontWeight: FontWeight.w900)),
              ],
            )
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(title,
                    style: TextStyle(
                        fontFamily: font,
                        color: meta.fg.withValues(alpha: 0.85),
                        fontSize: 12.5,
                        fontWeight: FontWeight.bold)),
                Text(
                  value,
                  style: TextStyle(
                    color: meta.border,
                    fontSize: 36,
                    fontWeight: FontWeight.w900,
                    fontFamily:
                        font ?? (meta.id == 'scoreboard' ? 'Consolas' : null),
                  ),
                ),
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
