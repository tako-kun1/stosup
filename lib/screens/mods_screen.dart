import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';

enum _ModFilter { all, reaction, theme, overlay, script, error }

/// ユーザー作成演出 MOD の導入・有効化・テスト再生・URL コピーを管理する画面
class ModsScreen extends StatefulWidget {
  const ModsScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<ModsScreen> createState() => _ModsScreenState();
}

class _ModsScreenState extends State<ModsScreen> {
  _ModFilter _filter = _ModFilter.all;
  bool _busy = false;
  bool _logsExpanded = true;

  @override
  void initState() {
    super.initState();
    // 画面表示時に最新のロジックスクリプト実行ログを取得（テスト用の非 FFI 環境では内部で安全にスキップ）
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && widget.controller.isInitialized) {
        widget.controller.refreshModScriptLogs();
      }
    });
  }

  static String _scriptEventLabel(String ev) {
    switch (ev) {
      case 'comment':
        return 'コメント';
      case 'superchat':
        return 'スパチャ/メンバー';
      case 'first_time':
        return '初見';
      case 'lottery':
        return '抽選';
      default:
        return ev;
    }
  }

  Future<void> _showScriptTestDialog(
    InstalledMod mod,
    ModLogicScriptDef sc,
  ) async {
    final msgCtrl = TextEditingController(text: '草 www テストコメント！');
    bool previewOnly = true;
    final run = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setLocal) => AlertDialog(
          title: Row(
            children: [
              const Icon(Icons.code_outlined, size: 20),
              const SizedBox(width: 8),
              Expanded(child: Text('スクリプトをテスト実行: ${sc.name}')),
            ],
          ),
          content: SizedBox(
            width: 440,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'on_test(ctx) が定義されていれば on_test を、無ければ on_comment(ctx) をこのテストコメントで実行します。',
                  style: TextStyle(
                    fontSize: 12.5,
                    color: context.palette.textSecondary,
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: msgCtrl,
                  decoration: const InputDecoration(
                    labelText: 'テスト用コメント本文',
                    hintText: '例: 草 www',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                ),
                const SizedBox(height: 10),
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: const Text(
                    'プレビュー専用環境へ出力（本番・OBS に影響しない）',
                    style: TextStyle(fontSize: 13),
                  ),
                  value: previewOnly,
                  onChanged: (v) => setLocal(() => previewOnly = v),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('キャンセル'),
            ),
            FilledButton.icon(
              onPressed: () => Navigator.pop(ctx, true),
              icon: const Icon(Icons.play_arrow, size: 17),
              label: const Text('テスト実行'),
            ),
          ],
        ),
      ),
    );
    final text = msgCtrl.text;
    msgCtrl.dispose();
    if (run == true && mounted) {
      await widget.controller.testModLogicScript(
        modId: mod.id,
        scriptId: sc.scriptId,
        sampleMessage: text,
        previewOnly: previewOnly,
      );
    }
  }

  void _copyUrl(String url, String label) {
    Clipboard.setData(ClipboardData(text: url));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('$label の URL をコピーしました')),
    );
  }

  Future<void> _confirmAndImport() async {
    final p = context.palette;
    final agreed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Row(
          children: [
            Icon(Icons.extension_outlined, size: 22),
            SizedBox(width: 8),
            Text('演出 MOD を導入する前に'),
          ],
        ),
        content: SizedBox(
          width: 480,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'ユーザー作成 MOD（.stosupmod / .zip）には JavaScript・CSS・画像・音声のほか、Stosup 本体のサンドボックス上で動く Rhai ロジックスクリプト（.rhai）が含まれる場合があります。',
                style: TextStyle(fontSize: 13.5, height: 1.5),
              ),
              const SizedBox(height: 10),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: p.warning.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: p.warning.withValues(alpha: 0.45)),
                ),
                child: const Text(
                  '・信頼できる制作者・配布元から入手した MOD のみ導入してください。\n'
                  '・実行ファイル（.exe / .dll / .bat 等）を含むアーカイブは自動で拒否されます。\n'
                  '・ロジックスクリプト（.rhai）はファイル・OS へ一切アクセスできない安全なサンドボックス上で動作します。\n'
                  '・第三者製 MOD の利用に関する詳細は利用規約（第10条）をご確認ください。',
                  style: TextStyle(fontSize: 12, height: 1.5),
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton.icon(
            onPressed: () => Navigator.pop(ctx, true),
            icon: const Icon(Icons.file_open_outlined, size: 17),
            label: const Text('ファイルを選択して導入'),
          ),
        ],
      ),
    );
    if (agreed != true || !mounted) return;

    setState(() => _busy = true);
    try {
      await widget.controller.pickAndImportMod();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _reload() async {
    setState(() => _busy = true);
    try {
      await widget.controller.reloadMods(notifyUser: true);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _confirmUninstall(InstalledMod mod) async {
    final p = context.palette;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('MOD を削除しますか？'),
        content: Text(
          '「${mod.name}」（ID: ${mod.id}）の MOD フォルダを削除します。\n'
          'この MOD を参照しているリアクションルールやオーバーレイテーマは、標準設定または無効状態へフォールバックされます。',
          style: const TextStyle(fontSize: 13.5, height: 1.5),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('キャンセル'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: p.danger),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('削除する'),
          ),
        ],
      ),
    );
    if (ok == true && mounted) {
      await widget.controller.uninstallMod(mod.id, modName: mod.name);
    }
  }

  List<InstalledMod> _applyFilter(List<InstalledMod> list) {
    switch (_filter) {
      case _ModFilter.all:
        return list;
      case _ModFilter.reaction:
        return list.where((m) => m.reactionEffects.isNotEmpty).toList();
      case _ModFilter.theme:
        return list.where((m) => m.overlayThemes.isNotEmpty).toList();
      case _ModFilter.overlay:
        return list.where((m) => m.customOverlays.isNotEmpty).toList();
      case _ModFilter.script:
        return list.where((m) => m.logicScripts.isNotEmpty).toList();
      case _ModFilter.error:
        return list.where((m) => m.loadError != null).toList();
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final allMods = ctrl.installedMods;
    final filtered = _applyFilter(allMods);

    final enabledCount =
        allMods.where((m) => m.enabled && m.loadError == null).length;
    final reactionCount = ctrl.enabledModReactionEffects.length;
    final themeCount = allMods
        .where((m) => m.enabled && m.loadError == null)
        .fold<int>(0, (sum, m) => sum + m.overlayThemes.length);
    final customOverlayCount = ctrl.enabledModCustomOverlays.length;
    final logicScriptCount = ctrl.enabledModLogicScripts.length;
    final errorCount = allMods.where((m) => m.loadError != null).length;
    final obsEnabled = ctrl.featureFlags.obsOverlayEnabled;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // ヘッダー
          Wrap(
            spacing: 10,
            runSpacing: 10,
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'MOD 管理（演出・テーマ・独自オーバーレイ・ロジックスクリプト）',
                    style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'ユーザー作成の演出 MOD（JavaScript / CSS / .rhai スクリプト）を導入・管理します。',
                    style: TextStyle(fontSize: 12.5, color: p.textSecondary),
                  ),
                ],
              ),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilledButton.icon(
                    onPressed: _busy ? null : _confirmAndImport,
                    icon: const Icon(Icons.add_box_outlined, size: 17),
                    label: const Text('MOD を導入 (.stosupmod / .zip)'),
                  ),
                  OutlinedButton.icon(
                    onPressed: _busy ? null : _reload,
                    icon: _busy
                        ? const SizedBox(
                            width: 15,
                            height: 15,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.refresh, size: 17),
                    label: const Text('再読み込み'),
                  ),
                  OutlinedButton.icon(
                    onPressed: ctrl.openModsFolder,
                    icon: const Icon(Icons.folder_open_outlined, size: 17),
                    label: const Text('MOD フォルダを開く'),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 14),

          // サマリー指標バー
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              _StatChip(
                icon: Icons.extension_outlined,
                label: '導入済み MOD',
                value: '${allMods.length} 件（有効 $enabledCount）',
                color: p.accent,
              ),
              _StatChip(
                icon: Icons.auto_awesome_outlined,
                label: 'リアクション演出',
                value: '$reactionCount 種',
                color: p.warning,
              ),
              _StatChip(
                icon: Icons.palette_outlined,
                label: '追加テーマ',
                value: '$themeCount 種',
                color: p.success,
              ),
              _StatChip(
                icon: Icons.web_outlined,
                label: '独自オーバーレイ',
                value: '$customOverlayCount 種',
                color: p.accent,
              ),
              _StatChip(
                icon: Icons.code_outlined,
                label: 'ロジックスクリプト',
                value: '$logicScriptCount 本',
                color: const Color(0xFFA855F7),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // フォルダパス ＆ 安全上の注意バー
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            decoration: BoxDecoration(
              color: p.subtleBg.withValues(alpha: 0.7),
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: p.border),
            ),
            child: Wrap(
              spacing: 12,
              runSpacing: 4,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Icon(Icons.folder_outlined, size: 15, color: p.textSecondary),
                Text(
                  'MOD 配置フォルダ:',
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: p.textSecondary,
                  ),
                ),
                SelectableText(
                  ctrl.modsFolderPath.isEmpty
                      ? '%APPDATA%\\Stosup\\mods'
                      : ctrl.modsFolderPath,
                  style: TextStyle(
                    fontFamily: 'Consolas',
                    fontSize: 12,
                    color: p.accent,
                  ),
                ),
                Text(
                  '（開発中はフォルダを直接編集して「再読み込み」で即時反映できます）',
                  style: TextStyle(fontSize: 11.5, color: p.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),

          // フィルタチップ
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              ChoiceChip(
                label: Text('すべて (${allMods.length})'),
                selected: _filter == _ModFilter.all,
                onSelected: (_) => setState(() => _filter = _ModFilter.all),
              ),
              ChoiceChip(
                label: const Text('リアクション演出'),
                selected: _filter == _ModFilter.reaction,
                onSelected: (_) =>
                    setState(() => _filter = _ModFilter.reaction),
              ),
              ChoiceChip(
                label: const Text('オーバーレイテーマ'),
                selected: _filter == _ModFilter.theme,
                onSelected: (_) => setState(() => _filter = _ModFilter.theme),
              ),
              ChoiceChip(
                label: const Text('独自オーバーレイ'),
                selected: _filter == _ModFilter.overlay,
                onSelected: (_) => setState(() => _filter = _ModFilter.overlay),
              ),
              ChoiceChip(
                label: const Text('ロジックスクリプト (.rhai)'),
                selected: _filter == _ModFilter.script,
                onSelected: (_) => setState(() => _filter = _ModFilter.script),
              ),
              if (errorCount > 0)
                ChoiceChip(
                  label: Text('エラー ($errorCount)'),
                  selected: _filter == _ModFilter.error,
                  onSelected: (_) => setState(() => _filter = _ModFilter.error),
                ),
            ],
          ),
          const SizedBox(height: 14),

          // MOD 一覧 or 空状態
          if (filtered.isEmpty)
            _buildEmptyState(context, allMods.isEmpty)
          else
            Column(
              children: filtered
                  .map((mod) => Padding(
                        padding: const EdgeInsets.only(bottom: 12),
                        child: _buildModCard(context, mod, obsEnabled),
                      ))
                  .toList(),
            ),

          const SizedBox(height: 16),
          _buildScriptLogConsole(context, ctrl),
        ],
      ),
    );
  }

  Widget _buildEmptyState(BuildContext context, bool trulyEmpty) {
    final p = context.palette;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.extension_outlined, size: 28, color: p.accent),
                const SizedBox(width: 10),
                Text(
                  trulyEmpty
                      ? 'まだ演出 MOD が導入されていません'
                      : 'この条件に一致する MOD はありません',
                  style: const TextStyle(
                      fontSize: 16, fontWeight: FontWeight.bold),
                ),
              ],
            ),
            const SizedBox(height: 10),
            Text(
              'Stosup の演出 MOD は 1 つのパッケージ形式（mod.json）で以下の 4 種類を拡張できます。',
              style: TextStyle(fontSize: 13, color: p.textSecondary),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 12,
              runSpacing: 10,
              children: [
                _GuideMiniCard(
                  icon: Icons.auto_awesome_outlined,
                  title: '1. リアクション独自演出',
                  desc:
                      'コメントやスパチャに反応する独自 Canvas / CSS / 動画エフェクトを追加。リアクションステージ画面からパラメータ調整もできます。',
                ),
                _GuideMiniCard(
                  icon: Icons.palette_outlined,
                  title: '2. オーバーレイ追加テーマ',
                  desc:
                      'トークモニター・エールボックスなど全 9 種 OBS オーバーレイへオリジナルのデザインテーマ（CSS / JS）を追加できます。',
                ),
                _GuideMiniCard(
                  icon: Icons.web_outlined,
                  title: '3. 独立カスタムオーバーレイ',
                  desc:
                      'StosupMod SDK でリアルタイムイベントを受け取り、盛り上がりゲージ等の完全オリジナル OBS 画面を作成できます。',
                ),
                _GuideMiniCard(
                  icon: Icons.code_outlined,
                  title: '4. ロジックスクリプト (.rhai)',
                  desc:
                      '軽量スクリプト言語 Rhai でコンボ判定・状態管理・動的演出発火・OBS シーン／ソース自動操作をコンパイル不要で記述できます。',
                ),
              ],
            ),
            const SizedBox(height: 14),
            Text(
              '制作ガイド: docs/MOD_DEVELOPMENT.md・docs/MOD_SCRIPT_GUIDE.md・docs/MOD_API_REFERENCE.md / 実働サンプル: examples/mods/',
              style: TextStyle(fontSize: 12, color: p.textMuted),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildModCard(
    BuildContext context,
    InstalledMod mod,
    bool obsEnabled,
  ) {
    final p = context.palette;
    final ctrl = widget.controller;
    final hasError = mod.loadError != null;

    return Card(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(
          color: hasError
              ? p.danger.withValues(alpha: 0.65)
              : (mod.enabled ? p.border : p.border.withValues(alpha: 0.5)),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // タイトル行
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  width: 40,
                  height: 40,
                  decoration: BoxDecoration(
                    color: (hasError ? p.danger : p.accent)
                        .withValues(alpha: 0.14),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    hasError
                        ? Icons.error_outline
                        : Icons.extension_outlined,
                    color: hasError ? p.danger : p.accent,
                    size: 22,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Wrap(
                        spacing: 8,
                        runSpacing: 4,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: [
                          Text(
                            mod.name,
                            style: const TextStyle(
                              fontSize: 15.5,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          if (mod.version.isNotEmpty)
                            _TagBadge('v${mod.version}', p.accent),
                          _TagBadge('ID: ${mod.id}', p.textSecondary),
                          if (mod.author.isNotEmpty)
                            Text(
                              'by ${mod.author}',
                              style: TextStyle(
                                  fontSize: 12, color: p.textSecondary),
                            ),
                          if (mod.license.isNotEmpty)
                            _TagBadge(mod.license, p.textMuted),
                        ],
                      ),
                      if (mod.description.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          mod.description,
                          style: TextStyle(
                              fontSize: 12.5, color: p.textSecondary),
                        ),
                      ],
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                if (mod.homepage.startsWith('http://') ||
                    mod.homepage.startsWith('https://'))
                  IconButton(
                    tooltip: '配布ページを開く',
                    onPressed: () => ctrl.openUrl(mod.homepage),
                    icon: const Icon(Icons.open_in_new, size: 18),
                  ),
                Switch(
                  value: mod.enabled && !hasError,
                  onChanged: hasError
                      ? null
                      : (v) => ctrl.setModEnabled(mod.id, v),
                ),
                IconButton(
                  tooltip: 'この MOD を削除',
                  onPressed: () => _confirmUninstall(mod),
                  icon: Icon(Icons.delete_outline, color: p.danger, size: 20),
                ),
              ],
            ),

            // 読み込みエラー詳細（開発者向けデバッグ表示）
            if (hasError) ...[
              const SizedBox(height: 10),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: p.danger.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: p.danger.withValues(alpha: 0.45)),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '読み込みエラー（フォルダ: ${mod.folderName}）',
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                        color: p.danger,
                      ),
                    ),
                    const SizedBox(height: 4),
                    SelectableText(
                      mod.loadError!,
                      style: const TextStyle(
                          fontFamily: 'Consolas', fontSize: 11.5),
                    ),
                  ],
                ),
              ),
            ],

            // 1. リアクション独自演出
            if (mod.reactionEffects.isNotEmpty) ...[
              const Divider(height: 22),
              Text(
                'リアクション独自演出 (${mod.reactionEffects.length})',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.bold,
                  color: p.warning,
                ),
              ),
              const SizedBox(height: 6),
              ...mod.reactionEffects.map(
                (ef) => Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 10, vertical: 8),
                    decoration: BoxDecoration(
                      color: p.subtleBg.withValues(alpha: 0.65),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: p.border),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.auto_awesome, size: 17, color: p.warning),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                '${ef.name}  (${ef.effectId})',
                                style: const TextStyle(
                                    fontWeight: FontWeight.w600, fontSize: 13),
                              ),
                              Text(
                                '${ef.description.isEmpty ? "リアクションステージ画面の「カスタム MOD」からトリガー登録できます" : ef.description}'
                                '  /  設定項目: ${ef.params.length}件',
                                style: TextStyle(
                                    fontSize: 11.5, color: p.textSecondary),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        OutlinedButton.icon(
                          style: OutlinedButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                          ),
                          onPressed: (mod.enabled && !hasError && obsEnabled)
                              ? () => ctrl.testModReactionEffect(
                                    modId: mod.id,
                                    effectId: ef.effectId,
                                    previewOnly: true,
                                  )
                              : null,
                          icon: const Icon(Icons.play_circle_outline, size: 15),
                          label: const Text('プレビュー再生'),
                        ),
                        const SizedBox(width: 6),
                        TextButton.icon(
                          style: TextButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                          ),
                          onPressed: (mod.enabled && !hasError && obsEnabled)
                              ? () => ctrl.openUrl(
                                    '${ctrl.overlayBaseUrl}/overlay/preview/media',
                                  )
                              : null,
                          icon: const Icon(Icons.open_in_new, size: 14),
                          label: const Text('画面を開く'),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],

            // 2. オーバーレイ追加テーマ
            if (mod.overlayThemes.isNotEmpty) ...[
              const Divider(height: 22),
              Text(
                'オーバーレイ追加テーマ (${mod.overlayThemes.length})',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.bold,
                  color: p.success,
                ),
              ),
              const SizedBox(height: 6),
              ...mod.overlayThemes.map(
                (th) => Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 10, vertical: 8),
                    decoration: BoxDecoration(
                      color: p.subtleBg.withValues(alpha: 0.65),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: p.border),
                    ),
                    child: Row(
                      children: [
                        Container(
                          width: 22,
                          height: 22,
                          decoration: BoxDecoration(
                            color: AppTheme.parseCssHex(th.previewBg),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(
                              color: AppTheme.parseCssHex(th.previewBorder),
                              width: 2.5,
                            ),
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                '${th.name}  (${th.fullThemeId})',
                                style: const TextStyle(
                                    fontWeight: FontWeight.w600, fontSize: 13),
                              ),
                              Text(
                                '${th.description.isEmpty ? "オーバーレイスタジオの各タブから選択して適用できます" : th.description}'
                                '  /  対象: ${th.targets.join(", ")}',
                                style: TextStyle(
                                    fontSize: 11.5, color: p.textSecondary),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],

            // 3. 独立カスタムオーバーレイ
            if (mod.customOverlays.isNotEmpty) ...[
              const Divider(height: 22),
              Text(
                '独立カスタムオーバーレイ (${mod.customOverlays.length})',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.bold,
                  color: p.accent,
                ),
              ),
              const SizedBox(height: 6),
              ...mod.customOverlays.map((ov) {
                final prodUrl = '${ctrl.overlayBaseUrl}${ov.overlayPath}';
                final prevUrl = '${ctrl.overlayBaseUrl}${ov.previewPath}';
                return Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 10, vertical: 8),
                    decoration: BoxDecoration(
                      color: p.subtleBg.withValues(alpha: 0.65),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: p.border),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(Icons.web_outlined,
                                size: 17, color: p.accent),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    '${ov.name}  (推奨 ${ov.recommendedWidth}×${ov.recommendedHeight})',
                                    style: const TextStyle(
                                        fontWeight: FontWeight.w600,
                                        fontSize: 13),
                                  ),
                                  if (ov.description.isNotEmpty)
                                    Text(
                                      ov.description,
                                      style: TextStyle(
                                          fontSize: 11.5,
                                          color: p.textSecondary),
                                    ),
                                ],
                              ),
                            ),
                            FilledButton.icon(
                              style: FilledButton.styleFrom(
                                visualDensity: VisualDensity.compact,
                              ),
                              onPressed: (mod.enabled && !hasError)
                                  ? () => _copyUrl(prodUrl, ov.name)
                                  : null,
                              icon: const Icon(Icons.copy, size: 14),
                              label: const Text('OBS URL コピー'),
                            ),
                            const SizedBox(width: 6),
                            OutlinedButton.icon(
                              style: OutlinedButton.styleFrom(
                                visualDensity: VisualDensity.compact,
                              ),
                              onPressed:
                                  (mod.enabled && !hasError && obsEnabled)
                                      ? () => ctrl.openUrl(prevUrl)
                                      : null,
                              icon: const Icon(Icons.open_in_new, size: 14),
                              label: const Text('プレビュー'),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        SelectableText(
                          prodUrl,
                          style: TextStyle(
                            fontFamily: 'Consolas',
                            fontSize: 11.5,
                            color: p.accent,
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              }),
            ],

            // 4. ロジックスクリプト (.rhai)
            if (mod.logicScripts.isNotEmpty) ...[
              const Divider(height: 22),
              const Text(
                'ロジックスクリプト (.rhai)',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.bold,
                  color: Color(0xFFA855F7),
                ),
              ),
              const SizedBox(height: 6),
              ...mod.logicScripts.map(
                (sc) => Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 10, vertical: 8),
                    decoration: BoxDecoration(
                      color: p.subtleBg.withValues(alpha: 0.65),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(color: p.border),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.code_outlined,
                            size: 17, color: Color(0xFFA855F7)),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Wrap(
                                spacing: 6,
                                runSpacing: 4,
                                crossAxisAlignment: WrapCrossAlignment.center,
                                children: [
                                  Text(
                                    '${sc.name}  (${sc.entry})',
                                    style: const TextStyle(
                                      fontWeight: FontWeight.w600,
                                      fontSize: 13,
                                    ),
                                  ),
                                  ...sc.events.map(
                                    (ev) => _TagBadge(
                                      _scriptEventLabel(ev),
                                      const Color(0xFFA855F7),
                                    ),
                                  ),
                                ],
                              ),
                              if (sc.description.isNotEmpty) ...[
                                const SizedBox(height: 2),
                                Text(
                                  sc.description,
                                  style: TextStyle(
                                    fontSize: 11.5,
                                    color: p.textSecondary,
                                  ),
                                ),
                              ],
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        OutlinedButton.icon(
                          style: OutlinedButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                          ),
                          onPressed: (mod.enabled && !hasError)
                              ? () => ctrl.testModLogicScript(
                                    modId: mod.id,
                                    scriptId: sc.scriptId,
                                    previewOnly: true,
                                  )
                              : null,
                          icon: const Icon(Icons.play_arrow, size: 15),
                          label: const Text('テスト実行'),
                        ),
                        const SizedBox(width: 4),
                        IconButton(
                          tooltip: 'テストコメントを指定して実行',
                          visualDensity: VisualDensity.compact,
                          onPressed: (mod.enabled && !hasError)
                              ? () => _showScriptTestDialog(mod, sc)
                              : null,
                          icon: const Icon(Icons.tune, size: 16),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildScriptLogConsole(BuildContext context, StosupController ctrl) {
    final p = context.palette;
    final logs = ctrl.modScriptLogs;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.terminal_outlined,
                    size: 18, color: Color(0xFFA855F7)),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'スクリプト実行ログ（デバッグコンソール） (${logs.length})',
                    style: const TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
                OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                    visualDensity: VisualDensity.compact,
                  ),
                  onPressed: () => ctrl.refreshModScriptLogs(),
                  icon: const Icon(Icons.refresh, size: 15),
                  label: const Text('ログ更新'),
                ),
                const SizedBox(width: 6),
                TextButton.icon(
                  style: TextButton.styleFrom(
                    visualDensity: VisualDensity.compact,
                  ),
                  onPressed: logs.isEmpty ? null : ctrl.clearModScriptLogs,
                  icon: const Icon(Icons.delete_sweep_outlined, size: 15),
                  label: const Text('クリア'),
                ),
                IconButton(
                  tooltip: _logsExpanded ? '折りたたむ' : '展開する',
                  visualDensity: VisualDensity.compact,
                  onPressed: () =>
                      setState(() => _logsExpanded = !_logsExpanded),
                  icon: Icon(
                    _logsExpanded ? Icons.expand_less : Icons.expand_more,
                    size: 18,
                  ),
                ),
              ],
            ),
            if (_logsExpanded) ...[
              const SizedBox(height: 8),
              if (logs.isEmpty)
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: p.subtleBg.withValues(alpha: 0.6),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: p.border),
                  ),
                  child: Text(
                    'まだ実行ログはありません。ロジックスクリプト（.rhai）内の log(...) 出力・発火したアクション・実行時エラーがここに新しい順で表示されます。',
                    style: TextStyle(fontSize: 12, color: p.textMuted),
                  ),
                )
              else
                Container(
                  width: double.infinity,
                  constraints: const BoxConstraints(maxHeight: 240),
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: p.subtleBg.withValues(alpha: 0.75),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: p.border),
                  ),
                  child: ListView.separated(
                    shrinkWrap: true,
                    itemCount: logs.length,
                    separatorBuilder: (_, _) =>
                        Divider(height: 8, color: p.border.withValues(alpha: 0.5)),
                    itemBuilder: (ctx, i) {
                      final entry = logs[i];
                      final dt = DateTime.fromMillisecondsSinceEpoch(
                        entry.timestampMs.toInt(),
                      );
                      final ts =
                          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
                      final Color badgeColor;
                      switch (entry.level) {
                        case 'error':
                          badgeColor = p.danger;
                          break;
                        case 'warn':
                          badgeColor = p.warning;
                          break;
                        case 'action':
                          badgeColor = p.success;
                          break;
                        default:
                          badgeColor = p.accent;
                      }
                      return Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            ts,
                            style: TextStyle(
                              fontFamily: 'Consolas',
                              fontSize: 11,
                              color: p.textMuted,
                            ),
                          ),
                          const SizedBox(width: 6),
                          _TagBadge(entry.level.toUpperCase(), badgeColor),
                          const SizedBox(width: 6),
                          Text(
                            '${entry.modId}:${entry.scriptId}',
                            style: TextStyle(
                              fontFamily: 'Consolas',
                              fontSize: 11,
                              color: p.textSecondary,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: SelectableText(
                              entry.message,
                              style: const TextStyle(
                                fontFamily: 'Consolas',
                                fontSize: 11.5,
                              ),
                            ),
                          ),
                        ],
                      );
                    },
                  ),
                ),
            ],
          ],
        ),
      ),
    );
  }
}

class _StatChip extends StatelessWidget {
  const _StatChip({
    required this.icon,
    required this.label,
    required this.value,
    required this.color,
  });

  final IconData icon;
  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: p.cardBg,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: p.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 17, color: color),
          const SizedBox(width: 8),
          Text('$label: ',
              style: TextStyle(fontSize: 12, color: p.textSecondary)),
          Text(value,
              style:
                  const TextStyle(fontSize: 12.5, fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }
}

class _TagBadge extends StatelessWidget {
  const _TagBadge(this.text, this.color);

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }
}

class _GuideMiniCard extends StatelessWidget {
  const _GuideMiniCard({
    required this.icon,
    required this.title,
    required this.desc,
  });

  final IconData icon;
  final String title;
  final String desc;

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    return Container(
      width: 290,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: p.subtleBg.withValues(alpha: 0.65),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: p.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 17, color: p.accent),
              const SizedBox(width: 6),
              Text(title,
                  style: const TextStyle(
                      fontWeight: FontWeight.bold, fontSize: 13)),
            ],
          ),
          const SizedBox(height: 6),
          Text(desc, style: TextStyle(fontSize: 12, color: p.textSecondary)),
        ],
      ),
    );
  }
}
