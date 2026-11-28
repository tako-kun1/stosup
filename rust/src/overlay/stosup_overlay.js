/* Stosup Overlay Common Engine (Visual Style + Safe Emoji Runs + Theme Normalization) */
(function (global) {
  'use strict';

  const THEME_ALIAS_MAP = {
    default: 'broadcast',
    broadcast: 'broadcast',
    editorial: 'editorial',
    pastel: 'editorial',
    cute: 'editorial',
    tactical: 'tactical',
    neon: 'tactical',
    scoreboard: 'tactical',
    variety: 'variety',
    pop: 'variety',
    festive: 'variety',
    cinema: 'cinema',
    minimal: 'cinema',
    compact: 'cinema',
    japanese: 'cinema',
    edge_glow: 'edge_glow',
    luminous: 'edge_glow',
    glow: 'edge_glow',
  };

  function normalizeTheme(raw) {
    const rawTrimmed = String(raw || 'broadcast').trim();
    if (rawTrimmed.toLowerCase().startsWith('mod:')) {
      return rawTrimmed;
    }
    const k = rawTrimmed.toLowerCase();
    return THEME_ALIAS_MAP[k] || 'broadcast';
  }

  // ==================== MOD 追加テーマ ランタイム管理 ====================
  let _modsList = [];
  let _modsRevision = 0;
  let _modsFetching = false;
  let _activeModThemeKey = '';
  let _activeModThemeRev = -1;
  let _activeModThemeHook = null;
  let _activeModThemeOverlayKey = '';
  const _loadedThemeScripts = new Map();

  function _findModTheme(fullThemeId) {
    if (!fullThemeId || !fullThemeId.startsWith('mod:')) return null;
    for (const mod of _modsList) {
      if (!mod || !mod.enabled || mod.load_error) continue;
      const themes = Array.isArray(mod.overlay_themes) ? mod.overlay_themes : [];
      for (const th of themes) {
        if (th && th.full_theme_id === fullThemeId) {
          return { mod, theme: th };
        }
      }
    }
    return null;
  }

  function _unmountCurrentModTheme() {
    if (_activeModThemeHook && typeof _activeModThemeHook.unmount === 'function') {
      try {
        _activeModThemeHook.unmount({ overlayKey: _activeModThemeOverlayKey });
      } catch (e) {
        console.error('[StosupOverlay] MOD theme unmount error:', e);
      }
    }
    _activeModThemeHook = null;
    _activeModThemeKey = '';
    _activeModThemeRev = -1;
    const link = document.getElementById('stosup-mod-theme-css');
    if (link && link.parentNode) {
      link.parentNode.removeChild(link);
    }
    if (document.body) {
      document.body.removeAttribute('data-mod-id');
      document.body.removeAttribute('data-mod-theme');
    }
  }

  function _loadScriptOnce(url, modId, componentId) {
    if (_loadedThemeScripts.has(url)) {
      return _loadedThemeScripts.get(url);
    }
    const promise = new Promise((resolve) => {
      const s = document.createElement('script');
      s.src = url;
      s.async = false;
      if (global.StosupMod && typeof global.StosupMod._setLoadingContext === 'function') {
        global.StosupMod._setLoadingContext(modId, componentId);
      }
      s.onload = () => {
        if (global.StosupMod && typeof global.StosupMod._clearLoadingContext === 'function') {
          global.StosupMod._clearLoadingContext();
        }
        resolve(true);
      };
      s.onerror = () => {
        if (global.StosupMod && typeof global.StosupMod._clearLoadingContext === 'function') {
          global.StosupMod._clearLoadingContext();
        }
        console.error('[StosupOverlay] MOD theme script load failed:', url);
        resolve(false);
      };
      document.head.appendChild(s);
    });
    _loadedThemeScripts.set(url, promise);
    return promise;
  }

  function _ensureModSdkLoaded() {
    if (global.StosupMod) return Promise.resolve(true);
    return _loadScriptOnce('/overlay/stosup-mod-sdk.js', '', '');
  }

  function syncMods(mods, revision) {
    if (Array.isArray(mods)) {
      _modsList = mods;
    }
    if (typeof revision === 'number') {
      _modsRevision = revision;
    }
  }

  function _fetchRuntimeModsAndReapply(overlayKey, themeId) {
    if (_modsFetching) return;
    _modsFetching = true;
    fetch('/api/mods/runtime', { cache: 'no-store' })
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        _modsFetching = false;
        if (data && Array.isArray(data.mods)) {
          syncMods(data.mods, data.revision || _modsRevision);
          applyModTheme(overlayKey, themeId);
        }
      })
      .catch(() => {
        _modsFetching = false;
      });
  }

  function applyModTheme(overlayKey, themeId) {
    const id = String(themeId || '').trim();
    if (!id.startsWith('mod:')) {
      if (_activeModThemeKey) {
        _unmountCurrentModTheme();
      }
      return;
    }

    const found = _findModTheme(id);
    if (!found) {
      if (_modsList.length === 0) {
        _fetchRuntimeModsAndReapply(overlayKey, id);
      } else if (_activeModThemeKey) {
        _unmountCurrentModTheme();
      }
      return;
    }

    const { mod, theme } = found;
    if (document.body) {
      document.body.dataset.modId = mod.id;
      document.body.dataset.modTheme = theme.theme_id;
    }

    if (_activeModThemeKey === id && _activeModThemeRev === _modsRevision) {
      if (_activeModThemeHook && typeof _activeModThemeHook.update === 'function') {
        try {
          _activeModThemeHook.update({
            overlayKey,
            modId: mod.id,
            themeId: theme.theme_id,
            baseUrl: `/mods/${encodeURIComponent(mod.id)}/`,
          });
        } catch (e) {
          console.error('[StosupOverlay] MOD theme update error:', e);
        }
      }
      return;
    }

    _unmountCurrentModTheme();
    _activeModThemeKey = id;
    _activeModThemeRev = _modsRevision;
    _activeModThemeOverlayKey = overlayKey;

    if (document.body) {
      document.body.dataset.modId = mod.id;
      document.body.dataset.modTheme = theme.theme_id;
    }

    // CSS を <style id="stosup-visual-css"> の直前に差し込むことで、
    // MOD テーマの上から Stosup の GUI ビジュアルスタイル＆カスタム CSS で微調整できる
    const cssHref = `/mods/${encodeURIComponent(mod.id)}/${theme.css}?v=${_modsRevision}`;
    const link = document.createElement('link');
    link.id = 'stosup-mod-theme-css';
    link.rel = 'stylesheet';
    link.href = cssHref;
    const visualEl = ensureVisualStyleTag();
    if (visualEl && visualEl.parentNode) {
      visualEl.parentNode.insertBefore(link, visualEl);
    } else {
      document.head.appendChild(link);
    }

    if (theme.script) {
      const scriptUrl = `/mods/${encodeURIComponent(mod.id)}/${theme.script}?v=${_modsRevision}`;
      _ensureModSdkLoaded().then(() => {
        if (_activeModThemeKey !== id) return;
        return _loadScriptOnce(scriptUrl, mod.id, theme.theme_id).then(() => {
          if (_activeModThemeKey !== id || !global.StosupMod) return;
          const hook = global.StosupMod._getThemeHook(mod.id, theme.theme_id);
          if (hook) {
            _activeModThemeHook = hook;
            if (typeof hook.mount === 'function') {
              try {
                hook.mount({
                  overlayKey,
                  modId: mod.id,
                  themeId: theme.theme_id,
                  baseUrl: `/mods/${encodeURIComponent(mod.id)}/`,
                });
              } catch (e) {
                console.error('[StosupOverlay] MOD theme mount error:', e);
              }
            }
          }
        });
      });
    }
  }

  function resolveBg(raw) {
    const v = String(raw || 'transparent').trim();
    if (!v || v.toLowerCase() === 'transparent') return 'transparent';
    if (/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v)) return '#' + v;
    if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v)) return v;
    return 'transparent';
  }

  function safeHexColor(raw) {
    const v = String(raw || '').trim();
    if (/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v)) return v;
    if (/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(v)) return '#' + v;
    return '';
  }

  function hexToRgba(hex, opacityPercent) {
    const h = safeHexColor(hex);
    if (!h) return '';
    const r = parseInt(h.slice(1, 3), 16);
    const g = parseInt(h.slice(3, 5), 16);
    const b = parseInt(h.slice(5, 7), 16);
    const a = Math.max(0, Math.min(100, Number(opacityPercent ?? 92))) / 100;
    return `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
  }

  let _lastAppliedFont = null;
  function applyFont(raw) {
    const f = String(raw || '').trim();
    if (f === _lastAppliedFont) return;
    _lastAppliedFont = f;
    if (f) {
      const safe = f.replace(/["\\;{}<>]/g, '');
      document.documentElement.style.setProperty(
        '--font-family',
        `"${safe}", "BIZ UDPGothic", "Yu Gothic UI", "Meiryo", sans-serif`
      );
      document.body.classList.add('has-custom-font');
    } else {
      document.documentElement.style.removeProperty('--font-family');
      document.body.classList.remove('has-custom-font');
    }
  }

  function buildOuterRingShadow(widthPx, color) {
    const w = Math.max(1, Math.min(10, Number(widthPx) || 0));
    const c = safeHexColor(color) || '#ffffff';
    const pts = [
      [w, 0], [-w, 0], [0, w], [0, -w],
      [w, w], [-w, w], [w, -w], [-w, -w],
      [Math.round(w * 0.7), w], [-Math.round(w * 0.7), w],
      [Math.round(w * 0.7), -w], [-Math.round(w * 0.7), -w],
    ];
    return pts.map(([x, y]) => `${x}px ${y}px 0 ${c}`).join(', ');
  }

  /**
   * テキストのエッジ発光（Edge Glow）用の多層 text-shadow リストを構築する
   * 輪郭コア層 + 中距離グロー層 + 広域ハロー層（高強度時は追加ブースター層）
   */
  function buildEdgeGlowShadows(vs) {
    if (!vs || !vs.text_glow_enabled) return [];
    const color = safeHexColor(vs.text_glow_color) || '#38bdf8';
    const radius = Math.max(1, Math.min(40, Number(vs.text_glow_radius ?? 12)));
    const intensity = Math.max(10, Math.min(200, Number(vs.text_glow_intensity ?? 85)));
    const spread = Math.max(0, Math.min(10, Number(vs.text_glow_spread ?? 2)));
    const k = intensity / 100;

    const coreAlpha = Math.min(100, Math.round(78 * k));
    const midAlpha = Math.min(95, Math.round(56 * k));
    const outerAlpha = Math.min(85, Math.round(36 * k));

    const coreCol = hexToRgba(color, coreAlpha);
    const midCol = hexToRgba(color, midAlpha);
    const outerCol = hexToRgba(color, outerAlpha);

    const out = [];
    // 1. テキスト輪郭すぐ外周のエッジコア発光
    if (spread > 0) {
      const coreBlur = Math.max(1, Math.round(spread * 1.4));
      out.push(`${spread}px 0 ${coreBlur}px ${coreCol}`);
      out.push(`-${spread}px 0 ${coreBlur}px ${coreCol}`);
      out.push(`0 ${spread}px ${coreBlur}px ${coreCol}`);
      out.push(`0 -${spread}px ${coreBlur}px ${coreCol}`);
    }
    const tightR = Math.max(2, Math.round(radius * 0.25));
    out.push(`0 0 ${tightR}px ${coreCol}`);

    // 2. 中距離エッジグロー
    const midR = Math.max(4, Math.round(radius * 0.65));
    out.push(`0 0 ${midR}px ${midCol}`);

    // 3. 広域光輪（ハロー）
    out.push(`0 0 ${radius}px ${outerCol}`);

    // 4. 高強度 (100% 超) 時のネオン／オーラ増幅レイヤー
    if (intensity > 100) {
      const boostAlpha = Math.min(85, Math.round((k - 0.85) * 65));
      const boostCol = hexToRgba(color, boostAlpha);
      out.push(`0 0 ${Math.max(3, Math.round(radius * 0.45))}px ${coreCol}`);
      out.push(`0 0 ${Math.round(radius * 1.55)}px ${boostCol}`);
    }
    return out;
  }

  /**
   * 2色グラデーション文字時のエッジ発光用 filter: drop-shadow(...) リストを構築する
   */
  function buildEdgeGlowFilters(vs) {
    if (!vs || !vs.text_glow_enabled) return [];
    const color = safeHexColor(vs.text_glow_color) || '#38bdf8';
    const radius = Math.max(1, Math.min(40, Number(vs.text_glow_radius ?? 12)));
    const intensity = Math.max(10, Math.min(200, Number(vs.text_glow_intensity ?? 85)));
    const spread = Math.max(0, Math.min(10, Number(vs.text_glow_spread ?? 2)));
    const k = intensity / 100;

    const coreCol = hexToRgba(color, Math.min(100, Math.round(82 * k)));
    const midCol = hexToRgba(color, Math.min(95, Math.round(58 * k)));
    const outerCol = hexToRgba(color, Math.min(85, Math.round(38 * k)));

    const filters = [];
    const coreBlur = Math.max(1, spread > 0 ? Math.round(spread * 1.5) : 2);
    filters.push(`drop-shadow(0 0 ${coreBlur}px ${coreCol})`);
    filters.push(`drop-shadow(0 0 ${Math.max(3, Math.round(radius * 0.55))}px ${midCol})`);
    filters.push(`drop-shadow(0 0 ${radius}px ${outerCol})`);
    if (intensity > 110) {
      const boostCol = hexToRgba(color, Math.min(82, Math.round((k - 0.85) * 62)));
      filters.push(`drop-shadow(0 0 ${Math.round(radius * 1.45)}px ${boostCol})`);
    }
    return filters;
  }

  function ensureVisualStyleTag() {
    let el = document.getElementById('stosup-visual-css');
    if (!el) {
      el = document.createElement('style');
      el.id = 'stosup-visual-css';
      const customEl = document.getElementById('stosup-custom-css');
      if (customEl && customEl.parentNode) {
        customEl.parentNode.insertBefore(el, customEl);
      } else {
        document.head.appendChild(el);
      }
    }
    return el;
  }

  /**
   * オーバーレイ種別ごとの GUI ビジュアルスタイルを動的 CSS へ変換適用する
   * @param {string} overlayKey
   * @param {object} config - OverlayConfig
   * @param {object} selectors - { card, text, author, accent, animate }
   */
  function applyOverlayVisualStyle(overlayKey, config, selectors) {
    // 現在適用中のテーマが mod:* の場合は MOD 追加テーマ CSS / Hook を適用（通常テーマ時は自動解除）
    applyModTheme(overlayKey, document.body ? document.body.dataset.theme : '');
    const styleEl = ensureVisualStyleTag();
    const stylesMap = (config && config.visual_styles) || {};
    const vs = stylesMap[overlayKey];
    if (!vs || !vs.enabled) {
      if (styleEl.textContent !== '') {
        styleEl.textContent = '';
      }
      if (document.body.hasAttribute('data-anim')) {
        document.body.removeAttribute('data-anim');
      }
      return;
    }

    const cardSel = (selectors && selectors.card) || '.st-card';
    const textSel = (selectors && selectors.text) || '.st-text';
    const authorSel = (selectors && selectors.author) || '.st-author';
    const animSel = (selectors && selectors.animate) || cardSel;

    const rules = [];

    // 1. テキスト・タイポグラフィ装飾
    const textDecls = [];
    if (vs.font_size && vs.font_size > 0) {
      textDecls.push(`font-size: ${Math.max(10, Math.min(96, vs.font_size))}px !important;`);
    }
    if (vs.font_weight && vs.font_weight >= 300) {
      textDecls.push(`font-weight: ${Math.min(900, vs.font_weight)} !important;`);
    }
    if (typeof vs.letter_spacing_tenths === 'number' && vs.letter_spacing_tenths !== 0) {
      textDecls.push(`letter-spacing: ${(vs.letter_spacing_tenths / 10).toFixed(1)}px !important;`);
    }
    if (vs.line_height_percent && vs.line_height_percent >= 100) {
      textDecls.push(`line-height: ${(vs.line_height_percent / 100).toFixed(2)} !important;`);
    }

    const tc = safeHexColor(vs.text_color);
    const strokeW = Math.max(0, Math.min(10, Number(vs.stroke_width || 0)));
    const strokeC = safeHexColor(vs.stroke_color) || '#111114';
    const outerW = Math.max(0, Math.min(10, Number(vs.outer_stroke_width || 0)));
    const outerC = safeHexColor(vs.outer_stroke_color) || '#ffffff';

    const shadows = [];
    if (outerW > 0) {
      shadows.push(buildOuterRingShadow(strokeW + outerW, outerC));
    }
    if (vs.text_glow_enabled) {
      shadows.push(...buildEdgeGlowShadows(vs));
    }
    if (vs.text_shadow_enabled) {
      const sc = safeHexColor(vs.text_shadow_color) || '#000000';
      const sx = Number(vs.text_shadow_offset_x || 0);
      const sy = Number(vs.text_shadow_offset_y || 0);
      const sb = Math.max(0, Number(vs.text_shadow_blur || 0));
      shadows.push(`${sx}px ${sy}px ${sb}px ${sc}`);
    }

    if (vs.use_text_gradient) {
      const g1 = safeHexColor(vs.text_gradient_start) || '#ffffff';
      const g2 = safeHexColor(vs.text_gradient_end) || '#fde047';
      const ang = Math.max(0, Math.min(360, Number(vs.text_gradient_angle ?? 180)));
      textDecls.push(`background-image: linear-gradient(${ang}deg, ${g1}, ${g2}) !important;`);
      textDecls.push(`-webkit-background-clip: text !important;`);
      textDecls.push(`background-clip: text !important;`);
      textDecls.push(`-webkit-text-fill-color: transparent !important;`);
      // グラデーション文字と袋文字・エッジ発光・影を美しく両立するため drop-shadow フィルタを使用
      const filters = [];
      if (strokeW > 0) {
        const sw = Math.max(1, Math.min(6, strokeW));
        filters.push(`drop-shadow(${sw}px 0 0 ${strokeC})`);
        filters.push(`drop-shadow(-${sw}px 0 0 ${strokeC})`);
        filters.push(`drop-shadow(0 ${sw}px 0 ${strokeC})`);
        filters.push(`drop-shadow(0 -${sw}px 0 ${strokeC})`);
      }
      if (outerW > 0) {
        const ow = Math.max(1, Math.min(6, outerW));
        filters.push(`drop-shadow(${ow}px 0 0 ${outerC})`);
        filters.push(`drop-shadow(-${ow}px 0 0 ${outerC})`);
        filters.push(`drop-shadow(0 ${ow}px 0 ${outerC})`);
        filters.push(`drop-shadow(0 -${ow}px 0 ${outerC})`);
      }
      if (vs.text_glow_enabled) {
        filters.push(...buildEdgeGlowFilters(vs));
      }
      if (vs.text_shadow_enabled) {
        const sc = safeHexColor(vs.text_shadow_color) || '#000000';
        const sx = Number(vs.text_shadow_offset_x || 0);
        const sy = Number(vs.text_shadow_offset_y || 0);
        const sb = Math.max(0, Number(vs.text_shadow_blur || 0));
        filters.push(`drop-shadow(${sx}px ${sy}px ${sb}px ${sc})`);
      }
      if (filters.length) {
        textDecls.push(`filter: ${filters.join(' ')} !important;`);
      }
    } else {
      if (tc) {
        textDecls.push(`color: ${tc} !important;`);
        textDecls.push(`-webkit-text-fill-color: ${tc} !important;`);
      }
      if (strokeW > 0) {
        textDecls.push(`paint-order: stroke fill !important;`);
        textDecls.push(`-webkit-text-stroke: ${strokeW * 2}px ${strokeC} !important;`);
      }
      if (shadows.length) {
        textDecls.push(`text-shadow: ${shadows.join(', ')} !important;`);
      }
    }

    if (textDecls.length) {
      rules.push(`${textSel} { ${textDecls.join(' ')} }`);
    }

    // 2. 投稿者名・アクセント色
    const ac = safeHexColor(vs.author_color);
    if (ac) {
      rules.push(`${authorSel} { color: ${ac} !important; }`);
    }
    const accent = safeHexColor(vs.accent_color);
    if (accent) {
      rules.push(`:root { --accent: ${accent}; --opt-color: ${accent}; }`);
    }

    // 3. カード・ボックス装飾
    const cardDecls = [];
    const bgHex = safeHexColor(vs.card_bg_color);
    if (bgHex) {
      cardDecls.push(`background: ${hexToRgba(bgHex, vs.card_bg_opacity)} !important;`);
    } else if (typeof vs.card_bg_opacity === 'number' && vs.card_bg_opacity === 0) {
      cardDecls.push(`background: transparent !important; box-shadow: none !important; border-color: transparent !important;`);
    }
    if (typeof vs.border_radius === 'number' && vs.border_radius >= 0) {
      cardDecls.push(`border-radius: ${Math.min(36, vs.border_radius)}px !important;`);
    }
    if (typeof vs.border_width === 'number' && vs.border_width >= 0) {
      const bc = safeHexColor(vs.border_color) || accent || 'currentColor';
      if (vs.border_width === 0) {
        cardDecls.push(`border: none !important;`);
      } else {
        cardDecls.push(`border: ${Math.min(8, vs.border_width)}px solid ${bc} !important;`);
      }
    } else if (safeHexColor(vs.border_color)) {
      cardDecls.push(`border-color: ${safeHexColor(vs.border_color)} !important;`);
    }

    switch (vs.card_shadow) {
      case 'none':
        cardDecls.push(`box-shadow: none !important;`);
        break;
      case 'soft':
        cardDecls.push(`box-shadow: 0 4px 14px rgba(0, 0, 0, 0.22) !important;`);
        break;
      case 'medium':
        cardDecls.push(`box-shadow: 0 10px 28px rgba(0, 0, 0, 0.42) !important;`);
        break;
      case 'hard':
        cardDecls.push(`box-shadow: 5px 5px 0 rgba(15, 18, 28, 0.92) !important;`);
        break;
      default:
        break;
    }

    switch (vs.padding_scale) {
      case 'compact':
        cardDecls.push(`padding: 7px 12px !important;`);
        break;
      case 'normal':
        cardDecls.push(`padding: 12px 18px !important;`);
        break;
      case 'relaxed':
        cardDecls.push(`padding: 18px 24px !important;`);
        break;
      default:
        break;
    }

    if (cardDecls.length) {
      rules.push(`${cardSel} { ${cardDecls.join(' ')} }`);
    }

    // 4. 出現アニメーション上書き
    const anim = String(vs.entrance_animation || 'inherit');
    if (anim !== 'inherit') {
      document.body.dataset.anim = anim;
      if (anim === 'none') {
        rules.push(`${animSel} { animation: none !important; }`);
      } else if (anim === 'wipe') {
        rules.push(`
          @keyframes stosupWipeIn {
            from { opacity: 0; clip-path: inset(0 100% 0 0); transform: translateX(-10px); }
            to   { opacity: 1; clip-path: inset(0 0 0 0); transform: translateX(0); }
          }
          ${animSel} { animation: stosupWipeIn 0.34s cubic-bezier(0.16, 1, 0.3, 1) !important; }
        `);
      } else if (anim === 'slide') {
        rules.push(`
          @keyframes stosupSlideIn {
            from { opacity: 0; transform: translateY(14px); }
            to   { opacity: 1; transform: translateY(0); }
          }
          ${animSel} { animation: stosupSlideIn 0.28s cubic-bezier(0.16, 1, 0.3, 1) !important; }
        `);
      } else if (anim === 'fade') {
        rules.push(`
          @keyframes stosupFadeIn {
            from { opacity: 0; }
            to   { opacity: 1; }
          }
          ${animSel} { animation: stosupFadeIn 0.28s ease-out !important; }
        `);
      } else if (anim === 'pop') {
        rules.push(`
          @keyframes stosupPopIn {
            from { opacity: 0; transform: scale(0.88); }
            to   { opacity: 1; transform: scale(1); }
          }
          ${animSel} { animation: stosupPopIn 0.32s cubic-bezier(0.34, 1.56, 0.64, 1) !important; }
        `);
      }
    } else {
      document.body.removeAttribute('data-anim');
    }

    const nextCss = rules.join('\n');
    if (styleEl.textContent !== nextCss) {
      styleEl.textContent = nextCss;
    }
  }

  /**
   * 絵文字・メンバースタンプ混在の message_runs を XSS 安全に DOM 描画する
   */
  function renderMessageRuns(container, fallbackText, runs) {
    container.textContent = '';
    const hasEmoji =
      Array.isArray(runs) &&
      runs.some((r) => r && r.kind === 'Emoji' && r.emoji_url);

    if (!hasEmoji) {
      container.textContent = fallbackText || '';
      return;
    }

    for (const run of runs) {
      if (!run) continue;
      if (run.kind === 'Emoji' && run.emoji_url) {
        const url = String(run.emoji_url);
        const safeUrl =
          url.startsWith('https://') ||
          url.startsWith('http://') ||
          url.startsWith('data:image/svg+xml');
        if (safeUrl) {
          const img = document.createElement('img');
          img.className = 'stosup-emoji';
          img.src = url;
          img.alt = run.alt || run.text || '';
          img.title = run.alt || run.text || '';
          img.style.height = '1.35em';
          img.style.width = 'auto';
          img.style.verticalAlign = '-0.24em';
          img.style.margin = '0 2px';
          img.style.display = 'inline-block';
          img.onerror = () => {
            const span = document.createElement('span');
            span.textContent = run.text || run.alt || '';
            img.replaceWith(span);
          };
          container.appendChild(img);
          continue;
        }
      }
      if (run.text) {
        container.appendChild(document.createTextNode(run.text));
      }
    }
  }

  function isPreviewMode() {
    return /^\/overlay\/preview\//.test(location.pathname);
  }

  function mountPreviewBadge() {
    if (!isPreviewMode()) return;
    const params = new URLSearchParams(location.search);
    if (params.get('badge') === '0') return;
    if (document.getElementById('stosup-preview-badge')) return;
    const mount = () => {
      if (!document.body || document.getElementById('stosup-preview-badge')) return;
      const badge = document.createElement('div');
      badge.id = 'stosup-preview-badge';
      badge.textContent = 'PREVIEW MODE / 本番データ非連動';
      badge.style.cssText = [
        'position:fixed',
        'top:10px',
        'right:12px',
        'z-index:999999',
        'pointer-events:none',
        'padding:4px 10px',
        'border-radius:4px',
        'background:rgba(15, 23, 42, 0.88)',
        'border:1px solid rgba(251, 191, 36, 0.65)',
        'color:#fde68a',
        'font-family:"BIZ UDPGothic","Yu Gothic UI",sans-serif',
        'font-size:11px',
        'font-weight:700',
        'letter-spacing:0.06em',
      ].join(';');
      document.body.appendChild(badge);
    };
    if (document.body) {
      mount();
    } else {
      document.addEventListener('DOMContentLoaded', mount, { once: true });
    }
  }

  function buildWsUrl(channel) {
    mountPreviewBadge();
    const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const preview = isPreviewMode() ? '&preview=1' : '';
    return `${proto}//${location.host}/ws?channel=${encodeURIComponent(channel)}${preview}`;
  }

  function fallbackAvatar(name) {
    const ch = String(name || '?').slice(0, 1);
    return `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'><rect width='64' height='64' rx='8' fill='%232b303a'/><text x='50%25' y='54%25' dominant-baseline='middle' text-anchor='middle' fill='%23f7f8fa' font-family='sans-serif' font-weight='bold' font-size='28'>${encodeURIComponent(ch)}</text></svg>`;
  }

  function applyCustomCss(styleEl, rawCss) {
    if (!styleEl) return;
    const nextCss = String(rawCss || '');
    if (styleEl.textContent !== nextCss) {
      styleEl.textContent = nextCss;
    }
  }

  global.StosupOverlay = {
    normalizeTheme,
    resolveBg,
    safeHexColor,
    applyFont,
    applyOverlayVisualStyle,
    applyCustomCss,
    applyModTheme,
    syncMods,
    renderMessageRuns,
    fallbackAvatar,
    isPreviewMode,
    buildWsUrl,
  };
})(window);
