import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import '../widgets/color_picker_row.dart';

/// OBS Studio ブラウザソースオーバーレイの URL 管理 ＆ 外観設定画面
class OverlayScreen extends StatefulWidget {
  const OverlayScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<OverlayScreen> createState() => _OverlayScreenState();
}

class _OverlayScreenState extends State<OverlayScreen> {
  late TextEditingController _portCtrl;
  late int _fontSize;
  late int _maxItems;
  late bool _showIcons;
  late bool _showTimestamps;
  late String _defaultBg;
  late String _defaultText;

  @override
  void initState() {
    super.initState();
    _loadFromConfig(widget.controller.overlayConfig);
  }

  void _loadFromConfig(OverlayConfig cfg) {
    _portCtrl = TextEditingController(text: cfg.port.toString());
    _fontSize = cfg.commentFontSize;
    _maxItems = cfg.commentMaxItems;
    _showIcons = cfg.showIcons;
    _showTimestamps = cfg.showTimestamps;
    _defaultBg = cfg.defaultBubbleBg;
    _defaultText = cfg.defaultTextColor;
  }

  @override
  void dispose() {
    _portCtrl.dispose();
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
    ));
  }

  @override
  Widget build(BuildContext context) {
    final base = widget.controller.overlayBaseUrl;
    final commentsUrl = '$base/overlay/comments';
    final lotteryUrl = '$base/overlay/lottery';
    final counterUrl = '$base/overlay/counter';

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'OBS Studio オーバーレイ（ブラウザソース）',
            style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 4),
          const Text(
            '以下の URL を OBS Studio の「ブラウザ」ソースに貼り付けるだけで、背景透過のリアルタイムオーバーレイを表示できます。',
            style: TextStyle(color: Colors.white60),
          ),
          const SizedBox(height: 18),

          // URL 一覧カード
          _OverlayUrlCard(
            icon: Icons.chat_bubble,
            color: AppTheme.accentSky,
            title: 'コメント表示オーバーレイ（特定ワード背景色変更対応）',
            description: '推奨サイズ: 幅 480 × 高さ 720。特定ワード反応ルールの吹き出し色がそのまま反映されます。',
            url: commentsUrl,
            onCopy: () => _copy(commentsUrl, 'コメント表示'),
            onOpen: () => widget.controller.openUrl(commentsUrl),
          ),
          const SizedBox(height: 12),
          _OverlayUrlCard(
            icon: Icons.emoji_events,
            color: AppTheme.accentAmber,
            title: '抽選演出・当選者表示オーバーレイ',
            description: '推奨サイズ: 幅 960 × 高さ 540。ユーザー抽選・コメント抽選の当選者のアイコンと名前を大きく表示します。',
            url: lotteryUrl,
            onCopy: () => _copy(lotteryUrl, '抽選結果表示'),
            onOpen: () => widget.controller.openUrl(lotteryUrl),
          ),
          const SizedBox(height: 12),
          _OverlayUrlCard(
            icon: Icons.pin,
            color: AppTheme.accentEmerald,
            title: '特定ワードカウンターオーバーレイ',
            description: '推奨サイズ: 幅 600 × 高さ 240。有効なすべてのカウンターをリアルタイム表示します（単体URLはカウンター画面からコピー可能）。',
            url: counterUrl,
            onCopy: () => _copy(counterUrl, 'ワードカウンター'),
            onOpen: () => widget.controller.openUrl(counterUrl),
          ),
          const SizedBox(height: 20),

          // 外観・サーバー設定
          Card(
            child: Padding(
              padding: const EdgeInsets.all(18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'オーバーレイ外観・サーバー設定',
                    style: TextStyle(fontSize: 17, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 14),
                  Row(
                    children: [
                      SizedBox(
                        width: 180,
                        child: TextField(
                          controller: _portCtrl,
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            labelText: 'ローカルサーバーポート',
                            hintText: '18765',
                          ),
                        ),
                      ),
                      const SizedBox(width: 24),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('コメント文字サイズ: ${_fontSize}px'),
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
                            Text('最大保持コメント数: $_maxItems件'),
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
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: SwitchListTile(
                          contentPadding: EdgeInsets.zero,
                          value: _showIcons,
                          onChanged: (v) => setState(() => _showIcons = v),
                          title: const Text('ユーザーアイコンを表示する'),
                        ),
                      ),
                      Expanded(
                        child: SwitchListTile(
                          contentPadding: EdgeInsets.zero,
                          value: _showTimestamps,
                          onChanged: (v) => setState(() => _showTimestamps = v),
                          title: const Text('時刻を表示する'),
                        ),
                      ),
                    ],
                  ),
                  const Divider(),
                  ColorPickerRow(
                    label: '通常コメントの吹き出し背景色',
                    hexValue: _defaultBg,
                    onChanged: (v) => setState(() => _defaultBg = v),
                  ),
                  const SizedBox(height: 12),
                  ColorPickerRow(
                    label: '通常コメントの文字色',
                    hexValue: _defaultText,
                    presets: const [
                      '#F8FAFC',
                      '#E2E8F0',
                      '#FEF9C3',
                      '#ECFDF5',
                      '#0F172A',
                    ],
                    onChanged: (v) => setState(() => _defaultText = v),
                  ),
                  const SizedBox(height: 16),
                  Align(
                    alignment: Alignment.centerRight,
                    child: FilledButton.icon(
                      onPressed: _save,
                      icon: const Icon(Icons.save),
                      label: const Text('オーバーレイ設定を保存・即時反映'),
                    ),
                  ),
                ],
              ),
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
    required this.color,
    required this.title,
    required this.description,
    required this.url,
    required this.onCopy,
    required this.onOpen,
  });

  final IconData icon;
  final Color color;
  final String title;
  final String description;
  final String url;
  final VoidCallback onCopy;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: color.withValues(alpha: 0.16),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Icon(icon, color: color, size: 26),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title,
                      style: const TextStyle(
                          fontWeight: FontWeight.bold, fontSize: 15)),
                  const SizedBox(height: 3),
                  Text(description,
                      style:
                          const TextStyle(fontSize: 12.5, color: Colors.white60)),
                  const SizedBox(height: 6),
                  SelectableText(
                    url,
                    style: const TextStyle(
                      fontFamily: 'Consolas',
                      color: AppTheme.accentSky,
                      fontSize: 13,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            FilledButton.icon(
              onPressed: onCopy,
              icon: const Icon(Icons.copy, size: 16),
              label: const Text('URLコピー'),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: onOpen,
              icon: const Icon(Icons.open_in_new, size: 16),
              label: const Text('ブラウザで確認'),
            ),
          ],
        ),
      ),
    );
  }
}
