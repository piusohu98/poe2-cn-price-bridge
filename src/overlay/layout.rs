use windows_sys::Win32::Foundation::RECT;

use crate::overlay::interaction::OverlayInteraction;
use crate::overlay::model::{UiButton, UiButtonSpec};
use crate::{UiState, ViewKind, visible_listing_indices};

pub fn rect_contains(rect: &RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

// ── LayoutPlan ──

#[derive(Clone)]
pub struct LayoutPlan {
    pub title_bar: RECT,
    pub item_details: RECT,
    pub modifiers: RECT,
    pub filter_status: RECT,
    pub filter_actions: RECT,
    pub price_summary: RECT,
    pub table_header: RECT,
    pub table_body: RECT,
    #[allow(dead_code)]
    pub footer: RECT,
}

impl LayoutPlan {
    pub fn compute(rect: RECT, item_detail_lines: i32, modifier_count: usize) -> Self {
        let mut y = 0;
        let title_bar = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + 52,
        };
        y = 56;
        let item_details = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + item_detail_lines * 20 + 8,
        };
        y = item_details.bottom + 4;
        let mod_count = modifier_count.min(8) as i32;
        let modifiers = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + mod_count * 22 + 28,
        };
        y = modifiers.bottom + 4;
        let filter_status = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + 20,
        };
        y = filter_status.bottom;
        let filter_actions = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + 24,
        };
        y = filter_actions.bottom + 4;
        let price_summary = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + 48,
        };
        y = price_summary.bottom + 4;
        let table_header = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: y + 24,
        };
        y = table_header.bottom;
        let table_body = RECT {
            left: 0,
            top: y,
            right: rect.right,
            bottom: rect.bottom - 48,
        };
        let footer = RECT {
            left: 0,
            top: table_body.bottom,
            right: rect.right,
            bottom: rect.bottom,
        };
        Self {
            title_bar,
            item_details,
            modifiers,
            filter_status,
            filter_actions,
            price_summary,
            table_header,
            table_body,
            footer,
        }
    }

    /// 计算建议窗口高度，根据内容动态调整
    /// - item_detail_lines: 物品详情行数
    /// - modifier_count: 词缀数量
    /// - entry_count: 实际挂单数
    /// - min_height: 最小高度 (默认 480)
    /// - max_height: 最大高度 (显示器工作区 90%)
    pub fn suggested_height(
        item_detail_lines: i32,
        modifier_count: usize,
        entry_count: usize,
        min_height: i32,
        max_height: i32,
    ) -> i32 {
        let fixed_height = 52 + 4 + // 标题栏
            item_detail_lines * 20 + 8 + 4 + // 物品详情
            (modifier_count.min(8) as i32) * 22 + 28 + 4 + // 词缀
            20 + 24 + 4 + // 筛选状态 + 动作
            48 + 4 + // 价格摘要
            24 + // 表头
            48; // 底部状态栏

        let row_height = 24;
        let min_table_rows = 3;
        let max_table_rows = 20;
        let table_rows = entry_count.max(min_table_rows).min(max_table_rows) as i32;
        let table_height = table_rows * row_height;

        let total = fixed_height + table_height;
        total.max(min_height).min(max_height)
    }

    /// 返回所有区域的矩形列表，用于重叠检测
    #[allow(dead_code)]
    pub fn all_rects(&self) -> Vec<RECT> {
        vec![
            self.title_bar,
            self.item_details,
            self.modifiers,
            self.filter_status,
            self.filter_actions,
            self.price_summary,
            self.table_header,
            self.table_body,
            self.footer,
        ]
    }
}

// ── 物品详情行数计算 ──

/// 计算物品详情区的行数，与 render.rs paint_item_details 保持一致
pub fn compute_item_detail_lines(item: &crate::ParsedItem) -> i32 {
    let mut lines = 0i32;
    // name
    lines += 1;
    // base type
    lines += 1;
    // basic attrs (quality / required_level / item_level)
    let mut basic_count = 0;
    if item.quality.is_some() {
        basic_count += 1;
    }
    if item.required_level.is_some() {
        basic_count += 1;
    }
    if item.item_level.is_some() {
        basic_count += 1;
    }
    if basic_count > 0 {
        lines += basic_count;
    }
    // weapon attrs
    if item.is_weapon() {
        if item.physical_damage_min.is_some() && item.physical_damage_max.is_some() {
            lines += 1;
        }
        for (min, max) in &[
            (item.fire_damage_min, item.fire_damage_max),
            (item.cold_damage_min, item.cold_damage_max),
            (item.lightning_damage_min, item.lightning_damage_max),
            (item.chaos_damage_min, item.chaos_damage_max),
        ] {
            if min.is_some() && max.is_some() {
                lines += 1;
            }
        }
        if item.critical_strike_chance.is_some() {
            lines += 1;
        }
        if item.attacks_per_second.is_some() {
            lines += 1;
        }
        let has_phys = item.physical_damage_min.is_some() && item.physical_damage_max.is_some();
        let has_elem = item.fire_damage_min.is_some()
            || item.cold_damage_min.is_some()
            || item.lightning_damage_min.is_some()
            || item.chaos_damage_min.is_some();
        if has_phys || has_elem {
            if has_phys {
                lines += 1;
            }
            if has_elem {
                lines += 1;
            }
            lines += 1; // total DPS
        }
    }
    // defense
    let mut def_count = 0;
    if item.armour.is_some() {
        def_count += 1;
    }
    if item.evasion.is_some() {
        def_count += 1;
    }
    if item.energy_shield.is_some() {
        def_count += 1;
    }
    if def_count > 0 {
        lines += def_count;
    }
    // sockets
    if item.sockets.is_some() {
        lines += 1;
    }
    lines
}

// ── 列布局 ──

#[derive(Debug, Clone, Copy)]
pub struct ColumnLayout {
    pub level: i32,
    pub price: i32,
    pub status: i32,
    pub time: i32,
    pub seller: i32,
    pub action: i32,
}

pub fn compute_column_layout(table_width: i32) -> ColumnLayout {
    let col_level_w = 40;
    let col_status_w = 36;
    let col_time_w = 60;
    let col_action_w = 50;
    let remaining = table_width - 40 - col_level_w - col_status_w - col_time_w - col_action_w;
    let col_price_w = remaining * 3 / 10;
    let col_seller_w = remaining * 7 / 10;
    ColumnLayout {
        level: col_level_w,
        price: col_price_w,
        status: col_status_w,
        time: col_time_w,
        seller: col_seller_w,
        action: col_action_w,
    }
}

// ── 可见行数 ──

pub fn visible_row_count(table_body: &RECT) -> usize {
    let available_height = table_body.bottom - table_body.top;
    let row_height = 24;
    (available_height / row_height).max(0) as usize
}

// ── 旧版兼容函数（保留给 Message 视图使用） ──

/// Calculate where the item details section ends.
#[allow(dead_code)]
fn compute_details_end_y(item: &crate::ParsedItem) -> i32 {
    let mut y = 56;
    // name
    y += 22;
    // base type
    y += 20;
    // basic attrs (quality / required_level / item_level)
    let mut basic_count = 0;
    if item.quality.is_some() {
        basic_count += 1;
    }
    if item.required_level.is_some() {
        basic_count += 1;
    }
    if item.item_level.is_some() {
        basic_count += 1;
    }
    if basic_count > 0 {
        y += basic_count * 20 + 2;
    }
    // weapon attrs
    if item.is_weapon() {
        if item.physical_damage_min.is_some() && item.physical_damage_max.is_some() {
            y += 20;
        }
        for (min, max) in &[
            (item.fire_damage_min, item.fire_damage_max),
            (item.cold_damage_min, item.cold_damage_max),
            (item.lightning_damage_min, item.lightning_damage_max),
            (item.chaos_damage_min, item.chaos_damage_max),
        ] {
            if min.is_some() && max.is_some() {
                y += 20;
            }
        }
        if item.critical_strike_chance.is_some() {
            y += 20;
        }
        if item.attacks_per_second.is_some() {
            y += 20;
        }
        let has_phys = item.physical_damage_min.is_some() && item.physical_damage_max.is_some();
        let has_elem = item.fire_damage_min.is_some()
            || item.cold_damage_min.is_some()
            || item.lightning_damage_min.is_some()
            || item.chaos_damage_min.is_some();
        if has_phys || has_elem {
            y += 2 + 1 + 6;
            if has_phys {
                y += 20;
            }
            if has_elem {
                y += 20;
            }
            y += 20; // total DPS
        }
    }
    // defense
    let mut def_count = 0;
    if item.armour.is_some() {
        def_count += 1;
    }
    if item.evasion.is_some() {
        def_count += 1;
    }
    if item.energy_shield.is_some() {
        def_count += 1;
    }
    if def_count > 0 {
        y += def_count * 20 + 2;
    }
    // sockets
    if item.sockets.is_some() {
        y += 20;
    }
    y
}

/// Calculate where the mod list ends.
#[allow(dead_code)]
fn compute_mods_end_y(details_end: i32, mod_count: usize) -> i32 {
    let mut y = details_end + 4;
    let max_display = 8;
    let display_count = mod_count.min(max_display);
    if display_count > 0 {
        y += display_count as i32 * 24;
    }
    if mod_count > max_display {
        y += 20;
    }
    if mod_count == 0 {
        y += 20;
    }
    y += 18; // hint text
    y
}

/// Calculate table_top y position from mods_end.
#[allow(dead_code)]
pub fn compute_table_top(mods_end: i32, filters_dirty: bool) -> i32 {
    let control_y = mods_end + 4;
    let hint_y = control_y + 14;
    let summary_y = if filters_dirty {
        hint_y + 22
    } else {
        hint_y + 6
    };
    summary_y + 44 + 10
}

// ── OverlayLayout trait ──

/// Extension trait for UiState's layout-related methods.
pub trait OverlayLayout {
    fn button_specs(&self, rect: RECT) -> Vec<UiButtonSpec>;
    fn button_specs_for_result(
        &self,
        plan: &LayoutPlan,
        result: &crate::TradeResult,
    ) -> Vec<UiButtonSpec>;
}

impl OverlayLayout for UiState {
    fn button_specs(&self, rect: RECT) -> Vec<UiButtonSpec> {
        let result = self.current_result();

        let mut specs = vec![
            // 固定按钮：标题栏右侧
            UiButtonSpec {
                button: UiButton::Pin,
                label: (if self.pinned { "已固定" } else { "固定" }).to_string(),
                rect: RECT {
                    left: rect.right - 128,
                    top: 10,
                    right: rect.right - 80,
                    bottom: 32,
                },
                enabled: true,
                primary: false,
                visible: true,
            },
            // 关闭按钮：标题栏右侧
            UiButtonSpec {
                button: UiButton::Close,
                label: "关闭".to_string(),
                rect: RECT {
                    left: rect.right - 72,
                    top: 10,
                    right: rect.right - 24,
                    bottom: 32,
                },
                enabled: true,
                primary: false,
                visible: true,
            },
        ];

        if matches!(self.view.kind, ViewKind::Message(_)) {
            specs.extend([
                UiButtonSpec {
                    button: UiButton::Wizard,
                    label: "向导".to_string(),
                    rect: RECT {
                        left: 16,
                        top: rect.bottom - 74,
                        right: 96,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: false,
                    visible: true,
                },
                UiButtonSpec {
                    button: UiButton::Cookie,
                    label: "设置Cookie".to_string(),
                    rect: RECT {
                        left: 104,
                        top: rect.bottom - 74,
                        right: 200,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: true,
                    visible: true,
                },
                UiButtonSpec {
                    button: UiButton::ValidateCookie,
                    label: "验证Cookie".to_string(),
                    rect: RECT {
                        left: 208,
                        top: rect.bottom - 74,
                        right: 304,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: false,
                    visible: true,
                },
                UiButtonSpec {
                    button: UiButton::Open,
                    label: "打开官网".to_string(),
                    rect: RECT {
                        left: 312,
                        top: rect.bottom - 74,
                        right: 408,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: false,
                    visible: true,
                },
                UiButtonSpec {
                    button: UiButton::History,
                    label: "历史".to_string(),
                    rect: RECT {
                        left: 416,
                        top: rect.bottom - 74,
                        right: 496,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: false,
                    visible: true,
                },
                UiButtonSpec {
                    button: UiButton::Diagnostics,
                    label: "诊断".to_string(),
                    rect: RECT {
                        left: 504,
                        top: rect.bottom - 74,
                        right: 584,
                        bottom: rect.bottom - 44,
                    },
                    enabled: true,
                    primary: false,
                    visible: true,
                },
            ]);
        } else if let Some(result) = result {
            let detail_lines = compute_item_detail_lines(&result.item);
            let plan = LayoutPlan::compute(rect, detail_lines, result.item.mods.len());
            specs.extend(self.button_specs_for_result(&plan, result));
        }
        specs
    }

    fn button_specs_for_result(
        &self,
        plan: &LayoutPlan,
        result: &crate::TradeResult,
    ) -> Vec<UiButtonSpec> {
        let has_url = !self.view.current_url.is_empty();
        let has_mods = !result.item.mods.is_empty();
        let visible_rows = visible_row_count(&plan.table_body);
        let page_size = visible_rows.max(1);
        let can_prev = self.page > 0;
        let can_next = (self.page + 1) * page_size < result.entries.len();

        let mut specs = Vec::new();

        // 标题栏热区（不可见）
        specs.push(UiButtonSpec {
            button: UiButton::Backdrop,
            rect: plan.title_bar,
            label: String::new(),
            enabled: true,
            primary: false,
            visible: false,
        });

        // 词缀热区（不可见，paint_modifier_list 单独绘制）
        let mod_count = result.item.mods.len().min(8);
        for i in 0..mod_count {
            let y = plan.modifiers.top + 4 + i as i32 * 22;
            specs.push(UiButtonSpec {
                button: UiButton::ModToggle(i),
                rect: RECT {
                    left: 16,
                    top: y,
                    right: plan.modifiers.right - 16,
                    bottom: y + 22,
                },
                label: String::new(),
                enabled: true,
                primary: false,
                visible: false,
            });
        }

        // 重新搜索按钮（可见，在词缀区底部）
        specs.push(UiButtonSpec {
            button: UiButton::RerunSearch,
            label: "重新搜索".to_string(),
            rect: RECT {
                left: 16,
                top: plan.modifiers.bottom - 24,
                right: 120,
                bottom: plan.modifiers.bottom,
            },
            enabled: true,
            primary: self.filters_dirty,
            visible: true,
        });

        // 筛选动作按钮（可见）
        let act = &plan.filter_actions;
        let btn_w = (act.right - 32 - 5 * 4) / 6;
        let mut x = 16;
        specs.push(UiButtonSpec {
            button: UiButton::Mods,
            label: (if self.query_options.use_mods {
                "同属性开"
            } else {
                "同属性"
            })
            .to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: has_mods,
            primary: self.query_options.use_mods,
            visible: true,
        });
        x += btn_w + 4;
        specs.push(UiButtonSpec {
            button: UiButton::Values,
            label: (if self.query_options.use_values {
                "数值开"
            } else {
                "数值"
            })
            .to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: has_mods,
            primary: self.query_options.use_values,
            visible: true,
        });
        x += btn_w + 4;
        specs.push(UiButtonSpec {
            button: UiButton::Prev,
            label: "上一页".to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: can_prev,
            primary: false,
            visible: true,
        });
        x += btn_w + 4;
        specs.push(UiButtonSpec {
            button: UiButton::Next,
            label: "下一页".to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: can_next,
            primary: false,
            visible: true,
        });
        x += btn_w + 4;
        specs.push(UiButtonSpec {
            button: UiButton::OpenTrade,
            label: "打开市集".to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: true,
            primary: false,
            visible: true,
        });
        x += btn_w + 4;
        specs.push(UiButtonSpec {
            button: UiButton::Copy,
            label: "复制链接".to_string(),
            rect: RECT {
                left: x,
                top: act.top,
                right: x + btn_w,
                bottom: act.bottom,
            },
            enabled: has_url,
            primary: false,
            visible: true,
        });

        // 表头排序热区（不可见）
        let col_layout = compute_column_layout(plan.table_header.right - 20);
        let mut col_x = 20;
        specs.push(UiButtonSpec {
            button: UiButton::SortLevel,
            rect: RECT {
                left: col_x,
                top: plan.table_header.top,
                right: col_x + col_layout.level,
                bottom: plan.table_header.bottom,
            },
            label: String::new(),
            enabled: true,
            primary: false,
            visible: false,
        });
        col_x += col_layout.level;
        specs.push(UiButtonSpec {
            button: UiButton::SortPrice,
            rect: RECT {
                left: col_x,
                top: plan.table_header.top,
                right: col_x + col_layout.price,
                bottom: plan.table_header.bottom,
            },
            label: String::new(),
            enabled: true,
            primary: false,
            visible: false,
        });
        col_x += col_layout.price + col_layout.status + col_layout.time;
        // 时间排序热区
        specs.push(UiButtonSpec {
            button: UiButton::SortTime,
            rect: RECT {
                left: col_x,
                top: plan.table_header.top,
                right: col_x + col_layout.time,
                bottom: plan.table_header.bottom,
            },
            label: String::new(),
            enabled: true,
            primary: false,
            visible: false,
        });

        // 私聊按钮（可见）
        let visible_rows = visible_row_count(&plan.table_body);
        let visible =
            visible_listing_indices(&result.entries, self.current_sort, self.page, page_size);
        for (idx, (orig_idx, _entry)) in visible.iter().enumerate() {
            if idx >= visible_rows {
                break;
            }
            let ry = plan.table_body.top + 2 + idx as i32 * 24;
            let cx = plan.table_body.right - 50;
            specs.push(UiButtonSpec {
                button: UiButton::Whisper(*orig_idx),
                label: "私聊".to_string(),
                rect: RECT {
                    left: cx,
                    top: ry,
                    right: cx + 44,
                    bottom: ry + 20,
                },
                enabled: true,
                primary: false,
                visible: true,
            });
        }

        specs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_point_inside() {
        let rect = RECT {
            left: 10,
            top: 20,
            right: 100,
            bottom: 80,
        };
        assert!(rect_contains(&rect, 10, 20));
        assert!(rect_contains(&rect, 50, 50));
        assert!(rect_contains(&rect, 99, 79));
    }

    #[test]
    fn rect_contains_point_outside() {
        let rect = RECT {
            left: 10,
            top: 20,
            right: 100,
            bottom: 80,
        };
        assert!(!rect_contains(&rect, 9, 50));
        assert!(!rect_contains(&rect, 100, 50));
        assert!(!rect_contains(&rect, 50, 19));
        assert!(!rect_contains(&rect, 50, 80));
    }

    #[test]
    fn table_column_widths_dont_overflow_window() {
        let w = 580i32;
        let col = compute_column_layout(w - 20);
        let total =
            20 + col.level + col.price + col.status + col.time + col.seller + col.action + 20;
        assert!(
            total <= w,
            "total column width {total} exceeds window width {w}"
        );
    }

    #[test]
    fn layout_plan_rects_dont_overlap() {
        let rect = RECT {
            left: 0,
            top: 0,
            right: 580,
            bottom: 780,
        };
        let plan = LayoutPlan::compute(rect, 6, 6);
        let rects = plan.all_rects();
        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let a = rects[i];
                let b = rects[j];
                let overlap_x = a.left < b.right && a.right > b.left;
                let overlap_y = a.top < b.bottom && a.bottom > b.top;
                assert!(
                    !(overlap_x && overlap_y),
                    "rects {} and {} overlap: ({},{},{},{}) and ({},{},{},{})",
                    i,
                    j,
                    a.left,
                    a.top,
                    a.right,
                    a.bottom,
                    b.left,
                    b.top,
                    b.right,
                    b.bottom
                );
            }
        }
    }

    #[test]
    fn layout_plan_rects_sequential_y() {
        let rect = RECT {
            left: 0,
            top: 0,
            right: 580,
            bottom: 780,
        };
        let plan = LayoutPlan::compute(rect, 6, 6);
        let rects = plan.all_rects();
        // 验证每个区域 Y 坐标按顺序排列，不交叉
        for i in 0..rects.len() - 1 {
            assert!(
                rects[i].bottom <= rects[i + 1].top,
                "rect {} bottom {} > rect {} top {}",
                i,
                rects[i].bottom,
                i + 1,
                rects[i + 1].top
            );
        }
    }
}
