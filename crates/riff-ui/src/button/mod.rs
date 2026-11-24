//! riff-uiux `Button`: primary, secondary, ghost and destructive buttons with
//! icons, three sizes and a shimmering loading state.
//!
//! The template for riff-ui components: gpui-base supplies the behavior (here
//! [`base::Button`]: activation, focus, Enter/Space, disabled, accessibility)
//! and this module supplies everything visible. The button paints itself
//! because it scales to 97% while pressed, and GPUI transforms only glyphs and
//! SVGs, not boxes: see [`crate::paint`].

mod a11y;
mod state;
mod style;
#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{self, Easing, Transition, transition};
use gpui_kit::{
    App, Bounds, ClickEvent, CursorStyle, ElementId, FontWeight, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseUpEvent, ParentElement, Pixels, RenderOnce, ShapedLine,
    SharedString, StatefulInteractiveElement, Styled, TextRun, Window, canvas, font, point, px,
    size,
};

use crate::paint::{self, Scale, Tap, lerp_fill, mix};
use crate::theme::RiffTheme;
pub use state::ButtonState;
use state::{ICON, ICON_RIGHT, LABEL, Shared};
use style::{BORDER, Look, Metrics};

/// `variant`: what the button looks like.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ButtonVariant {
    /// Dark and filled.
    #[default]
    Primary,
    /// White with an outline.
    Secondary,
    /// Text only; the gray layer appears on hover.
    Ghost,
    /// Red, for deleting.
    Destructive,
}

/// `size`: 32, 40 or 48px high.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// An icon: an SVG path in the application's asset source, such as a
/// [`gpui_kit::assets::IconName`].
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ButtonIcon(pub(crate) SharedString);

impl<T: gpui_kit::assets::IconNamed> From<T> for ButtonIcon {
    fn from(icon: T) -> Self {
        Self(icon.path())
    }
}

impl ButtonIcon {
    /// An icon at an asset path, e.g. `"icons/plus.svg"`.
    pub fn path(path: impl Into<SharedString>) -> Self {
        Self(path.into())
    }
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A riff-uiux button.
///
/// ```ignore
/// Button::new("save").label("Save changes").on_click(|_, _, _| save());
/// Button::new("add").ghost().small().icon(IconName::Plus).accessibility_label("Add");
/// ```
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    variant: ButtonVariant,
    size: ButtonSize,
    label: Option<SharedString>,
    icon: Option<ButtonIcon>,
    icon_right: Option<ButtonIcon>,
    loading: bool,
    disabled: bool,
    accessibility_label: Option<SharedString>,
    on_click: Option<ClickHandler>,
    forced: ButtonState,
    frozen_at: Option<Duration>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            label: None,
            icon: None,
            icon_right: None,
            loading: false,
            disabled: false,
            accessibility_label: None,
            on_click: None,
            forced: ButtonState::default(),
            frozen_at: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }

    pub fn secondary(self) -> Self {
        self.variant(ButtonVariant::Secondary)
    }

    pub fn ghost(self) -> Self {
        self.variant(ButtonVariant::Ghost)
    }

    pub fn destructive(self) -> Self {
        self.variant(ButtonVariant::Destructive)
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(self) -> Self {
        self.size(ButtonSize::Sm)
    }

    pub fn large(self) -> Self {
        self.size(ButtonSize::Lg)
    }

    /// The text. An empty label counts as none, so an icon stays centered.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }

    /// Icon before the label. Without a label the button becomes square.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Icon after the label.
    pub fn icon_right(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon_right = Some(icon.into());
        self
    }

    /// Swaps the left icon for a spinner, sweeps a band across the label and
    /// ignores activation. Unlike `disabled`, the button stays focusable and
    /// tells assistive technology it is busy.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The accessible name; defaults to the label. Icon-only buttons need one.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Shows interactive states regardless of input, for galleries and
    /// screenshots. Live input still applies on top.
    pub fn force_state(mut self, state: ButtonState) -> Self {
        self.forced = state;
        self
    }

    /// Holds the loading sweep and spinner at `elapsed` into their cycle and
    /// skips enter animations, so screenshots are reproducible.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }

    fn icon_only(&self) -> bool {
        self.label.is_none() && (self.icon.is_some() || self.icon_right.is_some())
    }
}

/// `transition: ... 160ms ease`, and `transform 160ms var(--ui-btn-ease)`.
const STATE_DURATION: Duration = Duration::from_millis(160);
/// The width glide and the label enter: 480ms `cubic-bezier(0.25, 1, 0.5, 1)`.
const MORPH_DURATION: Duration = Duration::from_millis(480);
/// The icon enter: 520ms on the same curve.
const ICON_IN_DURATION: Duration = Duration::from_millis(520);
/// `--ui-btn-load` × 1.6: one sweep of the loading band.
const SWEEP_DURATION: Duration = Duration::from_millis(1040);
/// One turn of the spinner.
const SPIN_DURATION: Duration = Duration::from_millis(900);
/// `scale(0.97)` while pressed.
const PRESSED_SCALE: f32 = 0.97;
/// `opacity: 0.4` while disabled.
const DISABLED_OPACITY: f32 = 0.4;
/// `color-mix(in srgb, currentColor 55%, transparent)` on a loading label.
const LOADING_LABEL_ALPHA: f32 = 0.55;

fn morph_easing() -> Easing {
    Easing::cubic_bezier(0.25, 1., 0.5, 1.).expect("valid control points")
}

/// The loading spinner of button.tsx: a faint full circle and a quarter arc.
const SPINNER_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="8.5" opacity="0.2"/><path d="M12 3.5a8.5 8.5 0 0 1 8.5 8.5"/></svg>"#;
const SPINNER_KEY: &str = "riff-button-spinner";

/// The shaped label and the button geometry it implies.
struct Content {
    metrics: Metrics,
    label: Option<ShapedLine>,
    /// The shaped width with `letter-spacing: -0.01em` applied.
    label_width: Pixels,
    width: Pixels,
}

/// The width of a shaped label under the button's letter spacing. CSS adds
/// the spacing after every character, including the last, which the painted
/// glyphs leave out; that trailing space is not visible, but it makes the
/// box match riff-uiux's.
fn label_width(line: &ShapedLine, text_len: usize, font_size: Pixels) -> Pixels {
    let glyph_count = line
        .runs
        .iter()
        .map(|run| run.glyphs.len())
        .sum::<usize>()
        .max(text_len);
    line.width + paint::tracking(font_size) * glyph_count as f32
}

impl Content {
    fn new(button: &Button, theme: &RiffTheme, window: &mut Window) -> Self {
        let metrics = Metrics::of(button.size, theme);
        let label = button.label.as_ref().map(|text| {
            let mut f = font(theme.font_family.clone());
            f.weight = FontWeight::MEDIUM;
            let run = TextRun {
                len: text.len(),
                font: f,
                color: Hsla::black(),
                ..Default::default()
            };
            window
                .text_system()
                .shape_line(text.clone(), metrics.font_size, &[run], None)
        });
        let label_width = match (&label, &button.label) {
            (Some(line), Some(text)) => label_width(line, text.len(), metrics.font_size),
            _ => px(0.),
        };
        let width = if button.icon_only() {
            metrics.height
        } else {
            let icons =
                usize::from(button.icon.is_some()) + usize::from(button.icon_right.is_some());
            let parts = icons + usize::from(label.is_some());
            BORDER * 2.
                + metrics.padding_x * 2.
                + label_width
                + metrics.icon * icons as f32
                + metrics.gap * parts.saturating_sub(1) as f32
        };
        Self {
            metrics,
            label,
            label_width,
            width,
        }
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let content = Content::new(&self, &theme, window);
        let look = Look::of(self.variant, &theme);
        let reduce_motion = cx.reduce_motion();

        let shared = window
            .use_keyed_state(
                ElementId::NamedChild(self.id.clone().into(), "riff-button".into()),
                cx,
                |_, cx| Shared::new(cx.focus_handle()),
            )
            .read(cx)
            .clone();
        let focus = shared.focus();

        let spinning = self.loading && self.icon.is_some();
        let keys = [
            self.label.clone(),
            self.icon.as_ref().map(|icon| {
                if spinning {
                    SPINNER_KEY.into()
                } else {
                    icon.0.clone()
                }
            }),
            self.icon_right.as_ref().map(|icon| icon.0.clone()),
        ];
        let now = cx.background_executor().now();
        let frame = shared.update(keys, content.width, self.loading, now);

        // The width glides when the content changes, unless motion is reduced.
        let width_policy = if frame.snap_width {
            Transition::new(Duration::ZERO)
        } else {
            Transition::new(MORPH_DURATION).easing(morph_easing())
        };
        let width = shared
            .width()
            .sample(content.width, width_policy, window, cx)
            .value;

        let interactive = !self.disabled && !self.loading;
        let hovered = !self.disabled && (shared.hovered() || self.forced.hovered);
        let pressed = interactive && (shared.pressed() || self.forced.pressed);
        let focus_visible = self.forced.focus_visible
            || (focus.is_focused(window) && window.last_input_was_keyboard());

        let state_id = |channel: &'static str| (self.id.clone(), channel);
        let ease = Transition::new(STATE_DURATION).easing(Easing::Ease);
        let hover = transition(
            state_id("riff-hover"),
            if hovered { 1. } else { 0. },
            ease.clone(),
            window,
            cx,
        );
        let scale = transition(
            state_id("riff-scale"),
            if pressed { PRESSED_SCALE } else { 1. },
            Transition::new(STATE_DURATION).easing(theme.ease.clone()),
            window,
            cx,
        );
        let opacity = transition(
            state_id("riff-opacity"),
            if self.disabled { DISABLED_OPACITY } else { 1. },
            ease,
            window,
            cx,
        );

        let mut frame = frame;
        if let Some(at) = self.frozen_at {
            frame.part_elapsed = [None; 3];
            frame.loading_for = frame.loading_for.map(|_| at);
        }
        let mut motion = Motion::new(&frame, reduce_motion);
        motion.running &= self.frozen_at.is_none();
        if motion.running {
            window.request_animation_frame();
        }
        motion.spinning = spinning;

        let painter = Painter {
            content,
            look,
            hover,
            scale,
            focus_visible,
            faded: opacity < 1.,
            loading: self.loading,
            icon: self.icon.clone(),
            icon_right: self.icon_right.clone(),
            motion,
        };

        let on_click = self.on_click.clone();
        let loading = self.loading;
        let hover_state = shared.clone();
        let press_state = shared.clone();
        let release_state = shared.clone();

        let button = base::Button::new(self.id.clone())
            .track_focus(&focus)
            .disabled(self.disabled)
            .block_mouse_except_scroll()
            .flex_none()
            .w(width)
            .h(painter.content.metrics.height)
            .opacity(opacity)
            .cursor(if interactive {
                CursorStyle::PointingHand
            } else {
                CursorStyle::Arrow
            })
            .on_hover(move |hovered, window, _| {
                hover_state.set_hovered(*hovered);
                window.refresh();
            })
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                press_state.set_pressed(true);
                window.refresh();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window, cx| {
                        // Releasing anywhere ends the press, as `:active` does.
                        window.on_mouse_event(move |_: &MouseUpEvent, _, window, _| {
                            if release_state.set_pressed(false) {
                                window.refresh();
                            }
                        });
                        painter.paint(bounds, window, cx);
                    },
                )
                .size_full(),
            );
        let button = match self.accessibility_label.or(self.label) {
            Some(label) => button.accessibility_label(label),
            None => button,
        };
        // riff-uiux swallows the click while loading instead of disabling.
        let button = if loading {
            button.on_click(|_, _, _| {})
        } else if let Some(on_click) = on_click {
            button.on_click(move |event, window, cx| on_click(event, window, cx))
        } else {
            button
        };

        // Render the base button here so the busy flag lands on the node its
        // root writes; the Component wrapper writes none of its own.
        a11y::Busy::new(button.render(window, cx).into_element(), self.loading)
    }
}

/// Where each time-driven effect is in this frame.
struct Motion {
    /// Enter progress of the label and the icons, 0 to 1 after easing.
    label_in: f32,
    icon_in: f32,
    icon_right_in: f32,
    /// Position of the loading sweep, 0 to 1, if it runs.
    sweep: Option<f32>,
    /// Spinner rotation in radians.
    spin: f32,
    spinning: bool,
    running: bool,
}

impl Motion {
    fn new(frame: &state::Frame, reduce_motion: bool) -> Self {
        let curve = morph_easing();
        let enter = |elapsed: Option<Duration>, duration: Duration| match elapsed {
            Some(elapsed) if !reduce_motion && elapsed < duration => {
                Some(curve.sample(elapsed.as_secs_f32() / duration.as_secs_f32()))
            }
            _ => None,
        };
        let label_in = enter(frame.part_elapsed[LABEL], MORPH_DURATION);
        let icon_in = enter(frame.part_elapsed[ICON], ICON_IN_DURATION);
        let icon_right_in = enter(frame.part_elapsed[ICON_RIGHT], ICON_IN_DURATION);
        let entering = label_in.is_some() || icon_in.is_some() || icon_right_in.is_some();

        // Under reduced motion the sweep stops, but riff-uiux's spinner keeps
        // turning: its media query only names the sweep and the transitions.
        let fraction = |elapsed: Duration, period: Duration| {
            (elapsed.as_secs_f64() % period.as_secs_f64() / period.as_secs_f64()) as f32
        };
        let sweep = frame
            .loading_for
            .filter(|_| !reduce_motion)
            .map(|elapsed| fraction(elapsed, SWEEP_DURATION));
        let spin = frame.loading_for.map_or(0., |elapsed| {
            fraction(elapsed, SPIN_DURATION) * std::f32::consts::TAU
        });

        Self {
            label_in: label_in.unwrap_or(1.),
            icon_in: icon_in.unwrap_or(1.),
            icon_right_in: icon_right_in.unwrap_or(1.),
            sweep,
            spin,
            spinning: false,
            running: entering || frame.loading_for.is_some(),
        }
    }
}

struct Painter {
    content: Content,
    look: Look,
    hover: f32,
    scale: f32,
    focus_visible: bool,
    faded: bool,
    loading: bool,
    icon: Option<ButtonIcon>,
    icon_right: Option<ButtonIcon>,
    motion: Motion,
}

impl Painter {
    fn paint(self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let m = self.content.metrics;
        let scale = Scale {
            center: bounds.center(),
            factor: self.scale,
        };
        #[cfg(test)]
        tests::LAST_SCALE.with(|last| last.set(self.scale));
        let look = &self.look;
        let fill = lerp_fill(look.fill, look.fill_hover, self.hover);
        let text = mix(look.text, look.text_hover, self.hover);

        paint::paint_box(
            bounds,
            m.radius,
            BORDER,
            fill,
            look.border,
            &look.shadows,
            scale,
            self.faded,
            window,
        );

        // `overflow: clip` on the button: content never paints past its box.
        let clip = gpui_kit::ContentMask {
            bounds: scale.bounds(bounds),
        };
        window.with_content_mask(Some(clip), |window| {
            self.paint_content(bounds, text, scale, window, cx);
        });

        if self.focus_visible {
            paint::paint_focus_ring(bounds, m.radius, look.focus, scale, window);
        }
    }

    fn paint_content(
        &self,
        bounds: Bounds<Pixels>,
        text: Hsla,
        scale: Scale,
        window: &mut Window,
        cx: &mut App,
    ) {
        let m = self.content.metrics;
        let label_width = self.content.label_width;
        let parts_width = {
            let icons = usize::from(self.icon.is_some()) + usize::from(self.icon_right.is_some());
            let parts = icons + usize::from(self.content.label.is_some());
            label_width + m.icon * icons as f32 + m.gap * parts.saturating_sub(1) as f32
        };
        // Centered, as `justify-content: center` does when the box is wider
        // or narrower (mid-glide) than its content.
        let mut x = bounds.center().x - parts_width / 2.;
        let middle = bounds.center().y;

        if let Some(icon) = &self.icon {
            let slot = Bounds::new(point(x, middle - m.icon / 2.), size(m.icon, m.icon));
            self.paint_icon_slot(
                slot,
                icon,
                self.motion.spinning,
                self.motion.icon_in,
                text,
                scale,
                window,
                cx,
            );
            x += m.icon + m.gap;
        }

        if let Some(line) = &self.content.label {
            self.paint_label(line, x, middle, text, scale, window);
            x += label_width + m.gap;
        }

        if let Some(icon) = &self.icon_right {
            let slot = Bounds::new(point(x, middle - m.icon / 2.), size(m.icon, m.icon));
            self.paint_icon_slot(
                slot,
                icon,
                false,
                self.motion.icon_right_in,
                text,
                scale,
                window,
                cx,
            );
        }
    }

    /// `ui-button-icon-in`: from `opacity: 0; filter: blur(2px);
    /// transform: scale(0.7)`. The spinner turns about the slot center.
    #[allow(clippy::too_many_arguments)]
    fn paint_icon_slot(
        &self,
        slot: Bounds<Pixels>,
        icon: &ButtonIcon,
        spinner: bool,
        t: f32,
        color: Hsla,
        scale: Scale,
        window: &mut Window,
        cx: &mut App,
    ) {
        let own_scale = 0.7 + 0.3 * t;
        let blur = scale.len(px(2. * (1. - t)));
        let taps = paint::blur_taps(blur);
        let color = color.opacity(t);
        let (path, data, rotation) = if spinner {
            (SPINNER_KEY.into(), Some(SPINNER_SVG), self.motion.spin)
        } else {
            (icon.0.clone(), None, 0.)
        };
        paint::paint_icon(
            &path, data, slot, color, scale, own_scale, rotation, &taps, window, cx,
        );
    }

    /// `ui-button-label-in`: from `opacity: 0; filter: blur(3px);
    /// transform: translateY(25%)`, plus the loading dim and sweep.
    fn paint_label(
        &self,
        line: &ShapedLine,
        x: Pixels,
        middle: Pixels,
        color: Hsla,
        scale: Scale,
        window: &mut Window,
    ) {
        let m = self.content.metrics;
        let t = self.motion.label_in;
        // `line-height: normal` box, centered; translateY(25%) of its height.
        // Browsers round ascent and descent to whole pixels each, and the
        // line box is their sum (Blink's SimpleFontData): Inter at 14px is
        // 13.56 + 3.38, laid out as 14 + 3. Unrounded, the label sat ~0.5px
        // above riff-uiux's.
        let ascent = line.ascent.round();
        let descent = line.descent.round();
        let box_height = ascent + descent;
        let shift = box_height * 0.25 * (1. - t);
        let baseline = middle - box_height / 2. + ascent + shift;
        // A label centered in an odd leftover space has its baseline on a
        // half pixel. GPUI snaps glyphs with round-half-down, browsers with
        // round-half-up, so at rest the label drew a pixel above riff-uiux's.
        // Snap it here the browser's way, in device pixels; while it moves
        // (enter animation, press scale) it stays continuous.
        let baseline = if t >= 1. && scale.factor == 1. {
            let sf = window.scale_factor();
            px((baseline.as_f32() * sf + 0.5).floor() / sf)
        } else {
            baseline
        };
        let origin = point(x, baseline);

        let color = if self.loading {
            color.opacity(LOADING_LABEL_ALPHA)
        } else {
            color
        };
        let taps: Vec<Tap> = paint::blur_taps(scale.len(px(3. * (1. - t))));
        paint::paint_text(
            line,
            origin,
            m.font_size,
            color.opacity(t),
            scale,
            &taps,
            window,
        );

        if self.loading
            && let Some(sweep) = self.motion.sweep
        {
            // The gradient image is 300% of the label width W, its band
            // centered at 50% and ±12% wide: center 1.5W, half-width 0.36W.
            // `background-position` 100% → 0% offsets it by -2W → 0, so the
            // band's center runs from -0.5W to 1.5W, left to right.
            let w = self.content.label_width;
            let center = x + w * (2. * sweep - 0.5);
            let half_width = w * 0.36;
            let line_bounds = Bounds::new(point(x, baseline - ascent), size(w, box_height));
            paint::paint_sweep(
                line,
                origin,
                line_bounds,
                m.font_size,
                self.look.shine.opacity(t),
                center,
                half_width,
                scale,
                window,
            );
        }
    }
}
