import 'package:material_ui/material_ui.dart';

import '../src/rust/models.dart';

/// Rust 側で生成した 2D モジュール行列を鮮明に描画する QR コードウィジェット
class QrCodeView extends StatelessWidget {
  const QrCodeView({
    super.key,
    required this.matrix,
    this.size = 156,
  });

  final QrCodeMatrix matrix;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFFD4D4D8)),
      ),
      child: CustomPaint(
        painter: _QrCodePainter(matrix),
        size: Size.square(size - 20),
      ),
    );
  }
}

class _QrCodePainter extends CustomPainter {
  const _QrCodePainter(this.matrix);

  final QrCodeMatrix matrix;

  @override
  void paint(Canvas canvas, Size size) {
    final dim = matrix.size;
    if (dim == 0 || matrix.modules.length < dim * dim) return;

    final cellW = size.width / dim;
    final cellH = size.height / dim;
    final paint = Paint()
      ..color = const Color(0xFF09090B)
      ..style = PaintingStyle.fill
      ..isAntiAlias = false;

    for (var y = 0; y < dim; y++) {
      for (var x = 0; x < dim; x++) {
        if (matrix.modules[y * dim + x]) {
          canvas.drawRect(
            Rect.fromLTWH(
              x * cellW,
              y * cellH,
              cellW + 0.3,
              cellH + 0.3,
            ),
            paint,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant _QrCodePainter oldDelegate) =>
      oldDelegate.matrix != matrix;
}
