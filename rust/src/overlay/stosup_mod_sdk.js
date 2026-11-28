/* Stosup Mod SDK v1.0.0 — ユーザー作成演出 MOD（リアクション / テーマ / 独自オーバーレイ）用公式ランタイム */
(function (global) {
  'use strict';

  const SDK_VERSION = '1.0.0';

  // リアクション演出ハンドラ登録テーブル
  // key: `${modId}:${effectId}` または `effectId`（ロード中のアクティブ MOD コンテキストと自動結合）
  const effectRegistry = new Map();
  // オーバーレイテーマフック登録テーブル
  const themeHookRegistry = new Map();

  // スクリプトロード中に登録元の modId / componentId を自動ひも付けするためのコンテキスト
  let activeLoadingModId = '';
  let activeLoadingComponentId = '';

  function _setLoadingContext(modId, componentId) {
    activeLoadingModId = String(modId || '');
    activeLoadingComponentId = String(componentId || '');
  }

  function _clearLoadingContext() {
    activeLoadingModId = '';
    activeLoadingComponentId = '';
  }

  /**
   * リアクションステージ独自演出ハンドラを登録する
   * @param {string} effectId - mod.json の reaction_effects[].id（または "modId:effectId"）
   * @param {function(object): (void|Promise<void>)} handler - 演出実行関数
   */
  function registerEffect(effectId, handler) {
    const id = String(effectId || '').trim();
    if (!id || typeof handler !== 'function') {
      console.warn('[StosupMod] registerEffect: 無効な引数です', effectId);
      return;
    }
    if (id.includes(':')) {
      effectRegistry.set(id, handler);
    } else if (activeLoadingModId) {
      effectRegistry.set(`${activeLoadingModId}:${id}`, handler);
    }
    effectRegistry.set(id, handler);
  }

  function _getEffect(modId, effectId) {
    const full = `${modId}:${effectId}`;
    return effectRegistry.get(full) || effectRegistry.get(effectId) || null;
  }

  /**
   * オーバーレイ追加テーマの任意 JS フックを登録する
   * @param {string} themeId - mod.json の overlay_themes[].id（または "mod:modId:themeId"）
   * @param {{ mount?: Function, update?: Function, unmount?: Function }} hooks
   */
  function registerThemeHook(themeId, hooks) {
    const id = String(themeId || '').trim();
    if (!id || !hooks || typeof hooks !== 'object') {
      console.warn('[StosupMod] registerThemeHook: 無効な引数です', themeId);
      return;
    }
    if (id.startsWith('mod:')) {
      themeHookRegistry.set(id, hooks);
    } else if (activeLoadingModId) {
      themeHookRegistry.set(`mod:${activeLoadingModId}:${id}`, hooks);
    }
    themeHookRegistry.set(id, hooks);
  }

  function _getThemeHook(modId, themeId) {
    const full = `mod:${modId}:${themeId}`;
    return themeHookRegistry.get(full) || themeHookRegistry.get(themeId) || null;
  }

  function isPreviewMode() {
    return /^\/overlay\/preview\//.test(location.pathname);
  }

  function escapeHtml(raw) {
    return String(raw ?? '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function assetUrl(modId, relativePath) {
    const id = encodeURIComponent(String(modId || '').trim());
    const rel = String(relativePath || '')
      .trim()
      .replace(/^\/+/, '');
    return `/mods/${id}/${rel}`;
  }

  function getQueryParams() {
    const out = {};
    const sp = new URLSearchParams(location.search);
    for (const [k, v] of sp.entries()) {
      out[k] = v;
    }
    return out;
  }

  // 簡易 Audio 再生ヘルパー（独立カスタムオーバーレイ向け）
  let sharedAudioCtx = null;
  const audioBufferCache = new Map();

  async function playAudio(url, volume0to100) {
    if (!url) return;
    const vol = Math.max(0, Math.min(1, Number(volume0to100 ?? 80) / 100));
    if (vol <= 0) return;
    try {
      const AC = global.AudioContext || global.webkitAudioContext;
      if (AC) {
        if (!sharedAudioCtx) sharedAudioCtx = new AC();
        if (sharedAudioCtx.state === 'suspended') {
          await sharedAudioCtx.resume().catch(() => {});
        }
        let buf = audioBufferCache.get(url);
        if (!buf) {
          const res = await fetch(url);
          if (!res.ok) return;
          const arr = await res.arrayBuffer();
          buf = await sharedAudioCtx.decodeAudioData(arr);
          if (audioBufferCache.size > 32) {
            const first = audioBufferCache.keys().next().value;
            audioBufferCache.delete(first);
          }
          audioBufferCache.set(url, buf);
        }
        const src = sharedAudioCtx.createBufferSource();
        const gain = sharedAudioCtx.createGain();
        src.buffer = buf;
        gain.gain.value = vol;
        src.connect(gain);
        gain.connect(sharedAudioCtx.destination);
        src.start(0);
        return;
      }
    } catch (_) {
      // Web Audio 不可時は HTMLAudioElement にフォールバック
    }
    try {
      const a = new Audio(url);
      a.volume = vol;
      a.play().catch(() => {});
    } catch (_) {}
  }

  /**
   * 独立カスタムオーバーレイ向けの自動再接続 WebSocket クライアントを生成する
   * @param {{ channels?: string[], reconnectDelayMs?: number }} [options]
   */
  function connect(options) {
    const opts = options || {};
    const channels =
      Array.isArray(opts.channels) && opts.channels.length > 0
        ? opts.channels.map((c) => String(c).trim().toLowerCase()).filter(Boolean)
        : ['all'];
    const reconnectDelayMs = Math.max(500, Number(opts.reconnectDelayMs ?? 2000));
    // 複数チャンネル指定時は all で接続し、単一チャンネルならそのチャンネル専用で軽量購読
    const wsChannel = channels.length === 1 ? channels[0] : 'all';

    const listeners = new Map();
    let ws = null;
    let closedByUser = false;
    let reconnectTimer = null;
    let lastSnapshot = null;

    function emit(eventName, payload) {
      const set = listeners.get(eventName);
      if (set) {
        for (const fn of [...set]) {
          try {
            fn(payload);
          } catch (e) {
            console.error(`[StosupMod] listener error (${eventName}):`, e);
          }
        }
      }
      if (eventName !== '*') {
        const wild = listeners.get('*');
        if (wild) {
          for (const fn of [...wild]) {
            try {
              fn({ event: eventName, data: payload });
            } catch (e) {
              console.error('[StosupMod] wildcard listener error:', e);
            }
          }
        }
      }
    }

    function openSocket() {
      if (closedByUser) return;
      const url =
        global.StosupOverlay && typeof global.StosupOverlay.buildWsUrl === 'function'
          ? global.StosupOverlay.buildWsUrl(wsChannel)
          : `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws?channel=${encodeURIComponent(wsChannel)}${isPreviewMode() ? '&preview=1' : ''}`;

      ws = new WebSocket(url);
      ws.onopen = () => emit('open', { channel: wsChannel, isPreview: isPreviewMode() });
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data);
          if (!msg || !msg.type) return;
          if (msg.type === 'snapshot') {
            lastSnapshot = msg;
            if (global.StosupOverlay && typeof global.StosupOverlay.syncMods === 'function') {
              global.StosupOverlay.syncMods(msg.mods, msg.mods_revision);
            }
            emit('snapshot', msg);
            return;
          }
          if (msg.type === 'mods_updated') {
            if (global.StosupOverlay && typeof global.StosupOverlay.syncMods === 'function') {
              global.StosupOverlay.syncMods(msg.mods, msg.revision);
            }
            emit('mods_updated', msg);
            return;
          }
          // 使いやすいイベント名で配信（snake_case そのまま + ペイロード展開）
          switch (msg.type) {
            case 'comment':
              emit('comment', msg.comment);
              break;
            case 'super_chat':
              emit('superchat', { items: msg.items || [], enabled: msg.enabled });
              emit('super_chat', { items: msg.items || [], enabled: msg.enabled });
              break;
            case 'picked_comment':
              emit('pickup', { picked: msg.picked, enabled: msg.enabled });
              emit('picked_comment', { picked: msg.picked, enabled: msg.enabled });
              break;
            case 'poll':
              emit('poll', { poll: msg.poll, enabled: msg.enabled });
              break;
            case 'clear_poll':
              emit('clear_poll', {});
              break;
            case 'queue':
              emit('queue', { queue: msg.queue, enabled: msg.enabled });
              break;
            case 'counters':
              emit('counters', msg.counters || []);
              break;
            case 'first_time':
              emit('first_time', { first_time: msg.first_time, enabled: msg.enabled });
              break;
            case 'lottery_result':
              emit('lottery_result', msg.result);
              break;
            case 'clear_lottery':
              emit('clear_lottery', {});
              break;
            case 'media_trigger':
              emit('media_trigger', msg.trigger);
              break;
            case 'media_config':
              emit('media_config', { media_config: msg.media_config, enabled: msg.enabled });
              break;
            case 'clear_media':
              emit('clear_media', {});
              break;
            case 'config_updated':
              emit('config_updated', msg.config);
              break;
            default:
              emit(msg.type, msg);
              break;
          }
        } catch (e) {
          console.error('[StosupMod] message parse error:', e);
        }
      };
      ws.onclose = () => {
        emit('close', {});
        if (!closedByUser) {
          reconnectTimer = setTimeout(openSocket, reconnectDelayMs);
        }
      };
      ws.onerror = () => {
        try {
          ws.close();
        } catch (_) {}
      };
    }

    openSocket();

    const client = {
      get isPreview() {
        return isPreviewMode();
      },
      get lastSnapshot() {
        return lastSnapshot;
      },
      on(eventName, callback) {
        if (!eventName || typeof callback !== 'function') return client;
        if (!listeners.has(eventName)) listeners.set(eventName, new Set());
        listeners.get(eventName).add(callback);
        return client;
      },
      off(eventName, callback) {
        const set = listeners.get(eventName);
        if (set) set.delete(callback);
        return client;
      },
      close() {
        closedByUser = true;
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        if (ws) {
          try {
            ws.close();
          } catch (_) {}
          ws = null;
        }
      },
    };
    return client;
  }

  const utils = {
    renderMessageRuns(container, fallbackText, runs) {
      if (global.StosupOverlay && typeof global.StosupOverlay.renderMessageRuns === 'function') {
        return global.StosupOverlay.renderMessageRuns(container, fallbackText, runs);
      }
      container.textContent = fallbackText || '';
    },
    fallbackAvatar(name) {
      if (global.StosupOverlay && typeof global.StosupOverlay.fallbackAvatar === 'function') {
        return global.StosupOverlay.fallbackAvatar(name);
      }
      return '';
    },
    escapeHtml,
    playAudio,
    assetUrl,
    getQueryParams,
    isPreviewMode,
  };

  global.StosupMod = {
    sdkVersion: SDK_VERSION,
    registerEffect,
    registerThemeHook,
    connect,
    utils,
    // 内部ランタイム連携 API（通常 MOD から直接呼ぶ必要はありません）
    _setLoadingContext,
    _clearLoadingContext,
    _getEffect,
    _getThemeHook,
  };
})(window);
