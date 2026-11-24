//! riff-uiux `Spinner`: a turning arc for ongoing work — sized in `px`,
//! tinted by `currentColor`, and announced as a status only when labelled.

mod style;
#[cfg(test)]
mod tests;

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use gpui_kit::{
    App, Global, Hsla, IntoElement, ParentElement, Pixels, RenderOnce, Role, SharedString, Styled,
    Window, canvas, div, px,
};

use crate::a11y::A11yLabel;
use crate::paint::{self, Scale, Tap};
use style::{DEFAULT_SIZE, NORMAL_SPEED, REDUCED_SPEED, SPINNER_SVG, SPINNER_SVG_KEY};

/// A riff-uiux loading spinner.
///
/// ```ignore
/// Spinner::new();
/// Spinner::new().size(px(12.)).label("Checking domain");
/// ```
#[derive(Clone, IntoElement)]
pub struct Spinner {
    size: Pixels,
    label: Option<SharedString>,
    color: Option<Hsla>,
    speed: Option<Duration>,
    frozen_at: Option<Duration>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: DEFAULT_SIZE,
            label: None,
            color: None,
            speed: None,
            frozen_at: None,
        }
    }

    /// Width and height in pixels (defaults to `14px`).
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size.max(px(0.));
        self
    }

    /// Announces the spinner to screen readers (`role="status"`). Leave unset
    /// when surrounding text already describes the work. Empty counts as none.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }

    /// Explicit stroke color. When unset, inherits the current text color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Duration of one full turn (defaults to `750ms`, or `2.4s` under
    /// `reduce_motion`).
    pub fn speed(mut self, speed: Duration) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Holds the rotation at `elapsed` for story comparison screenshots.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }
}

/// Shared clock origin so un-IDed spinners turn smoothly across frames.
struct SpinEpoch(Instant);

impl Global for SpinEpoch {}

fn angle_at(elapsed: Duration, period: Duration) -> f32 {
    let period_secs = period.as_secs_f64();
    if period_secs <= 0. {
        return 0.;
    }
    let phase = (elapsed.as_secs_f64() % period_secs / period_secs) as f32;
    phase * TAU
}

impl RenderOnce for Spinner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduce_motion = cx.reduce_motion();
        let period = self.speed.unwrap_or(if reduce_motion {
            REDUCED_SPEED
        } else {
            NORMAL_SPEED
        });

        let elapsed = match self.frozen_at {
            Some(at) => at,
            None => {
                window.request_animation_frame();
                let now = cx.background_executor().now();
                let start = match cx.try_global::<SpinEpoch>() {
                    Some(epoch) => epoch.0,
                    None => {
                        cx.set_global(SpinEpoch(now));
                        now
                    }
                };
                now.saturating_duration_since(start)
            }
        };
        let angle = angle_at(elapsed, period);
        let explicit_color = self.color;

        let graphic = div().flex_none().size(self.size).child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, cx| {
                    let color = explicit_color.unwrap_or_else(|| window.text_style().color);
                    let scale = Scale {
                        center: bounds.center(),
                        factor: 1.,
                    };
                    let taps = [Tap {
                        offset: Default::default(),
                        weight: 1.,
                    }];
                    paint::paint_icon(
                        &SPINNER_SVG_KEY.into(),
                        Some(SPINNER_SVG),
                        bounds,
                        color,
                        scale,
                        1.,
                        angle,
                        &taps,
                        window,
                        cx,
                    );
                },
            )
            .size_full(),
        );

        A11yLabel::with_role(graphic, Role::Status, self.label)
    }
}
