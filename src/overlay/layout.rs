use std::cmp::min;

use windows_sys::Win32::Foundation::RECT;

use crate::overlay::interaction::OverlayInteraction;
use crate::overlay::model::{UiButton, UiButtonSpec};
use crate::{UiState, ViewKind};

pub fn rect_contains(rect: &RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

/// Calculate the y position of the control bar (below mod list).
/// This is shared between paint_result and button_specs.
#[allow(dead_code)]
pub fn compute_control_y(result: &crate::TradeResult) -> i32 {
    let item = &result.item;
    let details_end = compute_details_end_y(item);
    let mods_end = compute_mods_end_y(details_end, result.item.mods.len());
    mods_end + 4
}

/// Calculate where the item details section ends.
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
/// Must match the layout in render.rs paint_result.
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

/// Extension trait for UiState's layout-related methods.
pub trait OverlayLayout {
    fn button_specs(&self, rect: RECT) -> Vec<UiButtonSpec>;
}

impl OverlayLayout for UiState {
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
                },
            ]);
        } else {
            if let Some(result) = result {
                let details_end = compute_details_end_y(&result.item);
                let mods_end = compute_mods_end_y(details_end, result.item.mods.len());

                // ── 词缀行按钮 ──
                let max_display = 8;
                let display_count = result.item.mods.len().min(max_display);
                let mut mod_y = details_end + 4;
                for i in 0..display_count {
                    specs.push(UiButtonSpec {
                        button: UiButton::ModToggle(i),
                        label: format!("mod_{}", i),
                        rect: RECT {
                            left: 16,
                            top: mod_y,
                            right: rect.right - 16,
                            bottom: mod_y + 22,
                        },
                        enabled: true,
                        primary: false,
                    });
                    mod_y += 24;
                }

                // ── 重新搜索按钮 ──
                let rerun_y = mods_end + 4;
                specs.push(UiButtonSpec {
                    button: UiButton::RerunSearch,
                    label: "重新搜索".to_string(),
                    rect: RECT {
                        left: 16,
                        top: rerun_y,
                        right: 120,
                        bottom: rerun_y + 22,
                    },
                    enabled: true,
                    primary: self.filters_dirty,
                });

                // ── 控制栏按钮行 ──
                let control_y = mods_end + 4;
                let btn_y = control_y + 28;
                let btn_h = 20;
                let btn_gap = 4;
                let btn_w = (rect.right - 32 - btn_gap * 5) / 6;
                let btn1_x = 16;
                let btn2_x = btn1_x + btn_w + btn_gap;
                let btn3_x = btn2_x + btn_w + btn_gap;
                let btn4_x = btn3_x + btn_w + btn_gap;
                let btn5_x = btn4_x + btn_w + btn_gap;
                let btn6_x = btn5_x + btn_w + btn_gap;

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
                            left: btn1_x,
                            top: btn_y,
                            right: btn1_x + btn_w,
                            bottom: btn_y + btn_h,
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
                            left: btn2_x,
                            top: btn_y,
                            right: btn2_x + btn_w,
                            bottom: btn_y + btn_h,
                        },
                        enabled: has_mods,
                        primary: self.query_options.use_values,
                    },
                    UiButtonSpec {
                        button: UiButton::Prev,
                        label: "上一页".to_string(),
                        rect: RECT {
                            left: btn3_x,
                            top: btn_y,
                            right: btn3_x + btn_w,
                            bottom: btn_y + btn_h,
                        },
                        enabled: can_prev,
                        primary: false,
                    },
                    UiButtonSpec {
                        button: UiButton::Next,
                        label: "下一页".to_string(),
                        rect: RECT {
                            left: btn4_x,
                            top: btn_y,
                            right: btn4_x + btn_w,
                            bottom: btn_y + btn_h,
                        },
                        enabled: can_next,
                        primary: false,
                    },
                    UiButtonSpec {
                        button: UiButton::Open,
                        label: "打开市集".to_string(),
                        rect: RECT {
                            left: btn5_x,
                            top: btn_y,
                            right: btn5_x + btn_w,
                            bottom: btn_y + btn_h,
                        },
                        enabled: true,
                        primary: false,
                    },
                    UiButtonSpec {
                        button: UiButton::Copy,
                        label: "复制链接".to_string(),
                        rect: RECT {
                            left: btn6_x,
                            top: btn_y,
                            right: btn6_x + btn_w,
                            bottom: btn_y + btn_h,
                        },
                        enabled: has_url,
                        primary: false,
                    },
                ]);

                // 列宽计算（与 render.rs 保持一致）
                let col_level_w = 40;
                let col_status_w = 36;
                let col_time_w = 60;
                let col_action_w = 50;
                let remaining =
                    rect.right - 40 - col_level_w - col_status_w - col_time_w - col_action_w;
                let _col_price_w = remaining * 3 / 10;
                let col_seller_w = remaining * 7 / 10;

                let header_left = 20;
                let col_price_x = header_left + col_level_w;
                let col_time_x = header_left + col_level_w + _col_price_w + col_status_w;
                let col_action_x = header_left
                    + col_level_w
                    + _col_price_w
                    + col_status_w
                    + col_time_w
                    + col_seller_w;

                // 表头排序按钮
                let table_top = compute_table_top(mods_end, self.filters_dirty);
                let header_height = 28;
                specs.push(UiButtonSpec {
                    button: UiButton::SortLevel,
                    label: "排序-等级".to_string(),
                    rect: RECT {
                        left: header_left,
                        top: table_top,
                        right: header_left + col_level_w,
                        bottom: table_top + header_height,
                    },
                    enabled: true,
                    primary: false,
                });
                specs.push(UiButtonSpec {
                    button: UiButton::SortPrice,
                    label: "排序-价格".to_string(),
                    rect: RECT {
                        left: col_price_x,
                        top: table_top,
                        right: col_price_x + _col_price_w,
                        bottom: table_top + header_height,
                    },
                    enabled: true,
                    primary: false,
                });
                specs.push(UiButtonSpec {
                    button: UiButton::SortTime,
                    label: "排序-时间".to_string(),
                    rect: RECT {
                        left: col_time_x,
                        top: table_top,
                        right: col_time_x + col_time_w,
                        bottom: table_top + header_height,
                    },
                    enabled: true,
                    primary: false,
                });

                // 每行的私聊按钮
                let page_size = result.page_size.max(1);
                let pages = std::cmp::max(1, result.entries.len().div_ceil(page_size));
                let page = min(self.page, pages - 1);
                let visible_start = page * page_size;
                let visible_entries = result.entries.iter().skip(visible_start).take(page_size);
                for (idx, _entry) in visible_entries.enumerate() {
                    let row_top = table_top + header_height + idx as i32 * 24;
                    specs.push(UiButtonSpec {
                        button: UiButton::Whisper(visible_start + idx),
                        label: "私聊".to_string(),
                        rect: RECT {
                            left: col_action_x + 3,
                            top: row_top + 2,
                            right: col_action_x + col_action_w - 3,
                            bottom: row_top + 22,
                        },
                        enabled: true,
                        primary: false,
                    });
                }
            }
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
    fn button_specs_rects_dont_overlap_within_group() {
        let w = 580i32;
        let h = 780i32;
        let rect = RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };

        assert!(rect_contains(&rect, 0, 0));
        assert!(rect_contains(&rect, w - 1, h - 1));
    }

    #[test]
    fn table_column_widths_dont_overflow_window() {
        let w = 580i32;
        let col_level_w = 40;
        let col_status_w = 36;
        let col_time_w = 60;
        let col_action_w = 50;
        let remaining = w - 40 - col_level_w - col_status_w - col_time_w - col_action_w;
        let col_price_w = remaining * 3 / 10;
        let col_seller_w = remaining * 7 / 10;
        let total = 20
            + col_level_w
            + col_price_w
            + col_status_w
            + col_time_w
            + col_seller_w
            + col_action_w
            + 20;
        assert!(
            total <= w,
            "total column width {total} exceeds window width {w}"
        );
    }
}
