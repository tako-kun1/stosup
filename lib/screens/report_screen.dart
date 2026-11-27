import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';
import '../state/stosup_controller.dart';
import '../theme/app_theme.dart';

/// オンエアレポート（配信後の振り返り・盛り上がり推移グラフ・切り抜きピーク・CSV/JSON出力）
class ReportScreen extends StatefulWidget {
  const ReportScreen({super.key, required this.controller});

  final StosupController controller;

  @override
  State<ReportScreen> createState() => _ReportScreenState();
}

class _ReportScreenState extends State<ReportScreen> {
  String? _selectedSessionId;
  int? _selectedPeakMinute;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      await widget.controller.refreshStreamSessions();
      if (!mounted) return;
      final list = widget.controller.streamSessions;
      if (list.isNotEmpty) {
        _selectSession(list.first.sessionId);
      }
    });
  }

  Future<void> _selectSession(String sessionId) async {
    setState(() {
      _selectedSessionId = sessionId;
      _selectedPeakMinute = null;
    });
    await widget.controller.loadOnAirReport(sessionId);
  }

  String _formatDateTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    final y = dt.year.toString();
    final mo = dt.month.toString().padLeft(2, '0');
    final d = dt.day.toString().padLeft(2, '0');
    final h = dt.hour.toString().padLeft(2, '0');
    final mi = dt.minute.toString().padLeft(2, '0');
    return '$y/$mo/$d $h:$mi';
  }

  @override
  Widget build(BuildContext context) {
    final p = context.palette;
    final ctrl = widget.controller;
    final sessions = ctrl.streamSessions;
    final report = ctrl.currentReport;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // 左側：配信セッション一覧
        SizedBox(
          width: 280,
          child: Material(
            color: p.sidebarBg,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(14, 16, 10, 10),
                  child: Row(
                    children: [
                      const Expanded(
                        child: Text(
                          '配信履歴一覧',
                          style: TextStyle(
                              fontSize: 15, fontWeight: FontWeight.bold),
                        ),
                      ),
                      IconButton(
                        tooltip: '最新の履歴を取得',
                        onPressed: ctrl.refreshStreamSessions,
                        icon: const Icon(Icons.refresh, size: 18),
                      ),
                    ],
                  ),
                ),
                const Divider(height: 1),
                Expanded(
                  child: sessions.isEmpty
                      ? Center(
                          child: Padding(
                            padding: const EdgeInsets.all(16),
                            child: Text(
                              '配信に接続してコメントを受信すると、自動でオンエアレポートが記録されます。',
                              style: TextStyle(
                                  fontSize: 12.5, color: p.textSecondary),
                              textAlign: TextAlign.center,
                            ),
                          ),
                        )
                      : ListView.separated(
                          itemCount: sessions.length,
                          separatorBuilder: (_, _) => const Divider(height: 1),
                          itemBuilder: (_, idx) {
                            final s = sessions[idx];
                            final selected = s.sessionId == _selectedSessionId;
                            return ListTile(
                              selected: selected,
                              selectedTileColor:
                                  p.accent.withValues(alpha: 0.14),
                              dense: true,
                              title: Text(
                                s.title.isEmpty ? s.streamId : s.title,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                    fontWeight: FontWeight.bold, fontSize: 13),
                              ),
                              subtitle: Text(
                                '${_formatDateTime(s.startedAtMs.toInt())}\n${s.totalComments}コメ / ${s.uniqueUsers}人 / 初見${s.firstTimeUsers}',
                                style: TextStyle(
                                    fontSize: 11, color: p.textSecondary),
                              ),
                              isThreeLine: true,
                              trailing: IconButton(
                                tooltip: 'このセッション履歴を削除',
                                icon: Icon(Icons.delete_outline,
                                    size: 17, color: p.textMuted),
                                onPressed: () =>
                                    ctrl.deleteStreamSession(s.sessionId),
                              ),
                              onTap: () => _selectSession(s.sessionId),
                            );
                          },
                        ),
                ),
              ],
            ),
          ),
        ),
        const VerticalDivider(width: 1),

        // 右側：オンエアレポート本体
        Expanded(
          child: report == null
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.analytics_outlined,
                          size: 48, color: p.textMuted),
                      const SizedBox(height: 10),
                      Text(
                        '左の配信履歴からセッションを選択してください',
                        style: TextStyle(color: p.textSecondary, fontSize: 14),
                      ),
                    ],
                  ),
                )
              : SingleChildScrollView(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Wrap(
                        spacing: 10,
                        runSpacing: 8,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: [
                          Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text(
                                'オンエアレポート',
                                style: TextStyle(
                                    fontSize: 20, fontWeight: FontWeight.bold),
                              ),
                              Text(
                                '${report.session.title}  (${_formatDateTime(report.session.startedAtMs.toInt())} / 約${report.durationMinutes}分)',
                                style: TextStyle(
                                    fontSize: 12.5, color: p.textSecondary),
                              ),
                            ],
                          ),
                          const SizedBox(width: 12),
                          OutlinedButton.icon(
                            onPressed: () =>
                                ctrl.exportReportCsv(report.session.sessionId),
                            icon: const Icon(Icons.table_view_outlined,
                                size: 16),
                            label: const Text('CSV 出力 (Excel対応)'),
                          ),
                          OutlinedButton.icon(
                            onPressed: () => ctrl
                                .exportReportJson(report.session.sessionId),
                            icon: const Icon(Icons.data_object, size: 16),
                            label: const Text('JSON 出力'),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),

                      // KPI サマリーカード
                      Wrap(
                        spacing: 10,
                        runSpacing: 10,
                        children: [
                          _kpiCard(p, '総コメント数',
                              '${report.session.totalComments} 件', p.accent),
                          _kpiCard(p, 'ユニーク視聴者',
                              '${report.session.uniqueUsers} 人', p.success),
                          _kpiCard(p, '初見さん',
                              '${report.session.firstTimeUsers} 人', p.warning),
                          _kpiCard(
                            p,
                            '最大 / 平均 分速',
                            '${report.maxCommentsPerMin} / ${(report.avgCommentsPerMinTenths / 10).toStringAsFixed(1)} コメ/分',
                            const Color(0xFF0EA5E9),
                          ),
                          _kpiCard(
                            p,
                            'エール件数',
                            report.monetizationTotals.isEmpty
                                ? '${report.session.monetizationCount} 件'
                                : '${report.session.monetizationCount} 件 (${report.monetizationTotals.join(" / ")})',
                            const Color(0xFFEAB308),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),

                      // 盛り上がり推移グラフ
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Row(
                                children: [
                                  Icon(Icons.show_chart, size: 19),
                                  SizedBox(width: 8),
                                  Text(
                                    '盛り上がり推移グラフ（1分あたりコメント数）',
                                    style: TextStyle(
                                        fontWeight: FontWeight.bold,
                                        fontSize: 14.5),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 12),
                              SizedBox(
                                width: double.infinity,
                                height: 190,
                                child: CustomPaint(
                                  painter: _MomentumChartPainter(
                                    buckets: report.buckets,
                                    accentColor: p.accent,
                                    monetizationColor: p.warning,
                                    gridColor: p.border,
                                    labelColor: p.textSecondary,
                                    highlightMinute: _selectedPeakMinute,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),

                      // 盛り上がりピーク TOP 5（切り抜きポイント）
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Text(
                                '盛り上がりピーク TOP 5（切り抜きポイント候補）',
                                style: TextStyle(
                                    fontWeight: FontWeight.bold,
                                    fontSize: 14.5),
                              ),
                              const SizedBox(height: 8),
                              if (report.peaks.isEmpty)
                                Text('ピークデータがありません',
                                    style: TextStyle(color: p.textSecondary))
                              else
                                ...report.peaks.asMap().entries.map((entry) {
                                  final rank = entry.key + 1;
                                  final pk = entry.value;
                                  final selected =
                                      _selectedPeakMinute == pk.minuteOffset;
                                  return Padding(
                                    padding: const EdgeInsets.only(bottom: 8),
                                    child: Material(
                                      color: selected
                                          ? p.accent.withValues(alpha: 0.12)
                                          : p.subtleBg.withValues(alpha: 0.4),
                                      shape: RoundedRectangleBorder(
                                        borderRadius: BorderRadius.circular(6),
                                        side: BorderSide(
                                          color: selected ? p.accent : p.border,
                                        ),
                                      ),
                                      clipBehavior: Clip.antiAlias,
                                      child: ListTile(
                                        dense: true,
                                        onTap: () => setState(() =>
                                            _selectedPeakMinute =
                                                pk.minuteOffset),
                                        leading: CircleAvatar(
                                          radius: 14,
                                          backgroundColor: p.accent,
                                          child: Text(
                                            '$rank',
                                            style: const TextStyle(
                                                color: Colors.white,
                                                fontSize: 12,
                                                fontWeight: FontWeight.bold),
                                          ),
                                        ),
                                        title: Text(
                                          '経過 ${pk.elapsedLabel}  —  ${pk.commentCount} コメント/分',
                                          style: const TextStyle(
                                              fontWeight: FontWeight.bold,
                                              fontSize: 13),
                                        ),
                                        subtitle: pk.sampleMessages.isEmpty
                                            ? null
                                            : Text(
                                                pk.sampleMessages.join('  /  '),
                                                maxLines: 2,
                                                overflow: TextOverflow.ellipsis,
                                                style: TextStyle(
                                                    fontSize: 11.5,
                                                    color: p.textSecondary),
                                              ),
                                      ),
                                    ),
                                  );
                                }),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 16),

                      // 頻出ワード & アクティブ視聴者
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Card(
                              child: Padding(
                                padding: const EdgeInsets.all(16),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    const Text(
                                      '頻出ワード TOP 15',
                                      style: TextStyle(
                                          fontWeight: FontWeight.bold,
                                          fontSize: 14),
                                    ),
                                    const SizedBox(height: 10),
                                    if (report.topWords.isEmpty)
                                      Text('データなし',
                                          style:
                                              TextStyle(color: p.textSecondary))
                                    else
                                      ...report.topWords.map((w) {
                                        final maxC = math.max(
                                            1,
                                            report.topWords.first.count
                                                .toInt());
                                        final ratio = w.count.toInt() / maxC;
                                        return Padding(
                                          padding:
                                              const EdgeInsets.only(bottom: 7),
                                          child: Row(
                                            children: [
                                              SizedBox(
                                                width: 110,
                                                child: Text(
                                                  w.word,
                                                  overflow:
                                                      TextOverflow.ellipsis,
                                                  style: const TextStyle(
                                                      fontSize: 12.5,
                                                      fontWeight:
                                                          FontWeight.w600),
                                                ),
                                              ),
                                              Expanded(
                                                child: LinearProgressIndicator(
                                                  value: ratio,
                                                  minHeight: 7,
                                                  borderRadius:
                                                      BorderRadius.circular(3),
                                                ),
                                              ),
                                              const SizedBox(width: 8),
                                              Text(
                                                '${w.count}',
                                                style: const TextStyle(
                                                    fontFamily: 'Consolas',
                                                    fontSize: 12,
                                                    fontWeight:
                                                        FontWeight.bold),
                                              ),
                                            ],
                                          ),
                                        );
                                      }),
                                  ],
                                ),
                              ),
                            ),
                          ),
                          const SizedBox(width: 14),
                          Expanded(
                            child: Card(
                              child: Padding(
                                padding: const EdgeInsets.all(16),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    const Text(
                                      'アクティブ視聴者 TOP 10',
                                      style: TextStyle(
                                          fontWeight: FontWeight.bold,
                                          fontSize: 14),
                                    ),
                                    const SizedBox(height: 10),
                                    if (report.topUsers.isEmpty)
                                      Text('データなし',
                                          style:
                                              TextStyle(color: p.textSecondary))
                                    else
                                      ...report.topUsers
                                          .asMap()
                                          .entries
                                          .map((e) {
                                        final u = e.value;
                                        return ListTile(
                                          dense: true,
                                          contentPadding: EdgeInsets.zero,
                                          leading: Text(
                                            '#${e.key + 1}',
                                            style: TextStyle(
                                              fontFamily: 'Consolas',
                                              fontWeight: FontWeight.bold,
                                              color: p.accent,
                                            ),
                                          ),
                                          title: Text(u.userName,
                                              overflow: TextOverflow.ellipsis,
                                              style: const TextStyle(
                                                  fontSize: 13,
                                                  fontWeight: FontWeight.w600)),
                                          trailing: Text(
                                            '${u.commentCount} 件',
                                            style: const TextStyle(
                                                fontFamily: 'Consolas',
                                                fontWeight: FontWeight.bold),
                                          ),
                                        );
                                      }),
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
        ),
      ],
    );
  }

  Widget _kpiCard(StosupPalette p, String label, String value, Color accent) {
    return Container(
      width: 175,
      padding: const EdgeInsets.all(13),
      decoration: BoxDecoration(
        color: p.cardBg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: p.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label,
              style: TextStyle(fontSize: 11.5, color: p.textSecondary)),
          const SizedBox(height: 4),
          Text(
            value,
            style: TextStyle(
              fontSize: 17,
              fontWeight: FontWeight.w900,
              color: accent,
            ),
          ),
        ],
      ),
    );
  }
}

class _MomentumChartPainter extends CustomPainter {
  _MomentumChartPainter({
    required this.buckets,
    required this.accentColor,
    required this.monetizationColor,
    required this.gridColor,
    required this.labelColor,
    required this.highlightMinute,
  });

  final List<ReportMinuteBucket> buckets;
  final Color accentColor;
  final Color monetizationColor;
  final Color gridColor;
  final Color labelColor;
  final int? highlightMinute;

  @override
  void paint(Canvas canvas, Size size) {
    const padL = 36.0;
    const padR = 14.0;
    const padT = 12.0;
    const padB = 24.0;
    final w = (size.width - padL - padR).clamp(1.0, double.infinity);
    final h = (size.height - padT - padB).clamp(1.0, double.infinity);

    final gridPaint = Paint()
      ..color = gridColor.withValues(alpha: 0.55)
      ..strokeWidth = 1;
    final maxVal = math.max(
      1,
      buckets.fold<int>(0, (m, b) => math.max(m, b.commentCount.toInt())),
    );

    // 横グリッド線 4 本
    for (int i = 0; i <= 3; i++) {
      final y = padT + h * (i / 3);
      canvas.drawLine(Offset(padL, y), Offset(padL + w, y), gridPaint);
      final v = (maxVal * (3 - i) / 3).round();
      final tp = TextPainter(
        text: TextSpan(
          text: '$v',
          style: TextStyle(color: labelColor, fontSize: 10),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      tp.paint(canvas, Offset(padL - tp.width - 6, y - tp.height / 2));
    }

    if (buckets.isEmpty) return;

    final pts = <Offset>[];
    final denom = math.max(1, buckets.length - 1);
    for (int i = 0; i < buckets.length; i++) {
      final x = padL + (buckets.length == 1 ? w / 2 : w * (i / denom));
      final ratio = buckets[i].commentCount.toInt() / maxVal;
      final y = padT + h * (1.0 - ratio);
      pts.add(Offset(x, y));
    }

    // エリア塗り
    final areaPath = Path()..moveTo(pts.first.dx, padT + h);
    for (final pt in pts) {
      areaPath.lineTo(pt.dx, pt.dy);
    }
    areaPath
      ..lineTo(pts.last.dx, padT + h)
      ..close();

    final areaPaint = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [
          accentColor.withValues(alpha: 0.34),
          accentColor.withValues(alpha: 0.02),
        ],
      ).createShader(Rect.fromLTWH(padL, padT, w, h));
    canvas.drawPath(areaPath, areaPaint);

    // 折れ線
    final linePath = Path()..moveTo(pts.first.dx, pts.first.dy);
    for (int i = 1; i < pts.length; i++) {
      linePath.lineTo(pts[i].dx, pts[i].dy);
    }
    final linePaint = Paint()
      ..color = accentColor
      ..strokeWidth = 2.2
      ..style = PaintingStyle.stroke;
    canvas.drawPath(linePath, linePaint);

    // 支援発生ポイント・選択ピークマーカー
    for (int i = 0; i < buckets.length; i++) {
      final b = buckets[i];
      if (b.monetizationCount > BigInt.zero) {
        canvas.drawCircle(pts[i], 4.2, Paint()..color = monetizationColor);
      }
      if (highlightMinute != null && b.minuteOffset == highlightMinute) {
        canvas.drawLine(
          Offset(pts[i].dx, padT),
          Offset(pts[i].dx, padT + h),
          Paint()
            ..color = accentColor
            ..strokeWidth = 1.5,
        );
        canvas.drawCircle(pts[i], 5.5, Paint()..color = accentColor);
      }
    }

    // X 軸ラベル（最大 6 点）
    final step = math.max(1, (buckets.length / 5).ceil());
    for (int i = 0; i < buckets.length; i += step) {
      final m = buckets[i].minuteOffset;
      final label = '${m}m';
      final tp = TextPainter(
        text: TextSpan(
          text: label,
          style: TextStyle(color: labelColor, fontSize: 10),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      tp.paint(canvas, Offset(pts[i].dx - tp.width / 2, padT + h + 6));
    }
  }

  @override
  bool shouldRepaint(covariant _MomentumChartPainter oldDelegate) {
    return !identical(buckets, oldDelegate.buckets) ||
        buckets.length != oldDelegate.buckets.length ||
        highlightMinute != oldDelegate.highlightMinute ||
        accentColor != oldDelegate.accentColor ||
        monetizationColor != oldDelegate.monetizationColor ||
        gridColor != oldDelegate.gridColor ||
        labelColor != oldDelegate.labelColor;
  }
}
