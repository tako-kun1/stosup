(function () {
  'use strict';

  if (!window.StosupMod) return;

  window.StosupMod.registerEffect('combo_burst', function (ctx) {
    const { ctx: c, width, height } = ctx.createCanvas();
    const duration = Math.max(1200, Number(ctx.trigger.duration_ms) || 3600);
    const combo = Number(ctx.params.combo) || 3;
    const label = String(ctx.params.label || 'COMBO FEVER!');
    const accent = String(ctx.params.accentColor || '#22C55E');
    const tier = String(ctx.params.tier || 'normal');

    if (ctx.trigger.caption) {
      ctx.showCaption();
    }

    const cx = width * 0.5;
    const cy = height * 0.48;
    const particleCount = tier === 'legend' ? 72 : tier === 'fever' ? 48 : 30;
    const palette =
      tier === 'legend'
        ? [accent, '#FDE047', '#FFFFFF', '#FB7185']
        : tier === 'fever'
        ? [accent, '#38BDF8', '#FFFFFF', '#F472B6']
        : [accent, '#86EFAC', '#FFFFFF'];

    const particles = [];
    for (let i = 0; i < particleCount; i++) {
      const angle = (Math.PI * 2 * i) / particleCount + (Math.random() - 0.5) * 0.25;
      const speed = 220 + Math.random() * (tier === 'legend' ? 520 : 360);
      particles.push({
        x: cx,
        y: cy,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed - 60,
        size: 5 + Math.random() * 8,
        rot: Math.random() * Math.PI * 2,
        vrot: (Math.random() - 0.5) * 8,
        color: palette[i % palette.length],
      });
    }

    let rafId = 0;
    let lastTime = performance.now();
    const startTime = lastTime;

    function easeOutBack(x) {
      const c1 = 1.70158;
      const c3 = c1 + 1;
      return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2);
    }

    function frame(now) {
      const dt = Math.min(0.05, (now - lastTime) / 1000);
      lastTime = now;
      const elapsed = now - startTime;
      const t = Math.min(1, elapsed / duration);

      c.clearRect(0, 0, width, height);

      // 1. 広がる衝撃波リング
      const ringProgress = Math.min(1, elapsed / 750);
      if (ringProgress < 1) {
        const radius = 30 + ringProgress * Math.min(width, height) * 0.42;
        c.save();
        c.beginPath();
        c.arc(cx, cy, radius, 0, Math.PI * 2);
        c.strokeStyle = accent;
        c.lineWidth = (1 - ringProgress) * 14;
        c.globalAlpha = (1 - ringProgress) * 0.85;
        c.stroke();
        c.restore();
      }

      // 2. パーティクル飛散
      const fadeOut = t > 0.72 ? Math.max(0, 1 - (t - 0.72) / 0.28) : 1;
      for (const p of particles) {
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.vy += 420 * dt;
        p.vx *= 0.985;
        p.rot += p.vrot * dt;

        c.save();
        c.translate(p.x, p.y);
        c.rotate(p.rot);
        c.globalAlpha = fadeOut * 0.92;
        c.fillStyle = p.color;
        c.fillRect(-p.size * 0.5, -p.size * 0.5, p.size, p.size * 0.65);
        c.restore();
      }

      // 3. 中央のコンボバッジ
      const popT = Math.min(1, elapsed / 320);
      const scale = easeOutBack(popT) * (t > 0.82 ? Math.max(0, 1 - (t - 0.82) / 0.18) : 1);
      if (scale > 0.01) {
        c.save();
        c.translate(cx, cy);
        c.scale(scale, scale);
        c.globalAlpha = fadeOut;
        c.textAlign = 'center';
        c.textBaseline = 'middle';

        // 発光する数値
        const mainText = combo >= 100 ? String(combo) : `${combo} COMBO`;
        c.font = '900 76px "Segoe UI", "Yu Gothic UI", sans-serif';
        c.lineWidth = 12;
        c.strokeStyle = 'rgba(15, 23, 42, 0.9)';
        c.shadowColor = accent;
        c.shadowBlur = 24;
        c.strokeText(mainText, 0, -14);
        c.fillStyle = '#FFFFFF';
        c.fillText(mainText, 0, -14);

        // ラベル
        c.font = '800 28px "Segoe UI", "Yu Gothic UI", sans-serif';
        c.lineWidth = 7;
        c.shadowBlur = 12;
        c.strokeText(label, 0, 44);
        c.fillStyle = accent;
        c.fillText(label, 0, 44);
        c.restore();
      }

      if (t < 1) {
        rafId = requestAnimationFrame(frame);
      } else {
        ctx.done();
      }
    }

    ctx.onCleanup(function () {
      if (rafId) cancelAnimationFrame(rafId);
    });

    rafId = requestAnimationFrame(frame);
  });
})();
