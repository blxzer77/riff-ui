//! Sizes and colors of riff-uiux's `.ui-button`, one field per CSS variable.

use gpui_kit::{BoxShadow, Hsla, Pixels, Rgba, px};

use super::{ButtonSize, ButtonVariant};
use crate::theme::{RiffTheme, css_shadow};

/// `--h`, `--px`, `--gap`, `--fs`, `--icon` and `--radius` of one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Metrics {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub gap: Pixels,
    pub font_size: Pixels,
    pub icon: Pixels,
    pub radius: Pixels,
}

/// The transparent 1px border every variant has; it is part of the width.
pub(super) const BORDER: Pixels = px(1.);

impl Metrics {
    pub fn of(size: ButtonSize, theme: &RiffTheme) -> Self {
        // `--radius` is `--riff-radius`, two pixels less for sm and more for lg.
        let r = theme.radius;
        let (height, padding_x, gap, font_size, icon, radius) = match size {
            ButtonSize::Sm => (32., 12., 6., 13., 16., r - px(2.)),
            ButtonSize::Md => (40., 16., 8., 14., 18., r),
            ButtonSize::Lg => (48., 20., 10., 15., 20., r + px(2.)),
        };
        Self {
            height: px(height),
            padding_x: px(padding_x),
            gap: px(gap),
            font_size: px(font_size),
            icon: px(icon),
            radius,
        }
    }
}

/// How a variant paints its box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Fill {
    Solid(Hsla),
    /// primary: a top-to-bottom gradient inside the border, over a three-stop
    /// gradient filling the border box (which shows through the 1px border).
    Gradient {
        top: Hsla,
        bottom: Hsla,
        edge: [Hsla; 3],
    },
}

/// The resolved look of one variant in one mode.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Look {
    pub fill: Fill,
    pub fill_hover: Fill,
    pub border: Hsla,
    pub text: Hsla,
    pub text_hover: Hsla,
    /// Built with [`css_shadow`] from the CSS values.
    pub shadows: Vec<BoxShadow>,
    /// `--ui-btn-shine` as seen by this variant's loading sweep.
    pub shine: Hsla,
    pub focus: Hsla,
}

impl Look {
    pub fn of(variant: ButtonVariant, theme: &RiffTheme) -> Self {
        let p = Palette::new(theme);
        let transparent = Hsla::transparent_black();
        match variant {
            ButtonVariant::Primary => Look {
                fill: Fill::Gradient {
                    top: p.dark_top,
                    bottom: p.dark_bottom,
                    edge: p.edge,
                },
                fill_hover: Fill::Gradient {
                    top: p.dark_hover_top,
                    bottom: p.dark_hover_bottom,
                    edge: p.edge,
                },
                border: transparent,
                text: p.on_ink,
                text_hover: p.on_ink,
                shadows: vec![
                    css_shadow(0., 0., 1., p.ring),
                    css_shadow(2., 4., 0., p.dark_shadow),
                ],
                shine: p.shine_on_ink,
                focus: p.focus,
            },
            ButtonVariant::Secondary => Look {
                fill: Fill::Solid(p.surface),
                fill_hover: Fill::Solid(p.surface_hover),
                border: p.line,
                text: p.ink,
                text_hover: p.ink,
                shadows: vec![css_shadow(1., 2., 0., p.surface_shadow)],
                shine: p.shine,
                focus: p.focus,
            },
            ButtonVariant::Ghost => Look {
                fill: Fill::Solid(transparent),
                fill_hover: Fill::Solid(p.fill),
                border: transparent,
                text: p.muted,
                text_hover: p.hover,
                shadows: Vec::new(),
                shine: p.shine,
                focus: p.focus,
            },
            ButtonVariant::Destructive => Look {
                fill: Fill::Solid(p.danger_bg),
                // color-mix(in srgb, var(--ui-btn-danger-bg) 88%, var(--ui-btn-danger))
                fill_hover: Fill::Solid(mix(p.danger_bg, p.danger, 0.12)),
                border: transparent,
                text: p.danger,
                text_hover: p.danger,
                shadows: Vec::new(),
                // Set on the sweep itself, so the dark block does not override it.
                shine: rgba(255, 255, 255, 0.82),
                focus: p.focus,
            },
        }
    }
}

/// The `--ui-btn-*` variables. Those that read a `--riff-*` token take it
/// from the theme; the rest are the literal values of button.tsx.
struct Palette {
    ink: Hsla,
    on_ink: Hsla,
    dark_top: Hsla,
    dark_bottom: Hsla,
    dark_hover_top: Hsla,
    dark_hover_bottom: Hsla,
    edge: [Hsla; 3],
    ring: Hsla,
    dark_shadow: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_shadow: Hsla,
    line: Hsla,
    muted: Hsla,
    hover: Hsla,
    fill: Hsla,
    shine: Hsla,
    shine_on_ink: Hsla,
    danger: Hsla,
    danger_bg: Hsla,
    focus: Hsla,
}

impl Palette {
    fn new(theme: &RiffTheme) -> Self {
        let common = |on_ink, dark_top, dark_bottom, dark_hover_top, dark_hover_bottom, edge| {
            (
                on_ink,
                dark_top,
                dark_bottom,
                dark_hover_top,
                dark_hover_bottom,
                edge,
            )
        };
        if theme.mode.is_dark() {
            let (on_ink, dark_top, dark_bottom, dark_hover_top, dark_hover_bottom, edge) = common(
                hex(0x121213),
                hex(0xf4f4f5),
                hex(0xe4e4e7),
                hex(0xffffff),
                hex(0xececef),
                [hex(0xffffff), hex(0xececef), hex(0xd4d4d8)],
            );
            Self {
                ink: theme.ink,
                on_ink,
                dark_top,
                dark_bottom,
                dark_hover_top,
                dark_hover_bottom,
                edge,
                ring: rgba(0, 0, 0, 0.6),
                dark_shadow: rgba(0, 0, 0, 0.4),
                surface: theme.surface,
                surface_hover: hex(0x2a2a2d),
                surface_shadow: rgba(0, 0, 0, 0.4),
                line: theme.border,
                muted: theme.muted,
                hover: hex(0xededed),
                fill: rgba(255, 255, 255, 0.06),
                shine: rgba(237, 237, 237, 0.95),
                shine_on_ink: rgba(255, 255, 255, 0.9),
                danger: theme.danger,
                // color-mix(in srgb, var(--riff-red) 16%, transparent)
                danger_bg: mix(Hsla::transparent_black(), theme.red, 0.16),
                focus: theme.focus,
            }
        } else {
            let (on_ink, dark_top, dark_bottom, dark_hover_top, dark_hover_bottom, edge) = common(
                hex(0xfafafa),
                hex(0x323137),
                hex(0x201e25),
                hex(0x3c3b42),
                hex(0x29272f),
                [hex(0x4b4951), hex(0x31303a), hex(0x18171b)],
            );
            Self {
                ink: theme.ink,
                on_ink,
                dark_top,
                dark_bottom,
                dark_hover_top,
                dark_hover_bottom,
                edge,
                ring: hex(0x0d0d0d),
                dark_shadow: rgba(0, 0, 0, 0.1),
                surface: theme.surface,
                surface_hover: hex(0xfafafa),
                surface_shadow: rgba(0, 0, 0, 0.06),
                line: theme.border,
                muted: theme.muted,
                hover: hex(0x404040),
                fill: hex(0xf0f0f0),
                shine: rgba(255, 255, 255, 0.82),
                shine_on_ink: rgba(255, 255, 255, 0.95),
                danger: theme.danger,
                danger_bg: hex(0xffe9e6),
                focus: theme.focus,
            }
        }
    }
}

fn hex(value: u32) -> Hsla {
    gpui_kit::rgb(value).into()
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    Rgba {
        r: f32::from(r) / 255.,
        g: f32::from(g) / 255.,
        b: f32::from(b) / 255.,
        a,
    }
    .into()
}

/// `color-mix(in srgb, a (1 - t), b t)`, which like CSS transitions mixes
/// premultiplied channels: fading out of `transparent` keeps the hue instead
/// of passing through black.
pub(super) fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    let alpha = a.a + (b.a - a.a) * t;
    if alpha <= 0. {
        return Hsla::transparent_black();
    }
    let channel = |x: f32, y: f32| ((x * a.a) + (y * b.a - x * a.a) * t) / alpha;
    Rgba {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: alpha,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_rgba(color: Hsla, expected: (f32, f32, f32, f32)) {
        let c = color.to_rgb();
        let actual = (c.r * 255., c.g * 255., c.b * 255., c.a);
        let close = (actual.0 - expected.0).abs() < 0.6
            && (actual.1 - expected.1).abs() < 0.6
            && (actual.2 - expected.2).abs() < 0.6
            && (actual.3 - expected.3).abs() < 0.002;
        assert!(close, "{actual:?} != {expected:?}");
    }

    #[test]
    fn mixing_with_transparent_keeps_the_color() {
        let red = rgba(255, 107, 95, 1.);
        assert_rgba(
            mix(Hsla::transparent_black(), red, 0.16),
            (255., 107., 95., 0.16),
        );
    }

    #[test]
    fn destructive_hover_matches_color_mix() {
        // color-mix(in srgb, #ffe9e6 88%, #c4320a)
        let mixed = mix(hex(0xffe9e6), hex(0xc4320a), 0.12);
        assert_rgba(mixed, (248.08, 211.04, 203.76, 1.));
    }
}
