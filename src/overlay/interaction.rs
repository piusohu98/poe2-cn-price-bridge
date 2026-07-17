use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, SW_HIDE, ShowWindow};

use crate::overlay::layout::{self, OverlayLayout};
use crate::overlay::model::{UiButton, ViewKind};
use crate::{
    ParsedMod, SortOrder, TradeResult, UiState, copy_text_to_clipboard,
    start_price_query_from_parsed,
};

/// Extension trait for UiState's interaction methods.
pub trait OverlayInteraction {
    fn current_result(&self) -> Option<&TradeResult>;
    #[allow(dead_code)]
    fn is_mod_selected(&self, item_mod: &ParsedMod) -> bool;
    #[allow(dead_code)]
    fn mod_chip_label(index: usize, item_mod: &ParsedMod) -> String
    where
        Self: Sized;

    unsafe fn handle_button_click(&mut self, x: i32, y: i32) -> bool;
    unsafe fn handle_key_down(&mut self, vk_code: u32) -> bool;
    unsafe fn touch_activity(&mut self);
    unsafe fn toggle_pin(&mut self);
    unsafe fn copy_url(&mut self);
    unsafe fn copy_whisper_for_row(&mut self, row_index: usize);
    unsafe fn toggle_mod_filters(&mut self);
    unsafe fn toggle_value_filters(&mut self);
    unsafe fn toggle_single_mod(&mut self, index: usize);
    unsafe fn page_prev(&mut self);
    unsafe fn page_next(&mut self);
    unsafe fn rerun_current_query(&mut self);
    unsafe fn handle_sort_click(&mut self, primary: SortOrder, secondary: SortOrder);
}

impl OverlayInteraction for UiState {
    fn current_result(&self) -> Option<&TradeResult> {
        match &self.view.kind {
            ViewKind::Result(result) => Some(result.as_ref()),
            ViewKind::Message(_) => None,
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

    unsafe fn handle_button_click(&mut self, x: i32, y: i32) -> bool {
        self.touch_activity();
        let mut rect = RECT::default();
        GetClientRect(self.hwnd, &mut rect);
        for spec in self.button_specs(rect) {
            if !layout::rect_contains(&spec.rect, x, y) {
                continue;
            }
            if !spec.enabled {
                self.view.status = "这个操作当前不可用".to_string();
                InvalidateRect(self.hwnd, std::ptr::null(), 1);
                return true;
            }
            match spec.button {
                UiButton::Pin => self.toggle_pin(),
                UiButton::Close => {
                    ShowWindow(self.hwnd, SW_HIDE);
                }
                UiButton::Wizard => self.open_first_run_wizard(),
                UiButton::Cookie => self.open_cookie_setup(),
                UiButton::ValidateCookie => crate::start_cookie_validation(self.event_tx.clone()),
                UiButton::Diagnostics => self.export_diagnostics(),
                UiButton::History => self.open_history(),
                UiButton::Mods => self.toggle_mod_filters(),
                UiButton::Values => self.toggle_value_filters(),
                UiButton::ModToggle(index) => self.toggle_single_mod(index),
                UiButton::RerunSearch => self.rerun_current_query(),
                UiButton::Prev => self.page_prev(),
                UiButton::Next => self.page_next(),
                UiButton::Open => self.open_current_url(),
                UiButton::Copy => self.copy_url(),
                UiButton::Whisper(index) => self.copy_whisper_for_row(index),
                UiButton::SortPrice => {
                    self.handle_sort_click(SortOrder::PriceAsc, SortOrder::PriceDesc)
                }
                UiButton::SortTime => {
                    self.handle_sort_click(SortOrder::IndexedTimeAsc, SortOrder::IndexedTimeAsc)
                }
                UiButton::SortLevel => {
                    self.handle_sort_click(SortOrder::ItemLevelDesc, SortOrder::ItemLevelDesc)
                }
            }
            return true;
        }
        false
    }

    /// 用户交互时刷新自动隐藏倒计时
    unsafe fn touch_activity(&mut self) {
        if !self.pinned {
            self.hide_deadline = Some(Instant::now() + Duration::from_secs(15));
        }
    }

    /// 键盘快捷键处理，返回 true 表示已处理
    unsafe fn handle_key_down(&mut self, vk_code: u32) -> bool {
        self.touch_activity();
        let has_result = self.current_result().is_some();
        match vk_code {
            0x25 => {
                if has_result {
                    self.page_prev();
                } else {
                    return false;
                }
            } // ← 上一页
            0x27 => {
                if has_result {
                    self.page_next();
                } else {
                    return false;
                }
            } // → 下一页
            0x4D => {
                if has_result {
                    self.toggle_mod_filters();
                } else {
                    return false;
                }
            } // M 同属性
            0x56 => {
                if has_result {
                    self.toggle_value_filters();
                } else {
                    return false;
                }
            } // V 数值
            0x50 => {
                self.toggle_pin();
            } // P 固定
            0x43 => {
                if has_result {
                    self.copy_url();
                } else {
                    return false;
                }
            } // C 复制链接
            0x4F => {
                self.open_current_url();
            } // O 打开市集
            0x31..=0x34 => {
                // 1-4 切换属性chip
                if has_result {
                    let index = (vk_code - 0x31) as usize;
                    self.toggle_single_mod(index);
                } else {
                    return false;
                }
            }
            0x1B => {
                ShowWindow(self.hwnd, SW_HIDE);
            } // Esc 关闭
            _ => return false,
        }
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
        true
    }

    unsafe fn toggle_pin(&mut self) {
        self.pinned = !self.pinned;
        self.hide_deadline = None;
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
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
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
    }

    /// 复制私聊消息到剪贴板，index 为原始 entries 中的行索引
    unsafe fn copy_whisper_for_row(&mut self, row_index: usize) {
        let Some(result) = self.current_result() else {
            self.view.status = "没有可复制的查询结果".to_string();
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
            return;
        };
        let Some(entry) = result.entries.get(row_index) else {
            self.view.status = format!("行索引 {row_index} 无效");
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
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
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
    }

    unsafe fn rerun_current_query(&mut self) {
        let Some(result) = self.current_result() else {
            return;
        };
        let parsed = result.item.clone();
        let options = self.query_options.clone();
        self.filters_dirty = false;
        self.view.status = "正在按新筛选重新查询...".to_string();
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
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
                InvalidateRect(self.hwnd, std::ptr::null(), 1);
                return;
            }
            selected.remove(position);
        } else {
            selected.push(pattern);
        }

        self.query_options.use_mods = true;
        self.query_options.selected_mod_patterns = Some(selected);
        self.filters_dirty = true;
        self.view.status = "筛选条件已更改，请点击重新搜索".to_string();
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
    }

    unsafe fn page_prev(&mut self) {
        if self.page > 0 {
            self.page -= 1;
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
        }
    }

    unsafe fn page_next(&mut self) {
        let Some(result) = self.current_result() else {
            return;
        };
        if (self.page + 1) * result.page_size < result.entries.len() {
            self.page += 1;
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
        }
    }

    unsafe fn handle_sort_click(&mut self, primary: SortOrder, secondary: SortOrder) {
        if self.current_sort == primary {
            self.current_sort = secondary;
        } else {
            self.current_sort = primary;
        }
        self.page = 0;
        InvalidateRect(self.hwnd, std::ptr::null(), 1);
    }
}
