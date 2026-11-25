//! Colors, sizing, typography, and animation timing matching riff-uiux's `.ui-dlg`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::paint;
use crate::theme::RiffTheme;

pub(super) const SCRIM_PAD_Y: Pixels = px(24.);
pub(super) const SCRIM_PAD_X: Pixels = px(16.);
pub(super) const TRAY_PAD: Pixels = px(4.);
pub(super) const DRIFT: Pixels = px(8.);

pub(super) const HEAD_TOP: Pixels = px(18.);
pub(super) const HEAD_TOP_ALERT: Pixels = px(20.);
pub(super) const HEAD_BOTTOM_EMPTY: Pixels = px(20.);
pub(super) const HEAD_LEFT: Pixels = px(20.);
pub(super) const HEAD_RIGHT: Pixels = px(13.);
pub(super) const HEAD_GAP: Pixels = px(12.);
pub(super) const TITLE_DESC_GAP: Pixels = px(4.);

pub(super) const TITLE_FONT_SIZE: Pixels = px(15.);
pub(super) const TITLE_LINE_HEIGHT: Pixels = px(22.);
pub(super) const BODY_FONT_SIZE: Pixels = px(13.5);
pub(super) const BODY_LINE_HEIGHT: Pixels = px(20.);

pub(super) const BODY_PAD_TOP: Pixels = px(12.);
pub(super) const BODY_PAD_X: Pixels = px(20.);
pub(super) const BODY_PAD_BOTTOM: Pixels = px(20.);

pub(super) const FOOT_PAD_TOP: Pixels = px(8.);
pub(super) const FOOT_PAD_X: Pixels = px(4.);
pub(super) const FOOT_PAD_BOTTOM: Pixels = px(4.);
pub(super) const FOOT_GAP: Pixels = px(8.);

pub(super) const CLOSE_SIZE: Pixels = px(32.);
pub(super) const CLOSE_ICON_SIZE: Pixels = px(16.);
pub(super) const CLOSE_TOP_OFFSET: Pixels = px(-5.);

pub(super) const SCRIM_DURATION: Duration = Duration::from_millis(180);
pub(super) const ENTER_DURATION: Duration = Duration::from_millis(260);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub scrim: Hsla,
    pub tray: Hsla,
    pub surface: Hsla,
    pub ink: Hsla,
    pub text: Hsla,
    pub icon: Hsla,
    pub wash: Hsla,
    pub tray_radius: Pixels,
    pub card_radius: Pixels,
    pub close_radius: Pixels,
    pub alert_cancel_bg: Option<Hsla>,
    pub tray_shadows: Vec<BoxShadow>,
    pub card_shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let dark = theme.mode.is_dark();
        Self {
            scrim: paint::rgba(0, 0, 0, if dark { 0.5 } else { 0.18 }),
            tray: theme.subtle,
            surface: theme.surface,
            ink: theme.ink,
            text: theme.muted,
            icon: theme.muted,
            wash: if dark {
                paint::rgba(255, 255, 255, 0.06)
            } else {
                paint::hex(0xf0f0f0)
            },
            tray_radius: theme.radius_lg + px(4.),
            card_radius: theme.radius_lg,
            close_radius: (theme.radius - px(2.)).max(px(0.)),
            alert_cancel_bg: dark.then(|| paint::hex(0x232326)),
            tray_shadows: theme.shadow_popover.clone(),
            card_shadows: theme.shadow_card.clone(),
        }
    }
}
