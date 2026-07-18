use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DeleteObject, FF_DONTCARE, FW_BOLD, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
};

use crate::{ParsedItem, QueryOptions, TradeResult, rgb, wide};

/// Overlay 显示模式：控制窗口是否激活/抢焦点
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayShowMode {
    /// 被动模式：不抢焦点，不激活窗口（自动查价、查询结果）
    Passive,
    /// 交互模式：允许激活窗口（托盘打开面板时）
    Interactive,
}

/// 输入上下文：统一管理鼠标是否位于插件窗口内
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContext {
    /// 鼠标在游戏内，插件只接收剪贴板事件
    Game,
    /// 鼠标在 Overlay 窗口内，插件处理鼠标点击、滚轮、局部快捷键
    Overlay,
}

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

/// 查价状态
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum QueryState {
    /// 正在查询
    Loading,
    /// 查询成功
    Success,
    /// 无结果
    Empty,
    /// 查询失败，附带错误信息
    Error(String),
    /// 正在重试
    Retrying,
}

#[derive(Debug)]
pub enum OverlayEvent {
    Message {
        title: String,
        lines: Vec<String>,
        accent: u32,
        timeout: Duration,
    },
    /// 查询开始，立即显示物品详情窗口
    QueryStarted {
        item: Box<ParsedItem>,
        options: QueryOptions,
        accent: u32,
    },
    Result {
        result: Box<TradeResult>,
        accent: u32,
        timeout: Duration,
    },
}

#[derive(Debug, Clone)]
pub enum ViewKind {
    Message(Vec<String>),
    Result(Box<TradeResult>),
}

#[derive(Debug, Clone)]
pub struct OverlayView {
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub current_url: String,
    pub accent: u32,
    pub kind: ViewKind,
    /// 查价状态，仅在 ViewKind::Result 时有效
    pub query_state: Option<QueryState>,
    /// 查询选项，仅在 ViewKind::Result 时有效
    #[allow(dead_code)]
    pub query_options: Option<QueryOptions>,
    /// 查询创建时间，用于过期检测
    #[allow(dead_code)]
    pub query_created: Option<Instant>,
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
            query_state: None,
            query_options: None,
            query_created: None,
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
