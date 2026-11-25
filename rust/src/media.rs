use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::models::{ImportedMediaFile, MediaKind};

/// 許可する画像拡張子
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"];
/// 許可する動画拡張子
const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mov", "m4v", "ogv"];
/// 許可する音声（SE）拡張子
const AUDIO_EXTS: &[&str] = &["mp3", "wav", "ogg", "m4a", "flac", "aac"];

/// 拡張子からメディア種別を判定する
pub fn detect_media_kind_from_ext(ext: &str) -> Option<MediaKind> {
    let lower = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    if IMAGE_EXTS.contains(&lower.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTS.contains(&lower.as_str()) {
        Some(MediaKind::Video)
    } else if AUDIO_EXTS.contains(&lower.as_str()) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

/// ファイル拡張子から HTTP Content-Type を判定する
pub fn content_type_for_file_name(file_name: &str) -> &'static str {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" | "aac" => "audio/mp4",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

/// パストラバーサルを防止した安全なファイル名チェック
pub fn is_safe_stored_file_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 160 {
        return false;
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.contains('\0') {
        return false;
    }
    Path::new(name).file_name().and_then(|s| s.to_str()) == Some(name)
}

/// ローカルの画像・動画・音声ファイルを Stosup のメディア保存フォルダへコピー保存する
pub fn import_media_file(media_dir: &Path, source_path: &str) -> Result<ImportedMediaFile> {
    let trimmed = source_path.trim().trim_matches('"');
    if trimmed.is_empty() {
        return Err(anyhow!("ファイルパスが空です"));
    }
    let src = PathBuf::from(trimmed);
    if !src.is_file() {
        return Err(anyhow!("指定されたファイルが見つかりません: {}", trimmed));
    }

    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let media_kind = detect_media_kind_from_ext(&ext).ok_or_else(|| {
        anyhow!(
            "対応していないファイル形式です（.{}）。画像（PNG/JPG/GIF/WebP/SVG）、動画（MP4/WebM/MOV）、または音声（MP3/WAV/OGG）を選択してください。",
            ext
        )
    })?;

    std::fs::create_dir_all(media_dir)
        .context("メディア保存ディレクトリの作成に失敗しました")?;

    let stored_file_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
    let dest = media_dir.join(&stored_file_name);
    let file_size_bytes = std::fs::copy(&src, &dest)
        .with_context(|| format!("メディアファイルのコピー保存に失敗しました: {}", trimmed))?;

    let original_file_name = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&stored_file_name)
        .to_string();

    Ok(ImportedMediaFile {
        stored_file_name,
        original_file_name,
        media_kind,
        file_size_bytes,
    })
}

/// 保存済みメディアファイルを安全に削除する
pub fn remove_stored_media_file(media_dir: &Path, stored_file_name: &str) {
    if !is_safe_stored_file_name(stored_file_name) {
        return;
    }
    let target = media_dir.join(stored_file_name);
    let _ = std::fs::remove_file(target);
}

/// OS のファイル選択ダイアログを開き、画像・動画・音声ファイルパスを 1 つ選択する
pub fn pick_media_file_dialog() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let filter = "画像・動画・音声 (*.png;*.jpg;*.gif;*.webp;*.svg;*.mp4;*.webm;*.mov;*.mp3;*.wav;*.ogg)\0*.png;*.jpg;*.jpeg;*.gif;*.webp;*.svg;*.bmp;*.mp4;*.webm;*.mov;*.m4v;*.mp3;*.wav;*.ogg;*.m4a\0画像ファイル (*.png;*.jpg;*.gif;*.webp;*.svg)\0*.png;*.jpg;*.jpeg;*.gif;*.webp;*.svg;*.bmp\0動画ファイル (*.mp4;*.webm;*.mov)\0*.mp4;*.webm;*.mov;*.m4v\0音声ファイル (*.mp3;*.wav;*.ogg)\0*.mp3;*.wav;*.ogg;*.m4a\0すべてのファイル (*.*)\0*.*\0\0";
        pick_file_windows("演出に使用する画像・動画・音声を選択", filter)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// OS のファイル選択ダイアログを開き、効果音（SE）ファイルパスを 1 つ選択する
pub fn pick_sound_file_dialog() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let filter = "音声ファイル (*.mp3;*.wav;*.ogg;*.m4a)\0*.mp3;*.wav;*.ogg;*.m4a;*.flac;*.aac\0すべてのファイル (*.*)\0*.*\0\0";
        pick_file_windows("同時再生する効果音 (SE) ファイルを選択", filter)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn pick_file_windows(title: &str, filter_str: &str) -> Option<String> {
    use std::ffi::c_void;

    const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
    const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
    const OFN_NOCHANGEDIR: u32 = 0x0000_0008;

    #[repr(C)]
    struct OpenFileNameW {
        l_struct_size: u32,
        hwnd_owner: *mut c_void,
        h_instance: *mut c_void,
        lpstr_filter: *const u16,
        lpstr_custom_filter: *mut u16,
        n_max_cust_filter: u32,
        n_filter_index: u32,
        lpstr_file: *mut u16,
        n_max_file: u32,
        lpstr_file_title: *mut u16,
        n_max_file_title: u32,
        lpstr_initial_dir: *const u16,
        lpstr_title: *const u16,
        flags: u32,
        n_file_offset: u16,
        n_file_extension: u16,
        lpstr_def_ext: *const u16,
        l_cust_data: isize,
        lpfn_hook: *const c_void,
        lp_template_name: *const u16,
        pv_reserved: *mut c_void,
        dw_reserved: u32,
        flags_ex: u32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(lpofn: *mut OpenFileNameW) -> i32;
    }

    let filter_w: Vec<u16> = filter_str.encode_utf16().collect();
    let title_w: Vec<u16> = format!("{}\0", title).encode_utf16().collect();
    let mut file_buf = vec![0u16; 4096];

    let mut ofn = OpenFileNameW {
        l_struct_size: std::mem::size_of::<OpenFileNameW>() as u32,
        hwnd_owner: std::ptr::null_mut(),
        h_instance: std::ptr::null_mut(),
        lpstr_filter: filter_w.as_ptr(),
        lpstr_custom_filter: std::ptr::null_mut(),
        n_max_cust_filter: 0,
        n_filter_index: 1,
        lpstr_file: file_buf.as_mut_ptr(),
        n_max_file: file_buf.len() as u32,
        lpstr_file_title: std::ptr::null_mut(),
        n_max_file_title: 0,
        lpstr_initial_dir: std::ptr::null(),
        lpstr_title: title_w.as_ptr(),
        flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        n_file_offset: 0,
        n_file_extension: 0,
        lpstr_def_ext: std::ptr::null(),
        l_cust_data: 0,
        lpfn_hook: std::ptr::null(),
        lp_template_name: std::ptr::null(),
        pv_reserved: std::ptr::null_mut(),
        dw_reserved: 0,
        flags_ex: 0,
    };

    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if ok != 0 {
        let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
        let selected = String::from_utf16_lossy(&file_buf[..len]).trim().to_string();
        if !selected.is_empty() {
            return Some(selected);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_import_and_safety_checks() {
        assert!(is_safe_stored_file_name("abc-123.mp4"));
        assert!(!is_safe_stored_file_name("../secret.png"));
        assert!(!is_safe_stored_file_name("a/b.png"));
        assert!(!is_safe_stored_file_name("a\\b.png"));
        assert_eq!(detect_media_kind_from_ext("PNG"), Some(MediaKind::Image));
        assert_eq!(detect_media_kind_from_ext("mp4"), Some(MediaKind::Video));
        assert_eq!(detect_media_kind_from_ext("mp3"), Some(MediaKind::Audio));
        assert_eq!(detect_media_kind_from_ext("wav"), Some(MediaKind::Audio));
        assert_eq!(detect_media_kind_from_ext("exe"), None);
        assert_eq!(content_type_for_file_name("anim.webm"), "video/webm");
        assert_eq!(content_type_for_file_name("se.mp3"), "audio/mpeg");

        let dir = tempfile::tempdir().unwrap();
        let src_file = dir.path().join("sample.gif");
        std::fs::write(&src_file, b"GIF89a").unwrap();
        let media_dir = dir.path().join("stored");

        let imported = import_media_file(&media_dir, src_file.to_str().unwrap()).unwrap();
        assert_eq!(imported.media_kind, MediaKind::Image);
        assert_eq!(imported.original_file_name, "sample.gif");
        assert_eq!(imported.file_size_bytes, 6);
        assert!(media_dir.join(&imported.stored_file_name).is_file());

        remove_stored_media_file(&media_dir, &imported.stored_file_name);
        assert!(!media_dir.join(&imported.stored_file_name).exists());
    }
}
