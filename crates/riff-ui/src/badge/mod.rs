//! riff-uiux `Badge`: a small status pill in six soft tones, with a leading
//! dot or icon, a pulsing live dot, or a tabular round count up to `99+`.

mod style;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::base::Easing;
use gpui_kit::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, Empty, FontFeatures, FontWeight, Global,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels,
    RenderOnce, Role, SharedString, Styled, Window, accesskit, canvas, div,
    prelude::FluentBuilder as _, px,
};

pub use crate::button::ButtonIcon as BadgeIcon;
use crate::paint;
use crate::theme::RiffTheme;
use style::{COUNT_PAD_X, DOT_MARGIN_X, DOT_SIZE, Look, Metrics};

/// `tone`: the color role of the badge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum BadgeTone {
    /// Gray, for idle/queued/default labels.
    #[default]
    Neutral,
    /// Blue, for running or active items.
    Info,
    /// Green, for completed/verified items.
    Success,
    /// Orange, for items needing attention.
    Warning,
    /// Red, for errors/failures.
    Danger,
    /// Purple, for kinds such as Beta or Pro.
    Purple,
}

/// `size`: 20px high with 11px text (`Sm`, default), or 22px high with 12px text (`Md`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum BadgeSize {
    #[default]
    Sm,
    Md,
}

/// A riff-uiux status badge or count pill.
///
/// ```ignore
/// Badge::new().info().live().label("Running");
/// Badge::new().warning().count(128).accessibility_label("128 waiting");
/// ```
#[derive(Clone, IntoElement)]
pub struct Badge {
    tone: BadgeTone,
    size: BadgeSize,
    dot: bool,
    live: bool,
    icon: Option<BadgeIcon>,
    label: Option<SharedString>,
    count: Option<i64>,
    max: i64,
    accessibility_label: Option<SharedString>,
    frozen_at: Option<Duration>,
}

impl Default for Badge {
    fn default() -> Self {
        Self::new()
    }
}

impl Badge {
    pub fn new() -> Self {
        Self {
            tone: BadgeTone::default(),
            size: BadgeSize::default(),
            dot: false,
            live: false,
            icon: None,
            label: None,
            count: None,
            max: 99,
            accessibility_label: None,
            frozen_at: None,
        }
    }

    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn neutral(self) -> Self {
        self.tone(BadgeTone::Neutral)
    }

    pub fn info(self) -> Self {
        self.tone(BadgeTone::Info)
    }

    pub fn success(self) -> Self {
        self.tone(BadgeTone::Success)
    }

    pub fn warning(self) -> Self {
        self.tone(BadgeTone::Warning)
    }

    pub fn danger(self) -> Self {
        self.tone(BadgeTone::Danger)
    }

    pub fn purple(self) -> Self {
        self.tone(BadgeTone::Purple)
    }

    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(self) -> Self {
        self.size(BadgeSize::Sm)
    }

    pub fn medium(self) -> Self {
        self.size(BadgeSize::Md)
    }

    /// Shows a small dot in the tone's color before the text.
    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self
    }

    /// Pulses a soft halo around the dot (`1.8s` cycle); implies [`Self::dot`].
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Leading icon before the text (ignored when `dot`/`live` or `count` is set).
    pub fn icon(mut self, icon: impl Into<BadgeIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Badge text. An empty string counts as none.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }

    /// Makes the badge a round count, clamped at 0 and capped at `max+`.
    pub fn count(mut self, count: i64) -> Self {
        self.count = Some(count);
        self
    }

    /// Highest count shown in full before switching to `max+` (defaults to `99`).
    pub fn max(mut self, max: i64) -> Self {
        self.max = max;
        self
    }

    /// Accessible name (`aria-label`), recommended for bare count badges.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Holds the `live` pulse at `elapsed` into its `1.8s` cycle for screenshots.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }
}

/// Shared clock origin for `live` badge pulses so callers don't need an `ElementId`.
struct LiveEpoch(Instant);

impl Global for LiveEpoch {}

const LIVE_PERIOD: Duration = Duration::from_millis(1800);
/// `0% -> 70%` of the keyframe expands and fades out; `70% -> 100%` rests at 0.
const LIVE_ACTIVE_FRACTION: f32 = 0.7;
const LIVE_MAX_SCALE: f32 = 2.6;
const LIVE_START_ALPHA: f32 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lead {
    None,
    Dot,
    Icon,
}

fn live_halo(elapsed: Duration, ease: &Easing) -> Option<(f32, f32)> {
    let phase =
        (elapsed.as_secs_f64() % LIVE_PERIOD.as_secs_f64() / LIVE_PERIOD.as_secs_f64()) as f32;
    if phase >= LIVE_ACTIVE_FRACTION {
        return None;
    }
    let progress = ease.sample(phase / LIVE_ACTIVE_FRACTION);
    let scale = 1. + (LIVE_MAX_SCALE - 1.) * progress;
    let alpha = LIVE_START_ALPHA * (1. - progress);
    (alpha > 0.001).then_some((scale, alpha))
}

/// Formats `count` as a localized-style grouped integer (`1,234`), clamped to
/// `>= 0` and capped at `max+`.
fn format_count(count: i64, max: i64) -> SharedString {
    let n = count.max(0) as u64;
    let top = max.max(0) as u64;
    if n > top {
        format!("{}+", format_grouped(top)).into()
    } else {
        format_grouped(n).into()
    }
}

fn format_grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn tabular_figures() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_count = self.count.is_some();
        let lead = if is_count {
            Lead::None
        } else if self.live || self.dot {
            Lead::Dot
        } else if self.icon.is_some() {
            Lead::Icon
        } else {
            Lead::None
        };
        if !is_count && lead == Lead::None && self.label.is_none() {
            return A11yLabel::new(Empty.into_any_element(), None);
        }

        let theme = RiffTheme::global(cx).clone();
        let metrics = Metrics::of(self.size);
        let look = Look::of(self.tone, &theme);
        let reduce_motion = cx.reduce_motion();

        let halo = if lead == Lead::Dot && self.live && !reduce_motion {
            let elapsed = match self.frozen_at {
                Some(at) => at,
                None => {
                    window.request_animation_frame();
                    let now = cx.background_executor().now();
                    let start = match cx.try_global::<LiveEpoch>() {
                        Some(epoch) => epoch.0,
                        None => {
                            cx.set_global(LiveEpoch(now));
                            now
                        }
                    };
                    now.saturating_duration_since(start)
                }
            };
            live_halo(elapsed, &theme.ease)
        } else {
            None
        };

        let (pad_left, pad_right) = if is_count {
            (COUNT_PAD_X, COUNT_PAD_X)
        } else if lead != Lead::None {
            (metrics.lead, metrics.pad)
        } else {
            (metrics.pad, metrics.pad)
        };

        let text = self
            .count
            .map(|count| format_count(count, self.max))
            .or(self.label);

        let pill = div()
            .flex()
            .flex_none()
            .items_center()
            .when(is_count, |this| this.justify_center().min_w(metrics.height))
            .h(metrics.height)
            .max_w_full()
            .pl(pad_left)
            .pr(pad_right)
            .gap(metrics.gap)
            .rounded(px(999.))
            .bg(look.bg)
            .text_color(look.fg)
            .font_family(theme.font_family)
            .text_size(metrics.font_size)
            .font_weight(FontWeight::MEDIUM)
            .line_height(metrics.line_height)
            .font_features(tabular_figures())
            .when(lead == Lead::Dot, |this| {
                this.child(dot_mark(look.dot, halo))
            })
            .when(lead == Lead::Icon, |this| {
                this.when_some(self.icon, |this, icon| {
                    this.child(
                        gpui_kit::svg()
                            .path(icon.0)
                            .size(metrics.mark)
                            .flex_none()
                            .text_color(look.fg),
                    )
                })
            })
            .when(is_count, |this| {
                this.when_some(text.clone(), |this, text| this.child(text))
            })
            .when(!is_count, |this| {
                this.child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .when_some(text, |this, text| this.child(text)),
                )
            });

        A11yLabel::new(pill.into_any_element(), self.accessibility_label)
    }
}

fn dot_mark(color: Hsla, halo: Option<(f32, f32)>) -> impl IntoElement {
    let radius = DOT_SIZE / 2.;
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let center = bounds.center();
            if let Some((scale, alpha)) = halo {
                paint::paint_circle(center, radius * scale, color.opacity(alpha), window);
            }
            paint::paint_circle(center, radius, color, window);
        },
    )
    .flex_none()
    .mx(DOT_MARGIN_X)
    .w(DOT_SIZE)
    .h(DOT_SIZE)
}

/// Attaches an `accesskit` label to a non-interactive badge without requiring
/// an `ElementId` or adding a layout node.
struct A11yLabel<E> {
    inner: E,
    label: Option<SharedString>,
}

impl<E> A11yLabel<E> {
    fn new(inner: E, label: Option<SharedString>) -> Self {
        Self { inner, label }
    }
}

impl<E: Element> IntoElement for A11yLabel<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for A11yLabel<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner
            .a11y_role()
            .or_else(|| self.label.as_ref().map(|_| Role::GenericContainer))
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if let Some(label) = &self.label {
            node.set_label(label.as_ref());
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(prepaint, builder);
    }
}
