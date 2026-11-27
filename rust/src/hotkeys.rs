use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::models::HotkeyBinding;

/// ホットキー管理マネージャー（Windows の RegisterHotKey をメッセージ専用ウィンドウで完全イベント駆動管理）
pub struct HotkeyManager {
    tx: Mutex<Option<std_mpsc::Sender<Vec<HotkeyBinding>>>>,
    #[cfg(windows)]
    msg_hwnd: Arc<AtomicIsize>,
    #[cfg(windows)]
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            tx: Mutex::new(None),
            #[cfg(windows)]
            msg_hwnd: Arc::new(AtomicIsize::new(0)),
            #[cfg(windows)]
            worker: Mutex::new(None),
        }
    }

    /// 稼働中のホットキー監視スレッドがあればクリーンに停止・回収する
    #[cfg(windows)]
    fn stop_worker(&self) {
        if let Ok(mut guard) = self.tx.lock() {
            *guard = None;
        }
        let hwnd = self.msg_hwnd.swap(0, Ordering::SeqCst);
        if hwnd != 0 {
            use windows_sys::Win32::Foundation::HWND;
            use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;
            unsafe {
                PostMessageW(hwnd as HWND, WM_HOTKEY_STOP, 0, 0);
            }
        }
        if let Ok(mut handle_guard) = self.worker.lock() {
            if let Some(handle) = handle_guard.take() {
                let _ = handle.join();
            }
        }
    }

    /// ホットキー監視スレッドを起動し、押下されたバインディング ID を非同期チャンネルへ送る
    pub fn start(&self, initial: Vec<HotkeyBinding>) -> mpsc::UnboundedReceiver<String> {
        #[cfg(windows)]
        self.stop_worker();

        let (fire_tx, fire_rx) = mpsc::unbounded_channel::<String>();
        let (cmd_tx, cmd_rx) = std_mpsc::channel::<Vec<HotkeyBinding>>();
        let _ = cmd_tx.send(initial);

        if let Ok(mut guard) = self.tx.lock() {
            *guard = Some(cmd_tx);
        }

        #[cfg(windows)]
        {
            let hwnd_slot = Arc::clone(&self.msg_hwnd);
            let (ready_tx, ready_rx) = std_mpsc::sync_channel::<()>(1);
            if let Ok(handle) = std::thread::Builder::new()
                .name("stosup-hotkeys".to_string())
                .spawn(move || run_windows_hotkey_loop(cmd_rx, fire_tx, hwnd_slot, ready_tx))
            {
                // メッセージ専用ウィンドウ作成と初回登録完了を待機し、直後の更新・終了との競合を排除
                let _ = ready_rx.recv();
                if let Ok(mut worker_guard) = self.worker.lock() {
                    *worker_guard = Some(handle);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (cmd_rx, fire_tx);
        }

        fire_rx
    }

    /// 登録するホットキー一覧をリアルタイムに更新する
    pub fn update_bindings(&self, bindings: Vec<HotkeyBinding>) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(bindings);
            }
        }
        #[cfg(windows)]
        {
            let hwnd = self.msg_hwnd.load(Ordering::SeqCst);
            if hwnd != 0 {
                use windows_sys::Win32::Foundation::HWND;
                use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;
                unsafe {
                    PostMessageW(hwnd as HWND, WM_HOTKEY_RELOAD, 0, 0);
                }
            }
        }
    }
}

#[cfg(windows)]
const WM_HOTKEY_RELOAD: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;
#[cfg(windows)]
const WM_HOTKEY_STOP: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[cfg(windows)]
impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

/// キー名文字列を Windows 仮想キーコード (VK_*) へ変換する
pub fn parse_virtual_key(key: &str) -> Option<u32> {
    let k = key.trim().to_ascii_uppercase();
    if k.len() == 1 {
        let ch = k.as_bytes()[0];
        if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
            return Some(ch as u32);
        }
    }
    match k.as_str() {
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "F4" => Some(0x73),
        "F5" => Some(0x74),
        "F6" => Some(0x75),
        "F7" => Some(0x76),
        "F8" => Some(0x77),
        "F9" => Some(0x78),
        "F10" => Some(0x79),
        "F11" => Some(0x7A),
        "F12" => Some(0x7B),
        "SPACE" => Some(0x20),
        "ENTER" | "RETURN" => Some(0x0D),
        "ESCAPE" | "ESC" => Some(0x1B),
        "TAB" => Some(0x09),
        "BACKSPACE" => Some(0x08),
        "INSERT" => Some(0x2D),
        "DELETE" => Some(0x2E),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "UP" => Some(0x26),
        "DOWN" => Some(0x28),
        "LEFT" => Some(0x25),
        "RIGHT" => Some(0x27),
        "NUM0" => Some(0x60),
        "NUM1" => Some(0x61),
        "NUM2" => Some(0x62),
        "NUM3" => Some(0x63),
        "NUM4" => Some(0x64),
        "NUM5" => Some(0x65),
        "NUM6" => Some(0x66),
        "NUM7" => Some(0x67),
        "NUM8" => Some(0x68),
        "NUM9" => Some(0x69),
        _ => None,
    }
}

#[cfg(windows)]
fn run_windows_hotkey_loop(
    cmd_rx: std_mpsc::Receiver<Vec<HotkeyBinding>>,
    fire_tx: mpsc::UnboundedSender<String>,
    hwnd_slot: Arc<AtomicIsize>,
    ready_tx: std_mpsc::SyncSender<()>,
) {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetMessageW, HWND_MESSAGE, MSG, WM_HOTKEY,
    };

    // メッセージ専用ウィンドウ (HWND_MESSAGE) を生成し、25ms ビジーポーリングを撤廃して完全イベント駆動にする
    const STATIC_CLASS_UTF16: &[u16] = &[b'S' as u16, b'T' as u16, b'A' as u16, b'T' as u16, b'I' as u16, b'C' as u16, 0];
    let hwnd: HWND = unsafe {
        CreateWindowExW(
            0,
            STATIC_CLASS_UTF16.as_ptr(),
            null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            null_mut(),
            null(),
        )
    };
    if !hwnd.is_null() {
        hwnd_slot.store(hwnd as isize, Ordering::SeqCst);
    }

    let mut registered: HashMap<i32, String> = HashMap::new();

    let apply_bindings =
        |bindings: Vec<HotkeyBinding>, registered: &mut HashMap<i32, String>, target_hwnd: HWND| {
            // 既存の登録をすべて解除
            for &id in registered.keys() {
                unsafe {
                    UnregisterHotKey(target_hwnd, id);
                }
            }
            registered.clear();

            let mut next_id: i32 = 1001;
            for b in bindings {
                if !b.enabled {
                    continue;
                }
                let Some(vk) = parse_virtual_key(&b.key) else {
                    continue;
                };
                // 修飾キーなし単体キーは誤爆しやすいため F1〜F12 以外は最低 1 つの修飾キーを必須とする
                let is_fn_key = (0x70..=0x7B).contains(&vk);
                if !is_fn_key && !b.ctrl && !b.alt && !b.shift {
                    continue;
                }
                let mut mods: u32 = MOD_NOREPEAT;
                if b.ctrl {
                    mods |= MOD_CONTROL;
                }
                if b.alt {
                    mods |= MOD_ALT;
                }
                if b.shift {
                    mods |= MOD_SHIFT;
                }
                let ok = unsafe { RegisterHotKey(target_hwnd, next_id, mods, vk) };
                if ok != 0 {
                    registered.insert(next_id, b.id);
                    next_id += 1;
                }
            }
        };

    // 初期化時にすでに積まれているバインディングを先に反映
    let drain_pending = |registered: &mut HashMap<i32, String>| -> bool {
        let mut latest: Option<Vec<HotkeyBinding>> = None;
        let mut disconnected = false;
        loop {
            match cmd_rx.try_recv() {
                Ok(bindings) => latest = Some(bindings),
                Err(std_mpsc::TryRecvError::Empty) => break,
                Err(std_mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if let Some(bindings) = latest {
            apply_bindings(bindings, registered, hwnd);
        }
        disconnected
    };

    if !hwnd.is_null() {
        let disconnected = drain_pending(&mut registered);
        let _ = ready_tx.send(());
        if !disconnected {
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                // GetMessageW でカーネル待機（アイドル時 CPU 起床 0Hz・押下時即時応答）
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                    match msg.message {
                        WM_HOTKEY => {
                            let id = msg.wParam as i32;
                            if let Some(binding_id) = registered.get(&id) {
                                let _ = fire_tx.send(binding_id.clone());
                            }
                        }
                        WM_HOTKEY_RELOAD => {
                            if drain_pending(&mut registered) {
                                break;
                            }
                        }
                        WM_HOTKEY_STOP => break,
                        _ => {}
                    }
                }
            }
        }
    } else {
        // 万一メッセージウィンドウ作成に失敗した場合のフォールバック（未登録時は完全ブロッキング待機）
        let _ = ready_tx.send(());
        use std::time::Duration;
        use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};
        loop {
            if registered.is_empty() {
                match cmd_rx.recv() {
                    Ok(bindings) => apply_bindings(bindings, &mut registered, null_mut()),
                    Err(_) => break,
                }
            } else if drain_pending(&mut registered) {
                break;
            }
            if registered.is_empty() {
                continue;
            }
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_HOTKEY {
                        let id = msg.wParam as i32;
                        if let Some(binding_id) = registered.get(&id) {
                            let _ = fire_tx.send(binding_id.clone());
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    hwnd_slot.store(0, Ordering::Release);
    for &id in registered.keys() {
        unsafe {
            UnregisterHotKey(hwnd, id);
        }
    }
    if !hwnd.is_null() {
        unsafe {
            DestroyWindow(hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AutomationActionKind;

    #[test]
    fn test_parse_virtual_key() {
        assert_eq!(parse_virtual_key("F1"), Some(0x70));
        assert_eq!(parse_virtual_key("f12"), Some(0x7B));
        assert_eq!(parse_virtual_key("a"), Some(b'A' as u32));
        assert_eq!(parse_virtual_key("9"), Some(b'9' as u32));
        assert_eq!(parse_virtual_key("Space"), Some(0x20));
        assert_eq!(parse_virtual_key("INVALID_KEY"), None);
    }

    #[test]
    fn test_hotkey_manager_event_driven_lifecycle() {
        let binding_a = HotkeyBinding {
            id: "hk-test-a".to_string(),
            name: "テストA".to_string(),
            ctrl: true,
            alt: true,
            shift: true,
            key: "F11".to_string(),
            action: AutomationActionKind::ClearPickup,
            target_id: String::new(),
            secondary_target: String::new(),
            enabled: true,
        };
        let binding_b = HotkeyBinding {
            id: "hk-test-b".to_string(),
            name: "テストB".to_string(),
            ctrl: true,
            alt: true,
            shift: true,
            key: "F12".to_string(),
            action: AutomationActionKind::ClearMedia,
            target_id: String::new(),
            secondary_target: String::new(),
            enabled: true,
        };

        let mgr = HotkeyManager::new();
        let mut rx = mgr.start(vec![binding_a]);

        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::HWND;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                IsWindow, PostMessageW, WM_HOTKEY,
            };

            // start() 完了時点で HWND_MESSAGE が決定論的に生成済みであること
            let hwnd = mgr.msg_hwnd.load(Ordering::Acquire);
            assert_ne!(hwnd, 0);
            assert_ne!(unsafe { IsWindow(hwnd as HWND) }, 0);

            // WM_HOTKEY をウィンドウへ直接ポストし、イベント駆動ループが即座に発火 ID を返すこと
            unsafe {
                PostMessageW(hwnd as HWND, WM_HOTKEY, 1001, 0);
            }
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();
            let fired_a = rt
                .block_on(async {
                    tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await
                })
                .expect("WM_HOTKEY 応答がタイムアウトしないこと")
                .expect("バインディング ID が届くこと");
            assert_eq!(fired_a, "hk-test-a");

            // バインディング更新 (WM_HOTKEY_RELOAD) 後に新しい ID が発火すること
            mgr.update_bindings(vec![binding_b]);
            // PostMessageW は FIFO 順序が保証されるため、続けて WM_HOTKEY を送れば更新後テーブルで処理される
            unsafe {
                PostMessageW(hwnd as HWND, WM_HOTKEY, 1001, 0);
            }
            let fired_b = rt
                .block_on(async {
                    tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await
                })
                .expect("更新後の WM_HOTKEY 応答がタイムアウトしないこと")
                .expect("更新後のバインディング ID が届くこと");
            assert_eq!(fired_b, "hk-test-b");

            // 二重 start() でも旧スレッドと旧 HWND がリークせず安全に破棄されること
            let _rx2 = mgr.start(Vec::new());
            assert_eq!(unsafe { IsWindow(hwnd as HWND) }, 0);
            let hwnd2 = mgr.msg_hwnd.load(Ordering::Acquire);
            assert_ne!(hwnd2, 0);

            // Drop でスレッドが join されウィンドウが確実に破棄されること
            drop(mgr);
            assert_eq!(unsafe { IsWindow(hwnd2 as HWND) }, 0);
        }
    }
}
