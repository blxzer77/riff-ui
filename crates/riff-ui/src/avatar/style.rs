//! Sizes, colors, and SVG/pixel-mark constants of riff-uiux's `.ui-avt` and `.ui-avt__group`.

use std::time::Duration;

use gpui_kit::{Hsla, Pixels, hsla, px};

use crate::paint::{hex, rgba};
use crate::theme::RiffTheme;

/// `size`: `20`, `24`, `32` (default), or `40` pixels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum AvatarSize {
    S20,
    S24,
    #[default]
    S32,
    S40,
}

/// `shape`: `Circle` (people default) or rounded `Square` (workspaces/agents).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AvatarShape {
    Circle,
    Square,
}

/// Presence dot on the corner of an avatar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AvatarStatus {
    Online,
    Away,
    Busy,
}

impl AvatarStatus {
    pub(super) fn spoken(self) -> &'static str {
        match self {
            AvatarStatus::Online => "online",
            AvatarStatus::Away => "away",
            AvatarStatus::Busy => "busy",
        }
    }
}

/// `Person` (picture or initials) or `Agent` (3×3 pixel mark on a tile).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum AvatarKind {
    #[default]
    Person,
    Agent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Metrics {
    pub size: Pixels,
    pub font_size: Pixels,
    pub dot: Pixels,
    pub sq: Pixels,
    pub gap: Pixels,
}

impl Metrics {
    pub fn of(size: AvatarSize) -> Self {
        match size {
            AvatarSize::S20 => Self {
                size: px(20.),
                font_size: px(8.5),
                dot: px(6.),
                sq: px(3.),
                gap: px(1.),
            },
            AvatarSize::S24 => Self {
                size: px(24.),
                font_size: px(10.),
                dot: px(7.),
                sq: px(3.),
                gap: px(1.5),
            },
            AvatarSize::S32 => Self {
                size: px(32.),
                font_size: px(12.),
                dot: px(9.),
                sq: px(4.),
                gap: px(2.),
            },
            AvatarSize::S40 => Self {
                size: px(40.),
                font_size: px(14.),
                dot: px(10.),
                sq: px(5.),
                gap: px(2.5),
            },
        }
    }
}

const HUES: [f32; 8] = [214., 152., 40., 22., 330., 268., 174., 196.];

pub(super) const WORKING_PERIOD: Duration = Duration::from_millis(1200);
pub(super) const WAVE_MS: [u64; 9] = [0, 120, 240, 120, 240, 360, 240, 360, 480];
pub(super) const IMAGE_FADE: Duration = Duration::from_millis(180);

pub(super) const ANON_SVG_KEY: &str = "riff-avatar-anon";
pub(super) const ANON_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21C19.713 17.269 16.729 14.315 12.995 14.066L12 14C11.645 14.01 11.313 14.023 11.001 14.038C7.3 14.219 4.284 17.306 4 21"/><circle cx="12" cy="7" r="4"/></svg>"#;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Look {
    pub text: Hsla,
    pub fill: Hsla,
    pub tile_top: Hsla,
    pub tile_bottom: Hsla,
    pub edge: Hsla,
    pub ring: Hsla,
    pub mark: Hsla,
    pub online: Hsla,
    pub away: Hsla,
    pub busy: Hsla,
    pub bg_l: f32,
    pub bg_s: f32,
    pub fg_l: f32,
    pub fg_s: f32,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        if theme.mode.is_dark() {
            Self {
                text: hex(0xa1a1a6),
                fill: theme.subtle,
                tile_top: theme.surface,
                tile_bottom: hex(0x1b1b1e),
                edge: rgba(255, 255, 255, 0.1),
                ring: theme.surface,
                mark: rgba(77, 141, 255, 0.85),
                online: theme.green,
                away: theme.amber,
                busy: theme.red,
                bg_l: 0.24,
                bg_s: 0.28,
                fg_l: 0.80,
                fg_s: 0.60,
            }
        } else {
            Self {
                text: hex(0x616161),
                fill: theme.subtle,
                tile_top: theme.surface,
                tile_bottom: hex(0xf7f7f8),
                edge: rgba(0, 0, 0, 0.1),
                ring: theme.surface,
                mark: rgba(0, 96, 226, 0.78),
                online: theme.green,
                away: theme.amber,
                busy: theme.red,
                bg_l: 0.92,
                bg_s: 0.70,
                fg_l: 0.36,
                fg_s: 0.48,
            }
        }
    }

    pub fn hue_pair(&self, hue_index: usize) -> (Hsla, Hsla) {
        let h = HUES[hue_index % HUES.len()] / 360.;
        (
            hsla(h, self.bg_s, self.bg_l, 1.),
            hsla(h, self.fg_s, self.fg_l, 1.),
        )
    }

    pub fn status(&self, status: AvatarStatus) -> Hsla {
        match status {
            AvatarStatus::Online => self.online,
            AvatarStatus::Away => self.away,
            AvatarStatus::Busy => self.busy,
        }
    }
}
