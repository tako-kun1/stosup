/* Stosup リアクション演出 MOD サンプル: スターライト・バースト */
(function () {
  'use strict';

  function hexToRgb(hex) {
    const clean = String(hex || '#38BDF8').replace('#', '');
    if (clean.length < 6) return { r: 56, g: 189, b: 248 };
    return {
      r: parseInt(clean.slice(0, 2), 16) || 56,
      g: parseInt(clean.slice(2, 4), 16) || 189,
      b: parseInt(clean.slice(4, 6), 16) || 248,
    };
  }

  window.StosupMod.registerEffect('starlight_burst', function (ctx) {
    const { ctx: c, width, height } = ctx.createCanvas();
    const duration = Math.max(1200, Number(ctx.trigger.duration_ms) || 4200);
    const starCount = Math.max(8, Math.min(80, Number(ctx.params.starCount) || 28));
    const colorHex = String(ctx.params.primaryColor || '#38BDF8');
    const rgb = hexToRgb(colorHex);
    const mode = String(ctx.params.burstMode || 'meteor');
    const showFlash = Boolean(ctx.params.showFlash);
    const subText = String(ctx.params.subText || '').trim();

    if (ctx.trigger.caption) {
      ctx.showCaption();
    }

    // 任意サブメッセージバナー
    let bannerEl = null;
    let bannerTimer = null;
    if (subText) {
      bannerEl = document.createElement('div');
      bannerEl.className = 'starlight-banner';
      bannerEl.style.setProperty('--sl-color', colorHex);
      bannerEl.textContent = subText;
      ctx.stage.appendChild(bannerEl);
      bannerTimer = setTimeout(function () {
        if (bannerEl) bannerEl.classList.add('leaving');
      }, Math.max(600, duration - 420));
    }

    // 粒子初期化
    const particles = [];
    const cx = width * 0.5;
    const cy = height * 0.48;
    for (let i = 0; i < starCount; i++) {
      if (mode === 'firework') {
        const ang = (Math.PI * 2 * i) / starCount + (Math.random() - 0.5) * 0.25;
        const speed = 180 + Math.random() * 440;
        particles.push({
          x: cx,
          y: cy,
          vx: Math.cos(ang) * speed,
          vy: Math.sin(ang) * speed - 80,
          size: 3 + Math.random() * 5,
          delay: Math.random() * 0.14,
          hueShift: (Math.random() - 0.5) * 30,
        });
      } else if (mode === 'aurora') {
        particles.push({
          x: Math.random() * width,
          y: height + 30 + Math.random() * 140,
          vx: (Math.random() - 0.5) * 45,
          vy: -(160 + Math.random() * 240),
          size: 4 + Math.random() * 7,
          delay: Math.random() * 0.28,
          phase: Math.random() * Math.PI * 2,
        });
      } else {
        // meteor: 右上から左下へ駆け抜ける流星群
        particles.push({
          x: width * (0.2 + Math.random() * 1.1),
          y: -40 - Math.random() * height * 0.4,
          vx: -(420 + Math.random() * 380),
          vy: 260 + Math.random() * 240,
          size: 2.5 + Math.random() * 4,
          tail: 45 + Math.random() * 95,
          delay: Math.random() * 0.45,
        });
      }
    }

    let rafId = 0;
    const startMs = performance.now();
    let lastMs = startMs;

    function drawStar(x, y, r, alpha) {
      c.save();
      c.translate(x, y);
      c.fillStyle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha * 0.32})`;
      c.beginPath();
      c.arc(0, 0, r * 2.6, 0, Math.PI * 2);
      c.fill();

      c.fillStyle = `rgba(255, 255, 255, ${alpha})`;
      c.beginPath();
      for (let i = 0; i < 4; i++) {
        const a = (i * Math.PI) / 2;
        c.lineTo(Math.cos(a) * r * 1.7, Math.sin(a) * r * 1.7);
        c.lineTo(Math.cos(a + Math.PI / 4) * r * 0.45, Math.sin(a + Math.PI / 4) * r * 0.45);
      }
      c.closePath();
      c.fill();
      c.restore();
    }

    function tick(now) {
      const elapsed = now - startMs;
      const dt = Math.min(0.05, (now - lastMs) / 1000);
      lastMs = now;
      const progress = Math.min(1, elapsed / duration);

      c.clearRect(0, 0, width, height);

      // 瞬間フラッシュ
      if (showFlash && progress < 0.16) {
        const flashAlpha = (1 - progress / 0.16) * 0.24;
        c.fillStyle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${flashAlpha})`;
        c.fillRect(0, 0, width, height);
      }

      // 広がる光のリング
      if (progress < 0.65) {
        const ringT = progress / 0.65;
        const radius = 30 + ringT * Math.min(width, height) * 0.48;
        const ringAlpha = (1 - ringT) * 0.65;
        c.strokeStyle = `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${ringAlpha})`;
        c.lineWidth = Math.max(1.5, 6 * (1 - ringT));
        c.beginPath();
        c.arc(cx, cy, radius, 0, Math.PI * 2);
        c.stroke();
      }

      // 各粒子の更新＆描画
      for (const p of particles) {
        if (progress < p.delay) continue;
        const localT = (progress - p.delay) / Math.max(0.01, 1 - p.delay);
        const fade = localT > 0.72 ? Math.max(0, 1 - (localT - 0.72) / 0.28) : 1;

        if (mode === 'firework') {
          p.vy += 320 * dt;
          p.vx *= 0.985;
          p.x += p.vx * dt;
          p.y += p.vy * dt;
          drawStar(p.x, p.y, p.size, fade);
        } else if (mode === 'aurora') {
          p.x += (p.vx + Math.sin(elapsed * 0.004 + p.phase) * 55) * dt;
          p.y += p.vy * dt;
          drawStar(p.x, p.y, p.size, fade);
        } else {
          p.x += p.vx * dt;
          p.y += p.vy * dt;
          const speed = Math.hypot(p.vx, p.vy) || 1;
          const tx = p.x - (p.vx / speed) * p.tail;
          const ty = p.y - (p.vy / speed) * p.tail;
          const grad = c.createLinearGradient(p.x, p.y, tx, ty);
          grad.addColorStop(0, `rgba(255, 255, 255, ${fade})`);
          grad.addColorStop(0.35, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${fade * 0.85})`);
          grad.addColorStop(1, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0)`);
          c.strokeStyle = grad;
          c.lineWidth = p.size;
          c.lineCap = 'round';
          c.beginPath();
          c.moveTo(p.x, p.y);
          c.lineTo(tx, ty);
          c.stroke();
          drawStar(p.x, p.y, p.size * 0.9, fade);
        }
      }

      if (progress < 1) {
        rafId = requestAnimationFrame(tick);
      } else {
        ctx.done();
      }
    }

    ctx.onCleanup(function () {
      if (rafId) cancelAnimationFrame(rafId);
      if (bannerTimer) clearTimeout(bannerTimer);
    });

    rafId = requestAnimationFrame(tick);
  });
})();
