//! riff-uiux `Skeleton` and `Loading`: placeholder shapes sized to what they
//! replace, breathing in step with a single synchronized sheen, plus a delayed
//! `Loading` status container so quick loads never flash gray.

mod style;
#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use gpui_kit::base::Easing;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, Bounds, ContentMask, Div, Global, Hsla, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StyleRefinement, Styled, Window, canvas, div,
    linear_color_stop, linear_gradient, point, px, size,
};

use crate::a11y::A11yLabel;
use crate::paint;
use crate::theme::RiffTheme;
use style::{
    DEFAULT_HEIGHT, DEFAULT_SPEED, LOADING_DEFAULT_DELAY, LOADING_FADE_DURATION, Look,
    SWEEP_ACTIVE_FRACTION,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Radius {
    Default,
    Px(Pixels),
    Circle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LineBox {
    None,
    Auto,
    Fixed(Pixels),
}

/// A riff-uiux skeleton placeholder.
///
/// ```ignore
/// Skeleton::new().width(px(16.)).height(px(16.)).radius(px(4.));
/// Skeleton::new().line().width(px(180.)).height(px(10.));
/// ```
#[derive(Clone, IntoElement)]
pub struct Skeleton {
    width: Option<Pixels>,
    height: Pixels,
    radius: Radius,
    line: LineBox,
    speed: Duration,
    frozen_at: Option<Duration>,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            width: None,
            height: DEFAULT_HEIGHT,
            radius: Radius::Default,
            line: LineBox::None,
            speed: DEFAULT_SPEED,
            frozen_at: None,
        }
    }

    /// Fixed width in pixels (clamped to `max-width: 100%` of its parent).
    /// Defaults to `100%`.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width.max(px(0.)));
        self
    }

    /// The shape's own height (defaults to `12px`).
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height.max(px(0.));
        self
    }

    /// Corner radius in pixels (defaults to `theme.radius_sm`).
    pub fn radius(mut self, radius: Pixels) -> Self {
        self.radius = Radius::Px(radius.max(px(0.)));
        self
    }

    /// Pill / circular radius (`50%`, half the shorter edge).
    pub fn circle(mut self) -> Self {
        self.radius = Radius::Circle;
        self
    }

    /// Wraps the bar in a box as tall as the surrounding text line (`1lh`)
    /// and centers it vertically so rows keep their height when text arrives.
    pub fn line(mut self) -> Self {
        self.line = LineBox::Auto;
        self
    }

    /// Wraps the bar in a line box of explicit height `line_height`.
    pub fn line_height(mut self, line_height: Pixels) -> Self {
        self.line = LineBox::Fixed(line_height.max(px(0.)));
        self
    }

    /// Duration of one sweep cycle (defaults to `1.6s`).
    pub fn speed(mut self, speed: Duration) -> Self {
        self.speed = speed;
        self
    }

    /// Holds the sheen sweep at `elapsed` for story comparison screenshots.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }
}

/// Shared clock origin so every skeleton sweeps in step without needing an `ElementId`.
struct SkeletonEpoch(Instant);

impl Global for SkeletonEpoch {}

/// Normalized sweep position `-1.0..=1.0` (`-100%..100%`) at `elapsed`, or
/// `None` during the `60%..100%` off-screen pause.
pub(super) fn sweep_translate(elapsed: Duration, period: Duration, ease: &Easing) -> Option<f32> {
    let period_secs = period.as_secs_f64();
    if period_secs <= 0. {
        return None;
    }
    let phase = (elapsed.as_secs_f64() % period_secs / period_secs) as f32;
    if phase >= SWEEP_ACTIVE_FRACTION {
        return None;
    }
    let t = ease.sample(phase / SWEEP_ACTIVE_FRACTION);
    Some(-1. + 2. * t)
}

impl RenderOnce for Skeleton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let look = Look::of(theme);
        let ease = theme.ease.clone();
        let reduce_motion = cx.reduce_motion();

        let sweep = if reduce_motion {
            None
        } else {
            let elapsed = match self.frozen_at {
                Some(at) => at,
                None => {
                    window.request_animation_frame();
                    let now = cx.background_executor().now();
                    let start = match cx.try_global::<SkeletonEpoch>() {
                        Some(epoch) => epoch.0,
                        None => {
                            cx.set_global(SkeletonEpoch(now));
                            now
                        }
                    };
                    now.saturating_duration_since(start)
                }
            };
            sweep_translate(elapsed, self.speed, &ease)
        };

        let radius = self.radius;
        let shape = div()
            .flex_none()
            .max_w_full()
            .h(self.height)
            .map(|el| match self.width {
                Some(w) => el.w(w),
                None => el.w_full(),
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window, _| {
                        let r = match radius {
                            Radius::Default => look.default_radius,
                            Radius::Px(r) => r,
                            Radius::Circle => bounds.size.width.min(bounds.size.height) * 0.5,
                        }
                        .min(bounds.size.width * 0.5)
                        .min(bounds.size.height * 0.5);

                        paint::paint_rounded(bounds, r, look.fill, window);

                        if let Some(translate) = sweep {
                            paint_sheen(bounds, r, look.sheen, translate, window);
                        }
                    },
                )
                .size_full(),
            );

        let line_h = match self.line {
            LineBox::None => return shape.into_any_element(),
            LineBox::Auto => Some(window.text_style().line_height_in_pixels(window.rem_size())),
            LineBox::Fixed(h) => Some(h),
        };

        div()
            .flex()
            .items_center()
            .min_w_0()
            .max_w_full()
            .when_some(line_h, |el, h| el.h(h))
            .child(shape)
            .into_any_element()
    }
}

/// Paints `linear-gradient(90deg, transparent, sheen, transparent)` clipped to
/// the skeleton's rounded box, shifted horizontally by `translate * bounds.width`.
fn paint_sheen(
    bounds: Bounds<Pixels>,
    r: Pixels,
    sheen: Hsla,
    translate: f32,
    window: &mut Window,
) {
    let w = bounds.size.width;
    if w <= px(0.) || bounds.size.height <= px(0.) || sheen.a <= 0. {
        return;
    }
    let left_edge = bounds.left() + w * translate;
    let mid = left_edge + w * 0.5;
    let right_edge = left_edge + w;

    // `paint_rounded` expands its path bounds by a 1px fringe on each side;
    // map gradient stops into that padded span so every stop stays in [0, 1].
    let pad = px(1.);
    let path_left = bounds.left() - pad;
    let path_span = w + pad * 2.;
    let half = w * 0.5;

    for (seg_start, seg_end, from_full, to_full) in [
        (left_edge, mid, false, true),
        (mid, right_edge, true, false),
    ] {
        let cx0 = seg_start.max(bounds.left());
        let cx1 = seg_end.min(bounds.right());
        if cx1 <= cx0 {
            continue;
        }
        let u0 = ((cx0 - seg_start) / half).clamp(0., 1.);
        let u1 = ((cx1 - seg_start) / half).clamp(0., 1.);
        let alpha_at = |u: f32| match (from_full, to_full) {
            (false, true) => sheen.a * u,
            (true, false) => sheen.a * (1. - u),
            _ => sheen.a,
        };
        let c0 = Hsla {
            a: alpha_at(u0),
            ..sheen
        };
        let c1 = Hsla {
            a: alpha_at(u1),
            ..sheen
        };
        let p0 = ((cx0 - path_left) / path_span).clamp(0., 1.);
        let p1 = ((cx1 - path_left) / path_span).clamp(0., 1.);
        if p1 <= p0 {
            continue;
        }
        let clip = ContentMask {
            bounds: Bounds::new(
                point(cx0, bounds.top()),
                size(cx1 - cx0, bounds.size.height),
            ),
        };
        window.with_content_mask(Some(clip), |window| {
            paint::paint_rounded(
                bounds,
                r,
                linear_gradient(90., linear_color_stop(c0, p0), linear_color_stop(c1, p1)),
                window,
            );
        });
    }
}

/// Wraps a loading area's skeletons: exposes `role="status"` + `aria-busy`
/// with a label, and fades in after a `150ms` delay so quick loads don't flash.
///
/// ```ignore
/// Loading::new().label("Loading runs").flex().flex_col().gap_2().child(...)
/// ```
#[derive(IntoElement)]
pub struct Loading {
    base: Div,
    label: SharedString,
    delay: Duration,
    fade: Duration,
    frozen_at: Option<Duration>,
}

impl Default for Loading {
    fn default() -> Self {
        Self::new()
    }
}

impl Loading {
    pub fn new() -> Self {
        Self {
            base: div(),
            label: "Loading".into(),
            delay: LOADING_DEFAULT_DELAY,
            fade: LOADING_FADE_DURATION,
            frozen_at: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = if label.is_empty() {
            "Loading".into()
        } else {
            label
        };
        self
    }

    /// Delay before skeletons fade in (defaults to `150ms`).
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Holds the delayed fade-in at `elapsed` for testing or screenshots.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }
}

impl ParentElement for Loading {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for Loading {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

pub(super) fn loading_opacity(elapsed: Duration, delay: Duration, fade: Duration) -> f32 {
    if elapsed <= delay {
        return 0.;
    }
    let fade_secs = fade.as_secs_f32();
    if fade_secs <= 0. {
        return 1.;
    }
    let t = ((elapsed - delay).as_secs_f32() / fade_secs).clamp(0., 1.);
    // CSS `ease` = cubic-bezier(0.25, 0.1, 0.25, 1.0)
    Easing::cubic_bezier(0.25, 0.1, 0.25, 1.)
        .expect("valid control points")
        .sample(t)
}

impl RenderOnce for Loading {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduce_motion = cx.reduce_motion();
        let opacity = if reduce_motion {
            1.
        } else {
            let elapsed = match self.frozen_at {
                Some(at) => at,
                None => {
                    let mounted_at = *window
                        .use_state(cx, |_, cx| cx.background_executor().now())
                        .read(cx);
                    let elapsed = cx
                        .background_executor()
                        .now()
                        .saturating_duration_since(mounted_at);
                    if elapsed < self.delay + self.fade {
                        window.request_animation_frame();
                    }
                    elapsed
                }
            };
            loading_opacity(elapsed, self.delay, self.fade)
        };

        A11yLabel::with_role(self.base.opacity(opacity), Role::Status, Some(self.label)).busy(true)
    }
}
