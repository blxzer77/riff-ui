//! riff-uiux `Kbd`: a key chip for shortcuts — one key or a combo in one chip,
//! `⌘` on a Mac and `Ctrl` elsewhere from a single `mod` key, spelled out for
//! screen readers.

mod keys;
mod style;
#[cfg(test)]
mod tests;

use gpui_kit::{
    App, BorderStyle, Bounds, Corners, Edges, Empty, FontWeight, Hsla, IntoElement, ParentElement,
    Pixels, RenderOnce, ShapedLine, SharedString, Styled, TextRun, Window, canvas, div, font,
    point, px, quad,
};

use crate::a11y::A11yLabel;
use crate::paint::{self, Scale, Tap};
use crate::theme::RiffTheme;
pub use keys::KbdPlatform;
use keys::{KeyPart, describe, spoken_combo};
use style::{KEYS_OFFSET_Y, Look, Metrics};

/// `size`: 18px high with 11px text (`Sm`, default), or 22px high with 12px text (`Md`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum KbdSize {
    #[default]
    Sm,
    Md,
}

/// A riff-uiux keyboard shortcut chip.
///
/// ```ignore
/// Kbd::new().keys(["mod", "K"]);
/// Kbd::new().key("esc");
/// Kbd::new().medium().keys(["mod", "shift", "A"]);
/// Kbd::new().label("Fn");
/// ```
#[derive(Clone, IntoElement)]
pub struct Kbd {
    keys: Vec<SharedString>,
    label: Option<SharedString>,
    size: KbdSize,
    platform: Option<KbdPlatform>,
    accessibility_label: Option<SharedString>,
}

impl Default for Kbd {
    fn default() -> Self {
        Self::new()
    }
}

impl Kbd {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            label: None,
            size: KbdSize::default(),
            platform: None,
            accessibility_label: None,
        }
    }

    /// Adds one key to the combo (e.g. `"esc"` or `"mod"`).
    pub fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.keys.push(key.into());
        self
    }

    /// Sets the keys in the combo (e.g. `["mod", "K"]`).
    pub fn keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Plain text shown when `keys` is empty (e.g. `"Fn"`). Empty counts as none.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }

    pub fn size(mut self, size: KbdSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(self) -> Self {
        self.size(KbdSize::Sm)
    }

    pub fn medium(self) -> Self {
        self.size(KbdSize::Md)
    }

    /// Forces Mac (`⌘ ⌥ ⇧`) or Windows/Linux (`Ctrl Alt Shift`) key names.
    /// Defaults to the active platform.
    pub fn platform(mut self, platform: KbdPlatform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Overrides the accessible label (by default, `keys` are spelled out
    /// automatically, e.g. `"Command Shift A"`).
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

struct ShapedKey {
    line: ShapedLine,
    width: Pixels,
    font_size: Pixels,
}

fn shape_text(
    text: SharedString,
    font_size: Pixels,
    tracking: Pixels,
    theme: &RiffTheme,
    window: &mut Window,
) -> ShapedKey {
    let mut f = font(theme.font_family.clone());
    f.weight = FontWeight::MEDIUM;
    f.features = paint::tabular_figures();
    let run = TextRun {
        len: text.len(),
        font: f,
        color: Hsla::black(),
        ..Default::default()
    };
    let line = window
        .text_system()
        .shape_line(text.clone(), font_size, &[run], None);
    let glyph_count = line
        .runs
        .iter()
        .map(|run| run.glyphs.len())
        .sum::<usize>()
        .max(text.chars().count());
    let width = line.width + tracking * glyph_count as f32;
    ShapedKey {
        line,
        width,
        font_size,
    }
}

impl RenderOnce for Kbd {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let platform = self.platform.unwrap_or_else(|| {
            if matches!(cx.platform_info().os, gpui_kit::OperatingSystem::Mac) {
                KbdPlatform::Mac
            } else {
                KbdPlatform::Other
            }
        });

        let parts: Vec<KeyPart> = self
            .keys
            .iter()
            .filter(|k| !k.trim().is_empty())
            .map(|k| describe(k, platform))
            .collect();

        if parts.is_empty() && self.label.is_none() {
            return A11yLabel::new(Empty.into_any_element(), None);
        }

        let theme = RiffTheme::global(cx).clone();
        let metrics = Metrics::of(self.size, platform, &theme);
        let look = Look::of(&theme);

        let is_keys = !parts.is_empty();
        let spoken = if is_keys {
            self.accessibility_label
                .or_else(|| Some(spoken_combo(&parts)))
        } else {
            self.accessibility_label.or_else(|| self.label.clone())
        };

        let items: Vec<ShapedKey> = if is_keys {
            parts
                .into_iter()
                .map(|part| {
                    let size = if part.glyph {
                        metrics.glyph_font_size
                    } else {
                        metrics.font_size
                    };
                    shape_text(part.label, size, metrics.tracking, &theme, window)
                })
                .collect()
        } else {
            vec![shape_text(
                self.label.expect("checked non-empty"),
                metrics.font_size,
                metrics.tracking,
                &theme,
                window,
            )]
        };

        let gaps = metrics.gap * items.len().saturating_sub(1) as f32;
        let content_width = items.iter().fold(px(0.), |acc, item| acc + item.width) + gaps;
        let box_width = (metrics.pad_x * 2. + content_width).max(metrics.height);

        let painter = Painter {
            metrics,
            look,
            items,
            content_width,
            offset_y: if is_keys { KEYS_OFFSET_Y } else { px(0.) },
        };

        let chip = div().flex_none().w(box_width).h(metrics.height).child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    painter.paint(bounds, window);
                },
            )
            .size_full(),
        );

        A11yLabel::new(chip.into_any_element(), spoken)
    }
}

struct Painter {
    metrics: Metrics,
    look: Look,
    items: Vec<ShapedKey>,
    content_width: Pixels,
    offset_y: Pixels,
}

impl Painter {
    fn paint(self, bounds: Bounds<Pixels>, window: &mut Window) {
        let m = self.metrics;
        let r = Corners::all(m.radius);
        window.paint_quad(quad(
            bounds,
            r,
            self.look.bg,
            Edges::all(px(0.)),
            Hsla::transparent_black(),
            BorderStyle::Solid,
        ));
        window.paint_inset_shadows(bounds, r, &self.look.inset_shadows);

        let sf = window.scale_factor();
        let scale = Scale {
            center: bounds.center(),
            factor: 1.,
        };
        let taps = [Tap {
            offset: point(px(0.), px(0.)),
            weight: 1.,
        }];
        let mut x = bounds.center().x - self.content_width / 2.;
        let middle = bounds.center().y + self.offset_y;

        for item in &self.items {
            // `.ui-kbd` sets `line-height: 1` on inline-flex items: Chrome lays out
            // each key with `half_leading = floor((line_height - (ascent + descent)) / 2)`
            // using Blink's rounded `ascent` and `descent`.
            let ascent = item.line.ascent.round();
            let descent = item.line.descent.round();
            let half_leading = ((item.font_size - (ascent + descent)) / 2.).floor();
            let baseline = middle - item.font_size / 2. + half_leading + ascent;
            let baseline = px((baseline.as_f32() * sf + 0.5).floor() / sf);
            paint::paint_text_tracked(
                &item.line,
                point(x, baseline),
                item.font_size,
                m.tracking,
                self.look.ink,
                scale,
                &taps,
                window,
            );
            x += item.width + m.gap;
        }
    }
}
