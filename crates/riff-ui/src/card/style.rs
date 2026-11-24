//! Sizes, colors, and shadows of riff-uiux's `.ui-crd`.

use std::time::Duration;

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use crate::paint::{hex, mix, rgba};
use crate::theme::{RiffTheme, css_shadow};

pub(super) const PAD: Pixels = px(16.);
pub(super) const TRAY_PAD: Pixels = px(4.);
pub(super) const CARD_GAP: Pixels = px(12.);
pub(super) const HEAD_GAP: Pixels = px(12.);
pub(super) const TITLES_GAP: Pixels = px(2.);
pub(super) const ACTION_GAP: Pixels = px(4.);
pub(super) const ACTION_HEIGHT: Pixels = px(20.);
pub(super) const ICON_MIN: Pixels = px(20.);
pub(super) const ICON_SIZE: Pixels = px(16.);
pub(super) const TITLE_SIZE: Pixels = px(14.);
pub(super) const LINE_HEIGHT: Pixels = px(20.);
pub(super) const BODY_SIZE: Pixels = px(13.);
pub(super) const FOOT_MIN_HEIGHT: Pixels = px(40.);
pub(super) const FOOT_GAP: Pixels = px(8.);
pub(super) const FOOT_SIZE: Pixels = px(12.5);
pub(super) const FOOT_LINE_HEIGHT: Pixels = px(18.);
pub(super) const FOOT_PAD_RIGHT: Pixels = px(2.);
pub(super) const STATE_DURATION: Duration = Duration::from_millis(160);

#[derive(Clone, Debug)]
pub(super) struct Look {
    pub tray: Hsla,
    pub tray_selected: Hsla,
    pub surface: Hsla,
    pub strong: Hsla,
    pub ink: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub focus: Hsla,
    pub shadow: Vec<BoxShadow>,
    pub shadow_hover: Vec<BoxShadow>,
    pub shadow_selected: Vec<BoxShadow>,
}

impl Look {
    pub fn of(theme: &RiffTheme) -> Self {
        let shadow_selected = vec![
            css_shadow(0., 0., 1., theme.border_strong),
            css_shadow(1., 2., 0., rgba(0, 0, 0, 0.04)),
        ];
        if theme.mode.is_dark() {
            Self {
                tray: theme.subtle,
                tray_selected: hex(0x26262a),
                surface: theme.surface,
                strong: theme.ink,
                ink: theme.ink,
                muted: theme.muted,
                faint: hex(0x8b8b92),
                focus: theme.focus,
                shadow: theme.shadow_card.clone(),
                shadow_hover: vec![
                    css_shadow(0., 0., 1., theme.border_strong),
                    css_shadow(2., 6., 0., rgba(0, 0, 0, 0.4)),
                ],
                shadow_selected,
            }
        } else {
            Self {
                tray: theme.subtle,
                tray_selected: hex(0xe6e6e6),
                surface: theme.surface,
                strong: theme.ink,
                ink: theme.ink,
                muted: theme.muted,
                faint: hex(0x8f8f8f),
                focus: theme.focus,
                shadow: theme.shadow_card.clone(),
                shadow_hover: vec![
                    css_shadow(0., 0., 1., theme.border_strong),
                    css_shadow(2., 6., -1., rgba(0, 0, 0, 0.08)),
                ],
                shadow_selected,
            }
        }
    }

    pub fn lerp_shadows(&self, from: &[BoxShadow], to: &[BoxShadow], t: f32) -> Vec<BoxShadow> {
        if t <= 0. {
            return from.to_vec();
        }
        if t >= 1. {
            return to.to_vec();
        }
        let len = from.len().max(to.len());
        (0..len)
            .map(|i| {
                let a = from.get(i).cloned().unwrap_or(BoxShadow {
                    color: Hsla::transparent_black(),
                    offset: to[i].offset,
                    blur_radius: to[i].blur_radius,
                    spread_radius: to[i].spread_radius,
                    inset: false,
                });
                let b = to.get(i).cloned().unwrap_or(BoxShadow {
                    color: Hsla::transparent_black(),
                    offset: a.offset,
                    blur_radius: a.blur_radius,
                    spread_radius: a.spread_radius,
                    inset: false,
                });
                BoxShadow {
                    color: mix(a.color, b.color, t),
                    offset: a.offset + (b.offset - a.offset) * t,
                    blur_radius: a.blur_radius + (b.blur_radius - a.blur_radius) * t,
                    spread_radius: a.spread_radius + (b.spread_radius - a.spread_radius) * t,
                    inset: false,
                }
            })
            .collect()
    }
}
