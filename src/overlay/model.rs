use serde::{Deserialize, Serialize};
use std::time::Duration;

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DeleteObject, FF_DONTCARE, FW_BOLD, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
};

use crate::{TradeResult, rgb, wide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiButton {
    Pin,
    Close,
    Wizard,
    Cookie,
    ValidateCookie,
    Diagnostics,
    History,
    Mods,
    Values,
    #[allow(dead_code)]
    ModToggle(usize),
    #[allow(dead_code)]
    RerunSearch,
    Prev,
    Next,
    Open,
    Copy,
    SortPrice,
    SortTime,
    SortLevel,
    Whisper(usize),
    Backdrop,
    OpenTrade,
}

pub struct UiButtonSpec {
    pub button: UiButton,
    pub label: String,
    pub rect: RECT,
    pub enabled: bool,
    pub primary: bool,
    /// true = 需要绘制为可见按钮, false = 纯点击热区（不可见）
    pub visible: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct WindowPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub enum OverlayEvent {
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

#[derive(Clone)]
pub enum ViewKind {
    Message(Vec<String>),
    Result(Box<TradeResult>),
}

#[derive(Clone)]
pub struct OverlayView {
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub current_url: String,
    pub accent: u32,
    pub kind: ViewKind,
}

impl Default for OverlayView {
    fn default() -> Self {
        Self {
            title: crate::APP_DISPLAY_NAME.to_string(),
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

pub struct Fonts {
    pub title: HFONT,
    pub normal: HFONT,
    pub small: HFONT,
    pub bold: HFONT,
}

impl Fonts {
    pub unsafe fn new() -> Self {
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

pub unsafe fn make_font(size: i32, weight: i32) -> HFONT {
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
