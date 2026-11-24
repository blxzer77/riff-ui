//! riff-uiux `IconButton`: a square button that is only an icon — ghost,
//! secondary or primary, two sizes, a toggle `pressed` state and its label as
//! the tooltip.
//!
//! gpui-base supplies the behavior ([`base::Button`] for plain buttons,
//! [`base::Toggle`] when [`IconButton::pressed`] is set) and this module paints
//! the box, icon, focus ring, and 95% press scale via [`crate::paint`].

mod state;
mod style;
#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{self, Easing, Transition, transition};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    A11ySubtreeBuilder, App, Bounds, BoxShadow, ClickEvent, CursorStyle, Element, ElementId,
    FocusHandle, GlobalElementId, Hsla, InspectorElementId, InteractiveElement, IntoElement,
    LayoutId, MouseButton, MouseUpEvent, ParentElement, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, accesskit, canvas, point, size,
};

pub use crate::button::{ButtonIcon as IconButtonIcon, ButtonState as IconButtonState};
use crate::paint::{self, Fill, Scale, Tap};
use crate::theme::RiffTheme;
use state::Shared;
use style::{BORDER, Look, Metrics};

/// `variant`: what the icon button looks like.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum IconButtonVariant {
    /// Just the icon; a soft gray wash appears on hover.
    #[default]
    Ghost,
    /// White with a 1px card outline.
    Secondary,
    /// Dark gradient with a 1px ring, like the primary Button.
    Primary,
}

/// `size`: 32 × 32 with a 16px icon, or 40 × 40 with an 18px icon.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum IconButtonSize {
    Sm,
    #[default]
    Md,
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// A riff-uiux icon button.
///
/// ```ignore
/// IconButton::new("filter")
///     .secondary()
///     .icon(IconName::SlidersHorizontal)
///     .label("Filter");
/// IconButton::new("pin")
///     .secondary()
///     .icon(IconName::Pin)
///     .label("Pinned runs only")
///     .pressed(pinned)
///     .on_click(|_, _, _| toggle());
/// ```
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    variant: IconButtonVariant,
    size: IconButtonSize,
    icon: Option<IconButtonIcon>,
    label: Option<SharedString>,
    title: Option<SharedString>,
    pressed: Option<bool>,
    disabled: bool,
    on_click: Option<ClickHandler>,
    on_change: Option<ChangeHandler>,
    forced: IconButtonState,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            variant: IconButtonVariant::default(),
            size: IconButtonSize::default(),
            icon: None,
            label: None,
            title: None,
            pressed: None,
            disabled: false,
            on_click: None,
            on_change: None,
            forced: IconButtonState::default(),
        }
    }

    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn ghost(self) -> Self {
        self.variant(IconButtonVariant::Ghost)
    }

    pub fn secondary(self) -> Self {
        self.variant(IconButtonVariant::Secondary)
    }

    pub fn primary(self) -> Self {
        self.variant(IconButtonVariant::Primary)
    }

    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(self) -> Self {
        self.size(IconButtonSize::Sm)
    }

    pub fn medium(self) -> Self {
        self.size(IconButtonSize::Md)
    }

    pub fn icon(mut self, icon: impl Into<IconButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Required accessible name (`aria-label`) and default tooltip (`title`).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Tooltip text; defaults to `label`. Pass `""` to omit the tooltip.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Makes this button a toggle: sets `aria-pressed` and keeps the pressed
    /// look while `true`. Leave unset for a plain push button.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Handles toggle changes when [`Self::pressed`] is set, receiving the
    /// requested next state (`!pressed`).
    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Shows interactive states regardless of input, for galleries and
    /// screenshots. Live input still applies on top.
    pub fn force_state(mut self, state: IconButtonState) -> Self {
        self.forced = state;
        self
    }

    fn resolved_tooltip(&self) -> Option<SharedString> {
        match &self.title {
            Some(title) if title.is_empty() => None,
            Some(title) => Some(title.clone()),
            None => self.label.as_ref().filter(|l| !l.is_empty()).cloned(),
        }
    }
}

/// `transition: ... 160ms ease`, and `transform 160ms var(--ui-icb-ease)`.
const STATE_DURATION: Duration = Duration::from_millis(160);
/// `scale(0.95)` while `:active:not(:disabled)`.
const PRESSED_SCALE: f32 = 0.95;
/// `opacity: 0.4` while disabled.
const DISABLED_OPACITY: f32 = 0.4;

impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let metrics = Metrics::of(self.size, &theme);
        let look = Look::of(self.variant, &theme);
        let reduce_motion = cx.reduce_motion();

        let shared = window
            .use_keyed_state(
                ElementId::NamedChild(self.id.clone().into(), "riff-icon-button".into()),
                cx,
                |_, cx| Shared::new(cx.focus_handle()),
            )
            .read(cx)
            .clone();
        let focus = shared.focus();

        let hovered = !self.disabled && (shared.hovered() || self.forced.hovered);
        let active = !self.disabled && !reduce_motion && (shared.pressed() || self.forced.pressed);
        let toggled = self.pressed == Some(true);
        let focus_visible = self.forced.focus_visible
            || (focus.is_focused(window) && window.last_input_was_keyboard());

        let state_id = |channel: &'static str| (self.id.clone(), channel);
        let ease = Transition::new(STATE_DURATION).easing(Easing::Ease);
        let hover = transition(
            state_id("riff-icb-hover"),
            if hovered { 1. } else { 0. },
            ease.clone(),
            window,
            cx,
        );
        let toggled_t = transition(
            state_id("riff-icb-toggled"),
            if toggled { 1. } else { 0. },
            ease.clone(),
            window,
            cx,
        );
        let scale = transition(
            state_id("riff-icb-scale"),
            if active { PRESSED_SCALE } else { 1. },
            Transition::new(STATE_DURATION).easing(theme.ease.clone()),
            window,
            cx,
        );
        let opacity = transition(
            state_id("riff-icb-opacity"),
            if self.disabled { DISABLED_OPACITY } else { 1. },
            ease,
            window,
            cx,
        );

        let (fill, icon_color, shadows) = look.resolve(toggled_t, hover);
        let painter = Painter {
            metrics,
            fill,
            border: look.border,
            shadows,
            icon_color,
            focus_color: look.focus,
            scale,
            focus_visible,
            faded: opacity < 1.,
            icon: self.icon.clone(),
        };

        let tooltip = self.resolved_tooltip();
        let label = self.label;
        let disabled = self.disabled;
        let hover_state = shared.clone();
        let press_state = shared.clone();
        let release_state = shared.clone();

        let apply_common = |el: AnyHost| {
            let mut el = el
                .track_focus(&focus)
                .disabled(disabled)
                .flex_none()
                .w(metrics.side)
                .h(metrics.side)
                .opacity(opacity)
                .cursor(if disabled {
                    CursorStyle::Arrow
                } else {
                    CursorStyle::PointingHand
                })
                .on_hover(move |hovered, window, _| {
                    hover_state.set_hovered(*hovered);
                    window.refresh();
                })
                .on_mouse_down(MouseButton::Left, move |_, window, _| {
                    if !disabled {
                        press_state.set_pressed(true);
                        window.refresh();
                    }
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
            if let Some(label) = label {
                el = el.accessibility_label(label);
            }
            if let Some(tip) = tooltip {
                el = el.tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx));
            }
            el
        };

        match self.pressed {
            Some(pressed) => {
                let mut toggle = base::Toggle::new(self.id).pressed(pressed);
                let on_change = self.on_change;
                let on_click = self.on_click;
                if on_change.is_some() || on_click.is_some() {
                    toggle = toggle.on_change(move |next, event, window, cx| {
                        if let Some(handler) = &on_change {
                            handler(next, event, window, cx);
                        }
                        if let Some(handler) = &on_click {
                            handler(event, window, cx);
                        }
                    });
                }
                match apply_common(AnyHost::Toggle(Box::new(toggle))) {
                    AnyHost::Toggle(t) => Either::Right(t.render(window, cx).into_element()),
                    AnyHost::Button(_) => unreachable!(),
                }
            }
            None => {
                let mut button = base::Button::new(self.id);
                if let Some(on_click) = self.on_click {
                    button = button.on_click(move |event, window, cx| on_click(event, window, cx));
                }
                match apply_common(AnyHost::Button(Box::new(button))) {
                    AnyHost::Button(b) => Either::Left(b.render(window, cx).into_element()),
                    AnyHost::Toggle(_) => unreachable!(),
                }
            }
        }
    }
}

enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A: Element, B: Element> IntoElement for Either<A, B> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

enum EitherLayout<A, B> {
    Left(A),
    Right(B),
}

enum EitherPrepaint<A, B> {
    Left(A),
    Right(B),
}

impl<A: Element, B: Element> Element for Either<A, B> {
    type RequestLayoutState = EitherLayout<A::RequestLayoutState, B::RequestLayoutState>;
    type PrepaintState = EitherPrepaint<A::PrepaintState, B::PrepaintState>;

    fn id(&self) -> Option<ElementId> {
        match self {
            Self::Left(a) => a.id(),
            Self::Right(b) => b.id(),
        }
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        match self {
            Self::Left(a) => a.source_location(),
            Self::Right(b) => b.source_location(),
        }
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        match self {
            Self::Left(a) => {
                let (layout, state) = a.request_layout(id, inspector, window, cx);
                (layout, EitherLayout::Left(state))
            }
            Self::Right(b) => {
                let (layout, state) = b.request_layout(id, inspector, window, cx);
                (layout, EitherLayout::Right(state))
            }
        }
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
        match (self, layout) {
            (Self::Left(a), EitherLayout::Left(l)) => {
                EitherPrepaint::Left(a.prepaint(id, inspector, bounds, l, window, cx))
            }
            (Self::Right(b), EitherLayout::Right(l)) => {
                EitherPrepaint::Right(b.prepaint(id, inspector, bounds, l, window, cx))
            }
            _ => unreachable!(),
        }
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
        match (self, layout, prepaint) {
            (Self::Left(a), EitherLayout::Left(l), EitherPrepaint::Left(p)) => {
                a.paint(id, inspector, bounds, l, p, window, cx)
            }
            (Self::Right(b), EitherLayout::Right(l), EitherPrepaint::Right(p)) => {
                b.paint(id, inspector, bounds, l, p, window, cx)
            }
            _ => unreachable!(),
        }
    }

    fn a11y_role(&self) -> Option<Role> {
        match self {
            Self::Left(a) => a.a11y_role(),
            Self::Right(b) => b.a11y_role(),
        }
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        match self {
            Self::Left(a) => a.write_a11y_info(node),
            Self::Right(b) => b.write_a11y_info(node),
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        match (self, prepaint) {
            (Self::Left(a), EitherPrepaint::Left(p)) => a.a11y_synthetic_children(p, builder),
            (Self::Right(b), EitherPrepaint::Right(p)) => b.a11y_synthetic_children(p, builder),
            _ => unreachable!(),
        }
    }
}

/// Abstracts over [`base::Button`] and [`base::Toggle`] so shared styling,
/// pointer tracking, tooltips, and painting live in one place.
enum AnyHost {
    Button(Box<base::Button>),
    Toggle(Box<base::Toggle>),
}

impl AnyHost {
    fn track_focus(self, focus_handle: &FocusHandle) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.track_focus(focus_handle))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.track_focus(focus_handle))),
        }
    }

    fn disabled(self, disabled: bool) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.disabled(disabled))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.disabled(disabled))),
        }
    }

    fn accessibility_label(self, label: SharedString) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.accessibility_label(label))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.accessibility_label(label))),
        }
    }

    fn flex_none(self) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.flex_none())),
            Self::Toggle(t) => Self::Toggle(Box::new(t.flex_none())),
        }
    }

    fn w(self, width: Pixels) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.w(width))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.w(width))),
        }
    }

    fn h(self, height: Pixels) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.h(height))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.h(height))),
        }
    }

    fn opacity(self, opacity: f32) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.opacity(opacity))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.opacity(opacity))),
        }
    }

    fn cursor(self, cursor: CursorStyle) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.cursor(cursor))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.cursor(cursor))),
        }
    }

    fn on_hover(self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.on_hover(f))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.on_hover(f))),
        }
    }

    fn on_mouse_down(
        self,
        button: MouseButton,
        f: impl Fn(&gpui_kit::MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.on_mouse_down(button, f))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.on_mouse_down(button, f))),
        }
    }

    fn child(self, child: impl IntoElement) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.child(child))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.child(child))),
        }
    }

    fn tooltip(self, build: impl Fn(&mut Window, &mut App) -> gpui_kit::AnyView + 'static) -> Self {
        match self {
            Self::Button(b) => Self::Button(Box::new(b.tooltip(build))),
            Self::Toggle(t) => Self::Toggle(Box::new(t.tooltip(build))),
        }
    }
}

struct Painter {
    metrics: Metrics,
    fill: Fill,
    border: Hsla,
    shadows: Vec<BoxShadow>,
    icon_color: Hsla,
    focus_color: Hsla,
    scale: f32,
    focus_visible: bool,
    faded: bool,
    icon: Option<IconButtonIcon>,
}

impl Painter {
    fn paint(self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let m = self.metrics;
        let scale = Scale {
            center: bounds.center(),
            factor: self.scale,
        };
        #[cfg(test)]
        tests::LAST_SCALE.with(|last| last.set(self.scale));

        paint::paint_box(
            bounds,
            m.radius,
            BORDER,
            self.fill,
            self.border,
            &self.shadows,
            scale,
            self.faded,
            window,
        );

        if let Some(icon) = &self.icon {
            let center = bounds.center();
            let slot = Bounds::new(
                point(center.x - m.icon / 2., center.y - m.icon / 2.),
                size(m.icon, m.icon),
            );
            let taps = [Tap {
                offset: point(gpui_kit::px(0.), gpui_kit::px(0.)),
                weight: 1.,
            }];
            paint::paint_icon(
                &icon.0,
                None,
                slot,
                self.icon_color,
                scale,
                1.,
                0.,
                &taps,
                window,
                cx,
            );
        }

        if self.focus_visible {
            paint::paint_focus_ring(bounds, m.radius, self.focus_color, scale, window);
        }
    }
}
