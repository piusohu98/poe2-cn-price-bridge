//! Theme color constants for the overlay UI.
//! Uses `crate::rgb` for compile-time color values.
#![allow(dead_code)]

use crate::rgb;

// ── Backgrounds ───────────────────────────────────────────────

/// Main overlay background (dark).
pub const BG_DARK: u32 = rgb(12, 13, 16);
/// Header bar background.
pub const BG_HEADER: u32 = rgb(18, 20, 24);
/// Metric card background.
pub const BG_CARD: u32 = rgb(19, 23, 29);
/// Value tier badge background.
pub const BG_BADGE: u32 = rgb(22, 26, 32);
/// Table header row background.
pub const BG_TABLE_HEADER: u32 = rgb(28, 32, 39);
/// Even data row background.
pub const BG_ROW_EVEN: u32 = rgb(18, 22, 28);
/// Odd data row background.
pub const BG_ROW_ODD: u32 = rgb(23, 27, 34);

// ── Accent line ───────────────────────────────────────────────

/// Default view accent colour (sky blue).
pub const ACCENT_DEFAULT: u32 = rgb(56, 189, 248);
/// Success / cookie-ok accent.
pub const ACCENT_SUCCESS: u32 = rgb(34, 197, 94);
/// Error accent.
pub const ACCENT_ERROR: u32 = rgb(239, 68, 68);
/// Warning accent.
pub const ACCENT_WARNING: u32 = rgb(245, 158, 11);

// ── Text ──────────────────────────────────────────────────────

/// Secondary / muted text.
pub const TEXT_MUTED: u32 = rgb(148, 163, 184);
/// Bright body text.
pub const TEXT_BRIGHT: u32 = rgb(248, 250, 252);
/// Table header / primary label text.
pub const TEXT_HEADER: u32 = rgb(203, 213, 225);
/// Keyboard shortcut hint text.
pub const TEXT_HINT: u32 = rgb(100, 116, 139);
/// Pure white (e.g. W button).
pub const TEXT_WHITE: u32 = rgb(255, 255, 255);

// ── Buttons (default) ─────────────────────────────────────────

pub const BTN_BG_DEFAULT: u32 = rgb(26, 30, 38);
pub const BTN_BORDER_DEFAULT: u32 = rgb(58, 65, 78);
pub const BTN_TEXT_DEFAULT: u32 = rgb(226, 232, 240);

// ── Buttons (primary / green) ─────────────────────────────────

pub const BTN_BG_PRIMARY: u32 = rgb(31, 91, 72);
pub const BTN_BORDER_PRIMARY: u32 = rgb(52, 211, 153);
pub const BTN_TEXT_PRIMARY: u32 = rgb(220, 252, 231);

// ── Buttons (danger / close) ──────────────────────────────────

pub const BTN_BG_DANGER: u32 = rgb(52, 31, 36);
pub const BTN_BORDER_DANGER: u32 = rgb(101, 43, 55);
pub const BTN_TEXT_DANGER: u32 = rgb(254, 202, 202);

// ── Buttons (disabled) ────────────────────────────────────────

pub const BTN_BG_DISABLED: u32 = rgb(31, 34, 40);
pub const BTN_BORDER_DISABLED: u32 = rgb(45, 49, 58);
pub const BTN_TEXT_DISABLED: u32 = rgb(105, 113, 128);

// ── Metric card tints ─────────────────────────────────────────

pub const CARD_BORDER: u32 = rgb(43, 49, 60);

/// Price text (green) inside metric card.
pub const METRIC_PRICE: u32 = rgb(134, 239, 172);
/// Count text (blue) inside metric card.
pub const METRIC_COUNT: u32 = rgb(191, 219, 254);
/// Distribution text (yellow) inside metric card.
pub const METRIC_DISTRIBUTION: u32 = rgb(253, 230, 138);

// ── Table row text ────────────────────────────────────────────

/// Price text in data rows (gold).
pub const ROW_PRICE: u32 = rgb(254, 243, 199);
/// Seller text in data rows (soft blue).
pub const ROW_SELLER: u32 = rgb(219, 234, 254);
/// Item name text in data rows.
pub const ROW_ITEM: u32 = rgb(229, 231, 235);

// ── Whisper badge ─────────────────────────────────────────────

/// Whisper "W" button background.
pub const WHISPER_BG: u32 = rgb(22, 68, 52);

// ── Empty / warning states ────────────────────────────────────

/// Empty-result / no-pricing warning text.
pub const WARNING_TEXT: u32 = rgb(251, 191, 36);
