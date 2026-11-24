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
    if (widget.info.installerUrl == null) {
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
            // インストーラーが安全に置き換えられるようアプリを終了
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
    final info = widget.info;
    double? fraction;
    if (_progress != null &&
        _progress!.totalBytes != null &&
        _progress!.totalBytes! > BigInt.zero) {
      fraction =
          _progress!.downloadedBytes.toDouble() / _progress!.totalBytes!.toDouble();
    }

    return AlertDialog(
      backgroundColor: AppTheme.surfaceDark,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(18),
        side: const BorderSide(color: AppTheme.accentEmerald, width: 1.5),
      ),
      title: Row(
        children: [
          const Icon(Icons.system_update_alt, color: AppTheme.accentEmerald),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '新しいバージョンが利用可能です (${info.releaseName})',
                  style: const TextStyle(
                      fontSize: 17, fontWeight: FontWeight.bold),
                ),
                Text(
                  'v${info.currentVersion} → v${info.latestVersion}',
                  style: const TextStyle(
                      fontSize: 13, color: AppTheme.accentEmerald),
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
            const Text('変更内容（リリースノート）:',
                style: TextStyle(fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Container(
              height: 260,
              width: double.infinity,
              padding: const EdgeInsets.all(14),
              decoration: BoxDecoration(
                color: AppTheme.bgDark,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: AppTheme.borderDark),
              ),
              child: SingleChildScrollView(
                child: MarkdownNotesView(
                  markdown: info.releaseNotes.isEmpty
                      ? 'GitHub リリースページをご確認ください。'
                      : info.releaseNotes,
                ),
              ),
            ),
            const SizedBox(height: 12),
            if (!_isDownloading)
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
                style: const TextStyle(fontSize: 12.5, color: Colors.white70),
              ),
            ],
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(_error!, style: const TextStyle(color: AppTheme.accentRose)),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _isDownloading
              ? null
              : () => widget.controller.openUrl(info.htmlUrl),
          child: const Text('GitHub で開く'),
        ),
        TextButton(
          onPressed: _isDownloading ? null : () => Navigator.of(context).pop(),
          child: const Text('あとで'),
        ),
        FilledButton.icon(
          onPressed: _isDownloading ? null : _startDownload,
          icon: const Icon(Icons.download),
          label: Text(
            info.installerUrl != null ? 'ダウンロードしてアップデート' : 'リリースページを開く',
          ),
        ),
      ],
    );
  }
}
