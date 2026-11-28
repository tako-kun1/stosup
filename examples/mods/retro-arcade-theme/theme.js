/* Stosup オーバーレイ追加テーマ用 任意 JS フックサンプル */
(function () {
  'use strict';

  const TAG_ID = 'retro-arcade-hud-tag';

  window.StosupMod.registerThemeHook('retro_arcade', {
    mount(info) {
      if (!document.body || document.getElementById(TAG_ID)) return;
      const el = document.createElement('div');
      el.id = TAG_ID;
      el.textContent = `ARCADE // ${String(info.overlayKey || 'HUD').toUpperCase()}`;
      document.body.appendChild(el);
    },
    update(info) {
      const el = document.getElementById(TAG_ID);
      if (el) {
        el.textContent = `ARCADE // ${String(info.overlayKey || 'HUD').toUpperCase()}`;
      }
    },
    unmount() {
      const el = document.getElementById(TAG_ID);
      if (el && el.parentNode) {
        el.parentNode.removeChild(el);
      }
    },
  });
})();
