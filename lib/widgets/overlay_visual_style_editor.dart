import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../theme/app_theme.dart';
import 'color_picker_row.dart';

/// デフォルト（プリセット準拠）の OverlayVisualStyle を生成
OverlayVisualStyle defaultVisualStyle() => const OverlayVisualStyle(
      enabled: false,
      fontSize: 0,
      fontWeight: 0,
      letterSpacingTenths: 0,
      lineHeightPercent: 0,
      textColor: '',
      useTextGradient: false,
      textGradientStart: '#FFFFFF',
      textGradientEnd: '#FDE047',
      textGradientAngle: 180,
      authorColor: '',
      accentColor: '',
      strokeWidth: 0,
      strokeColor: '#111114',
      outerStrokeWidth: 0,
      outerStrokeColor: '#FFFFFF',
      textShadowEnabled: false,
      textShadowColor: '#000000',
      textShadowBlur: 6,
      textShadowOffsetX: 2,
      textShadowOffsetY: 2,
      textGlowEnabled: false,
      textGlowColor: '#38BDF8',
      textGlowRadius: 12,
      textGlowIntensity: 85,
      textGlowSpread: 2,
      cardBgColor: '',
      cardBgOpacity: 92,
      borderRadius: -1,
      borderWidth: -1,
      borderColor: '',
      cardShadow: 'inherit',
      paddingScale: 'inherit',
      entranceAnimation: 'inherit',
    );

OverlayVisualStyle copyVisualStyle(
  OverlayVisualStyle s, {
  bool? enabled,
  int? fontSize,
  int? fontWeight,
  int? letterSpacingTenths,
  int? lineHeightPercent,
  String? textColor,
  bool? useTextGradient,
  String? textGradientStart,
  String? textGradientEnd,
  int? textGradientAngle,
  String? authorColor,
  String? accentColor,
  int? strokeWidth,
  String? strokeColor,
  int? outerStrokeWidth,
  String? outerStrokeColor,
  bool? textShadowEnabled,
  String? textShadowColor,
  int? textShadowBlur,
  int? textShadowOffsetX,
  int? textShadowOffsetY,
  bool? textGlowEnabled,
  String? textGlowColor,
  int? textGlowRadius,
  int? textGlowIntensity,
  int? textGlowSpread,
  String? cardBgColor,
  int? cardBgOpacity,
  int? borderRadius,
  int? borderWidth,
  String? borderColor,
  String? cardShadow,
  String? paddingScale,
  String? entranceAnimation,
}) {
  return OverlayVisualStyle(
    enabled: enabled ?? s.enabled,
    fontSize: fontSize ?? s.fontSize,
    fontWeight: fontWeight ?? s.fontWeight,
    letterSpacingTenths: letterSpacingTenths ?? s.letterSpacingTenths,
    lineHeightPercent: lineHeightPercent ?? s.lineHeightPercent,
    textColor: textColor ?? s.textColor,
    useTextGradient: useTextGradient ?? s.useTextGradient,
    textGradientStart: textGradientStart ?? s.textGradientStart,
    textGradientEnd: textGradientEnd ?? s.textGradientEnd,
    textGradientAngle: textGradientAngle ?? s.textGradientAngle,
    authorColor: authorColor ?? s.authorColor,
    accentColor: accentColor ?? s.accentColor,
    strokeWidth: strokeWidth ?? s.strokeWidth,
    strokeColor: strokeColor ?? s.strokeColor,
    outerStrokeWidth: outerStrokeWidth ?? s.outerStrokeWidth,
    outerStrokeColor: outerStrokeColor ?? s.outerStrokeColor,
    textShadowEnabled: textShadowEnabled ?? s.textShadowEnabled,
    textShadowColor: textShadowColor ?? s.textShadowColor,
    textShadowBlur: textShadowBlur ?? s.textShadowBlur,
    textShadowOffsetX: textShadowOffsetX ?? s.textShadowOffsetX,
    textShadowOffsetY: textShadowOffsetY ?? s.textShadowOffsetY,
    textGlowEnabled: textGlowEnabled ?? s.textGlowEnabled,
    textGlowColor: textGlowColor ?? s.textGlowColor,
    textGlowRadius: textGlowRadius ?? s.textGlowRadius,
    textGlowIntensity: textGlowIntensity ?? s.textGlowIntensity,
    textGlowSpread: textGlowSpread ?? s.textGlowSpread,
    cardBgColor: cardBgColor ?? s.cardBgColor,
    cardBgOpacity: cardBgOpacity ?? s.cardBgOpacity,
    borderRadius: borderRadius ?? s.borderRadius,
    borderWidth: borderWidth ?? s.borderWidth,
    borderColor: borderColor ?? s.borderColor,
    cardShadow: cardShadow ?? s.cardShadow,
    paddingScale: paddingScale ?? s.paddingScale,
    entranceAnimation: entranceAnimation ?? s.entranceAnimation,
  );
}

/// CSS なしで高度なデザイン・テキスト装飾（袋文字・二重縁取り・グラデーション文字・カード装飾）を行える GUI エディタ
class OverlayVisualStyleEditor extends StatefulWidget {
  const OverlayVisualStyleEditor({
    super.key,
    required this.style,
    this.presetId = 'broadcast',
    required this.presetName,
    required this.presetBg,
    required this.presetFg,
    required this.presetAccent,
    required this.fontFamily,
    required this.sampleAuthor,
    required this.sampleText,
    required this.onChanged,
    required this.onCopyToAll,
  });

  final OverlayVisualStyle style;
  final String presetId;
  final String presetName;
  final Color presetBg;
  final Color presetFg;
  final Color presetAccent;
  final String? fontFamily;
  final String sampleAuthor;
  final String sampleText;
  final ValueChanged<OverlayVisualStyle> onChanged;
  final VoidCallback onCopyToAll;

  @override
  State<OverlayVisualStyleEditor> createState() =>
      _OverlayVisualStyleEditorState();
}

class _OverlayVisualStyleEditorState extends State<OverlayVisualStyleEditor> {
  String _previewStage = 'checker'; // checker | dark | light

  void _update(OverlayVisualStyle next) {
    widget.onChanged(copyVisualStyle(next, enabled: true));
  }

  void _applyRecipe(String recipe) {
    switch (recipe) {
      case 'tv_subtitle':
        widget.onChanged(
          copyVisualStyle(
            defaultVisualStyle(),
            enabled: true,
            fontSize: 22,
            fontWeight: 800,
            textColor: '#FFFFFF',
            strokeWidth: 3,
            strokeColor: '#111317',
            textShadowEnabled: true,
            textShadowBlur: 6,
            textShadowOffsetX: 0,
            textShadowOffsetY: 2,
            cardBgColor: '#111317',
            cardBgOpacity: 0,
            borderWidth: 0,
            cardShadow: 'none',
            entranceAnimation: 'wipe',
          ),
        );
      case 'variety_double':
        widget.onChanged(
          copyVisualStyle(
            defaultVisualStyle(),
            enabled: true,
            fontSize: 24,
            fontWeight: 900,
            useTextGradient: true,
            textGradientStart: '#FFF9C4',
            textGradientEnd: '#F59E0B',
            textGradientAngle: 180,
            strokeWidth: 3,
            strokeColor: '#182033',
            outerStrokeWidth: 3,
            outerStrokeColor: '#FFFFFF',
            textShadowEnabled: true,
            textShadowBlur: 4,
            textShadowOffsetX: 2,
            textShadowOffsetY: 3,
            cardBgColor: '#182033',
            cardBgOpacity: 0,
            borderWidth: 0,
            cardShadow: 'none',
            entranceAnimation: 'pop',
          ),
        );
      case 'editorial_card':
        widget.onChanged(
          copyVisualStyle(
            defaultVisualStyle(),
            enabled: true,
            fontSize: 18,
            fontWeight: 600,
            letterSpacingTenths: 4,
            lineHeightPercent: 155,
            textColor: '#1C1B18',
            authorColor: '#C2593F',
            accentColor: '#C2593F',
            cardBgColor: '#F8F5EF',
            cardBgOpacity: 96,
            borderRadius: 3,
            borderWidth: 1,
            borderColor: '#D4CDC0',
            cardShadow: 'soft',
            paddingScale: 'relaxed',
            entranceAnimation: 'fade',
          ),
        );
      case 'hard_pop':
        widget.onChanged(
          copyVisualStyle(
            defaultVisualStyle(),
            enabled: true,
            fontSize: 20,
            fontWeight: 800,
            textColor: '#182033',
            authorColor: '#D93829',
            accentColor: '#F5C518',
            cardBgColor: '#FFFDF7',
            cardBgOpacity: 100,
            borderRadius: 10,
            borderWidth: 3,
            borderColor: '#182033',
            cardShadow: 'hard',
            paddingScale: 'normal',
            entranceAnimation: 'pop',
          ),
        );
      case 'edge_glow':
        widget.onChanged(
          copyVisualStyle(
            defaultVisualStyle(),
            enabled: true,
            fontSize: 21,
            fontWeight: 800,
            textColor: '#F8FAFC',
            authorColor: '#7DD3FC',
            accentColor: '#38BDF8',
            strokeWidth: 1,
            strokeColor: '#0F172A',
            textGlowEnabled: true,
            textGlowColor: '#38BDF8',
            textGlowRadius: 14,
            textGlowIntensity: 115,
            textGlowSpread: 2,
            cardBgColor: '#0B1120',
            cardBgOpacity: 84,
            borderRadius: 6,
            borderWidth: 1,
            borderColor: '#38BDF8',
            cardShadow: 'soft',
            entranceAnimation: 'fade',
          ),
        );
      default:
        widget.onChanged(defaultVisualStyle());
    }
  }

  FontWeight _mapWeight(int w) {
    if (w <= 0) return FontWeight.w700;
    if (w <= 350) return FontWeight.w300;
    if (w <= 450) return FontWeight.w400;
    if (w <= 550) return FontWeight.w500;
    if (w <= 650) return FontWeight.w600;
    if (w <= 750) return FontWeight.w700;
    if (w <= 850) return FontWeight.w800;
    return FontWeight.w900;
  }

  String _summaryA(OverlayVisualStyle s) {
    final sizeStr = s.fontSize == 0 ? 'サイズ標準' : '${s.fontSize}px';
    final weightStr = s.fontWeight == 0 ? '太さ標準' : 'Weight ${s.fontWeight}';
    final colorStr = s.useTextGradient ? '2色グラデーション' : '単色';
    return '$sizeStr ・ $weightStr ・ $colorStr';
  }

  String _summaryB(OverlayVisualStyle s) {
    final inner = s.strokeWidth == 0 ? '内側フチなし' : '内側フチ ${s.strokeWidth}px';
    final outer =
        s.outerStrokeWidth == 0 ? '二重フチなし' : '二重フチ ${s.outerStrokeWidth}px';
    final glow = s.textGlowEnabled
        ? 'エッジ発光 ${s.textGlowRadius}px/${s.textGlowIntensity}%'
        : 'エッジ発光 OFF';
    final shadow = s.textShadowEnabled ? '文字影 ON' : '文字影 OFF';
    return '$inner ・ $outer ・ $glow ・ $shadow';
  }

  String _summaryC(OverlayVisualStyle s) {
    final radius = s.borderRadius < 0 ? '角丸標準' : '角丸 ${s.borderRadius}px';
    final animLabel = switch (s.entranceAnimation) {
      'wipe' => 'ワイプ',
      'slide' => 'スライド',
      'fade' => 'フェード',
      'pop' => 'ポップ',
      'none' => 'なし',
      _ => '標準',
    };
    return '不透明度 ${s.cardBgOpacity}% ・ $radius ・ モーション: $animLabel';
  }

  Widget _buildCategoryDropdown({
    required StosupPalette p,
    required IconData icon,
    required String title,
    required String summary,
    required List<Widget> children,
  }) {
    return Material(
      color: p.subtleBg.withValues(alpha: 0.42),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: p.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: Theme(
        data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
        child: ExpansionTile(
          initiallyExpanded: false,
          tilePadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
          childrenPadding: const EdgeInsets.fromLTRB(14, 4, 14, 14),
          expandedCrossAxisAlignment: CrossAxisAlignment.start,
          expandedAlignment: Alignment.centerLeft,
          leading: Icon(icon, size: 19, color: p.accent),
          title: Text(
            title,
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
          ),
          subtitle: Text(
            summary,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(fontSize: 11.5, color: p.textSecondary),
          ),
          children: [
            Divider(height: 12, color: p.border.withValues(alpha: 0.7)),
            ...children,
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final s = widget.style;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.auto_fix_high, color: p.accent, size: 19),
                const SizedBox(width: 8),
                const Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'ビジュアルスタイル・テキスト装飾エディタ（CSS 不要）',
                        style: TextStyle(
                            fontSize: 14.5, fontWeight: FontWeight.bold),
                      ),
                      Text(
                        '袋文字（縁取り・二重フチ）・文字グラデーション・カード透明度・角丸・アニメーションを GUI で細かく作り込めます。',
                        style: TextStyle(fontSize: 11.5),
                      ),
                    ],
                  ),
                ),
                Switch(
                  value: s.enabled,
                  onChanged: (v) =>
                      widget.onChanged(copyVisualStyle(s, enabled: v)),
                ),
                Text(
                  s.enabled ? 'カスタム有効' : 'プリセット標準',
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                    color: s.enabled ? p.accent : p.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // ライブプレビュー領域
            Row(
              children: [
                const Text('リアルタイム・ライブプレビュー',
                    style:
                        TextStyle(fontWeight: FontWeight.w700, fontSize: 12.5)),
                const Spacer(),
                SegmentedButton<String>(
                  style: ButtonStyle(
                    visualDensity: VisualDensity.compact,
                    textStyle:
                        WidgetStatePropertyAll(const TextStyle(fontSize: 11)),
                  ),
                  segments: const [
                    ButtonSegment(value: 'checker', label: Text('透過確認')),
                    ButtonSegment(value: 'dark', label: Text('暗めゲーム')),
                    ButtonSegment(value: 'light', label: Text('明るめ画面')),
                  ],
                  selected: {_previewStage},
                  onSelectionChanged: (v) =>
                      setState(() => _previewStage = v.first),
                ),
              ],
            ),
            const SizedBox(height: 8),
            _buildLivePreviewStage(s),
            const SizedBox(height: 12),

            // ワンクリック・スタイルレシピ
            Wrap(
              spacing: 6,
              runSpacing: 6,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Text(
                  'ワンクリック装飾レシピ:',
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: p.textSecondary,
                  ),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('テレビ字幕（白＋黒フチ）',
                      style: TextStyle(fontSize: 11.5)),
                  onPressed: () => _applyRecipe('tv_subtitle'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('バラエティ二重フチ',
                      style: TextStyle(fontSize: 11.5)),
                  onPressed: () => _applyRecipe('variety_double'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('マガジン風カード',
                      style: TextStyle(fontSize: 11.5)),
                  onPressed: () => _applyRecipe('editorial_card'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('くっきりハード影',
                      style: TextStyle(fontSize: 11.5)),
                  onPressed: () => _applyRecipe('hard_pop'),
                ),
                ActionChip(
                  visualDensity: VisualDensity.compact,
                  label: const Text('エッジ発光（シアン）',
                      style: TextStyle(fontSize: 11.5)),
                  onPressed: () => _applyRecipe('edge_glow'),
                ),
                TextButton.icon(
                  onPressed: () => _applyRecipe('reset'),
                  icon: const Icon(Icons.restart_alt, size: 15),
                  label: const Text('プリセット標準に戻す',
                      style: TextStyle(fontSize: 11.5)),
                ),
                OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                      visualDensity: VisualDensity.compact),
                  onPressed: widget.onCopyToAll,
                  icon: const Icon(Icons.copy_all, size: 14),
                  label: const Text('全オーバーレイにコピー',
                      style: TextStyle(fontSize: 11.5)),
                ),
              ],
            ),
            const Divider(height: 22),

            // A. テキスト・タイポグラフィ（折りたたみドロップダウン）
            _buildCategoryDropdown(
              p: p,
              icon: Icons.text_fields,
              title: 'A. テキスト・タイポグラフィ',
              summary: _summaryA(s),
              children: [
                Wrap(
                  spacing: 16,
                  runSpacing: 10,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    _sliderBox(
                      label: '文字サイズ',
                      valueLabel: s.fontSize == 0 ? '標準' : '${s.fontSize}px',
                      value: s.fontSize.toDouble(),
                      min: 0,
                      max: 56,
                      divisions: 56,
                      onChanged: (v) =>
                          _update(copyVisualStyle(s, fontSize: v.round())),
                    ),
                    SizedBox(
                      width: 195,
                      child: Builder(builder: (_) {
                        final fwVal = const [
                          0,
                          300,
                          400,
                          500,
                          600,
                          700,
                          800,
                          900
                        ].contains(s.fontWeight)
                            ? s.fontWeight
                            : 0;
                        return DropdownButtonFormField<int>(
                          key: ValueKey('fw_$fwVal'),
                          isExpanded: true,
                          initialValue: fwVal,
                          decoration: const InputDecoration(
                            isDense: true,
                            labelText: '文字の太さ (Weight)',
                          ),
                          items: const [
                            DropdownMenuItem(value: 0, child: Text('プリセット標準')),
                            DropdownMenuItem(value: 300, child: Text('300 (細字)')),
                            DropdownMenuItem(value: 400, child: Text('400 (標準)')),
                            DropdownMenuItem(value: 500, child: Text('500 (中太)')),
                            DropdownMenuItem(
                                value: 600, child: Text('600 (セミボールド)')),
                            DropdownMenuItem(value: 700, child: Text('700 (太字)')),
                            DropdownMenuItem(value: 800, child: Text('800 (極太)')),
                            DropdownMenuItem(value: 900, child: Text('900 (最太)')),
                          ],
                          onChanged: (v) {
                            if (v != null) {
                              _update(copyVisualStyle(s, fontWeight: v));
                            }
                          },
                        );
                      }),
                    ),
                    _sliderBox(
                      label: '字間',
                      valueLabel:
                          '${(s.letterSpacingTenths / 10).toStringAsFixed(1)}px',
                      value: s.letterSpacingTenths.toDouble().clamp(-10, 60),
                      min: -10,
                      max: 60,
                      divisions: 70,
                      onChanged: (v) => _update(
                          copyVisualStyle(s, letterSpacingTenths: v.round())),
                    ),
                    _sliderBox(
                      label: '行間',
                      valueLabel: s.lineHeightPercent == 0
                          ? '標準'
                          : (s.lineHeightPercent / 100).toStringAsFixed(2),
                      value: s.lineHeightPercent == 0
                          ? 145
                          : s.lineHeightPercent.toDouble().clamp(110, 200),
                      min: 110,
                      max: 200,
                      divisions: 18,
                      onChanged: (v) => _update(
                          copyVisualStyle(s, lineHeightPercent: v.round())),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                // 文字色 / グラデーション
                Wrap(
                  spacing: 14,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    SizedBox(
                      width: 240,
                      child: SwitchListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('文字2色グラデーション',
                            style: TextStyle(fontSize: 12.5)),
                        value: s.useTextGradient,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, useTextGradient: v)),
                      ),
                    ),
                    if (!s.useTextGradient)
                      SizedBox(
                        width: 300,
                        child: ColorPickerRow(
                          label: '本文文字色',
                          hexValue:
                              s.textColor.isEmpty ? '#FFFFFF' : s.textColor,
                          onChanged: (v) =>
                              _update(copyVisualStyle(s, textColor: v)),
                        ),
                      )
                    else ...[
                      SizedBox(
                        width: 260,
                        child: ColorPickerRow(
                          label: '開始色',
                          hexValue: s.textGradientStart,
                          onChanged: (v) => _update(
                              copyVisualStyle(s, textGradientStart: v)),
                        ),
                      ),
                      SizedBox(
                        width: 260,
                        child: ColorPickerRow(
                          label: '終了色',
                          hexValue: s.textGradientEnd,
                          onChanged: (v) =>
                              _update(copyVisualStyle(s, textGradientEnd: v)),
                        ),
                      ),
                      _sliderBox(
                        label: '角度',
                        valueLabel: '${s.textGradientAngle}°',
                        value: s.textGradientAngle.toDouble().clamp(0, 360),
                        min: 0,
                        max: 360,
                        divisions: 24,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textGradientAngle: v.round())),
                      ),
                    ],
                    SizedBox(
                      width: 270,
                      child: ColorPickerRow(
                        label: '名前・見出し色',
                        hexValue:
                            s.authorColor.isEmpty ? '#E4E7EE' : s.authorColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, authorColor: v)),
                      ),
                    ),
                    SizedBox(
                      width: 270,
                      child: ColorPickerRow(
                        label: 'アクセント色',
                        hexValue:
                            s.accentColor.isEmpty ? '#D93829' : s.accentColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, accentColor: v)),
                      ),
                    ),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 10),

            // B. 袋文字（縁取り・二重フチ）・エッジ発光・文字シャドウ（折りたたみドロップダウン）
            _buildCategoryDropdown(
              p: p,
              icon: Icons.border_color_outlined,
              title: 'B. 袋文字（縁取り・二重フチ）・エッジ発光・文字シャドウ',
              summary: _summaryB(s),
              children: [
                Wrap(
                  spacing: 16,
                  runSpacing: 10,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    _sliderBox(
                      label: '内側フチ太さ',
                      valueLabel:
                          s.strokeWidth == 0 ? 'なし' : '${s.strokeWidth}px',
                      value: s.strokeWidth.toDouble().clamp(0, 8),
                      min: 0,
                      max: 8,
                      divisions: 8,
                      onChanged: (v) =>
                          _update(copyVisualStyle(s, strokeWidth: v.round())),
                    ),
                    SizedBox(
                      width: 260,
                      child: ColorPickerRow(
                        label: '内側フチ色',
                        hexValue: s.strokeColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, strokeColor: v)),
                      ),
                    ),
                    _sliderBox(
                      label: '二重外フチ太さ',
                      valueLabel: s.outerStrokeWidth == 0
                          ? 'なし'
                          : '${s.outerStrokeWidth}px',
                      value: s.outerStrokeWidth.toDouble().clamp(0, 8),
                      min: 0,
                      max: 8,
                      divisions: 8,
                      onChanged: (v) => _update(
                          copyVisualStyle(s, outerStrokeWidth: v.round())),
                    ),
                    SizedBox(
                      width: 260,
                      child: ColorPickerRow(
                        label: '外側フチ色',
                        hexValue: s.outerStrokeColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, outerStrokeColor: v)),
                      ),
                    ),
                  ],
                ),
                Divider(height: 18, color: p.border.withValues(alpha: 0.6)),
                // テキストのエッジ発光（Edge Glow）
                Wrap(
                  spacing: 14,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    SizedBox(
                      width: 225,
                      child: SwitchListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('テキストエッジ発光',
                            style: TextStyle(fontSize: 12.5)),
                        value: s.textGlowEnabled,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, textGlowEnabled: v)),
                      ),
                    ),
                    if (s.textGlowEnabled) ...[
                      SizedBox(
                        width: 270,
                        child: ColorPickerRow(
                          label: '発光色',
                          hexValue: s.textGlowColor.isEmpty
                              ? '#38BDF8'
                              : s.textGlowColor,
                          presets: const [
                            '#38BDF8',
                            '#F472B6',
                            '#FACC15',
                            '#34D399',
                            '#A78BFA',
                            '#FB7185',
                            '#FFFFFF',
                          ],
                          onChanged: (v) =>
                              _update(copyVisualStyle(s, textGlowColor: v)),
                        ),
                      ),
                      _sliderBox(
                        label: '発光の広がり',
                        valueLabel: '${s.textGlowRadius}px',
                        value: s.textGlowRadius.toDouble().clamp(1, 40),
                        min: 1,
                        max: 40,
                        divisions: 39,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textGlowRadius: v.round())),
                      ),
                      _sliderBox(
                        label: '発光強度',
                        valueLabel: '${s.textGlowIntensity}%',
                        value: s.textGlowIntensity.toDouble().clamp(10, 200),
                        min: 10,
                        max: 200,
                        divisions: 38,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textGlowIntensity: v.round())),
                      ),
                      _sliderBox(
                        label: 'エッジ芯の太さ',
                        valueLabel: s.textGlowSpread == 0
                            ? 'なし'
                            : '${s.textGlowSpread}px',
                        value: s.textGlowSpread.toDouble().clamp(0, 10),
                        min: 0,
                        max: 10,
                        divisions: 10,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textGlowSpread: v.round())),
                      ),
                    ],
                  ],
                ),
                Divider(height: 18, color: p.border.withValues(alpha: 0.6)),
                Wrap(
                  spacing: 14,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    SizedBox(
                      width: 225,
                      child: SwitchListTile(
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: const Text('文字ドロップシャドウ',
                            style: TextStyle(fontSize: 12.5)),
                        value: s.textShadowEnabled,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, textShadowEnabled: v)),
                      ),
                    ),
                    if (s.textShadowEnabled) ...[
                      SizedBox(
                        width: 240,
                        child: ColorPickerRow(
                          label: '影の色',
                          hexValue: s.textShadowColor,
                          onChanged: (v) =>
                              _update(copyVisualStyle(s, textShadowColor: v)),
                        ),
                      ),
                      _sliderBox(
                        label: 'ぼかし',
                        valueLabel: '${s.textShadowBlur}px',
                        value: s.textShadowBlur.toDouble().clamp(0, 20),
                        min: 0,
                        max: 20,
                        divisions: 20,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textShadowBlur: v.round())),
                      ),
                      _sliderBox(
                        label: '影の縦ずれ',
                        valueLabel: '${s.textShadowOffsetY}px',
                        value: s.textShadowOffsetY.toDouble().clamp(-10, 14),
                        min: -10,
                        max: 14,
                        divisions: 24,
                        onChanged: (v) => _update(
                            copyVisualStyle(s, textShadowOffsetY: v.round())),
                      ),
                    ],
                  ],
                ),
              ],
            ),
            const SizedBox(height: 10),

            // C. カード装飾 ＆ モーション（折りたたみドロップダウン）
            _buildCategoryDropdown(
              p: p,
              icon: Icons.dashboard_customize_outlined,
              title: 'C. ボックス装飾 ＆ 出現モーション',
              summary: _summaryC(s),
              children: [
                Wrap(
                  spacing: 16,
                  runSpacing: 10,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    SizedBox(
                      width: 265,
                      child: ColorPickerRow(
                        label: 'カード背景色',
                        hexValue:
                            s.cardBgColor.isEmpty ? '#111317' : s.cardBgColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, cardBgColor: v)),
                      ),
                    ),
                    _sliderBox(
                      label: 'カード不透明度',
                      valueLabel: '${s.cardBgOpacity}%',
                      value: s.cardBgOpacity.toDouble().clamp(0, 100),
                      min: 0,
                      max: 100,
                      divisions: 20,
                      onChanged: (v) => _update(copyVisualStyle(
                        s,
                        cardBgOpacity: v.round(),
                        cardBgColor:
                            s.cardBgColor.isEmpty ? '#111317' : s.cardBgColor,
                      )),
                    ),
                    _sliderBox(
                      label: '角丸',
                      valueLabel:
                          s.borderRadius < 0 ? '標準' : '${s.borderRadius}px',
                      value: (s.borderRadius < 0 ? 4 : s.borderRadius)
                          .toDouble()
                          .clamp(0, 28),
                      min: 0,
                      max: 28,
                      divisions: 28,
                      onChanged: (v) =>
                          _update(copyVisualStyle(s, borderRadius: v.round())),
                    ),
                    _sliderBox(
                      label: '枠線太さ',
                      valueLabel:
                          s.borderWidth < 0 ? '標準' : '${s.borderWidth}px',
                      value: (s.borderWidth < 0 ? 1 : s.borderWidth)
                          .toDouble()
                          .clamp(0, 6),
                      min: 0,
                      max: 6,
                      divisions: 6,
                      onChanged: (v) =>
                          _update(copyVisualStyle(s, borderWidth: v.round())),
                    ),
                    SizedBox(
                      width: 255,
                      child: ColorPickerRow(
                        label: '枠線色',
                        hexValue:
                            s.borderColor.isEmpty ? '#D93829' : s.borderColor,
                        onChanged: (v) =>
                            _update(copyVisualStyle(s, borderColor: v)),
                      ),
                    ),
                    SizedBox(
                      width: 175,
                      child: Builder(builder: (_) {
                        final shadowVal = const [
                          'inherit',
                          'none',
                          'soft',
                          'medium',
                          'hard'
                        ].contains(s.cardShadow)
                            ? s.cardShadow
                            : 'inherit';
                        return DropdownButtonFormField<String>(
                          key: ValueKey('shadow_$shadowVal'),
                          isExpanded: true,
                          initialValue: shadowVal,
                          decoration: const InputDecoration(
                              isDense: true, labelText: 'カードの影'),
                          items: const [
                            DropdownMenuItem(value: 'inherit', child: Text('標準')),
                            DropdownMenuItem(value: 'none', child: Text('影なし')),
                            DropdownMenuItem(value: 'soft', child: Text('控えめ')),
                            DropdownMenuItem(
                                value: 'medium', child: Text('しっかり')),
                            DropdownMenuItem(value: 'hard', child: Text('ハード影')),
                          ],
                          onChanged: (v) {
                            if (v != null) {
                              _update(copyVisualStyle(s, cardShadow: v));
                            }
                          },
                        );
                      }),
                    ),
                    SizedBox(
                      width: 175,
                      child: Builder(builder: (_) {
                        final padVal = const [
                          'inherit',
                          'compact',
                          'normal',
                          'relaxed'
                        ].contains(s.paddingScale)
                            ? s.paddingScale
                            : 'inherit';
                        return DropdownButtonFormField<String>(
                          key: ValueKey('pad_$padVal'),
                          isExpanded: true,
                          initialValue: padVal,
                          decoration: const InputDecoration(
                              isDense: true, labelText: '内側余白'),
                          items: const [
                            DropdownMenuItem(value: 'inherit', child: Text('標準')),
                            DropdownMenuItem(
                                value: 'compact', child: Text('コンパクト')),
                            DropdownMenuItem(
                                value: 'normal', child: Text('標準サイズ')),
                            DropdownMenuItem(
                                value: 'relaxed', child: Text('ゆったり')),
                          ],
                          onChanged: (v) {
                            if (v != null) {
                              _update(copyVisualStyle(s, paddingScale: v));
                            }
                          },
                        );
                      }),
                    ),
                    SizedBox(
                      width: 185,
                      child: Builder(builder: (_) {
                        final animVal = const [
                          'inherit',
                          'wipe',
                          'slide',
                          'fade',
                          'pop',
                          'none'
                        ].contains(s.entranceAnimation)
                            ? s.entranceAnimation
                            : 'inherit';
                        return DropdownButtonFormField<String>(
                          key: ValueKey('anim_$animVal'),
                          isExpanded: true,
                          initialValue: animVal,
                          decoration: const InputDecoration(
                              isDense: true, labelText: '出現モーション'),
                          items: const [
                            DropdownMenuItem(value: 'inherit', child: Text('標準')),
                            DropdownMenuItem(value: 'wipe', child: Text('水平ワイプ')),
                            DropdownMenuItem(value: 'slide', child: Text('スライド')),
                            DropdownMenuItem(value: 'fade', child: Text('フェード')),
                            DropdownMenuItem(value: 'pop', child: Text('ポップ')),
                            DropdownMenuItem(value: 'none', child: Text('なし')),
                          ],
                          onChanged: (v) {
                            if (v != null) {
                              _update(copyVisualStyle(s, entranceAnimation: v));
                            }
                          },
                        );
                      }),
                    ),
                  ],
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _sliderBox({
    required String label,
    required String valueLabel,
    required double value,
    required double min,
    required double max,
    required int divisions,
    required ValueChanged<double> onChanged,
  }) {
    return SizedBox(
      width: 210,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              Text(label, style: const TextStyle(fontSize: 12)),
              const Spacer(),
              Text(valueLabel,
                  style: const TextStyle(
                      fontSize: 11.5,
                      fontFamily: 'Consolas',
                      fontWeight: FontWeight.bold)),
            ],
          ),
          Slider(
            value: value.clamp(min, max),
            min: min,
            max: max,
            divisions: divisions,
            onChanged: onChanged,
          ),
        ],
      ),
    );
  }

  Widget _buildLivePreviewStage(OverlayVisualStyle s) {
    final active = s.enabled;
    final bgColor = active && s.cardBgColor.isNotEmpty
        ? AppTheme.parseCssHex(s.cardBgColor, fallback: widget.presetBg)
            .withValues(alpha: (s.cardBgOpacity / 100.0).clamp(0.0, 1.0))
        : (active && s.cardBgOpacity == 0
            ? Colors.transparent
            : widget.presetBg);
    final accent = active && s.accentColor.isNotEmpty
        ? AppTheme.parseCssHex(s.accentColor, fallback: widget.presetAccent)
        : widget.presetAccent;
    final authorColor = active && s.authorColor.isNotEmpty
        ? AppTheme.parseCssHex(s.authorColor, fallback: accent)
        : widget.presetFg.withValues(alpha: 0.85);
    final textColor = active && s.textColor.isNotEmpty
        ? AppTheme.parseCssHex(s.textColor, fallback: widget.presetFg)
        : widget.presetFg;
    final borderRadius =
        active && s.borderRadius >= 0 ? s.borderRadius.toDouble() : 3.0;
    final borderWidth =
        active && s.borderWidth >= 0 ? s.borderWidth.toDouble() : 1.5;
    final borderColor = active && s.borderColor.isNotEmpty
        ? AppTheme.parseCssHex(s.borderColor, fallback: accent)
        : accent;
    final fontSize =
        (active && s.fontSize > 0 ? s.fontSize.toDouble() : 17.0).clamp(12.0, 34.0);
    final fontWeight = active ? _mapWeight(s.fontWeight) : FontWeight.w700;
    final letterSpacing = active ? s.letterSpacingTenths / 10.0 : 0.2;
    final lineHeight = active && s.lineHeightPercent >= 100
        ? s.lineHeightPercent / 100.0
        : 1.45;
    final pad = !active || s.paddingScale == 'inherit' || s.paddingScale == 'normal'
        ? const EdgeInsets.symmetric(horizontal: 16, vertical: 12)
        : (s.paddingScale == 'compact'
            ? const EdgeInsets.symmetric(horizontal: 11, vertical: 7)
            : const EdgeInsets.symmetric(horizontal: 22, vertical: 16));

    List<BoxShadow>? boxShadows;
    if (active) {
      switch (s.cardShadow) {
        case 'none':
          boxShadows = null;
        case 'soft':
          boxShadows = [
            BoxShadow(
                color: Colors.black.withValues(alpha: 0.22),
                blurRadius: 12,
                offset: const Offset(0, 4))
          ];
        case 'medium':
          boxShadows = [
            BoxShadow(
                color: Colors.black.withValues(alpha: 0.42),
                blurRadius: 22,
                offset: const Offset(0, 8))
          ];
        case 'hard':
          boxShadows = const [
            BoxShadow(color: Color(0xFF182033), offset: Offset(5, 5))
          ];
      }
    }

    final isPresetEdgeGlow = !active && widget.presetId == 'edge_glow';
    final effectiveAuthorColor = isPresetEdgeGlow
        ? const Color(0xFFBAE6FD)
        : authorColor;
    final authorShadows = isPresetEdgeGlow
        ? [
            Shadow(
              color: const Color(0xFF38BDF8).withValues(alpha: 0.75),
              blurRadius: 6,
            ),
            Shadow(
              color: const Color(0xFF38BDF8).withValues(alpha: 0.38),
              blurRadius: 12,
            ),
          ]
        : null;

    final stageBg = _previewStage == 'light'
        ? const Color(0xFFE7E5DF)
        : const Color(0xFF191D24);

    return ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: CustomPaint(
        painter: _previewStage == 'checker' ? _CheckerPainter() : null,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.all(20),
          color: _previewStage == 'checker' ? Colors.transparent : stageBg,
          child: Center(
            child: Container(
              constraints: const BoxConstraints(maxWidth: 560),
              padding: pad,
              decoration: BoxDecoration(
                color: bgColor,
                borderRadius: BorderRadius.circular(borderRadius),
                border: borderWidth > 0
                    ? Border.all(color: borderColor, width: borderWidth)
                    : null,
                boxShadow: boxShadows,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Container(
                        width: 4,
                        height: 14,
                        color: accent,
                      ),
                      const SizedBox(width: 7),
                      Text(
                        widget.sampleAuthor,
                        style: TextStyle(
                          fontFamily: widget.fontFamily,
                          color: effectiveAuthorColor,
                          fontSize: 12.5,
                          fontWeight: FontWeight.w800,
                          letterSpacing: 0.4,
                          shadows: authorShadows,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 5),
                  _buildStyledSampleText(
                    s: s,
                    active: active,
                    presetEdgeGlow: isPresetEdgeGlow,
                    text: widget.sampleText,
                    fontSize: fontSize,
                    fontWeight: fontWeight,
                    letterSpacing: letterSpacing,
                    lineHeight: lineHeight,
                    textColor: textColor,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  List<Shadow> _buildEdgeGlowShadows(OverlayVisualStyle s) {
    final glowColor = AppTheme.parseCssHex(
      s.textGlowColor,
      fallback: const Color(0xFF38BDF8),
    );
    final radius = s.textGlowRadius.clamp(1, 40).toDouble();
    final intensity = s.textGlowIntensity.clamp(10, 200) / 100.0;
    final spread = s.textGlowSpread.clamp(0, 10).toDouble();

    final coreAlpha = (0.78 * intensity).clamp(0.08, 1.0);
    final midAlpha = (0.56 * intensity).clamp(0.06, 0.95);
    final outerAlpha = (0.36 * intensity).clamp(0.04, 0.85);

    final out = <Shadow>[];
    if (spread > 0) {
      final cb = (spread * 1.4).clamp(1.0, 14.0);
      final c = glowColor.withValues(alpha: coreAlpha);
      out.add(Shadow(color: c, blurRadius: cb, offset: Offset(spread, 0)));
      out.add(Shadow(color: c, blurRadius: cb, offset: Offset(-spread, 0)));
      out.add(Shadow(color: c, blurRadius: cb, offset: Offset(0, spread)));
      out.add(Shadow(color: c, blurRadius: cb, offset: Offset(0, -spread)));
    }
    out.add(Shadow(
      color: glowColor.withValues(alpha: coreAlpha),
      blurRadius: (radius * 0.25).clamp(2.0, 12.0),
    ));
    out.add(Shadow(
      color: glowColor.withValues(alpha: midAlpha),
      blurRadius: (radius * 0.65).clamp(4.0, 28.0),
    ));
    out.add(Shadow(
      color: glowColor.withValues(alpha: outerAlpha),
      blurRadius: radius,
    ));
    if (intensity > 1.0) {
      final boostAlpha = ((intensity - 0.85) * 0.65).clamp(0.1, 0.85);
      out.add(Shadow(
        color: glowColor.withValues(alpha: boostAlpha),
        blurRadius: radius * 1.55,
      ));
    }
    return out;
  }

  Widget _buildStyledSampleText({
    required OverlayVisualStyle s,
    required bool active,
    bool presetEdgeGlow = false,
    required String text,
    required double fontSize,
    required FontWeight fontWeight,
    required double letterSpacing,
    required double lineHeight,
    required Color textColor,
  }) {
    final dropShadows = active && s.textShadowEnabled
        ? [
            Shadow(
              color: AppTheme.parseCssHex(s.textShadowColor,
                  fallback: Colors.black),
              blurRadius: s.textShadowBlur.toDouble(),
              offset: Offset(
                s.textShadowOffsetX.toDouble(),
                s.textShadowOffsetY.toDouble(),
              ),
            ),
          ]
        : const <Shadow>[];
    final glowShadows = active && s.textGlowEnabled
        ? _buildEdgeGlowShadows(s)
        : (presetEdgeGlow
            ? _buildEdgeGlowShadows(
                copyVisualStyle(
                  defaultVisualStyle(),
                  enabled: true,
                  textGlowEnabled: true,
                  textGlowColor: '#38BDF8',
                  textGlowRadius: 14,
                  textGlowIntensity: 95,
                  textGlowSpread: 1,
                ),
              )
            : const <Shadow>[]);
    final hasGlow = glowShadows.isNotEmpty;
    final hasStroke = active && (s.strokeWidth > 0 || s.outerStrokeWidth > 0);
    // グラデーション文字は ShaderMask 適用時に Shadow もマスクされないよう独立背面レイヤーで描画する
    final useBackdropShadowLayer =
        (active && s.useTextGradient && (hasGlow || dropShadows.isNotEmpty)) ||
            (hasGlow && hasStroke);

    final baseStyle = TextStyle(
      fontFamily: widget.fontFamily,
      fontSize: fontSize,
      fontWeight: fontWeight,
      letterSpacing: letterSpacing,
      height: lineHeight,
      color: textColor,
      shadows: useBackdropShadowLayer
          ? null
          : [
              ...glowShadows,
              ...dropShadows,
            ].isEmpty
              ? null
              : [...glowShadows, ...dropShadows],
    );

    Widget fillLayer = Text(text, style: baseStyle);
    if (active && s.useTextGradient) {
      final c1 =
          AppTheme.parseCssHex(s.textGradientStart, fallback: Colors.white);
      final c2 = AppTheme.parseCssHex(s.textGradientEnd,
          fallback: const Color(0xFFFDE047));
      final rad = (s.textGradientAngle - 90) * math.pi / 180.0;
      fillLayer = ShaderMask(
        shaderCallback: (bounds) => LinearGradient(
          begin: Alignment(-math.cos(rad), -math.sin(rad)),
          end: Alignment(math.cos(rad), math.sin(rad)),
          colors: [c1, c2],
        ).createShader(bounds),
        child: Text(text, style: baseStyle.copyWith(color: Colors.white)),
      );
    }

    if (!hasStroke && !useBackdropShadowLayer) {
      return fillLayer;
    }

    final layers = <Widget>[];
    if (useBackdropShadowLayer) {
      layers.add(
        Text(
          text,
          style: baseStyle.copyWith(
            color: Colors.transparent,
            shadows: [...glowShadows, ...dropShadows],
          ),
        ),
      );
    }
    if (active && s.outerStrokeWidth > 0) {
      final totalW = (s.strokeWidth + s.outerStrokeWidth) * 2.0;
      layers.add(
        Text(
          text,
          style: baseStyle.copyWith(
            color: null,
            shadows: null,
            foreground: Paint()
              ..style = PaintingStyle.stroke
              ..strokeWidth = totalW
              ..strokeJoin = StrokeJoin.round
              ..color = AppTheme.parseCssHex(s.outerStrokeColor,
                  fallback: Colors.white),
          ),
        ),
      );
    }
    if (active && s.strokeWidth > 0) {
      layers.add(
        Text(
          text,
          style: baseStyle.copyWith(
            color: null,
            shadows: null,
            foreground: Paint()
              ..style = PaintingStyle.stroke
              ..strokeWidth = s.strokeWidth * 2.0
              ..strokeJoin = StrokeJoin.round
              ..color = AppTheme.parseCssHex(s.strokeColor,
                  fallback: const Color(0xFF111114)),
          ),
        ),
      );
    }
    layers.add(fillLayer);
    return Stack(children: layers);
  }
}

class _CheckerPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    const cell = 14.0;
    final p1 = Paint()..color = const Color(0xFF262930);
    final p2 = Paint()..color = const Color(0xFF1D2026);
    for (double y = 0; y < size.height; y += cell) {
      for (double x = 0; x < size.width; x += cell) {
        final odd = ((x ~/ cell) + (y ~/ cell)) % 2 == 1;
        canvas.drawRect(Rect.fromLTWH(x, y, cell, cell), odd ? p1 : p2);
      }
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
