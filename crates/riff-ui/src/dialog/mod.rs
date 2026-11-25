//! riff-uiux `Dialog` and `AlertDialog`: modal windows on a dimmed backdrop —
//! gray outer tray + inner white card (`theme.shadow_card`), scrollable body
//! beneath pinned header and footer, backdrop & Escape dismissal, Tab focus
//! trap, initial focus priority, and `AlertDialog` confirmation buttons.

mod style;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, BoxShadow, ClickEvent, Context, Corners,
    DispatchPhase, Element, ElementId, Entity, FocusHandle, FontWeight, Global, GlobalElementId,
    Hitbox, HitboxBehavior, Hsla, InspectorElementId, InteractiveElement as _, IntoElement,
    LayoutId, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ParentElement as _, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Styled, Subscription,
    WeakEntity, WeakFocusHandle, Window, WindowId, canvas, div, fill, point,
    prelude::FluentBuilder as _, px, size, svg,
};

use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::overlay::OverlayStack;
use crate::paint;
use crate::theme::RiffTheme;
use crate::tooltip::Tooltip;
use style::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, BODY_PAD_BOTTOM, BODY_PAD_TOP, BODY_PAD_X, CLOSE_ICON_SIZE,
    CLOSE_SIZE, CLOSE_TOP_OFFSET, DRIFT, ENTER_DURATION, FOOT_GAP, FOOT_PAD_BOTTOM, FOOT_PAD_TOP,
    FOOT_PAD_X, HEAD_BOTTOM_EMPTY, HEAD_GAP, HEAD_LEFT, HEAD_RIGHT, HEAD_TOP, HEAD_TOP_ALERT, Look,
    SCRIM_DURATION, SCRIM_PAD_X, SCRIM_PAD_Y, TITLE_DESC_GAP, TITLE_FONT_SIZE, TITLE_LINE_HEIGHT,
    TRAY_PAD,
};

/// Width preset of a [`Dialog`] or [`AlertDialog`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DialogSize {
    /// `400px` (`AlertDialog` default).
    Sm,
    /// `480px` (`Dialog` default).
    #[default]
    Md,
    /// `720px`.
    Lg,
}

impl DialogSize {
    fn width(self) -> Pixels {
        match self {
            Self::Sm => px(400.),
            Self::Md => px(480.),
            Self::Lg => px(720.),
        }
    }
}

type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Window-wide modal keyboard interceptor: Escape dismisses innermost non-busy
/// dialog; Tab / Shift+Tab stays trapped inside the modal.
#[derive(Default)]
struct DialogKeyRegistry {
    interceptor: Option<Subscription>,
    controllers: HashMap<WindowId, Vec<WeakEntity<DialogController>>>,
}

impl Global for DialogKeyRegistry {}

impl DialogKeyRegistry {
    fn register(controller: &Entity<DialogController>, window: &Window, cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_none() {
            let sub = cx.intercept_keystrokes(|event, window, cx| {
                let wid = window.window_handle().window_id();
                let target = cx
                    .try_global::<Self>()
                    .and_then(|reg| reg.controllers.get(&wid))
                    .and_then(|list| {
                        list.iter().rev().find_map(|weak| {
                            let entity = weak.upgrade()?;
                            let state = entity.read(cx);
                            if state.effective_open()
                                && !state.props.inline
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
                let key = event.keystroke.key.as_str();
                if key == "escape" && event.keystroke.modifiers == Modifiers::default() {
                    window.prevent_default();
                    cx.stop_propagation();
                    ctrl.update(cx, |state, cx| state.dismiss_with_refocus(window, cx));
                } else if key == "tab"
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.alt
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    let panel_focus = ctrl.read(cx).panel_focus.clone();
                    trap_tab(event.keystroke.modifiers.shift, &panel_focus, window, cx);
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

fn trap_tab(shift: bool, panel_focus: &FocusHandle, window: &mut Window, cx: &mut App) {
    if !panel_focus.contains_focused(window, cx) {
        panel_focus.focus(window, cx);
        window.focus_next(cx);
        if !panel_focus.contains_focused(window, cx) {
            panel_focus.focus(window, cx);
        }
        return;
    }

    if shift {
        window.focus_prev(cx);
        if panel_focus.contains_focused(window, cx)
            && window.focused(cx).as_ref() != Some(panel_focus)
        {
            return;
        }
        // Wrapped past first element inside panel -> jump to last focusable inside panel.
        panel_focus.focus(window, cx);
        let mut last = None;
        for _ in 0..64 {
            window.focus_next(cx);
            let cur = window.focused(cx);
            if !panel_focus.contains_focused(window, cx) || cur.as_ref() == Some(panel_focus) {
                break;
            }
            if last.as_ref() == cur.as_ref() {
                break;
            }
            last = cur;
        }
        if let Some(handle) = last {
            handle.focus(window, cx);
        } else {
            panel_focus.focus(window, cx);
        }
    } else {
        window.focus_next(cx);
        if !panel_focus.contains_focused(window, cx) {
            panel_focus.focus(window, cx);
            window.focus_next(cx);
            if !panel_focus.contains_focused(window, cx) {
                panel_focus.focus(window, cx);
            }
        }
    }
}

#[derive(Clone, Default)]
struct ControllerProps {
    controlled_open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    on_dismiss: Option<ActionHandler>,
    busy: bool,
    contained: bool,
    inline: bool,
    initial_focus: Option<FocusHandle>,
}

struct DialogController {
    panel_focus: FocusHandle,
    body_focus: FocusHandle,
    footer_focus: FocusHandle,
    close_focus: FocusHandle,
    previous_focus: Option<WeakFocusHandle>,
    uncontrolled_open: bool,
    props: ControllerProps,
    was_open: bool,
    first_open: bool,
    enter_started: Option<Instant>,
}

impl DialogController {
    fn new(default_open: bool, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            panel_focus: cx.focus_handle().tab_stop(false),
            body_focus: cx.focus_handle().tab_stop(false),
            footer_focus: cx.focus_handle().tab_stop(false),
            close_focus: cx.focus_handle().tab_stop(true),
            previous_focus: None,
            uncontrolled_open: default_open,
            props: ControllerProps::default(),
            was_open: false,
            first_open: default_open,
            enter_started: None,
        }
    }

    fn effective_open(&self) -> bool {
        self.props.controlled_open.unwrap_or(self.uncontrolled_open)
    }

    fn open_from_trigger(
        &mut self,
        opener_focus: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(f) = opener_focus {
            self.previous_focus = Some(f.downgrade());
        } else if self.previous_focus.is_none() {
            self.previous_focus = window.focused(cx).map(|f| f.downgrade());
        }
        let prev = self.effective_open();
        if self.props.controlled_open.is_none() && !self.uncontrolled_open {
            self.uncontrolled_open = true;
            cx.notify();
            window.refresh();
        }
        if !prev && let Some(on_change) = self.props.on_open_change.clone() {
            on_change(true, window, cx);
        }
    }

    fn dismiss_with_refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.props.busy {
            return;
        }
        if let Some(on_dismiss) = self.props.on_dismiss.clone() {
            on_dismiss(window, cx);
        }
        let prev_open = self.effective_open();
        let prev_focus = self.previous_focus.take();
        let should_refocus = self.panel_focus.contains_focused(window, cx);
        if self.props.controlled_open.is_none() && self.uncontrolled_open {
            self.uncontrolled_open = false;
            cx.notify();
            window.refresh();
        }
        if prev_open && let Some(on_change) = self.props.on_open_change.clone() {
            on_change(false, window, cx);
        }
        if should_refocus && let Some(prev) = prev_focus.and_then(|w| w.upgrade()) {
            prev.focus(window, cx);
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
            if self.was_open && !self.props.inline {
                OverlayStack::remove(window, controller.entity_id(), cx);
                let prev = self.previous_focus.take();
                let should_refocus = self.panel_focus.contains_focused(window, cx);
                if should_refocus && let Some(handle) = prev.and_then(|w| w.upgrade()) {
                    let panel_focus = self.panel_focus.clone();
                    window.defer(cx, move |window, cx| {
                        if window.focused(cx).is_none() || panel_focus.contains_focused(window, cx)
                        {
                            handle.focus(window, cx);
                        }
                    });
                }
            }
            self.was_open = false;
            self.first_open = false;
            self.enter_started = None;
            return false;
        }

        if !self.was_open && !self.props.inline {
            DialogKeyRegistry::register(controller, window, cx);
            OverlayStack::push(window, controller.entity_id(), cx);
            if self.previous_focus.is_none() {
                self.previous_focus = window.focused(cx).map(|f| f.downgrade());
            }
            self.enter_started = if cx.reduce_motion() {
                None
            } else {
                Some(Instant::now())
            };

            let should_focus = !self.props.contained || !self.first_open;
            if should_focus {
                let initial = self.props.initial_focus.clone();
                let body = self.body_focus.clone();
                let footer = self.footer_focus.clone();
                let panel = self.panel_focus.clone();
                window.defer(cx, move |window, cx| {
                    if let Some(init) = initial {
                        init.focus(window, cx);
                        return;
                    }
                    body.focus(window, cx);
                    window.focus_next(cx);
                    if body.contains_focused(window, cx) {
                        return;
                    }
                    footer.focus(window, cx);
                    window.focus_next(cx);
                    if footer.contains_focused(window, cx) {
                        return;
                    }
                    panel.focus(window, cx);
                    window.focus_next(cx);
                    if !panel.contains_focused(window, cx) {
                        panel.focus(window, cx);
                    }
                });
            }
        }
        self.was_open = true;
        self.first_open = false;
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

    fn scrim_progress(&self, freeze_at: Option<Duration>, window: &mut Window, cx: &App) -> f32 {
        if self.props.inline || cx.reduce_motion() {
            return 1.;
        }
        let Some(started) = self.enter_started else {
            return 1.;
        };
        let elapsed = freeze_at.unwrap_or_else(|| started.elapsed());
        if elapsed >= SCRIM_DURATION {
            return 1.;
        }
        window.request_animation_frame();
        (elapsed.as_secs_f32() / SCRIM_DURATION.as_secs_f32()).clamp(0., 1.)
    }
}

/// A riff-uiux modal window on a dimmed backdrop.
#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    trigger: Option<AnyElement>,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    size: DialogSize,
    width: Option<Pixels>,
    close_on_backdrop: bool,
    close_button: bool,
    close_label: SharedString,
    initial_focus: Option<FocusHandle>,
    contained: bool,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeHandler>,
    on_dismiss: Option<ActionHandler>,
    role: Role,
    busy: bool,
    alert: bool,
    alert_actions: Option<(AlertAction, AlertAction)>,
    freeze_at: Option<Duration>,
    inline: bool,
    controller: Option<Entity<DialogController>>,
}

type ButtonClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone)]
struct AlertAction {
    wrap_id: ElementId,
    btn_id: ElementId,
    focus: Option<FocusHandle>,
    variant: ButtonVariant,
    label: SharedString,
    disabled: bool,
    loading: bool,
    on_click: ButtonClickHandler,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            title: title.into(),
            description: None,
            children: Vec::new(),
            footer: Vec::new(),
            size: DialogSize::Md,
            width: None,
            close_on_backdrop: true,
            close_button: true,
            close_label: "Close".into(),
            initial_focus: None,
            contained: false,
            open: None,
            default_open: false,
            on_open_change: None,
            on_dismiss: None,
            role: Role::Dialog,
            busy: false,
            alert: false,
            alert_actions: None,
            freeze_at: None,
            inline: false,
            controller: None,
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let d = description.into();
        self.description = (!d.is_empty()).then_some(d);
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

    pub fn footer(mut self, action: impl IntoElement) -> Self {
        self.footer.push(action.into_any_element());
        self
    }

    pub fn footers<I>(mut self, actions: impl IntoIterator<Item = I>) -> Self
    where
        I: IntoElement,
    {
        self.footer
            .extend(actions.into_iter().map(IntoElement::into_any_element));
        self
    }

    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    pub fn sm(self) -> Self {
        self.size(DialogSize::Sm)
    }

    pub fn md(self) -> Self {
        self.size(DialogSize::Md)
    }

    pub fn lg(self) -> Self {
        self.size(DialogSize::Lg)
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    pub fn close_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.close_label = l;
        }
        self
    }

    pub fn initial_focus(mut self, focus: FocusHandle) -> Self {
        self.initial_focus = Some(focus);
        self
    }

    /// Renders inside its immediate parent frame rather than covering the full window.
    pub fn contained(mut self, contained: bool) -> Self {
        self.contained = contained;
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

    fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    fn alert(mut self, alert: bool) -> Self {
        self.alert = alert;
        self
    }

    fn on_dismiss(mut self, handler: ActionHandler) -> Self {
        self.on_dismiss = Some(handler);
        self
    }

    fn with_controller(mut self, ctrl: Entity<DialogController>) -> Self {
        self.controller = Some(ctrl);
        self
    }

    fn with_alert_actions(mut self, cancel: AlertAction, confirm: AlertAction) -> Self {
        self.alert_actions = Some((cancel, confirm));
        self
    }
}

impl RenderOnce for Dialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let inline = self.inline;
        let contained = self.contained;
        let controller = self.controller.take().unwrap_or_else(|| {
            window.use_keyed_state(self.id.clone(), cx, |window, cx| {
                let initial = self.open.unwrap_or(self.default_open);
                DialogController::new(initial, window, cx)
            })
        });
        let trigger_focus =
            window.use_keyed_state((self.id.clone(), "dlg-trig-focus"), cx, |_, cx| {
                cx.focus_handle().tab_stop(false)
            });
        let trigger_focus_handle = trigger_focus.read(cx).clone();

        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                ControllerProps {
                    controlled_open: self.open,
                    on_open_change: self.on_open_change.clone(),
                    on_dismiss: self.on_dismiss.clone(),
                    busy: self.busy,
                    contained: self.contained,
                    inline,
                    initial_focus: self.initial_focus.clone(),
                },
                window,
                cx,
            )
        });

        let controlled = self.open.is_some();
        let panel_w = self.width.unwrap_or_else(|| self.size.width());

        let modal = open.then(|| {
            let enter_p = controller
                .read(cx)
                .enter_progress(self.freeze_at, window, cx);
            let scrim_p = controller
                .read(cx)
                .scrim_progress(self.freeze_at, window, cx);
            DialogLayerSpec {
                id: self.id.clone(),
                title: self.title.clone(),
                description: self.description.take(),
                children: std::mem::take(&mut self.children),
                footer: std::mem::take(&mut self.footer),
                alert_actions: self.alert_actions.take(),
                preferred_w: panel_w,
                close_on_backdrop: self.close_on_backdrop,
                close_button: self.close_button,
                close_label: self.close_label.clone(),
                inline,
                role: self.role,
                alert: self.alert,
                busy: self.busy,
                enter_p,
                scrim_p,
                controller: controller.clone(),
            }
        });

        let trigger = self.trigger.take().map(|tr| {
            if inline {
                tr
            } else {
                div()
                    .id((self.id.clone(), "dlg-trig-wrap"))
                    .track_focus(&trigger_focus_handle)
                    .child(tr)
                    .into_any_element()
            }
        });

        DialogElement {
            id: self.id,
            trigger,
            controller,
            trigger_focus: trigger_focus_handle,
            controlled,
            contained,
            inline,
            modal,
        }
        .into_any_element()
    }
}

/// A riff-uiux confirmation modal built on [`Dialog`] with safe-first Cancel,
/// primary/danger Confirm, busy state, and no accidental backdrop dismissal.
#[derive(IntoElement)]
pub struct AlertDialog {
    id: ElementId,
    trigger: Option<AnyElement>,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    size: DialogSize,
    width: Option<Pixels>,
    confirm_label: SharedString,
    cancel_label: SharedString,
    danger: bool,
    busy: bool,
    contained: bool,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeHandler>,
    on_confirm: Option<ActionHandler>,
    on_cancel: Option<ActionHandler>,
    freeze_at: Option<Duration>,
    inline: bool,
}

impl AlertDialog {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            title: title.into(),
            description: None,
            children: Vec::new(),
            size: DialogSize::Sm,
            width: None,
            confirm_label: "Confirm".into(),
            cancel_label: "Cancel".into(),
            danger: false,
            busy: false,
            contained: false,
            open: None,
            default_open: false,
            on_open_change: None,
            on_confirm: None,
            on_cancel: None,
            freeze_at: None,
            inline: false,
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let d = description.into();
        self.description = (!d.is_empty()).then_some(d);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = label.into();
        self
    }

    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn contained(mut self, contained: bool) -> Self {
        self.contained = contained;
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

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
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

impl RenderOnce for AlertDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cancel_focus =
            window.use_keyed_state((self.id.clone(), "alert-cancel-focus"), cx, |_, cx| {
                cx.focus_handle().tab_stop(true)
            });
        let cancel_focus_handle = cancel_focus.read(cx).clone();

        let busy = self.busy;
        let on_cancel = self.on_cancel.clone();
        let on_confirm = self.on_confirm.clone();
        let on_open_change = self.on_open_change.clone();
        let controlled = self.open.is_some();

        let dialog_id = self.id.clone();
        let initial_open = self.open.unwrap_or(self.default_open);
        let controller = window.use_keyed_state(dialog_id.clone(), cx, |window, cx| {
            DialogController::new(initial_open, window, cx)
        });
        let cancel_wrap_id: ElementId = (self.id.clone(), "alert-cancel").into();
        let confirm_wrap_id: ElementId = (self.id.clone(), "alert-confirm").into();

        // Notice cancel / confirm wire straight back into the shared DialogController state if uncontrolled.
        let mut dlg = Dialog::new(dialog_id.clone(), self.title)
            .with_controller(controller.clone())
            .size(self.size)
            .close_on_backdrop(false)
            .close_button(false)
            .contained(self.contained)
            .open(self.open)
            .default_open(self.default_open)
            .inline_preview(self.inline)
            .role(Role::AlertDialog)
            .alert(true)
            .busy(busy)
            .initial_focus(cancel_focus_handle.clone())
            .children(self.children);

        if let Some(tr) = self.trigger {
            dlg = dlg.trigger(tr);
        }
        if let Some(d) = self.description {
            dlg = dlg.description(d);
        }
        if let Some(w) = self.width {
            dlg = dlg.width(w);
        }
        if let Some(f) = self.freeze_at {
            dlg = dlg.freeze_motion_at(f);
        }
        if let Some(cb) = on_open_change.clone() {
            dlg = dlg.on_open_change(move |o, w, cx| cb(o, w, cx));
        }
        if let Some(cancel_cb) = on_cancel.clone() {
            dlg = dlg.on_dismiss(cancel_cb);
        }

        let cancel_action = {
            let cancel_cb = on_cancel.clone();
            let change_cb = on_open_change.clone();
            let ctrl = controller.clone();
            let fh = cancel_focus_handle.clone();
            AlertAction {
                wrap_id: cancel_wrap_id,
                btn_id: (self.id.clone(), "btn-cancel").into(),
                focus: Some(fh),
                variant: ButtonVariant::Secondary,
                label: self.cancel_label,
                disabled: busy,
                loading: false,
                on_click: Rc::new(move |_, window, cx| {
                    if busy {
                        return;
                    }
                    if let Some(cb) = &cancel_cb {
                        cb(window, cx);
                    }
                    if !controlled {
                        ctrl.update(cx, |s, cx| s.dismiss_with_refocus(window, cx));
                    } else if let Some(cb) = &change_cb {
                        cb(false, window, cx);
                    }
                }),
            }
        };

        let confirm_action = {
            let change_cb = on_open_change;
            let ctrl = controller;
            AlertAction {
                wrap_id: confirm_wrap_id,
                btn_id: (self.id.clone(), "btn-confirm").into(),
                focus: None,
                variant: if self.danger {
                    ButtonVariant::Destructive
                } else {
                    ButtonVariant::Primary
                },
                label: self.confirm_label,
                disabled: false,
                loading: busy,
                on_click: Rc::new(move |_, window, cx| {
                    if busy {
                        return;
                    }
                    if let Some(cb) = &on_confirm {
                        cb(window, cx);
                    }
                    if !controlled {
                        ctrl.update(cx, |s, cx| s.dismiss_with_refocus(window, cx));
                    } else if let Some(cb) = &change_cb {
                        cb(false, window, cx);
                    }
                }),
            }
        };

        dlg.with_alert_actions(cancel_action, confirm_action)
    }
}

struct DialogLayerSpec {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    alert_actions: Option<(AlertAction, AlertAction)>,
    preferred_w: Pixels,
    close_on_backdrop: bool,
    close_button: bool,
    close_label: SharedString,
    inline: bool,
    role: Role,
    alert: bool,
    busy: bool,
    enter_p: f32,
    scrim_p: f32,
    controller: Entity<DialogController>,
}

struct DialogElement {
    id: ElementId,
    trigger: Option<AnyElement>,
    controller: Entity<DialogController>,
    trigger_focus: FocusHandle,
    controlled: bool,
    contained: bool,
    inline: bool,
    modal: Option<DialogLayerSpec>,
}

impl IntoElement for DialogElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct DialogPrepaint {
    trigger_hitbox: Option<Hitbox>,
    contained_layer: Option<AnyElement>,
}

impl Element for DialogElement {
    type RequestLayoutState = ();
    type PrepaintState = DialogPrepaint;

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
        let layout_id = if let Some(trigger) = self.trigger.as_mut() {
            trigger.request_layout(window, cx)
        } else if self.inline || self.contained {
            window.request_layout(
                gpui_kit::Style {
                    size: gpui_kit::Size::full(),
                    ..Default::default()
                },
                [],
                cx,
            )
        } else {
            window.request_layout(gpui_kit::Style::default(), [], cx)
        };
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
    ) -> DialogPrepaint {
        if let Some(trigger) = self.trigger.as_mut() {
            trigger.prepaint(window, cx);
        }
        let trigger_hitbox = (!self.inline && !self.controlled)
            .then(|| {
                self.trigger
                    .as_ref()
                    .map(|_| window.insert_hitbox(bounds, HitboxBehavior::Normal))
            })
            .flatten();

        let contained_layer = if let Some(spec) = self.modal.take() {
            let layer_el = ModalLayer {
                id: (spec.id.clone(), "dlg-layer").into(),
                container_bounds: (self.contained || self.inline).then_some(bounds),
                spec: Some(spec),
            }
            .into_any_element();

            if self.contained || self.inline {
                let mut el = layer_el;
                el.layout_as_root(
                    size(
                        AvailableSpace::Definite(bounds.size.width),
                        AvailableSpace::Definite(bounds.size.height),
                    ),
                    window,
                    cx,
                );
                el.prepaint_at(bounds.origin, window, cx);
                Some(el)
            } else {
                let mut el = layer_el;
                let vp = window.viewport_size();
                el.request_layout(window, cx);
                let _ = vp;
                window.defer_draw(el, point(px(0.), px(0.)), 1000, None);
                None
            }
        } else {
            None
        };

        DialogPrepaint {
            trigger_hitbox,
            contained_layer,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut DialogPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(trigger) = self.trigger.as_mut() {
            trigger.paint(window, cx);
        }
        if let Some(mut layer) = prepaint.contained_layer.take() {
            layer.paint(window, cx);
        }
        if let Some(hitbox) = prepaint.trigger_hitbox.as_ref() {
            let hitbox = hitbox.clone();
            let controller = self.controller.clone();
            let trigger_focus = self.trigger_focus.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                    return;
                }
                if hitbox.is_hovered(window) {
                    let return_focus = window.focused(cx).or_else(|| Some(trigger_focus.clone()));
                    controller.update(cx, |s, cx| s.open_from_trigger(return_focus, window, cx));
                }
            });
        }
    }
}

struct ModalLayer {
    id: ElementId,
    container_bounds: Option<Bounds<Pixels>>,
    spec: Option<DialogLayerSpec>,
}

impl IntoElement for ModalLayer {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct ModalLayerPrepaint {
    inner: AnyElement,
    scrim_rect: Bounds<Pixels>,
    tray_rect: Bounds<Pixels>,
    scrim_color: Hsla,
    close_on_backdrop: bool,
    busy: bool,
    inline: bool,
    controller: Entity<DialogController>,
}

impl Element for ModalLayer {
    type RequestLayoutState = ();
    type PrepaintState = ModalLayerPrepaint;

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
        (
            window.request_layout(gpui_kit::Style::default(), [], cx),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> ModalLayerPrepaint {
        let spec = self.spec.take().expect("prepaint called once");
        let theme = RiffTheme::global(cx).clone();
        let look = Look::of(&theme);
        let scrim_rect = self
            .container_bounds
            .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), window.viewport_size()));

        let narrow = (scrim_rect.size.width - SCRIM_PAD_X * 2.) <= px(520.);
        let max_tray_w = if narrow {
            scrim_rect.size.width
        } else {
            (scrim_rect.size.width - SCRIM_PAD_X * 2.).max(px(120.))
        };
        let max_tray_h = if narrow {
            (scrim_rect.size.height - px(40.)).max(px(100.))
        } else {
            (scrim_rect.size.height - SCRIM_PAD_Y * 2.).max(px(100.))
        };
        let tray_w = if narrow {
            scrim_rect.size.width
        } else {
            spec.preferred_w.min(max_tray_w)
        };

        let enter_p = spec.enter_p.clamp(0., 1.);
        let scrim_color = look.scrim.opacity(spec.scrim_p.clamp(0., 1.));
        let close_on_backdrop = spec.close_on_backdrop;
        let busy = spec.busy;
        let inline = spec.inline;
        let controller = spec.controller.clone();

        let tray_rect_cell = Rc::new(Cell::new(Bounds::default()));
        let tray_cell_clone = tray_rect_cell.clone();

        let mut panel_el = build_panel(
            spec,
            tray_w,
            max_tray_h,
            narrow,
            &look,
            &theme.font_family,
            window,
            cx,
        );
        let panel_size = panel_el.layout_as_root(
            size(AvailableSpace::Definite(tray_w), AvailableSpace::MinContent),
            window,
            cx,
        );
        let panel_h = panel_size.height.min(max_tray_h);
        let panel_origin = if narrow {
            let drift_y = panel_h * (1. - enter_p);
            point(
                scrim_rect.left().round(),
                (scrim_rect.bottom() - panel_h + drift_y).round(),
            )
        } else {
            let drift_y = DRIFT * (1. - enter_p);
            point(
                (scrim_rect.left() + (scrim_rect.size.width - panel_size.width) / 2.).round(),
                (scrim_rect.top() + (scrim_rect.size.height - panel_h) / 2. + drift_y).round(),
            )
        };
        let final_rect = Bounds::new(panel_origin, size(panel_size.width, panel_h));
        tray_cell_clone.set(final_rect);
        panel_el.prepaint_at(panel_origin, window, cx);

        ModalLayerPrepaint {
            inner: panel_el,
            scrim_rect,
            tray_rect: final_rect,
            scrim_color,
            close_on_backdrop,
            busy,
            inline,
            controller,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut ModalLayerPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.paint_quad(fill(prepaint.scrim_rect, prepaint.scrim_color));
        prepaint.inner.paint(window, cx);
        if prepaint.inline {
            return;
        }

        let scrim_rect = prepaint.scrim_rect;
        let tray_rect = prepaint.tray_rect;
        let close_on_backdrop = prepaint.close_on_backdrop;
        let busy = prepaint.busy;
        let controller = prepaint.controller.clone();
        let down_on_backdrop = Rc::new(Cell::new(false));
        let down_clone = down_on_backdrop.clone();

        window.on_mouse_event(move |event: &MouseDownEvent, phase, _, _| {
            if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                return;
            }
            let hit = scrim_rect.contains(&event.position) && !tray_rect.contains(&event.position);
            down_clone.set(hit);
        });

        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                return;
            }
            let started = down_on_backdrop.replace(false);
            if !started {
                return;
            }
            if scrim_rect.contains(&event.position)
                && !tray_rect.contains(&event.position)
                && close_on_backdrop
                && !busy
                && OverlayStack::is_topmost(window, controller.entity_id(), cx)
            {
                cx.stop_propagation();
                controller.update(cx, |s, cx| s.dismiss_with_refocus(window, cx));
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn build_panel(
    spec: DialogLayerSpec,
    tray_w: Pixels,
    max_tray_h: Pixels,
    narrow: bool,
    look: &Look,
    font_family: &SharedString,
    _window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let ctrl = spec.controller.read(cx);
    let panel_focus = ctrl.panel_focus.clone();
    let body_focus = ctrl.body_focus.clone();
    let footer_focus = ctrl.footer_focus.clone();
    let close_focus = ctrl.close_focus.clone();
    let alpha = spec.enter_p.clamp(0., 1.);
    let has_body = !spec.children.is_empty();
    let has_footer = !spec.footer.is_empty() || spec.alert_actions.is_some();
    let controller_close = spec.controller.clone();

    let close_btn = spec.close_button.then(|| {
        let controller = controller_close.clone();
        let label = spec.close_label.clone();
        let wash = look.wash;
        let icon_color = look.icon;
        let ink = look.ink;
        let btn = div()
            .id((spec.id.clone(), "dlg-close"))
            .role(Role::Button)
            .aria_label(label.clone())
            .track_focus(&close_focus)
            .tab_index(0)
            .mt(CLOSE_TOP_OFFSET)
            .size(CLOSE_SIZE)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.close_radius)
            .text_color(icon_color)
            .hover(move |s| s.bg(wash).text_color(ink))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(move |_, window, cx| {
                controller.update(cx, |s, cx| s.dismiss_with_refocus(window, cx));
            })
            .child(
                svg()
                    .path("icons/x.svg")
                    .size(CLOSE_ICON_SIZE)
                    .text_color(icon_color),
            );
        #[cfg(test)]
        let btn = {
            use gpui_kit::TestSupportExt as _;
            btn.test_support()
        };
        Tooltip::new((spec.id.clone(), "dlg-close-tip"))
            .content(label)
            .child(btn)
            .into_any_element()
    });

    let head = div()
        .flex_none()
        .flex()
        .items_start()
        .gap(HEAD_GAP)
        .pt(if spec.alert { HEAD_TOP_ALERT } else { HEAD_TOP })
        .when(!has_body, |d| d.pb(HEAD_BOTTOM_EMPTY))
        .pl(HEAD_LEFT)
        .pr(if spec.close_button {
            HEAD_RIGHT
        } else {
            HEAD_LEFT
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(TITLE_DESC_GAP)
                .child(
                    div()
                        .text_size(TITLE_FONT_SIZE)
                        .line_height(TITLE_LINE_HEIGHT)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(look.ink)
                        .child(spec.title.clone()),
                )
                .when_some(spec.description.clone(), |d, desc| {
                    d.child(
                        div()
                            .text_size(BODY_FONT_SIZE)
                            .line_height(BODY_LINE_HEIGHT)
                            .text_color(look.text)
                            .child(desc),
                    )
                }),
        )
        .when_some(close_btn, |d, btn| d.child(btn));

    let body = has_body.then(|| {
        div()
            .id((spec.id.clone(), "dlg-body"))
            .when(!spec.inline, |d| d.track_focus(&body_focus))
            .flex_1()
            .min_h(px(0.))
            .pt(BODY_PAD_TOP)
            .px(BODY_PAD_X)
            .pb(BODY_PAD_BOTTOM)
            .overflow_y_scroll()
            .text_size(BODY_FONT_SIZE)
            .line_height(BODY_LINE_HEIGHT)
            .text_color(look.ink)
            .children(spec.children)
    });

    let card_corners = Corners::all(look.card_radius);
    let card_surface = div()
        .id((spec.id.clone(), "dlg-card-inner"))
        .relative()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .rounded(look.card_radius)
        .overflow_hidden()
        .child(surface_paint_layer(
            look.surface,
            card_corners,
            look.card_radius,
            look.card_shadows.clone(),
            alpha,
        ))
        .child(head)
        .when_some(body, |d, b| d.child(b));

    let footer_items: Vec<AnyElement> = if let Some((cancel, confirm)) = spec.alert_actions {
        [cancel, confirm]
            .into_iter()
            .enumerate()
            .map(|(idx, act)| {
                let on_click = act.on_click;
                let surface_override = (idx == 0).then_some(look.alert_cancel_bg).flatten();
                div()
                    .id(act.wrap_id)
                    .when_some(act.focus, |d, fh| d.track_focus(&fh))
                    .when(narrow, |d| d.flex_1().min_w_0())
                    .child(
                        Button::new(act.btn_id)
                            .variant(act.variant)
                            .size(ButtonSize::Sm)
                            .disabled(act.disabled)
                            .loading(act.loading)
                            .full_width(narrow)
                            .surface_override(surface_override)
                            .label(act.label)
                            .on_click(move |ev, window, cx| on_click(ev, window, cx)),
                    )
                    .into_any_element()
            })
            .collect()
    } else {
        spec.footer
    };

    let footer_el = has_footer.then(|| {
        div()
            .id((spec.id.clone(), "dlg-foot"))
            .when(!spec.inline, |d| d.track_focus(&footer_focus))
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap(FOOT_GAP)
            .pt(FOOT_PAD_TOP)
            .px(FOOT_PAD_X)
            .pb(FOOT_PAD_BOTTOM)
            .children(footer_items)
    });

    let bottom_r = if narrow { px(0.) } else { look.tray_radius };
    let tray_corners = Corners {
        top_left: look.tray_radius,
        top_right: look.tray_radius,
        bottom_right: bottom_r,
        bottom_left: bottom_r,
    };

    let tray_inner = div()
        .id((spec.id, "dlg-tray"))
        .relative()
        .role(spec.role)
        .aria_label(spec.title)
        .font_family(font_family.clone())
        .when(!spec.inline, |d| d.track_focus(&panel_focus).occlude())
        .w(tray_w)
        .max_h(max_tray_h)
        .min_h(px(0.))
        .p(TRAY_PAD)
        .flex()
        .flex_col()
        .rounded_tl(look.tray_radius)
        .rounded_tr(look.tray_radius)
        .rounded_bl(bottom_r)
        .rounded_br(bottom_r)
        .text_color(look.ink)
        .when(alpha < 1., |d| d.opacity(alpha))
        .child(surface_paint_layer(
            look.tray,
            tray_corners,
            look.tray_radius,
            look.tray_shadows.clone(),
            alpha,
        ))
        .child(card_surface)
        .when_some(footer_el, |d, f| d.child(f));

    #[cfg(test)]
    let tray_inner = {
        use gpui_kit::TestSupportExt as _;
        tray_inner.test_support().into_any_element()
    };
    #[cfg(not(test))]
    let tray_inner = tray_inner.into_any_element();

    tray_inner
}

fn surface_paint_layer(
    fill_color: Hsla,
    corners: Corners<Pixels>,
    shadow_radius: Pixels,
    shadows: Vec<BoxShadow>,
    alpha: f32,
) -> AnyElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let mut shadows = shadows.clone();
            if alpha < 1. {
                for s in &mut shadows {
                    s.color = s.color.opacity(alpha);
                }
            }
            paint::drop_shadows(bounds, shadow_radius, &shadows, window);
            window.paint_quad(fill(bounds, fill_color.opacity(alpha)).corner_radii(corners));
        },
    )
    .absolute()
    .inset_0()
    .size_full()
    .into_any_element()
}
