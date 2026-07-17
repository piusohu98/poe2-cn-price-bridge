use std::cmp::{max, min};
use std::mem::MaybeUninit;

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, DeleteDC,
    DeleteObject, DrawTextW, EndPaint, FillRect, HDC, HFONT, HGDIOBJ, PAINTSTRUCT, PS_SOLID,
    RoundRect, SRCCOPY, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

use crate::overlay::layout::{
    LayoutPlan, OverlayLayout, compute_column_layout, compute_item_detail_lines, visible_row_count,
};
use crate::overlay::model::{QueryState, UiButton, ViewKind};
use crate::overlay::theme;
use crate::{
    ItemValueTier, SortOrder, TradeResult, UiState, relative_time_ago, rgb,
    visible_listing_indices, wide,
};

pub unsafe fn fill(hdc: HDC, rect: RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, &rect, brush);
    DeleteObject(brush as _);
}

pub unsafe fn rounded_rect(hdc: HDC, rect: RECT, fill_color: u32, border_color: u32, radius: i32) {
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

pub unsafe fn draw_text(hdc: HDC, text: &str, mut rect: RECT, color: u32, font: HFONT, flags: u32) {
    let text = wide(text);
    let old_font = SelectObject(hdc, font as _);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    DrawTextW(hdc, text.as_ptr(), -1, &mut rect, flags);
    SelectObject(hdc, old_font);
}

/// Brighten a color by adding `amount` to each channel (capped at 255).
fn brighten(color: u32, amount: u32) -> u32 {
    let r = ((color & 0xFF) + amount).min(255);
    let g = (((color >> 8) & 0xFF) + amount).min(255);
    let b = (((color >> 16) & 0xFF) + amount).min(255);
    r | (g << 8) | (b << 16)
}

/// Extension trait for UiState's paint / rendering methods.
pub trait OverlayRenderer {
    unsafe fn paint(&self);
    unsafe fn paint_value_tier_badge(&self, hdc: HDC, rect: RECT, tier: ItemValueTier);
    unsafe fn paint_message(&self, hdc: HDC, rect: RECT, lines: &[String]);
    unsafe fn paint_buttons(&self, hdc: HDC, specs: &[crate::overlay::model::UiButtonSpec]);
    unsafe fn paint_result(&self, hdc: HDC, plan: &LayoutPlan, result: &TradeResult);
    #[allow(dead_code)]
    unsafe fn paint_item_details(&self, hdc: HDC, rect: RECT, result: &TradeResult) -> i32;
    #[allow(dead_code)]
    unsafe fn paint_modifier_list(
        &self,
        hdc: HDC,
        rect: RECT,
        result: &TradeResult,
        start_y: i32,
    ) -> i32;
}

impl OverlayRenderer for UiState {
    unsafe fn paint(&self) {
        let mut ps = MaybeUninit::<PAINTSTRUCT>::zeroed().assume_init();
        let screen_hdc = BeginPaint(self.hwnd, &mut ps);
        let mut rect = RECT::default();
        GetClientRect(self.hwnd, &mut rect);

        if rect.right <= 0 || rect.bottom <= 0 {
            EndPaint(self.hwnd, &ps);
            return;
        }

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        // 创建内存 DC
        let mem_hdc = CreateCompatibleDC(screen_hdc);
        if mem_hdc.is_null() {
            EndPaint(self.hwnd, &ps);
            return;
        }

        // 创建内存位图
        let mem_bmp = CreateCompatibleBitmap(screen_hdc, w, h);
        if mem_bmp.is_null() {
            DeleteDC(mem_hdc);
            EndPaint(self.hwnd, &ps);
            return;
        }

        let old_bmp = SelectObject(mem_hdc, mem_bmp as HGDIOBJ);

        // 所有绘制到 mem_hdc
        self.paint_to_dc(mem_hdc, rect);

        // 一次性 BitBlt 到屏幕
        BitBlt(screen_hdc, 0, 0, w, h, mem_hdc, 0, 0, SRCCOPY);

        // 恢复和释放
        SelectObject(mem_hdc, old_bmp);
        DeleteObject(mem_bmp as HGDIOBJ);
        DeleteDC(mem_hdc);
        EndPaint(self.hwnd, &ps);
    }

    unsafe fn paint_value_tier_badge(&self, hdc: HDC, rect: RECT, tier: ItemValueTier) {
        let label = match tier {
            ItemValueTier::Legendary => "神装",
            ItemValueTier::High => "高价值",
            ItemValueTier::Medium => "普通",
            ItemValueTier::Normal => "一般",
            ItemValueTier::Junk => "低价值",
            ItemValueTier::Unknown => return, // 未知不显示
        };
        let color = tier.color();
        let badge_w = 72;
        let badge_h = 22;
        // 右边缘在 rect.right - 160
        let badge_rect = RECT {
            left: rect.right - 160 - badge_w,
            top: 10,
            right: rect.right - 160,
            bottom: 10 + badge_h,
        };
        rounded_rect(hdc, badge_rect, theme::BG_BADGE, color, 6);
        draw_text(
            hdc,
            label,
            RECT {
                left: badge_rect.left + 4,
                top: badge_rect.top,
                right: badge_rect.right - 4,
                bottom: badge_rect.bottom,
            },
            color,
            self.fonts.small,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }

    unsafe fn paint_message(&self, hdc: HDC, rect: RECT, lines: &[String]) {
        let mut y = 60;
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
                theme::TEXT_BRIGHT,
                self.fonts.normal,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 30;
        }
    }

    unsafe fn paint_buttons(&self, hdc: HDC, specs: &[crate::overlay::model::UiButtonSpec]) {
        for spec in specs {
            if !spec.visible {
                continue;
            }
            let is_hovered = self.hovered_button == Some(spec.button);
            let (fill_color, border_color, text_color) = if !spec.enabled {
                (
                    theme::BTN_BG_DISABLED,
                    theme::BTN_BORDER_DISABLED,
                    theme::BTN_TEXT_DISABLED,
                )
            } else if spec.primary {
                let (fill, border, text) = (
                    theme::BTN_BG_PRIMARY,
                    theme::BTN_BORDER_PRIMARY,
                    theme::BTN_TEXT_PRIMARY,
                );
                if is_hovered {
                    (brighten(fill, 20), brighten(border, 20), text)
                } else {
                    (fill, border, text)
                }
            } else if spec.button == UiButton::Close {
                let (fill, border, text) = (
                    theme::BTN_BG_DANGER,
                    theme::BTN_BORDER_DANGER,
                    theme::BTN_TEXT_DANGER,
                );
                if is_hovered {
                    (brighten(fill, 20), brighten(border, 20), text)
                } else {
                    (fill, border, text)
                }
            } else {
                let (fill, border, text) = (
                    theme::BTN_BG_DEFAULT,
                    theme::BTN_BORDER_DEFAULT,
                    theme::BTN_TEXT_DEFAULT,
                );
                if is_hovered {
                    (brighten(fill, 20), brighten(border, 20), text)
                } else {
                    (fill, border, text)
                }
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

    unsafe fn paint_result(&self, hdc: HDC, plan: &LayoutPlan, result: &TradeResult) {
        let rect_width = plan.title_bar.right;

        // ── 物品详情区 ──
        let _details_end_y = self.paint_item_details(
            hdc,
            RECT {
                left: 0,
                top: 0,
                right: rect_width,
                bottom: plan.item_details.bottom,
            },
            result,
        );

        // ── 词缀筛选区 ──
        let _mods_end_y = self.paint_modifier_list(
            hdc,
            RECT {
                left: 0,
                top: 0,
                right: rect_width,
                bottom: plan.modifiers.bottom,
            },
            result,
            plan.modifiers.top,
        );

        // ── 筛选状态 ──
        let visible_rows = visible_row_count(&plan.table_body);
        let page_size = visible_rows.max(1);
        let pages = max(1, result.entries.len().div_ceil(page_size));
        let page = min(self.page, pages - 1);
        let detected_mods = result.item.mods.len();
        let matched_mods = result
            .item
            .mods
            .iter()
            .filter(|item_mod| item_mod.stat_id.is_some())
            .count();
        let filter_line = if detected_mods == 0 {
            if result.item.rarity == "Normal" || result.item.rarity == "普通" {
                "普通物品没有可筛选词缀".to_string()
            } else {
                "未解析到词缀".to_string()
            }
        } else if matched_mods == 0 {
            "未识别到词缀".to_string()
        } else {
            format!(
                "已识别 {}/{} 条交易属性   第 {}/{} 页",
                matched_mods,
                detected_mods,
                page + 1,
                pages
            )
        };
        draw_text(
            hdc,
            &filter_line,
            RECT {
                left: 18,
                top: plan.filter_status.top,
                right: rect_width - 18,
                bottom: plan.filter_status.top + 14,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        // ── 筛选提示 ──
        if self.filters_dirty {
            let hint_y = plan.filter_status.top + 14;
            draw_text(
                hdc,
                "条件已修改 — 点击重新搜索以应用",
                RECT {
                    left: 18,
                    top: hint_y,
                    right: rect_width - 18,
                    bottom: hint_y + 16,
                },
                theme::WARNING_TEXT,
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }

        let summary_y = plan.price_summary.top;

        // ── 价格摘要区 ──
        let priced = result
            .entries
            .iter()
            .map(|entry| entry.price.as_str())
            .find(|price| !price.is_empty() && *price != "未标价")
            .unwrap_or("无标价");

        let total_text = result.total.to_string();

        let distribution = result
            .summary
            .get(1)
            .map(|line| line.replace("分布: ", ""))
            .or_else(|| result.summary.first().cloned())
            .unwrap_or_else(|| "暂无分布".to_string());

        let summary_h = 44;
        let gap = 10;
        let card_w = (rect_width - 32 - gap * 2) / 3;

        let card1_x = 16;
        let card2_x = card1_x + card_w + gap;
        let card3_x = card2_x + card_w + gap;

        // 最低价卡片
        rounded_rect(
            hdc,
            RECT {
                left: card1_x,
                top: summary_y,
                right: card1_x + card_w,
                bottom: summary_y + summary_h,
            },
            theme::BG_CARD,
            theme::CARD_BORDER,
            8,
        );
        draw_text(
            hdc,
            "最低价",
            RECT {
                left: card1_x + 8,
                top: summary_y + 4,
                right: card1_x + card_w - 8,
                bottom: summary_y + 20,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            priced,
            RECT {
                left: card1_x + 8,
                top: summary_y + 22,
                right: card1_x + card_w - 8,
                bottom: summary_y + summary_h - 4,
            },
            theme::METRIC_PRICE,
            self.fonts.bold,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_END_ELLIPSIS,
        );

        // 挂单数卡片
        rounded_rect(
            hdc,
            RECT {
                left: card2_x,
                top: summary_y,
                right: card2_x + card_w,
                bottom: summary_y + summary_h,
            },
            theme::BG_CARD,
            theme::CARD_BORDER,
            8,
        );
        draw_text(
            hdc,
            "挂单数",
            RECT {
                left: card2_x + 8,
                top: summary_y + 4,
                right: card2_x + card_w - 8,
                bottom: summary_y + 20,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            &total_text,
            RECT {
                left: card2_x + 8,
                top: summary_y + 22,
                right: card2_x + card_w - 8,
                bottom: summary_y + summary_h - 4,
            },
            theme::METRIC_COUNT,
            self.fonts.bold,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_END_ELLIPSIS,
        );

        // 常见价格卡片
        rounded_rect(
            hdc,
            RECT {
                left: card3_x,
                top: summary_y,
                right: card3_x + card_w,
                bottom: summary_y + summary_h,
            },
            theme::BG_CARD,
            theme::CARD_BORDER,
            8,
        );
        draw_text(
            hdc,
            "常见价格",
            RECT {
                left: card3_x + 8,
                top: summary_y + 4,
                right: card3_x + card_w - 8,
                bottom: summary_y + 20,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            &distribution,
            RECT {
                left: card3_x + 8,
                top: summary_y + 22,
                right: card3_x + card_w - 8,
                bottom: summary_y + summary_h - 4,
            },
            theme::METRIC_DISTRIBUTION,
            self.fonts.bold,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_END_ELLIPSIS,
        );

        // ── 挂单列表 ──
        let table_top = plan.table_header.top;
        let header_height = (plan.table_header.bottom - plan.table_header.top).max(28);
        let row_height = 24;

        // 列宽计算（使用统一的 compute_column_layout）
        let col = compute_column_layout(plan.table_header.right - 20);

        let header_left = 20;
        let col_level_x = header_left;
        let col_price_x = col_level_x + col.level;
        let col_status_x = col_price_x + col.price;
        let col_time_x = col_status_x + col.status;
        let col_seller_x = col_time_x + col.time;
        let col_action_x = col_seller_x + col.seller;

        // 根据查询状态渲染表格区域
        match &self.view.query_state {
            Some(QueryState::Loading) => {
                // 绘制表头（简化版）
                rounded_rect(
                    hdc,
                    RECT {
                        left: header_left,
                        top: table_top,
                        right: col_action_x + col.action,
                        bottom: table_top + header_height,
                    },
                    theme::BG_TABLE_HEADER,
                    theme::BTN_BORDER_DEFAULT,
                    8,
                );
                draw_text(
                    hdc,
                    "挂单列表",
                    RECT {
                        left: header_left,
                        top: table_top,
                        right: col_action_x + col.action,
                        bottom: table_top + header_height,
                    },
                    theme::TEXT_HEADER,
                    self.fonts.bold,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
                // 表格区域显示"查询中"
                draw_text(
                    hdc,
                    "正在请求国服市集…",
                    RECT {
                        left: 18,
                        top: table_top + header_height + 8,
                        right: rect_width - 18,
                        bottom: plan.table_body.bottom,
                    },
                    theme::TEXT_MUTED,
                    self.fonts.normal,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
                return;
            }
            Some(QueryState::Error(msg)) => {
                draw_text(
                    hdc,
                    &format!("查询失败: {}", msg),
                    RECT {
                        left: 18,
                        top: table_top + header_height + 8,
                        right: rect_width - 18,
                        bottom: plan.table_body.bottom,
                    },
                    theme::WARNING_TEXT,
                    self.fonts.normal,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
            }
            Some(QueryState::Empty) | None => {
                // 正常渲染表格（None 兼容旧逻辑）
            }
            _ => {}
        }

        // 排序方向箭头
        let price_arrow = match self.current_sort {
            SortOrder::PriceAsc => " ▲",
            SortOrder::PriceDesc => " ▼",
            _ => "",
        };
        let level_arrow = match self.current_sort {
            SortOrder::ItemLevelAsc => " ▲",
            SortOrder::ItemLevelDesc => " ▼",
            _ => "",
        };
        let time_arrow = match self.current_sort {
            SortOrder::IndexedTimeAsc => " ▲",
            SortOrder::IndexedTimeDesc => " ▼",
            _ => "",
        };

        // 表头
        rounded_rect(
            hdc,
            RECT {
                left: header_left,
                top: table_top,
                right: col_action_x + col.action,
                bottom: table_top + header_height,
            },
            theme::BG_TABLE_HEADER,
            theme::BTN_BORDER_DEFAULT,
            8,
        );
        // 等级表头
        let level_header = if level_arrow.is_empty() {
            "等级".to_string()
        } else {
            format!("等级{}", level_arrow)
        };
        draw_text(
            hdc,
            &level_header,
            RECT {
                left: col_level_x,
                top: table_top,
                right: col_level_x + col.level,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        // 价格表头
        let price_header = if price_arrow.is_empty() {
            "价格".to_string()
        } else {
            format!("价格{}", price_arrow)
        };
        draw_text(
            hdc,
            &price_header,
            RECT {
                left: col_price_x,
                top: table_top,
                right: col_price_x + col.price,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        // 状态表头
        draw_text(
            hdc,
            "状态",
            RECT {
                left: col_status_x,
                top: table_top,
                right: col_status_x + col.status,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        // 上架时间表头
        let time_header = if time_arrow.is_empty() {
            "上架时间".to_string()
        } else {
            format!("上架{}", time_arrow)
        };
        draw_text(
            hdc,
            &time_header,
            RECT {
                left: col_time_x,
                top: table_top,
                right: col_time_x + col.time,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        // 卖家表头
        draw_text(
            hdc,
            "卖家",
            RECT {
                left: col_seller_x,
                top: table_top,
                right: col_seller_x + col.seller,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        // 操作表头
        draw_text(
            hdc,
            "操作",
            RECT {
                left: col_action_x,
                top: table_top,
                right: col_action_x + col.action,
                bottom: table_top + header_height,
            },
            theme::TEXT_HEADER,
            self.fonts.bold,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
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
                    left: 18,
                    top: table_top + header_height + 14,
                    right: rect_width - 18,
                    bottom: table_top + header_height + 50,
                },
                theme::WARNING_TEXT,
                self.fonts.normal,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            return;
        }

        // 排序后分页显示
        let visible =
            visible_listing_indices(&result.entries, self.current_sort, self.page, page_size);

        for (idx, (_orig_idx, entry)) in visible.iter().enumerate() {
            let top = table_top + header_height + idx as i32 * row_height;
            let bg = if idx % 2 == 0 {
                theme::BG_ROW_EVEN
            } else {
                theme::BG_ROW_ODD
            };
            let row_rect = RECT {
                left: header_left,
                top,
                right: col_action_x + col.action,
                bottom: top + row_height,
            };
            if !visible.is_empty() && idx == visible.len() - 1 {
                rounded_rect(hdc, row_rect, bg, bg, 8);
            } else {
                fill(hdc, row_rect, bg);
            }

            // 等级列
            let level_text = entry
                .item_level
                .map(|lv| lv.to_string())
                .unwrap_or_else(|| "-".to_string());
            draw_text(
                hdc,
                &level_text,
                RECT {
                    left: col_level_x,
                    top,
                    right: col_level_x + col.level,
                    bottom: top + row_height,
                },
                theme::TEXT_MUTED,
                self.fonts.small,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 价格列
            draw_text(
                hdc,
                &entry.price,
                RECT {
                    left: col_price_x,
                    top,
                    right: col_price_x + col.price,
                    bottom: top + row_height,
                },
                theme::ROW_PRICE,
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );

            // 状态列
            let status_text = match entry.online {
                Some(true) => "●",
                Some(false) => "●",
                None => "-",
            };
            let status_color = match entry.online {
                Some(true) => theme::BTN_BORDER_PRIMARY,
                Some(false) => theme::TEXT_HINT,
                None => theme::TEXT_MUTED,
            };
            draw_text(
                hdc,
                status_text,
                RECT {
                    left: col_status_x,
                    top,
                    right: col_status_x + col.status,
                    bottom: top + row_height,
                },
                status_color,
                self.fonts.small,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 上架时间列
            let time_text = entry
                .indexed_time
                .as_deref()
                .map(relative_time_ago)
                .unwrap_or_else(|| "-".to_string());
            draw_text(
                hdc,
                &time_text,
                RECT {
                    left: col_time_x,
                    top,
                    right: col_time_x + col.time,
                    bottom: top + row_height,
                },
                theme::TEXT_MUTED,
                self.fonts.small,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 卖家列
            draw_text(
                hdc,
                &entry.seller,
                RECT {
                    left: col_seller_x,
                    top,
                    right: col_seller_x + col.seller,
                    bottom: top + row_height,
                },
                theme::ROW_SELLER,
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
        }
    }

    unsafe fn paint_item_details(&self, hdc: HDC, rect: RECT, result: &TradeResult) -> i32 {
        let item = &result.item;
        let mut y = 56;

        let rarity_color = match item.rarity_raw.as_str() {
            "魔法" | "Magic" => rgb(100, 149, 237),
            "稀有" | "Rare" => rgb(216, 179, 93),
            "传奇" | "Unique" => rgb(175, 96, 37),
            _ => rgb(200, 200, 200),
        };

        draw_text(
            hdc,
            &item.name,
            RECT {
                left: 16,
                top: y,
                right: rect.right - 16,
                bottom: y + 22,
            },
            rarity_color,
            self.fonts.title,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 22;

        let base_info = format!("{}   {}", item.base_type, item.rarity_raw);
        draw_text(
            hdc,
            &base_info,
            RECT {
                left: 16,
                top: y,
                right: rect.right - 16,
                bottom: y + 18,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 20;

        let mut has_basic = false;
        if let Some(q) = item.quality {
            draw_text(
                hdc,
                &format!("品质: +{}%", q),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
            has_basic = true;
        }
        if let Some(rl) = item.required_level {
            draw_text(
                hdc,
                &format!("需求等级: {}", rl),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
            has_basic = true;
        }
        if let Some(il) = item.item_level {
            draw_text(
                hdc,
                &format!("物品等级: {}", il),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
            has_basic = true;
        }

        if has_basic {
            y += 2;
        }

        if item.is_weapon() {
            if let (Some(min), Some(max)) = (item.physical_damage_min, item.physical_damage_max) {
                draw_text(
                    hdc,
                    &format!("物理伤害: {}-{}", min, max),
                    RECT {
                        left: 16,
                        top: y,
                        right: rect.right - 16,
                        bottom: y + 20,
                    },
                    theme::TEXT_BRIGHT,
                    self.fonts.small,
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
                y += 20;
            }

            for (label, min, max) in &[
                ("火焰伤害", item.fire_damage_min, item.fire_damage_max),
                ("冰霜伤害", item.cold_damage_min, item.cold_damage_max),
                (
                    "闪电伤害",
                    item.lightning_damage_min,
                    item.lightning_damage_max,
                ),
                ("混沌伤害", item.chaos_damage_min, item.chaos_damage_max),
            ] {
                if let (Some(min), Some(max)) = (min, max) {
                    draw_text(
                        hdc,
                        &format!("{label}: {}-{}", min, max),
                        RECT {
                            left: 16,
                            top: y,
                            right: rect.right - 16,
                            bottom: y + 20,
                        },
                        theme::TEXT_BRIGHT,
                        self.fonts.small,
                        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    y += 20;
                }
            }

            if let Some(crit) = item.critical_strike_chance {
                draw_text(
                    hdc,
                    &format!("暴击率: {:.2}%", crit),
                    RECT {
                        left: 16,
                        top: y,
                        right: rect.right - 16,
                        bottom: y + 20,
                    },
                    theme::TEXT_BRIGHT,
                    self.fonts.small,
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
                y += 20;
            }

            if let Some(aps) = item.attacks_per_second {
                draw_text(
                    hdc,
                    &format!("每秒攻击次数: {:.2}", aps),
                    RECT {
                        left: 16,
                        top: y,
                        right: rect.right - 16,
                        bottom: y + 20,
                    },
                    theme::TEXT_BRIGHT,
                    self.fonts.small,
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
                y += 20;
            }

            let has_phys_dps = item.physical_dps().is_some();
            let has_elem_dps = item.elemental_dps().is_some();
            if has_phys_dps || has_elem_dps {
                y += 2;
                fill(
                    hdc,
                    RECT {
                        left: 16,
                        top: y,
                        right: rect.right - 16,
                        bottom: y + 1,
                    },
                    theme::BTN_BORDER_DEFAULT,
                );
                y += 6;

                if let Some(dps) = item.physical_dps() {
                    draw_text(
                        hdc,
                        &format!("物理 DPS: {:.1}", dps),
                        RECT {
                            left: 16,
                            top: y,
                            right: rect.right - 16,
                            bottom: y + 20,
                        },
                        theme::METRIC_PRICE,
                        self.fonts.bold,
                        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    y += 20;
                }
                if let Some(dps) = item.elemental_dps() {
                    draw_text(
                        hdc,
                        &format!("元素 DPS: {:.1}", dps),
                        RECT {
                            left: 16,
                            top: y,
                            right: rect.right - 16,
                            bottom: y + 20,
                        },
                        theme::METRIC_COUNT,
                        self.fonts.bold,
                        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    y += 20;
                }
                if let Some(dps) = item.total_dps() {
                    draw_text(
                        hdc,
                        &format!("总 DPS: {:.1}", dps),
                        RECT {
                            left: 16,
                            top: y,
                            right: rect.right - 16,
                            bottom: y + 20,
                        },
                        theme::METRIC_DISTRIBUTION,
                        self.fonts.bold,
                        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    y += 20;
                }
            }
        }

        let mut has_defense = false;
        if let Some(a) = item.armour {
            has_defense = true;
            draw_text(
                hdc,
                &format!("护甲: {}", a),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
        }
        if let Some(e) = item.evasion {
            if !has_defense {
                has_defense = true;
            }
            draw_text(
                hdc,
                &format!("闪避: {}", e),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
        }
        if let Some(es) = item.energy_shield {
            if !has_defense {
                has_defense = true;
            }
            draw_text(
                hdc,
                &format!("能量护盾: {}", es),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
        }
        if has_defense {
            y += 2;
        }

        if let Some(sockets) = &item.sockets {
            draw_text(
                hdc,
                &format!("插槽: {}", sockets),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 20,
                },
                theme::TEXT_BRIGHT,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
        }

        y
    }

    unsafe fn paint_modifier_list(
        &self,
        hdc: HDC,
        rect: RECT,
        result: &TradeResult,
        start_y: i32,
    ) -> i32 {
        let item_mods = &result.item.mods;
        let max_display = 8;

        let selected_patterns = self.query_options.selected_mod_patterns.as_ref();
        let use_mods = self.query_options.use_mods;

        let mut y = start_y + 4;

        for (_i, item_mod) in item_mods.iter().enumerate().take(max_display) {
            let is_selected = if use_mods {
                selected_patterns
                    .map(|p| p.iter().any(|pat| pat == &item_mod.pattern))
                    .unwrap_or(true)
            } else {
                false
            };
            let has_stat_id = item_mod.stat_id.is_some();

            let label = item_mod
                .stat_text
                .as_deref()
                .unwrap_or(item_mod.text.as_str());
            let display_label = if has_stat_id {
                label.to_string()
            } else {
                format!("{} (未识别)", label)
            };

            let (border_color, text_color) = if !has_stat_id {
                (theme::BTN_BORDER_DEFAULT, theme::TEXT_HINT)
            } else if is_selected {
                (theme::BTN_BORDER_PRIMARY, theme::BTN_BORDER_PRIMARY)
            } else {
                (theme::BTN_BORDER_DEFAULT, theme::BTN_TEXT_DEFAULT)
            };

            let row_rect = RECT {
                left: 16,
                top: y,
                right: rect.right - 16,
                bottom: y + 22,
            };
            rounded_rect(hdc, row_rect, theme::BG_CARD, border_color, 6);
            draw_text(
                hdc,
                &display_label,
                RECT {
                    left: 24,
                    top: y,
                    right: rect.right - 24,
                    bottom: y + 22,
                },
                text_color,
                self.fonts.small,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 24;
        }

        if item_mods.len() > max_display {
            let remaining = item_mods.len() - max_display;
            draw_text(
                hdc,
                &format!("... 还有 {} 条词缀", remaining),
                RECT {
                    left: 16,
                    top: y,
                    right: rect.right - 16,
                    bottom: y + 18,
                },
                theme::TEXT_MUTED,
                self.fonts.small,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            y += 20;
        }

        draw_text(
            hdc,
            "点击词缀选择，然后点击重新搜索",
            RECT {
                left: 16,
                top: y,
                right: rect.right - 16,
                bottom: y + 16,
            },
            theme::TEXT_HINT,
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        y += 18;

        y
    }
}

impl UiState {
    /// 实际绘制到指定 DC
    unsafe fn paint_to_dc(&self, hdc: HDC, rect: RECT) {
        // 整体背景
        fill(hdc, rect, theme::BG_DARK);

        // 标题栏背景
        fill(
            hdc,
            RECT {
                left: 0,
                top: 0,
                right: rect.right,
                bottom: 52,
            },
            theme::BG_HEADER,
        );

        // 左侧绿色竖线
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

        // 标题文字
        draw_text(
            hdc,
            &self.view.title,
            RECT {
                left: 16,
                top: 8,
                right: rect.right - 170,
                bottom: 30,
            },
            theme::TEXT_BRIGHT,
            self.fonts.title,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        // 副标题文字
        draw_text(
            hdc,
            &self.view.subtitle,
            RECT {
                left: 16,
                top: 30,
                right: rect.right - 170,
                bottom: 50,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        match &self.view.kind {
            ViewKind::Message(lines) => self.paint_message(hdc, rect, lines),
            ViewKind::Result(result) => {
                self.paint_value_tier_badge(hdc, rect, result.value_tier);
                let detail_lines = compute_item_detail_lines(&result.item);
                let plan = LayoutPlan::compute(rect, detail_lines, result.item.mods.len());
                self.paint_result(hdc, &plan, result);
                let is_loading = matches!(self.view.query_state, Some(QueryState::Loading));
                let specs = self.button_specs(rect);
                // Loading 状态下禁用翻页和 Whisper，但保留其他按钮
                let filtered_specs: Vec<_> = if is_loading {
                    specs
                        .into_iter()
                        .map(|mut spec| {
                            match spec.button {
                                UiButton::Prev | UiButton::Next | UiButton::Whisper(_) => {
                                    spec.enabled = false;
                                }
                                _ => {}
                            }
                            spec
                        })
                        .collect()
                } else {
                    specs
                };
                self.paint_buttons(hdc, &filtered_specs);
                // 底部状态栏
                if is_loading {
                    // Loading 状态：左侧显示提示，右侧显示"查询中"
                    draw_text(
                        hdc,
                        "正在请求国服市集… · Esc 关闭",
                        RECT {
                            left: 16,
                            top: rect.bottom - 44,
                            right: rect.right - 200,
                            bottom: rect.bottom - 14,
                        },
                        theme::TEXT_HINT,
                        self.fonts.small,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    draw_text(
                        hdc,
                        "查询中...",
                        RECT {
                            left: rect.right - 200,
                            top: rect.bottom - 44,
                            right: rect.right - 16,
                            bottom: rect.bottom - 14,
                        },
                        theme::TEXT_MUTED,
                        self.fonts.small,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                } else {
                    // 左侧：快捷键提示
                    draw_text(
                        hdc,
                        "←→ 翻页 · M 属性 · V 数值 · Esc 关闭",
                        RECT {
                            left: 16,
                            top: rect.bottom - 44,
                            right: rect.right - 200,
                            bottom: rect.bottom - 14,
                        },
                        theme::TEXT_HINT,
                        self.fonts.small,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                    // 右侧：完整状态文字
                    let page_range = if result.entries.is_empty() {
                        String::new()
                    } else {
                        let page_size = visible_row_count(&plan.table_body).max(1);
                        let start = self.page * page_size + 1;
                        let end = ((self.page + 1) * page_size).min(result.entries.len());
                        format!("查询到 {} 条，当前 {}-{}", result.total, start, end)
                    };
                    draw_text(
                        hdc,
                        &page_range,
                        RECT {
                            left: rect.right - 200,
                            top: rect.bottom - 44,
                            right: rect.right - 16,
                            bottom: rect.bottom - 14,
                        },
                        theme::TEXT_MUTED,
                        self.fonts.small,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
                    );
                }
                return;
            }
        }

        self.paint_buttons(hdc, &self.button_specs(rect));

        // 底部状态栏
        let status = if self.view.status.is_empty() {
            if self.pinned {
                "面板已固定".to_string()
            } else {
                String::new()
            }
        } else {
            self.view.status.clone()
        };

        // 左侧：快捷键提示
        draw_text(
            hdc,
            "←→ 翻页 · M 属性 · V 数值 · Esc 关闭",
            RECT {
                left: 16,
                top: rect.bottom - 44,
                right: rect.right - 200,
                bottom: rect.bottom - 14,
            },
            theme::TEXT_HINT,
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );

        // 右侧：状态文字
        draw_text(
            hdc,
            &status,
            RECT {
                left: rect.right - 200,
                top: rect.bottom - 44,
                right: rect.right - 16,
                bottom: rect.bottom - 14,
            },
            theme::TEXT_MUTED,
            self.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
}
