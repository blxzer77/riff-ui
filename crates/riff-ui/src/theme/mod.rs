//! The riff design tokens, and their projection onto the gpui-component theme.
//!
//! Every field mirrors a `--riff-*` CSS variable of riff-uiux: `border_strong`
//! is `--riff-border-strong`. riff-ui components read [`RiffTheme`]; the
//! gpui-component [`Theme`] is kept in step so wrapped gpui-component widgets
//! use the same palette.
//!
//! Not carried over: `--riff-font-mono` / `--riff-mono` name macOS fonts only,
//! so monospace text keeps gpui-component's per-platform mono font.
//!
//! Several tokens are translucent (`--riff-ink` is `rgba(0, 0, 0, 0.88)`), so
//! their on-screen color depends on what is behind them, as in the browser.

mod palette;

use std::ops::Deref;
use std::rc::Rc;

use gpui_kit::base::Easing;
use gpui_kit::component::{Theme, ThemeConfig};
use gpui_kit::{App, BoxShadow, Global, Hsla, Pixels, Rgba, SharedString, hsla, point, px};

pub use gpui_kit::component::ThemeMode;

/// `--riff-font`. GPUI takes a single family and falls back on its own when
/// it is not installed.
const FONT_FAMILY: &str = "Inter";
/// `--riff-radius-sm`, `--riff-radius`, `--riff-radius-lg`.
const RADIUS_SM: f32 = 5.;
const RADIUS: f32 = 7.;
const RADIUS_LG: f32 = 10.;

macro_rules! colors {
    ($($name:ident),* $(,)?) => {
        /// The color tokens of one mode, as CSS color values.
        struct Palette {
            $($name: &'static str,)*
        }

        impl Palette {
            /// The value of `--riff-<name>`.
            fn get(&self, css_name: &str) -> Option<&'static str> {
                $(if css_name == stringify!($name).replace('_', "-") {
                    return Some(self.$name);
                })*
                None
            }
        }

        /// The color tokens of the active mode.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct Colors {
            $(pub $name: Hsla,)*
        }

        impl Colors {
            fn from_palette(palette: &Palette) -> Self {
                Self {
                    $($name: resolve(palette, palette.$name),)*
                }
            }

            /// Every color with its token name, in declaration order.
            pub fn entries(&self) -> Vec<(&'static str, Hsla)> {
                vec![$((stringify!($name), self.$name),)*]
            }
        }
    };
}

colors!(
    ink,
    muted,
    placeholder,
    surface,
    overlay,
    subtle,
    border,
    border_strong,
    primary,
    on_primary,
    focus,
    danger,
    danger_soft,
    success,
    success_soft,
    warning,
    warning_soft,
    info,
    info_soft,
    blue,
    red,
    green,
    amber,
    orange,
    purple,
    pink,
    teal,
    cyan,
    violet,
    indigo,
);

/// Resolves a palette value, following `var(--riff-*, <fallback>)` within
/// the same palette. Panics on a malformed value: the palettes are constants,
/// so this is caught by the tests rather than at a user's runtime.
fn resolve(palette: &Palette, value: &str) -> Hsla {
    let value = value.trim();
    if let Some(inner) = value.strip_prefix("var(").and_then(|v| v.strip_suffix(')')) {
        let (name, fallback) = inner.split_once(',').unwrap_or((inner, ""));
        let token = name.trim().strip_prefix("--riff-");
        return match token.and_then(|token| palette.get(token)) {
            Some(target) => resolve(palette, target),
            None => resolve(palette, fallback),
        };
    }
    parse_color(value).unwrap_or_else(|| panic!("invalid palette color {value:?}"))
}

/// Parses `#hex`, `rgb(r, g, b)` or `rgba(r, g, b, a)`.
fn parse_color(value: &str) -> Option<Hsla> {
    if value.starts_with('#') {
        return Rgba::try_from(value).ok().map(Into::into);
    }
    let args = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts = args
        .split(',')
        .map(|part| part.trim().parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    let (r, g, b, a) = match parts[..] {
        [r, g, b] => (r, g, b, 1.),
        [r, g, b, a] => (r, g, b, a),
        _ => return None,
    };
    let rgba = Rgba {
        r: r / 255.,
        g: g / 255.,
        b: b / 255.,
        a,
    };
    Some(rgba.into())
}

/// `#rrggbbaa`, the one form gpui-component's theme config parses that keeps
/// the alpha of an `rgba()` token.
fn to_hex(color: Hsla) -> SharedString {
    let rgba = color.to_rgb();
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        channel(rgba.r),
        channel(rgba.g),
        channel(rgba.b),
        channel(rgba.a)
    )
    .into()
}

fn palette(mode: ThemeMode) -> &'static Palette {
    if mode.is_dark() {
        &palette::DARK
    } else {
        &palette::LIGHT
    }
}

/// The riff design tokens for one mode. Read it with [`RiffTheme::global`].
#[derive(Clone, Debug)]
pub struct RiffTheme {
    pub mode: ThemeMode,
    pub colors: Colors,
    pub radius_sm: Pixels,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub font_family: SharedString,
    /// `--riff-shadow-card`
    pub shadow_card: Vec<BoxShadow>,
    /// `--riff-shadow-popover`
    pub shadow_popover: Vec<BoxShadow>,
    /// `--riff-ease`
    pub ease: Easing,
}

impl Global for RiffTheme {}

impl Deref for RiffTheme {
    type Target = Colors;

    fn deref(&self) -> &Colors {
        &self.colors
    }
}

impl RiffTheme {
    pub fn new(mode: ThemeMode) -> Self {
        let colors = Colors::from_palette(palette(mode));
        let black = |alpha| hsla(0., 0., 0., alpha);
        let white = |alpha| hsla(0., 0., 1., alpha);

        // `--riff-shadow-card` and `--riff-shadow-popover` from tokens.css.
        let (shadow_card, shadow_popover) = if mode.is_dark() {
            (
                vec![ring(white(0.1)), shadow(1., 2., 0., black(0.4))],
                vec![
                    ring(colors.border_strong),
                    shadow(12., 32., -8., black(0.6)),
                ],
            )
        } else {
            (
                vec![
                    ring(black(0.06)),
                    shadow(1., 1., -1., black(0.1)),
                    shadow(1., 2., 0., black(0.05)),
                ],
                vec![
                    ring(colors.border_strong),
                    shadow(12., 32., -8., black(0.16)),
                ],
            )
        };

        Self {
            mode,
            colors,
            radius_sm: px(RADIUS_SM),
            radius: px(RADIUS),
            radius_lg: px(RADIUS_LG),
            font_family: FONT_FAMILY.into(),
            shadow_card,
            shadow_popover,
            ease: Easing::cubic_bezier(0.22, 1., 0.36, 1.).expect("valid control points"),
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
}

/// A hairline outline drawn as a 0.5px spread shadow.
fn ring(color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(0.5),
        inset: false,
    }
}

fn shadow(y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

/// The gpui-component theme for `mode`, built from the same palette.
fn component_config(mode: ThemeMode) -> ThemeConfig {
    let p = Colors::from_palette(palette(mode));
    let mut config = ThemeConfig::default();
    config.name = if mode.is_dark() {
        "Riff Dark"
    } else {
        "Riff Light"
    }
    .into();
    config.mode = mode;
    config.font_family = Some(FONT_FAMILY.into());
    config.radius = Some(RADIUS as usize);
    config.radius_lg = Some(RADIUS_LG as usize);

    let c = &mut config.colors;
    c.background = Some(to_hex(p.surface));
    c.foreground = Some(to_hex(p.ink));
    c.border = Some(to_hex(p.border));
    c.input = Some(to_hex(p.border_strong));
    c.ring = Some(to_hex(p.focus));
    c.muted = Some(to_hex(p.subtle));
    c.muted_foreground = Some(to_hex(p.muted));
    c.accent = Some(to_hex(p.subtle));
    c.accent_foreground = Some(to_hex(p.ink));
    c.secondary = Some(to_hex(p.subtle));
    c.secondary_foreground = Some(to_hex(p.ink));
    c.primary = Some(to_hex(p.primary));
    c.primary_foreground = Some(to_hex(p.on_primary));
    c.popover = Some(to_hex(p.overlay));
    c.popover_foreground = Some(to_hex(p.ink));
    c.danger = Some(to_hex(p.danger));
    c.danger_foreground = Some(to_hex(p.on_primary));
    c.success = Some(to_hex(p.success));
    c.success_foreground = Some(to_hex(p.on_primary));
    c.warning = Some(to_hex(p.warning));
    c.warning_foreground = Some(to_hex(p.on_primary));
    c.info = Some(to_hex(p.info));
    c.info_foreground = Some(to_hex(p.on_primary));
    c.link = Some(to_hex(p.blue));
    config
}

pub(crate) fn init(cx: &mut App) {
    let light = Rc::new(component_config(ThemeMode::Light));
    let dark = Rc::new(component_config(ThemeMode::Dark));
    Theme::update(cx, |theme| {
        theme.light_theme = light;
        theme.dark_theme = dark;
    });
    set_mode(Theme::global(cx).mode, cx);

    // A mode change made through gpui-component (e.g. following the system
    // appearance) switches the riff tokens too.
    cx.observe_global::<Theme>(|cx| {
        let mode = Theme::global(cx).mode;
        if RiffTheme::global(cx).mode != mode {
            cx.set_global(RiffTheme::new(mode));
        }
    })
    .detach();
}

/// Switches both the riff tokens and the gpui-component theme to `mode`.
pub fn set_mode(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    cx.set_global(RiffTheme::new(mode));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Equal to within one 8-bit step per channel: the projection onto
    /// gpui-component goes through `#rrggbbaa`.
    #[track_caller]
    fn assert_close(actual: Hsla, expected: Hsla) {
        let (a, e) = (actual.to_rgb(), expected.to_rgb());
        let close = [(a.r, e.r), (a.g, e.g), (a.b, e.b), (a.a, e.a)]
            .iter()
            .all(|(x, y)| (x - y).abs() <= 1. / 255. + f32::EPSILON);
        assert!(close, "{a:?} != {e:?}");
    }

    fn rgba(r: f32, g: f32, b: f32, a: f32) -> Hsla {
        Rgba {
            r: r / 255.,
            g: g / 255.,
            b: b / 255.,
            a,
        }
        .into()
    }

    #[test]
    fn parses_css_color_forms() {
        assert_close(parse_color("#4fb0fe").unwrap(), rgba(79., 176., 254., 1.));
        assert_close(
            parse_color("rgba(0, 0, 0, 0.88)").unwrap(),
            rgba(0., 0., 0., 0.88),
        );
        assert_close(
            parse_color("rgb(255, 107, 95)").unwrap(),
            rgba(255., 107., 95., 1.),
        );
        assert_eq!(parse_color("rgba(0, 0, 0)").map(|c| c.a), Some(1.));
        assert!(parse_color("rgba(0, 0)").is_none());
        assert!(parse_color("hsl(0, 0%, 0%)").is_none());
    }

    #[test]
    fn var_resolves_to_the_named_token_then_the_fallback() {
        let light = Colors::from_palette(&palette::LIGHT);
        assert_eq!(light.overlay, light.surface);
        assert_close(
            resolve(&palette::LIGHT, "var(--riff-missing, #123456)"),
            rgba(18., 52., 86., 1.),
        );
    }

    #[test]
    fn palettes_parse() {
        let light = Colors::from_palette(&palette::LIGHT);
        let dark = Colors::from_palette(&palette::DARK);
        assert_close(light.ink, rgba(0., 0., 0., 0.88));
        assert_close(dark.ink, rgba(255., 255., 255., 0.9));
        assert_close(light.focus, rgba(0., 96., 226., 0.7));
        assert_close(dark.focus, rgba(77., 141., 255., 0.8));
        assert_eq!(dark.overlay, dark.surface);
    }

    #[test]
    fn hex_keeps_alpha() {
        assert_eq!(to_hex(rgba(0., 0., 0., 0.88)).as_ref(), "#000000e0");
        assert_eq!(to_hex(rgba(79., 176., 254., 1.)).as_ref(), "#4fb0feff");
    }

    #[gpui_kit::test]
    fn init_projects_tokens_onto_component_theme(cx: &mut gpui_kit::TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| {
            let riff = RiffTheme::global(cx);
            let theme = Theme::global(cx);
            assert_eq!(riff.mode, ThemeMode::Light);
            assert_close(theme.foreground, riff.ink);
            assert_close(theme.background, riff.surface);
            assert_close(theme.primary, riff.primary);
            assert_close(theme.border, riff.border);
            assert_close(theme.ring, riff.focus);
            assert_eq!(theme.radius, riff.radius);
        });
    }

    #[gpui_kit::test]
    fn set_mode_switches_both_themes(cx: &mut gpui_kit::TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| set_mode(ThemeMode::Dark, cx));
        cx.update(|cx| {
            let riff = RiffTheme::global(cx);
            assert_eq!(riff.mode, ThemeMode::Dark);
            assert_close(riff.ink, rgba(255., 255., 255., 0.9));
            assert_close(Theme::global(cx).foreground, riff.ink);
            assert_close(Theme::global(cx).background, riff.surface);
        });
    }

    #[gpui_kit::test]
    fn component_mode_change_switches_riff_tokens(cx: &mut gpui_kit::TestAppContext) {
        cx.update(crate::init);
        cx.update(|cx| Theme::change(ThemeMode::Dark, None, cx));
        cx.update(|cx| {
            let riff = RiffTheme::global(cx);
            assert_eq!(riff.mode, ThemeMode::Dark);
            assert_close(Theme::global(cx).foreground, riff.ink);
        });
    }
}
