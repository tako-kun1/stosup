import 'dart:async';
import 'dart:io';

import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import 'markdown_notes_view.dart';

/// GitHub Releases からの自動アップデート確認・ダウンロード・適用ダイアログ
class UpdateDialog extends StatefulWidget {
  const UpdateDialog({
    super.key,
    required this.controller,
    required this.info,
  });

  final StosupController controller;
  final UpdateInfo info;

  @override
  State<UpdateDialog> createState() => _UpdateDialogState();
}

class _UpdateDialogState extends State<UpdateDialog> {
  StreamSubscription<UpdateDownloadProgress>? _sub;
  UpdateDownloadProgress? _progress;
  bool _isDownloading = false;
  bool _silentInstall = true;
  String? _error;

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  void _startDownload() {
    if (widget.info.isPrerelease || widget.info.installerUrl == null) {
      widget.controller.openUrl(widget.info.htmlUrl);
      return;
    }

    setState(() {
      _isDownloading = true;
      _error = null;
    });

    _sub = widget.controller.downloadUpdate(widget.info).listen(
      (p) async {
        if (!mounted) return;
        setState(() {
          _progress = p;
          if (p.error != null) {
            _error = p.error;
            _isDownloading = false;
          }
        });

        if (p.status == 'ready' && p.filePath != null) {
          try {
            await widget.controller.launchUpdateInstaller(
              p.filePath!,
              silent: _silentInstall,
            );
            exit(0);
          } catch (e) {
            if (!mounted) return;
            setState(() {
              _error = e.toString();
              _isDownloading = false;
            });
          }
        }
      },
      onError: (e) {
        if (!mounted) return;
        setState(() {
          _error = e.toString();
          _isDownloading = false;
        });
      },
    );
  }

  String _formatBytes(BigInt bytes) {
    final mb = bytes.toDouble() / (1024 * 1024);
    return '${mb.toStringAsFixed(1)} MB';
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final info = widget.info;
    final isPre = info.isPrerelease;
    double? fraction;
    if (_progress != null &&
        _progress!.totalBytes != null &&
        _progress!.totalBytes! > BigInt.zero) {
      fraction = _progress!.downloadedBytes.toDouble() /
          _progress!.totalBytes!.toDouble();
    }

    return AlertDialog(
      title: Row(
        children: [
          Icon(
            isPre ? Icons.campaign_outlined : Icons.system_update_alt,
            color: isPre ? p.warning : p.accent,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  isPre
                      ? '先行版（pre）のお知らせ (${info.releaseName})'
                      : 'アップデート利用可能 (${info.releaseName})',
                  style: const TextStyle(
                      fontSize: 16.5, fontWeight: FontWeight.bold),
                ),
                Text(
                  'v${info.currentVersion} → v${info.latestVersion}',
                  style: TextStyle(
                    fontSize: 12.5,
                    color: isPre ? p.warning : p.success,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      content: SizedBox(
        width: 580,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (isPre) ...[
              Container(
                width: double.infinity,
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                decoration: BoxDecoration(
                  color: p.warning.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: p.warning.withValues(alpha: 0.45)),
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(Icons.info_outline, color: p.warning, size: 18),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'pre が付く先行バージョンは自動更新を行いません（利用可能のアナウンスのみ表示しています）。'
                        'お試しになりたい場合のみ GitHub リリースページから手動でご利用ください。',
                        style: TextStyle(
                          fontSize: 12.5,
                          color: p.textPrimary,
                          height: 1.45,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 12),
            ],
            const Text('変更内容（リリースノート）',
                style: TextStyle(fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Container(
              height: 250,
              width: double.infinity,
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: p.windowBg,
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: p.border),
              ),
              child: SingleChildScrollView(
                child: MarkdownNotesView(
                  markdown: info.releaseNotes.isEmpty
                      ? 'GitHub リリースページをご確認ください。'
                      : info.releaseNotes,
                ),
              ),
            ),
            const SizedBox(height: 10),
            if (!isPre && !_isDownloading)
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                value: _silentInstall,
                onChanged: (v) => setState(() => _silentInstall = v ?? true),
                title: const Text('サイレント更新（完了後に自動で再起動してリリースノートを表示）'),
                controlAffinity: ListTileControlAffinity.leading,
              ),
            if (_isDownloading && _progress != null) ...[
              const SizedBox(height: 8),
              LinearProgressIndicator(value: fraction),
              const SizedBox(height: 6),
              Text(
                _progress!.status == 'verifying'
                    ? 'SHA-256 チェックサムを検証中...'
                    : 'ダウンロード中: ${_formatBytes(_progress!.downloadedBytes)}'
                        '${_progress!.totalBytes != null ? " / ${_formatBytes(_progress!.totalBytes!)}" : ""}',
                style: TextStyle(fontSize: 12.5, color: p.textSecondary),
              ),
            ],
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(_error!, style: TextStyle(color: p.danger)),
            ],
          ],
        ),
      ),
      actions: isPre
          ? [
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('閉じる'),
              ),
              FilledButton.icon(
                onPressed: () => widget.controller.openUrl(info.htmlUrl),
                icon: const Icon(Icons.open_in_new, size: 17),
                label: const Text('GitHub リリースページで確認'),
              ),
            ]
          : [
              TextButton(
                onPressed: _isDownloading
                    ? null
                    : () => widget.controller.openUrl(info.htmlUrl),
                child: const Text('GitHub で開く'),
              ),
              TextButton(
                onPressed:
                    _isDownloading ? null : () => Navigator.of(context).pop(),
                child: const Text('あとで'),
              ),
              FilledButton.icon(
                onPressed: _isDownloading ? null : _startDownload,
                icon: const Icon(Icons.download, size: 18),
                label: Text(
                  info.installerUrl != null ? 'ダウンロードして更新' : 'リリースページを開く',
                ),
              ),
            ],
    );
  }
}
