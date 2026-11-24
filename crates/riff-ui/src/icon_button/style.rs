//! Sizes and colors of riff-uiux's `.ui-icb`, one field per CSS variable.

use gpui_kit::{BoxShadow, Hsla, Pixels, px};

use super::{IconButtonSize, IconButtonVariant};
use crate::paint::{Fill, hex, lerp_fill, mix, rgba};
use crate::theme::{RiffTheme, css_shadow};

/// `--size`, `--icon` and `--radius` of one size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Metrics {
    pub side: Pixels,
    pub icon: Pixels,
    pub radius: Pixels,
}

/// The 1px border every variant has (`border: 1px solid transparent` by default).
pub(super) const BORDER: Pixels = px(1.);

impl Metrics {
    pub fn of(size: IconButtonSize, theme: &RiffTheme) -> Self {
        let r = theme.radius;
        let (side, icon, radius) = match size {
            IconButtonSize::Sm => (32., 16., r - px(2.)),
            IconButtonSize::Md => (40., 18., r),
        };
        Self {
            side: px(side),
            icon: px(icon),
            radius,
        }
    }
}

/// The resolved look of one variant in one mode.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Look {
    pub fill: Fill,
    pub fill_toggled: Option<Fill>,
    pub fill_hover: Fill,
    pub border: Hsla,
    pub icon: Hsla,
    pub icon_toggled: Hsla,
    pub icon_hover: Hsla,
    pub shadows: Vec<BoxShadow>,
    /// Whether `aria-pressed="true"` removes the resting box shadow (secondary).
    pub clear_shadow_on_toggle: bool,
    pub focus: Hsla,
}

impl Look {
    pub fn of(variant: IconButtonVariant, theme: &RiffTheme) -> Self {
        let p = Palette::new(theme);
        let transparent = Hsla::transparent_black();
        match variant {
            IconButtonVariant::Ghost => Look {
                fill: Fill::Solid(transparent),
                fill_toggled: Some(Fill::Solid(p.pressed)),
                fill_hover: Fill::Solid(p.fill),
                border: transparent,
                icon: p.icon,
                icon_toggled: p.ink,
                icon_hover: p.hover,
                shadows: Vec::new(),
                clear_shadow_on_toggle: false,
                focus: p.focus,
            },
            IconButtonVariant::Secondary => Look {
                fill: Fill::Solid(p.surface),
                fill_toggled: Some(Fill::Solid(p.fill)),
                fill_hover: Fill::Solid(p.surface_hover),
                border: p.line,
                icon: p.icon,
                icon_toggled: p.ink,
                icon_hover: p.hover,
                shadows: vec![css_shadow(1., 2., 0., p.surface_shadow)],
                clear_shadow_on_toggle: true,
                focus: p.focus,
            },
            IconButtonVariant::Primary => Look {
                fill: Fill::Gradient {
                    top: p.dark_top,
                    bottom: p.dark_bottom,
                    edge: p.edge,
                },
                fill_toggled: None,
                fill_hover: Fill::Gradient {
                    top: p.dark_hover_top,
                    bottom: p.dark_hover_bottom,
                    edge: p.edge,
                },
                border: transparent,
                icon: p.on_ink,
                icon_toggled: p.on_ink,
                icon_hover: p.on_ink,
                shadows: vec![
                    css_shadow(0., 0., 1., p.ring),
                    css_shadow(2., 4., 0., p.dark_shadow),
                ],
                clear_shadow_on_toggle: false,
                focus: p.focus,
            },
        }
    }

    /// Resolves `(fill, icon_color, shadows)` at toggle fraction `toggled_t`
    /// and hover fraction `hover_t`. CSS specificity puts `:hover:not(:disabled)`
    /// (`0,4,0`) above `[aria-pressed="true"]` (`0,3,0`), so hover overrides
    /// toggled fill/color while keeping `box-shadow: none` on secondary.
    pub fn resolve(&self, toggled_t: f32, hover_t: f32) -> (Fill, Hsla, Vec<BoxShadow>) {
        let mut fill = self.fill;
        if let Some(fill_toggled) = self.fill_toggled {
            fill = lerp_fill(fill, fill_toggled, toggled_t);
        }
        fill = lerp_fill(fill, self.fill_hover, hover_t);

        let icon = mix(
            mix(self.icon, self.icon_toggled, toggled_t),
            self.icon_hover,
            hover_t,
        );

        let shadows = if self.clear_shadow_on_toggle && toggled_t > 0. {
            let alpha = (1. - toggled_t).clamp(0., 1.);
            if alpha <= 0. {
                Vec::new()
            } else {
                self.shadows
                    .iter()
                    .map(|s| BoxShadow {
                        color: s.color.opacity(alpha),
                        ..*s
                    })
                    .collect()
            }
        } else {
            self.shadows.clone()
        };

        (fill, icon, shadows)
    }
}

/// The `--ui-icb-*` variables of icon-button.tsx.
struct Palette {
    icon: Hsla,
    ink: Hsla,
    hover: Hsla,
    fill: Hsla,
    pressed: Hsla,
    surface: Hsla,
    surface_hover: Hsla,
    surface_shadow: Hsla,
    line: Hsla,
    on_ink: Hsla,
    dark_top: Hsla,
    dark_bottom: Hsla,
    dark_hover_top: Hsla,
    dark_hover_bottom: Hsla,
    edge: [Hsla; 3],
    ring: Hsla,
    dark_shadow: Hsla,
    focus: Hsla,
}

impl Palette {
    fn new(theme: &RiffTheme) -> Self {
        if theme.mode.is_dark() {
            Self {
                icon: theme.muted,
                ink: theme.ink,
                hover: hex(0xededed),
                fill: rgba(255, 255, 255, 0.06),
                pressed: rgba(255, 255, 255, 0.1),
                surface: theme.surface,
                surface_hover: hex(0x2a2a2d),
                surface_shadow: rgba(0, 0, 0, 0.4),
                line: theme.border,
                on_ink: hex(0x121213),
                dark_top: hex(0xf4f4f5),
                dark_bottom: hex(0xe4e4e7),
                dark_hover_top: hex(0xffffff),
                dark_hover_bottom: hex(0xececef),
                edge: [hex(0xffffff), hex(0xececef), hex(0xd4d4d8)],
                ring: rgba(0, 0, 0, 0.6),
                dark_shadow: rgba(0, 0, 0, 0.4),
                focus: theme.focus,
            }
        } else {
            Self {
                icon: theme.muted,
                ink: theme.ink,
                hover: hex(0x404040),
                fill: hex(0xf0f0f0),
                pressed: hex(0xe8e8e8),
                surface: theme.surface,
                surface_hover: hex(0xfafafa),
                surface_shadow: rgba(0, 0, 0, 0.06),
                line: theme.border,
                on_ink: hex(0xfafafa),
                dark_top: hex(0x323137),
                dark_bottom: hex(0x201e25),
                dark_hover_top: hex(0x3c3b42),
                dark_hover_bottom: hex(0x29272f),
                edge: [hex(0x4b4951), hex(0x31303a), hex(0x18171b)],
                ring: hex(0x0d0d0d),
                dark_shadow: rgba(0, 0, 0, 0.1),
                focus: theme.focus,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    #[test]
    fn secondary_resting_icon_is_muted_and_toggled_clears_shadow() {
        let theme = RiffTheme::new(ThemeMode::Light);
        let look = Look::of(IconButtonVariant::Secondary, &theme);

        let (fill, icon, shadows) = look.resolve(0., 0.);
        assert_eq!(fill, Fill::Solid(theme.surface));
        assert_eq!(icon, theme.muted);
        assert_eq!(shadows.len(), 1);

        let (fill_toggled, icon_toggled, shadows_toggled) = look.resolve(1., 0.);
        assert_eq!(fill_toggled, Fill::Solid(hex(0xf0f0f0)));
        assert_eq!(icon_toggled, theme.ink);
        assert!(shadows_toggled.is_empty());

        // Hovered while toggled: CSS :hover:not(:disabled) beats [aria-pressed="true"]
        // on background and color, while box-shadow stays none.
        let (fill_both, icon_both, shadows_both) = look.resolve(1., 1.);
        assert_eq!(fill_both, Fill::Solid(hex(0xfafafa)));
        assert_eq!(icon_both, hex(0x404040));
        assert!(shadows_both.is_empty());
    }

    #[test]
    fn ghost_toggled_uses_pressed_fill_and_hover_wins_over_toggled() {
        let theme = RiffTheme::new(ThemeMode::Dark);
        let look = Look::of(IconButtonVariant::Ghost, &theme);

        let (fill, icon, shadows) = look.resolve(1., 0.);
        assert_eq!(fill, Fill::Solid(rgba(255, 255, 255, 0.1)));
        assert_eq!(icon, theme.ink);
        assert!(shadows.is_empty());

        let (fill_hover, icon_hover, _) = look.resolve(1., 1.);
        assert_eq!(fill_hover, Fill::Solid(rgba(255, 255, 255, 0.06)));
        assert_eq!(icon_hover, hex(0xededed));
    }
}
