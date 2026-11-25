use std::collections::BTreeSet;

/// 動作中の OS にインストールされているシステムフォントのファミリー名一覧を取得する
pub fn list_installed_system_fonts() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        list_windows_system_fonts()
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![
            "Yu Gothic UI".to_string(),
            "Meiryo".to_string(),
            "Noto Sans JP".to_string(),
            "Segoe UI".to_string(),
        ]
    }
}

#[cfg(target_os = "windows")]
fn list_windows_system_fonts() -> Vec<String> {
    use std::ffi::c_void;

    const DEFAULT_CHARSET: u8 = 1;

    #[repr(C)]
    struct LogFontW {
        lf_height: i32,
        lf_width: i32,
        lf_escapement: i32,
        lf_orientation: i32,
        lf_weight: i32,
        lf_italic: u8,
        lf_underline: u8,
        lf_strike_out: u8,
        lf_char_set: u8,
        lf_out_precision: u8,
        lf_clip_precision: u8,
        lf_quality: u8,
        lf_pitch_and_family: u8,
        lf_face_name: [u16; 32],
    }

    type EnumFontFamExProcW =
        unsafe extern "system" fn(*const LogFontW, *const c_void, u32, isize) -> i32;

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        fn ReleaseDC(hwnd: *mut c_void, hdc: *mut c_void) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn EnumFontFamiliesExW(
            hdc: *mut c_void,
            lp_logfont: *mut LogFontW,
            lp_proc: EnumFontFamExProcW,
            l_param: isize,
            dw_flags: u32,
        ) -> i32;
    }

    unsafe extern "system" fn callback(
        lpelfe: *const LogFontW,
        _lpntme: *const c_void,
        _font_type: u32,
        lparam: isize,
    ) -> i32 {
        if lpelfe.is_null() || lparam == 0 {
            return 1;
        }
        let set = &mut *(lparam as *mut BTreeSet<String>);
        let face = &(*lpelfe).lf_face_name;
        let len = face.iter().position(|&c| c == 0).unwrap_or(face.len());
        let name = String::from_utf16_lossy(&face[..len]).trim().to_string();
        // 縦書き専用(@始まり)や空名を除外
        if !name.is_empty() && !name.starts_with('@') {
            set.insert(name);
        }
        1
    }

    let mut fonts = BTreeSet::<String>::new();
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());
        if !hdc.is_null() {
            let mut lf = LogFontW {
                lf_height: 0,
                lf_width: 0,
                lf_escapement: 0,
                lf_orientation: 0,
                lf_weight: 0,
                lf_italic: 0,
                lf_underline: 0,
                lf_strike_out: 0,
                lf_char_set: DEFAULT_CHARSET,
                lf_out_precision: 0,
                lf_clip_precision: 0,
                lf_quality: 0,
                lf_pitch_and_family: 0,
                lf_face_name: [0u16; 32],
            };
            EnumFontFamiliesExW(
                hdc,
                &mut lf,
                callback,
                (&mut fonts as *mut BTreeSet<String>) as isize,
                0,
            );
            ReleaseDC(std::ptr::null_mut(), hdc);
        }
    }

    fonts.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_installed_system_fonts_non_empty() {
        let fonts = list_installed_system_fonts();
        assert!(!fonts.is_empty(), "システムフォントが1件以上取得できること");
        assert!(
            fonts.iter().all(|f| !f.starts_with('@')),
            "縦書き用の @ 始まりフォントが除外されていること"
        );
    }
}
