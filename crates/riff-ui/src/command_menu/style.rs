//! Colors, geometry, typography, and motion tokens matching riff-uiux's `.ui-cm`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::paint;
use crate::theme::RiffTheme;

pub(super) const MAX_WIDTH: Pixels = px(600.);
pub(super) const TRAY_PAD: Pixels = px(4.);

pub(super) const SEARCH_HEIGHT: Pixels = px(52.);
pub(super) const SEARCH_PAD_X: Pixels = px(16.);
pub(super) const SEARCH_GAP: Pixels = px(10.);
pub(super) const SEARCH_ICON_SIZE: Pixels = px(18.);
pub(super) const SEARCH_FONT_SIZE: Pixels = px(15.);

pub(super) const ENTER_HINT_FONT_SIZE: Pixels = px(11.5);
pub(super) const ENTER_HINT_ICON_SIZE: Pixels = px(14.);
pub(super) const ENTER_HINT_GAP: Pixels = px(4.);

pub(super) const LIST_MAX_HEIGHT: Pixels = px(340.);
pub(super) const LIST_MAX_HEIGHT_NARROW: Pixels = px(360.);
pub(super) const LIST_PAD_TOP: Pixels = px(4.);
pub(super) const LIST_PAD_X: Pixels = px(6.);
pub(super) const LIST_PAD_BOTTOM: Pixels = px(8.);

pub(super) const GROUP_LABEL_FONT_SIZE: Pixels = px(12.);
pub(super) const GROUP_LABEL_LINE_HEIGHT: Pixels = px(15.);
pub(super) const GROUP_LABEL_PAD_TOP: Pixels = px(10.);
pub(super) const GROUP_LABEL_PAD_X: Pixels = px(10.);
pub(super) const GROUP_LABEL_PAD_BOTTOM: Pixels = px(4.);
pub(super) const GROUP_LABEL_HEIGHT: Pixels = px(29.);

pub(super) const ITEM_HEIGHT: Pixels = px(40.);
pub(super) const ITEM_HEIGHT_NARROW: Pixels = px(44.);
pub(super) const ITEM_PAD_X: Pixels = px(10.);
pub(super) const ITEM_GAP: Pixels = px(12.);
pub(super) const ITEM_FONT_SIZE: Pixels = px(13.5);
pub(super) const ITEM_LINE_HEIGHT: Pixels = px(18.);

pub(super) const TILE_SIZE: Pixels = px(26.);
pub(super) const TILE_RADIUS: Pixels = px(7.);
pub(super) const LOGO_SIZE: Pixels = px(14.);
pub(super) const ICON_GLYPH_SIZE: Pixels = px(17.);
pub(super) const TEXT_GAP: Pixels = px(8.);

pub(super) const KBD_MIN_WIDTH: Pixels = px(20.);
pub(super) const KBD_HEIGHT: Pixels = px(20.);
pub(super) const KBD_PAD_X: Pixels = px(5.);
pub(super) const KBD_FONT_SIZE: Pixels = px(11.);
pub(super) const KEYS_GAP: Pixels = px(3.);

pub(super) const FOOT_HEIGHT: Pixels = px(38.);
pub(super) const FOOT_PAD_X: Pixels = px(12.);
pub(super) const FOOT_GAP: Pixels = px(12.);
pub(super) const FOOT_FONT_SIZE: Pixels = px(12.5);
pub(super) const DONE_GAP: Pixels = px(6.);
pub(super) const DONE_ICON_SIZE: Pixels = px(14.);
pub(super) const NARROW_BREAKPOINT: Pixels = px(480.);

pub(super) const PILL_DURATION: Duration = Duration::from_millis(220);
pub(super) const LIST_HEIGHT_DURATION: Duration = Duration::from_millis(320);
pub(super) const RAN_DURATION: Duration = Duration::from_millis(600);
pub(super) const DONE_DURATION: Duration = Duration::from_millis(260);
pub(super) const ANNOUNCEMENT_TTL: Duration = Duration::from_millis(1800);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub tray: Hsla,
    pub card: Hsla,
    pub line: Hsla,
    pub ink: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub active: Hsla,
    pub tile: Hsla,
    pub tile_ring: Hsla,
    pub kbd: Hsla,
    pub mark: Hsla,
    pub ok: Hsla,
    pub tray_radius: Pixels,
    pub card_radius: Pixels,
    pub pill_radius: Pixels,
    pub kbd_radius: Pixels,
    pub card_shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let dark = theme.mode.is_dark();
        Self {
            tray: theme.subtle,
            card: theme.overlay,
            line: theme.border,
            ink: theme.ink,
            text: theme.ink,
            muted: theme.muted,
            faint: theme.placeholder,
            active: theme.subtle,
            tile: if dark {
                paint::hex(0x26262a)
            } else {
                paint::hex(0xffffff)
            },
            tile_ring: if dark {
                paint::rgba(255, 255, 255, 0.1)
            } else {
                paint::rgba(0, 0, 0, 0.09)
            },
            kbd: if dark {
                paint::hex(0x232326)
            } else {
                paint::hex(0xffffff)
            },
            mark: theme.blue.opacity(if dark { 0.24 } else { 0.14 }),
            ok: if dark {
                paint::hex(0x5fc88a)
            } else {
                theme.success
            },
            tray_radius: theme.radius_lg + px(4.),
            card_radius: theme.radius_lg,
            pill_radius: theme.radius_sm + px(3.),
            kbd_radius: (theme.radius_sm - px(2.)).max(px(0.)),
            card_shadows: theme.shadow_popover.clone(),
        }
    }
}
