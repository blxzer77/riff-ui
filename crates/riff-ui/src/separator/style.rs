//! Metrics and colors of riff-uiux's `.ui-sep`.

use gpui_kit::{Hsla, Pixels, px};

use crate::theme::RiffTheme;

pub(super) const WEIGHT: Pixels = px(1.);
pub(super) const VERTICAL_HEIGHT: Pixels = px(18.);
pub(super) const MIN_LINE_WIDTH: Pixels = px(12.);
pub(super) const LABEL_GAP: Pixels = px(12.);
pub(super) const LABEL_FONT_SIZE: Pixels = px(12.5);
pub(super) const LABEL_LINE_HEIGHT: Pixels = px(18.);
pub(super) const LABEL_MAX_WIDTH_FRACTION: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Look {
    pub line: Hsla,
    pub text: Hsla,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        Self {
            line: theme.border,
            text: theme.muted,
        }
    }
}
