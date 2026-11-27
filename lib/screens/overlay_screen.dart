import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';
import '../widgets/feature_toggle_banner.dart';
import '../widgets/overlay_bg_picker.dart';
import '../widgets/overlay_visual_style_editor.dart';
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
    'トークモニター / CSS',
    'エールボックス / CSS',
    'スポットテロップ / CSS',
    'ライブアンケート / CSS',
    'エントリーレーン / CSS',
    'リアクションステージ / CSS',
    'ドローボックス / CSS',
    'コールカウンター / CSS',
    'ウェルカムカウンター / CSS',
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
  late Map<String, OverlayVisualStyle> _visualStyles;
  late OverlayConfig _lastSyncedOverlayConfig;

  static const List<String> _allOverlayKeys = [
    'comments',
    'superchat',
    'pickup',
    'poll',
    'queue',
    'media',
    'lottery',
    'counter',
    'first_time',
  ];

  static String _normalizeThemeId(String raw) {
    switch (raw.trim().toLowerCase()) {
      case 'editorial':
      case 'pastel':
      case 'cute':
        return 'editorial';
      case 'tactical':
      case 'neon':
      case 'scoreboard':
        return 'tactical';
      case 'variety':
      case 'pop':
      case 'festive':
        return 'variety';
      case 'cinema':
      case 'minimal':
      case 'compact':
      case 'japanese':
        return 'cinema';
      case 'edge_glow':
      case 'luminous':
      case 'glow':
        return 'edge_glow';
      default:
        return 'broadcast';
    }
  }

  static const List<_PresetMeta> _studioPresets = [
    _PresetMeta(
      id: 'broadcast',
      name: 'オンエア・テロップ',
      desc: 'TV 情報番組・公式放送のローワーサード風。墨色面＋シャープなアクセント。',
      bg: Color(0xFF111317),
      fg: Color(0xFFF7F8FA),
      border: Color(0xFFD93829),
    ),
    _PresetMeta(
      id: 'editorial',
      name: 'クリーン・マガジン',
      desc: '生成り紙色＋濃墨＋極細罫線。雑談・お絵描き・ラジオ配信に映える上質組版。',
      bg: Color(0xFFF8F5EF),
      fg: Color(0xFF1C1B18),
      border: Color(0xFFC2593F),
    ),
    _PresetMeta(
      id: 'tactical',
      name: 'タクティカル・HUD',
      desc: '斜めカット＋等幅数値＋計器アンバー。競技 e-sports・ゲーム配信向け計器 UI。',
      bg: Color(0xFF121417),
      fg: Color(0xFFEDEBE4),
      border: Color(0xFFE5A93B),
    ),
    _PresetMeta(
      id: 'variety',
      name: 'バラエティ・テロップ',
      desc: '日本のバラエティ番組・実況向け。高コントラスト濃紺枠＋ハードシャドウ。',
      bg: Color(0xFFFFFDF7),
      fg: Color(0xFF182033),
      border: Color(0xFFF5C518),
    ),
    _PresetMeta(
      id: 'cinema',
      name: 'シネマ・クリア字幕',
      desc: '背景箱を排し、美しい袋文字＋自然な落ち影だけで映像を邪魔しない字幕設計。',
      bg: Color(0x55101216),
      fg: Color(0xFFFFFFFF),
      border: Color(0xFFF5C518),
    ),
    _PresetMeta(
      id: 'edge_glow',
      name: 'エッジ・グロウ',
      desc: '深淵スモーク面＋高輝度テキストエッジ発光。暗いゲーム・歌枠で文字輪郭がくっきり輝く設計。',
      bg: Color(0xC8090D16),
      fg: Color(0xFFF8FAFC),
      border: Color(0xFF38BDF8),
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
    _commentsTheme = _normalizeThemeId(cfg.commentsTheme);
    _lotteryTheme = _normalizeThemeId(cfg.lotteryTheme);
    _counterTheme = _normalizeThemeId(cfg.counterTheme);
    _firstTimeTheme = _normalizeThemeId(cfg.firstTimeTheme);
    _superchatTheme = _normalizeThemeId(cfg.superchatTheme);
    _pollTheme = _normalizeThemeId(cfg.pollTheme);
    _pickupTheme = _normalizeThemeId(cfg.pickupTheme);
    _queueTheme = _normalizeThemeId(cfg.queueTheme);
    _mediaTheme = _normalizeThemeId(cfg.mediaTheme);
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
    _visualStyles = Map<String, OverlayVisualStyle>.from(cfg.visualStyles);
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
      visualStyles: Map<String, OverlayVisualStyle>.from(_visualStyles),
    ));
  }

  OverlayVisualStyle _styleFor(String key) =>
      _visualStyles[key] ?? defaultVisualStyle();

  void _copyStyleToAll(OverlayVisualStyle source) {
    setState(() {
      for (final k in _allOverlayKeys) {
        _visualStyles[k] = source;
      }
    });
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('現在のビジュアルスタイルを全 9 オーバーレイにコピーしました（「保存して即時反映」で適用されます）'),
      ),
    );
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
      _commentsTheme = _normalizeThemeId(latestCfg.commentsTheme);
      _lotteryTheme = _normalizeThemeId(latestCfg.lotteryTheme);
      _counterTheme = _normalizeThemeId(latestCfg.counterTheme);
      _firstTimeTheme = _normalizeThemeId(latestCfg.firstTimeTheme);
      _superchatTheme = _normalizeThemeId(latestCfg.superchatTheme);
      _pollTheme = _normalizeThemeId(latestCfg.pollTheme);
      _pickupTheme = _normalizeThemeId(latestCfg.pickupTheme);
      _queueTheme = _normalizeThemeId(latestCfg.queueTheme);
      _mediaTheme = _normalizeThemeId(latestCfg.mediaTheme);
      _visualStyles =
          Map<String, OverlayVisualStyle>.from(latestCfg.visualStyles);
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
                      'オーバーレイスタジオ（OBS ＆ デザインカスタマイズ）',
                      style:
                          TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                    ),
                  ),
                  FeatureToggleSwitch(
                    label: 'オーバーレイスタジオ',
                    enabled: obsEnabled,
                    onChanged: (v) => widget.controller.setFeatureEnabled(
                      obsOverlay: v,
                      featureLabel: 'オーバーレイスタジオ（OBS サーバー）',
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
                overlayKey: 'comments',
                title: 'トークモニターオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _commentsTheme,
                onSelect: (id) => setState(() => _commentsTheme = id),
                bgColor: _commentsBgColor,
                onBgChanged: (bg) => setState(() => _commentsBgColor = bg),
                fontFamily: _commentsFont,
                onFontChanged: (f) => setState(() => _commentsFont = f),
                cssController: _commentsCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/comments',
                obsEnabled: obsEnabled,
                sampleAuthor: 'スタジオ視聴者',
                sampleText: '縁取りやフォント、背景の透過具合をここで調整できます！',
                selectorsHelp:
                    '.comment-row / .avatar / .bubble / .bubble.monetization / .author / .message',
                sampleCss:
                    '/* 例: 左アクセントラインの色と幅を細かく調整 */\n.bubble {\n  border-left-width: 6px;\n}\n.message {\n  font-weight: 700;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'superchat',
                title: 'エールボックスオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _superchatTheme,
                onSelect: (id) => setState(() => _superchatTheme = id),
                bgColor: _superchatBgColor,
                onBgChanged: (bg) => setState(() => _superchatBgColor = bg),
                fontFamily: _superchatFont,
                onFontChanged: (f) => setState(() => _superchatFont = f),
                cssController: _superchatCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/superchat',
                obsEnabled: obsEnabled,
                sampleAuthor: '応援リスナー  ¥1,000',
                sampleText: 'いつも配信ありがとう！今日も楽しみにしてます！',
                selectorsHelp:
                    '.sc-card / .sc-header / .sc-avatar / .sc-name / .sc-amount / .sc-body',
                sampleCss:
                    '/* 例: 金額表示をさらに大きく強調 */\n.sc-amount {\n  font-size: 24px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'pickup',
                title: 'スポットテロップオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _pickupTheme,
                onSelect: (id) => setState(() => _pickupTheme = id),
                bgColor: _pickupBgColor,
                onBgChanged: (bg) => setState(() => _pickupBgColor = bg),
                fontFamily: _pickupFont,
                onFontChanged: (f) => setState(() => _pickupFont = f),
                cssController: _pickupCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/pickup',
                obsEnabled: obsEnabled,
                sampleAuthor: 'たこやき太郎（スポット表示中）',
                sampleText: '次のコラボ企画はいつ頃を予定していますか？',
                selectorsHelp:
                    '.pickup-shell / .pickup-plate / .pickup-card / .pickup-avatar / .pickup-author / .pickup-message',
                sampleCss:
                    '/* 例: テロップ本文を大きく強調 */\n.pickup-message {\n  font-size: 32px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'poll',
                title: 'ライブアンケートオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _pollTheme,
                onSelect: (id) => setState(() => _pollTheme = id),
                bgColor: _pollBgColor,
                onBgChanged: (bg) => setState(() => _pollBgColor = bg),
                fontFamily: _pollFont,
                onFontChanged: (f) => setState(() => _pollFont = f),
                cssController: _pollCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/poll',
                obsEnabled: obsEnabled,
                sampleAuthor: 'LIVE 受付中: 次の企画はどれがいい？',
                sampleText: '1. 雑談トーク (48%)  /  2. 参加型ゲーム (52%)',
                selectorsHelp:
                    '.poll-card / .poll-question / .poll-opt / .poll-opt.leader / .poll-bar / .poll-stats / .poll-footer',
                sampleCss:
                    '/* 例: 1位の選択肢を目立たせる */\n.poll-opt.leader {\n  border-width: 2.5px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'queue',
                title: 'エントリーレーンオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _queueTheme,
                onSelect: (id) => setState(() => _queueTheme = id),
                bgColor: _queueBgColor,
                onBgChanged: (bg) => setState(() => _queueBgColor = bg),
                fontFamily: _queueFont,
                onFontChanged: (f) => setState(() => _queueFont = f),
                cssController: _queueCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/queue',
                obsEnabled: obsEnabled,
                sampleAuthor: 'PLAYING 2/3  •  NEXT 4人待機中',
                sampleText: '1. たこやき太郎 [初見]   2. さくらもち [1回]',
                selectorsHelp:
                    '.q-card / .q-section-title / .q-item / .q-item.active / .q-rank / .q-name / .q-tag',
                sampleCss:
                    '/* 例: 順番待ちボードの幅を広げる */\n.q-card {\n  max-width: 500px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'media',
                title: 'リアクションステージオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _mediaTheme,
                onSelect: (id) => setState(() => _mediaTheme = id),
                bgColor: _mediaBgColor,
                onBgChanged: (bg) => setState(() => _mediaBgColor = bg),
                fontFamily: _mediaFont,
                onFontChanged: (f) => setState(() => _mediaFont = f),
                cssController: _mediaCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/media',
                obsEnabled: obsEnabled,
                sampleAuthor: 'リアクションステージ テロップ',
                sampleText: 'たこやき太郎 さんありがとう！！',
                selectorsHelp:
                    '.media-slot / .media-frame / .media-el / .media-caption',
                sampleCss:
                    '/* 例: テロップの文字サイズを大きくする */\n.media-caption {\n  font-size: 28px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'lottery',
                title: 'ドローボックスオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _lotteryTheme,
                onSelect: (id) => setState(() => _lotteryTheme = id),
                bgColor: _lotteryBgColor,
                onBgChanged: (bg) => setState(() => _lotteryBgColor = bg),
                fontFamily: _lotteryFont,
                onFontChanged: (f) => setState(() => _lotteryFont = f),
                cssController: _lotteryCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/lottery',
                obsEnabled: obsEnabled,
                sampleAuthor: 'DRAW WINNER（候補 24 名）',
                sampleText: 'ラッキー視聴者「参加します！当たりますように！」',
                selectorsHelp:
                    '.banner / .winner-card / .winner-avatar / .winner-name / .winner-comment / .rolling',
                sampleCss:
                    '/* 例: 当選者名をさらに大きく強調 */\n.winner-name {\n  font-size: 34px;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'counter',
                title: 'コールカウンターオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _counterTheme,
                onSelect: (id) => setState(() => _counterTheme = id),
                bgColor: _counterBgColor,
                onBgChanged: (bg) => setState(() => _counterBgColor = bg),
                fontFamily: _counterFont,
                onFontChanged: (f) => setState(() => _counterFont = f),
                cssController: _counterCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/counter',
                obsEnabled: obsEnabled,
                sampleAuthor: '草カウンター（上限なし）',
                sampleText: '128',
                selectorsHelp:
                    '#counters / .card / .label / .mode-tag / .value / .sub',
                sampleCss:
                    '/* 例: カウンターを縦並びにする */\n#counters {\n  flex-direction: column;\n}',
              ),
              _buildDesignTab(
                overlayKey: 'first_time',
                title: 'ウェルカムカウンターオーバーレイのデザイン',
                presets: _studioPresets,
                selectedId: _firstTimeTheme,
                onSelect: (id) => setState(() => _firstTimeTheme = id),
                bgColor: _firstTimeBgColor,
                onBgChanged: (bg) => setState(() => _firstTimeBgColor = bg),
                fontFamily: _firstTimeFont,
                onFontChanged: (f) => setState(() => _firstTimeFont = f),
                cssController: _firstTimeCssCtrl,
                previewUrl:
                    '${widget.controller.overlayBaseUrl}/overlay/preview/first_time',
                obsEnabled: obsEnabled,
                sampleAuthor: 'WELCOME — 本日の初見さん',
                sampleText: '12',
                selectorsHelp:
                    '.ft-card / .ft-title / .ft-count / .ft-profiles / .ft-user / .ft-avatar / .ft-user-name',
                sampleCss:
                    '/* 例: 初見カウント数字をさらに大きく表示 */\n.ft-count {\n  font-size: 56px;\n}',
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
              featureName: 'オーバーレイスタジオ（OBS サーバー）',
              description:
                  'OBS オーバーレイ配信を一時停止しています。OBS で表示する際はオンにしてください。',
              onEnable: () => widget.controller.setFeatureEnabled(
                obsOverlay: true,
                featureLabel: 'オーバーレイスタジオ（OBS サーバー）',
              ),
            ),
          _OverlayUrlCard(
            icon: Icons.chat_bubble_outline,
            title: 'トークモニター オーバーレイ（コメント表示）',
            description:
                '推奨サイズ: 幅 480 × 高さ 720。背景色・プリセット・カスタムCSSは「トークモニター / CSS」タブで調整できます。',
            url: commentsUrl,
            onCopy: () => _copy(commentsUrl, 'トークモニター'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(commentsUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.volunteer_activism_outlined,
            title: 'エールボックス オーバーレイ（スパチャ・メンバー表示）',
            description:
                '推奨サイズ: 幅 480 × 高さ 640。未読のみ表示する場合は末尾に ?unread_only=1 を付けられます。',
            url: superchatUrl,
            onCopy: () => _copy(superchatUrl, 'エールボックス'),
            onOpen: obsEnabled
                ? () => widget.controller.openUrl(superchatUrl)
                : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.push_pin_outlined,
            title: 'スポットテロップ オーバーレイ（ピン留め表示）',
            description:
                '推奨サイズ: 幅 900 × 高さ 260。トークモニター・エールボックスでピン留めしたコメントを大きく表示します。',
            url: pickupUrl,
            onCopy: () => _copy(pickupUrl, 'スポットテロップ'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(pickupUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.poll_outlined,
            title: 'ライブアンケート オーバーレイ（リアルタイム投票）',
            description:
                '推奨サイズ: 幅 680 × 高さ 420。視聴者コメントから集計したリアルタイム棒グラフを表示します。',
            url: pollUrl,
            onCopy: () => _copy(pollUrl, 'ライブアンケート'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(pollUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.queue_play_next,
            title: 'エントリーレーン オーバーレイ（参加型順番待ち）',
            description:
                '推奨サイズ: 幅 440 × 高さ 600。現在の参加中メンバーと順番待ちリストを表示します。',
            url: queueUrl,
            onCopy: () => _copy(queueUrl, 'エントリーレーン'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(queueUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.movie_filter_outlined,
            title: 'リアクションステージ オーバーレイ（演出・動画）',
            description:
                '推奨サイズ: 幅 1920 × 高さ 1080（配信キャンバスと同サイズ推奨）。パーティクル・画像・GIF・動画を再生します。',
            url: mediaUrl,
            onCopy: () => _copy(mediaUrl, 'リアクションステージ'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(mediaUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.emoji_events_outlined,
            title: 'ドローボックス オーバーレイ（抽選演出・当選者）',
            description:
                '推奨サイズ: 幅 960 × 高さ 540。当選者のアイコン・名前・応募コメントを表示します。',
            url: lotteryUrl,
            onCopy: () => _copy(lotteryUrl, 'ドローボックス'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(lotteryUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.pin_outlined,
            title: 'コールカウンター オーバーレイ（特定ワード集計）',
            description:
                '推奨サイズ: 幅 600 × 高さ 240。単体 URL はコールカウンター画面からコピーできます。',
            url: counterUrl,
            onCopy: () => _copy(counterUrl, 'コールカウンター'),
            onOpen:
                obsEnabled ? () => widget.controller.openUrl(counterUrl) : null,
          ),
          const SizedBox(height: 10),
          _OverlayUrlCard(
            icon: Icons.waving_hand_outlined,
            title: 'ウェルカムカウンター オーバーレイ（初見さん検知）',
            description:
                '推奨サイズ: 幅 380 × 高さ 320。オプションON時は初見さんのアイコンと名前も表示されます。',
            url: firstTimeUrl,
            onCopy: () => _copy(firstTimeUrl, 'ウェルカムカウンター'),
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
    required String overlayKey,
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
    required String sampleAuthor,
    required String sampleText,
    required String selectorsHelp,
    required String sampleCss,
  }) {
    final p = context.palette;
    final current = presets.firstWhere(
      (e) => e.id == selectedId,
      orElse: () => presets.first,
    );
    final effectiveFont = fontFamily.trim().isEmpty ? null : fontFamily.trim();
    final visualStyle = _styleFor(overlayKey);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 10,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text(
                title,
                style: const TextStyle(
                    fontSize: 16.5, fontWeight: FontWeight.bold),
              ),
              Tooltip(
                message:
                    '本番の /overlay/$overlayKey には影響せず、/overlay/preview/$overlayKey 専用環境のみへテストデータを送信します',
                child: OutlinedButton.icon(
                  onPressed: obsEnabled
                      ? () => widget.controller.sendOverlayTestEvent(overlayKey)
                      : null,
                  icon: const Icon(Icons.play_circle_outline, size: 16),
                  label: const Text('プレビューへテスト送信'),
                ),
              ),
              Tooltip(
                message: '本番データに影響しないテスト専用 URL ($previewUrl) をブラウザで開きます',
                child: OutlinedButton.icon(
                  onPressed: obsEnabled
                      ? () => widget.controller.openUrl(previewUrl)
                      : null,
                  icon: const Icon(Icons.open_in_new, size: 16),
                  label: const Text('ブラウザで実寸プレビュー'),
                ),
              ),
              Tooltip(
                message: 'OBS のテスト用シーンなどに貼れるプレビュー専用 URL をコピーします',
                child: TextButton.icon(
                  onPressed: () => _copy(previewUrl, '$title（プレビュー専用）'),
                  icon: const Icon(Icons.copy, size: 15),
                  label: const Text('プレビュー URL をコピー'),
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: p.subtleBg.withValues(alpha: 0.65),
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: p.border),
            ),
            child: Wrap(
              spacing: 8,
              runSpacing: 4,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Icon(Icons.shield_outlined, size: 15, color: p.accent),
                Text(
                  '本番非干渉プレビュー環境:',
                  style: TextStyle(
                    fontSize: 11.5,
                    fontWeight: FontWeight.bold,
                    color: p.textSecondary,
                  ),
                ),
                SelectableText(
                  previewUrl,
                  style: TextStyle(
                    fontFamily: 'Consolas',
                    fontSize: 11.5,
                    color: p.accent,
                  ),
                ),
                Text(
                  '（テスト送信はこの環境のみに反映され、本番配信・データには一切影響しません）',
                  style: TextStyle(fontSize: 11.5, color: p.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(height: 14),

          // 1. プリセット選択（脱AIデザイン 6系統）
          const Text('1. ベースデザイン（6 系統）を選択',
              style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5)),
          const SizedBox(height: 8),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: presets.map((meta) {
              final selected = meta.id == selectedId;
              return InkWell(
                onTap: () => onSelect(meta.id),
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  width: 215,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: p.cardBg,
                    borderRadius: BorderRadius.circular(6),
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
                              borderRadius: BorderRadius.circular(3),
                              border: Border.all(color: meta.border, width: 2.5),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              meta.name,
                              style: const TextStyle(
                                  fontWeight: FontWeight.bold, fontSize: 13),
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

          // 2. GUI ビジュアルスタイル＆テキスト装飾エディタ（リアルタイムプレビュー付き）
          OverlayVisualStyleEditor(
            style: visualStyle,
            presetId: current.id,
            presetName: current.name,
            presetBg: current.bg,
            presetFg: current.fg,
            presetAccent: current.border,
            fontFamily: effectiveFont,
            sampleAuthor: sampleAuthor,
            sampleText: sampleText,
            onChanged: (next) => setState(() => _visualStyles[overlayKey] = next),
            onCopyToAll: () => _copyStyleToAll(visualStyle),
          ),
          const SizedBox(height: 14),

          // 3. 背景色の切り替え
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

          // 4. システムフォントの選択
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

          // 5. ユーザーカスタム CSS（上級者向け）
          Card(
            child: ExpansionTile(
              initiallyExpanded: cssController.text.trim().isNotEmpty,
              leading: Icon(Icons.code, size: 19, color: p.accent),
              title: const Text(
                '5. 上級者向けカスタム CSS（GUI スタイルの上からさらに上書き）',
                style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13.5),
              ),
              childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              children: [
                Row(
                  children: [
                    const Spacer(),
                    TextButton.icon(
                      onPressed: () {
                        setState(() {
                          if (cssController.text.trim().isEmpty) {
                            cssController.text = sampleCss;
                          } else {
                            cssController.text =
                                '${cssController.text}\n\n$sampleCss';
                          }
                        });
                      },
                      icon: const Icon(Icons.code, size: 15),
                      label: const Text('サンプル CSS を挿入'),
                    ),
                    const SizedBox(width: 6),
                    TextButton.icon(
                      onPressed: () => setState(() => cssController.clear()),
                      icon: const Icon(Icons.clear, size: 15),
                      label: const Text('CSS をクリア'),
                    ),
                  ],
                ),
                Container(
                  width: double.infinity,
                  padding:
                      const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
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
                  maxLines: 7,
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
              ],
            ),
          ),
          const SizedBox(height: 14),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: _save,
              icon: const Icon(Icons.save_outlined, size: 17),
              label: const Text('デザイン・テキスト装飾・CSS を保存して OBS へ即時反映'),
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
