#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

mod currency;
mod overlay;

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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, LocalFree,
    POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{HBRUSH, InvalidateRect};
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, RegisterHotKey, ReleaseCapture, TME_LEAVE, TRACKMOUSEEVENT,
    TrackMouseEvent, UnregisterHotKey,
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
    GetWindowRect, HICON, HTCAPTION, HWND_TOPMOST, IDC_ARROW, IDI_APPLICATION, IMAGE_ICON,
    KillTimer, LR_LOADFROMFILE, LoadCursorW, LoadIconW, LoadImageW, MF_SEPARATOR, MF_STRING, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SendMessageW, SetForegroundWindow,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_BOTTOMALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_CONTEXTMENU, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_HOTKEY, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_NULL,
    WM_PAINT, WM_RBUTTONUP, WM_SIZE, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::overlay::interaction::OverlayInteraction;
use crate::overlay::layout;
use crate::overlay::layout::{
    LayoutPlan, OverlayLayout, compute_item_detail_lines, visible_row_count,
};
use crate::overlay::model::{
    Fonts, InputContext, OverlayEvent, OverlayShowMode, OverlayView, QueryState, UiButton,
    ViewKind, WindowPos,
};
use crate::overlay::render::OverlayRenderer;

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
const DEFAULT_RESULT_TIMEOUT_SECONDS: u64 = 30;
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

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
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
    #[serde(default)]
    filter_rules: FilterRulesConfig,
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
            filter_rules: FilterRulesConfig::default(),
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
         filter_rules_enabled: {}\n\
         filter_rules_count: {}\n\
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
        settings.filter_rules.enabled,
        settings.filter_rules.rules.len(),
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
    pub quality: Option<u32>,
    pub required_level: Option<u32>,
    pub physical_damage_min: Option<f64>,
    pub physical_damage_max: Option<f64>,
    pub fire_damage_min: Option<f64>,
    pub fire_damage_max: Option<f64>,
    pub cold_damage_min: Option<f64>,
    pub cold_damage_max: Option<f64>,
    pub lightning_damage_min: Option<f64>,
    pub lightning_damage_max: Option<f64>,
    pub chaos_damage_min: Option<f64>,
    pub chaos_damage_max: Option<f64>,
    pub critical_strike_chance: Option<f64>,
    pub attacks_per_second: Option<f64>,
    pub armour: Option<u32>,
    pub evasion: Option<u32>,
    pub energy_shield: Option<u32>,
    pub sockets: Option<String>,
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
            quality: None,
            required_level: None,
            physical_damage_min: None,
            physical_damage_max: None,
            fire_damage_min: None,
            fire_damage_max: None,
            cold_damage_min: None,
            cold_damage_max: None,
            lightning_damage_min: None,
            lightning_damage_max: None,
            chaos_damage_min: None,
            chaos_damage_max: None,
            critical_strike_chance: None,
            attacks_per_second: None,
            armour: None,
            evasion: None,
            energy_shield: None,
            sockets: None,
        }
    }
}

#[allow(dead_code)]
impl ParsedItem {
    /// 物理 DPS = (物理伤害下限 + 物理伤害上限) / 2 * 每秒攻击次数
    pub fn physical_dps(&self) -> Option<f64> {
        let min = self.physical_damage_min?;
        let max = self.physical_damage_max?;
        let aps = self.attacks_per_second?;
        Some((min + max) / 2.0 * aps)
    }

    /// 元素 DPS = 所有元素伤害平均值之和 * 每秒攻击次数
    pub fn elemental_dps(&self) -> Option<f64> {
        let aps = self.attacks_per_second?;
        let mut total = 0.0;
        for (min, max) in &[
            (self.fire_damage_min, self.fire_damage_max),
            (self.cold_damage_min, self.cold_damage_max),
            (self.lightning_damage_min, self.lightning_damage_max),
            (self.chaos_damage_min, self.chaos_damage_max),
        ] {
            if let (Some(min), Some(max)) = (min, max) {
                total += (min + max) / 2.0;
            }
        }
        if total == 0.0 {
            return None;
        }
        Some(total * aps)
    }

    /// 总 DPS = 物理 DPS + 元素 DPS
    pub fn total_dps(&self) -> Option<f64> {
        match (self.physical_dps(), self.elemental_dps()) {
            (Some(p), Some(e)) => Some(p + e),
            (Some(p), None) => Some(p),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        }
    }

    /// 是否为武器（有伤害范围或攻击速度的物品）
    pub fn is_weapon(&self) -> bool {
        self.physical_damage_min.is_some() || self.attacks_per_second.is_some()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TradeEntry {
    pub price: String,
    pub seller: String,
    pub item_name: String,
    // 新增价格字段
    pub price_amount: Option<f64>,
    pub price_currency: Option<String>,
    // 新增物品字段
    pub base_type: Option<String>,
    pub item_level: Option<u32>,
    // 新增卖家字段
    pub online: Option<bool>,
    pub indexed_time: Option<String>,
    // 私聊文本
    pub whisper_text: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for TradeEntry {
    fn default() -> Self {
        Self {
            price: String::new(),
            seller: String::new(),
            item_name: String::new(),
            price_amount: None,
            price_currency: None,
            base_type: None,
            item_level: None,
            online: None,
            indexed_time: None,
            whisper_text: None,
        }
    }
}

/// 排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SortOrder {
    PriceAsc,
    PriceDesc,
    ItemLevelAsc,
    ItemLevelDesc,
    IndexedTimeAsc,  // 最早上架
    IndexedTimeDesc, // 最新上架
    OnlineFirst,
}

#[allow(dead_code)]
impl TradeEntry {
    /// 按价格数值比较（处理 None 的情况，None 排在最后）
    fn compare_price(a: &TradeEntry, b: &TradeEntry) -> std::cmp::Ordering {
        match (a.price_amount, b.price_amount) {
            (Some(pa), Some(pb)) => pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// 按物品等级比较
    fn compare_item_level(a: &TradeEntry, b: &TradeEntry) -> std::cmp::Ordering {
        match (a.item_level, b.item_level) {
            (Some(la), Some(lb)) => lb.cmp(&la),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// 按上架时间比较（更新更靠前）
    fn compare_indexed_time(a: &TradeEntry, b: &TradeEntry) -> std::cmp::Ordering {
        match (&a.indexed_time, &b.indexed_time) {
            (Some(ta), Some(tb)) => tb.cmp(ta),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// 在线优先
    fn compare_online(a: &TradeEntry, b: &TradeEntry) -> std::cmp::Ordering {
        match (a.online, b.online) {
            (Some(true), Some(false)) => std::cmp::Ordering::Less,
            (Some(false), Some(true)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    }
}

/// 对挂单列表排序
#[allow(dead_code)]
pub(crate) fn sort_entries(entries: &mut [TradeEntry], order: SortOrder) {
    match order {
        SortOrder::PriceAsc => entries.sort_by(TradeEntry::compare_price),
        SortOrder::PriceDesc => entries.sort_by(|a, b| TradeEntry::compare_price(b, a)),
        SortOrder::ItemLevelAsc => entries.sort_by(|a, b| TradeEntry::compare_item_level(b, a)),
        SortOrder::ItemLevelDesc => entries.sort_by(TradeEntry::compare_item_level),
        SortOrder::IndexedTimeAsc => entries.sort_by(TradeEntry::compare_indexed_time),
        SortOrder::IndexedTimeDesc => {
            entries.sort_by(|a, b| TradeEntry::compare_indexed_time(b, a))
        }
        SortOrder::OnlineFirst => entries.sort_by(|a, b| {
            TradeEntry::compare_online(a, b).then_with(|| TradeEntry::compare_price(a, b))
        }),
    }
}

/// 返回当前页面上可见的挂单在原始 entries 中的索引
/// 排序后分页，返回 (原始索引, TradeEntry引用) 的列表
pub(crate) fn visible_listing_indices(
    entries: &[TradeEntry],
    sort: SortOrder,
    page: usize,
    page_size: usize,
) -> Vec<(usize, &TradeEntry)> {
    let mut indexed: Vec<(usize, &TradeEntry)> = entries.iter().enumerate().collect();

    match sort {
        SortOrder::PriceAsc => indexed.sort_by(|a, b| TradeEntry::compare_price(a.1, b.1)),
        SortOrder::PriceDesc => indexed.sort_by(|a, b| TradeEntry::compare_price(b.1, a.1)),
        SortOrder::ItemLevelAsc => indexed.sort_by(|a, b| TradeEntry::compare_item_level(b.1, a.1)),
        SortOrder::ItemLevelDesc => {
            indexed.sort_by(|a, b| TradeEntry::compare_item_level(a.1, b.1))
        }
        SortOrder::IndexedTimeAsc => {
            indexed.sort_by(|a, b| TradeEntry::compare_indexed_time(a.1, b.1))
        }
        SortOrder::IndexedTimeDesc => {
            indexed.sort_by(|a, b| TradeEntry::compare_indexed_time(b.1, a.1))
        }
        SortOrder::OnlineFirst => indexed.sort_by(|a, b| {
            TradeEntry::compare_online(a.1, b.1).then_with(|| TradeEntry::compare_price(a.1, b.1))
        }),
    }

    // 防御性分页，避免越界 panic
    let start = page.saturating_mul(page_size);
    if start >= indexed.len() || page_size == 0 {
        return Vec::new();
    }
    let end = (start + page_size).min(indexed.len());
    // 使用 get 替代直接切片，双重保险
    indexed.get(start..end).unwrap_or(&[]).to_vec()
}

/// 将 ISO 8601 时间字符串转换为易读文本
#[allow(dead_code)]
fn relative_time(iso_time: &str) -> String {
    let cleaned = iso_time.replace('T', " ").replace('Z', "");
    if let Some(dot_pos) = cleaned.find('.') {
        cleaned[..dot_pos].to_string()
    } else {
        cleaned.chars().take(19).collect()
    }
}

/// 将 ISO 8601 时间字符串转换为相对时间文本
/// 如 "2024-01-15T10:30:00Z" → "2小时" / "3天" / "1月"
pub(crate) fn relative_time_ago(iso_time: &str) -> String {
    // 解析 ISO 8601 格式
    let cleaned = iso_time.replace('T', " ").replace('Z', "");
    let time_str: String = cleaned.chars().take(19).collect(); // "YYYY-MM-DD HH:MM:SS"

    if time_str.len() < 19 {
        return cleaned;
    }

    // 解析年月日时分秒
    let year: i32 = time_str[0..4].parse().unwrap_or(0);
    let month: u32 = time_str[5..7].parse().unwrap_or(1);
    let day: u32 = time_str[8..10].parse().unwrap_or(1);
    let hour: u32 = time_str[11..13].parse().unwrap_or(0);
    let min: u32 = time_str[14..16].parse().unwrap_or(0);

    // 使用 UTC 时间（因为没有时区信息，假设为 UTC）
    // 获取当前 UTC 时间
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now_secs = now.as_secs() as i64;

    // 计算目标时间从 epoch 起的天数
    let target_days = days_since_epoch(year, month, day);
    let total_secs = target_days * 86400 + hour as i64 * 3600 + min as i64 * 60;
    let elapsed = now_secs - total_secs;

    if elapsed < 0 {
        return "刚刚".to_string();
    }
    if elapsed < 60 {
        return "刚刚".to_string();
    }
    if elapsed < 3600 {
        return format!("{}分钟", elapsed / 60);
    }
    if elapsed < 86400 {
        return format!("{}小时", elapsed / 3600);
    }
    if elapsed < 2592000 {
        return format!("{}天", elapsed / 86400);
    }
    if elapsed < 31536000 {
        return format!("{}月", elapsed / 2592000);
    }
    format!("{}年", elapsed / 31536000)
}

/// 计算从 1970-01-01 到指定日期的天数
fn days_since_epoch(y: i32, m: u32, d: u32) -> i64 {
    let mut days = 0i64;
    for year in 1970..y {
        days += if is_leap(year as i64) { 366 } else { 365 };
    }
    let month_days: [i64; 12] = if is_leap(y as i64) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for month in 1..m {
        days += month_days[(month - 1) as usize];
    }
    days + (d - 1) as i64
}

#[allow(dead_code)]
fn chrono_like_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days_since_epoch = secs / 86400;
    let mut y = 1970i64;
    let mut d = days_since_epoch as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for (i, md) in month_days.iter().enumerate() {
        if d < *md {
            m = i + 1;
            break;
        }
        d -= *md;
    }
    format!("{y:04}-{m:02}-{:02}", d + 1)
}

#[allow(dead_code)]
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
    value_tier: ItemValueTier,
}

/// 物品价值等级
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ItemValueTier {
    /// 神装 / 极高价值
    Legendary,
    /// 高价值
    High,
    /// 中等价值
    Medium,
    /// 普通 / 一般
    Normal,
    /// 低价值 / 垃圾
    Junk,
    /// 未估价 / 无法判断
    #[default]
    Unknown,
}

impl ItemValueTier {
    /// 价值等级对应的颜色（RGB）
    fn color(&self) -> u32 {
        match self {
            ItemValueTier::Legendary => rgb(251, 191, 36),
            ItemValueTier::High => rgb(244, 114, 182),
            ItemValueTier::Medium => rgb(134, 239, 172),
            ItemValueTier::Normal => rgb(147, 197, 253),
            ItemValueTier::Junk => rgb(148, 163, 184),
            ItemValueTier::Unknown => rgb(156, 163, 175),
        }
    }
}

/// 单条筛选规则
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterRule {
    /// 规则名称
    name: String,
    /// 匹配的稀有度（空列表表示全部匹配）
    rarities: Vec<String>,
    /// 匹配的物品类别（空列表表示全部匹配）
    item_classes: Vec<String>,
    /// 最低价格（混沌石等价，0 表示不限制）
    min_price_chaos: f64,
    /// 最高价格（0 表示不限制）
    max_price_chaos: f64,
    /// 满足条件后判定的价值等级
    tier: ItemValueTier,
    /// 是否启用
    enabled: bool,
}

impl Default for FilterRule {
    fn default() -> Self {
        Self {
            name: "新规则".to_string(),
            rarities: Vec::new(),
            item_classes: Vec::new(),
            min_price_chaos: 0.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Normal,
            enabled: true,
        }
    }
}

/// 筛选规则集合
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterRulesConfig {
    /// 是否启用筛选规则
    enabled: bool,
    /// 规则列表（按顺序匹配，第一条匹配的生效）
    rules: Vec<FilterRule>,
    /// 默认价值等级（无规则匹配时使用）
    default_tier: ItemValueTier,
}

impl Default for FilterRulesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: default_filter_rules(),
            default_tier: ItemValueTier::Normal,
        }
    }
}

/// 默认筛选规则模板
fn default_filter_rules() -> Vec<FilterRule> {
    vec![
        FilterRule {
            name: "传奇装备-高价值".to_string(),
            rarities: vec!["unique".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 50.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::High,
            enabled: true,
        },
        FilterRule {
            name: "传奇装备-普通".to_string(),
            rarities: vec!["unique".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 0.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Medium,
            enabled: true,
        },
        FilterRule {
            name: "稀有装备-高价值".to_string(),
            rarities: vec!["rare".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 20.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::High,
            enabled: true,
        },
        FilterRule {
            name: "稀有装备-中等".to_string(),
            rarities: vec!["rare".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 5.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Medium,
            enabled: true,
        },
        FilterRule {
            name: "稀有装备-垃圾".to_string(),
            rarities: vec!["rare".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 0.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Junk,
            enabled: true,
        },
        FilterRule {
            name: "魔法装备-垃圾".to_string(),
            rarities: vec!["magic".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 10.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Medium,
            enabled: true,
        },
        FilterRule {
            name: "魔法装备-普通".to_string(),
            rarities: vec!["magic".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 0.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Junk,
            enabled: true,
        },
        FilterRule {
            name: "普通装备-垃圾".to_string(),
            rarities: vec!["normal".to_string()],
            item_classes: Vec::new(),
            min_price_chaos: 0.0,
            max_price_chaos: 0.0,
            tier: ItemValueTier::Junk,
            enabled: true,
        },
    ]
}

/// 解析价格字符串为混沌石等价数量
/// 支持格式："10 chaos"、"5 divine"、"1 chaos" 等
fn parse_price_to_chaos(price_str: &str) -> Option<f64> {
    let price_str = price_str.trim();
    if price_str.is_empty() || price_str == "未标价" {
        return None;
    }

    let mut currency = String::new();

    let chars: Vec<char> = price_str.chars().collect();
    let mut i = 0;

    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }

    let mut num_str = String::new();
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',') {
        num_str.push(chars[i]);
        i += 1;
    }

    num_str = num_str.replace(',', "");
    let amount: f64 = match num_str.parse() {
        Ok(val) => val,
        Err(_) => return None,
    };

    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }

    while i < chars.len() && !chars[i].is_whitespace() {
        currency.push(chars[i]);
        i += 1;
    }

    if currency.is_empty() {
        return Some(amount);
    }

    let currency_lower = currency.to_ascii_lowercase();
    let chaos_value = match currency_lower.as_str() {
        "chaos" | "混沌" | "混沌石" | "c" => 1.0,
        "divine" | "div" | "神圣" | "神圣石" | "d" => 100.0,
        "exalted" | "ex" | "崇高" | "崇高石" | "e" => 50.0,
        "fusing" | "链结石" | "链" => 0.5,
        "alchemy" | "点金石" | "点金" => 0.3,
        "scouring" | "洗点" | "洗点石" => 0.2,
        "chromatic" | "幻色" | "幻色石" => 0.1,
        "chance" | "机会" | "机会石" => 0.1,
        "jeweller" | "工匠" | "工匠石" => 0.1,
        _ => 1.0,
    };

    Some(amount * chaos_value)
}

/// 从挂单列表中获取第一个有效价格（混沌石等价）
fn first_price_in_chaos(entries: &[TradeEntry]) -> Option<f64> {
    for entry in entries {
        if let Some(value) = parse_price_to_chaos(&entry.price) {
            if value > 0.0 {
                return Some(value);
            }
        }
    }
    None
}

/// 判断物品是否匹配某条规则
fn rule_matches(rule: &FilterRule, item: &ParsedItem, price_chaos: Option<f64>) -> bool {
    if !rule.enabled {
        return false;
    }

    if !rule.rarities.is_empty() {
        let rarity_matched = rule
            .rarities
            .iter()
            .any(|r| r.eq_ignore_ascii_case(&item.rarity));
        if !rarity_matched {
            return false;
        }
    }

    if !rule.item_classes.is_empty() {
        let class_matched = rule
            .item_classes
            .iter()
            .any(|c| item.item_class.contains(c));
        if !class_matched {
            return false;
        }
    }

    if rule.min_price_chaos > 0.0 || rule.max_price_chaos > 0.0 {
        let price = match price_chaos {
            Some(p) => p,
            None => return false,
        };
        if rule.min_price_chaos > 0.0 && price < rule.min_price_chaos {
            return false;
        }
        if rule.max_price_chaos > 0.0 && price > rule.max_price_chaos {
            return false;
        }
    }

    true
}

/// 根据筛选规则评估物品价值等级
fn evaluate_item_value(
    item: &ParsedItem,
    entries: &[TradeEntry],
    config: &FilterRulesConfig,
) -> ItemValueTier {
    if !config.enabled {
        return ItemValueTier::Unknown;
    }

    let price_chaos = first_price_in_chaos(entries);

    for rule in &config.rules {
        if rule_matches(rule, item, price_chaos) {
            return rule.tier;
        }
    }

    config.default_tier
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

fn parse_damage_range(line: &str) -> (Option<f64>, Option<f64>) {
    // 从行中提取两个数字，如 "10-20" 或 "10~20"
    let numbers: Vec<f64> = line
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if numbers.len() >= 2 {
        (Some(numbers[0]), Some(numbers[1]))
    } else {
        (None, None)
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

    // 解析物品属性（品质、伤害、防御、插槽等）
    for line in &lines {
        let lower = line.to_ascii_lowercase();

        // 品质
        if lower.contains("品质") || lower.contains("quality") {
            if let Some(val) = first_number(line) {
                parsed.quality = Some(val as u32);
            }
        }
        // 需求等级
        else if lower.contains("需求")
            || lower.contains("需要等级")
            || lower.contains("等级需求")
            || lower.contains("requires level")
        {
            if let Some(pos) = lower.find("等级") {
                let after = &line[pos + 6..]; // 跳过"等级"两个字 (UTF-8: 3 bytes each)
                parsed.required_level = first_number(after).map(|n| n as u32);
            } else if let Some(pos) = lower.find("level") {
                let after = &line[pos + 5..];
                parsed.required_level = first_number(after).map(|n| n as u32);
            } else {
                parsed.required_level = first_number(line).map(|n| n as u32);
            }
        }
        // 物理伤害 (不包含 "元素" 或 "火焰/冰霜/闪电/混沌")
        else if (lower.contains("物理伤害") || lower.contains("physical damage"))
            && !lower.contains("元素")
        {
            let (min, max) = parse_damage_range(line);
            parsed.physical_damage_min = min;
            parsed.physical_damage_max = max;
        }
        // 火焰伤害
        else if lower.contains("火焰伤害") || lower.contains("fire damage") {
            let (min, max) = parse_damage_range(line);
            parsed.fire_damage_min = min;
            parsed.fire_damage_max = max;
        }
        // 冰霜伤害
        else if lower.contains("冰霜伤害") || lower.contains("cold damage") {
            let (min, max) = parse_damage_range(line);
            parsed.cold_damage_min = min;
            parsed.cold_damage_max = max;
        }
        // 闪电伤害
        else if lower.contains("闪电伤害") || lower.contains("lightning damage") {
            let (min, max) = parse_damage_range(line);
            parsed.lightning_damage_min = min;
            parsed.lightning_damage_max = max;
        }
        // 混沌伤害
        else if lower.contains("混沌伤害") || lower.contains("chaos damage") {
            let (min, max) = parse_damage_range(line);
            parsed.chaos_damage_min = min;
            parsed.chaos_damage_max = max;
        }
        // 暴击率
        else if lower.contains("暴击率") || lower.contains("critical strike chance") {
            parsed.critical_strike_chance = first_number(line);
        }
        // 攻击速度
        else if lower.contains("每秒攻击次数")
            || lower.contains("攻击速度")
            || lower.contains("attacks per second")
        {
            parsed.attacks_per_second = first_number(line);
        }
        // 护甲
        else if lower.contains("护甲") || lower.contains("armour") || lower.contains("armor") {
            parsed.armour = line
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok();
        }
        // 闪避
        else if lower.contains("闪避") || lower.contains("evasion") {
            parsed.evasion = line
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok();
        }
        // 能量护盾
        else if lower.contains("能量护盾") || lower.contains("energy shield") {
            parsed.energy_shield = line
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok();
        }
        // 插槽
        else if lower.starts_with("插槽")
            || lower.starts_with("孔")
            || lower.starts_with("sockets")
        {
            parsed.sockets = Some(value_after_colon(line));
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
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
static UI_HWND: OnceLock<isize> = OnceLock::new();
static NEXT_QUERY_ID: AtomicU64 = AtomicU64::new(0);

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .expect("build reqwest client")
    })
}

fn load_stat_defs() -> Result<&'static Vec<StatDef>, TradeError> {
    if let Some(defs) = STAT_DEFS.get() {
        return Ok(defs);
    }
    let client = get_http_client();
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
    let amount_str = price
        .get("amount")
        .or_else(|| price.get("value"))
        .map(scalar_text)
        .unwrap_or_default();
    let currency = price
        .get("currency")
        .or_else(|| price.get("type"))
        .map(scalar_text)
        .unwrap_or_default();
    if amount_str.is_empty() || currency.is_empty() {
        return "未标价".to_string();
    }
    if let Ok(amount) = amount_str.parse::<f64>() {
        currency::format_price_zh(amount, &currency)
    } else {
        format!("{amount_str} {currency}")
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
    let priced: Vec<&TradeEntry> = entries
        .iter()
        .filter(|entry| !entry.price.is_empty() && entry.price != "未标价")
        .collect();
    if priced.is_empty() {
        return vec!["没有可读标价".to_string()];
    }

    // 按价格数值排序
    let mut sorted = priced.clone();
    sorted.sort_by(|a, b| TradeEntry::compare_price(a, b));

    let min_price = &sorted[0].price;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for entry in &sorted {
        *counts.entry(entry.price.clone()).or_insert(0) += 1;
    }
    let mut common: Vec<(String, usize)> = counts.into_iter().collect();
    common.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let dist = common
        .into_iter()
        .take(4)
        .map(|(price, count)| format!("{price} x{count}"))
        .collect::<Vec<_>>()
        .join(" / ");
    vec![format!("最低: {min_price}"), format!("分布: {dist}")]
}

fn direct_trade_search(
    mut parsed: ParsedItem,
    options: QueryOptions,
) -> Result<TradeResult, TradeError> {
    let settings = load_config().settings.normalized();
    let client = get_http_client();
    let cookie = load_cookie()
        .map_err(|err| TradeError::Auth(format!("读取 POESESSID 失败: {err}")))?
        .ok_or_else(|| TradeError::Auth(format!("没有保存 POESESSID。{}", support_hint())))?;
    if options.use_mods {
        resolve_item_mods(&mut parsed)?;
    }
    let payloads = build_payload_variants(&parsed, &options);
    let mut failures = Vec::new();

    for league in settings.leagues() {
        for (index, payload) in payloads.iter().enumerate() {
            let search_data = match request_json_with_cookie(
                client,
                &trade_search_url(&league),
                Method::POST,
                Some(payload),
                &cookie,
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
                    match request_json_with_cookie(
                        client,
                        &fetch_url(chunk, &query_id),
                        Method::GET,
                        None,
                        &cookie,
                    ) {
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
                                    let item = row.get("item");

                                    // 价格解析
                                    let price = price_text(listing.and_then(|l| l.get("price")));
                                    let price_amount = listing
                                        .and_then(|l| l.get("price"))
                                        .and_then(|p| p.get("amount"))
                                        .or_else(|| {
                                            listing
                                                .and_then(|l| l.get("price"))
                                                .and_then(|p| p.get("value"))
                                        })
                                        .and_then(|v| {
                                            v.as_f64().or_else(|| {
                                                v.as_str().and_then(|s| s.parse::<f64>().ok())
                                            })
                                        });
                                    let price_currency = listing
                                        .and_then(|l| l.get("price"))
                                        .and_then(|p| p.get("currency"))
                                        .or_else(|| {
                                            listing
                                                .and_then(|l| l.get("price"))
                                                .and_then(|p| p.get("type"))
                                        })
                                        .and_then(|v| v.as_str().map(String::from));

                                    // 卖家解析
                                    let seller = seller_text(listing);

                                    // 物品名称
                                    let item_name = item_text(item);

                                    // 基底类型
                                    let base_type = item
                                        .and_then(|i| i.get("typeLine"))
                                        .or_else(|| item.and_then(|i| i.get("baseType")))
                                        .and_then(|v| v.as_str())
                                        .map(String::from);

                                    // 物品等级
                                    let item_level = item
                                        .and_then(|i| i.get("ilvl"))
                                        .or_else(|| item.and_then(|i| i.get("itemLevel")))
                                        .and_then(|v| v.as_u64().map(|n| n as u32));

                                    // 在线状态 - 支持多种 API 响应格式
                                    let online = listing
                                        .and_then(|l| l.get("online"))
                                        .and_then(|v| v.as_bool())
                                        .or_else(|| {
                                            // account.online 可能是 bool 或对象
                                            listing
                                                .and_then(|l| l.get("account"))
                                                .and_then(|a| a.get("online"))
                                                .and_then(|v| {
                                                    v.as_bool()
                                                        .or_else(|| v.as_object().map(|_| true))
                                                })
                                        })
                                        .or_else(|| {
                                            listing
                                                .and_then(|l| l.get("account"))
                                                .and_then(|a| a.get("status"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s == "online")
                                        });

                                    // 上架时间
                                    let indexed_time = listing
                                        .and_then(|l| l.get("indexed"))
                                        .and_then(|v| v.as_str())
                                        .map(String::from);

                                    // 私聊文本
                                    let whisper_text = listing
                                        .and_then(|l| l.get("whisper"))
                                        .or_else(|| {
                                            listing.and_then(|l| l.get("whisper_tokenized"))
                                        })
                                        .and_then(|v| v.as_str())
                                        .map(String::from);

                                    entries.push(TradeEntry {
                                        price,
                                        seller,
                                        item_name,
                                        price_amount,
                                        price_currency,
                                        base_type,
                                        item_level,
                                        online,
                                        indexed_time,
                                        whisper_text,
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
            let value_tier = evaluate_item_value(&parsed, &entries, &settings.filter_rules);
            return Ok(TradeResult {
                item: parsed,
                league,
                total,
                entries,
                summary,
                url,
                options,
                page_size: settings.page_size,
                value_tier,
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
enum Action {
    Price,
    OpenHome,
    Quit,
}

pub(crate) struct UiState {
    pub(crate) hwnd: HWND,
    pub(crate) event_tx: Sender<OverlayEvent>,
    pub(crate) event_rx: Receiver<OverlayEvent>,
    pub(crate) action_rx: Receiver<Action>,
    pub(crate) view: OverlayView,
    pub(crate) fonts: Fonts,
    pub(crate) pinned: bool,
    pub(crate) hide_deadline: Option<Instant>,
    pub(crate) overlay_pos: Option<WindowPos>,
    pub(crate) page: usize,
    pub(crate) query_options: QueryOptions,
    pub(crate) last_clipboard_text: String,
    pub(crate) last_clipboard_check: Instant,
    pub(crate) last_settings_reload: Instant,
    pub(crate) settings: AppSettings,
    pub(crate) registered_manual_hotkey: Option<String>,
    pub(crate) tray_added: bool,
    pub(crate) app_icon: HICON,
    pub(crate) app_icon_owned: bool,
    pub(crate) auto_paused: bool,
    pub(crate) balloon_counter: u64,
    pub(crate) current_sort: SortOrder,
    #[allow(dead_code)]
    pub(crate) filters_dirty: bool,
    pub(crate) hovered_button: Option<UiButton>,
    pub(crate) track_mouse: bool,
    #[allow(dead_code)]
    pub(crate) last_query_id: u64,
    pub(crate) show_mode: OverlayShowMode,
    pub(crate) input_context: InputContext,
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
            current_sort: SortOrder::PriceAsc,
            filters_dirty: false,
            hovered_button: None,
            track_mouse: true,
            last_query_id: 0,
            show_mode: OverlayShowMode::Passive,
            input_context: InputContext::Game,
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
        self.show_mode = OverlayShowMode::Interactive;
        ShowWindow(self.hwnd, SW_SHOW);
        SetForegroundWindow(self.hwnd);
        self.hide_deadline = None;
        InvalidateRect(self.hwnd, null(), 0);
        self.show_mode = OverlayShowMode::Passive;
    }

    #[allow(dead_code)]
    unsafe fn activate_existing_window(&self) {
        if self.show_mode == OverlayShowMode::Interactive {
            SetForegroundWindow(self.hwnd);
        }
        SetWindowPos(
            self.hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
    }

    unsafe fn show_tray_menu(&mut self) {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let labels = [
            wide("打开面板"),
            wide("立即查价"),
            wide(if self.auto_paused {
                "恢复自动查价"
            } else {
                "暂停自动查价"
            }),
            wide("首次使用向导"),
            wide("设置"),
            wide(if self.settings.primary_league == "永久" {
                "切换联赛: 奥杜尔秘符"
            } else {
                "切换联赛: 永久"
            }),
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
                    query_state: None,
                    query_options: None,
                    query_created: None,
                };
                self.show_panel(580, 360, timeout);
            }
            OverlayEvent::QueryStarted {
                item,
                options,
                accent,
            } => {
                self.show_mode = OverlayShowMode::Passive;
                // 重置筛选状态，避免成功结果继续显示旧警告
                self.filters_dirty = false;
                // 创建一个仅包含物品信息的"空" TradeResult
                let result = TradeResult {
                    item: (*item).clone(),
                    league: String::new(),
                    total: 0,
                    entries: vec![],
                    summary: vec!["查询中...".to_string()],
                    url: String::new(),
                    options: options.clone(),
                    page_size: 10,
                    value_tier: ItemValueTier::Unknown,
                };
                let detail_lines = compute_item_detail_lines(&item);
                let modifier_count = item.mods.len();
                let entry_count = 0usize;
                let screen_h = GetSystemMetrics(SM_CYSCREEN);
                let max_height = screen_h * 90 / 100;
                let height = LayoutPlan::suggested_height(
                    detail_lines,
                    modifier_count,
                    entry_count,
                    480,
                    max_height,
                );
                self.page = 0;
                self.query_options = options.clone();
                self.current_sort = SortOrder::PriceAsc;
                self.view = OverlayView {
                    title: "流放2查价助手".to_string(),
                    subtitle: "国服查价".to_string(),
                    status: "正在查询...".to_string(),
                    current_url: String::new(),
                    accent,
                    kind: ViewKind::Result(Box::new(result)),
                    query_state: Some(QueryState::Loading),
                    query_options: Some(options),
                    query_created: Some(Instant::now()),
                };
                self.show_panel(580, height, Duration::from_secs(30));
            }
            OverlayEvent::Result {
                result,
                accent,
                timeout,
            } => {
                self.show_mode = OverlayShowMode::Passive;
                self.page = 0;
                self.query_options = result.options.clone();
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
                let options = result.options.clone();
                self.view = OverlayView {
                    title: "流放2查价助手".to_string(),
                    subtitle: "国服查价".to_string(),
                    status,
                    current_url: balloon_url,
                    accent,
                    kind: ViewKind::Result(result),
                    query_state: Some(QueryState::Success),
                    query_options: Some(options),
                    query_created: Some(Instant::now()),
                };
                let height = match &self.view.kind {
                    ViewKind::Result(result) => {
                        let detail_lines = compute_item_detail_lines(&result.item);
                        let modifier_count = result.item.mods.len();
                        let entry_count = result.entries.len();
                        let screen_h = GetSystemMetrics(SM_CYSCREEN);
                        let max_height = screen_h * 90 / 100;
                        LayoutPlan::suggested_height(
                            detail_lines,
                            modifier_count,
                            entry_count,
                            480,
                            max_height,
                        )
                    }
                    _ => 780,
                };
                self.show_panel(580, height, timeout);
                self.current_sort = SortOrder::PriceAsc;
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
        let flags = match self.show_mode {
            OverlayShowMode::Passive => SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOMOVE,
            OverlayShowMode::Interactive => SWP_SHOWWINDOW,
        };
        SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, width, height, flags);
        // 只在 Interactive 模式下才抢焦点
        if self.show_mode == OverlayShowMode::Interactive {
            SetForegroundWindow(self.hwnd);
        }
        self.hide_deadline = if self.pinned {
            None
        } else {
            Some(Instant::now() + timeout)
        };
        InvalidateRect(self.hwnd, null(), 0);
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
            self.close_panel();
        }
    }

    fn close_panel(&mut self) {
        self.input_context = InputContext::Game;
        self.hovered_button = None;
        self.hide_deadline = None;
        unsafe {
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
        InvalidateRect(self.hwnd, null(), 0);
    }

    unsafe fn open_settings(&mut self) {
        match launch_settings_setup() {
            Ok(_) => self.view.status = "已打开设置窗口".to_string(),
            Err(err) => self.view.status = format!("打开设置失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 0);
    }

    unsafe fn open_first_run_wizard(&mut self) {
        match launch_first_run_wizard() {
            Ok(_) => self.view.status = "已打开首次使用向导".to_string(),
            Err(err) => self.view.status = format!("打开向导失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 0);
    }

    unsafe fn open_history(&mut self) {
        match launch_history_viewer() {
            Ok(_) => self.view.status = "已打开查询历史".to_string(),
            Err(err) => self.view.status = format!("打开历史失败: {err}"),
        }
        InvalidateRect(self.hwnd, null(), 0);
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
        InvalidateRect(self.hwnd, null(), 0);
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
        InvalidateRect(self.hwnd, null(), 0);
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
        InvalidateRect(self.hwnd, null(), 0);
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
            query_state: None,
            query_options: None,
            query_created: None,
        };
        self.show_panel(580, 360, Duration::from_secs(18));
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

    /// 获取当前页的实际 page_size（动态可见行数）
    fn current_page_size(&self) -> usize {
        if let Some(result) = self.current_result() {
            let item_lines = compute_item_detail_lines(&result.item);
            let mod_count = result.item.mods.len();
            let entry_count = result.entries.len();
            let height = LayoutPlan::suggested_height(item_lines, mod_count, entry_count, 480, 900);
            let plan = LayoutPlan::compute(
                RECT {
                    left: 0,
                    top: 0,
                    right: 580,
                    bottom: height,
                },
                item_lines,
                mod_count,
            );
            visible_row_count(&plan.table_body).max(1)
        } else {
            8 // 默认值
        }
    }

    /// 计算总页数
    fn page_count(&self, total: usize, page_size: usize) -> usize {
        if total == 0 || page_size == 0 {
            return 1;
        }
        total.div_ceil(page_size)
    }

    /// 钳制页码到有效范围
    #[allow(dead_code)]
    fn clamp_page(&mut self, total: usize, page_size: usize) {
        let pc = self.page_count(total, page_size);
        if pc > 0 {
            self.page = self.page.min(pc - 1);
        } else {
            self.page = 0;
        }
    }
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
            let _ = UI_HWND.set(hwnd as isize);
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
        WM_MOUSELEAVE => {
            if let Some(state) = state_from_hwnd(hwnd) {
                state.input_context = InputContext::Game;
                state.hovered_button = None;
                state.track_mouse = true;
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_KEYDOWN => {
            if let Some(state) = state_from_hwnd(hwnd) {
                // 只有 Interactive 模式或 Overlay 上下文才处理按键
                if (state.show_mode == OverlayShowMode::Interactive
                    || state.input_context == InputContext::Overlay)
                    && state.handle_key_down(wparam as u32)
                {
                    return 0;
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            let x = (lparam as i16) as i32;
            let y = ((lparam >> 16) as i16) as i32;
            if let Some(state) = state_from_hwnd(hwnd) {
                // 首次进入 Overlay
                if state.input_context == InputContext::Game {
                    state.input_context = InputContext::Overlay;
                    state.touch_activity();
                }
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect);
                let new_hover = state
                    .button_specs(rect)
                    .iter()
                    .find(|spec| layout::rect_contains(&spec.rect, x, y))
                    .map(|spec| spec.button);
                // 只有 hover 变化时才重绘
                if new_hover != state.hovered_button {
                    state.hovered_button = new_hover;
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
                // 追踪鼠标离开
                if state.track_mouse {
                    state.track_mouse = false;
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as i16) as i32;
            if let Some(state) = state_from_hwnd(hwnd) {
                if state.input_context == InputContext::Overlay {
                    state.touch_activity();
                    if delta > 0 {
                        state.page_prev();
                    } else {
                        state.page_next();
                    }
                }
            }
            0
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
        WM_ERASEBKGND => {
            // 返回1表示我们已经处理了背景擦除
            // 避免系统再擦除一次造成闪烁
            1
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
            if let Some(state) = state_from_hwnd(hwnd) {
                state.close_panel();
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
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

    // 递增查询 ID 用于过期保护
    let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::Relaxed) + 1;

    // 不再发送"查价中"Message，改为发送 QueryStarted 立即显示物品详情
    let _ = event_tx.send(OverlayEvent::QueryStarted {
        item: Box::new(parsed.clone()),
        options: options.clone(),
        accent: rgb(56, 189, 248),
    });

    if let Some(hwnd) = UI_HWND.get() {
        unsafe {
            PostMessageW(*hwnd as HWND, WM_TIMER, TIMER_ID as WPARAM, 0);
        }
    }

    thread::spawn(
        move || match direct_trade_search(parsed.clone(), options.clone()) {
            Ok(result) => {
                // 查询过期保护：如果在此期间有新的查询，丢弃旧结果
                if query_id != NEXT_QUERY_ID.load(Ordering::Relaxed) {
                    log(format!("查询 {} 已过期，丢弃结果", query_id));
                    return;
                }
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
                if query_id != NEXT_QUERY_ID.load(Ordering::Relaxed) {
                    log(format!("查询 {} 已过期，丢弃认证错误", query_id));
                    return;
                }
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
                if query_id != NEXT_QUERY_ID.load(Ordering::Relaxed) {
                    log(format!("查询 {} 已过期，丢弃请求错误", query_id));
                    return;
                }
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
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
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
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
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
        // 后台预加载属性库，减少首次按属性查价的延迟
        thread::spawn(|| {
            if let Err(err) = load_stat_defs() {
                log(format!("预加载属性库失败: {err}"));
            }
        });
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
impl UiState {
    pub(crate) fn new_for_test() -> Self {
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (_, action_rx) = std::sync::mpsc::channel();
        Self {
            hwnd: std::ptr::null_mut(),
            event_tx,
            event_rx,
            action_rx,
            view: OverlayView::default(),
            fonts: unsafe { std::mem::zeroed() },
            pinned: false,
            hide_deadline: None,
            overlay_pos: None,
            page: 0,
            query_options: QueryOptions::default(),
            last_clipboard_text: String::new(),
            last_clipboard_check: std::time::Instant::now(),
            last_settings_reload: std::time::Instant::now(),
            settings: AppSettings::default(),
            registered_manual_hotkey: None,
            tray_added: false,
            app_icon: std::ptr::null_mut(),
            app_icon_owned: false,
            auto_paused: false,
            balloon_counter: 0,
            current_sort: SortOrder::PriceAsc,
            filters_dirty: false,
            hovered_button: None,
            track_mouse: true,
            last_query_id: 0,
            show_mode: OverlayShowMode::Passive,
            input_context: InputContext::Game,
        }
    }
}

impl TradeResult {
    #[allow(dead_code)]
    pub(crate) fn new_for_test(item: ParsedItem) -> Self {
        Self {
            item,
            league: String::new(),
            total: 0,
            entries: Vec::new(),
            summary: Vec::new(),
            url: String::new(),
            options: QueryOptions::default(),
            page_size: 10,
            value_tier: ItemValueTier::Unknown,
        }
    }
}

#[cfg(test)]
pub(crate) fn make_test_item_with_mods(mod_count: usize) -> ParsedItem {
    let mut item = ParsedItem::default();
    for i in 0..mod_count {
        item.mods.push(ParsedMod {
            text: format!("词缀 {}", i + 1),
            pattern: format!("mod_{}", i + 1),
            value: Some((i + 1) as f64),
            stat_id: None,
            stat_text: None,
        });
    }
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::layout::visible_row_count;

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
            quality: None,
            required_level: None,
            physical_damage_min: None,
            physical_damage_max: None,
            fire_damage_min: None,
            fire_damage_max: None,
            cold_damage_min: None,
            cold_damage_max: None,
            lightning_damage_min: None,
            lightning_damage_max: None,
            chaos_damage_min: None,
            chaos_damage_max: None,
            critical_strike_chance: None,
            attacks_per_second: None,
            armour: None,
            evasion: None,
            energy_shield: None,
            sockets: None,
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

    #[test]
    fn trade_entry_sort_by_price_numerically() {
        let mut entries = vec![
            TradeEntry {
                price: "5 chaos".into(),
                price_amount: Some(5.0),
                ..Default::default()
            },
            TradeEntry {
                price: "1 divine".into(),
                price_amount: Some(1.0),
                ..Default::default()
            },
            TradeEntry {
                price: "10 chaos".into(),
                price_amount: Some(10.0),
                ..Default::default()
            },
        ];
        sort_entries(&mut entries, SortOrder::PriceAsc);
        assert_eq!(entries[0].price_amount, Some(1.0));
        assert_eq!(entries[1].price_amount, Some(5.0));
        assert_eq!(entries[2].price_amount, Some(10.0));
    }

    #[test]
    fn trade_entry_sort_online_first() {
        let mut entries = vec![
            TradeEntry {
                price: "5 chaos".into(),
                price_amount: Some(5.0),
                online: Some(false),
                ..Default::default()
            },
            TradeEntry {
                price: "10 chaos".into(),
                price_amount: Some(10.0),
                online: Some(true),
                ..Default::default()
            },
        ];
        sort_entries(&mut entries, SortOrder::OnlineFirst);
        assert_eq!(entries[0].online, Some(true));
        assert_eq!(entries[1].online, Some(false));
    }

    #[test]
    fn trade_entry_default_has_all_none() {
        let entry = TradeEntry::default();
        assert!(entry.price_amount.is_none());
        assert!(entry.price_currency.is_none());
        assert!(entry.base_type.is_none());
        assert!(entry.item_level.is_none());
        assert!(entry.online.is_none());
        assert!(entry.indexed_time.is_none());
        assert!(entry.whisper_text.is_none());
    }

    #[test]
    fn relative_time_parses_iso_format() {
        let result = relative_time("2024-01-15T10:30:00Z");
        assert_eq!(result, "2024-01-15 10:30:00");
    }

    #[test]
    fn parse_price_to_chaos_handles_missing_fields() {
        // 测试 price_amount 为 None 时的排序行为
        let mut entries = vec![
            TradeEntry {
                price: "no price".into(),
                price_amount: None,
                ..Default::default()
            },
            TradeEntry {
                price: "5 chaos".into(),
                price_amount: Some(5.0),
                ..Default::default()
            },
        ];
        sort_entries(&mut entries, SortOrder::PriceAsc);
        assert_eq!(entries[0].price_amount, Some(5.0)); // 有价格的排在前面
    }

    #[test]
    fn parses_weapon_physical_damage() {
        let text = "Rarity: Rare\n物品类别: 单手剑\n物理伤害: 30-60\n每秒攻击次数: 1.5\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.physical_damage_min, Some(30.0));
        assert_eq!(item.physical_damage_max, Some(60.0));
        assert_eq!(item.attacks_per_second, Some(1.5));
    }

    #[test]
    fn calculates_physical_dps_correctly() {
        let text = "Rarity: Rare\n物品类别: 单手剑\n物理伤害: 30-60\n每秒攻击次数: 1.5\n--------";
        let item = parse_item_text(text);
        let dps = item.physical_dps().unwrap();
        assert!((dps - 67.5).abs() < 0.01); // (30+60)/2 * 1.5 = 67.5
    }

    #[test]
    fn calculates_total_dps_with_elements() {
        let text = "Rarity: Rare\n物品类别: 弓\n物理伤害: 20-40\n火焰伤害: 10-20\n每秒攻击次数: 1.4\n--------";
        let item = parse_item_text(text);
        let total = item.total_dps().unwrap();
        // physical: (20+40)/2*1.4 = 42, fire: (10+20)/2*1.4 = 21, total = 63
        assert!((total - 63.0).abs() < 0.01);
    }

    #[test]
    fn parses_armour_and_evasion() {
        let text = "Rarity: Rare\n物品类别: 胸甲\n护甲: 200\n闪避值: 150\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.armour, Some(200));
        assert_eq!(item.evasion, Some(150));
    }

    #[test]
    fn parses_quality_and_required_level() {
        let text = "Rarity: Rare\n物品类别: 单手剑\n品质: +20%\n需求: 等级 60\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.quality, Some(20));
        assert_eq!(item.required_level, Some(60));
    }

    #[test]
    fn non_weapon_has_no_dps() {
        let text = "Rarity: Rare\n物品类别: 戒指\n--------";
        let item = parse_item_text(text);
        assert!(item.total_dps().is_none());
        assert!(!item.is_weapon());
    }

    #[test]
    fn missing_attack_speed_returns_none_dps() {
        let text = "Rarity: Rare\n物品类别: 单手剑\n物理伤害: 30-60\n--------";
        let item = parse_item_text(text);
        assert!(item.physical_dps().is_none());
    }

    #[test]
    fn parses_required_level_with_multiple_numbers() {
        let text = "需求: 等级 11, 23 智慧\n--------";
        let item = parse_item_text(text);
        assert_eq!(
            item.required_level,
            Some(11),
            "should parse level 11, not 1123"
        );
    }

    #[test]
    fn required_level_not_concatenated() {
        // 确保不会回到拼接数字的行为
        let text = "需求: 等级 11, 23 智慧\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.required_level, Some(11));
        assert_ne!(item.required_level, Some(1123));
        assert_ne!(item.required_level, Some(23));
    }

    #[test]
    fn parses_required_level_english() {
        let text = "Requires Level 11, 23 Int\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.required_level, Some(11));
    }

    #[test]
    fn parses_required_level_simple() {
        let text = "需求等级: 60\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.required_level, Some(60));
    }

    #[test]
    fn required_level_none_when_not_present() {
        let text = "Rarity: Normal\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.required_level, None);
    }

    #[test]
    fn quality_parses_first_number_only() {
        let text = "品质: +20%\n--------";
        let item = parse_item_text(text);
        assert_eq!(item.quality, Some(20));
    }

    #[test]
    fn visible_indices_preserve_original_after_sort() {
        let entries = vec![
            TradeEntry {
                seller: "A".into(),
                price: "3 chaos".into(),
                price_amount: Some(3.0),
                ..Default::default()
            },
            TradeEntry {
                seller: "B".into(),
                price: "1 chaos".into(),
                price_amount: Some(1.0),
                ..Default::default()
            },
            TradeEntry {
                seller: "C".into(),
                price: "2 chaos".into(),
                price_amount: Some(2.0),
                ..Default::default()
            },
        ];
        let visible = visible_listing_indices(&entries, SortOrder::PriceAsc, 0, 10);
        // 排序后 B(1.0), C(2.0), A(3.0)
        assert_eq!(visible[0].0, 1); // B 原始索引 1
        assert_eq!(visible[0].1.seller, "B");
        assert_eq!(visible[1].0, 2); // C 原始索引 2
        assert_eq!(visible[2].0, 0); // A 原始索引 0
    }

    #[test]
    fn visible_indices_respect_pagination() {
        let entries: Vec<TradeEntry> = (0..20)
            .map(|i| TradeEntry {
                seller: format!("Seller{i}"),
                price: format!("{i} chaos"),
                price_amount: Some(i as f64),
                ..Default::default()
            })
            .collect();
        let page0 = visible_listing_indices(&entries, SortOrder::PriceAsc, 0, 5);
        let page1 = visible_listing_indices(&entries, SortOrder::PriceAsc, 1, 5);
        assert_eq!(page0.len(), 5);
        assert_eq!(page1.len(), 5);
        // 确保没有重复索引
        let all_indices: Vec<usize> = page0
            .iter()
            .map(|(i, _)| *i)
            .chain(page1.iter().map(|(i, _)| *i))
            .collect();
        let mut unique: Vec<usize> = all_indices.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            all_indices.len(),
            unique.len(),
            "indices should be unique across pages"
        );
    }

    /// 辅助函数：将 Unix 时间戳转换为 ISO 8601 格式字符串
    fn format_utc_timestamp(unix_secs: u64) -> String {
        let secs = unix_secs % 60;
        let mins = (unix_secs / 60) % 60;
        let hours = (unix_secs / 3600) % 24;
        let total_days = (unix_secs / 86400) as i64;
        let mut y = 1970i64;
        let mut remaining_days = total_days;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            y += 1;
        }
        let month_days = if is_leap(y) {
            [31i64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31i64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut m = 0usize;
        let mut d = remaining_days;
        for (i, md) in month_days.iter().enumerate() {
            if d < *md {
                m = i + 1;
                break;
            }
            d -= *md;
        }
        format!("{y:04}-{m:02}-{:02}T{hours:02}:{mins:02}:{secs:02}Z", d + 1)
    }

    #[test]
    fn relative_time_ago_formats_correctly() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let recent = format_utc_timestamp(now - 120); // 2分钟前
        assert_eq!(relative_time_ago(&recent), "2分钟");

        let hours_ago = format_utc_timestamp(now - 7200); // 2小时前
        assert_eq!(relative_time_ago(&hours_ago), "2小时");
    }

    #[test]
    fn sort_order_toggles_direction() {
        // 测试排序方向切换
        let mut sort = SortOrder::PriceAsc;
        sort = match sort {
            SortOrder::PriceAsc => SortOrder::PriceDesc,
            _ => SortOrder::PriceAsc,
        };
        assert_eq!(sort, SortOrder::PriceDesc);
    }

    #[test]
    fn online_status_handles_object_format() {
        // 模拟 account.online 为对象的情况
        let entry = TradeEntry {
            online: Some(true),
            ..Default::default()
        };
        assert_eq!(entry.online, Some(true));
    }

    fn should_repaint_on_hover(old: Option<UiButton>, new: Option<UiButton>) -> bool {
        old != new
    }

    #[test]
    fn hover_transition_does_not_trigger_redundant_repaint() {
        // 模拟 hover 状态转换：同一按钮→不触发重绘
        let same = UiButton::ModToggle(0);
        assert!(!should_repaint_on_hover(Some(same), Some(same)));
        // 不同按钮→触发重绘
        assert!(should_repaint_on_hover(
            Some(UiButton::ModToggle(0)),
            Some(UiButton::ModToggle(1))
        ));
        // 进入→触发
        assert!(should_repaint_on_hover(None, Some(UiButton::Close)));
        // 离开→触发
        assert!(should_repaint_on_hover(Some(UiButton::Close), None));
    }

    #[test]
    fn hover_changes_trigger_repaint_only_when_different() {
        // 同一按钮 hover 不触发重绘
        assert!(!should_repaint_on_hover(
            Some(UiButton::Close),
            Some(UiButton::Close)
        ));
        // 不同按钮 hover 触发重绘
        assert!(should_repaint_on_hover(
            Some(UiButton::Close),
            Some(UiButton::Pin)
        ));
        // None → Some 触发重绘
        assert!(should_repaint_on_hover(None, Some(UiButton::Close)));
        // Some → None 触发重绘
        assert!(should_repaint_on_hover(Some(UiButton::Close), None));
    }

    #[test]
    fn query_state_transitions() {
        // 测试状态转换正确性
        assert!(matches!(QueryState::Loading, QueryState::Loading));
        assert!(matches!(QueryState::Success, QueryState::Success));
        assert!(matches!(
            QueryState::Error("err".into()),
            QueryState::Error(_)
        ));
        assert!(matches!(QueryState::Empty, QueryState::Empty));
    }

    #[test]
    fn query_state_not_equal() {
        assert_ne!(QueryState::Loading, QueryState::Success);
        assert_ne!(QueryState::Loading, QueryState::Empty);
        assert_ne!(QueryState::Success, QueryState::Error("err".into()));
    }

    #[test]
    fn whisper_count_per_listing() {
        // 验证每个可见挂单只有一个 whisper 按钮
        let rect = RECT {
            left: 0,
            top: 0,
            right: 580,
            bottom: 780,
        };
        let plan = LayoutPlan::compute(rect, 6, 6);
        let mut result = TradeResult::new_for_test(make_test_item_with_mods(6));
        let visible_rows = visible_row_count(&plan.table_body);
        // 添加足够多的挂单以占满可见行
        for i in 0..visible_rows + 2 {
            result.entries.push(TradeEntry {
                seller: format!("Seller{}", i),
                price: format!("{} chaos", i + 1),
                price_amount: Some((i + 1) as f64),
                ..Default::default()
            });
        }
        let state = UiState::new_for_test();
        let specs = state.button_specs_for_result(&plan, &result);
        let whisper_count = specs
            .iter()
            .filter(|s| matches!(s.button, UiButton::Whisper(_)) && s.visible)
            .count();
        assert_eq!(whisper_count, visible_rows);
    }

    #[test]
    fn query_id_increments_to_prevent_stale_results() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let counter = AtomicU64::new(0);
        let id1 = counter.fetch_add(1, Ordering::SeqCst);
        let id2 = counter.fetch_add(1, Ordering::SeqCst);
        assert_ne!(id1, id2);
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
    }

    // ── 回归测试：page_size 不一致导致切片越界 ──

    #[test]
    fn visible_listing_indices_empty_when_page_out_of_range() {
        // len=43, page_size=20, page=3 → start=60 > 43 → 应返回空
        let entries: Vec<TradeEntry> = (0..43)
            .map(|i| TradeEntry {
                seller: format!("S{}", i),
                price: format!("{} chaos", i),
                price_amount: Some(i as f64),
                ..Default::default()
            })
            .collect();
        let result = visible_listing_indices(&entries, SortOrder::PriceAsc, 3, 20);
        assert!(
            result.is_empty(),
            "should return empty for out-of-range page"
        );
    }

    #[test]
    fn visible_listing_indices_handles_zero_page_size() {
        let entries: Vec<TradeEntry> = (0..10)
            .map(|i| TradeEntry {
                seller: format!("S{}", i),
                price: format!("{} chaos", i),
                price_amount: Some(i as f64),
                ..Default::default()
            })
            .collect();
        let result = visible_listing_indices(&entries, SortOrder::PriceAsc, 0, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn visible_listing_indices_last_partial_page() {
        // 最后一页只有 3 条
        let entries: Vec<TradeEntry> = (0..43)
            .map(|i| TradeEntry {
                seller: format!("S{}", i),
                price: format!("{} chaos", i),
                price_amount: Some(i as f64),
                ..Default::default()
            })
            .collect();
        let result = visible_listing_indices(&entries, SortOrder::PriceAsc, 2, 20);
        assert_eq!(result.len(), 3); // 43 - 40 = 3
    }

    #[test]
    fn page_count_calculates_correctly() {
        assert_eq!(page_count(43, 20), 3);
        assert_eq!(page_count(60, 20), 3);
        assert_eq!(page_count(0, 20), 1);
        assert_eq!(page_count(20, 20), 1);
        assert_eq!(page_count(21, 20), 2);
    }

    #[test]
    fn visible_listing_indices_page_60_entries_page5_size20() {
        // len=60, page=5, page_size=20 → start=100 > 60 → 应返回空（崩溃场景二）
        let entries: Vec<TradeEntry> = (0..60)
            .map(|i| TradeEntry {
                seller: format!("S{}", i),
                price: format!("{} chaos", i),
                price_amount: Some(i as f64),
                ..Default::default()
            })
            .collect();
        let result = visible_listing_indices(&entries, SortOrder::PriceAsc, 5, 20);
        assert!(
            result.is_empty(),
            "should return empty for out-of-range page (len=60,page=5,page_size=20)"
        );
    }

    fn page_count(total: usize, page_size: usize) -> usize {
        if total == 0 || page_size == 0 {
            1
        } else {
            total.div_ceil(page_size)
        }
    }

    #[test]
    fn input_context_transitions() {
        // Game → Overlay
        assert_ne!(InputContext::Game, InputContext::Overlay);
        // 状态对比
        assert!(matches!(InputContext::Game, InputContext::Game));
        assert!(matches!(InputContext::Overlay, InputContext::Overlay));
    }

    #[test]
    fn overlay_context_allows_interaction() {
        let ctx = InputContext::Overlay;
        assert!(ctx == InputContext::Overlay);
    }

    #[test]
    fn game_context_blocks_overlay_keys() {
        let ctx = InputContext::Game;
        assert!(ctx != InputContext::Overlay);
    }
}
