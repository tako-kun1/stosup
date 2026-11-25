import 'package:material_ui/material_ui.dart';

import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';
import 'markdown_notes_view.dart';

/// 起動時に表示する利用規約（使用許諾契約書）の必須同意画面
/// 同意ボタンを押すまでメイン画面へ進まず、不同意時はアプリを強制終了する
class LicenseAgreementScreen extends StatefulWidget {
  const LicenseAgreementScreen({
    super.key,
    required this.controller,
    this.onDecline,
  });

  final StosupController controller;
  final VoidCallback? onDecline;

  @override
  State<LicenseAgreementScreen> createState() => _LicenseAgreementScreenState();
}

class _LicenseAgreementScreenState extends State<LicenseAgreementScreen> {
  bool _agreed = false;
  bool _submitting = false;
  bool _refreshing = false;
  String? _lastHash;

  String _sourceLabel(String source) {
    switch (source) {
      case 'online':
        return 'GitHub 最新版';
      case 'cached':
        return '取得済み最新版';
      default:
        return 'アプリ同梱版';
    }
  }

  Future<void> _refreshOnline() async {
    setState(() => _refreshing = true);
    try {
      await widget.controller.refreshLicenseStatus(checkOnline: true);
    } finally {
      if (mounted) setState(() => _refreshing = false);
    }
  }

  Future<void> _accept() async {
    if (!_agreed || _submitting) return;
    setState(() => _submitting = true);
    try {
      await widget.controller.acceptLicenseAgreement();
    } finally {
      if (mounted) setState(() => _submitting = false);
    }
  }

  void _decline() {
    if (widget.onDecline != null) {
      widget.onDecline!();
    } else {
      widget.controller.declineLicenseAndExit();
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final status = widget.controller.licenseStatus;
    if (status == null) {
      return const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      );
    }

    // オンライン再取得で規約内容が切り替わったら同意チェックをリセットする
    if (_lastHash != null && _lastHash != status.contentHash) {
      _agreed = false;
    }
    _lastHash = status.contentHash;

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 820, maxHeight: 740),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(22),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // ヘッダー
                    Row(
                      children: [
                        Container(
                          width: 42,
                          height: 42,
                          decoration: BoxDecoration(
                            color: (status.isUpdatedLicense
                                    ? p.warning
                                    : p.accent)
                                .withValues(alpha: 0.16),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: Icon(
                            status.isUpdatedLicense
                                ? Icons.gavel_rounded
                                : Icons.verified_user_outlined,
                            color: status.isUpdatedLicense
                                ? p.warning
                                : p.accent,
                            size: 24,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                status.isUpdatedLicense
                                    ? 'Stosup 利用規約が更新されました（再同意が必要です）'
                                    : 'Stosup 利用規約への同意',
                                style: const TextStyle(
                                  fontSize: 18,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              const SizedBox(height: 2),
                              Text(
                                '本ソフトウェアをご利用いただくには、以下の利用規約（使用許諾契約書）への同意が必要です。',
                                style: TextStyle(
                                  fontSize: 12.5,
                                  color: p.textSecondary,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 4),
                          decoration: BoxDecoration(
                            color: p.accent.withValues(alpha: 0.14),
                            borderRadius: BorderRadius.circular(5),
                          ),
                          child: Text(
                            status.revisionLabel,
                            style: TextStyle(
                              fontSize: 11.5,
                              fontWeight: FontWeight.bold,
                              color: p.accent,
                            ),
                          ),
                        ),
                        const SizedBox(width: 6),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 4),
                          decoration: BoxDecoration(
                            color: p.subtleBg,
                            borderRadius: BorderRadius.circular(5),
                            border: Border.all(color: p.border),
                          ),
                          child: Text(
                            _sourceLabel(status.source),
                            style: TextStyle(
                              fontSize: 11,
                              color: p.textSecondary,
                            ),
                          ),
                        ),
                        const SizedBox(width: 4),
                        IconButton(
                          tooltip: 'GitHub から最新の規約を再取得',
                          onPressed: _refreshing ? null : _refreshOnline,
                          icon: _refreshing
                              ? const SizedBox(
                                  width: 16,
                                  height: 16,
                                  child:
                                      CircularProgressIndicator(strokeWidth: 2),
                                )
                              : const Icon(Icons.refresh, size: 19),
                        ),
                      ],
                    ),

                    if (status.isUpdatedLicense) ...[
                      const SizedBox(height: 12),
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.symmetric(
                            horizontal: 12, vertical: 9),
                        decoration: BoxDecoration(
                          color: p.warning.withValues(alpha: 0.14),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(
                            color: p.warning.withValues(alpha: 0.45),
                          ),
                        ),
                        child: Row(
                          children: [
                            Icon(Icons.info_outline,
                                size: 17, color: p.warning),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                '前回同意された利用規約（${status.acceptedRevision ?? "旧版"}）から内容が更新されました。変更内容をご確認のうえ再度同意してください。',
                                style: const TextStyle(fontSize: 12.5),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],

                    const SizedBox(height: 14),

                    // 規約本文スクロール領域
                    Expanded(
                      child: Container(
                        width: double.infinity,
                        padding: const EdgeInsets.all(16),
                        decoration: BoxDecoration(
                          color: p.subtleBg.withValues(alpha: 0.55),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: p.border),
                        ),
                        child: Scrollbar(
                          thumbVisibility: true,
                          child: SingleChildScrollView(
                            padding: const EdgeInsets.only(right: 12),
                            child: MarkdownNotesView(
                              markdown: status.markdownBody,
                            ),
                          ),
                        ),
                      ),
                    ),

                    const SizedBox(height: 12),

                    // 同意チェックボックス
                    CheckboxListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      controlAffinity: ListTileControlAffinity.leading,
                      value: _agreed,
                      onChanged: _submitting
                          ? null
                          : (v) => setState(() => _agreed = v ?? false),
                      title: const Text(
                        '上記の利用規約（使用許諾契約書）をすべて読み、内容に同意します',
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          fontSize: 13.5,
                        ),
                      ),
                    ),
                    const SizedBox(height: 10),

                    // アクションボタン行（狭幅ウィンドウでも溢れない Wrap 配置）
                    SizedBox(
                      width: double.infinity,
                      child: Wrap(
                        spacing: 10,
                        runSpacing: 8,
                        alignment: WrapAlignment.spaceBetween,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: [
                          TextButton.icon(
                            onPressed: () => widget.controller.openUrl(
                              'https://github.com/tako-kun1/stosup/blob/main/License.md',
                            ),
                            icon: const Icon(Icons.open_in_new, size: 15),
                            label: const Text('GitHub で規約を開く'),
                          ),
                          Wrap(
                            spacing: 10,
                            runSpacing: 8,
                            children: [
                              OutlinedButton.icon(
                                style: OutlinedButton.styleFrom(
                                  foregroundColor: p.danger,
                                  side: BorderSide(color: p.danger),
                                ),
                                onPressed: _submitting ? null : _decline,
                                icon: const Icon(Icons.close, size: 17),
                                label: const Text('同意しない（アプリを終了）'),
                              ),
                              FilledButton.icon(
                                onPressed:
                                    (_agreed && !_submitting) ? _accept : null,
                                icon: _submitting
                                    ? const SizedBox(
                                        width: 15,
                                        height: 15,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                          color: Colors.white,
                                        ),
                                      )
                                    : const Icon(Icons.check_circle_outline,
                                        size: 18),
                                label: const Text('同意して Stosup を起動する'),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// 設定画面「ソフトウェア情報」から利用規約の閲覧 ＆ 最新規約のオンライン確認を行うダイアログ
class LicenseViewerDialog extends StatefulWidget {
  const LicenseViewerDialog({
    super.key,
    required this.controller,
  });

  final StosupController controller;

  @override
  State<LicenseViewerDialog> createState() => _LicenseViewerDialogState();
}

class _LicenseViewerDialogState extends State<LicenseViewerDialog> {
  bool _checking = false;

  String _formatAcceptedAt(int? ms) {
    if (ms == null) return '未同意';
    final dt = DateTime.fromMillisecondsSinceEpoch(ms).toLocal();
    final m = dt.month.toString().padLeft(2, '0');
    final d = dt.day.toString().padLeft(2, '0');
    final h = dt.hour.toString().padLeft(2, '0');
    final min = dt.minute.toString().padLeft(2, '0');
    return '${dt.year}/$m/$d $h:$min';
  }

  Future<void> _checkLatestOnline() async {
    setState(() => _checking = true);
    try {
      await widget.controller.refreshLicenseStatus(
        checkOnline: true,
        notifyResult: true,
      );
      if (mounted && widget.controller.needsLicenseAcceptance) {
        // 規約が更新されていたら閲覧ダイアログを閉じて強制再同意画面へ移行
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) setState(() => _checking = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final status = widget.controller.licenseStatus;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.gavel_rounded, color: p.accent, size: 22),
          const SizedBox(width: 8),
          const Expanded(child: Text('Stosup 利用規約（使用許諾契約書）')),
          if (status != null)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
              decoration: BoxDecoration(
                color: p.accent.withValues(alpha: 0.14),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                status.revisionLabel,
                style: TextStyle(
                  fontSize: 11.5,
                  fontWeight: FontWeight.bold,
                  color: p.accent,
                ),
              ),
            ),
        ],
      ),
      content: SizedBox(
        width: 720,
        height: 480,
        child: status == null
            ? const Center(child: CircularProgressIndicator())
            : Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '同意済み版: ${status.acceptedRevision ?? "未同意"}（同意日時: ${_formatAcceptedAt(status.acceptedAtMs)}）',
                    style: TextStyle(fontSize: 12, color: p.textSecondary),
                  ),
                  const SizedBox(height: 10),
                  Expanded(
                    child: Container(
                      width: double.infinity,
                      padding: const EdgeInsets.all(14),
                      decoration: BoxDecoration(
                        color: p.subtleBg.withValues(alpha: 0.55),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: p.border),
                      ),
                      child: Scrollbar(
                        thumbVisibility: true,
                        child: SingleChildScrollView(
                          padding: const EdgeInsets.only(right: 10),
                          child: MarkdownNotesView(
                            markdown: status.markdownBody,
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
      ),
      actions: [
        TextButton.icon(
          onPressed: () => widget.controller.openUrl(
            'https://github.com/tako-kun1/stosup/blob/main/License.md',
          ),
          icon: const Icon(Icons.open_in_new, size: 15),
          label: const Text('GitHub で開く'),
        ),
        OutlinedButton.icon(
          onPressed: _checking ? null : _checkLatestOnline,
          icon: _checking
              ? const SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.refresh, size: 16),
          label: const Text('最新の規約を確認'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('閉じる'),
        ),
      ],
    );
  }
}
