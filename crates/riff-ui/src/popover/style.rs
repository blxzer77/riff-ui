//! Colors, typography, and layout constants of riff-uiux's `.ui-pop`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::theme::RiffTheme;

pub(super) const VIEWPORT_MARGIN: Pixels = px(8.);
pub(super) const OFFSET_DEFAULT: Pixels = px(6.);
pub(super) const MAX_WIDTH_DEFAULT: Pixels = px(360.);
pub(super) const PADDING: Pixels = px(16.);
pub(super) const HEAD_GAP: Pixels = px(2.);
pub(super) const HEAD_MARGIN_BOTTOM: Pixels = px(14.);
pub(super) const DRIFT: Pixels = px(4.);

pub(super) const BODY_FONT_SIZE: Pixels = px(13.5);
pub(super) const BODY_LINE_HEIGHT: Pixels = px(20.);
pub(super) const DESC_FONT_SIZE: Pixels = px(12.5);
pub(super) const DESC_LINE_HEIGHT: Pixels = px(17.);

pub(super) const ENTER_DURATION: Duration = Duration::from_millis(220);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub surface: Hsla,
    pub ink: Hsla,
    pub muted: Hsla,
    pub radius: Pixels,
    pub shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        Self {
            surface: theme.overlay,
            ink: theme.ink,
            muted: theme.muted,
            radius: theme.radius_lg + px(6.),
            shadows: theme.shadow_popover.clone(),
        }
    }
}
