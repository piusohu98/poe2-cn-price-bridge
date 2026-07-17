#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

use anyhow::{Context, Result, anyhow, bail};
use arboard::Clipboard;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use reqwest::Method;
use reqwest::blocking::Client;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::mem::MaybeUninit;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::slice;
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, LocalFree,
    POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreatePen, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_TOP,
    DT_VCENTER, DT_WORDBREAK, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_BOLD, FW_NORMAL,
    FillRect, HBRUSH, HDC, HFONT, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID,
    RoundRect, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, RegisterHotKey, ReleaseCapture, UnregisterHotKey,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIIF_WARNING, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW,
    GWLP_USERDATA, GetClientRect, GetCursorPos, GetMessageW, GetSystemMetrics, GetWindowLongPtrW,
    GetWindowRect, HICON, HTCAPTION, IDC_ARROW, IDI_APPLICATION, IMAGE_ICON, KillTimer,
    LR_LOADFROMFILE, LoadCursorW, LoadIconW, LoadImageW, MF_SEPARATOR, MF_STRING, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW,
    SWP_SHOWWINDOW, SendMessageW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage,
    WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_CREATE, WM_DESTROY, WM_EXITSIZEMOVE,
    WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCCREATE, WM_NCDESTROY, WM_NULL,
    WM_PAINT, WM_RBUTTONUP, WM_SIZE, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

const WINDOW_CLASS_NAME: &str = "Poe2CnPriceBridgeRustWindow";
const APP_DISPLAY_NAME: &str = "流放2查价助手";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const USER_AGENT: &str = concat!("POE2PriceHelper/", env!("CARGO_PKG_VERSION"));
const TRADE_HOME: &str = "https://poe.game.qq.com/trade2";
const TRADE_API_ROOT: &str = "https://poe.game.qq.com/api/trade2";
const REDACTED_SECRET: &str = "[REDACTED]";
const RELEASE_API: &str =
    "https://api.github.com/repos/zijinan/poe2-cn-price-bridge/releases/latest";
const RELEASE_PAGE: &str = "https://github.com/zijinan/poe2-cn-price-bridge/releases/latest";
const DEFAULT_PRIMARY_LEAGUE: &str = "奥杜尔秘符";
const DEFAULT_FALLBACK_LEAGUE: &str = "永久";
const REALM: &str = "poe2";
const DEFAULT_STATUS: &str = "securable";
const DEFAULT_MAX_FETCH_RESULTS: usize = 80;
const DEFAULT_FETCH_BATCH_SIZE: usize = 10;
const DEFAULT_PAGE_SIZE: usize = 8;
const DEFAULT_RESULT_TIMEOUT_SECONDS: u64 = 16;
const MAX_HISTORY_ENTRIES: usize = 80;
const CREATE_NO_WINDOW: u32 = 0x08000000;

const HOTKEY_PRICE_ID: i32 = 0x504f_4532;
const VK_D: u32 = 0x44;
const VK_F6: u32 = 0x75;
const VK_F7: u32 = 0x76;
const VK_F8: u32 = 0x77;
const VK_F9: u32 = 0x78;
const VK_F10: u32 = 0x79;
const TIMER_ID: usize = 1;
const WM_TRAYICON: u32 = WM_APP + 1;
const TRAY_UID: u32 = 1;
const IDM_SHOW: usize = 1001;
const IDM_PRICE: usize = 1002;
const IDM_WIZARD: usize = 1003;
const IDM_SETTINGS: usize = 1004;
const IDM_COOKIE: usize = 1005;
const IDM_VALIDATE_COOKIE: usize = 1006;
const IDM_HISTORY: usize = 1007;
const IDM_TRADE_HOME: usize = 1008;
const IDM_DIAGNOSTICS: usize = 1009;
const IDM_SELF_CHECK: usize = 1010;
const IDM_UPDATE_CHECK: usize = 1011;
const IDM_ABOUT: usize = 1012;
const IDM_QUIT: usize = 1013;
const IDM_TOGGLE_AUTO: usize = 1014;
// 联赛快捷切换
const IDM_LEAGUE_STANDARD: usize = 2001;
const IDM_LEAGUE_PERMANENT: usize = 2002;

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

fn copy_wide_fixed<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    let encoded = wide(text);
    let count = encoded.len().saturating_sub(1).min(N.saturating_sub(1));
    target[..count].copy_from_slice(&encoded[..count]);
}

unsafe fn notify_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_UID,
        ..Default::default()
    }
}

fn app_icon_path() -> Option<PathBuf> {
    let root = project_root_dir();
    let candidates = [
        root.join("assets").join("app.ico"),
        root.join("app.ico"),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("assets").join("app.ico")))
            .unwrap_or_default(),
    ];
    candidates.into_iter().find(|path| path.exists())
}

unsafe fn load_app_icon(size: i32) -> (HICON, bool) {
    if let Some(path) = app_icon_path() {
        let path = wide(&path.to_string_lossy());
        let handle = LoadImageW(
            null_mut(),
            path.as_ptr(),
            IMAGE_ICON,
            size,
            size,
            LR_LOADFROMFILE,
        );
        if !handle.is_null() {
            return (handle as HICON, true);
        }
    }
    (LoadIconW(null_mut(), IDI_APPLICATION), false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiButton {
    Pin,
    Close,
    Wizard,
    Cookie,
    ValidateCookie,
    Diagnostics,
    History,
    Mods,
    Values,
    ModToggle(usize),
    Prev,
    Next,
    Open,
    Copy,
    Whisper(usize),
}

struct UiButtonSpec {
    button: UiButton,
    label: String,
    rect: RECT,
    enabled: bool,
    primary: bool,
}

fn rect_contains(rect: &RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

fn last_win_error(prefix: &str) -> anyhow::Error {
    anyhow!("{prefix}: Windows error {}", unsafe { GetLastError() })
}

/// 从已保存的 Cookie 中提取 POESESSID，供运行时直接值脱敏使用。
fn known_poesessid_value(cookie: &str) -> Option<&str> {
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name.trim().eq_ignore_ascii_case("POESESSID") && !value.trim().is_empty() {
            Some(value.trim())
        } else {
            None
        }
    })
}

/// 按 ASCII 键名脱敏后续值，兼容请求头、普通键值和 JSON 键值格式。
fn redact_keyed_values(text: &str, key: &str, redact_to_line_end: bool) -> String {
    let mut output = text.to_string();
    let key_lower = key.to_ascii_lowercase();
    let mut search_from = 0;

    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative) = lower[search_from..].find(&key_lower) else {
            break;
        };
        let key_start = search_from + relative;
        let key_end = key_start + key.len();
        let bytes = output.as_bytes();
        let has_word_prefix = key_start > 0
            && (bytes[key_start - 1].is_ascii_alphanumeric() || bytes[key_start - 1] == b'_');
        if has_word_prefix {
            search_from = key_end;
            continue;
        }

        let mut cursor = key_end;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_whitespace()
                || bytes[cursor] == b'\''
                || bytes[cursor] == b'"')
        {
            cursor += 1;
        }
        if cursor >= bytes.len() || !matches!(bytes[cursor], b':' | b'=') {
            search_from = key_end;
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let quote = if cursor < bytes.len() && matches!(bytes[cursor], b'\'' | b'"') {
            let quote = bytes[cursor];
            cursor += 1;
            Some(quote)
        } else {
            None
        };
        let value_start = cursor;
        let value_end = if let Some(quote) = quote {
            bytes[value_start..]
                .iter()
                .position(|byte| *byte == quote)
                .map(|offset| value_start + offset)
                .unwrap_or(bytes.len())
        } else if redact_to_line_end {
            bytes[value_start..]
                .iter()
                .position(|byte| matches!(*byte, b'\r' | b'\n'))
                .map(|offset| value_start + offset)
                .unwrap_or(bytes.len())
        } else {
            bytes[value_start..]
                .iter()
                .position(|byte| byte.is_ascii_whitespace() || matches!(*byte, b';' | b',' | b'}'))
                .map(|offset| value_start + offset)
                .unwrap_or(bytes.len())
        };

        if value_end <= value_start {
            search_from = key_end;
            continue;
        }
        output.replace_range(value_start..value_end, REDACTED_SECRET);
        search_from = value_start + REDACTED_SECRET.len();
    }

    output
}

/// 统一脱敏 Cookie 请求头、POESESSID 键值和运行时已知的真实 Secret。
fn redact_sensitive_text(text: &str, known_cookie: Option<&str>) -> String {
    let mut output = text.to_string();
    if let Some(cookie) = known_cookie
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        output = output.replace(cookie, REDACTED_SECRET);
        if let Some(value) = known_poesessid_value(cookie) {
            output = output.replace(value, REDACTED_SECRET);
        } else if !cookie.contains('=') && !cookie.contains(';') && !cookie.contains(':') {
            output = output.replace(cookie, REDACTED_SECRET);
        }
    }
    output = redact_keyed_values(&output, "POESESSID", false);
    redact_keyed_values(&output, "Cookie", true)
}

/// 使用当前 Windows 用户已保存的 Cookie 对文本执行运行时脱敏。
fn redact_runtime_text(text: &str) -> String {
    let cookie = load_cookie().ok().flatten();
    redact_sensitive_text(text, cookie.as_deref())
}

fn log(message: impl AsRef<str>) {
    let message = redact_runtime_text(message.as_ref());
    let line = format!("[{}] {message}", chrono_like_time());
    println!("{line}");
    if let Err(err) = append_log_line(&line) {
        eprintln!("写入日志失败: {err}");
    }
}

fn chrono_like_time() -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let seconds = since_epoch.as_secs() % 86_400;
    let h = (seconds + 8 * 3600) % 86_400 / 3600;
    let m = seconds % 3600 / 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
struct WindowPos {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    #[serde(default = "default_primary_league")]
    primary_league: String,
    #[serde(default = "default_fallback_league")]
    fallback_league: String,
    #[serde(default = "default_max_fetch_results")]
    max_fetch_results: usize,
    #[serde(default = "default_fetch_batch_size")]
    fetch_batch_size: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
    #[serde(default = "default_result_timeout_seconds")]
    result_timeout_seconds: u64,
    #[serde(default = "default_auto_clipboard")]
    auto_clipboard: bool,
    #[serde(default = "default_manual_hotkey")]
    manual_hotkey: String,
}

fn default_primary_league() -> String {
    DEFAULT_PRIMARY_LEAGUE.to_string()
}

fn default_fallback_league() -> String {
    DEFAULT_FALLBACK_LEAGUE.to_string()
}

fn default_max_fetch_results() -> usize {
    DEFAULT_MAX_FETCH_RESULTS
}

fn default_fetch_batch_size() -> usize {
    DEFAULT_FETCH_BATCH_SIZE
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

fn default_result_timeout_seconds() -> u64 {
    DEFAULT_RESULT_TIMEOUT_SECONDS
}

fn default_auto_clipboard() -> bool {
    true
}

fn default_manual_hotkey() -> String {
    "F8".to_string()
}

#[derive(Debug, Clone, Copy)]
struct ManualHotkeySpec {
    label: &'static str,
    modifiers: u32,
    vk: u32,
}

fn is_manual_hotkey_disabled(value: &str) -> bool {
    let key = value.trim().replace(' ', "").to_ascii_uppercase();
    matches!(key.as_str(), "" | "OFF" | "NONE" | "DISABLED" | "关闭")
}

fn manual_hotkey_spec(value: &str) -> Option<ManualHotkeySpec> {
    if is_manual_hotkey_disabled(value) {
        return None;
    }
    let key = value.trim().replace(' ', "").to_ascii_uppercase();
    let spec = match key.as_str() {
        "F6" => ManualHotkeySpec {
            label: "F6",
            modifiers: 0,
            vk: VK_F6,
        },
        "F7" => ManualHotkeySpec {
            label: "F7",
            modifiers: 0,
            vk: VK_F7,
        },
        "F9" => ManualHotkeySpec {
            label: "F9",
            modifiers: 0,
            vk: VK_F9,
        },
        "F10" => ManualHotkeySpec {
            label: "F10",
            modifiers: 0,
            vk: VK_F10,
        },
        "CTRL+ALT+D" | "CTRL_ALT_D" | "CTRLALTD" => ManualHotkeySpec {
            label: "Ctrl+Alt+D",
            modifiers: MOD_CONTROL | MOD_ALT,
            vk: VK_D,
        },
        _ => ManualHotkeySpec {
            label: "F8",
            modifiers: 0,
            vk: VK_F8,
        },
    };
    Some(spec)
}

fn normalize_manual_hotkey(value: &str) -> String {
    manual_hotkey_spec(value)
        .map(|spec| spec.label.to_string())
        .unwrap_or_else(|| "关闭".to_string())
}

fn manual_hotkey_label(settings: &AppSettings) -> String {
    normalize_manual_hotkey(&settings.manual_hotkey)
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            primary_league: default_primary_league(),
            fallback_league: default_fallback_league(),
            max_fetch_results: default_max_fetch_results(),
            fetch_batch_size: default_fetch_batch_size(),
            page_size: default_page_size(),
            result_timeout_seconds: default_result_timeout_seconds(),
            auto_clipboard: default_auto_clipboard(),
            manual_hotkey: default_manual_hotkey(),
        }
    }
}

impl AppSettings {
    fn normalized(mut self) -> Self {
        self.primary_league = self.primary_league.trim().to_string();
        self.fallback_league = self.fallback_league.trim().to_string();
        if self.primary_league.is_empty() {
            self.primary_league = default_primary_league();
        }
        if self.fallback_league.is_empty() {
            self.fallback_league = default_fallback_league();
        }
        self.max_fetch_results = self.max_fetch_results.clamp(8, 100);
        self.fetch_batch_size = self.fetch_batch_size.clamp(1, 10);
        self.page_size = self.page_size.clamp(4, 12);
        self.result_timeout_seconds = self.result_timeout_seconds.clamp(5, 90);
        self.manual_hotkey = normalize_manual_hotkey(&self.manual_hotkey);
        self
    }

    fn leagues(&self) -> Vec<String> {
        let mut leagues = vec![self.primary_league.clone()];
        if self.fallback_league != self.primary_league {
            leagues.push(self.fallback_league.clone());
        }
        leagues
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    cookie_dpapi: Option<String>,
    #[serde(default)]
    cookie_hint: Option<String>,
    #[serde(default)]
    overlay_pos: Option<WindowPos>,
    #[serde(default)]
    settings: AppSettings,
    #[serde(default)]
    last_cookie_check_unix: Option<u64>,
    #[serde(default)]
    last_cookie_check_status: Option<String>,
}

fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("poe2_cn_price_bridge")
}

fn config_file() -> PathBuf {
    app_dir().join("config.json")
}

fn logs_dir() -> PathBuf {
    app_dir().join("logs")
}

fn crashes_dir() -> PathBuf {
    app_dir().join("crashes")
}

fn log_file() -> PathBuf {
    logs_dir().join("app.log")
}

fn history_file() -> PathBuf {
    app_dir().join("history.jsonl")
}

fn append_log_line(line: &str) -> Result<()> {
    fs::create_dir_all(logs_dir())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file())?;
    writeln!(file, "{line}")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    ts: u64,
    status: String,
    item: String,
    base_type: String,
    rarity: String,
    league: Option<String>,
    total: Option<usize>,
    priced: Option<String>,
    message: Option<String>,
    url: Option<String>,
    used_mods: bool,
    used_values: bool,
}

fn load_history() -> Vec<HistoryEntry> {
    let Ok(text) = fs::read_to_string(history_file()) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<HistoryEntry>(line).ok())
        .collect()
}

fn save_history(entries: &[HistoryEntry]) -> Result<()> {
    fs::create_dir_all(app_dir())?;
    let mut text = String::new();
    for entry in entries.iter().rev().take(MAX_HISTORY_ENTRIES).rev() {
        text.push_str(&redact_runtime_text(&serde_json::to_string(entry)?));
        text.push('\n');
    }
    fs::write(history_file(), text)?;
    Ok(())
}

fn append_history(entry: HistoryEntry) {
    let mut entries = load_history();
    entries.push(entry);
    if entries.len() > MAX_HISTORY_ENTRIES {
        let keep_from = entries.len() - MAX_HISTORY_ENTRIES;
        entries = entries.split_off(keep_from);
    }
    if let Err(err) = save_history(&entries) {
        log(format!("保存查询历史失败: {err}"));
    }
}

fn history_preview(max_entries: usize) -> String {
    let entries = load_history();
    if entries.is_empty() {
        return "暂无查询历史。".to_string();
    }
    entries
        .iter()
        .rev()
        .take(max_entries)
        .map(|entry| {
            let item = if entry.item.is_empty() {
                entry.base_type.as_str()
            } else {
                entry.item.as_str()
            };
            let detail = entry
                .priced
                .as_deref()
                .or(entry.message.as_deref())
                .unwrap_or("");
            format!("{} | {} | {} | {}", entry.ts, entry.status, item, detail)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn support_hint() -> &'static str {
    "请在托盘右键选择“设置 Cookie”，或运行发布包里的 SetCookie.bat。"
}

fn diagnostics_path() -> PathBuf {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    app_dir().join(format!("diagnostics-{unix}.txt"))
}

fn self_check_path() -> PathBuf {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    app_dir().join(format!("selfcheck-{unix}.txt"))
}

fn update_check_path() -> PathBuf {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    app_dir().join(format!("update-check-{unix}.txt"))
}

fn crash_report_path() -> PathBuf {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    crashes_dir().join(format!("crash-{unix}.txt"))
}

fn tail_log(max_lines: usize) -> String {
    let Ok(text) = fs::read_to_string(log_file()) else {
        return "日志文件不存在。".to_string();
    };
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn write_diagnostics(target: Option<PathBuf>) -> Result<PathBuf> {
    fs::create_dir_all(app_dir())?;
    let path = target.unwrap_or_else(diagnostics_path);
    let config = load_config();
    let settings = config.settings.clone().normalized();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("(unknown)"));
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("(unknown)"));
    let root = project_root_dir();
    let icon_path = app_icon_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(not found)".to_string());
    let content = redact_runtime_text(&format!(
        "{APP_DISPLAY_NAME} diagnostics\n\
         version: {APP_VERSION}\n\
         exe: {}\n\
         cwd: {}\n\
         app_dir: {}\n\
         config_file: {}\n\
         log_file: {}\n\
         project_root: {}\n\
         icon: {}\n\
         cookie_saved: {}\n\
         cookie_hint: {}\n\
         last_cookie_check_unix: {}\n\
         last_cookie_check_status: {}\n\
         overlay_pos: {:?}\n\
         primary_league: {}\n\
         fallback_league: {}\n\
         max_fetch_results: {}\n\
         fetch_batch_size: {}\n\
         page_size: {}\n\
         result_timeout_seconds: {}\n\
         auto_clipboard: {}\n\
         manual_hotkey: {}\n\
         history_file: {}\n\
         \n--- recent history ---\n{}\n\
         \n--- recent log ---\n{}\n",
        exe.display(),
        cwd.display(),
        app_dir().display(),
        config_file().display(),
        log_file().display(),
        root.display(),
        icon_path,
        config.cookie_dpapi.is_some(),
        config.cookie_hint.unwrap_or_else(|| "(none)".to_string()),
        config
            .last_cookie_check_unix
            .map(|value| value.to_string())
            .unwrap_or_else(|| "(never)".to_string()),
        config
            .last_cookie_check_status
            .unwrap_or_else(|| "(never)".to_string()),
        config.overlay_pos,
        settings.primary_league,
        settings.fallback_league,
        settings.max_fetch_results,
        settings.fetch_batch_size,
        settings.page_size,
        settings.result_timeout_seconds,
        settings.auto_clipboard,
        manual_hotkey_label(&settings),
        history_file().display(),
        history_preview(30),
        tail_log(160)
    ));
    fs::write(&path, content)?;
    log(format!("已导出诊断: {}", path.display()));
    Ok(path)
}

fn check_file(root: &std::path::Path, relative: &str) -> String {
    let path = root.join(relative);
    if path.exists() {
        format!("{relative}: ok")
    } else {
        format!("{relative}: missing")
    }
}

fn check_trade_home_network() -> String {
    let client = match Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(err) => return format!("failed: 创建 HTTP 客户端失败: {err}"),
    };
    match client.get(TRADE_HOME).send() {
        Ok(response) => format!("ok: HTTP {}", response.status()),
        Err(err) => format!("failed: {err}"),
    }
}

#[derive(Debug)]
struct UpdateInfo {
    latest_tag: String,
    latest_name: String,
    release_url: String,
    assets: Vec<String>,
}

fn version_numbers(value: &str) -> Vec<u64> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            numbers.push(current.parse::<u64>().unwrap_or(0));
            current.clear();
        }
    }
    if !current.is_empty() {
        numbers.push(current.parse::<u64>().unwrap_or(0));
    }
    while numbers.len() < 3 {
        numbers.push(0);
    }
    numbers.truncate(3);
    numbers
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    version_numbers(latest) > version_numbers(current)
}

fn latest_release_info() -> Result<UpdateInfo> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(USER_AGENT)
        .build()
        .context("创建 HTTP 客户端失败")?;
    let data = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .context("访问 GitHub Release API 失败")?;
    let status = data.status();
    let body = data.text().context("读取 GitHub 返回失败")?;
    if !status.is_success() {
        bail!("GitHub 返回 HTTP {}: {}", status.as_u16(), body);
    }
    let data: Value = serde_json::from_str(&body).context("GitHub 返回不是 JSON")?;
    let latest_tag = data
        .get("tag_name")
        .and_then(Value::as_str)
        .filter(|tag| !tag.trim().is_empty())
        .unwrap_or("unknown")
        .to_string();
    let latest_name = data
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&latest_tag)
        .to_string();
    let release_url = data
        .get("html_url")
        .and_then(Value::as_str)
        .unwrap_or(RELEASE_PAGE)
        .to_string();
    let assets = data
        .get("assets")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(UpdateInfo {
        latest_tag,
        latest_name,
        release_url,
        assets,
    })
}

fn update_check_report() -> String {
    match latest_release_info() {
        Ok(info) => {
            let status = if is_newer_version(&info.latest_tag, APP_VERSION) {
                "update_available"
            } else {
                "up_to_date"
            };
            let asset_text = if info.assets.is_empty() {
                "(no assets listed)".to_string()
            } else {
                info.assets.join("\n")
            };
            format!(
                "{APP_DISPLAY_NAME} update check\n\
                 status: {status}\n\
                 current_version: v{APP_VERSION}\n\
                 latest_tag: {}\n\
                 latest_name: {}\n\
                 release_url: {}\n\
                 \n--- assets ---\n{}\n\
                 \n说明: 这是手动检查更新，不会自动下载或自动替换本机文件。\n",
                info.latest_tag, info.latest_name, info.release_url, asset_text
            )
        }
        Err(err) => format!(
            "{APP_DISPLAY_NAME} update check\n\
             status: failed\n\
             current_version: v{APP_VERSION}\n\
             release_url: {RELEASE_PAGE}\n\
             error: {err:#}\n\
             \n说明: GitHub API 偶尔会因为网络、TLS 握手、地区链路或限流失败；普通客户检查更新不需要授权。可以稍后重试，或直接打开 release_url 查看。\n"
        ),
    }
}

fn write_update_check(target: Option<PathBuf>) -> Result<PathBuf> {
    fs::create_dir_all(app_dir())?;
    let path = target.unwrap_or_else(update_check_path);
    fs::write(&path, update_check_report())?;
    log(format!("已导出更新检查: {}", path.display()));
    Ok(path)
}

fn cookie_self_check(cookie_saved: bool) -> String {
    if !cookie_saved {
        return "missing: 尚未保存 Cookie".to_string();
    }
    match validate_saved_cookie() {
        Ok(_) => "ok: Cookie 可用于国服 trade2".to_string(),
        Err(err) => format!("failed: {err}"),
    }
}

fn write_self_check(target: Option<PathBuf>) -> Result<PathBuf> {
    fs::create_dir_all(app_dir())?;
    let path = target.unwrap_or_else(self_check_path);
    let config = load_config();
    let settings = config.settings.clone().normalized();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("(unknown)"));
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("(unknown)"));
    let root = project_root_dir();
    let cookie_saved = config.cookie_dpapi.is_some();
    let package_files = [
        "POE2PriceHelper.exe",
        "StartHere.bat",
        "开始使用.bat",
        "ControlCenter.bat",
        "control_center.ps1",
        "Start.bat",
        "启动查价.bat",
        "FirstRun.bat",
        "首次向导.bat",
        "SetCookie.bat",
        "设置Cookie.bat",
        "ValidateCookie.bat",
        "Settings.bat",
        "常用设置.bat",
        "History.bat",
        "查询历史.bat",
        "SelfCheck.bat",
        "运行自检.bat",
        "CheckUpdate.bat",
        "检查更新.bat",
        "Diagnostics.bat",
        "SupportBundle.bat",
        "生成支持包.bat",
        "SupportBundle.ps1",
        "ResetData.bat",
        "ResetData.ps1",
        "Uninstall.bat",
        "Uninstall.ps1",
        "InstallShortcut.bat",
        "InstallShortcut.ps1",
        "ClearCookie.bat",
        "first_run_wizard.ps1",
        "set_cookie_gui.ps1",
        "settings_gui.ps1",
        "history_gui.ps1",
        "assets\\app.ico",
        "assets\\app_256.png",
        "README.md",
        "CHANGELOG.md",
        "SUPPORT.md",
        "VERSION.txt",
        "LICENSE",
    ]
    .iter()
    .map(|relative| check_file(&root, relative))
    .collect::<Vec<_>>()
    .join("\n");
    let content = redact_runtime_text(&format!(
        "{APP_DISPLAY_NAME} self-check\n\
         version: {APP_VERSION}\n\
         generated_at_unix: {}\n\
         exe: {}\n\
         cwd: {}\n\
         app_dir: {}\n\
         config_file: {}\n\
         log_file: {}\n\
         crash_dir: {}\n\
         project_root: {}\n\
         cookie_saved: {}\n\
         cookie_validation: {}\n\
         network_trade_home: {}\n\
         primary_league: {}\n\
         fallback_league: {}\n\
         max_fetch_results: {}\n\
         fetch_batch_size: {}\n\
         page_size: {}\n\
         result_timeout_seconds: {}\n\
         auto_clipboard: {}\n\
         manual_hotkey: {}\n\
         \n--- package files ---\n{}\n\
         \n--- recent log ---\n{}\n",
        unix_now(),
        exe.display(),
        cwd.display(),
        app_dir().display(),
        config_file().display(),
        log_file().display(),
        crashes_dir().display(),
        root.display(),
        cookie_saved,
        cookie_self_check(cookie_saved),
        check_trade_home_network(),
        settings.primary_league,
        settings.fallback_league,
        settings.max_fetch_results,
        settings.fetch_batch_size,
        settings.page_size,
        settings.result_timeout_seconds,
        settings.auto_clipboard,
        manual_hotkey_label(&settings),
        package_files,
        tail_log(80)
    ));
    fs::write(&path, content)?;
    log(format!("已导出自检报告: {}", path.display()));
    Ok(path)
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(value) = info.payload().downcast_ref::<&str>() {
            (*value).to_string()
        } else if let Some(value) = info.payload().downcast_ref::<String>() {
            value.clone()
        } else {
            "未知 panic".to_string()
        };
        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "(unknown)".to_string());
        let _ = fs::create_dir_all(crashes_dir());
        let path = crash_report_path();
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("(unknown)"));
        let payload = redact_runtime_text(&payload);
        let content = format!(
            "{APP_DISPLAY_NAME} crash report\n\
             version: {APP_VERSION}\n\
             time_unix: {}\n\
             exe: {}\n\
             location: {}\n\
             panic: {}\n",
            unix_now(),
            exe.display(),
            location,
            payload
        );
        let _ = fs::write(&path, content);
        let _ = append_log_line(&format!(
            "[{}] 崩溃: {} | {}",
            chrono_like_time(),
            payload,
            path.display()
        ));
    }));
}

fn load_config() -> Config {
    let path = config_file();
    let Ok(text) = fs::read_to_string(path) else {
        return Config::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_config(config: &Config) -> Result<()> {
    fs::create_dir_all(app_dir())?;
    fs::write(config_file(), serde_json::to_string_pretty(config)?)?;
    Ok(())
}

fn protect_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    let ok = unsafe {
        CryptProtectData(
            &input,
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        bail!(last_win_error("CryptProtectData failed"));
    }

    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(bytes)
}

fn unprotect_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    let ok = unsafe {
        CryptUnprotectData(
            &input,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        bail!(last_win_error("CryptUnprotectData failed"));
    }

    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as _);
    }
    Ok(bytes)
}

fn normalize_cookie_input(raw: &str) -> String {
    let mut value = raw.trim().to_string();
    if value.to_ascii_lowercase().starts_with("cookie:") {
        value = value
            .split_once(':')
            .map(|(_, right)| right.trim().to_string())
            .unwrap_or_default();
    }
    value = value.lines().map(str::trim).collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        return value;
    }
    if !value.contains('=') && !value.contains(';') {
        format!("POESESSID={value}")
    } else {
        value
    }
}

fn save_cookie(cookie: &str) -> Result<()> {
    let cookie = normalize_cookie_input(cookie);
    if cookie.is_empty() {
        bail!("Cookie 为空");
    }
    let encrypted = protect_bytes(cookie.as_bytes())?;
    let mut config = load_config();
    config.cookie_dpapi = Some(B64.encode(encrypted));
    config.cookie_hint = Some("Windows DPAPI encrypted; same Windows user only".to_string());
    save_config(&config)
}

fn load_cookie() -> Result<Option<String>> {
    let config = load_config();
    let Some(encrypted) = config.cookie_dpapi else {
        return Ok(None);
    };
    let bytes = B64.decode(encrypted)?;
    let plain = unprotect_bytes(&bytes)?;
    Ok(Some(String::from_utf8(plain)?))
}

fn clear_cookie() -> Result<()> {
    let mut config = load_config();
    config.cookie_dpapi = None;
    config.cookie_hint = None;
    config.last_cookie_check_unix = Some(unix_now());
    config.last_cookie_check_status = Some("cleared".to_string());
    save_config(&config)
}

fn update_cookie_validation_status(status: &str) {
    let mut config = load_config();
    config.last_cookie_check_unix = Some(unix_now());
    config.last_cookie_check_status = Some(status.to_string());
    if let Err(err) = save_config(&config) {
        log(format!("保存 Cookie 验证状态失败: {err}"));
    }
}

fn set_cookie_interactive() -> Result<()> {
    println!("输入国服 poe.game.qq.com 的 POESESSID，或完整 Cookie 请求头。");
    println!("输入不会显示；会用 Windows DPAPI 加密保存到当前 Windows 用户。");
    let raw = prompt_password("POESESSID/Cookie: ")?;
    if raw.trim().is_empty() {
        bail!("未输入，已取消");
    }
    save_cookie(&raw)?;
    println!("已保存到 {}", config_file().display());
    Ok(())
}

fn set_cookie_from_file(path: &str) -> Result<()> {
    let raw = fs::read_to_string(path).with_context(|| format!("读取 Cookie 文件失败: {path}"))?;
    if raw.trim().is_empty() {
        bail!("Cookie 为空");
    }
    save_cookie(&raw)
}

const MAX_STDIN_POESESSID_BYTES: usize = 4096;
const STDIN_POESESSID_TIMEOUT: Duration = Duration::from_secs(5);

/// 解析并复用登录助手传入的裸 POESESSID 缓冲区，拒绝请求头和控制字符注入。
fn parse_poesessid_stdin(mut value: String) -> Result<String> {
    let leading = value.len() - value.trim_start().len();
    if leading != 0 {
        value.drain(..leading);
    }
    value.truncate(value.trim_end().len());
    if value.is_empty() {
        bail!("标准输入中的 POESESSID 为空");
    }
    if value.len() > MAX_STDIN_POESESSID_BYTES {
        bail!("标准输入中的 POESESSID 过长");
    }
    if value
        .get(.."cookie:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("cookie:"))
        || value
            .as_bytes()
            .windows("poesessid=".len())
            .any(|window| window.eq_ignore_ascii_case(b"poesessid="))
    {
        bail!("标准输入只接受裸 POESESSID 值");
    }
    if !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b';' | b',' | b'"' | b'\\'))
    {
        bail!("标准输入中的 POESESSID 包含不允许的字符");
    }
    Ok(value)
}

/// 从任意读取器最多读取 4097 字节；超长或管道错误不会回显已读内容。
fn read_poesessid_from_reader(reader: &mut impl Read) -> Result<String> {
    let mut value = String::new();
    reader
        .take((MAX_STDIN_POESESSID_BYTES + 1) as u64)
        .read_to_string(&mut value)
        .context("读取标准输入失败")?;
    parse_poesessid_stdin(value)
}

/// 在专用线程执行可能阻塞的读取，主线程超过期限后立即安全失败。
fn receive_poesessid_with_timeout<F>(read: F, timeout: Duration) -> Result<String>
where
    F: FnOnce() -> Result<String> + Send + 'static,
{
    let (result_tx, result_rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let _ = result_tx.send(read());
    });
    match result_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => bail!("读取标准输入超时"),
        Err(mpsc::RecvTimeoutError::Disconnected) => bail!("标准输入读取线程意外中断"),
    }
}

/// 从标准输入接收候选值，验证通过后才覆盖 DPAPI 存储。
fn set_cookie_from_stdin() -> Result<()> {
    let mut cookie = receive_poesessid_with_timeout(
        || {
            let mut stdin = io::stdin();
            read_poesessid_from_reader(&mut stdin)
        },
        STDIN_POESESSID_TIMEOUT,
    )?;
    cookie.insert_str(0, "POESESSID=");
    validate_cookie_against_cn(&cookie)
        .map_err(|error| anyhow!(redact_sensitive_text(&error.to_string(), Some(&cookie))))?;
    save_cookie(&cookie)?;
    update_cookie_validation_status("ok");
    Ok(())
}
fn cookie_validation_payload() -> Value {
    json!({
        "query": {
            "status": { "option": DEFAULT_STATUS },
            "stats": [{ "type": "and", "filters": [] }],
            "filters": {}
        },
        "sort": { "price": "asc" }
    })
}

/// 仅使用候选 Cookie 访问国服 trade2；验证失败时不会修改现有认证存储。
fn validate_cookie_against_cn(cookie: &str) -> Result<(), TradeError> {
    let settings = load_config().settings.normalized();
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| TradeError::Request(format!("创建 HTTP 客户端失败: {err}")))?;
    let payload = cookie_validation_payload();
    let mut failures = Vec::new();

    for league in settings.leagues() {
        match request_json_with_cookie(
            &client,
            &trade_search_url(&league),
            Method::POST,
            Some(&payload),
            cookie,
        ) {
            Ok(data) => {
                if data.get("error").unwrap_or(&Value::Null).is_null() {
                    return Ok(());
                }
                let message = data
                    .get("error")
                    .and_then(|err| err.get("message"))
                    .and_then(Value::as_str)
                    .or_else(|| data.get("error").and_then(Value::as_str))
                    .unwrap_or("trade validation error");
                failures.push(format!("{league}: {message}"));
            }
            Err(TradeError::Auth(message)) => return Err(TradeError::Auth(message)),
            Err(TradeError::Request(message)) => failures.push(format!("{league}: {message}")),
        }
    }

    let message = if failures.is_empty() {
        "验证请求没有返回可用结果".to_string()
    } else {
        failures.join(" / ")
    };
    Err(TradeError::Request(message))
}

fn validate_saved_cookie() -> Result<()> {
    let settings = load_config().settings.normalized();
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("创建 HTTP 客户端失败")?;
    let payload = cookie_validation_payload();
    let mut failures = Vec::new();

    for league in settings.leagues() {
        match request_json(
            &client,
            &trade_search_url(&league),
            Method::POST,
            Some(&payload),
        ) {
            Ok(data) => {
                if data.get("error").unwrap_or(&Value::Null).is_null() {
                    update_cookie_validation_status("ok");
                    log(format!("Cookie 验证通过: {league}"));
                    return Ok(());
                }
                let message = data
                    .get("error")
                    .and_then(|err| err.get("message"))
                    .and_then(Value::as_str)
                    .or_else(|| data.get("error").and_then(Value::as_str))
                    .unwrap_or("trade validation error");
                failures.push(format!("{league}: {message}"));
            }
            Err(TradeError::Auth(message)) => {
                update_cookie_validation_status("auth_failed");
                bail!(message);
            }
            Err(TradeError::Request(message)) => {
                failures.push(format!("{league}: {message}"));
            }
        }
    }

    let message = if failures.is_empty() {
        "验证请求没有返回可用结果".to_string()
    } else {
        failures.join(" / ")
    };
    update_cookie_validation_status("request_failed");
    bail!("Cookie 已保存，但验证请求失败: {message}")
}

#[derive(Debug, Clone, Serialize)]
struct ParsedItem {
    rarity_raw: String,
    rarity: String,
    name: String,
    base_type: String,
    item_class: String,
    item_level: Option<u32>,
    query_mode: String,
    display: String,
    mods: Vec<ParsedMod>,
}

#[derive(Debug, Clone, Serialize)]
struct ParsedMod {
    text: String,
    pattern: String,
    value: Option<f64>,
    stat_id: Option<String>,
    stat_text: Option<String>,
}

impl Default for ParsedItem {
    fn default() -> Self {
        Self {
            rarity_raw: String::new(),
            rarity: String::new(),
            name: String::new(),
            base_type: String::new(),
            item_class: String::new(),
            item_level: None,
            query_mode: "type".to_string(),
            display: String::new(),
            mods: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct TradeEntry {
    price: String,
    seller: String,
    item_name: String,
}

#[derive(Debug, Clone, Default)]
struct QueryOptions {
    use_mods: bool,
    use_values: bool,
    selected_mod_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct TradeResult {
    item: ParsedItem,
    league: String,
    total: usize,
    entries: Vec<TradeEntry>,
    summary: Vec<String>,
    url: String,
    options: QueryOptions,
    page_size: usize,
}

#[derive(Debug)]
enum TradeError {
    Auth(String),
    Request(String),
}

impl fmt::Display for TradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeError::Auth(message) | TradeError::Request(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for TradeError {}

fn normalize_line(line: &str) -> String {
    line.replace('：', ":").trim().to_string()
}

fn is_separator(line: &str) -> bool {
    !line.trim().is_empty() && line.trim().chars().all(|c| c == '-')
}

fn value_after_colon(line: &str) -> String {
    line.split_once(':')
        .map(|(_, right)| right.trim().to_string())
        .unwrap_or_default()
}

fn first_number(text: &str) -> Option<f64> {
    let mut current = String::new();
    let mut seen_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit()
            || (ch == '.' && seen_digit)
            || ((ch == '+' || ch == '-') && current.is_empty())
        {
            if ch.is_ascii_digit() {
                seen_digit = true;
            }
            current.push(ch);
            continue;
        }
        if seen_digit {
            return current.parse::<f64>().ok();
        }
        current.clear();
    }
    if seen_digit {
        current.parse::<f64>().ok()
    } else {
        None
    }
}

fn stat_pattern(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        let sign_number = (ch == '+' || ch == '-')
            && chars
                .peek()
                .map(|next| next.is_ascii_digit())
                .unwrap_or(false);
        if ch.is_ascii_digit() || sign_number {
            while let Some(next) = chars.peek() {
                if next.is_ascii_digit() || *next == '.' {
                    chars.next();
                } else {
                    break;
                }
            }
            out.push('#');
        } else {
            out.push(ch);
        }
    }
    out
}

fn compact_stat_pattern(text: &str) -> String {
    stat_pattern(text)
        .replace('＋', "+")
        .replace('－', "-")
        .replace("+#", "#")
        .replace("-#", "#")
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '[' | ']' | '(' | ')' | '（' | '）' | '，' | ',' | ':' | '：'
                )
        })
        .collect::<String>()
        .to_lowercase()
}

fn is_probable_mod_line(line: &str) -> bool {
    if !line.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if line.contains(':') {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    let blocked = [
        "item level",
        "requires",
        "需求",
        "需要",
        "物品等级",
        "品质",
        "quality",
        "已鉴定",
        "已污染",
        "已複製",
        "已复制",
        "堆叠数量",
        "stack size",
        "sockets",
    ];
    !blocked
        .iter()
        .any(|prefix| lower.starts_with(prefix) || line.starts_with(prefix))
}

fn rarity_to_trade(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "普通" | "normal" => "normal",
        "魔法" | "magic" => "magic",
        "稀有" | "rare" => "rare",
        "传奇" | "傳奇" | "unique" => "unique",
        _ => "",
    }
    .to_string()
}

fn parse_item_text(text: &str) -> ParsedItem {
    let lines: Vec<String> = text
        .replace("\r\n", "\n")
        .lines()
        .map(normalize_line)
        .filter(|line| !line.is_empty() && !is_separator(line))
        .collect();

    let mut parsed = ParsedItem::default();
    if lines.is_empty() {
        return parsed;
    }

    let mut rarity_index = None;
    for (index, line) in lines.iter().enumerate() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("rarity:")
            || line.starts_with("稀有度:")
            || line.starts_with("稀 有 度:")
        {
            parsed.rarity_raw = value_after_colon(line);
            parsed.rarity = rarity_to_trade(&parsed.rarity_raw);
            rarity_index = Some(index);
        } else if lower.starts_with("item class:")
            || line.starts_with("物品类别:")
            || line.starts_with("物品類別:")
        {
            parsed.item_class = value_after_colon(line);
        } else if lower.starts_with("item level:")
            || line.starts_with("物品等级:")
            || line.starts_with("物品等級:")
        {
            parsed.item_level = line
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok();
        }
    }

    let mut candidates = Vec::new();
    if let Some(index) = rarity_index {
        for line in lines.iter().skip(index + 1) {
            let lower = line.to_ascii_lowercase();
            if line.contains(':') {
                break;
            }
            if matches!(lower.as_str(), "identified" | "unidentified" | "corrupted") {
                continue;
            }
            if matches!(
                line.as_str(),
                "已鉴定" | "未鉴定" | "已污染" | "已複製" | "已复制"
            ) {
                continue;
            }
            candidates.push(line.clone());
            if candidates.len() >= 2 {
                break;
            }
        }
    } else {
        candidates = lines
            .iter()
            .filter(|line| !line.contains(':'))
            .take(2)
            .cloned()
            .collect();
    }

    if parsed.rarity == "unique" {
        parsed.name = candidates.first().cloned().unwrap_or_default();
        parsed.base_type = candidates.get(1).cloned().unwrap_or_default();
        parsed.query_mode = "name".to_string();
    } else if parsed.rarity == "rare" || parsed.rarity == "magic" {
        parsed.name = candidates.first().cloned().unwrap_or_default();
        parsed.base_type = candidates
            .get(1)
            .cloned()
            .unwrap_or_else(|| parsed.name.clone());
        parsed.query_mode = "type".to_string();
    } else {
        parsed.base_type = candidates.last().cloned().unwrap_or_default();
        parsed.name = candidates.first().cloned().unwrap_or_default();
        parsed.query_mode = "type".to_string();
    }

    let mut seen_mods: HashMap<String, bool> = HashMap::new();
    for line in &lines {
        if !is_probable_mod_line(line) {
            continue;
        }
        let pattern = compact_stat_pattern(line);
        if pattern.is_empty() || seen_mods.insert(pattern.clone(), true).is_some() {
            continue;
        }
        parsed.mods.push(ParsedMod {
            text: line.clone(),
            pattern,
            value: first_number(line),
            stat_id: None,
            stat_text: None,
        });
        if parsed.mods.len() >= 8 {
            break;
        }
    }

    parsed.display = if !parsed.name.is_empty() {
        parsed.name.clone()
    } else if !parsed.base_type.is_empty() {
        parsed.base_type.clone()
    } else {
        "(unparsed item)".to_string()
    };
    parsed
}

fn build_trade_payload(
    parsed: &ParsedItem,
    options: &QueryOptions,
    status: &str,
    use_name: bool,
    use_type: bool,
    use_rarity: bool,
) -> Value {
    let mut query = json!({
        "status": { "option": status },
        "stats": [{ "type": "and", "filters": [] }],
        "filters": {}
    });

    if parsed.query_mode == "name" && !parsed.name.is_empty() && use_name {
        query["name"] = json!(parsed.name);
        if !parsed.base_type.is_empty() && use_type {
            query["type"] = json!(parsed.base_type);
        }
    } else if !parsed.base_type.is_empty() && use_type {
        query["type"] = json!(parsed.base_type);
    } else if !parsed.name.is_empty() && use_name {
        query["type"] = json!(parsed.name);
    }

    if !parsed.rarity.is_empty() && use_rarity {
        query["filters"] = json!({
            "type_filters": {
                "filters": {
                    "rarity": { "option": parsed.rarity }
                }
            }
        });
    }

    if options.use_mods {
        let filters: Vec<Value> = parsed
            .mods
            .iter()
            .filter_map(|item_mod| {
                if let Some(selected_patterns) = &options.selected_mod_patterns
                    && !selected_patterns
                        .iter()
                        .any(|pattern| pattern == &item_mod.pattern)
                {
                    return None;
                }
                let id = item_mod.stat_id.as_ref()?;
                let mut filter = json!({ "id": id });
                if options.use_values
                    && let Some(value) = item_mod.value
                {
                    filter["value"] = json!({ "min": value });
                }
                Some(filter)
            })
            .collect();
        if !filters.is_empty() {
            query["stats"] = json!([{ "type": "and", "filters": filters }]);
        }
    }

    json!({
        "query": query,
        "sort": { "price": "asc" }
    })
}

fn build_payload_variants(parsed: &ParsedItem, options: &QueryOptions) -> Vec<Value> {
    let candidates = vec![
        build_trade_payload(parsed, options, DEFAULT_STATUS, true, true, true),
        build_trade_payload(parsed, options, DEFAULT_STATUS, true, true, false),
        build_trade_payload(parsed, options, DEFAULT_STATUS, false, true, false),
        build_trade_payload(parsed, options, DEFAULT_STATUS, true, false, false),
        build_trade_payload(parsed, options, "online", true, true, true),
        build_trade_payload(parsed, options, "online", true, true, false),
    ];
    let mut seen = HashMap::new();
    let mut variants = Vec::new();
    for candidate in candidates {
        let key = serde_json::to_string(&candidate).unwrap_or_default();
        if seen.insert(key, true).is_none() {
            variants.push(candidate);
        }
    }
    variants
}

fn trade_search_url(league: &str) -> String {
    format!(
        "{TRADE_API_ROOT}/search/{}/{}",
        REALM,
        urlencoding::encode(league)
    )
}

fn trade_result_url(league: &str, query_id: &str) -> String {
    format!(
        "https://poe.game.qq.com/trade2/search/{}/{}/{}",
        REALM,
        urlencoding::encode(league),
        query_id
    )
}

fn fetch_url(ids: &[String], query_id: &str) -> String {
    format!(
        "{TRADE_API_ROOT}/fetch/{}?query={}",
        ids.join(","),
        urlencoding::encode(query_id)
    )
}

/// 返回国服 trade2 属性库地址。
fn trade_stats_url() -> String {
    format!("{TRADE_API_ROOT}/data/stats")
}

/// 判断地址是否严格属于允许访问的国服 trade2 页面或 API。
fn is_allowed_trade_endpoint(url: &str) -> bool {
    url == TRADE_HOME || url == TRADE_API_ROOT || url.starts_with(&format!("{TRADE_API_ROOT}/"))
}

#[derive(Debug, Clone)]
struct StatDef {
    id: String,
    text: String,
    pattern: String,
    stat_type: String,
}

static STAT_DEFS: OnceLock<Vec<StatDef>> = OnceLock::new();

fn load_stat_defs() -> Result<&'static Vec<StatDef>, TradeError> {
    if let Some(defs) = STAT_DEFS.get() {
        return Ok(defs);
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| TradeError::Request(format!("创建 HTTP 客户端失败: {err}")))?;
    let stats_url = trade_stats_url();
    if !is_allowed_trade_endpoint(&stats_url) {
        return Err(TradeError::Request(
            "已阻止非国服 trade2 属性库地址".to_string(),
        ));
    }
    let data = client
        .get(&stats_url)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .send()
        .map_err(|err| TradeError::Request(format!("读取属性库失败: {err}")))?
        .json::<Value>()
        .map_err(|err| TradeError::Request(format!("解析属性库失败: {err}")))?;
    let mut defs = Vec::new();
    if let Some(groups) = data.get("result").and_then(Value::as_array) {
        for group in groups {
            if let Some(entries) = group.get("entries").and_then(Value::as_array) {
                for entry in entries {
                    let id = entry.get("id").and_then(Value::as_str).unwrap_or_default();
                    let text = entry
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if id.is_empty() || text.is_empty() {
                        continue;
                    }
                    defs.push(StatDef {
                        id: id.to_string(),
                        text: text.to_string(),
                        pattern: compact_stat_pattern(text),
                        stat_type: entry
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    });
                }
            }
        }
    }
    let _ = STAT_DEFS.set(defs);
    Ok(STAT_DEFS.get().expect("stat defs set"))
}

fn resolve_item_mods(parsed: &mut ParsedItem) -> Result<(), TradeError> {
    if parsed.mods.is_empty() {
        return Ok(());
    }
    let defs = load_stat_defs()?;
    for item_mod in &mut parsed.mods {
        if item_mod.stat_id.is_some() {
            continue;
        }
        let mut best: Option<&StatDef> = None;
        for preferred in ["explicit", "pseudo", "implicit", "enchant"] {
            if let Some(found) = defs
                .iter()
                .find(|def| def.pattern == item_mod.pattern && def.stat_type == preferred)
            {
                best = Some(found);
                break;
            }
        }
        if best.is_none() {
            best = defs.iter().find(|def| def.pattern == item_mod.pattern);
        }
        if let Some(def) = best {
            item_mod.stat_id = Some(def.id.clone());
            item_mod.stat_text = Some(def.text.clone());
        }
    }
    Ok(())
}

fn request_json(
    client: &Client,
    url: &str,
    method: Method,
    payload: Option<&Value>,
) -> Result<Value, TradeError> {
    if !is_allowed_trade_endpoint(url) {
        return Err(TradeError::Request(
            "已阻止非国服 trade2 请求地址".to_string(),
        ));
    }
    let cookie = load_cookie()
        .map_err(|err| TradeError::Auth(format!("读取 POESESSID 失败: {err}")))?
        .ok_or_else(|| TradeError::Auth(format!("没有保存 POESESSID。{}", support_hint())))?;

    let mut req = client
        .request(method, url)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("Cookie", cookie)
        .header("Referer", TRADE_HOME)
        .header("Origin", "https://poe.game.qq.com");

    if let Some(payload) = payload {
        req = req.json(payload);
    }

    let resp = req
        .send()
        .map_err(|err| TradeError::Request(format!("网络请求失败: {err}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .map_err(|err| TradeError::Request(format!("读取接口返回失败: {err}")))?;

    if !status.is_success() {
        let mut message = body.clone();
        if let Ok(data) = serde_json::from_str::<Value>(&body) {
            if let Some(text) = data
                .get("error")
                .and_then(|err| err.get("message"))
                .and_then(Value::as_str)
            {
                message = text.to_string();
            } else if let Some(text) = data.get("error").and_then(Value::as_str) {
                message = text.to_string();
            }
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(TradeError::Auth(format!(
                "POESESSID 无效/过期。{}",
                support_hint()
            )));
        }
        return Err(TradeError::Request(format!(
            "HTTP {}: {message}",
            status.as_u16()
        )));
    }

    serde_json::from_str(&body).map_err(|_| TradeError::Request("接口返回不是 JSON".to_string()))
}

/// 将候选 Cookie 验证错误稳定分类为认证失败或网络/服务请求失败。
fn classify_candidate_failure(status: Option<u16>, detail: &str) -> TradeError {
    match status {
        Some(401 | 403) => TradeError::Auth(format!("POESESSID 无效/过期。{}", support_hint())),
        Some(code) => TradeError::Request(format!("HTTP {code}: {detail}")),
        None => TradeError::Request(format!("网络请求失败: {detail}")),
    }
}

/// 使用显式 Cookie 请求受限的国服 trade2 端点，候选 Secret 不需要提前落盘。
fn request_json_with_cookie(
    client: &Client,
    url: &str,
    method: Method,
    payload: Option<&Value>,
    cookie: &str,
) -> Result<Value, TradeError> {
    if !is_allowed_trade_endpoint(url) {
        return Err(TradeError::Request(
            "已阻止非国服 trade2 请求地址".to_string(),
        ));
    }

    let mut req = client
        .request(method, url)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("Cookie", cookie)
        .header("Referer", TRADE_HOME)
        .header("Origin", "https://poe.game.qq.com");

    if let Some(payload) = payload {
        req = req.json(payload);
    }

    let resp = req
        .send()
        .map_err(|err| classify_candidate_failure(None, &err.to_string()))?;
    let status = resp.status();
    let body = resp
        .text()
        .map_err(|err| TradeError::Request(format!("读取接口返回失败: {err}")))?;

    if !status.is_success() {
        let mut message = body.clone();
        if let Ok(data) = serde_json::from_str::<Value>(&body) {
            if let Some(text) = data
                .get("error")
                .and_then(|err| err.get("message"))
                .and_then(Value::as_str)
            {
                message = text.to_string();
            } else if let Some(text) = data.get("error").and_then(Value::as_str) {
                message = text.to_string();
            }
        }
        return Err(classify_candidate_failure(Some(status.as_u16()), &message));
    }

    serde_json::from_str(&body).map_err(|_| TradeError::Request("接口返回不是 JSON".to_string()))
}
fn scalar_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        text.to_string()
    } else if let Some(number) = value.as_i64() {
        number.to_string()
    } else if let Some(number) = value.as_f64() {
        if number.fract() == 0.0 {
            format!("{}", number as i64)
        } else {
            format!("{number:.2}")
        }
    } else {
        String::new()
    }
}

fn price_text(price: Option<&Value>) -> String {
    let Some(price) = price else {
        return "未标价".to_string();
    };
    let amount = price
        .get("amount")
        .or_else(|| price.get("value"))
        .map(scalar_text)
        .unwrap_or_default();
    let currency = price
        .get("currency")
        .or_else(|| price.get("type"))
        .map(scalar_text)
        .unwrap_or_default();
    let text = format!("{amount} {currency}").trim().to_string();
    if text.is_empty() {
        "未标价".to_string()
    } else {
        text
    }
}

fn item_text(item: Option<&Value>) -> String {
    let Some(item) = item else {
        return String::new();
    };
    let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
    let type_line = item
        .get("typeLine")
        .or_else(|| item.get("baseType"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    format!("{name} {type_line}").trim().to_string()
}

fn seller_text(listing: Option<&Value>) -> String {
    let Some(listing) = listing else {
        return String::new();
    };
    if let Some(account) = listing.get("account") {
        if let Some(name) = account.get("name").and_then(Value::as_str) {
            return name.to_string();
        }
        if let Some(name) = account.get("lastCharacterName").and_then(Value::as_str) {
            return name.to_string();
        }
    }
    listing
        .get("indexed")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn summarize_entries(entries: &[TradeEntry]) -> Vec<String> {
    let priced: Vec<String> = entries
        .iter()
        .filter_map(|entry| {
            if entry.price.is_empty() || entry.price == "未标价" {
                None
            } else {
                Some(entry.price.clone())
            }
        })
        .collect();
    if priced.is_empty() {
        return vec!["没有可读标价".to_string()];
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for price in &priced {
        *counts.entry(price.clone()).or_insert(0) += 1;
    }
    let mut common: Vec<(String, usize)> = counts.into_iter().collect();
    common.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let dist = common
        .into_iter()
        .take(4)
        .map(|(price, count)| format!("{price} x{count}"))
        .collect::<Vec<_>>()
        .join(" / ");
    vec![format!("最低: {}", priced[0]), format!("分布: {dist}")]
}

fn direct_trade_search(
    mut parsed: ParsedItem,
    options: QueryOptions,
) -> Result<TradeResult, TradeError> {
    let settings = load_config().settings.normalized();
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| TradeError::Request(format!("创建 HTTP 客户端失败: {err}")))?;
    if options.use_mods {
        resolve_item_mods(&mut parsed)?;
    }
    let payloads = build_payload_variants(&parsed, &options);
    let mut failures = Vec::new();

    for league in settings.leagues() {
        for (index, payload) in payloads.iter().enumerate() {
            let search_data = match request_json(
                &client,
                &trade_search_url(&league),
                Method::POST,
                Some(payload),
            ) {
                Ok(data) => data,
                Err(err @ TradeError::Auth(_)) => return Err(err),
                Err(err) => {
                    failures.push(format!("{league} #{}: {err}", index + 1));
                    continue;
                }
            };

            if !search_data.get("error").unwrap_or(&Value::Null).is_null() {
                let message = search_data
                    .get("error")
                    .and_then(|err| err.get("message"))
                    .and_then(Value::as_str)
                    .or_else(|| search_data.get("error").and_then(Value::as_str))
                    .unwrap_or("trade search error");
                failures.push(format!("{league} #{}: {message}", index + 1));
                continue;
            }

            let query_id = search_data
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if query_id.is_empty() {
                failures.push(format!("{league} #{}: 缺少查询 ID", index + 1));
                continue;
            }

            let result_ids: Vec<String> = search_data
                .get("result")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .take(settings.max_fetch_results)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_default();

            let total = search_data
                .get("total")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(result_ids.len());

            let mut entries = Vec::new();
            if !result_ids.is_empty() {
                let mut fetch_failures = Vec::new();
                for chunk in result_ids.chunks(settings.fetch_batch_size) {
                    match request_json(&client, &fetch_url(chunk, &query_id), Method::GET, None) {
                        Ok(fetch_data) => {
                            if !fetch_data.get("error").unwrap_or(&Value::Null).is_null() {
                                let message = fetch_data
                                    .get("error")
                                    .and_then(|err| err.get("message"))
                                    .and_then(Value::as_str)
                                    .or_else(|| fetch_data.get("error").and_then(Value::as_str))
                                    .unwrap_or("trade fetch error");
                                fetch_failures.push(message.to_string());
                                continue;
                            }
                            if let Some(results) =
                                fetch_data.get("result").and_then(Value::as_array)
                            {
                                for row in results {
                                    let listing = row.get("listing");
                                    let price =
                                        price_text(listing.and_then(|item| item.get("price")));
                                    let seller = seller_text(listing);
                                    let item_name = item_text(row.get("item"));
                                    entries.push(TradeEntry {
                                        price,
                                        seller,
                                        item_name,
                                    });
                                }
                            } else {
                                fetch_failures.push("明细响应缺少 result".to_string());
                            }
                        }
                        Err(err @ TradeError::Auth(_)) => return Err(err),
                        Err(err) => fetch_failures.push(err.to_string()),
                    }
                }
                if entries.is_empty() && !fetch_failures.is_empty() {
                    failures.push(format!(
                        "{league} fetch: {}",
                        fetch_failures
                            .into_iter()
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(" / ")
                    ));
                    continue;
                }
            }

            let summary = summarize_entries(&entries);
            let url = trade_result_url(&league, &query_id);
            return Ok(TradeResult {
                item: parsed,
                league,
                total,
                entries,
                summary,
                url,
                options,
                page_size: settings.page_size,
            });
        }
    }

    let message = if failures.is_empty() {
        "没有可用查询".to_string()
    } else {
        failures
            .into_iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" / ")
    };
    Err(TradeError::Request(message))
}

#[derive(Debug)]
enum OverlayEvent {
    Message {
        title: String,
        lines: Vec<String>,
        accent: u32,
        timeout: Duration,
    },
    Result {
        result: Box<TradeResult>,
        accent: u32,
        timeout: Duration,
    },
}

#[derive(Debug)]
enum Action {
    Price,
    OpenHome,
    Quit,
}

#[derive(Clone)]
enum ViewKind {
    Message(Vec<String>),
    Result(Box<TradeResult>),
}

#[derive(Clone)]
struct OverlayView {
    title: String,
    subtitle: String,
    status: String,
    current_url: String,
    accent: u32,
    kind: ViewKind,
}

struct MetricCard<'a> {
    x: i32,
    y: i32,
    w: i32,
    label: &'a str,
    value: &'a str,
    color: u32,
}

impl Default for OverlayView {
    fn default() -> Self {
        Self {
            title: APP_DISPLAY_NAME.to_string(),
            subtitle: "Ctrl+C 自动查价".to_string(),
            status: String::new(),
            current_url: String::new(),
            accent: rgb(56, 189, 248),
            kind: ViewKind::Message(vec![
                "游戏内 Ctrl+C 后自动查价。".to_string(),
                "首次使用请先在托盘右键设置 Cookie。".to_string(),
            ]),
        }
    }
}

struct Fonts {
    title: HFONT,
    normal: HFONT,
    small: HFONT,
    bold: HFONT,
}

impl Fonts {
    unsafe fn new() -> Self {
        Self {
            title: make_font(19, FW_BOLD as i32),
            normal: make_font(15, FW_NORMAL as i32),
            small: make_font(13, FW_NORMAL as i32),
            bold: make_font(15, FW_BOLD as i32),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.title as _);
            DeleteObject(self.normal as _);
            DeleteObject(self.small as _);
            DeleteObject(self.bold as _);
        }
    }
}

unsafe fn make_font(size: i32, weight: i32) -> HFONT {
    let face = wide("Microsoft YaHei UI");
    CreateFontW(
        -size,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        (DEFAULT_PITCH | FF_DONTCARE) as u32,
        face.as_ptr(),
    )
}

struct UiState {
    hwnd: HWND,
    event_tx: Sender<OverlayEvent>,
    event_rx: Receiver<OverlayEvent>,
    action_rx: Receiver<Action>,
    view: OverlayView,
    fonts: Fonts,
    pinned: bool,
    hide_deadline: Option<Instant>,
    overlay_pos: Option<WindowPos>,
    page: usize,
    query_options: QueryOptions,
    last_clipboard_text: String,
    last_clipboard_check: Instant,
    last_settings_reload: Instant,
    settings: AppSettings,
    registered_manual_hotkey: Option<String>,
    tray_added: bool,
    app_icon: HICON,
    app_icon_owned: bool,
    auto_paused: bool,
    balloon_counter: u64,
}

impl UiState {
    unsafe fn new(
        event_tx: Sender<OverlayEvent>,
        event_rx: Receiver<OverlayEvent>,
        action_rx: Receiver<Action>,
    ) -> Self {
        Self {
            hwnd: null_mut(),
            event_tx,
            event_rx,
            action_rx,
            view: OverlayView::default(),
            fonts: Fonts::new(),
            pinned: false,
            hide_deadline: None,
            overlay_pos: load_config().overlay_pos,
            page: 0,
            query_options: QueryOptions::default(),
            last_clipboard_text: read_clipboard_text().unwrap_or_default(),
            last_clipboard_check: Instant::now(),
            last_settings_reload: Instant::now(),
            settings: load_config().settings.normalized(),
            registered_manual_hotkey: None,
            tray_added: false,
            app_icon: null_mut(),
            app_icon_owned: false,
            auto_paused: false,
            balloon_counter: 0,
        }
    }

    unsafe fn init_shell_presence(&mut self) {
        let (icon, owned) = load_app_icon(32);
        self.app_icon = icon;
        self.app_icon_owned = owned;
        self.add_tray_icon();
    }

    unsafe fn add_tray_icon(&mut self) {
        if self.tray_added {
            return;
        }
        let mut data = notify_icon_data(self.hwnd);
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WM_TRAYICON;
        data.hIcon = self.app_icon;
        copy_wide_fixed(&mut data.szTip, "流放2查价助手 - Ctrl+C 自动查价");
        if Shell_NotifyIconW(NIM_ADD, &data) != 0 {
            self.tray_added = true;
            let mut version_data = notify_icon_data(self.hwnd);
            version_data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            Shell_NotifyIconW(NIM_SETVERSION, &version_data);
        }
    }

    /// 显示托盘气泡通知
    unsafe fn show_tray_balloon(&mut self, title: &str, message: &str, is_error: bool) {
        self.balloon_counter = self.balloon_counter.wrapping_add(1);
        let mut data = notify_icon_data(self.hwnd);
        data.uFlags = NIF_INFO;
        data.dwInfoFlags = if is_error { NIIF_WARNING } else { NIIF_INFO };
        data.Anonymous.uTimeout = 10000;
        copy_wide_fixed(&mut data.szInfoTitle, title);
        copy_wide_fixed(&mut data.szInfo, message);
        Shell_NotifyIconW(NIM_MODIFY, &data);
    }

    unsafe fn remove_tray_icon(&mut self) {
        if self.tray_added {
            let data = notify_icon_data(self.hwnd);
            Shell_NotifyIconW(NIM_DELETE, &data);
            self.tray_added = false;
        }
        if self.app_icon_owned && !self.app_icon.is_null() {
            DestroyIcon(self.app_icon);
        }
        self.app_icon = null_mut();
        self.app_icon_owned = false;
    }

    unsafe fn refresh_manual_hotkey(&mut self) {
        let desired = manual_hotkey_label(&self.settings);
        if self.registered_manual_hotkey.as_deref() == Some(desired.as_str()) {
            return;
        }

        UnregisterHotKey(self.hwnd, HOTKEY_PRICE_ID);
        self.registered_manual_hotkey = Some(desired.clone());

        let Some(spec) = manual_hotkey_spec(&self.settings.manual_hotkey) else {
            log("手动查价热键已关闭。");
            return;
        };

        if RegisterHotKey(self.hwnd, HOTKEY_PRICE_ID, spec.modifiers, spec.vk) == 0 {
            let message = format!(
                "注册手动查价热键 {} 失败，可能已被其他程序占用。",
                spec.label
            );
            eprintln!("{message}");
            log(message);
        } else {
            log(format!("手动查价热键已启用: {}", spec.label));
        }
    }

    unsafe fn show_from_tray(&mut self) {
        ShowWindow(self.hwnd, SW_SHOW);
        SetForegroundWindow(self.hwnd);
        self.hide_deadline = None;
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn show_tray_menu(&mut self) {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let labels = [
            wide("打开面板"),
            wide("立即查价"),
            wide(if self.auto_paused { "恢复自动查价" } else { "暂停自动查价" }),
            wide("首次使用向导"),
            wide("设置"),
            wide(if self.settings.primary_league == "永久" { "切换联赛: 奥杜尔秘符" } else { "切换联赛: 永久" }),
            wide("设置 Cookie"),
            wide("验证 Cookie"),
            wide("查询历史"),
            wide("打开国服市集"),
            wide("运行自检"),
            wide("导出诊断"),
            wide("检查更新"),
            wide("关于"),
            wide("退出"),
        ];
        AppendMenuW(menu, MF_STRING, IDM_SHOW, labels[0].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_PRICE, labels[1].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_TOGGLE_AUTO, labels[2].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_WIZARD, labels[3].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_SETTINGS, labels[4].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_LEAGUE_STANDARD, labels[5].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_COOKIE, labels[6].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_VALIDATE_COOKIE, labels[7].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_HISTORY, labels[8].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_TRADE_HOME, labels[9].as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, IDM_SELF_CHECK, labels[10].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_DIAGNOSTICS, labels[11].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_UPDATE_CHECK, labels[12].as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_ABOUT, labels[13].as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, IDM_QUIT, labels[14].as_ptr());

        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        SetForegroundWindow(self.hwnd);
        let command_id = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            self.hwnd,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(self.hwnd, WM_NULL, 0, 0);

        if command_id != 0 {
            self.handle_tray_command(command_id as usize);
        }
    }

    unsafe fn handle_tray_command(&mut self, command_id: usize) {
        match command_id {
            IDM_SHOW => self.show_from_tray(),
            IDM_PRICE => start_price_query(self.event_tx.clone()),
            IDM_TOGGLE_AUTO => {
                self.auto_paused = !self.auto_paused;
                let mut data = notify_icon_data(self.hwnd);
                data.uFlags = NIF_TIP;
                copy_wide_fixed(
                    &mut data.szTip,
                    if self.auto_paused {
                        "流放2查价助手 - Ctrl+C 自动查价 [已暂停]"
                    } else {
                        "流放2查价助手 - Ctrl+C 自动查价"
                    },
                );
                Shell_NotifyIconW(NIM_MODIFY, &data);
            }
            IDM_WIZARD => self.open_first_run_wizard(),
            IDM_SETTINGS => self.open_settings(),
            IDM_LEAGUE_STANDARD | IDM_LEAGUE_PERMANENT => {
                let new_league = if command_id == IDM_LEAGUE_PERMANENT {
                    "永久"
                } else {
                    "奥杜尔秘符"
                };
                let mut config = load_config();
                config.settings.primary_league = new_league.to_string();
                if let Err(err) = save_config(&config) {
                    eprintln!("保存联赛设置失败: {err}");
                } else {
                    self.settings = load_config().settings.normalized();
                }
            }
            IDM_COOKIE => self.open_cookie_setup(),
            IDM_VALIDATE_COOKIE => start_cookie_validation(self.event_tx.clone()),
            IDM_HISTORY => self.open_history(),
            IDM_TRADE_HOME => open_url(TRADE_HOME),
            IDM_SELF_CHECK => self.export_self_check(),
            IDM_DIAGNOSTICS => self.export_diagnostics(),
            IDM_UPDATE_CHECK => self.export_update_check(),
            IDM_ABOUT => self.show_about(),
            IDM_QUIT => {
                DestroyWindow(self.hwnd);
            }
            _ => {}
        }
    }

    unsafe fn apply_event(&mut self, event: OverlayEvent) {
        match event {
            OverlayEvent::Message {
                title,
                lines,
                accent,
                timeout,
            } => {
                self.view = OverlayView {
                    title,
                    subtitle: APP_DISPLAY_NAME.to_string(),
                    status: String::new(),
                    current_url: String::new(),
                    accent,
                    kind: ViewKind::Message(lines),
                };
                self.show_panel(720, 250, timeout);
            }
            OverlayEvent::Result {
                result,
                accent,
                timeout,
            } => {
                self.page = 0;
                self.query_options = result.options.clone();
                let subtitle = [
                    result.item.base_type.clone(),
                    result.item.rarity_raw.clone(),
                    result.league.clone(),
                ]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" | ");
                let status = format!(
                    "查询到 {} 条，显示前 {} 条",
                    result.total,
                    min(result.entries.len(), result.page_size)
                );
                let display_name = result.item.display.clone();
                let balloon_total = result.total;
                let balloon_priced = result
                    .entries
                    .iter()
                    .map(|e| e.price.as_str())
                    .find(|p| !p.is_empty() && *p != "未标价")
                    .unwrap_or("无标价")
                    .to_string();
                let balloon_url = result.url.clone();
                self.view = OverlayView {
                    title: display_name.clone(),
                    subtitle: if subtitle.is_empty() {
                        APP_DISPLAY_NAME.to_string()
                    } else {
                        subtitle
                    },
                    status,
                    current_url: balloon_url,
                    accent,
                    kind: ViewKind::Result(result),
                };
                let height = match &self.view.kind {
                    ViewKind::Result(result) if result.entries.is_empty() => 360,
                    _ => 500,
                };
                self.show_panel(760, height, timeout);
                // 查询成功时显示托盘气泡通知
                self.show_tray_balloon(
                    "查价完成",
                    &format!(
                        "{}  最低价: {}  共{}条挂单",
                        display_name, balloon_priced, balloon_total
                    ),
                    false,
                );
            }
        }
    }

    unsafe fn show_panel(&mut self, width: i32, height: i32, timeout: Duration) {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos = self.overlay_pos.unwrap_or(WindowPos {
            x: max(20, screen_w - width - 36),
            y: 92,
        });
        let x = min(max(0, pos.x), max(0, screen_w - width));
        let y = min(max(0, pos.y), max(0, screen_h - height));
        SetWindowPos(self.hwnd, -1isize as _, x, y, width, height, SWP_SHOWWINDOW);
        self.hide_deadline = if self.pinned {
            None
        } else {
            Some(Instant::now() + timeout)
        };
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn handle_timer(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                Action::Price => start_price_query(self.event_tx.clone()),
                Action::OpenHome => open_url(TRADE_HOME),
                Action::Quit => {
                    DestroyWindow(self.hwnd);
                    return;
                }
            }
        }

        while let Ok(event) = self.event_rx.try_recv() {
            self.apply_event(event);
        }

        self.reload_settings_if_needed();
        self.poll_clipboard_auto_query();

        if !self.pinned
            && let Some(deadline) = self.hide_deadline
            && Instant::now() >= deadline
        {
            self.hide_deadline = None;
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    fn poll_clipboard_auto_query(&mut self) {
        if self.auto_paused {
            return;
        }
        if !self.settings.auto_clipboard {
            return;
        }
        if self.last_clipboard_check.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_clipboard_check = Instant::now();
        let Ok(text) = read_clipboard_text() else {
            return;
        };
        if text == self.last_clipboard_text {
            return;
        }
        self.last_clipboard_text = text.clone();
        if !looks_like_poe_item_text(&text) {
            return;
        }
        start_price_query_from_text(text, self.event_tx.clone(), false, QueryOptions::default());
    }

    fn reload_settings_if_needed(&mut self) {
        if self.last_settings_reload.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_settings_reload = Instant::now();
        let next_settings = load_config().settings.normalized();
        let hotkey_changed = next_settings.manual_hotkey != self.settings.manual_hotkey;
        self.settings = next_settings;
        if hotkey_changed {
            unsafe {
                self.refresh_manual_hotkey();
            }
        }
    }

    unsafe fn toggle_pin(&mut self) {
        self.pinned = !self.pinned;
        self.hide_deadline = None;
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn copy_url(&mut self) {
        if self.view.current_url.is_empty() {
            self.view.status = "没有可复制的市集链接".to_string();
        } else {
            match copy_text_to_clipboard(&self.view.current_url) {
                Ok(_) => self.view.status = "已复制市集链接".to_string(),
                Err(err) => self.view.status = format!("复制失败: {err}"),
            }
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    /// 复制私聊消息到剪贴板，index 为原始 entries 中的行索引
    unsafe fn copy_whisper_for_row(&mut self, row_index: usize) {
        let Some(result) = self.current_result() else {
            self.view.status = "没有可复制的查询结果".to_string();
            InvalidateRect(self.hwnd, null(), 1);
            return;
        };
        let Some(entry) = result.entries.get(row_index) else {
            self.view.status = format!("行索引 {row_index} 无效");
            InvalidateRect(self.hwnd, null(), 1);
            return;
        };
        let message = format!(
            "@{} Hi, I'd like to buy your {} listed for {} in {}",
            entry.seller, entry.item_name, entry.price, result.league
        );
        match copy_text_to_clipboard(&message) {
            Ok(_) => {
                self.view.status = format!("已复制 whisper 消息: @{}", entry.seller);
            }
            Err(err) => self.view.status = format!("复制失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    fn current_result(&self) -> Option<&TradeResult> {
        match &self.view.kind {
            ViewKind::Result(result) => Some(result.as_ref()),
            ViewKind::Message(_) => None,
        }
    }

    unsafe fn rerun_current_query(&mut self) {
        let Some(result) = self.current_result() else {
            return;
        };
        let parsed = result.item.clone();
        let options = self.query_options.clone();
        self.view.status = "正在按新筛选重新查询...".to_string();
        InvalidateRect(self.hwnd, null(), 1);
        start_price_query_from_parsed(parsed, options, self.event_tx.clone());
    }

    unsafe fn toggle_mod_filters(&mut self) {
        if self.current_result().is_none() {
            return;
        }
        self.query_options.use_mods = !self.query_options.use_mods;
        self.query_options.selected_mod_patterns = None;
        if !self.query_options.use_mods {
            self.query_options.use_values = false;
        }
        self.rerun_current_query();
    }

    unsafe fn toggle_value_filters(&mut self) {
        if self.current_result().is_none() {
            return;
        }
        self.query_options.use_mods = true;
        self.query_options.use_values = !self.query_options.use_values;
        self.rerun_current_query();
    }

    unsafe fn toggle_single_mod(&mut self, index: usize) {
        let Some(result) = self.current_result() else {
            return;
        };
        let patterns = result
            .item
            .mods
            .iter()
            .map(|item_mod| item_mod.pattern.clone())
            .collect::<Vec<_>>();
        let Some(pattern) = patterns.get(index).cloned() else {
            return;
        };

        let mut selected = self
            .query_options
            .selected_mod_patterns
            .clone()
            .unwrap_or_else(|| patterns.clone());
        if let Some(position) = selected.iter().position(|value| value == &pattern) {
            if selected.len() <= 1 {
                self.view.status = "至少保留一个属性筛选".to_string();
                InvalidateRect(self.hwnd, null(), 1);
                return;
            }
            selected.remove(position);
        } else {
            selected.push(pattern);
        }

        self.query_options.use_mods = true;
        self.query_options.selected_mod_patterns = Some(selected);
        self.rerun_current_query();
    }

    unsafe fn page_prev(&mut self) {
        if self.page > 0 {
            self.page -= 1;
            InvalidateRect(self.hwnd, null(), 1);
        }
    }

    unsafe fn page_next(&mut self) {
        let Some(result) = self.current_result() else {
            return;
        };
        if (self.page + 1) * result.page_size < result.entries.len() {
            self.page += 1;
            InvalidateRect(self.hwnd, null(), 1);
        }
    }

    fn is_mod_selected(&self, item_mod: &ParsedMod) -> bool {
        if !self.query_options.use_mods {
            return false;
        }
        self.query_options
            .selected_mod_patterns
            .as_ref()
            .map(|patterns| patterns.iter().any(|pattern| pattern == &item_mod.pattern))
            .unwrap_or(true)
    }

    fn mod_chip_label(index: usize, item_mod: &ParsedMod) -> String {
        let label = item_mod
            .stat_text
            .as_deref()
            .unwrap_or(item_mod.text.as_str());
        format!("属性{} {}", index + 1, label)
    }

    fn button_specs(&self, rect: RECT) -> Vec<UiButtonSpec> {
        let result = self.current_result();
        let has_url = !self.view.current_url.is_empty();
        let has_mods = result
            .map(|result| !result.item.mods.is_empty())
            .unwrap_or(false);
        let can_prev = self.page > 0;
        let can_next = result
            .map(|result| (self.page + 1) * result.page_size < result.entries.len())
            .unwrap_or(false);

        let mut specs = vec![
            UiButtonSpec {
                button: UiButton::Pin,
                label: (if self.pinned { "已固定" } else { "固定" }).to_string(),
                rect: RECT {
                    left: rect.right - 150,
                    top: 14,
                    right: rect.right - 92,
                    bottom: 42,
                },
                enabled: true,
                primary: false,
            },
            UiButtonSpec {
                button: UiButton::Close,
                label: "关闭".to_string(),
                rect: RECT {
                    left: rect.right - 84,
                    top: 14,
                    right: rect.right - 26,
                    bottom: 42,
                },
                enabled: true,
                primary: false,
            },
        ];

        if matches!(self.view.kind, ViewKind::Message(_)) {
            specs.extend([
                UiButtonSpec {
                    button: UiButton::Wizard,
                    label: "向导".to_string(),
                    rect: RECT {
                        left: 16,
                        top: rect.bottom - 44,
                        right: 96,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Cookie,
                    label: "设置Cookie".to_string(),
                    rect: RECT {
                        left: 104,
                        top: rect.bottom - 44,
                        right: 200,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: true,
                },
                UiButtonSpec {
                    button: UiButton::ValidateCookie,
                    label: "验证Cookie".to_string(),
                    rect: RECT {
                        left: 208,
                        top: rect.bottom - 44,
                        right: 304,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Open,
                    label: "打开官网".to_string(),
                    rect: RECT {
                        left: 312,
                        top: rect.bottom - 44,
                        right: 408,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::History,
                    label: "历史".to_string(),
                    rect: RECT {
                        left: 416,
                        top: rect.bottom - 44,
                        right: 496,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Diagnostics,
                    label: "诊断".to_string(),
                    rect: RECT {
                        left: 504,
                        top: rect.bottom - 44,
                        right: 584,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
            ]);
        } else {
            if let Some(result) = result {
                let chip_count = min(4, result.item.mods.len());
                if chip_count > 0 {
                    let chip_gap = 8;
                    let chip_w =
                        (rect.right - 40 - chip_gap * (chip_count as i32 - 1)) / chip_count as i32;
                    for (index, item_mod) in result.item.mods.iter().take(chip_count).enumerate() {
                        let left = 20 + index as i32 * (chip_w + chip_gap);
                        specs.push(UiButtonSpec {
                            button: UiButton::ModToggle(index),
                            label: Self::mod_chip_label(index, item_mod),
                            rect: RECT {
                                left,
                                top: 176,
                                right: left + chip_w,
                                bottom: 202,
                            },
                            enabled: true,
                            primary: self.is_mod_selected(item_mod),
                        });
                    }
                }
                // 每行的私聊W按钮
                let page_size = result.page_size.max(1);
                let pages = max(1, result.entries.len().div_ceil(page_size));
                let page = min(self.page, pages - 1);
                let visible_start = page * page_size;
                let visible_entries = result
                    .entries
                    .iter()
                    .skip(visible_start)
                    .take(page_size);
                for (idx, _entry) in visible_entries.enumerate() {
                    let row_top = 210 + 28 + idx as i32 * 27;
                    let entry_index = visible_start + idx;
                    specs.push(UiButtonSpec {
                        button: UiButton::Whisper(entry_index),
                        label: "W".to_string(),
                        rect: RECT {
                            left: rect.right - 70,
                            top: row_top + 6,
                            right: rect.right - 32,
                            bottom: row_top + 22,
                        },
                        enabled: true,
                        primary: false,
                    });
                }
            }
            specs.extend([
                UiButtonSpec {
                    button: UiButton::Mods,
                    label: (if self.query_options.use_mods {
                        "同属性开"
                    } else {
                        "同属性"
                    })
                    .to_string(),
                    rect: RECT {
                        left: 16,
                        top: rect.bottom - 44,
                        right: 112,
                        bottom: rect.bottom - 14,
                    },
                    enabled: has_mods,
                    primary: self.query_options.use_mods,
                },
                UiButtonSpec {
                    button: UiButton::Values,
                    label: (if self.query_options.use_values {
                        "数值开"
                    } else {
                        "数值"
                    })
                    .to_string(),
                    rect: RECT {
                        left: 120,
                        top: rect.bottom - 44,
                        right: 216,
                        bottom: rect.bottom - 14,
                    },
                    enabled: has_mods,
                    primary: self.query_options.use_values,
                },
                UiButtonSpec {
                    button: UiButton::Prev,
                    label: "上一页".to_string(),
                    rect: RECT {
                        left: 224,
                        top: rect.bottom - 44,
                        right: 304,
                        bottom: rect.bottom - 14,
                    },
                    enabled: can_prev,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Next,
                    label: "下一页".to_string(),
                    rect: RECT {
                        left: 312,
                        top: rect.bottom - 44,
                        right: 392,
                        bottom: rect.bottom - 14,
                    },
                    enabled: can_next,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Open,
                    label: "打开市集".to_string(),
                    rect: RECT {
                        left: 400,
                        top: rect.bottom - 44,
                        right: 496,
                        bottom: rect.bottom - 14,
                    },
                    enabled: true,
                    primary: false,
                },
                UiButtonSpec {
                    button: UiButton::Copy,
                    label: "复制链接".to_string(),
                    rect: RECT {
                        left: 504,
                        top: rect.bottom - 44,
                        right: 600,
                        bottom: rect.bottom - 14,
                    },
                    enabled: has_url,
                    primary: false,
                },
            ]);
        }
        specs
    }

    unsafe fn handle_button_click(&mut self, x: i32, y: i32) -> bool {
        let mut rect = RECT::default();
        GetClientRect(self.hwnd, &mut rect);
        for spec in self.button_specs(rect) {
            if !rect_contains(&spec.rect, x, y) {
                continue;
            }
            if !spec.enabled {
                self.view.status = "这个操作当前不可用".to_string();
                InvalidateRect(self.hwnd, null(), 1);
                return true;
            }
            match spec.button {
                UiButton::Pin => self.toggle_pin(),
                UiButton::Close => {
                    ShowWindow(self.hwnd, SW_HIDE);
                }
                UiButton::Wizard => self.open_first_run_wizard(),
                UiButton::Cookie => self.open_cookie_setup(),
                UiButton::ValidateCookie => start_cookie_validation(self.event_tx.clone()),
                UiButton::Diagnostics => self.export_diagnostics(),
                UiButton::History => self.open_history(),
                UiButton::Mods => self.toggle_mod_filters(),
                UiButton::Values => self.toggle_value_filters(),
                UiButton::ModToggle(index) => self.toggle_single_mod(index),
                UiButton::Prev => self.page_prev(),
                UiButton::Next => self.page_next(),
                UiButton::Open => self.open_current_url(),
                UiButton::Copy => self.copy_url(),
                UiButton::Whisper(index) => self.copy_whisper_for_row(index),
            }
            return true;
        }
        false
    }

    /// 键盘快捷键处理，返回 true 表示已处理
    unsafe fn handle_key_down(&mut self, vk_code: u32) -> bool {
        let has_result = self.current_result().is_some();
        match vk_code {
            0x25 => { if has_result { self.page_prev(); } else { return false; } } // ← 上一页
            0x27 => { if has_result { self.page_next(); } else { return false; } } // → 下一页
            0x4D => { if has_result { self.toggle_mod_filters(); } else { return false; } } // M 同属性
            0x56 => { if has_result { self.toggle_value_filters(); } else { return false; } } // V 数值
            0x50 => { self.toggle_pin(); } // P 固定
            0x43 => { if has_result { self.copy_url(); } else { return false; } } // C 复制链接
            0x4F => { self.open_current_url(); } // O 打开市集
            0x31..=0x34 => { // 1-4 切换属性chip
                if has_result {
                    let index = (vk_code - 0x31) as usize;
                    self.toggle_single_mod(index);
                } else { return false; }
            }
            0x1B => { ShowWindow(self.hwnd, SW_HIDE); } // Esc 关闭
            _ => return false,
        }
        InvalidateRect(self.hwnd, null(), 1);
        true
    }

    unsafe fn open_current_url(&self) {
        if self.view.current_url.is_empty() {
            open_url(TRADE_HOME);
        } else {
            open_url(&self.view.current_url);
        }
    }

    unsafe fn open_cookie_setup(&mut self) {
        match launch_cookie_setup() {
            Ok(_) => self.view.status = "已打开 Cookie 设置窗口".to_string(),
            Err(err) => {
                self.view.status = format!("打开 Cookie 设置失败: {err}");
                open_url(TRADE_HOME);
            }
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn open_settings(&mut self) {
        match launch_settings_setup() {
            Ok(_) => self.view.status = "已打开设置窗口".to_string(),
            Err(err) => self.view.status = format!("打开设置失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn open_first_run_wizard(&mut self) {
        match launch_first_run_wizard() {
            Ok(_) => self.view.status = "已打开首次使用向导".to_string(),
            Err(err) => self.view.status = format!("打开向导失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn open_history(&mut self) {
        match launch_history_viewer() {
            Ok(_) => self.view.status = "已打开查询历史".to_string(),
            Err(err) => self.view.status = format!("打开历史失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn export_diagnostics(&mut self) {
        match write_diagnostics(None) {
            Ok(path) => {
                self.view.status = format!("诊断已导出: {}", path.display());
                open_path(&path);
            }
            Err(err) => {
                self.view.status = format!("导出诊断失败: {err}");
            }
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn export_self_check(&mut self) {
        match write_self_check(None) {
            Ok(path) => {
                self.view.status = format!("自检已导出: {}", path.display());
                open_path(&path);
            }
            Err(err) => {
                self.view.status = format!("运行自检失败: {err}");
            }
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn export_update_check(&mut self) {
        match write_update_check(None) {
            Ok(path) => {
                self.view.status = format!("更新检查已导出: {}", path.display());
                open_path(&path);
            }
            Err(err) => {
                self.view.status = format!("检查更新失败: {err}");
            }
        }
        InvalidateRect(self.hwnd, null(), 1);
    }

    unsafe fn show_about(&mut self) {
        let settings = load_config().settings.normalized();
        self.view = OverlayView {
            title: format!("{APP_DISPLAY_NAME} v{APP_VERSION}"),
            subtitle: "流放2查价助手".to_string(),
            status: "托盘右键可运行自检、导出诊断或退出。".to_string(),
            current_url: TRADE_HOME.to_string(),
            accent: rgb(56, 189, 248),
            kind: ViewKind::Message(vec![
                "国服 trade2 查价；不修改游戏进程，不读写游戏内存。".to_string(),
                format!(
                    "Ctrl+C 自动查价；手动热键: {}",
                    manual_hotkey_label(&settings)
                ),
                "Cookie 使用 Windows DPAPI 加密，仅当前 Windows 用户可解密。".to_string(),
                "遇到问题先运行自检，再把 selfcheck.txt 发给维护者。".to_string(),
            ]),
        };
        self.show_panel(720, 250, Duration::from_secs(18));
    }

    unsafe fn save_position(&mut self) {
        let mut rect = RECT::default();
        if GetWindowRect(self.hwnd, &mut rect) == 0 {
            return;
        }
        self.overlay_pos = Some(WindowPos {
            x: rect.left,
            y: rect.top,
        });
        let mut config = load_config();
        config.overlay_pos = self.overlay_pos;
        if let Err(err) = save_config(&config) {
            eprintln!("保存面板位置失败: {err}");
        }
    }

    unsafe fn paint(&self) {
        let mut ps = MaybeUninit::<PAINTSTRUCT>::zeroed().assume_init();
        let hdc = BeginPaint(self.hwnd, &mut ps);
        let mut rect = RECT::default();
        GetClientRect(self.hwnd, &mut rect);

        fill(hdc, rect, rgb(12, 13, 16));
        fill(
            hdc,
            RECT {
                left: 0,
                top: 0,
                right: rect.right,
                bottom: 64,
            },
            rgb(18, 20, 24),
        );
        fill(
            hdc,
            RECT {
                left: 0,
                top: 0,
                right: 5,
                bottom: rect.bottom,
            },
            self.view.accent,
        );

        draw_text(
            hdc,
            &self.view.title,
            RECT {
                left: 16,
                top: 9,
                right: rect.right - 170,
                bottom: 34,
            },
            self.view.accent,
            self.fonts.title,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            &self.view.subtitle,
            RECT {
                left: 16,
                top: 36,
                right: rect.right - 170,
                bottom: 58,
            },
            rgb(148, 163, 184),
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        match &self.view.kind {
            ViewKind::Message(lines) => self.paint_message(hdc, rect, lines),
            ViewKind::Result(result) => self.paint_result(hdc, rect, result),
        }

        self.paint_buttons(hdc, rect);

        let status = if self.view.status.is_empty() {
            if self.pinned {
                "面板已固定".to_string()
            } else {
                String::new()
            }
        } else {
            self.view.status.clone()
        };
        draw_text(
            hdc,
            &status,
            RECT {
                left: 612,
                top: rect.bottom - 40,
                right: rect.right - 16,
                bottom: rect.bottom - 12,
            },
            rgb(148, 163, 184),
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        // 快捷键提示：放在底部按钮上方，避免与按钮重叠
        draw_text(
            hdc,
            "快捷键: ←→ 翻页  M 切换属性  V 数值  P 固定  C 复制  O 市集  Esc 关闭",
            RECT {
                left: 16,
                top: rect.bottom - 66,
                right: rect.right - 16,
                bottom: rect.bottom - 48,
            },
            rgb(100, 116, 139),
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        EndPaint(self.hwnd, &ps);
    }

    unsafe fn paint_message(&self, hdc: HDC, rect: RECT, lines: &[String]) {
        let mut y = 82;
        for line in lines.iter().take(4) {
            draw_text(
                hdc,
                line,
                RECT {
                    left: 20,
                    top: y,
                    right: rect.right - 20,
                    bottom: y + 28,
                },
                rgb(248, 250, 252),
                self.fonts.normal,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 30;
        }
    }

    unsafe fn paint_buttons(&self, hdc: HDC, rect: RECT) {
        for spec in self.button_specs(rect) {
            let (fill_color, border_color, text_color) = if !spec.enabled {
                (rgb(31, 34, 40), rgb(45, 49, 58), rgb(105, 113, 128))
            } else if spec.primary {
                (rgb(31, 91, 72), rgb(52, 211, 153), rgb(220, 252, 231))
            } else if spec.button == UiButton::Close {
                (rgb(52, 31, 36), rgb(101, 43, 55), rgb(254, 202, 202))
            } else {
                (rgb(26, 30, 38), rgb(58, 65, 78), rgb(226, 232, 240))
            };
            rounded_rect(hdc, spec.rect, fill_color, border_color, 10);
            draw_text(
                hdc,
                &spec.label,
                RECT {
                    left: spec.rect.left + 8,
                    top: spec.rect.top,
                    right: spec.rect.right - 8,
                    bottom: spec.rect.bottom,
                },
                text_color,
                self.fonts.small,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }
    }

    unsafe fn paint_result(&self, hdc: HDC, rect: RECT, result: &TradeResult) {
        let priced = result
            .entries
            .iter()
            .map(|entry| entry.price.as_str())
            .find(|price| !price.is_empty() && *price != "未标价")
            .unwrap_or("无标价");
        let distribution = result
            .summary
            .get(1)
            .map(|line| line.replace("分布: ", ""))
            .or_else(|| result.summary.first().cloned())
            .unwrap_or_else(|| "暂无分布".to_string());

        let gap = 10;
        let card_w = (rect.right - 40 - gap * 2) / 3;
        let total_text = result.total.to_string();
        self.metric_card(
            hdc,
            MetricCard {
                x: 20,
                y: 78,
                w: card_w,
                label: "最低价",
                value: priced,
                color: rgb(134, 239, 172),
            },
        );
        self.metric_card(
            hdc,
            MetricCard {
                x: 20 + card_w + gap,
                y: 78,
                w: card_w,
                label: "挂单数",
                value: &total_text,
                color: rgb(191, 219, 254),
            },
        );
        self.metric_card(
            hdc,
            MetricCard {
                x: 20 + (card_w + gap) * 2,
                y: 78,
                w: card_w,
                label: "常见价格",
                value: &distribution,
                color: rgb(253, 230, 138),
            },
        );

        let page_size = result.page_size.max(1);
        let pages = max(1, result.entries.len().div_ceil(page_size));
        let page = min(self.page, pages - 1);
        let detected_mods = result.item.mods.len();
        let matched_mods = result
            .item
            .mods
            .iter()
            .filter(|item_mod| item_mod.stat_id.is_some())
            .count();
        let mod_mode = if self.query_options.use_mods {
            "同属性 开"
        } else {
            "同属性 关"
        };
        let value_mode = if self.query_options.use_values {
            "数值 开"
        } else {
            "数值 关"
        };
        let selected_mod_count = if self.query_options.use_mods {
            self.query_options
                .selected_mod_patterns
                .as_ref()
                .map(|patterns| patterns.len())
                .unwrap_or(detected_mods)
        } else {
            0
        };
        let filter_line = if detected_mods > 0 {
            format!(
                "{mod_mode}   {value_mode}   已选属性 {selected_mod_count}/{detected_mods}   匹配 {matched_mods}/{detected_mods}   第 {}/{} 页",
                page + 1,
                pages
            )
        } else {
            format!(
                "{mod_mode}   {value_mode}   未识别到可筛选属性   第 {}/{} 页",
                page + 1,
                pages
            )
        };
        draw_text(
            hdc,
            &filter_line,
            RECT {
                left: 22,
                top: 152,
                right: rect.right - 22,
                bottom: 174,
            },
            rgb(148, 163, 184),
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        let table_top = 210;
        rounded_rect(
            hdc,
            RECT {
                left: 20,
                top: table_top,
                right: rect.right - 20,
                bottom: table_top + 28,
            },
            rgb(28, 32, 39),
            rgb(58, 65, 78),
            8,
        );
        draw_text(
            hdc,
            "价格",
            RECT {
                left: 32,
                top: table_top,
                right: 180,
                bottom: table_top + 28,
            },
            rgb(203, 213, 225),
            self.fonts.bold,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        draw_text(
            hdc,
            "卖家",
            RECT {
                left: 190,
                top: table_top,
                right: 330,
                bottom: table_top + 28,
            },
            rgb(203, 213, 225),
            self.fonts.bold,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        draw_text(
            hdc,
            "物品",
            RECT {
                left: 340,
                top: table_top,
                right: rect.right - 32,
                bottom: table_top + 28,
            },
            rgb(203, 213, 225),
            self.fonts.bold,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );

        if result.entries.is_empty() {
            let empty_message = if result.total > 0 {
                "搜索到了挂单，但明细没有取到；可以点打开市集查看，或重试一次。"
            } else if result.options.use_mods || result.options.use_values {
                "没有匹配挂单；同属性/数值可能过严，可关闭筛选后重试。"
            } else {
                "没搜到在线挂单，可以放宽底材/稀有度再试。"
            };
            draw_text(
                hdc,
                empty_message,
                RECT {
                    left: 22,
                    top: table_top + 42,
                    right: rect.right - 22,
                    bottom: table_top + 80,
                },
                rgb(251, 191, 36),
                self.fonts.normal,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            return;
        }

        let visible_entries = result
            .entries
            .iter()
            .skip(page * page_size)
            .take(page_size)
            .collect::<Vec<_>>();

        for (idx, entry) in visible_entries.iter().enumerate() {
            let top = table_top + 28 + idx as i32 * 27;
            let bg = if idx % 2 == 0 {
                rgb(18, 22, 28)
            } else {
                rgb(23, 27, 34)
            };
            let row_rect = RECT {
                left: 20,
                top,
                right: rect.right - 20,
                bottom: top + 27,
            };
            if idx == visible_entries.len() - 1 {
                rounded_rect(hdc, row_rect, bg, bg, 8);
            } else {
                fill(hdc, row_rect, bg);
            }
            draw_text(
                hdc,
                &entry.price,
                RECT {
                    left: 32,
                    top,
                    right: 180,
                    bottom: top + 27,
                },
                rgb(254, 243, 199),
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            draw_text(
                hdc,
                &entry.seller,
                RECT {
                    left: 190,
                    top,
                    right: 330,
                    bottom: top + 27,
                },
                rgb(219, 234, 254),
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            draw_text(
                hdc,
                &entry.item_name,
                RECT {
                    left: 340,
                    top,
                    right: rect.right - 72,
                    bottom: top + 27,
                },
                rgb(229, 231, 235),
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            // 私聊W按钮
            let w_btn_rect = RECT {
                left: rect.right - 70,
                top: top + 6,
                right: rect.right - 32,
                bottom: top + 22,
            };
            rounded_rect(hdc, w_btn_rect, rgb(22, 68, 52), rgb(52, 211, 153), 4);
            draw_text(
                hdc,
                "W",
                w_btn_rect,
                rgb(255, 255, 255),
                self.fonts.small,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
        }
    }

    unsafe fn metric_card(&self, hdc: HDC, card: MetricCard<'_>) {
        rounded_rect(
            hdc,
            RECT {
                left: card.x,
                top: card.y,
                right: card.x + card.w,
                bottom: card.y + 68,
            },
            rgb(19, 23, 29),
            rgb(43, 49, 60),
            10,
        );
        draw_text(
            hdc,
            card.label,
            RECT {
                left: card.x + 10,
                top: card.y + 8,
                right: card.x + card.w - 10,
                bottom: card.y + 26,
            },
            rgb(148, 163, 184),
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            card.value,
            RECT {
                left: card.x + 10,
                top: card.y + 29,
                right: card.x + card.w - 10,
                bottom: card.y + 64,
            },
            card.color,
            self.fonts.bold,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_END_ELLIPSIS,
        );
    }
}

unsafe fn fill(hdc: HDC, rect: RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, &rect, brush);
    DeleteObject(brush as _);
}

unsafe fn rounded_rect(hdc: HDC, rect: RECT, fill_color: u32, border_color: u32, radius: i32) {
    let brush = CreateSolidBrush(fill_color);
    let pen = CreatePen(PS_SOLID, 1, border_color);
    let old_brush = SelectObject(hdc, brush as _);
    let old_pen = SelectObject(hdc, pen as _);
    RoundRect(
        hdc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    DeleteObject(pen as _);
    DeleteObject(brush as _);
}

unsafe fn draw_text(hdc: HDC, text: &str, mut rect: RECT, color: u32, font: HFONT, flags: u32) {
    let text = wide(text);
    let old_font = SelectObject(hdc, font as _);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    DrawTextW(hdc, text.as_ptr(), -1, &mut rect, flags);
    SelectObject(hdc, old_font);
}

unsafe fn state_from_hwnd(hwnd: HWND) -> Option<&'static mut UiState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
    if ptr.is_null() { None } else { Some(&mut *ptr) }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let createstruct =
                lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
            let state_ptr = (*createstruct).lpCreateParams as *mut UiState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            (*state_ptr).hwnd = hwnd;
            1
        }
        WM_CREATE => {
            if let Some(state) = state_from_hwnd(hwnd) {
                state.init_shell_presence();
            }
            0
        }
        WM_SIZE => 0,
        WM_TIMER => {
            if wparam == TIMER_ID
                && let Some(state) = state_from_hwnd(hwnd)
            {
                state.handle_timer();
            }
            0
        }
        WM_HOTKEY => {
            if wparam as i32 == HOTKEY_PRICE_ID
                && let Some(state) = state_from_hwnd(hwnd)
            {
                start_price_query(state.event_tx.clone());
            }
            0
        }
        msg if msg == WM_TRAYICON => {
            if let Some(state) = state_from_hwnd(hwnd) {
                let tray_event = (lparam & 0xffff) as u32;
                match tray_event {
                    WM_LBUTTONUP | NIN_SELECT => state.show_from_tray(),
                    WM_RBUTTONUP | WM_CONTEXTMENU => state.show_tray_menu(),
                    _ => {}
                }
            }
            0
        }
        WM_COMMAND => {
            if let Some(state) = state_from_hwnd(hwnd) {
                state.handle_tray_command(wparam & 0xffff);
            }
            0
        }
        WM_KEYDOWN => {
            if let Some(state) = state_from_hwnd(hwnd)
                && state.handle_key_down(wparam as u32)
            {
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam as i16) as i32;
            let y = ((lparam >> 16) as i16) as i32;
            if let Some(state) = state_from_hwnd(hwnd)
                && state.handle_button_click(x, y)
            {
                return 0;
            }
            ReleaseCapture();
            SendMessageW(hwnd, 0x00A1, HTCAPTION as WPARAM, 0);
            0
        }
        WM_EXITSIZEMOVE => {
            if let Some(state) = state_from_hwnd(hwnd) {
                state.save_position();
            }
            0
        }
        WM_PAINT => {
            if let Some(state) = state_from_hwnd(hwnd) {
                state.paint();
                0
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_NCDESTROY => {
            KillTimer(hwnd, TIMER_ID);
            UnregisterHotKey(hwnd, HOTKEY_PRICE_ID);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
            if !ptr.is_null() {
                (*ptr).remove_tray_icon();
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(ptr));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn read_clipboard_text() -> Result<String> {
    let mut clipboard = Clipboard::new().context("打开剪贴板失败")?;
    clipboard.get_text().context("读取剪贴板文本失败")
}

fn copy_text_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().context("打开剪贴板失败")?;
    clipboard
        .set_text(text.to_string())
        .context("写入剪贴板失败")
}

fn looks_like_poe_item_text(text: &str) -> bool {
    let normalized = text.replace('：', ":");
    let has_item_marker = normalized.lines().any(|line| {
        let line = line.trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("rarity:")
            || lower.starts_with("item class:")
            || lower.starts_with("item level:")
            || line.starts_with("稀有度:")
            || line.starts_with("稀 有 度:")
            || line.starts_with("物品类别:")
            || line.starts_with("物品類別:")
            || line.starts_with("物品等级:")
            || line.starts_with("物品等級:")
    });
    has_item_marker && normalized.contains("--------")
}

fn auth_error_panel(message: &str) -> (String, Vec<String>, u32) {
    let category = if message.contains("没有保存") {
        "Cookie 未设置"
    } else if message.contains("过期") || message.contains("无效") {
        "Cookie 已过期"
    } else {
        "Cookie 认证失败"
    };
    (
        category.to_string(),
        vec![
            format!("类型: {category}"),
            format!("原因: {message}"),
            "建议: 重新登录国服市集，复制新的 POESESSID。".to_string(),
            "操作: 点击下方“设置Cookie”或“向导”。".to_string(),
        ],
        rgb(245, 158, 11),
    )
}

fn request_error_panel(item: &str, message: &str) -> (String, Vec<String>, u32) {
    let category = if message.contains("网络请求失败") {
        "网络连接失败"
    } else if message.contains("HTTP 429") {
        "请求过于频繁"
    } else if message.contains("HTTP 5") {
        "官方接口异常"
    } else if message.contains("接口返回不是 JSON") || message.contains("读取接口返回失败")
    {
        "接口返回异常"
    } else if message.contains("读取属性库失败") || message.contains("解析属性库失败")
    {
        "属性库读取失败"
    } else if message.contains("没有可用查询") || message.contains("trade search error") {
        "查询条件不可用"
    } else if message.contains("fetch") || message.contains("明细响应") {
        "挂单明细读取失败"
    } else {
        "查价请求失败"
    };

    let suggestion = match category {
        "网络连接失败" => "建议: 检查网络、代理、防火墙，确认官网能打开。",
        "请求过于频繁" => "建议: 稍等一会再查，减少连续查询。",
        "官方接口异常" => "建议: 稍后重试，或打开官网确认市集状态。",
        "接口返回异常" => "建议: 打开官网重新登录，再验证 Cookie。",
        "属性库读取失败" => "建议: 先关闭同属性筛选，或稍后重试。",
        "查询条件不可用" => "建议: 放宽同属性/数值筛选，或打开官网手动筛选。",
        "挂单明细读取失败" => "建议: 点打开官网查看，或稍后重试。",
        _ => "建议: 查看查询历史并导出诊断给维护者。",
    };

    (
        format!("{item} - {category}"),
        vec![
            format!("类型: {category}"),
            format!("原因: {message}"),
            suggestion.to_string(),
            "操作: 可点“历史”或“诊断”发送排查信息。".to_string(),
        ],
        rgb(239, 68, 68),
    )
}

fn start_cookie_validation(event_tx: Sender<OverlayEvent>) {
    let _ = event_tx.send(OverlayEvent::Message {
        title: "正在验证 Cookie".to_string(),
        lines: vec!["正在请求国服 trade2，请稍候。".to_string()],
        accent: rgb(56, 189, 248),
        timeout: Duration::from_secs(6),
    });
    thread::spawn(move || match validate_saved_cookie() {
        Ok(_) => {
            let _ = event_tx.send(OverlayEvent::Message {
                title: "Cookie 可用".to_string(),
                lines: vec!["POESESSID 验证通过，可以正常查价。".to_string()],
                accent: rgb(34, 197, 94),
                timeout: Duration::from_secs(8),
            });
        }
        Err(err) => {
            let message = err.to_string();
            let (title, lines, accent) = auth_error_panel(&message);
            let _ = event_tx.send(OverlayEvent::Message {
                title: format!("Cookie 验证失败 - {title}"),
                lines,
                accent,
                timeout: Duration::from_secs(16),
            });
        }
    });
}

fn start_price_query(event_tx: Sender<OverlayEvent>) {
    let text = match read_clipboard_text() {
        Ok(text) => text,
        Err(err) => {
            let _ = event_tx.send(OverlayEvent::Message {
                title: "读取剪贴板失败".to_string(),
                lines: vec![err.to_string()],
                accent: rgb(239, 68, 68),
                timeout: Duration::from_secs(8),
            });
            return;
        }
    };
    start_price_query_from_text(text, event_tx, true, QueryOptions::default());
}

fn start_price_query_from_text(
    text: String,
    event_tx: Sender<OverlayEvent>,
    show_invalid_feedback: bool,
    options: QueryOptions,
) {
    let parsed = parse_item_text(&text);
    if parsed.name.is_empty() && parsed.base_type.is_empty() {
        if show_invalid_feedback {
            let _ = event_tx.send(OverlayEvent::Message {
                title: "POE2 查价".to_string(),
                lines: vec![
                    "剪贴板不是物品文本".to_string(),
                    "先在游戏里悬停物品按 Ctrl+C".to_string(),
                    "复制后会自动查价，手动热键只用于重查。".to_string(),
                ],
                accent: rgb(245, 158, 11),
                timeout: Duration::from_secs(5),
            });
        }
        return;
    }

    start_price_query_from_parsed(parsed, options, event_tx);
}

fn start_price_query_from_parsed(
    parsed: ParsedItem,
    options: QueryOptions,
    event_tx: Sender<OverlayEvent>,
) {
    let target = if parsed.query_mode == "name" && !parsed.name.is_empty() {
        parsed.name.clone()
    } else {
        parsed.base_type.clone()
    };
    log(format!(
        "准备查价: {} ({})",
        target,
        if parsed.rarity_raw.is_empty() {
            "未知稀有度"
        } else {
            &parsed.rarity_raw
        }
    ));
    let _ = event_tx.send(OverlayEvent::Message {
        title: format!("查价中: {target}"),
        lines: vec![
            if options.use_mods {
                "正在按同属性请求国服市集...".to_string()
            } else {
                "正在直接请求国服市集...".to_string()
            },
            format!("第一次使用请先设置 Cookie。{}", support_hint()),
        ],
        accent: rgb(56, 189, 248),
        timeout: Duration::from_secs(8),
    });

    thread::spawn(
        move || match direct_trade_search(parsed.clone(), options.clone()) {
            Ok(result) => {
                log(format!(
                    "查价结果: {} | {}",
                    result.item.display,
                    result.summary.join("; ")
                ));
                append_history(HistoryEntry {
                    ts: unix_now(),
                    status: "ok".to_string(),
                    item: result.item.display.clone(),
                    base_type: result.item.base_type.clone(),
                    rarity: result.item.rarity_raw.clone(),
                    league: Some(result.league.clone()),
                    total: Some(result.total),
                    priced: result
                        .entries
                        .iter()
                        .map(|entry| entry.price.as_str())
                        .find(|price| !price.is_empty() && *price != "未标价")
                        .map(ToString::to_string),
                    message: None,
                    url: Some(result.url.clone()),
                    used_mods: result.options.use_mods,
                    used_values: result.options.use_values,
                });
                let timeout_seconds = load_config().settings.normalized().result_timeout_seconds;
                let _ = event_tx.send(OverlayEvent::Result {
                    result: Box::new(result),
                    accent: rgb(34, 197, 94),
                    timeout: Duration::from_secs(timeout_seconds),
                });
            }
            Err(TradeError::Auth(message)) => {
                log(format!("查价认证失败: {message}"));
                append_history(HistoryEntry {
                    ts: unix_now(),
                    status: "auth_failed".to_string(),
                    item: parsed.display.clone(),
                    base_type: parsed.base_type.clone(),
                    rarity: parsed.rarity_raw.clone(),
                    league: None,
                    total: None,
                    priced: None,
                    message: Some(message.clone()),
                    url: None,
                    used_mods: false,
                    used_values: false,
                });
                let (title, lines, accent) = auth_error_panel(&message);
                let _ = event_tx.send(OverlayEvent::Message {
                    title,
                    lines,
                    accent,
                    timeout: Duration::from_secs(16),
                });
            }
            Err(TradeError::Request(message)) => {
                log(format!("查价失败: {message}"));
                append_history(HistoryEntry {
                    ts: unix_now(),
                    status: "request_failed".to_string(),
                    item: parsed.display.clone(),
                    base_type: parsed.base_type.clone(),
                    rarity: parsed.rarity_raw.clone(),
                    league: None,
                    total: None,
                    priced: None,
                    message: Some(message.clone()),
                    url: None,
                    used_mods: options.use_mods,
                    used_values: options.use_values,
                });
                let (title, lines, accent) = request_error_panel(&parsed.display, &message);
                let _ = event_tx.send(OverlayEvent::Message {
                    title,
                    lines,
                    accent,
                    timeout: Duration::from_secs(16),
                });
            }
        },
    );
}

fn command_loop(action_tx: Sender<Action>) {
    let settings = load_config().settings.normalized();
    let manual_hotkey = manual_hotkey_label(&settings);
    println!();
    println!("POE2 国服查价桥 Rust 版已启动。");
    println!("用法: 游戏里悬停物品 Ctrl+C，工具会自动查价。");
    if manual_hotkey == "关闭" {
        println!("手动热键: 已关闭。");
    } else {
        println!("手动重查: {manual_hotkey}。");
    }
    println!("窗口命令: Enter=查剪贴板, o=打开国服市集, q=退出");
    println!();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let command = line.unwrap_or_default().trim().to_ascii_lowercase();
        let action = match command.as_str() {
            "q" | "quit" | "exit" => Action::Quit,
            "o" => Action::OpenHome,
            _ => Action::Price,
        };
        if action_tx.send(action).is_err() {
            return;
        }
    }
}

fn open_url(url: &str) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

fn open_path(path: &std::path::Path) {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn project_root_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        if exe_dir.join("set_cookie_gui.ps1").exists()
            || exe_dir.join("settings_gui.ps1").exists()
            || exe_dir.join("first_run_wizard.ps1").exists()
            || exe_dir.join("history_gui.ps1").exists()
            || exe_dir.join("assets").join("app.ico").exists()
        {
            return exe_dir.to_path_buf();
        }
        if let Some(target_dir) = exe_dir.parent()
            && let Some(source_root) = target_dir.parent()
            && source_root.join("set_cookie_gui.ps1").exists()
        {
            return source_root.to_path_buf();
        }
    }
    if let Ok(cwd) = std::env::current_dir()
        && cwd.join("set_cookie_gui.ps1").exists()
    {
        return cwd;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 使用统一脱敏规则原地清理诊断或支持包中的文本文件。
fn redact_file(path: &str) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("读取待脱敏文件失败: {path}"))?;
    fs::write(path, redact_runtime_text(&text))
        .with_context(|| format!("写入脱敏文件失败: {path}"))?;
    Ok(())
}

fn launch_cookie_setup() -> Result<()> {
    let root = project_root_dir();
    let script = root.join("set_cookie_gui.ps1");
    if !script.exists() {
        bail!("找不到 {}", script.display());
    }
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .arg("-Root")
        .arg(&root)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("启动 PowerShell Cookie 设置窗口失败")?;
    Ok(())
}

fn launch_settings_setup() -> Result<()> {
    let root = project_root_dir();
    let script = root.join("settings_gui.ps1");
    if !script.exists() {
        bail!("找不到 {}", script.display());
    }
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .arg("-Root")
        .arg(&root)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("启动 PowerShell 设置窗口失败")?;
    Ok(())
}

fn launch_first_run_wizard() -> Result<()> {
    let root = project_root_dir();
    let script = root.join("first_run_wizard.ps1");
    if !script.exists() {
        bail!("找不到 {}", script.display());
    }
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .arg("-Root")
        .arg(&root)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("启动 PowerShell 首次使用向导失败")?;
    Ok(())
}

fn launch_history_viewer() -> Result<()> {
    let root = project_root_dir();
    let script = root.join("history_gui.ps1");
    if !script.exists() {
        bail!("找不到 {}", script.display());
    }
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .arg("-Root")
        .arg(&root)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("启动 PowerShell 查询历史窗口失败")?;
    Ok(())
}

fn activate_existing_window() -> bool {
    unsafe {
        let class_name = wide(WINDOW_CLASS_NAME);
        let hwnd = FindWindowW(class_name.as_ptr(), null());
        if hwnd.is_null() {
            return false;
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        true
    }
}

fn activate_existing_window_retry() {
    for _ in 0..20 {
        if activate_existing_window() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn claim_single_instance() -> Result<Option<HANDLE>> {
    unsafe {
        let name = wide("Local\\POE2CNPriceBridgeRust");
        let handle = CreateMutexW(null_mut(), 1, name.as_ptr());
        if handle.is_null() {
            bail!(last_win_error("CreateMutexW failed"));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return Ok(None);
        }
        Ok(Some(handle))
    }
}

fn run_ui(
    event_tx: Sender<OverlayEvent>,
    event_rx: Receiver<OverlayEvent>,
    action_rx: Receiver<Action>,
) -> Result<()> {
    unsafe {
        let hinstance = GetModuleHandleW(null());
        let class_name = wide(WINDOW_CLASS_NAME);
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name.as_ptr(),
            hIcon: load_app_icon(32).0,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: 0 as HBRUSH,
            ..MaybeUninit::zeroed().assume_init()
        };
        if RegisterClassW(&wnd_class) == 0 {
            bail!(last_win_error("RegisterClassW failed"));
        }

        let state = Box::new(UiState::new(event_tx.clone(), event_rx, action_rx));
        let state_ptr = Box::into_raw(state);
        let title = wide(APP_DISPLAY_NAME);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            700,
            450,
            null_mut(),
            null_mut(),
            hinstance,
            state_ptr as _,
        );
        if hwnd.is_null() {
            drop(Box::from_raw(state_ptr));
            bail!(last_win_error("CreateWindowExW failed"));
        }

        if SetTimer(hwnd, TIMER_ID, 100, None) == 0 {
            eprintln!("SetTimer 失败，面板可能不会自动刷新。");
        }
        let manual_hotkey_line = if let Some(state) = state_from_hwnd(hwnd) {
            state.refresh_manual_hotkey();
            let label = manual_hotkey_label(&state.settings);
            if label == "关闭" {
                "手动查价热键已关闭。".to_string()
            } else {
                format!("{label} 可用于手动重查。")
            }
        } else {
            "手动查价热键按设置启用。".to_string()
        };

        let _ = event_tx.send(OverlayEvent::Message {
            title: format!("{APP_DISPLAY_NAME}已启动"),
            lines: vec![
                "游戏内 Ctrl+C 后自动查价。".to_string(),
                manual_hotkey_line,
                "后台托盘图标可打开面板、设置 Cookie、退出。".to_string(),
            ],
            accent: rgb(56, 189, 248),
            timeout: Duration::from_secs(7),
        });

        ShowWindow(hwnd, SW_SHOW);
        if load_config().cookie_dpapi.is_none() {
            log("未检测到 Cookie，自动打开首次使用向导。");
            if let Err(err) = launch_first_run_wizard() {
                log(format!("自动打开首次使用向导失败: {err}"));
            }
        }

        let mut msg = MaybeUninit::<MSG>::zeroed().assume_init();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

fn print_usage() {
    println!("{APP_DISPLAY_NAME} {APP_VERSION}");
    println!("  --set-cookie     保存 POESESSID/Cookie");
    println!("  --set-cookie-file PATH");
    println!("  --set-cookie-stdin 从标准输入验证并保存裸 POESESSID");
    println!("  --clear-cookie   清除已保存 Cookie");
    println!("  --validate-cookie 验证已保存 Cookie");
    println!("  --diagnostics [PATH] 导出诊断文件");
    println!("  --self-check [PATH] 导出客户自检报告");
    println!("  --check-update [PATH] 检查 GitHub Release 最新版本");
    println!("  --redact-file PATH 脱敏诊断或支持包文本");
}

fn main() -> Result<()> {
    install_panic_hook();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("{APP_DISPLAY_NAME} {APP_VERSION}");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--set-cookie") {
        return set_cookie_interactive();
    }
    if args.iter().any(|arg| arg == "--set-cookie-stdin") {
        return set_cookie_from_stdin();
    }
    if let Some(index) = args.iter().position(|arg| arg == "--set-cookie-file") {
        let Some(path) = args.get(index + 1) else {
            bail!("--set-cookie-file 缺少路径");
        };
        set_cookie_from_file(path)?;
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--clear-cookie") {
        clear_cookie()?;
        println!("已清除保存的 Cookie。");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--validate-cookie") {
        validate_saved_cookie()?;
        println!("Cookie 验证通过。");
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--diagnostics") {
        let target = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .map(PathBuf::from);
        let path = write_diagnostics(target)?;
        println!("诊断已导出: {}", path.display());
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--self-check") {
        let target = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .map(PathBuf::from);
        let path = write_self_check(target)?;
        println!("自检报告已导出: {}", path.display());
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--check-update") {
        let target = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .map(PathBuf::from);
        let path = write_update_check(target)?;
        println!("更新检查已导出: {}", path.display());
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--redact-file") {
        let Some(path) = args.get(index + 1) else {
            bail!("--redact-file 缺少路径");
        };
        redact_file(path)?;
        return Ok(());
    }
    let Some(instance_mutex) = claim_single_instance()? else {
        activate_existing_window_retry();
        println!("{APP_DISPLAY_NAME} 已在运行，已唤出原窗口。");
        return Ok(());
    };
    log(format!("{APP_DISPLAY_NAME} {APP_VERSION} 启动"));

    let (event_tx, event_rx) = mpsc::channel();
    let (action_tx, action_rx) = mpsc::channel();
    thread::spawn(move || command_loop(action_tx));
    let result = run_ui(event_tx, event_rx, action_rx);
    unsafe {
        CloseHandle(instance_mutex);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_cookie_inputs_without_leaking_extra_text() {
        assert_eq!(normalize_cookie_input("abc123"), "POESESSID=abc123");
        assert_eq!(
            normalize_cookie_input("Cookie: POESESSID=abc123; other=value"),
            "POESESSID=abc123; other=value"
        );
        assert_eq!(
            normalize_cookie_input("POESESSID=abc123\r\nfoo=bar"),
            "POESESSID=abc123 foo=bar"
        );
        assert_eq!(normalize_cookie_input("   "), "");
    }

    #[test]
    fn parses_only_bare_poesessid_from_stdin() {
        let secret = ["SYNTHETIC_", "LOGIN_", "7f3a91d2"].concat();
        assert_eq!(
            parse_poesessid_stdin(format!("{secret}\r\n")).unwrap(),
            secret
        );

        for invalid in [
            "",
            "Cookie: POESESSID=value",
            "POESESSID=value",
            "value; other=1",
            "value\r\ninjected",
            "value,other",
        ] {
            let error = parse_poesessid_stdin(invalid.to_string())
                .unwrap_err()
                .to_string();
            if !invalid.is_empty() {
                assert!(!error.contains(invalid), "错误信息不得回显 Secret");
            }
        }
        assert!(parse_poesessid_stdin("x".repeat(MAX_STDIN_POESESSID_BYTES + 1)).is_err());
    }

    #[test]
    fn stdin_reader_rejects_oversize_broken_pipe_and_unclosed_input() {
        let mut oversized = std::io::Cursor::new(vec![b'x'; MAX_STDIN_POESESSID_BYTES + 1]);
        assert!(read_poesessid_from_reader(&mut oversized).is_err());

        struct BrokenPipeReader;
        impl Read for BrokenPipeReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "synthetic pipe interruption",
                ))
            }
        }
        let error = read_poesessid_from_reader(&mut BrokenPipeReader)
            .unwrap_err()
            .to_string();
        assert!(error.contains("读取标准输入失败"));
        assert!(!error.contains("POESESSID="));

        let (hold_tx, hold_rx) = mpsc::channel::<()>();
        let error = receive_poesessid_with_timeout(
            move || {
                let _ = hold_rx.recv();
                Ok("late-value".to_string())
            },
            Duration::from_millis(30),
        )
        .unwrap_err()
        .to_string();
        assert_eq!(error, "读取标准输入超时");
        drop(hold_tx);
    }

    #[test]
    fn candidate_failures_distinguish_network_and_authentication() {
        assert!(matches!(
            classify_candidate_failure(None, "offline"),
            TradeError::Request(message) if message.contains("网络请求失败")
        ));
        assert!(matches!(
            classify_candidate_failure(Some(401), "ignored"),
            TradeError::Auth(message) if message.contains("POESESSID 无效/过期")
        ));
        assert!(matches!(
            classify_candidate_failure(Some(503), "unavailable"),
            TradeError::Request(message) if message.contains("HTTP 503")
        ));
    }
    #[test]
    fn manual_hotkey_is_normalized_and_can_be_disabled() {
        assert_eq!(normalize_manual_hotkey("f9"), "F9");
        assert_eq!(normalize_manual_hotkey("ctrl alt d"), "Ctrl+Alt+D");
        assert_eq!(normalize_manual_hotkey("Ctrl+Alt+D"), "Ctrl+Alt+D");
        assert_eq!(normalize_manual_hotkey("mouse4"), "F8");
        assert_eq!(normalize_manual_hotkey("off"), "关闭");
        assert_eq!(normalize_manual_hotkey("关闭"), "关闭");
    }

    #[test]
    fn compares_release_versions_numerically() {
        assert!(is_newer_version("v0.10.0", "0.2.0"));
        assert!(is_newer_version("v1.0.0", "0.99.9"));
        assert!(!is_newer_version("v0.2.0", "0.2.0"));
        assert!(!is_newer_version("v0.1.9", "0.2.0"));
    }

    #[test]
    fn parses_cn_rare_item_copy_text() {
        let text = r#"
稀有度: 稀有
苦痛 导航
引路石（15阶）
--------
物品类别: 地图
物品等级: 80
--------
+20 最大生命
+12% 火焰抗性
"#;

        let parsed = parse_item_text(text);
        assert_eq!(parsed.rarity, "rare");
        assert_eq!(parsed.name, "苦痛 导航");
        assert_eq!(parsed.base_type, "引路石（15阶）");
        assert_eq!(parsed.query_mode, "type");
        assert_eq!(parsed.item_level, Some(80));
        assert!(
            parsed
                .mods
                .iter()
                .any(|item_mod| item_mod.value == Some(20.0))
        );
        assert!(looks_like_poe_item_text(text));
    }

    #[test]
    fn parses_unique_item_as_name_query() {
        let text = r#"
稀有度: 传奇
猎首
重革腰带
--------
物品等级: 84
"#;

        let parsed = parse_item_text(text);
        assert_eq!(parsed.rarity, "unique");
        assert_eq!(parsed.name, "猎首");
        assert_eq!(parsed.base_type, "重革腰带");
        assert_eq!(parsed.query_mode, "name");
        assert_eq!(parsed.display, "猎首");
    }

    #[test]
    fn trade_payload_keeps_required_trade2_shape() {
        let mut parsed = ParsedItem {
            rarity_raw: "稀有".to_string(),
            rarity: "rare".to_string(),
            name: "苦痛 导航".to_string(),
            base_type: "引路石（15阶）".to_string(),
            item_class: "地图".to_string(),
            item_level: Some(80),
            query_mode: "type".to_string(),
            display: "苦痛 导航".to_string(),
            mods: Vec::new(),
        };
        parsed.mods.push(ParsedMod {
            text: "+20 最大生命".to_string(),
            pattern: compact_stat_pattern("+20 最大生命"),
            value: Some(20.0),
            stat_id: Some("explicit.stat_3299347043".to_string()),
            stat_text: Some("# 最大生命".to_string()),
        });

        let payload = build_trade_payload(
            &parsed,
            &QueryOptions {
                use_mods: true,
                use_values: true,
                selected_mod_patterns: None,
            },
            DEFAULT_STATUS,
            true,
            true,
            true,
        );

        assert_eq!(payload["query"]["status"]["option"], DEFAULT_STATUS);
        assert_eq!(payload["query"]["type"], "引路石（15阶）");
        assert_eq!(
            payload["query"]["filters"]["type_filters"]["filters"]["rarity"]["option"],
            "rare"
        );
        assert_eq!(payload["query"]["stats"][0]["type"], "and");
        assert_eq!(
            payload["query"]["stats"][0]["filters"][0]["id"],
            "explicit.stat_3299347043"
        );
        assert_eq!(
            payload["query"]["stats"][0]["filters"][0]["value"]["min"],
            20.0
        );
        assert_eq!(payload["sort"]["price"], "asc");
    }

    #[test]
    fn redacts_cookie_headers_json_values_and_known_secret() {
        let secret = ["SYNTHETIC_", "POESESSID_", "7f3a91d2"].concat();
        let text = format!(
            "POESESSID={secret}\nCookie: foo=bar; POESESSID={secret}\n{{\"POESESSID\":\"{secret}\"}}\nknown={secret}"
        );
        let known_cookie = format!("POESESSID={secret}; foo=bar");
        let redacted = redact_sensitive_text(&text, Some(&known_cookie));

        assert!(!redacted.contains(&secret));
        assert!(redacted.contains(REDACTED_SECRET));
        assert!(redacted.contains("POESESSID=[REDACTED]"));
        assert!(redacted.contains("Cookie: [REDACTED]"));
        assert!(redacted.contains("\"POESESSID\":\"[REDACTED]\""));
    }

    #[test]
    fn trade_endpoints_are_restricted_to_cn_official_trade2() {
        let search = trade_search_url("永久");
        let fetch = fetch_url(&["item-id".to_string()], "query-id");
        let stats = trade_stats_url();

        for url in [TRADE_HOME.to_string(), search, fetch, stats] {
            assert!(
                is_allowed_trade_endpoint(&url),
                "unexpected endpoint: {url}"
            );
        }
        assert!(!is_allowed_trade_endpoint(&trade_result_url(
            "永久", "query-id"
        )));
        assert!(!is_allowed_trade_endpoint(
            "https://www.pathofexile.com/trade2"
        ));
        assert!(!is_allowed_trade_endpoint(
            "https://poe.game.qq.com.evil.example/api/trade2/search/poe2/test"
        ));
        assert!(!is_allowed_trade_endpoint(
            "https://poe.game.qq.com/api/trade/search/poe2/test"
        ));
    }
}
