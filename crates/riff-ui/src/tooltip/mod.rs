//! riff-uiux `Tooltip`: a small inverted label for one control — shows on
//! hover after a short wait and at once under keyboard focus, neighbors follow
//! instantly within 300ms, an optional shortcut chip inside, flips to stay on
//! screen, and stays open while the pointer crosses onto the label.

mod style;
#[cfg(test)]
mod tests;
#[cfg(test)]
use gpui_kit::TestSupportExt as _;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::LayoutDirection;
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Context, Corners, DispatchPhase, Element, ElementId,
    Entity, EntityId, FocusHandle, FontWeight, Global, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement as _, Pixels, Point, RenderOnce, Role,
    ShapedLine, SharedString, StatefulInteractiveElement as _, Styled, Subscription, Task, TextRun,
    WeakEntity, Window, WindowId, accesskit, canvas, div, fill, font, point,
    prelude::FluentBuilder as _, px, size,
};

use crate::overlay::{self, OverlayAlign, OverlaySide};
use crate::paint;
use crate::theme::RiffTheme;
use style::{
    DELAY_DEFAULT, DRIFT, ENTER_DURATION, FONT_SIZE, GAP, KEY_FONT_SIZE, KEY_LINE_HEIGHT,
    KEY_MARGIN_RIGHT, KEY_PAD_X, KEY_RADIUS, KEY_TRACKING, LEAVE_GRACE, LINE_HEIGHT, Look,
    MAX_WIDTH, OFFSET_DEFAULT, PAD_X, PAD_Y, SKIP_WINDOW, VIEWPORT_MARGIN,
};

pub type TooltipSide = OverlaySide;
pub type TooltipAlign = OverlayAlign;

/// Interactive states a gallery can pin, see [`Tooltip::force_state`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TooltipState {
    pub open: bool,
}

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// Window-scoped tooltip coordination: only one interactive tooltip stays open
/// at a time, and closing opens a 300ms warm window so neighboring controls
/// appear instantly without delay or enter fade.
#[derive(Default)]
struct TooltipGroups {
    windows: HashMap<WindowId, WindowGroup>,
    interceptor: Option<Subscription>,
}

impl Global for TooltipGroups {}

#[derive(Default)]
struct WindowGroup {
    active: Option<(EntityId, WeakEntity<TooltipController>)>,
    warm: bool,
    warm_epoch: u64,
    warm_task: Option<Task<()>>,
}

impl TooltipGroups {
    fn ensure_interceptor(cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_some() {
            return;
        }
        let sub = cx.intercept_keystrokes(|event, window, cx| {
            if event.keystroke.key != "escape" || event.keystroke.modifiers != Modifiers::default()
            {
                return;
            }
            let window_id = window.window_handle().window_id();
            let active = cx
                .try_global::<Self>()
                .and_then(|groups| groups.windows.get(&window_id))
                .and_then(|group| group.active.as_ref())
                .and_then(|(_, weak)| weak.upgrade());
            if let Some(controller) = active
                && controller.read(cx).effective_open()
            {
                window.prevent_default();
                cx.stop_propagation();
                controller.update(cx, |state, cx| {
                    state.blocked = true;
                    state.hide(false, window, cx);
                });
            }
        });
        cx.default_global::<Self>().interceptor = Some(sub);
    }

    fn is_warm(window: &Window, self_id: EntityId, cx: &App) -> bool {
        let Some(groups) = cx.try_global::<Self>() else {
            return false;
        };
        let Some(group) = groups.windows.get(&window.window_handle().window_id()) else {
            return false;
        };
        group.warm
            || group
                .active
                .as_ref()
                .is_some_and(|(id, weak)| *id != self_id && weak.upgrade().is_some())
    }

    fn activate(controller: &Entity<TooltipController>, window: &mut Window, cx: &mut App) {
        Self::ensure_interceptor(cx);
        let window_id = window.window_handle().window_id();
        let self_id = controller.entity_id();
        let prev = {
            let groups = cx.default_global::<Self>();
            let group = groups.windows.entry(window_id).or_default();
            group.warm_task = None;
            group.warm = false;
            let prev = group
                .active
                .as_ref()
                .filter(|(id, _)| *id != self_id)
                .and_then(|(_, weak)| weak.upgrade());
            group.active = Some((self_id, controller.downgrade()));
            prev
        };
        if let Some(prev) = prev {
            prev.update(cx, |state, cx| {
                state.close_from_group(window, cx);
            });
        }
    }

    fn deactivate(controller_id: EntityId, window: &mut Window, cx: &mut App) {
        let window_id = window.window_handle().window_id();
        let epoch = {
            let groups = cx.default_global::<Self>();
            let group = groups.windows.entry(window_id).or_default();
            let Some((active_id, _)) = &group.active else {
                return;
            };
            if *active_id != controller_id {
                return;
            }
            group.active = None;
            group.warm = true;
            group.warm_epoch = group.warm_epoch.wrapping_add(1);
            group.warm_epoch
        };
        let task = window.spawn(cx, async move |async_window| {
            async_window.background_executor().timer(SKIP_WINDOW).await;
            _ = async_window.update(|_, cx| {
                if cx.has_global::<TooltipGroups>()
                    && let Some(group) =
                        cx.global_mut::<TooltipGroups>().windows.get_mut(&window_id)
                    && group.warm_epoch == epoch
                {
                    group.warm = false;
                    group.warm_task = None;
                }
            });
        });
        cx.default_global::<Self>()
            .windows
            .entry(window_id)
            .or_default()
            .warm_task = Some(task);
    }
}

struct ControllerProps {
    disabled: bool,
    empty: bool,
    delay: Duration,
    controlled_open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    inline: bool,
}

impl Default for ControllerProps {
    fn default() -> Self {
        Self {
            disabled: false,
            empty: true,
            delay: DELAY_DEFAULT,
            controlled_open: None,
            on_open_change: None,
            inline: false,
        }
    }
}

struct TooltipController {
    focus_handle: FocusHandle,
    props: ControllerProps,
    shown: bool,
    instant: bool,
    blocked: bool,
    trigger_hovered: bool,
    popup_hovered: bool,
    timer_epoch: u64,
    timer: Option<Task<()>>,
    was_open: bool,
    enter_started: Option<Instant>,
    _subscriptions: Vec<Subscription>,
}

impl TooltipController {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle().tab_stop(false);
        let weak = cx.entity().downgrade();
        let focus_in = window.on_focus_in(&focus_handle, cx, {
            let weak = weak.clone();
            move |window, cx| {
                let visible = window.last_input_was_keyboard();
                _ = weak.update(cx, |this, cx| {
                    if visible {
                        this.show(true, window, cx);
                    }
                });
            }
        });
        let focus_out = window.on_focus_out(&focus_handle, cx, {
            let weak = weak.clone();
            move |_, window, cx| {
                _ = weak.update(cx, |this, cx| {
                    this.blocked = false;
                    this.hide(false, window, cx);
                });
            }
        });

        Self {
            focus_handle,
            props: ControllerProps::default(),
            shown: false,
            instant: false,
            blocked: false,
            trigger_hovered: false,
            popup_hovered: false,
            timer_epoch: 0,
            timer: None,
            was_open: false,
            enter_started: None,
            _subscriptions: vec![focus_in, focus_out],
        }
    }

    fn effective_open(&self) -> bool {
        !self.props.disabled
            && !self.props.empty
            && self.props.controlled_open.unwrap_or(self.shown)
    }

    fn cancel_timer(&mut self) {
        self.timer_epoch = self.timer_epoch.wrapping_add(1);
        self.timer = None;
    }

    fn apply_open(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        let prev = self.effective_open();
        if self.props.controlled_open.is_none() && self.shown != next {
            self.shown = next;
            cx.notify();
            window.refresh();
        }
        let now_open = self.effective_open();
        if !self.props.inline {
            let entity = cx.entity();
            if now_open && !prev {
                self.enter_started = if self.instant || cx.reduce_motion() {
                    None
                } else {
                    Some(Instant::now())
                };
                self.was_open = true;
                TooltipGroups::activate(&entity, window, cx);
            } else if !now_open && prev {
                self.enter_started = None;
                self.instant = false;
                self.was_open = false;
                TooltipGroups::deactivate(entity.entity_id(), window, cx);
            }
        }
        if now_open != prev
            && let Some(on_change) = self.props.on_open_change.clone()
        {
            on_change(now_open, window, cx);
        }
    }

    fn close_from_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_open(false, window, cx);
    }

    fn show(&mut self, now: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        if self.props.disabled || self.props.empty || self.blocked {
            return;
        }
        let warm = TooltipGroups::is_warm(window, cx.entity_id(), cx);
        if now || warm || self.props.delay.is_zero() {
            self.instant = warm;
            self.apply_open(true, window, cx);
        } else {
            let epoch = self.timer_epoch;
            let delay = self.props.delay;
            let weak = cx.entity().downgrade();
            self.timer = Some(window.spawn(cx, async move |async_window| {
                async_window.background_executor().timer(delay).await;
                _ = async_window.update(|window, cx| {
                    _ = weak.update(cx, |this, cx| {
                        if this.timer_epoch == epoch {
                            this.instant = false;
                            this.apply_open(true, window, cx);
                        }
                    });
                });
            }));
        }
    }

    fn hide(&mut self, with_grace: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        if !with_grace {
            self.apply_open(false, window, cx);
            return;
        }
        let epoch = self.timer_epoch;
        let weak = cx.entity().downgrade();
        self.timer = Some(window.spawn(cx, async move |async_window| {
            async_window.background_executor().timer(LEAVE_GRACE).await;
            _ = async_window.update(|window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.timer_epoch == epoch && !this.trigger_hovered && !this.popup_hovered {
                        this.apply_open(false, window, cx);
                    }
                });
            });
        }));
    }

    fn sync_frame(
        &mut self,
        controller: &Entity<Self>,
        props: ControllerProps,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.props = props;
        let open = self.effective_open();
        let reduce_motion = cx.reduce_motion();

        if !self.props.inline {
            if open && !self.was_open {
                TooltipGroups::activate(controller, window, cx);
            } else if !open && self.was_open {
                TooltipGroups::deactivate(controller.entity_id(), window, cx);
            }
        }

        if open && !self.was_open {
            if self.props.inline || self.instant || reduce_motion {
                self.enter_started = None;
            } else {
                self.enter_started = Some(Instant::now());
            }
        } else if !open {
            self.enter_started = None;
            self.instant = false;
        }

        self.was_open = open;
        open
    }

    fn enter_progress(&self, freeze_at: Option<Duration>, window: &mut Window, cx: &App) -> f32 {
        if self.props.inline || self.instant || cx.reduce_motion() {
            return 1.;
        }
        let Some(started) = self.enter_started else {
            return 1.;
        };
        let elapsed = freeze_at.unwrap_or_else(|| started.elapsed());
        if elapsed >= ENTER_DURATION {
            return 1.;
        }
        window.request_animation_frame();
        RiffTheme::global(cx)
            .ease
            .sample((elapsed.as_secs_f32() / ENTER_DURATION.as_secs_f32()).clamp(0., 1.))
    }
}

/// A riff-uiux tooltip attached to a single trigger control.
///
/// ```ignore
/// Tooltip::new("save-tip")
///     .content("Save changes")
///     .shortcut("⌘S")
///     .child(Button::new("save").label("Save"))
/// ```
#[derive(IntoElement)]
pub struct Tooltip {
    id: ElementId,
    content: SharedString,
    shortcut: Option<SharedString>,
    side: TooltipSide,
    align: TooltipAlign,
    offset: Pixels,
    delay: Duration,
    disabled: bool,
    open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    child: Option<AnyElement>,
    forced: TooltipState,
    inline: bool,
    freeze_at: Option<Duration>,
}

impl Tooltip {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            content: SharedString::default(),
            shortcut: None,
            side: TooltipSide::Top,
            align: TooltipAlign::Center,
            offset: OFFSET_DEFAULT,
            delay: DELAY_DEFAULT,
            disabled: false,
            open: None,
            on_open_change: None,
            child: None,
            forced: TooltipState::default(),
            inline: false,
            freeze_at: None,
        }
    }

    /// Prepares per-window tooltip state before constructing an `Element` child
    /// whose `render()` borrows `window` and `cx`.
    pub(crate) fn prepare(self, window: &mut Window, cx: &mut App) -> PreparedTooltip {
        let controller = window.use_keyed_state(self.id.clone(), cx, TooltipController::new);
        let empty = self.content.is_empty();
        let controlled_open = self.open.or(self.forced.open.then_some(true));
        let inline = self.inline;

        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                ControllerProps {
                    disabled: self.disabled,
                    empty,
                    delay: self.delay,
                    controlled_open,
                    on_open_change: self.on_open_change.clone(),
                    inline,
                },
                window,
                cx,
            )
        });

        let focus_handle = controller.read(cx).focus_handle.clone();
        let progress = if open {
            controller
                .read(cx)
                .enter_progress(self.freeze_at, window, cx)
        } else {
            0.
        };

        let popup = open.then(|| {
            let cfg = TooltipPopupConfig {
                id: self.id.clone(),
                content: self.content,
                shortcut: self.shortcut,
                side: self.side,
                align: self.align,
                offset: self.offset,
                progress,
                inline,
            };
            (cfg, controller.clone())
        });

        PreparedTooltip {
            id: self.id,
            controller,
            focus_handle,
            interactive: !self.disabled && !empty && !self.inline,
            popup,
        }
    }

    fn build_element<C: TooltipChild>(
        self,
        child: C,
        window: &mut Window,
        cx: &mut App,
    ) -> TooltipElement<C> {
        self.prepare(window, cx).wrap_child(child)
    }
}

pub(crate) struct PreparedTooltip {
    id: ElementId,
    controller: Entity<TooltipController>,
    focus_handle: FocusHandle,
    interactive: bool,
    popup: Option<(TooltipPopupConfig, Entity<TooltipController>)>,
}

impl PreparedTooltip {
    pub(crate) fn wrap_element<E: Element>(self, child: E) -> TooltipElement<ConcreteChild<E>> {
        self.wrap_child(ConcreteChild(child))
    }

    fn wrap_child<C: TooltipChild>(self, child: C) -> TooltipElement<C> {
        TooltipElement {
            id: self.id,
            child,
            controller: self.controller,
            focus_handle: self.focus_handle,
            interactive: self.interactive,
            popup: self.popup,
            deferred_popup: None,
            bounds_slot: Rc::new(Cell::new(Bounds::default())),
        }
    }
}

impl Tooltip {
    pub fn content(mut self, content: impl Into<SharedString>) -> Self {
        self.content = content.into();
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    pub fn side(mut self, side: TooltipSide) -> Self {
        self.side = side;
        self
    }

    pub fn top(self) -> Self {
        self.side(TooltipSide::Top)
    }

    pub fn bottom(self) -> Self {
        self.side(TooltipSide::Bottom)
    }

    pub fn left(self) -> Self {
        self.side(TooltipSide::Left)
    }

    pub fn right(self) -> Self {
        self.side(TooltipSide::Right)
    }

    pub fn align(mut self, align: TooltipAlign) -> Self {
        self.align = align;
        self
    }

    pub fn start(self) -> Self {
        self.align(TooltipAlign::Start)
    }

    pub fn center(self) -> Self {
        self.align(TooltipAlign::Center)
    }

    pub fn end(self) -> Self {
        self.align(TooltipAlign::End)
    }

    pub fn offset(mut self, offset: impl Into<Pixels>) -> Self {
        self.offset = offset.into();
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn open(mut self, open: impl Into<Option<bool>>) -> Self {
        self.open = open.into();
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Forces the tooltip open state for static gallery comparison or tests.
    pub fn force_state(mut self, state: TooltipState) -> Self {
        self.forced = state;
        self
    }

    /// Freezes enter motion `at` a fixed offset for snapshot verification.
    pub fn freeze_motion_at(mut self, at: Duration) -> Self {
        self.freeze_at = Some(at);
        self
    }

    /// Renders the popup inline in the local paint tree rather than as a
    /// deferred window overlay. Used by `riff-story`'s Compare tab so scroll
    /// clipping and 50%-opacity overlay alignment work as in other stories.
    #[doc(hidden)]
    pub fn inline_preview(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl RenderOnce for Tooltip {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(child) = self.child.take() else {
            return gpui_kit::Empty.into_any_element();
        };
        self.build_element(child, window, cx).into_any_element()
    }
}

pub(crate) trait TooltipChild: 'static {
    type LayoutState: 'static;
    type PrepaintState: 'static;

    fn child_id(&self) -> Option<ElementId>;
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>>;
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::LayoutState);
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::LayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState;
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::LayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    );
    fn a11y_role(&self) -> Option<Role>;
    fn write_a11y_info(&self, node: &mut accesskit::Node);
}

impl TooltipChild for AnyElement {
    type LayoutState = ();
    type PrepaintState = ();

    fn child_id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (AnyElement::request_layout(self, window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        AnyElement::prepaint(self, window, cx);
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        AnyElement::paint(self, window, cx);
    }

    fn a11y_role(&self) -> Option<Role> {
        None
    }

    fn write_a11y_info(&self, _node: &mut accesskit::Node) {}
}

pub(crate) struct ConcreteChild<E>(pub(crate) E);

impl<E: Element> TooltipChild for ConcreteChild<E> {
    type LayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn child_id(&self) -> Option<ElementId> {
        self.0.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.0.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, E::RequestLayoutState) {
        self.0.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut E::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> E::PrepaintState {
        self.0.prepaint(id, inspector, bounds, layout, window, cx)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut E::RequestLayoutState,
        prepaint: &mut E::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }

    fn a11y_role(&self) -> Option<Role> {
        self.0.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.0.write_a11y_info(node);
    }
}

#[derive(Clone)]
struct TooltipPopupConfig {
    id: ElementId,
    content: SharedString,
    shortcut: Option<SharedString>,
    side: TooltipSide,
    align: TooltipAlign,
    offset: Pixels,
    progress: f32,
    inline: bool,
}

pub(crate) struct TooltipElement<C: TooltipChild> {
    id: ElementId,
    child: C,
    controller: Entity<TooltipController>,
    focus_handle: FocusHandle,
    interactive: bool,
    popup: Option<(TooltipPopupConfig, Entity<TooltipController>)>,
    deferred_popup: Option<AnyElement>,
    bounds_slot: Rc<Cell<Bounds<Pixels>>>,
}

impl<C: TooltipChild> IntoElement for TooltipElement<C> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

pub(crate) struct TooltipPrepaint<P> {
    child_prepaint: P,
    child_hitbox: Option<Hitbox>,
    inline_popup: Option<AnyElement>,
}

impl<C: TooltipChild> Element for TooltipElement<C> {
    type RequestLayoutState = C::LayoutState;
    type PrepaintState = TooltipPrepaint<C::PrepaintState>;

    fn id(&self) -> Option<ElementId> {
        self.child.child_id().or_else(|| Some(self.id.clone()))
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.child.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, C::LayoutState) {
        let (layout_id, child_layout) = self.child.request_layout(id, inspector, window, cx);
        if let Some((cfg, controller)) = &self.popup
            && !cfg.inline
        {
            let mut anchor = AnchoredTooltip {
                cfg: cfg.clone(),
                trigger_bounds: self.bounds_slot.clone(),
                controller: controller.clone(),
            }
            .into_any_element();
            anchor.request_layout(window, cx);
            self.deferred_popup = Some(anchor);
        }
        (layout_id, child_layout)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut C::LayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> TooltipPrepaint<C::PrepaintState> {
        if self.interactive {
            window.set_focus_handle(&self.focus_handle, cx);
        }
        let child_prepaint = self
            .child
            .prepaint(id, inspector, bounds, layout, window, cx);
        self.bounds_slot.set(bounds);

        let child_hitbox = self
            .interactive
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal));

        let inline_popup = if let Some((cfg, controller)) = self.popup.take() {
            if cfg.inline {
                let mut el = AnchoredTooltip {
                    cfg,
                    trigger_bounds: self.bounds_slot.clone(),
                    controller,
                }
                .into_any_element();
                let available = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
                el.layout_as_root(available, window, cx);
                el.prepaint_at(point(px(0.), px(0.)), window, cx);
                Some(el)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(deferred) = self.deferred_popup.take() {
            window.defer_draw(deferred, point(px(0.), px(0.)), 1001, None);
        }

        TooltipPrepaint {
            child_prepaint,
            child_hitbox,
            inline_popup,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut C::LayoutState,
        prepaint: &mut TooltipPrepaint<C::PrepaintState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(
            id,
            inspector,
            bounds,
            layout,
            &mut prepaint.child_prepaint,
            window,
            cx,
        );

        if let Some(mut inline_popup) = prepaint.inline_popup.take() {
            inline_popup.paint(window, cx);
        }

        if let Some(hitbox) = prepaint.child_hitbox.as_ref() {
            let hitbox_move = hitbox.clone();
            let controller = self.controller.clone();
            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let hovered = !window.last_input_was_touch() && hitbox_move.is_hovered(window);
                let was = controller.read(cx).trigger_hovered;
                if hovered == was {
                    return;
                }
                controller.update(cx, |state, cx| {
                    state.trigger_hovered = hovered;
                    if hovered {
                        state.show(false, window, cx);
                    } else {
                        state.blocked = false;
                        state.hide(true, window, cx);
                    }
                });
            });

            let controller_down = self.controller.clone();
            let hitbox_down = hitbox.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                if hitbox_down.is_hovered(window) || bounds.contains(&event.position) {
                    let _ = MouseButton::Left;
                    controller_down.update(cx, |state, cx| {
                        state.blocked = true;
                        state.hide(false, window, cx);
                    });
                }
            });
        }
    }

    fn a11y_role(&self) -> Option<Role> {
        self.child.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.child.write_a11y_info(node);
    }
}

struct AnchoredTooltip {
    cfg: TooltipPopupConfig,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    controller: Entity<TooltipController>,
}

impl IntoElement for AnchoredTooltip {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct AnchoredPrepaint {
    surface: AnyElement,
    popup_hitbox: Option<Hitbox>,
}

impl Element for AnchoredTooltip {
    type RequestLayoutState = ();
    type PrepaintState = AnchoredPrepaint;

    fn id(&self) -> Option<ElementId> {
        Some((self.cfg.id.clone(), "riff-tip-anchor").into())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout_id = window.request_layout(gpui_kit::Style::default(), [], cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> AnchoredPrepaint {
        let viewport = Bounds::new(point(px(0.), px(0.)), window.viewport_size());
        let max_w = if self.cfg.inline {
            MAX_WIDTH
        } else {
            MAX_WIDTH.min((viewport.size.width - VIEWPORT_MARGIN * 2.).max(px(40.)))
        };

        let mut surface = build_surface(&self.cfg, max_w, window, cx);
        let available = size(AvailableSpace::MinContent, AvailableSpace::MinContent);
        let popup_size = surface.layout_as_root(available, window, cx);

        let place_viewport = if self.cfg.inline {
            Bounds::new(
                point(px(-1_000_000.), px(-1_000_000.)),
                size(px(2_000_000.), px(2_000_000.)),
            )
        } else {
            viewport
        };
        let rtl = LayoutDirection::global(cx).is_rtl();
        let placed = overlay::place_side(
            self.trigger_bounds.get(),
            popup_size,
            place_viewport,
            self.cfg.side,
            self.cfg.align,
            self.cfg.offset,
            VIEWPORT_MARGIN,
            rtl,
        );
        let drift = placed.side.enter_drift(self.cfg.progress, DRIFT);
        let origin = placed.origin + drift;
        surface.prepaint_at(origin, window, cx);

        let popup_hitbox = (!self.cfg.inline)
            .then(|| window.insert_hitbox(Bounds::new(origin, popup_size), HitboxBehavior::Normal));

        AnchoredPrepaint {
            surface,
            popup_hitbox,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut AnchoredPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        prepaint.surface.paint(window, cx);

        if let Some(hitbox) = prepaint.popup_hitbox.as_ref() {
            let hitbox = hitbox.clone();
            let controller = self.controller.clone();
            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let hovered = !window.last_input_was_touch() && hitbox.is_hovered(window);
                let was = controller.read(cx).popup_hovered;
                if hovered == was {
                    return;
                }
                controller.update(cx, |state, cx| {
                    state.popup_hovered = hovered;
                    if hovered {
                        state.cancel_timer();
                    } else if !state.trigger_hovered {
                        state.hide(true, window, cx);
                    }
                });
            });
        }
    }
}

fn build_surface(
    cfg: &TooltipPopupConfig,
    max_w: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = RiffTheme::global(cx).clone();
    let look = Look::of(&theme);
    let alpha = cfg.progress.clamp(0., 1.);
    let spoken = match &cfg.shortcut {
        Some(sc) => SharedString::from(format!("{}, {}", cfg.content, sc)),
        None => cfg.content.clone(),
    };

    let key_chip = cfg.shortcut.clone().map(|sc| {
        let shaped = shape_key_chip(sc, theme.font_family.clone(), window);
        KeyChip {
            shaped,
            text_color: look.muted,
            bg_color: look.key_bg,
        }
    });

    TooltipSurfaceBox {
        id: (cfg.id.clone(), "riff-tip").into(),
        spoken,
        max_w,
        content: cfg.content.clone(),
        font_family: theme.font_family.clone(),
        key_chip,
        look,
        alpha,
    }
    .into_any_element()
}

struct ShapedKeyChip {
    line: ShapedLine,
    width: Pixels,
    outer_width: Pixels,
}

fn shape_key_chip(
    text: SharedString,
    font_family: SharedString,
    window: &mut Window,
) -> ShapedKeyChip {
    let mut key_font = font(font_family);
    key_font.weight = FontWeight::MEDIUM;
    key_font.features = paint::tabular_figures();
    let run = TextRun {
        len: text.len(),
        font: key_font,
        color: gpui_kit::Hsla::black(),
        ..Default::default()
    };
    let line: ShapedLine =
        window
            .text_system()
            .shape_line(text.clone(), KEY_FONT_SIZE, &[run], None);
    let glyph_count = line
        .runs
        .iter()
        .map(|r| r.glyphs.len())
        .sum::<usize>()
        .max(text.chars().count());
    let width = line.width + KEY_TRACKING * glyph_count as f32;
    let outer_width = width + KEY_PAD_X * 2.;
    ShapedKeyChip {
        line,
        width,
        outer_width,
    }
}

#[derive(IntoElement)]
struct KeyChip {
    shaped: ShapedKeyChip,
    text_color: gpui_kit::Hsla,
    bg_color: gpui_kit::Hsla,
}

impl RenderOnce for KeyChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.text_color;
        let line = self.shaped.line;
        let width = self.shaped.width;

        div()
            .flex_none()
            .self_center()
            .mr(KEY_MARGIN_RIGHT)
            .px(KEY_PAD_X)
            .h(KEY_LINE_HEIGHT)
            .rounded(KEY_RADIUS)
            .bg(self.bg_color)
            .flex()
            .items_center()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        let ascent = line.ascent.round();
                        let descent = line.descent.round();
                        let box_h = ascent + descent;
                        let baseline = bounds.center().y - box_h / 2. + ascent;
                        let sf = window.scale_factor();
                        let baseline = px((baseline.as_f32() * sf + 0.5).floor() / sf);
                        paint_tracked_line(
                            &line,
                            point(bounds.left(), baseline),
                            KEY_FONT_SIZE,
                            KEY_TRACKING,
                            color,
                            window,
                            cx,
                        );
                    },
                )
                .w(width)
                .h(KEY_LINE_HEIGHT),
            )
    }
}

fn paint_tracked_line(
    line: &ShapedLine,
    origin: Point<Pixels>,
    font_size: Pixels,
    tracking: Pixels,
    color: gpui_kit::Hsla,
    window: &mut Window,
    _cx: &mut App,
) {
    let mut index = 0usize;
    for run in &line.runs {
        for glyph in &run.glyphs {
            let pos =
                origin + glyph.position + point(paint::tracked_offset(tracking, index), px(0.));
            if glyph.is_emoji {
                let _ = window.paint_emoji(pos, run.font_id, glyph.id, font_size);
            } else {
                let _ = window.paint_glyph(pos, run.font_id, glyph.id, font_size, color);
            }
            index += 1;
        }
    }
}

struct TooltipSurfaceBox {
    id: ElementId,
    spoken: SharedString,
    max_w: Pixels,
    content: SharedString,
    font_family: SharedString,
    key_chip: Option<KeyChip>,
    look: Look,
    alpha: f32,
}

impl IntoElement for TooltipSurfaceBox {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for TooltipSurfaceBox {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let key_chip = self.key_chip.take();
        let keys_space = key_chip.as_ref().map_or(px(0.), |chip| {
            chip.shaped.outer_width + KEY_MARGIN_RIGHT + GAP
        });
        let max_text_w = (self.max_w - PAD_X * 2. - keys_space).max(px(1.));

        let inner = div()
            .id((self.id.clone(), "inner"))
            .role(Role::Tooltip)
            .aria_label(self.spoken.clone())
            .font_family(self.font_family.clone())
            .max_w(self.max_w)
            .px(PAD_X)
            .py(PAD_Y)
            .flex()
            .items_center()
            .gap(GAP)
            .when(self.alpha < 1., |d| d.opacity(self.alpha))
            .child(
                div()
                    .min_w_0()
                    .max_w(max_text_w)
                    .text_size(FONT_SIZE)
                    .line_height(LINE_HEIGHT)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(self.look.ink)
                    .child(self.content.clone()),
            )
            .when_some(key_chip, |d, chip| d.child(chip));

        #[cfg(test)]
        let mut inner = inner.test_support().into_any_element();
        #[cfg(not(test))]
        let mut inner = inner.into_any_element();

        let layout_id = inner.request_layout(window, cx);
        (layout_id, inner)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        inner: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        inner.prepaint_at(bounds.origin, window, cx);
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        inner: &mut AnyElement,
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut shadows = self.look.shadows.clone();
        if self.alpha < 1. {
            for s in &mut shadows {
                s.color = s.color.opacity(self.alpha);
            }
        }
        paint::drop_shadows(bounds, self.look.radius, &shadows, window);
        window.paint_quad(
            fill(bounds, self.look.bg.opacity(self.alpha))
                .corner_radii(Corners::all(self.look.radius)),
        );
        inner.paint(window, cx);
    }
}
