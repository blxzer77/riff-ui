//! Colors, timing, and layout constants of riff-uiux's `.ui-tip`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, hsla, point, px};

use crate::paint::hex;
use crate::theme::{RiffTheme, css_shadow};

pub(super) const MAX_WIDTH: Pixels = px(240.);
pub(super) const VIEWPORT_MARGIN: Pixels = px(8.);
pub(super) const OFFSET_DEFAULT: Pixels = px(6.);
pub(super) const PAD_X: Pixels = px(9.);
pub(super) const PAD_Y: Pixels = px(5.);
pub(super) const GAP: Pixels = px(8.);
pub(super) const DRIFT: Pixels = px(3.);

pub(super) const FONT_SIZE: Pixels = px(12.5);
pub(super) const LINE_HEIGHT: Pixels = px(18.);

pub(super) const KEY_PAD_X: Pixels = px(5.);
pub(super) const KEY_MARGIN_RIGHT: Pixels = px(-3.);
pub(super) const KEY_RADIUS: Pixels = px(4.);
pub(super) const KEY_FONT_SIZE: Pixels = px(11.);
pub(super) const KEY_LINE_HEIGHT: Pixels = px(16.);
pub(super) const KEY_TRACKING: Pixels = px(11. * 0.02);

pub(super) const DELAY_DEFAULT: Duration = Duration::from_millis(500);
pub(super) const SKIP_WINDOW: Duration = Duration::from_millis(300);
pub(super) const LEAVE_GRACE: Duration = Duration::from_millis(120);
pub(super) const ENTER_DURATION: Duration = Duration::from_millis(160);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub bg: Hsla,
    pub ink: Hsla,
    pub muted: Hsla,
    pub key_bg: Hsla,
    pub radius: Pixels,
    pub shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let black = |alpha| hsla(0., 0., 0., alpha);
        let white = |alpha| hsla(0., 0., 1., alpha);
        if theme.mode.is_dark() {
            Self {
                bg: hex(0xfafafa),
                ink: hex(0x171717),
                muted: black(0.6),
                key_bg: black(0.07),
                radius: theme.radius_sm,
                shadows: vec![
                    css_shadow(8., 20., -8., black(0.7)),
                    BoxShadow {
                        color: white(0.06),
                        offset: point(px(0.), px(0.)),
                        blur_radius: px(0.),
                        spread_radius: px(1.),
                        inset: false,
                    },
                ],
            }
        } else {
            Self {
                bg: hex(0x171717),
                ink: hex(0xffffff),
                muted: white(0.72),
                key_bg: white(0.14),
                radius: theme.radius_sm,
                shadows: vec![
                    css_shadow(8., 20., -8., black(0.32)),
                    css_shadow(2., 6., -2., black(0.12)),
                ],
            }
        }
    }
}
