//! Colors, typography, timing, and layout constants shared by `DropdownMenu`
//! and `ContextMenu` (matching riff-uiux's `.ui-dm` / `.ui-cm`).

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::paint;
use crate::theme::RiffTheme;

pub(crate) const VIEWPORT_MARGIN: Pixels = px(8.);
pub(crate) const TRIGGER_OFFSET: Pixels = px(6.);
pub(crate) const SUBMENU_OFFSET: Pixels = px(4.);
pub(crate) const PANEL_MIN_WIDTH: Pixels = px(180.);
pub(crate) const PANEL_MAX_WIDTH: Pixels = px(320.);
pub(crate) const PANEL_PADDING: Pixels = px(5.);
pub(crate) const LIST_GAP: Pixels = px(1.);

pub(crate) const ITEM_MIN_HEIGHT: Pixels = px(34.);
pub(crate) const RICH_ITEM_MIN_HEIGHT: Pixels = px(46.);
pub(crate) const ITEM_PAD_X: Pixels = px(10.);
pub(crate) const RICH_ITEM_PAD_Y: Pixels = px(7.);
pub(crate) const ITEM_GAP: Pixels = px(9.);

pub(crate) const ICON_SIZE: Pixels = px(16.);
pub(crate) const LOGO_SIZE: Pixels = px(14.);
pub(crate) const ACCESSORY_SIZE: Pixels = px(14.);

pub(crate) const TITLE_PAD_TOP: Pixels = px(7.);
pub(crate) const TITLE_PAD_BOTTOM: Pixels = px(5.);
pub(crate) const TITLE_PAD_X: Pixels = px(10.);
pub(crate) const TITLE_FONT_SIZE: Pixels = px(12.);
pub(crate) const TITLE_LINE_HEIGHT: Pixels = px(16.);

pub(crate) const LABEL_FONT_SIZE: Pixels = px(13.5);
pub(crate) const LABEL_LINE_HEIGHT: Pixels = px(20.);
pub(crate) const DESC_FONT_SIZE: Pixels = px(12.5);
pub(crate) const DESC_LINE_HEIGHT: Pixels = px(17.);
pub(crate) const KBD_FONT_SIZE: Pixels = px(12.);
pub(crate) const KBD_PAD_LEFT: Pixels = px(16.);

pub(crate) const SEP_HEIGHT: Pixels = px(1.);
pub(crate) const SEP_MARGIN_Y: Pixels = px(5.);
pub(crate) const SEP_MARGIN_X: Pixels = px(4.);
pub(crate) const EMPTY_PAD_Y: Pixels = px(6.);
pub(crate) const EMPTY_PAD_X: Pixels = px(8.);

pub(crate) const DRIFT: Pixels = px(4.);
pub(crate) const ENTER_DURATION: Duration = Duration::from_millis(220);
pub(crate) const SUB_ENTER_DURATION: Duration = Duration::from_millis(160);
pub(crate) const GLOW_DURATION: Duration = Duration::from_millis(180);
pub(crate) const TICK_DELAY: Duration = Duration::from_millis(40);
pub(crate) const TICK_DURATION: Duration = Duration::from_millis(280);
pub(crate) const SUB_OPEN_DELAY: Duration = Duration::from_millis(90);
pub(crate) const SUB_CLOSE_DELAY: Duration = Duration::from_millis(220);
pub(crate) const TYPEAHEAD_WINDOW: Duration = Duration::from_millis(700);

#[derive(Clone, Debug)]
pub(crate) struct Look {
    pub surface: Hsla,
    pub ink: Hsla,
    pub text: Hsla,
    pub icon: Hsla,
    pub faint: Hsla,
    pub line: Hsla,
    pub glow: Hsla,
    pub danger: Hsla,
    pub danger_glow: Hsla,
    pub focus: Hsla,
    pub radius: Pixels,
    pub row_radius: Pixels,
    pub shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let dark = theme.mode.is_dark();
        Self {
            surface: theme.overlay,
            ink: theme.ink,
            text: theme.ink,
            icon: theme.muted,
            faint: theme.placeholder,
            line: theme.border,
            glow: if dark {
                paint::hex(0x26262a)
            } else {
                paint::hex(0xf4f4f5)
            },
            danger: theme.danger,
            danger_glow: paint::mix(
                Hsla::transparent_black(),
                theme.red,
                if dark { 0.14 } else { 0.08 },
            ),
            focus: theme.focus,
            radius: theme.radius_lg + px(4.),
            row_radius: theme.radius_sm + px(4.),
            shadows: theme.shadow_popover.clone(),
        }
    }
}
