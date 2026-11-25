//! riff-uiux `Popover`: a small floating card attached to its trigger —
//! optional title and quiet description, arbitrary content, flips and switches
//! edge alignment to stay on screen, closes on outside press, Escape, or focus
//! leaving trigger and card, and hands focus back.

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
    Entity, FocusHandle, FontWeight, Global, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, Modifiers, MouseButton,
    MouseDownEvent, ParentElement as _, Pixels, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window, WindowId, div, fill, point,
    prelude::FluentBuilder as _, px, size,
};

use crate::overlay::{self, OverlayAlign, OverlaySide, OverlayStack, PlaceOptions};
use crate::paint;
use crate::theme::RiffTheme;
use style::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, DESC_FONT_SIZE, DESC_LINE_HEIGHT, DRIFT, ENTER_DURATION,
    HEAD_GAP, HEAD_MARGIN_BOTTOM, Look, MAX_WIDTH_DEFAULT, OFFSET_DEFAULT, PADDING,
    VIEWPORT_MARGIN,
};

pub type PopoverSide = OverlaySide;
pub type PopoverAlign = OverlayAlign;

type CloseHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// Window-wide Escape interceptor for open popovers; respects [`OverlayStack`]
/// so nested popups/menus close before enclosing popovers.
#[derive(Default)]
struct PopoverEscapeRegistry {
    interceptor: Option<Subscription>,
    controllers: HashMap<WindowId, Vec<gpui_kit::WeakEntity<PopoverController>>>,
}

impl Global for PopoverEscapeRegistry {}

impl PopoverEscapeRegistry {
    fn register(controller: &Entity<PopoverController>, window: &Window, cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_none() {
            let sub = cx.intercept_keystrokes(|event, window, cx| {
                if event.keystroke.key != "escape"
                    || event.keystroke.modifiers != Modifiers::default()
                {
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
                if let Some(ctrl) = target {
                    window.prevent_default();
                    cx.stop_propagation();
                    ctrl.update(cx, |state, cx| {
                        state.close_with_refocus(window, cx);
                    });
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
}

struct ControllerProps {
    controlled_open: Option<bool>,
    on_close: Option<CloseHandler>,
    on_open_change: Option<OpenChangeHandler>,
    focus_on_open: bool,
    inline: bool,
}

impl Default for ControllerProps {
    fn default() -> Self {
        Self {
            controlled_open: None,
            on_close: None,
            on_open_change: None,
            focus_on_open: true,
            inline: false,
        }
    }
}

struct PopoverController {
    scope_focus: FocusHandle,
    panel_focus: FocusHandle,
    previous_focus: Option<gpui_kit::WeakFocusHandle>,
    uncontrolled_open: bool,
    props: ControllerProps,
    was_closed: bool,
    was_open: bool,
    enter_started: Option<Instant>,
    focus_within_panel: bool,
    _subscriptions: Vec<Subscription>,
}

impl PopoverController {
    fn new(default_open: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scope_focus = cx.focus_handle().tab_stop(false);
        let panel_focus = cx.focus_handle().tab_stop(false);
        let weak = cx.entity().downgrade();

        let focus_in = window.on_focus_in(&panel_focus, cx, {
            let weak = weak.clone();
            move |_, cx| {
                _ = weak.update(cx, |this, _| {
                    this.focus_within_panel = true;
                });
            }
        });

        let scope_out = window.on_focus_out(&scope_focus, cx, {
            let weak = weak.clone();
            move |_, window, cx| {
                _ = weak.update(cx, |this, cx| {
                    this.focus_within_panel = this.panel_focus.contains_focused(window, cx);
                    if this.props.inline || !this.effective_open() {
                        return;
                    }
                    // Delay until focus settles so internal focus moves aren't treated as exits.
                    let inner_weak = cx.entity().downgrade();
                    window.defer(cx, move |window, cx| {
                        _ = inner_weak.update(cx, |state, cx| {
                            if state.effective_open()
                                && !state.scope_focus.contains_focused(window, cx)
                                && !OverlayStack::has_descendant_after(window, cx.entity_id(), cx)
                            {
                                state.close_without_refocus(window, cx);
                            }
                        });
                    });
                });
            }
        });

        Self {
            scope_focus,
            panel_focus,
            previous_focus: None,
            uncontrolled_open: default_open,
            props: ControllerProps::default(),
            was_closed: !default_open,
            was_open: false,
            enter_started: None,
            focus_within_panel: false,
            _subscriptions: vec![focus_in, scope_out],
        }
    }

    fn effective_open(&self) -> bool {
        self.props.controlled_open.unwrap_or(self.uncontrolled_open)
    }

    fn close_with_refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let should_refocus =
            self.focus_within_panel || self.panel_focus.contains_focused(window, cx);
        let prev = self.previous_focus.take();
        self.close_internal(window, cx);
        if should_refocus && let Some(prev) = prev.and_then(|w| w.upgrade()) {
            prev.focus(window, cx);
        }
    }

    fn close_without_refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.previous_focus = None;
        self.close_internal(window, cx);
    }

    fn close_internal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_open = self.effective_open();
        if self.props.controlled_open.is_none() && self.uncontrolled_open {
            self.uncontrolled_open = false;
            cx.notify();
            window.refresh();
        }
        if was_open {
            if let Some(on_close) = self.props.on_close.clone() {
                on_close(window, cx);
            }
            if let Some(on_change) = self.props.on_open_change.clone() {
                on_change(false, window, cx);
            }
        }
    }

    fn toggle_uncontrolled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.props.controlled_open.is_some() {
            return;
        }
        let next = !self.uncontrolled_open;
        if next {
            self.previous_focus = window.focused(cx).map(|f| f.downgrade());
        }
        self.uncontrolled_open = next;
        cx.notify();
        window.refresh();
        if let Some(on_change) = self.props.on_open_change.clone() {
            on_change(next, window, cx);
        }
        if !next && let Some(on_close) = self.props.on_close.clone() {
            on_close(window, cx);
        }
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
        if !open {
            self.was_closed = true;
            if self.was_open && !self.props.inline {
                let entity_id = controller.entity_id();
                OverlayStack::remove(window, entity_id, cx);
                let prev = self.previous_focus.take();
                let refocus =
                    self.focus_within_panel || self.panel_focus.contains_focused(window, cx);
                self.focus_within_panel = false;
                if refocus && let Some(prev) = prev.and_then(|w| w.upgrade()) {
                    let panel_focus = self.panel_focus.clone();
                    window.defer(cx, move |window, cx| {
                        if window.focused(cx).is_none() || panel_focus.contains_focused(window, cx)
                        {
                            prev.focus(window, cx);
                        }
                    });
                }
            }
            self.enter_started = None;
            self.was_open = false;
            return false;
        }

        if !self.was_open && !self.props.inline {
            PopoverEscapeRegistry::register(controller, window, cx);
            OverlayStack::push(window, controller.entity_id(), cx);
            if self.previous_focus.is_none() {
                self.previous_focus = window.focused(cx).map(|f| f.downgrade());
            }
            self.enter_started = if cx.reduce_motion() {
                None
            } else {
                Some(Instant::now())
            };
            if self.props.focus_on_open && self.was_closed {
                let panel_focus = self.panel_focus.clone();
                let weak = controller.downgrade();
                window.defer(cx, move |window, cx| {
                    if !panel_focus.contains_focused(window, cx) {
                        panel_focus.focus(window, cx);
                        window.focus_next(cx);
                        if !panel_focus.contains_focused(window, cx) {
                            panel_focus.focus(window, cx);
                        }
                    }
                    _ = weak.update(cx, |this, _| {
                        this.focus_within_panel = true;
                    });
                });
            }
        }

        self.was_open = true;
        true
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

/// A riff-uiux floating card attached to a trigger element.
///
/// ```ignore
/// Popover::new("invite-pop")
///     .title("Invite to the workspace")
///     .description("They get an email with a link to join.")
///     .align(PopoverAlign::End)
///     .width(px(300.))
///     .open(open)
///     .on_close(|_, _| set_open(false))
///     .trigger(Button::new("invite").label("Invite members"))
///     .child(form)
/// ```
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    side: PopoverSide,
    align: PopoverAlign,
    offset: Pixels,
    width: Option<Pixels>,
    focus_on_open: bool,
    unstyled: bool,
    role: Role,
    accessibility_label: Option<SharedString>,
    open: Option<bool>,
    default_open: bool,
    on_close: Option<CloseHandler>,
    on_open_change: Option<OpenChangeHandler>,
    inline: bool,
    freeze_at: Option<Duration>,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            title: None,
            description: None,
            children: Vec::new(),
            side: PopoverSide::Bottom,
            align: PopoverAlign::Start,
            offset: OFFSET_DEFAULT,
            width: None,
            focus_on_open: true,
            unstyled: false,
            role: Role::Dialog,
            accessibility_label: None,
            open: None,
            default_open: false,
            on_close: None,
            on_open_change: None,
            inline: false,
            freeze_at: None,
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.title = (!title.is_empty()).then_some(title);
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.description = (!description.is_empty()).then_some(description);
        self
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

    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    pub fn bottom(self) -> Self {
        self.side(PopoverSide::Bottom)
    }

    pub fn top(self) -> Self {
        self.side(PopoverSide::Top)
    }

    pub fn left(self) -> Self {
        self.side(PopoverSide::Left)
    }

    pub fn right(self) -> Self {
        self.side(PopoverSide::Right)
    }

    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    pub fn start(self) -> Self {
        self.align(PopoverAlign::Start)
    }

    pub fn center(self) -> Self {
        self.align(PopoverAlign::Center)
    }

    pub fn end(self) -> Self {
        self.align(PopoverAlign::End)
    }

    pub fn offset(mut self, offset: impl Into<Pixels>) -> Self {
        self.offset = offset.into();
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn focus_on_open(mut self, focus: bool) -> Self {
        self.focus_on_open = focus;
        self
    }

    pub fn unstyled(mut self, unstyled: bool) -> Self {
        self.unstyled = unstyled;
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.accessibility_label = (!label.is_empty()).then_some(label);
        self
    }

    pub fn open(mut self, open: impl Into<Option<bool>>) -> Self {
        self.open = open.into();
        self
    }

    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
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

impl RenderOnce for Popover {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(trigger) = self.trigger.take() else {
            return gpui_kit::Empty.into_any_element();
        };

        let controller = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            let initial = self.open.unwrap_or(self.default_open);
            PopoverController::new(initial, window, cx)
        });

        let inline = self.inline;
        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                ControllerProps {
                    controlled_open: self.open,
                    on_close: self.on_close.clone(),
                    on_open_change: self.on_open_change.clone(),
                    focus_on_open: self.focus_on_open,
                    inline,
                },
                window,
                cx,
            )
        });

        let scope_focus = controller.read(cx).scope_focus.clone();
        let panel_focus = controller.read(cx).panel_focus.clone();
        let controlled = self.open.is_some();

        let popup = open.then(|| {
            let progress = controller
                .read(cx)
                .enter_progress(self.freeze_at, window, cx);
            PopoverPopupConfig {
                id: self.id.clone(),
                title: self.title.take(),
                description: self.description.take(),
                children: std::mem::take(&mut self.children),
                side: self.side,
                align: self.align,
                offset: self.offset,
                width: self.width,
                unstyled: self.unstyled,
                role: self.role,
                accessibility_label: self.accessibility_label.take(),
                progress,
                inline,
                panel_focus,
            }
        });

        PopoverElement {
            id: self.id,
            trigger,
            controller,
            scope_focus,
            controlled,
            inline,
            popup,
            deferred_popup: None,
            trigger_bounds: Rc::new(Cell::new(Bounds::default())),
        }
        .into_any_element()
    }
}

struct PopoverPopupConfig {
    id: ElementId,
    title: Option<SharedString>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    side: PopoverSide,
    align: PopoverAlign,
    offset: Pixels,
    width: Option<Pixels>,
    unstyled: bool,
    role: Role,
    accessibility_label: Option<SharedString>,
    progress: f32,
    inline: bool,
    panel_focus: FocusHandle,
}

struct PopoverElement {
    id: ElementId,
    trigger: AnyElement,
    controller: Entity<PopoverController>,
    scope_focus: FocusHandle,
    controlled: bool,
    inline: bool,
    popup: Option<PopoverPopupConfig>,
    deferred_popup: Option<AnyElement>,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl IntoElement for PopoverElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct PopoverPrepaint {
    trigger_hitbox: Option<Hitbox>,
    inline_popup: Option<AnyElement>,
}

impl Element for PopoverElement {
    type RequestLayoutState = ();
    type PrepaintState = PopoverPrepaint;

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
            let anchor = AnchoredPopover {
                id: cfg.id.clone(),
                cfg: Some(cfg),
                trigger_bounds: self.trigger_bounds.clone(),
                controller: self.controller.clone(),
            }
            .into_any_element();
            if self.inline {
                self.popup = None;
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
    ) -> PopoverPrepaint {
        if !self.inline {
            window.set_focus_handle(&self.scope_focus, cx);
        }
        self.trigger.prepaint(window, cx);
        self.trigger_bounds.set(bounds);

        let trigger_hitbox = (!self.inline && !self.controlled)
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal));

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

        PopoverPrepaint {
            trigger_hitbox,
            inline_popup,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut PopoverPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.trigger.paint(window, cx);

        if let Some(mut inline_popup) = prepaint.inline_popup.take() {
            inline_popup.paint(window, cx);
            return;
        }

        if let Some(hitbox) = prepaint.trigger_hitbox.as_ref() {
            let hitbox = hitbox.clone();
            let controller = self.controller.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }
                if hitbox.is_hovered(window) {
                    controller.update(cx, |state, cx| state.toggle_uncontrolled(window, cx));
                }
            });
        }
    }
}

struct AnchoredPopover {
    id: ElementId,
    cfg: Option<PopoverPopupConfig>,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    controller: Entity<PopoverController>,
}

impl IntoElement for AnchoredPopover {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct AnchoredPrepaint {
    surface: AnyElement,
}

impl Element for AnchoredPopover {
    type RequestLayoutState = ();
    type PrepaintState = AnchoredPrepaint;

    fn id(&self) -> Option<ElementId> {
        Some((self.id.clone(), "riff-pop-anchor").into())
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
        let trigger_bounds_cell = self.trigger_bounds.clone();
        let controller = self.controller.clone();

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
            trigger_bounds_cell,
            controller,
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
                flip_align_beside: true,
                prefer_bottom_fallback: true,
            },
        );
        let drift = placed.side.enter_drift(progress, DRIFT);
        let origin = placed.origin + drift;
        surface.prepaint_at(origin, window, cx);

        AnchoredPrepaint { surface }
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
    }
}

fn build_panel(
    cfg: PopoverPopupConfig,
    max_w: Pixels,
    exact_w: Option<Pixels>,
    max_h: Option<Pixels>,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    controller: Entity<PopoverController>,
    cx: &mut App,
) -> AnyElement {
    let theme = RiffTheme::global(cx).clone();
    let look = Look::of(&theme);
    let alpha = cfg.progress.clamp(0., 1.);
    let has_head = cfg.title.is_some() || cfg.description.is_some();
    let has_body = !cfg.children.is_empty();
    let spoken = cfg
        .title
        .clone()
        .or(cfg.accessibility_label.clone())
        .or(cfg.description.clone());

    let box_el = div()
        .id((cfg.id.clone(), "riff-pop"))
        .role(cfg.role)
        .when_some(spoken, |d, label| d.aria_label(label))
        .font_family(theme.font_family)
        .when(!cfg.inline, |d| {
            d.track_focus(&cfg.panel_focus).tab_group().occlude()
        })
        .when_some(exact_w, |d, w| d.w(w))
        .max_w(max_w)
        .when_some(max_h, |d, h| d.max_h(h).overflow_y_scroll())
        .when(!cfg.unstyled, |d| {
            d.p(PADDING)
                .text_size(BODY_FONT_SIZE)
                .line_height(BODY_LINE_HEIGHT)
                .text_color(look.ink)
        })
        .when(alpha < 1., |d| d.opacity(alpha))
        .when(!cfg.unstyled && has_head, |d| {
            d.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(HEAD_GAP)
                    .when(has_body, |h| h.mb(HEAD_MARGIN_BOTTOM))
                    .when_some(cfg.title, |h, title| {
                        h.child(
                            div()
                                .text_size(BODY_FONT_SIZE)
                                .line_height(BODY_LINE_HEIGHT)
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(look.ink)
                                .child(title),
                        )
                    })
                    .when_some(cfg.description, |h, desc| {
                        h.child(
                            div()
                                .text_size(DESC_FONT_SIZE)
                                .line_height(DESC_LINE_HEIGHT)
                                .text_color(look.muted)
                                .child(desc),
                        )
                    }),
            )
        })
        .children(cfg.children)
        .when(!cfg.inline, |d| {
            let controller_out = controller.clone();
            let trigger_out = trigger_bounds.clone();
            d.on_mouse_down_out(move |event, window, cx| {
                if trigger_out.get().contains(&event.position) {
                    return;
                }
                if OverlayStack::has_descendant_after(window, controller_out.entity_id(), cx) {
                    return;
                }
                controller_out.update(cx, |state, cx| state.close_without_refocus(window, cx));
            })
        });

    #[cfg(test)]
    let inner = {
        use gpui_kit::TestSupportExt as _;
        box_el.test_support().into_any_element()
    };
    #[cfg(not(test))]
    let inner = box_el.into_any_element();

    PopoverSurface {
        id: (cfg.id, "riff-pop-surf").into(),
        inner,
        look,
        alpha,
        unstyled: cfg.unstyled,
        inline: cfg.inline,
    }
    .into_any_element()
}

struct PopoverSurface {
    id: ElementId,
    inner: AnyElement,
    look: Look,
    alpha: f32,
    unstyled: bool,
    inline: bool,
}

impl IntoElement for PopoverSurface {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for PopoverSurface {
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
        if !self.unstyled {
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
        }
        self.inner.paint(window, cx);
        let _ = self.inline;
    }
}
