//! Sizes and colors of riff-uiux's `.ui-bdg`.

use gpui_kit::{Hsla, Pixels, px};

use super::{BadgeSize, BadgeTone};
use crate::paint::hex;
use crate::theme::RiffTheme;

/// `--h`, `--pad`, `--lead`, `--mark`, `gap` and `font-size` of one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Metrics {
    pub height: Pixels,
    pub pad: Pixels,
    pub lead: Pixels,
    pub mark: Pixels,
    pub gap: Pixels,
    pub font_size: Pixels,
    pub line_height: Pixels,
}

pub(super) const DOT_SIZE: Pixels = px(6.);
pub(super) const DOT_MARGIN_X: Pixels = px(1.);
pub(super) const COUNT_PAD_X: Pixels = px(6.);

impl Metrics {
    pub fn of(size: BadgeSize) -> Self {
        match size {
            BadgeSize::Sm => Self {
                height: px(20.),
                pad: px(8.),
                lead: px(6.),
                mark: px(12.),
                gap: px(4.),
                font_size: px(11.),
                line_height: px(16.),
            },
            BadgeSize::Md => Self {
                height: px(22.),
                pad: px(9.),
                lead: px(7.),
                mark: px(13.),
                gap: px(5.),
                font_size: px(12.),
                line_height: px(16.),
            },
        }
    }
}

/// Resolved background, foreground and dot colors for one tone in one mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Look {
    pub bg: Hsla,
    pub fg: Hsla,
    pub dot: Hsla,
}

impl Look {
    pub fn of(tone: BadgeTone, theme: &RiffTheme) -> Self {
        let dark = theme.mode.is_dark();
        match tone {
            BadgeTone::Neutral => Self {
                bg: theme.subtle,
                fg: hex(if dark { 0xa1a1a6 } else { 0x616161 }),
                dot: hex(if dark { 0x6b6b72 } else { 0xa3a3a3 }),
            },
            BadgeTone::Info => Self {
                bg: theme.info_soft,
                fg: theme.info,
                dot: theme.info,
            },
            BadgeTone::Success => Self {
                bg: theme.success_soft,
                fg: theme.success,
                dot: theme.success,
            },
            BadgeTone::Warning => Self {
                bg: theme.warning_soft,
                fg: theme.warning,
                dot: theme.warning,
            },
            BadgeTone::Danger => Self {
                bg: theme.danger_soft,
                fg: theme.danger,
                dot: theme.danger,
            },
            BadgeTone::Purple => {
                let (bg, fg) = if dark {
                    (hex(0x2a1c3d), hex(0xc9a2f5))
                } else {
                    (hex(0xf4ebfd), hex(0x8a35d6))
                };
                Self { bg, fg, dot: fg }
            }
        }
    }
}
