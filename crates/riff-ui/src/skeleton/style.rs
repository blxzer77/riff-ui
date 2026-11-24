//! Colors, geometry, and motion constants of riff-uiux's `.ui-skl` and `.ui-skl-loading`.

use std::time::Duration;

use gpui_kit::{Hsla, Pixels, px};

use crate::paint::{hex, rgba};
use crate::theme::RiffTheme;

pub(super) const DEFAULT_HEIGHT: Pixels = px(12.);
pub(super) const DEFAULT_SPEED: Duration = Duration::from_millis(1600);
/// `0% -> 60%` sweeps across; `60% -> 100%` pauses off-screen.
pub(super) const SWEEP_ACTIVE_FRACTION: f32 = 0.6;

pub(super) const LOADING_DEFAULT_DELAY: Duration = Duration::from_millis(150);
pub(super) const LOADING_FADE_DURATION: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Look {
    pub fill: Hsla,
    pub sheen: Hsla,
    pub default_radius: Pixels,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let (fill, sheen) = if theme.mode.is_dark() {
            (hex(0x242428), rgba(255, 255, 255, 0.05))
        } else {
            (hex(0xefefef), rgba(255, 255, 255, 0.8))
        };
        Self {
            fill,
            sheen,
            default_radius: theme.radius_sm,
        }
    }
}
