//! Sizes and colors of riff-uiux's `.ui-kbd`.

use gpui_kit::{BoxShadow, Hsla, Pixels, point, px};

use super::{KbdPlatform, KbdSize};
use crate::paint::hex;
use crate::theme::RiffTheme;

/// `--h`, `--pad`, `radius`, `font-size`, `letter-spacing` and `gap`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Metrics {
    pub height: Pixels,
    pub pad_x: Pixels,
    pub radius: Pixels,
    pub font_size: Pixels,
    pub glyph_font_size: Pixels,
    pub tracking: Pixels,
    pub gap: Pixels,
}

/// `.ui-kbd__keys { translate: 0 -0.5px; }`
pub(super) const KEYS_OFFSET_Y: Pixels = px(-0.5);

impl Metrics {
    pub fn of(size: KbdSize, platform: KbdPlatform, theme: &RiffTheme) -> Self {
        let (height, pad_x, radius, font_size) = match size {
            KbdSize::Sm => (
                px(18.),
                px(5.),
                (theme.radius_sm - px(2.)).max(px(0.)),
                px(11.),
            ),
            KbdSize::Md => (
                px(22.),
                px(6.),
                (theme.radius_sm - px(1.)).max(px(0.)),
                px(12.),
            ),
        };
        let gap = match platform {
            KbdPlatform::Mac => px(1.),
            KbdPlatform::Other => px(3.),
        };
        Self {
            height,
            pad_x,
            radius,
            font_size,
            glyph_font_size: font_size * 1.1,
            tracking: font_size * 0.02,
            gap,
        }
    }
}

/// Resolved colors and inset shadows of `.ui-kbd`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Look {
    pub bg: Hsla,
    pub ink: Hsla,
    pub inset_shadows: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let depth = hex(if theme.mode.is_dark() {
            0x1a1a1d
        } else {
            0xd9d9d9
        });
        Self {
            bg: theme.surface,
            ink: theme.muted,
            // CSS `box-shadow: A, B` draws A over B; GPUI paints slice order.
            inset_shadows: vec![
                BoxShadow {
                    color: depth,
                    offset: point(px(0.), px(-1.)),
                    blur_radius: px(0.),
                    spread_radius: px(0.),
                    inset: true,
                },
                BoxShadow {
                    color: theme.border,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: px(1.),
                    inset: true,
                },
            ],
        }
    }
}
