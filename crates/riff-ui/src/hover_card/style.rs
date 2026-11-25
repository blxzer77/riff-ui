//! Colors, typography, timing, and layout constants of riff-uiux's `.ui-hvc`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::theme::RiffTheme;

pub(super) const VIEWPORT_MARGIN: Pixels = px(8.);
pub(super) const OFFSET_DEFAULT: Pixels = px(8.);
pub(super) const MAX_WIDTH_DEFAULT: Pixels = px(320.);
pub(super) const PADDING: Pixels = px(16.);
pub(super) const DRIFT: Pixels = px(4.);

pub(super) const BODY_FONT_SIZE: Pixels = px(13.5);
pub(super) const BODY_LINE_HEIGHT: Pixels = px(20.);

pub(super) const OPEN_DELAY_DEFAULT: Duration = Duration::from_millis(600);
pub(super) const CLOSE_DELAY_DEFAULT: Duration = Duration::from_millis(300);
pub(super) const ENTER_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub surface: Hsla,
    pub ink: Hsla,
    pub radius: Pixels,
    pub shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        Self {
            surface: theme.overlay,
            ink: theme.ink,
            radius: theme.radius_lg + px(6.),
            shadows: theme.shadow_popover.clone(),
        }
    }
}
