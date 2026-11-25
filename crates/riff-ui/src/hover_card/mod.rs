//! riff-uiux `HoverCard`: a preview of what a link or name points to — opens
//! after hovering or keyboard focusing the trigger, keeps open through the
//! diagonal pointer corridor into the card, never opens on touch, and flips
//! to stay on screen.

mod corridor;
mod style;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::LayoutDirection;
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Context, Corners, DispatchPhase, Element, ElementId,
    Entity, EntityId, FocusHandle, FontWeight, Global, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, Modifiers, MouseDownEvent,
    MouseMoveEvent, ParentElement as _, Pixels, RenderOnce, StatefulInteractiveElement as _,
    Styled, Subscription, Task, WeakEntity, Window, WindowId, div, fill, point,
    prelude::FluentBuilder as _, px, size,
};

use crate::overlay::{self, OverlayAlign, OverlaySide, OverlayStack, PlaceOptions};
use crate::paint;
use crate::theme::RiffTheme;
use corridor::Pt;
use style::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, CLOSE_DELAY_DEFAULT, DRIFT, ENTER_DURATION, Look,
    MAX_WIDTH_DEFAULT, OFFSET_DEFAULT, OPEN_DELAY_DEFAULT, PADDING, VIEWPORT_MARGIN,
};

pub type HoverCardSide = OverlaySide;
pub type HoverCardAlign = OverlayAlign;

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// Window-wide Escape interceptor + single-active-card group per window.
#[derive(Default)]
struct HoverCardRegistry {
    interceptor: Option<Subscription>,
    controllers: HashMap<WindowId, Vec<WeakEntity<HoverCardController>>>,
    active: HashMap<WindowId, WeakEntity<HoverCardController>>,
}

impl Global for HoverCardRegistry {}

impl HoverCardRegistry {
    fn register(controller: &Entity<HoverCardController>, window: &Window, cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_none() {
            let sub = cx.intercept_keystrokes(|event, window, cx| {
                if event.keystroke.modifiers != Modifiers::default() {
                    return;
                }
                let key = event.keystroke.key.as_str();
                if key != "escape" && key != "tab" {
                    return;
                }
                let wid = window.window_handle().window_id();
                let target = cx
                    .try_global::<Self>()
                    .and_then(|reg| reg.controllers.get(&wid))
                    .and_then(|list| {
                        list.iter().rev().find_map(|weak| {
                            let entity = weak.upgrade()?;
                            if entity.read(cx).effective_open()
                                && OverlayStack::is_topmost(window, entity.entity_id(), cx)
                            {
                                Some(entity)
                            } else {
                                None
                            }
                        })
                    });
                let Some(ctrl) = target else {
                    return;
                };
                if key == "escape" {
                    window.prevent_default();
                    cx.stop_propagation();
                    ctrl.update(cx, |state, cx| {
                        state.dismiss_from_escape(window, cx);
                    });
                } else if ctrl.read(cx).trigger_focus.contains_focused(window, cx) {
                    let prev_focus = window.focused(cx);
                    let panel_focus = ctrl.read(cx).panel_focus.clone();
                    panel_focus.focus(window, cx);
                    window.focus_next(cx);
                    if panel_focus.contains_focused(window, cx) {
                        window.prevent_default();
                        cx.stop_propagation();
                    } else if let Some(prev) = prev_focus {
                        prev.focus(window, cx);
                    }
                }
            });
            cx.default_global::<Self>().interceptor = Some(sub);
        }
        let wid = window.window_handle().window_id();
        let list = cx
            .default_global::<Self>()
            .controllers
            .entry(wid)
            .or_default();
        list.retain(|w| w.upgrade().is_some());
        if !list.iter().any(|w| w.entity_id() == controller.entity_id()) {
            list.push(controller.downgrade());
        }
    }

    fn activate(controller: &Entity<HoverCardController>, window: &mut Window, cx: &mut App) {
        let wid = window.window_handle().window_id();
        let prev = cx
            .default_global::<Self>()
            .active
            .insert(wid, controller.downgrade())
            .and_then(|w| w.upgrade());
        if let Some(prev) = prev
            && prev.entity_id() != controller.entity_id()
        {
            prev.update(cx, |state, cx| state.close_from_group(window, cx));
        }
    }

    fn deactivate(id: EntityId, window: &Window, cx: &mut App) {
        if !cx.has_global::<Self>() {
            return;
        }
        let wid = window.window_handle().window_id();
        let reg = cx.global_mut::<Self>();
        if reg.active.get(&wid).is_some_and(|w| w.entity_id() == id) {
            reg.active.remove(&wid);
        }
    }
}

struct ControllerProps {
    disabled: bool,
    empty: bool,
    open_delay: Duration,
    close_delay: Duration,
    controlled_open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    inline: bool,
}

impl Default for ControllerProps {
    fn default() -> Self {
        Self {
            disabled: false,
            empty: true,
            open_delay: OPEN_DELAY_DEFAULT,
            close_delay: CLOSE_DELAY_DEFAULT,
            controlled_open: None,
            on_open_change: None,
            inline: false,
        }
    }
}

struct HoverCardController {
    trigger_focus: FocusHandle,
    panel_focus: FocusHandle,
    last_trigger_focus: Option<gpui_kit::WeakFocusHandle>,
    props: ControllerProps,
    shown: bool,
    blocked: bool,
    trigger_hovered: bool,
    card_hovered: bool,
    corridor: Option<Vec<Pt>>,
    timer_epoch: u64,
    timer: Option<Task<()>>,
    was_open: bool,
    enter_started: Option<Instant>,
    _subscriptions: Vec<Subscription>,
}

impl HoverCardController {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let trigger_focus = cx.focus_handle().tab_stop(false);
        let panel_focus = cx.focus_handle().tab_stop(false);
        let weak = cx.entity().downgrade();

        let trig_in = window.on_focus_in(&trigger_focus, cx, {
            let weak = weak.clone();
            move |window, cx| {
                let visible = window.last_input_was_keyboard();
                let current_focus = window.focused(cx).map(|f| f.downgrade());
                _ = weak.update(cx, |this, cx| {
                    this.last_trigger_focus = current_focus;
                    if visible {
                        this.show(window, cx);
                    }
                });
            }
        });

        let trig_out = window.on_focus_out(&trigger_focus, cx, {
            let weak = weak.clone();
            move |_, window, cx| {
                _ = weak.update(cx, |this, cx| {
                    this.blocked = false;
                    if this.props.inline {
                        return;
                    }
                    let inner_weak = cx.entity().downgrade();
                    window.defer(cx, move |window, cx| {
                        _ = inner_weak.update(cx, |state, cx| {
                            if state.panel_focus.contains_focused(window, cx)
                                || state.trigger_focus.contains_focused(window, cx)
                            {
                                return;
                            }
                            state.cancel_timer();
                            if (!state.trigger_hovered && !state.card_hovered)
                                || !state.effective_open()
                            {
                                state.apply_open(false, window, cx);
                            }
                        });
                    });
                });
            }
        });

        let panel_out = window.on_focus_out(&panel_focus, cx, {
            let weak = weak.clone();
            move |_, window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.props.inline {
                        return;
                    }
                    let inner_weak = cx.entity().downgrade();
                    window.defer(cx, move |window, cx| {
                        _ = inner_weak.update(cx, |state, cx| {
                            if state.panel_focus.contains_focused(window, cx)
                                || state.trigger_focus.contains_focused(window, cx)
                            {
                                return;
                            }
                            if state.effective_open() {
                                state.apply_open(false, window, cx);
                            }
                        });
                    });
                });
            }
        });

        Self {
            trigger_focus,
            panel_focus,
            last_trigger_focus: None,
            props: ControllerProps::default(),
            shown: false,
            blocked: false,
            trigger_hovered: false,
            card_hovered: false,
            corridor: None,
            timer_epoch: 0,
            timer: None,
            was_open: false,
            enter_started: None,
            _subscriptions: vec![trig_in, trig_out, panel_out],
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

    fn stay(&mut self) {
        self.cancel_timer();
        self.corridor = None;
    }

    fn apply_open(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        self.corridor = None;
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
                self.enter_started = if cx.reduce_motion() {
                    None
                } else {
                    Some(Instant::now())
                };
                self.was_open = true;
                HoverCardRegistry::register(&entity, window, cx);
                HoverCardRegistry::activate(&entity, window, cx);
                OverlayStack::push(window, entity.entity_id(), cx);
            } else if !now_open && prev {
                self.enter_started = None;
                self.was_open = false;
                OverlayStack::remove(window, entity.entity_id(), cx);
                HoverCardRegistry::deactivate(entity.entity_id(), window, cx);
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

    fn dismiss_from_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.blocked = true;
        let focus_back = self.panel_focus.contains_focused(window, cx);
        self.apply_open(false, window, cx);
        if focus_back {
            if let Some(prev) = self.last_trigger_focus.as_ref().and_then(|w| w.upgrade()) {
                prev.focus(window, cx);
            } else {
                let trigger_focus = self.trigger_focus.clone();
                trigger_focus.focus(window, cx);
                window.focus_next(cx);
                if !trigger_focus.contains_focused(window, cx) {
                    trigger_focus.focus(window, cx);
                }
            }
        }
    }

    fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        if self.props.disabled || self.props.empty || self.blocked || self.effective_open() {
            return;
        }
        let delay = self.props.open_delay;
        if delay.is_zero() {
            self.apply_open(true, window, cx);
            return;
        }
        let epoch = self.timer_epoch;
        let weak = cx.entity().downgrade();
        self.timer = Some(window.spawn(cx, async move |async_window| {
            async_window.background_executor().timer(delay).await;
            _ = async_window.update(|window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.timer_epoch == epoch {
                        this.apply_open(true, window, cx);
                    }
                });
            });
        }));
    }

    fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_timer();
        if !self.effective_open() {
            return;
        }
        let delay = self.props.close_delay;
        if delay.is_zero() {
            self.apply_open(false, window, cx);
            return;
        }
        let epoch = self.timer_epoch;
        let weak = cx.entity().downgrade();
        self.timer = Some(window.spawn(cx, async move |async_window| {
            async_window.background_executor().timer(delay).await;
            _ = async_window.update(|window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.timer_epoch == epoch
                        && !this.trigger_hovered
                        && !this.card_hovered
                        && this.corridor.is_none()
                    {
                        this.apply_open(false, window, cx);
                    }
                });
            });
        }));
    }

    fn handle_trigger_leave(
        &mut self,
        pos: gpui_kit::Point<Pixels>,
        trigger_bounds: Bounds<Pixels>,
        card_bounds: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.blocked = false;
        self.trigger_hovered = false;
        if self.card_hovered {
            self.stay();
            return;
        }
        if self.effective_open()
            && let Some(bounds) = card_bounds
        {
            let hull = corridor::hull_from_exit(pos, trigger_bounds, bounds);
            if corridor::in_hull(&hull, pos) {
                self.cancel_timer();
                self.corridor = Some(hull);
                return;
            }
        }
        self.corridor = None;
        if self.effective_open() {
            self.hide(window, cx);
        } else {
            self.cancel_timer();
        }
    }

    fn handle_corridor_move(
        &mut self,
        pos: gpui_kit::Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(hull) = &self.corridor else {
            return;
        };
        if !corridor::in_hull(hull, pos) {
            self.corridor = None;
            self.hide(window, cx);
        }
    }

    fn sync_frame(
        &mut self,
        controller: &Entity<Self>,
        props: ControllerProps,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let was_open = self.was_open;
        let was_inline = self.props.inline;
        self.props = props;
        let open = self.effective_open();
        if open && !was_open && !self.props.inline {
            self.enter_started = if cx.reduce_motion() {
                None
            } else {
                Some(Instant::now())
            };
            HoverCardRegistry::register(controller, window, cx);
            HoverCardRegistry::activate(controller, window, cx);
            OverlayStack::push(window, controller.entity_id(), cx);
        } else if !open && was_open && !was_inline {
            self.enter_started = None;
            self.corridor = None;
            OverlayStack::remove(window, controller.entity_id(), cx);
            HoverCardRegistry::deactivate(controller.entity_id(), window, cx);
        }
        self.was_open = open;
        open
    }

    fn enter_progress(&self, freeze_at: Option<Duration>, window: &mut Window, cx: &App) -> f32 {
        if self.props.inline || cx.reduce_motion() {
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

/// A riff-uiux hover card attached to a trigger element (typically a link or
/// name).
///
/// ```ignore
/// HoverCard::new("agent-card")
///     .width(px(288.))
///     .trigger(link)
///     .child(summary)
/// ```
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    trigger: Option<AnyElement>,
    children: Vec<AnyElement>,
    side: HoverCardSide,
    align: HoverCardAlign,
    offset: Pixels,
    open_delay: Duration,
    close_delay: Duration,
    width: Option<Pixels>,
    disabled: bool,
    open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    freeze_at: Option<Duration>,
    inline: bool,
}

impl HoverCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            children: Vec::new(),
            side: HoverCardSide::Bottom,
            align: HoverCardAlign::Start,
            offset: OFFSET_DEFAULT,
            open_delay: OPEN_DELAY_DEFAULT,
            close_delay: CLOSE_DELAY_DEFAULT,
            width: None,
            disabled: false,
            open: None,
            on_open_change: None,
            freeze_at: None,
            inline: false,
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn content(self, content: impl IntoElement) -> Self {
        self.child(content)
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn children<I>(mut self, children: impl IntoIterator<Item = I>) -> Self
    where
        I: IntoElement,
    {
        self.children
            .extend(children.into_iter().map(IntoElement::into_any_element));
        self
    }

    pub fn side(mut self, side: HoverCardSide) -> Self {
        self.side = side;
        self
    }

    pub fn bottom(self) -> Self {
        self.side(HoverCardSide::Bottom)
    }

    pub fn top(self) -> Self {
        self.side(HoverCardSide::Top)
    }

    pub fn left(self) -> Self {
        self.side(HoverCardSide::Left)
    }

    pub fn right(self) -> Self {
        self.side(HoverCardSide::Right)
    }

    pub fn align(mut self, align: HoverCardAlign) -> Self {
        self.align = align;
        self
    }

    pub fn start(self) -> Self {
        self.align(HoverCardAlign::Start)
    }

    pub fn center(self) -> Self {
        self.align(HoverCardAlign::Center)
    }

    pub fn end(self) -> Self {
        self.align(HoverCardAlign::End)
    }

    pub fn offset(mut self, offset: impl Into<Pixels>) -> Self {
        self.offset = offset.into();
        self
    }

    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = delay;
        self
    }

    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = delay;
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
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

    pub fn freeze_motion_at(mut self, at: Duration) -> Self {
        self.freeze_at = Some(at);
        self
    }

    #[doc(hidden)]
    pub fn inline_preview(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl RenderOnce for HoverCard {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(trigger) = self.trigger.take() else {
            return gpui_kit::Empty.into_any_element();
        };

        let empty = self.children.is_empty();
        let inline = self.inline;

        let controller = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            HoverCardController::new(window, cx)
        });

        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                ControllerProps {
                    disabled: self.disabled,
                    empty,
                    open_delay: self.open_delay,
                    close_delay: self.close_delay,
                    controlled_open: self.open,
                    on_open_change: self.on_open_change.clone(),
                    inline,
                },
                window,
                cx,
            )
        });

        let trigger_focus = controller.read(cx).trigger_focus.clone();
        let panel_focus = controller.read(cx).panel_focus.clone();

        let card_bounds = Rc::new(Cell::new(None));
        let trigger_bounds = Rc::new(Cell::new(Bounds::default()));

        let popup = open.then(|| {
            let progress = controller
                .read(cx)
                .enter_progress(self.freeze_at, window, cx);
            HoverCardPopupConfig {
                id: self.id.clone(),
                children: std::mem::take(&mut self.children),
                side: self.side,
                align: self.align,
                offset: self.offset,
                width: self.width,
                progress,
                inline,
                panel_focus,
            }
        });

        HoverCardElement {
            id: self.id,
            trigger,
            controller,
            trigger_focus,
            inline,
            popup,
            deferred_popup: None,
            trigger_bounds,
            card_bounds,
        }
        .into_any_element()
    }
}

struct HoverCardPopupConfig {
    id: ElementId,
    children: Vec<AnyElement>,
    side: HoverCardSide,
    align: HoverCardAlign,
    offset: Pixels,
    width: Option<Pixels>,
    progress: f32,
    inline: bool,
    panel_focus: FocusHandle,
}

struct HoverCardElement {
    id: ElementId,
    trigger: AnyElement,
    controller: Entity<HoverCardController>,
    trigger_focus: FocusHandle,
    inline: bool,
    popup: Option<HoverCardPopupConfig>,
    deferred_popup: Option<AnyElement>,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    card_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl IntoElement for HoverCardElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct HoverCardPrepaint {
    trigger_hitbox: Option<Hitbox>,
    inline_popup: Option<AnyElement>,
}

impl Element for HoverCardElement {
    type RequestLayoutState = ();
    type PrepaintState = HoverCardPrepaint;

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
    ) -> (LayoutId, ()) {
        let layout_id = self.trigger.request_layout(window, cx);
        if let Some(cfg) = self.popup.take() {
            let anchor = AnchoredHoverCard {
                id: cfg.id.clone(),
                cfg: Some(cfg),
                trigger_bounds: self.trigger_bounds.clone(),
                card_bounds: self.card_bounds.clone(),
                controller: self.controller.clone(),
            }
            .into_any_element();
            if self.inline {
                self.deferred_popup = Some(anchor);
            } else {
                let mut anchor = anchor;
                anchor.request_layout(window, cx);
                self.deferred_popup = Some(anchor);
            }
        }
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> HoverCardPrepaint {
        if !self.inline {
            window.set_focus_handle(&self.trigger_focus, cx);
        }
        self.trigger.prepaint(window, cx);
        self.trigger_bounds.set(bounds);

        let trigger_hitbox =
            (!self.inline).then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal));

        let inline_popup = if self.inline {
            self.deferred_popup.take().map(|mut el| {
                let available = size(AvailableSpace::MaxContent, AvailableSpace::MinContent);
                el.layout_as_root(available, window, cx);
                el.prepaint_at(point(px(0.), px(0.)), window, cx);
                el
            })
        } else {
            if let Some(deferred) = self.deferred_popup.take() {
                window.defer_draw(deferred, point(px(0.), px(0.)), 1000, None);
            }
            None
        };

        HoverCardPrepaint {
            trigger_hitbox,
            inline_popup,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut HoverCardPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.trigger.paint(window, cx);

        if let Some(mut inline_popup) = prepaint.inline_popup.take() {
            inline_popup.paint(window, cx);
            return;
        }

        if let Some(hitbox) = prepaint.trigger_hitbox.as_ref() {
            let hitbox_move = hitbox.clone();
            let controller = self.controller.clone();
            let card_bounds = self.card_bounds.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                if window.last_input_was_touch() {
                    return;
                }
                let hovered = hitbox_move.is_hovered(window);
                let (was, has_corridor) = {
                    let st = controller.read(cx);
                    (st.trigger_hovered, st.corridor.is_some())
                };
                if hovered != was {
                    controller.update(cx, |state, cx| {
                        state.trigger_hovered = hovered;
                        if hovered {
                            state.corridor = None;
                            if state.effective_open() {
                                state.stay();
                            } else {
                                state.show(window, cx);
                            }
                        } else {
                            state.handle_trigger_leave(
                                event.position,
                                bounds,
                                card_bounds.get(),
                                window,
                                cx,
                            );
                        }
                    });
                } else if !hovered && has_corridor {
                    controller.update(cx, |state, cx| {
                        state.handle_corridor_move(event.position, window, cx);
                    });
                }
            });

            let controller_down = self.controller.clone();
            let hitbox_down = hitbox.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                if hitbox_down.is_hovered(window) || bounds.contains(&event.position) {
                    controller_down.update(cx, |state, cx| {
                        state.blocked = true;
                        state.apply_open(false, window, cx);
                    });
                }
            });
        }
    }
}

struct AnchoredHoverCard {
    id: ElementId,
    cfg: Option<HoverCardPopupConfig>,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    card_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    controller: Entity<HoverCardController>,
}

impl IntoElement for AnchoredHoverCard {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct AnchoredPrepaint {
    surface: AnyElement,
    popup_hitbox: Option<Hitbox>,
    card_rect: Bounds<Pixels>,
}

impl Element for AnchoredHoverCard {
    type RequestLayoutState = ();
    type PrepaintState = AnchoredPrepaint;

    fn id(&self) -> Option<ElementId> {
        Some((self.id.clone(), "riff-hvc-anchor").into())
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
        let cfg = self.cfg.take().expect("prepaint called once");
        let inline = cfg.inline;
        let side = cfg.side;
        let align = cfg.align;
        let offset = cfg.offset;
        let progress = cfg.progress;
        let width_opt = cfg.width;

        let viewport = Bounds::new(point(px(0.), px(0.)), window.viewport_size());
        let max_viewport_w = (viewport.size.width - VIEWPORT_MARGIN * 2.).max(px(80.));
        let (max_w, exact_w) = match width_opt {
            Some(w) => {
                let capped = if inline { w } else { w.min(max_viewport_w) };
                (capped, Some(capped))
            }
            None => {
                let cap = if inline {
                    MAX_WIDTH_DEFAULT
                } else {
                    MAX_WIDTH_DEFAULT.min(max_viewport_w)
                };
                (cap, None)
            }
        };

        let mut surface = build_panel(
            cfg,
            max_w,
            exact_w,
            (!inline).then(|| (viewport.size.height - VIEWPORT_MARGIN * 2.).max(px(80.))),
            cx,
        );
        let available = size(AvailableSpace::MaxContent, AvailableSpace::MinContent);
        let popup_size = surface.layout_as_root(available, window, cx);

        let place_viewport = if inline {
            Bounds::new(
                point(px(-1_000_000.), px(-1_000_000.)),
                size(px(2_000_000.), px(2_000_000.)),
            )
        } else {
            viewport
        };
        let rtl = LayoutDirection::global(cx).is_rtl();
        let placed = overlay::place_side_with(
            self.trigger_bounds.get(),
            popup_size,
            place_viewport,
            side,
            align,
            offset,
            VIEWPORT_MARGIN,
            rtl,
            PlaceOptions {
                flip_align: true,
                flip_align_beside: false,
                prefer_bottom_fallback: true,
            },
        );
        let drift = placed.side.enter_drift(progress, DRIFT);
        let origin = placed.origin + drift;
        let card_rect = Bounds::new(origin, popup_size);
        self.card_bounds.set(Some(card_rect));
        surface.prepaint_at(origin, window, cx);

        let popup_hitbox =
            (!inline).then(|| window.insert_hitbox(card_rect, HitboxBehavior::Normal));

        AnchoredPrepaint {
            surface,
            popup_hitbox,
            card_rect,
        }
    }

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
                if phase != DispatchPhase::Capture || window.last_input_was_touch() {
                    return;
                }
                let hovered = hitbox.is_hovered(window);
                let was = controller.read(cx).card_hovered;
                if hovered == was {
                    return;
                }
                controller.update(cx, |state, cx| {
                    state.card_hovered = hovered;
                    if hovered {
                        state.stay();
                    } else {
                        state.blocked = false;
                        if !state.trigger_hovered {
                            state.hide(window, cx);
                        }
                    }
                });
            });

            let controller_out = self.controller.clone();
            let trigger_out = self.trigger_bounds.clone();
            let card_rect = prepaint.card_rect;
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                if card_rect.contains(&event.position)
                    || trigger_out.get().contains(&event.position)
                    || OverlayStack::has_descendant_after(window, controller_out.entity_id(), cx)
                {
                    return;
                }
                controller_out.update(cx, |state, cx| {
                    state.apply_open(false, window, cx);
                });
            });
        }
    }
}

fn build_panel(
    cfg: HoverCardPopupConfig,
    max_w: Pixels,
    exact_w: Option<Pixels>,
    max_h: Option<Pixels>,
    cx: &mut App,
) -> AnyElement {
    let theme = RiffTheme::global(cx).clone();
    let look = Look::of(&theme);
    let alpha = cfg.progress.clamp(0., 1.);

    let box_el = div()
        .id((cfg.id.clone(), "riff-hvc"))
        .font_family(theme.font_family)
        .when(!cfg.inline, |d| d.track_focus(&cfg.panel_focus).occlude())
        .when_some(exact_w, |d, w| d.w(w))
        .max_w(max_w)
        .when_some(max_h, |d, h| d.max_h(h).overflow_y_scroll())
        .p(PADDING)
        .text_size(BODY_FONT_SIZE)
        .line_height(BODY_LINE_HEIGHT)
        .font_weight(FontWeight::NORMAL)
        .text_color(look.ink)
        .when(alpha < 1., |d| d.opacity(alpha))
        .children(cfg.children);

    #[cfg(test)]
    let inner = {
        use gpui_kit::TestSupportExt as _;
        box_el.test_support().into_any_element()
    };
    #[cfg(not(test))]
    let inner = box_el.into_any_element();

    HoverCardSurface {
        id: (cfg.id, "riff-hvc-surf").into(),
        inner,
        look,
        alpha,
    }
    .into_any_element()
}

struct HoverCardSurface {
    id: ElementId,
    inner: AnyElement,
    look: Look,
    alpha: f32,
}

impl IntoElement for HoverCardSurface {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for HoverCardSurface {
    type RequestLayoutState = ();
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
    ) -> (LayoutId, ()) {
        (self.inner.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.prepaint_at(bounds.origin, window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut (),
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
            fill(bounds, self.look.surface.opacity(self.alpha))
                .corner_radii(Corners::all(self.look.radius)),
        );
        self.inner.paint(window, cx);
    }
}
