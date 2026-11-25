//! Shared menu engine for [`crate::dropdown_menu::DropdownMenu`] and
//! `ContextMenu`: data model, gliding row highlight, animated checkmark,
//! submenus, type-ahead, roving keyboard focus, and top-layer placement.

pub(crate) mod paint_icons;
pub(crate) mod style;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::LayoutDirection;
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Context, Corners, DispatchPhase, Element, ElementId,
    Entity, FocusHandle, FontWeight, Global, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement as _, Pixels, Point, Role, SharedString, Size,
    StatefulInteractiveElement as _, Styled, Subscription, Task, WeakEntity, WeakFocusHandle,
    Window, WindowId, canvas, div, fill, point, prelude::FluentBuilder as _, px, size, svg,
};

use crate::button::ButtonIcon;
use crate::overlay::{self, OverlayAlign, OverlaySide, OverlayStack, PlaceOptions};
use crate::paint;
use crate::theme::RiffTheme;
use paint_icons::{CheckGlyph, ChevronGlyph};
use style::{
    DESC_FONT_SIZE, DESC_LINE_HEIGHT, DRIFT, EMPTY_PAD_X, EMPTY_PAD_Y, ENTER_DURATION,
    GLOW_DURATION, ICON_SIZE, ITEM_GAP, ITEM_MIN_HEIGHT, ITEM_PAD_X, KBD_FONT_SIZE, KBD_PAD_LEFT,
    LABEL_FONT_SIZE, LABEL_LINE_HEIGHT, LIST_GAP, LOGO_SIZE, Look, PANEL_MAX_WIDTH,
    PANEL_MIN_WIDTH, PANEL_PADDING, RICH_ITEM_MIN_HEIGHT, RICH_ITEM_PAD_Y, SEP_HEIGHT,
    SEP_MARGIN_X, SEP_MARGIN_Y, SUB_CLOSE_DELAY, SUB_ENTER_DURATION, SUB_OPEN_DELAY,
    SUBMENU_OFFSET, TICK_DELAY, TICK_DURATION, TITLE_FONT_SIZE, TITLE_LINE_HEIGHT,
    TITLE_PAD_BOTTOM, TITLE_PAD_TOP, TITLE_PAD_X, TYPEAHEAD_WINDOW, VIEWPORT_MARGIN,
};

/// Icon type for menu items (`IconName` or `"icons/<name>.svg"`).
pub type MenuIcon = ButtonIcon;

/// Custom brand mark builder (`14×14` slot inside the `16×16` icon column).
pub type MenuLogoFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;
pub type MenuSelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
pub type MenuHeaderFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// Choice semantics when [`MenuItem::checked`] is set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum MenuItemKind {
    #[default]
    Radio,
    Checkbox,
}

/// One row in a [`crate::dropdown_menu::DropdownMenu`] or `ContextMenu`.
#[derive(Clone)]
pub struct MenuItem {
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub icon: Option<MenuIcon>,
    pub logo: Option<MenuLogoFn>,
    pub checked: Option<bool>,
    pub kind: MenuItemKind,
    pub danger: bool,
    pub shortcut: Option<SharedString>,
    pub divider: bool,
    pub disabled: bool,
    pub keep_open: bool,
    pub submenu: Vec<MenuItem>,
    pub on_select: Option<MenuSelectHandler>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            icon: None,
            logo: None,
            checked: None,
            kind: MenuItemKind::Radio,
            danger: false,
            shortcut: None,
            divider: false,
            disabled: false,
            keep_open: false,
            submenu: Vec::new(),
            on_select: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let d = description.into();
        self.description = (!d.is_empty()).then_some(d);
        self
    }

    pub fn icon(mut self, icon: impl Into<MenuIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn logo(mut self, logo: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        self.logo = Some(Rc::new(logo));
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn kind(mut self, kind: MenuItemKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn radio(self) -> Self {
        self.kind(MenuItemKind::Radio)
    }

    pub fn checkbox(self) -> Self {
        self.kind(MenuItemKind::Checkbox)
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        let s = shortcut.into();
        self.shortcut = (!s.is_empty()).then_some(s);
        self
    }

    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = divider;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn keep_open(mut self, keep_open: bool) -> Self {
        self.keep_open = keep_open;
        self
    }

    pub fn submenu(mut self, submenu: Vec<MenuItem>) -> Self {
        self.submenu = submenu;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// How an open menu initially sets keyboard/visual focus.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MenuStart {
    /// Opened by click on trigger: container focus, no highlighted row until hovered or key pressed.
    #[default]
    Menu,
    /// Opened by Enter, Space, Down, or Shift+F10: focus checked item or first enabled item.
    First,
    /// Opened by Up: focus last enabled item.
    Last,
}

#[derive(Clone, Copy, Debug)]
struct GlowTransition {
    from_top: Pixels,
    from_height: Pixels,
    from_danger: bool,
    to_idx: usize,
    started: Instant,
}

#[derive(Default)]
struct LevelGeometry {
    list_bounds: Cell<Bounds<Pixels>>,
    panel_bounds: Cell<Bounds<Pixels>>,
    row_bounds: RefCell<Vec<Option<Bounds<Pixels>>>>,
}

struct LevelRuntime {
    items: Vec<MenuItem>,
    highlighted: Option<usize>,
    focus_visible: bool,
    opened_sub_index: Option<usize>,
    sub_focus: bool,
    typeahead_text: String,
    typeahead_at: Option<Instant>,
    enter_started: Option<Instant>,
    glow: Option<GlowTransition>,
    last_glow_rect: Option<(usize, Pixels, Pixels, bool)>,
    tick_started: HashMap<usize, Instant>,
    geom: Rc<LevelGeometry>,
}

impl LevelRuntime {
    fn new(items: Vec<MenuItem>, enter_started: Option<Instant>) -> Self {
        let mut tick_started = HashMap::new();
        if let Some(now) = enter_started {
            for (i, item) in items.iter().enumerate() {
                if item.checked == Some(true) && item.submenu.is_empty() {
                    tick_started.insert(i, now);
                }
            }
        }
        Self {
            items,
            highlighted: None,
            focus_visible: false,
            opened_sub_index: None,
            sub_focus: false,
            typeahead_text: String::new(),
            typeahead_at: None,
            enter_started,
            glow: None,
            last_glow_rect: None,
            tick_started,
            geom: Rc::new(LevelGeometry::default()),
        }
    }

    fn sync_items(&mut self, items: Vec<MenuItem>, animate: bool) {
        let now = animate.then(Instant::now);
        for (i, item) in items.iter().enumerate() {
            if item.checked == Some(true) && item.submenu.is_empty() {
                self.tick_started
                    .entry(i)
                    .or_insert_with(|| now.unwrap_or_else(Instant::now));
            } else {
                self.tick_started.remove(&i);
            }
        }
        self.items = items;
        if let Some(h) = self.highlighted
            && (h >= self.items.len() || self.items[h].disabled)
        {
            self.highlighted = None;
        }
        if let Some(s) = self.opened_sub_index
            && (s >= self.items.len() || self.items[s].submenu.is_empty())
        {
            self.opened_sub_index = None;
        }
    }

    fn enabled_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| (!item.disabled).then_some(i))
            .collect()
    }

    fn set_highlight(&mut self, next: Option<usize>, focus_visible: bool, reduce_motion: bool) {
        if self.highlighted == next && self.focus_visible == focus_visible {
            return;
        }
        if !reduce_motion
            && let (Some((prev_idx, prev_top, prev_h, prev_danger)), Some(next_idx)) =
                (self.last_glow_rect, next)
            && prev_idx != next_idx
        {
            self.glow = Some(GlowTransition {
                from_top: prev_top,
                from_height: prev_h,
                from_danger: prev_danger,
                to_idx: next_idx,
                started: Instant::now(),
            });
        } else if next.is_none() || self.last_glow_rect.is_none() || reduce_motion {
            self.glow = None;
        }
        self.highlighted = next;
        self.focus_visible = focus_visible && next.is_some();
    }
}

/// Window-wide key interceptor + single-active menu enforcement.
#[derive(Default)]
struct MenuKeyRegistry {
    interceptor: Option<Subscription>,
    controllers: HashMap<WindowId, Vec<WeakEntity<MenuController>>>,
}

impl Global for MenuKeyRegistry {}

impl MenuKeyRegistry {
    fn register(controller: &Entity<MenuController>, window: &Window, cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_none() {
            let sub = cx.intercept_keystrokes(|event, window, cx| {
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
                let handled = ctrl.update(cx, |state, cx| {
                    state.handle_key(&event.keystroke, window, cx)
                });
                if handled {
                    window.prevent_default();
                    cx.stop_propagation();
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

pub(crate) type OpenChangeFn = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

#[derive(Default)]
pub(crate) struct MenuControllerProps {
    pub controlled_open: Option<bool>,
    pub on_open_change: Option<OpenChangeFn>,
    pub on_select_close_focus: Option<WeakFocusHandle>,
    pub pinned_highlight: Option<(usize, bool)>,
    pub pinned_submenu: Option<usize>,
    pub inline: bool,
}

pub(crate) struct MenuController {
    pub panel_focus: FocusHandle,
    uncontrolled_open: bool,
    props: MenuControllerProps,
    levels: Vec<LevelRuntime>,
    previous_focus: Option<WeakFocusHandle>,
    pending_start: MenuStart,
    sub_timer_epoch: u64,
    sub_timer: Option<Task<()>>,
    pending_sub: Option<(usize, isize)>,
    was_open: bool,
}

impl MenuController {
    pub fn new(default_open: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _ = window;
        Self {
            panel_focus: cx.focus_handle().tab_stop(false),
            uncontrolled_open: default_open,
            props: MenuControllerProps::default(),
            levels: Vec::new(),
            previous_focus: None,
            pending_start: MenuStart::First,
            sub_timer_epoch: 0,
            sub_timer: None,
            pending_sub: None,
            was_open: false,
        }
    }

    pub fn effective_open(&self) -> bool {
        self.props.controlled_open.unwrap_or(self.uncontrolled_open)
    }

    fn cancel_sub_timer(&mut self) {
        self.sub_timer_epoch = self.sub_timer_epoch.wrapping_add(1);
        self.sub_timer = None;
        self.pending_sub = None;
    }

    pub fn open_with_start(
        &mut self,
        start: MenuStart,
        return_focus: Option<FocusHandle>,
        items: &[MenuItem],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_start = start;
        if let Some(f) = return_focus {
            self.previous_focus = Some(f.downgrade());
        } else if self.previous_focus.is_none() {
            self.previous_focus = window.focused(cx).map(|f| f.downgrade());
        }
        let already = self.effective_open();
        if self.props.controlled_open.is_none() {
            self.uncontrolled_open = true;
            cx.notify();
            window.refresh();
        }
        if self.levels.is_empty() || already {
            self.cancel_sub_timer();
            self.init_root_level(items, start, cx);
        }
        if !already && let Some(on_change) = self.props.on_open_change.clone() {
            on_change(true, window, cx);
        }
    }

    pub fn toggle_with_start(
        &mut self,
        start: MenuStart,
        return_focus: Option<FocusHandle>,
        items: &[MenuItem],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.effective_open() {
            self.close_internal(false, false, window, cx);
        } else {
            self.open_with_start(start, return_focus, items, window, cx);
        }
    }

    pub fn close_now(&mut self, focus_back: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.close_internal(focus_back, true, window, cx);
    }

    fn close_internal(
        &mut self,
        focus_back: bool,
        sync_refocus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_sub_timer();
        let was_open = self.effective_open();
        let prev = if focus_back {
            self.previous_focus
                .take()
                .or_else(|| self.props.on_select_close_focus.clone())
        } else {
            self.previous_focus = None;
            None
        };
        if focus_back
            && sync_refocus
            && let Some(handle) = prev.as_ref().and_then(|w| w.upgrade())
        {
            Self::refocus_handle(&handle, &self.props.on_select_close_focus, window, cx);
        }
        if self.props.controlled_open.is_none() && self.uncontrolled_open {
            self.uncontrolled_open = false;
            cx.notify();
            window.refresh();
        }
        if !self.effective_open() {
            self.levels.clear();
        }
        if was_open && let Some(on_change) = self.props.on_open_change.clone() {
            on_change(false, window, cx);
        }
        if focus_back
            && !sync_refocus
            && let Some(handle) = prev.and_then(|w| w.upgrade())
        {
            let fallback = self.props.on_select_close_focus.clone();
            window.defer(cx, move |window, cx| {
                if window.focused(cx).is_none() {
                    Self::refocus_handle(&handle, &fallback, window, cx);
                }
            });
        }
    }

    fn refocus_handle(
        handle: &FocusHandle,
        wrapper_fallback: &Option<WeakFocusHandle>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let is_wrapper = wrapper_fallback
            .as_ref()
            .and_then(|w| w.upgrade())
            .is_some_and(|w| &w == handle);
        handle.focus(window, cx);
        if is_wrapper {
            window.focus_next(cx);
            if !handle.contains_focused(window, cx) {
                handle.focus(window, cx);
            }
        }
    }

    fn init_root_level(&mut self, items: &[MenuItem], start: MenuStart, cx: &App) {
        let enter = (!cx.reduce_motion()).then(Instant::now);
        let mut root = LevelRuntime::new(items.to_vec(), enter);
        let enabled = root.enabled_indices();
        match start {
            MenuStart::Menu => {
                root.highlighted = None;
                root.focus_visible = false;
            }
            MenuStart::First => {
                let checked = enabled
                    .iter()
                    .copied()
                    .find(|&i| items[i].checked == Some(true));
                root.highlighted = checked.or_else(|| enabled.first().copied());
                root.focus_visible = root.highlighted.is_some();
            }
            MenuStart::Last => {
                root.highlighted = enabled.last().copied();
                root.focus_visible = root.highlighted.is_some();
            }
        }
        self.levels = vec![root];
    }

    pub fn sync_frame(
        &mut self,
        controller: &Entity<Self>,
        props: MenuControllerProps,
        items: &[MenuItem],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let was_inline = self.props.inline;
        let pinned_highlight = props.pinned_highlight;
        let pinned_submenu = props.pinned_submenu;
        self.props = props;
        let open = self.effective_open();
        if !open {
            self.cancel_sub_timer();
            self.levels.clear();
            if self.was_open && !was_inline {
                OverlayStack::remove(window, controller.entity_id(), cx);
            }
            self.was_open = false;
            return false;
        }

        if self.levels.is_empty() {
            let start = if self.props.inline {
                MenuStart::Menu
            } else {
                self.pending_start
            };
            self.init_root_level(items, start, cx);
        } else {
            self.levels[0].sync_items(items.to_vec(), !self.props.inline && !cx.reduce_motion());
            // Keep nested submenu item lists aligned with updated root items.
            for depth in 1..self.levels.len() {
                let parent_idx = self.levels[depth - 1].opened_sub_index;
                if let Some(p) = parent_idx
                    && p < self.levels[depth - 1].items.len()
                    && !self.levels[depth - 1].items[p].submenu.is_empty()
                {
                    let sub_items = self.levels[depth - 1].items[p].submenu.clone();
                    self.levels[depth]
                        .sync_items(sub_items, !self.props.inline && !cx.reduce_motion());
                } else {
                    self.levels.truncate(depth);
                    break;
                }
            }
        }

        if let Some((idx, fv)) = pinned_highlight
            && let Some(root) = self.levels.first_mut()
            && idx < root.items.len()
            && !root.items[idx].disabled
        {
            root.highlighted = Some(idx);
            root.focus_visible = fv;
        }

        if let Some(sub_idx) = pinned_submenu
            && self.levels.len() == 1
            && sub_idx < self.levels[0].items.len()
            && !self.levels[0].items[sub_idx].submenu.is_empty()
        {
            let sub_items = self.levels[0].items[sub_idx].submenu.clone();
            self.levels[0].opened_sub_index = Some(sub_idx);
            self.levels.push(LevelRuntime::new(sub_items, None));
        }

        if !self.was_open && !self.props.inline {
            MenuKeyRegistry::register(controller, window, cx);
            OverlayStack::push(window, controller.entity_id(), cx);
            self.panel_focus.focus(window, cx);
        }
        self.was_open = true;
        true
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    fn open_submenu(
        &mut self,
        depth: usize,
        index: usize,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_sub_timer();
        if depth >= self.levels.len() || index >= self.levels[depth].items.len() {
            return;
        }
        let sub_items = self.levels[depth].items[index].submenu.clone();
        if sub_items.is_empty() || self.levels[depth].items[index].disabled {
            return;
        }
        let already = self.levels[depth].opened_sub_index == Some(index);
        self.levels[depth].opened_sub_index = Some(index);
        self.levels[depth].sub_focus = focus;
        if already && self.levels.len() > depth + 1 {
            if focus {
                let child = &mut self.levels[depth + 1];
                let first = child.enabled_indices().first().copied();
                child.set_highlight(first, true, cx.reduce_motion());
                cx.notify();
                window.refresh();
            }
            return;
        }
        self.levels.truncate(depth + 1);
        let enter = (!self.props.inline && !cx.reduce_motion()).then(Instant::now);
        let mut child = LevelRuntime::new(sub_items, enter);
        if focus {
            child.highlighted = child.enabled_indices().first().copied();
            child.focus_visible = child.highlighted.is_some();
        }
        self.levels.push(child);
        cx.notify();
        window.refresh();
    }

    fn close_submenus_from(
        &mut self,
        from_depth: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_sub_timer();
        if from_depth == 0 {
            return;
        }
        if self.levels.len() > from_depth {
            if let Some(parent) = self.levels.get_mut(from_depth - 1) {
                parent.opened_sub_index = None;
                parent.sub_focus = false;
            }
            self.levels.truncate(from_depth);
            cx.notify();
            window.refresh();
        }
    }

    fn schedule_sub_open(
        &mut self,
        depth: usize,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_sub == Some((depth, index as isize)) {
            return;
        }
        self.cancel_sub_timer();
        let epoch = self.sub_timer_epoch;
        self.pending_sub = Some((depth, index as isize));
        let weak = cx.entity().downgrade();
        self.sub_timer = Some(window.spawn(cx, async move |async_window| {
            async_window
                .background_executor()
                .timer(SUB_OPEN_DELAY)
                .await;
            _ = async_window.update(|window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.sub_timer_epoch == epoch {
                        this.open_submenu(depth, index, false, window, cx);
                    }
                });
            });
        }));
    }

    fn schedule_sub_close(
        &mut self,
        from_depth: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_sub == Some((from_depth, -1)) {
            return;
        }
        self.cancel_sub_timer();
        let epoch = self.sub_timer_epoch;
        self.pending_sub = Some((from_depth, -1));
        let weak = cx.entity().downgrade();
        self.sub_timer = Some(window.spawn(cx, async move |async_window| {
            async_window
                .background_executor()
                .timer(SUB_CLOSE_DELAY)
                .await;
            _ = async_window.update(|window, cx| {
                _ = weak.update(cx, |this, cx| {
                    if this.sub_timer_epoch == epoch {
                        this.close_submenus_from(from_depth, window, cx);
                    }
                });
            });
        }));
    }

    fn activate(
        &mut self,
        depth: usize,
        index: usize,
        keyboard: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(level) = self.levels.get(depth) else {
            return;
        };
        let Some(item) = level.items.get(index).cloned() else {
            return;
        };
        if item.disabled {
            return;
        }
        if !item.submenu.is_empty() {
            if !keyboard && level.opened_sub_index == Some(index) && !level.sub_focus {
                self.close_submenus_from(depth + 1, window, cx);
            } else {
                self.open_submenu(depth, index, keyboard, window, cx);
            }
            return;
        }
        let keep_open = item.keep_open;
        let pre_focus = window.focused(cx);
        if let Some(handler) = item.on_select {
            handler(window, cx);
        }
        let handler_moved_focus = window.focused(cx) != pre_focus;
        if !keep_open {
            self.close_internal(!handler_moved_focus, true, window, cx);
        } else {
            cx.notify();
            window.refresh();
        }
    }

    fn handle_key(
        &mut self,
        keystroke: &gpui_kit::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.levels.is_empty() {
            return false;
        }
        let active_depth = self.levels.len() - 1;
        let key = keystroke.key.as_str();

        if key == "tab" {
            self.close_now(true, window, cx);
            // Let GPUI advance focus from the restored trigger.
            return false;
        }
        if key == "escape" && keystroke.modifiers == Modifiers::default() {
            if active_depth > 0 {
                self.close_submenus_from(active_depth, window, cx);
                if let Some(parent) = self.levels.last_mut() {
                    parent.focus_visible = parent.highlighted.is_some();
                }
            } else {
                self.close_now(true, window, cx);
            }
            return true;
        }
        if key == "left" && keystroke.modifiers == Modifiers::default() {
            if active_depth > 0 {
                self.close_submenus_from(active_depth, window, cx);
                if let Some(parent) = self.levels.last_mut() {
                    parent.focus_visible = parent.highlighted.is_some();
                }
                return true;
            }
            return false;
        }
        if key == "right" && keystroke.modifiers == Modifiers::default() {
            let sub_candidate = {
                let lvl = &self.levels[active_depth];
                lvl.highlighted.and_then(|idx| {
                    (!lvl.items[idx].disabled && !lvl.items[idx].submenu.is_empty()).then_some(idx)
                })
            };
            if let Some(idx) = sub_candidate {
                self.open_submenu(active_depth, idx, true, window, cx);
                return true;
            }
            return false;
        }

        let enabled = self.levels[active_depth].enabled_indices();
        let mut move_to = |this: &mut Self, next: Option<usize>| {
            let reduce = cx.reduce_motion();
            this.cancel_sub_timer();
            this.levels[active_depth].set_highlight(next, true, reduce);
            this.levels[active_depth].opened_sub_index = None;
            this.levels.truncate(active_depth + 1);
            cx.notify();
            window.refresh();
        };

        match key {
            "down" if keystroke.modifiers == Modifiers::default() => {
                if !enabled.is_empty() {
                    let cur = self.levels[active_depth].highlighted;
                    let next = match cur.and_then(|c| enabled.iter().position(|&x| x == c)) {
                        Some(pos) => enabled[(pos + 1) % enabled.len()],
                        None => enabled[0],
                    };
                    move_to(self, Some(next));
                }
                return true;
            }
            "up" if keystroke.modifiers == Modifiers::default() => {
                if !enabled.is_empty() {
                    let cur = self.levels[active_depth].highlighted;
                    let next = match cur.and_then(|c| enabled.iter().position(|&x| x == c)) {
                        Some(pos) => enabled[(pos + enabled.len() - 1) % enabled.len()],
                        None => enabled[enabled.len() - 1],
                    };
                    move_to(self, Some(next));
                }
                return true;
            }
            "home" | "pageup" => {
                if let Some(&first) = enabled.first() {
                    move_to(self, Some(first));
                }
                return true;
            }
            "end" | "pagedown" => {
                if let Some(&last) = enabled.last() {
                    move_to(self, Some(last));
                }
                return true;
            }
            "enter" if keystroke.modifiers == Modifiers::default() => {
                if let Some(idx) = self.levels[active_depth].highlighted {
                    self.activate(active_depth, idx, true, window, cx);
                }
                return true;
            }
            "space" if keystroke.modifiers == Modifiers::default() => {
                let in_typeahead = self.levels[active_depth]
                    .typeahead_at
                    .is_some_and(|at| at.elapsed() < TYPEAHEAD_WINDOW);
                if in_typeahead {
                    self.typeahead(active_depth, " ", window, cx);
                    return true;
                }
                if let Some(idx) = self.levels[active_depth].highlighted {
                    self.activate(active_depth, idx, true, window, cx);
                }
                return true;
            }
            _ => {}
        }

        if !keystroke.modifiers.control && !keystroke.modifiers.platform && !keystroke.modifiers.alt
        {
            let ch = keystroke
                .key_char
                .as_deref()
                .filter(|s| s.chars().count() == 1)
                .or_else(|| (key.chars().count() == 1).then_some(key));
            if let Some(ch) = ch {
                self.typeahead(active_depth, ch, window, cx);
                return true;
            }
        }
        false
    }

    fn typeahead(&mut self, depth: usize, ch: &str, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduce = cx.reduce_motion();
        let lvl = &mut self.levels[depth];
        let expired = lvl
            .typeahead_at
            .is_none_or(|at| now.saturating_duration_since(at) > TYPEAHEAD_WINDOW);
        if expired {
            lvl.typeahead_text.clear();
        }
        lvl.typeahead_text.push_str(ch);
        lvl.typeahead_at = Some(now);
        let q = lvl.typeahead_text.to_lowercase();
        let q_chars = q.chars().count();
        let enabled = lvl.enabled_indices();
        if enabled.is_empty() {
            return;
        }
        let at_pos = lvl
            .highlighted
            .and_then(|h| enabled.iter().position(|&x| x == h));
        let start_pos = if q_chars == 1 {
            at_pos.map_or(0, |p| p + 1)
        } else {
            at_pos.unwrap_or(0)
        };
        let mut matched = None;
        for step in 0..enabled.len() {
            let idx = enabled[(start_pos + step) % enabled.len()];
            if lvl.items[idx].label.to_lowercase().starts_with(&q) {
                matched = Some(idx);
                break;
            }
        }
        if let Some(idx) = matched {
            lvl.set_highlight(Some(idx), true, reduce);
            cx.notify();
            window.refresh();
        }
    }

    fn enter_progress(
        &self,
        depth: usize,
        freeze_at: Option<Duration>,
        window: &mut Window,
        cx: &App,
    ) -> f32 {
        if self.props.inline || cx.reduce_motion() {
            return 1.;
        }
        let Some(started) = self.levels.get(depth).and_then(|l| l.enter_started) else {
            return 1.;
        };
        let dur = if depth == 0 {
            ENTER_DURATION
        } else {
            SUB_ENTER_DURATION
        };
        let elapsed = freeze_at.unwrap_or_else(|| started.elapsed());
        if elapsed >= dur {
            return 1.;
        }
        window.request_animation_frame();
        RiffTheme::global(cx)
            .ease
            .sample((elapsed.as_secs_f32() / dur.as_secs_f32()).clamp(0., 1.))
    }

    fn tick_progress(
        &self,
        depth: usize,
        idx: usize,
        freeze_at: Option<Duration>,
        window: &mut Window,
        cx: &App,
    ) -> f32 {
        if self.props.inline || cx.reduce_motion() {
            return 1.;
        }
        let Some(started) = self
            .levels
            .get(depth)
            .and_then(|l| l.tick_started.get(&idx).copied())
        else {
            return 1.;
        };
        let elapsed = freeze_at.unwrap_or_else(|| started.elapsed());
        if elapsed <= TICK_DELAY {
            window.request_animation_frame();
            return 0.;
        }
        let active = elapsed - TICK_DELAY;
        if active >= TICK_DURATION {
            return 1.;
        }
        window.request_animation_frame();
        RiffTheme::global(cx)
            .ease
            .sample((active.as_secs_f32() / TICK_DURATION.as_secs_f32()).clamp(0., 1.))
    }
}

#[derive(Clone)]
pub(crate) enum MenuRootAnchor {
    Side {
        trigger: Rc<Cell<Bounds<Pixels>>>,
        side: OverlaySide,
        align: OverlayAlign,
        offset: Pixels,
    },
    Cursor {
        point: Rc<Cell<Point<Pixels>>>,
    },
}

pub(crate) struct MenuPanelSpec {
    pub id: ElementId,
    pub depth: usize,
    pub title: Option<SharedString>,
    pub empty: SharedString,
    pub header: Option<MenuHeaderFn>,
    pub width: Option<Pixels>,
    pub aria_label: Option<SharedString>,
    pub anchor: MenuRootAnchor,
    pub inline: bool,
    pub freeze_at: Option<Duration>,
    pub controller: Entity<MenuController>,
}

pub(crate) struct AnchoredMenuPanel {
    pub spec: Option<MenuPanelSpec>,
    id: ElementId,
}

impl AnchoredMenuPanel {
    pub fn new(id: ElementId, spec: MenuPanelSpec) -> Self {
        Self {
            spec: Some(spec),
            id,
        }
    }
}

impl IntoElement for AnchoredMenuPanel {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

pub(crate) struct MenuPanelPrepaint {
    surface: AnyElement,
    depth: usize,
    panel_rect: Bounds<Pixels>,
    row_hitboxes: Vec<Hitbox>,
    controller: Entity<MenuController>,
    inline: bool,
    trigger_rect: Option<Bounds<Pixels>>,
}

impl Element for AnchoredMenuPanel {
    type RequestLayoutState = ();
    type PrepaintState = MenuPanelPrepaint;

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
    ) -> MenuPanelPrepaint {
        let spec = self.spec.take().expect("prepaint called once");
        let depth = spec.depth;
        let inline = spec.inline;
        let controller = spec.controller.clone();
        let viewport = Bounds::new(point(px(0.), px(0.)), window.viewport_size());
        let max_vw = (viewport.size.width - VIEWPORT_MARGIN * 2.).max(px(80.));
        let min_w = if spec.width.is_some() {
            px(0.)
        } else {
            PANEL_MIN_WIDTH.min(max_vw)
        };
        let (max_w, exact_w) = match spec.width {
            Some(w) => {
                let capped = if inline { w } else { w.min(max_vw) };
                (capped, Some(capped))
            }
            None => {
                let cap = if inline {
                    PANEL_MAX_WIDTH
                } else {
                    PANEL_MAX_WIDTH.min(max_vw)
                };
                (cap, None)
            }
        };

        let progress = controller
            .read(cx)
            .enter_progress(depth, spec.freeze_at, window, cx);
        let geom = controller
            .read(cx)
            .levels
            .get(depth)
            .map(|l| l.geom.clone())
            .unwrap_or_else(|| Rc::new(LevelGeometry::default()));

        // Determine parent row rectangle if this is a submenu.
        let (anchor_rect, preferred_side, preferred_align, gap) = if depth == 0 {
            match &spec.anchor {
                MenuRootAnchor::Side {
                    trigger,
                    side,
                    align,
                    offset,
                } => (trigger.get(), *side, *align, *offset),
                MenuRootAnchor::Cursor { point } => (
                    Bounds::new(point.get(), size(px(0.), px(0.))),
                    OverlaySide::Bottom,
                    OverlayAlign::Start,
                    px(0.),
                ),
            }
        } else {
            let parent_row = controller
                .read(cx)
                .levels
                .get(depth - 1)
                .and_then(|p| {
                    let idx = p.opened_sub_index?;
                    p.geom.row_bounds.borrow().get(idx).copied().flatten()
                })
                .unwrap_or_default();
            (
                parent_row.dilate(px(4.)),
                OverlaySide::Right,
                OverlayAlign::Start,
                SUBMENU_OFFSET,
            )
        };

        let (mut surface, panel_size) = if let Some(w) = exact_w {
            let mut s = build_level_panel(
                &spec,
                depth,
                min_w,
                max_w,
                Some(w),
                true,
                progress,
                geom.clone(),
                window,
                cx,
            );
            let sz = s.layout_as_root(
                size(AvailableSpace::Definite(w), AvailableSpace::MinContent),
                window,
                cx,
            );
            (s, sz)
        } else {
            let mut probe = build_level_panel(
                &spec,
                depth,
                min_w,
                max_w,
                None,
                false,
                progress,
                geom.clone(),
                window,
                cx,
            );
            let probe_size = probe.layout_as_root(
                size(AvailableSpace::MaxContent, AvailableSpace::MinContent),
                window,
                cx,
            );
            let resolved_w = probe_size.width.max(min_w).min(max_w);
            let mut s = build_level_panel(
                &spec,
                depth,
                min_w,
                max_w,
                Some(resolved_w),
                true,
                progress,
                geom.clone(),
                window,
                cx,
            );
            let sz = s.layout_as_root(
                size(
                    AvailableSpace::Definite(resolved_w),
                    AvailableSpace::MinContent,
                ),
                window,
                cx,
            );
            (s, sz)
        };

        let place_viewport = if inline {
            Bounds::new(
                point(px(-1_000_000.), px(-1_000_000.)),
                size(px(2_000_000.), px(2_000_000.)),
            )
        } else {
            viewport
        };

        let rtl = LayoutDirection::global(cx).is_rtl();
        let (origin, side) = if depth == 0 {
            let placed = overlay::place_side_with(
                anchor_rect,
                panel_size,
                place_viewport,
                preferred_side,
                preferred_align,
                gap,
                VIEWPORT_MARGIN,
                rtl,
                PlaceOptions {
                    flip_align: true,
                    flip_align_beside: false,
                    prefer_bottom_fallback: true,
                },
            );
            (placed.origin, placed.side)
        } else {
            place_submenu(anchor_rect, panel_size, place_viewport, VIEWPORT_MARGIN)
        };

        let drift = if depth == 0 {
            side.enter_drift(progress, DRIFT)
        } else {
            point(px(0.), px(0.))
        };
        let final_origin = origin + drift;
        let panel_rect = Bounds::new(final_origin, panel_size);
        geom.panel_bounds.set(panel_rect);

        surface.prepaint_at(final_origin, window, cx);

        let row_hitboxes = if !inline {
            let rows = geom.row_bounds.borrow();
            rows.iter()
                .map(|r| window.insert_hitbox(r.unwrap_or_default(), HitboxBehavior::Normal))
                .collect()
        } else {
            Vec::new()
        };

        let trigger_rect = match &spec.anchor {
            MenuRootAnchor::Side { trigger, .. } if depth == 0 => Some(trigger.get()),
            _ => None,
        };

        MenuPanelPrepaint {
            surface,
            depth,
            panel_rect,
            row_hitboxes,
            controller,
            inline,
            trigger_rect,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut MenuPanelPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        prepaint.surface.paint(window, cx);
        if prepaint.inline {
            return;
        }

        let depth = prepaint.depth;
        let controller = prepaint.controller.clone();
        let row_hitboxes = prepaint.row_hitboxes.clone();
        let panel_rect = prepaint.panel_rect;

        // Pointer move tracking over rows and panels.
        window.on_mouse_event({
            let controller = controller.clone();
            let row_hitboxes = row_hitboxes.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || window.last_input_was_touch() {
                    return;
                }
                let hovered_row = row_hitboxes.iter().position(|hb| hb.is_hovered(window));
                let inside_panel = panel_rect.contains(&event.position);
                let deep_inside = controller
                    .read(cx)
                    .levels
                    .iter()
                    .rposition(|l| l.geom.panel_bounds.get().contains(&event.position));
                if deep_inside.is_some_and(|d| d > depth) {
                    // Pointer is inside a child submenu -> keep parent open and don't close!
                    if depth == controller.read(cx).levels.len().saturating_sub(2) {
                        controller.update(cx, |this, _| this.cancel_sub_timer());
                    }
                    return;
                }

                controller.update(cx, |this, cx| {
                    if depth >= this.levels.len() {
                        return;
                    }
                    let reduce = cx.reduce_motion();
                    if let Some(i) = hovered_row {
                        if i >= this.levels[depth].items.len() {
                            return;
                        }
                        if this.levels[depth].items[i].disabled {
                            if !this.levels[depth].focus_visible {
                                this.levels[depth].set_highlight(None, false, reduce);
                                cx.notify();
                            }
                            return;
                        }
                        this.levels[depth].set_highlight(Some(i), false, reduce);
                        let has_sub = !this.levels[depth].items[i].submenu.is_empty();
                        let sub_open = this.levels[depth].opened_sub_index;
                        if has_sub {
                            if sub_open == Some(i) {
                                this.cancel_sub_timer();
                            } else {
                                this.schedule_sub_open(depth, i, window, cx);
                            }
                        } else if sub_open.is_some() {
                            this.schedule_sub_close(depth + 1, window, cx);
                        } else {
                            this.cancel_sub_timer();
                        }
                        cx.notify();
                    } else if !inside_panel
                        && depth + 1 == this.levels.len()
                        && !this.levels[depth].focus_visible
                        && this.levels[depth].highlighted.is_some()
                    {
                        this.levels[depth].set_highlight(None, false, reduce);
                        cx.notify();
                    }
                });
            }
        });

        // Click on item rows (handled by deepest panel containing cursor) or outside press.
        let trigger_rect = prepaint.trigger_rect;
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture {
                return;
            }
            let deep_inside = controller
                .read(cx)
                .levels
                .iter()
                .rposition(|l| l.geom.panel_bounds.get().contains(&event.position));
            if let Some(d) = deep_inside {
                if event.button == MouseButton::Left && d == depth {
                    if let Some(i) = row_hitboxes.iter().position(|hb| hb.is_hovered(window)) {
                        cx.stop_propagation();
                        controller.update(cx, |this, cx| {
                            this.activate(depth, i, false, window, cx);
                        });
                    } else {
                        cx.stop_propagation();
                    }
                } else if event.button == MouseButton::Right {
                    cx.stop_propagation();
                }
                return;
            }
            if depth == 0 && matches!(event.button, MouseButton::Left | MouseButton::Right) {
                if trigger_rect.is_some_and(|r| r.contains(&event.position)) {
                    return;
                }
                if OverlayStack::has_descendant_after(window, controller.entity_id(), cx) {
                    return;
                }
                controller.update(cx, |this, cx| {
                    this.close_internal(false, true, window, cx);
                });
            }
        });
    }
}

fn place_submenu(
    trigger: Bounds<Pixels>,
    panel_size: Size<Pixels>,
    viewport: Bounds<Pixels>,
    margin: Pixels,
) -> (Point<Pixels>, OverlaySide) {
    let vw = viewport.right();
    let vh = viewport.bottom();
    let w = panel_size.width;
    let h = panel_size.height;
    let room_right = vw - trigger.right() - SUBMENU_OFFSET - margin;
    let room_left = trigger.left() - viewport.left() - SUBMENU_OFFSET - margin;

    let mut side = OverlaySide::Right;
    if room_right < w && room_left > room_right {
        side = OverlaySide::Left;
    }
    let room = if side == OverlaySide::Right {
        room_right
    } else {
        room_left
    };
    if room < w {
        // Narrow viewport fallback: below the row, indented by 16px.
        let top = trigger.bottom() - px(2.);
        let min_x = viewport.left() + margin;
        let max_x = (vw - margin - w).max(min_x);
        let left = (trigger.left() + px(16.)).max(min_x).min(max_x);
        return (point(left.round(), top.round()), OverlaySide::Bottom);
    }

    let left = match side {
        OverlaySide::Right => trigger.right() + SUBMENU_OFFSET,
        OverlaySide::Left => trigger.left() - SUBMENU_OFFSET - w,
        _ => unreachable!(),
    };
    let min_y = viewport.top() + margin;
    let max_y = (vh - margin - h).max(min_y);
    let top = trigger.top().max(min_y).min(max_y);
    (point(left.round(), top.round()), side)
}

#[allow(clippy::too_many_arguments)]
fn build_level_panel(
    spec: &MenuPanelSpec,
    depth: usize,
    min_w: Pixels,
    max_w: Pixels,
    exact_w: Option<Pixels>,
    definite_width: bool,
    progress: f32,
    geom: Rc<LevelGeometry>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = RiffTheme::global(cx).clone();
    let look = Look::of(&theme);
    let alpha = progress.clamp(0., 1.);
    let (panel_focus, items, highlighted, focus_visible, opened_sub_index, glow_trans) = {
        let ctrl = spec.controller.read(cx);
        let level = &ctrl.levels[depth];
        (
            ctrl.panel_focus.clone(),
            level.items.clone(),
            level.highlighted,
            level.focus_visible,
            level.opened_sub_index,
            level.glow,
        )
    };

    // Build rows first.
    let mut rows_el = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        if item.divider && i > 0 {
            rows_el.push(
                div()
                    .h(SEP_HEIGHT)
                    .my(SEP_MARGIN_Y)
                    .mx(SEP_MARGIN_X)
                    .bg(look.line)
                    .into_any_element(),
            );
        }

        let choice = item.checked.is_some() && item.submenu.is_empty();
        let role = if !choice {
            Role::MenuItem
        } else if item.kind == MenuItemKind::Checkbox {
            Role::MenuItemCheckBox
        } else {
            Role::MenuItemRadio
        };

        let is_highlighted = highlighted == Some(i);
        let is_open_sub = opened_sub_index == Some(i);
        let active = is_highlighted || is_open_sub;
        let rich = item.description.is_some();

        let text_color = if item.disabled {
            look.faint
        } else if item.danger {
            look.danger
        } else if active || item.checked == Some(true) {
            look.ink
        } else {
            look.text
        };

        let icon_color = if item.disabled {
            look.icon.opacity(0.5)
        } else if item.danger {
            look.danger
        } else if active {
            look.ink
        } else {
            look.icon
        };

        let check_el = if choice && item.checked == Some(true) {
            let tick_p =
                spec.controller
                    .read(cx)
                    .tick_progress(depth, i, spec.freeze_at, window, cx);
            Some(
                CheckGlyph::new(
                    (spec.id.clone(), format!("tick-{depth}-{i}")),
                    look.ink,
                    tick_p,
                )
                .into_any_element(),
            )
        } else if choice {
            Some(
                div()
                    .size(style::ACCESSORY_SIZE)
                    .flex_none()
                    .into_any_element(),
            )
        } else {
            None
        };

        let sub_el = (!item.submenu.is_empty()).then(|| {
            ChevronGlyph::new((spec.id.clone(), format!("chev-{depth}-{i}")), look.faint)
                .into_any_element()
        });

        let has_icon_slot = item.logo.is_some() || item.icon.is_some();
        let icon_slot = if let Some(logo_fn) = &item.logo {
            Some(
                div()
                    .size(ICON_SIZE)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(LOGO_SIZE)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(logo_fn(window, cx)),
                    )
                    .into_any_element(),
            )
        } else {
            item.icon.as_ref().map(|ic| {
                div()
                    .size(ICON_SIZE)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(ic.0.clone())
                            .size(ICON_SIZE)
                            .text_color(icon_color),
                    )
                    .into_any_element()
            })
        };

        let label_weight = if item.checked == Some(true) {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        };

        let row_geom = geom.clone();
        let row = div()
            .id((spec.id.clone(), format!("row-{depth}-{i}")))
            .role(role)
            .aria_label(item.label.clone())
            .relative()
            .flex()
            .items_center()
            .gap(ITEM_GAP)
            .min_w_0()
            .w_full()
            .when(rich, |d| d.min_h(RICH_ITEM_MIN_HEIGHT).py(RICH_ITEM_PAD_Y))
            .when(!rich, |d| d.min_h(ITEM_MIN_HEIGHT))
            .px(ITEM_PAD_X)
            .rounded(look.row_radius)
            .text_color(text_color)
            .when(focus_visible && is_highlighted && !item.disabled, |d| {
                d.border_2().border_color(look.focus)
            })
            .child(
                canvas(
                    move |bounds, _, _| {
                        let mut rows = row_geom.row_bounds.borrow_mut();
                        if rows.len() <= i {
                            rows.resize(i + 1, None);
                        }
                        rows[i] = Some(bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .when(has_icon_slot, |d| d.children(icon_slot))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .when(definite_width, |d| d.flex_1())
                    .when(!definite_width, |d| d.flex_grow(1.))
                    .min_w_0()
                    .gap(px(1.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(LABEL_FONT_SIZE)
                            .line_height(LABEL_LINE_HEIGHT)
                            .font_weight(label_weight)
                            .text_color(text_color)
                            .child(item.label.clone()),
                    )
                    .when_some(item.description.clone(), |d, desc| {
                        d.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(DESC_FONT_SIZE)
                                .line_height(DESC_LINE_HEIGHT)
                                .font_weight(FontWeight::NORMAL)
                                .text_color(if item.disabled { look.faint } else { look.icon })
                                .child(desc),
                        )
                    }),
            )
            .when_some(item.shortcut.clone(), |d, kbd| {
                let kbd_el = paint::tracked_text_el(
                    kbd,
                    theme.font_family.clone(),
                    KBD_FONT_SIZE,
                    LABEL_LINE_HEIGHT,
                    FontWeight::NORMAL,
                    KBD_FONT_SIZE * 0.04,
                    look.faint,
                    window,
                );
                d.child(div().flex_none().pl(KBD_PAD_LEFT).child(kbd_el))
            })
            .when_some(sub_el, |d, el| d.child(el))
            .when_some(check_el, |d, el| d.child(el));

        #[cfg(test)]
        let row = {
            use gpui_kit::TestSupportExt as _;
            row.test_support()
        };

        rows_el.push(row.into_any_element());
    }

    let menu_label = if depth == 0 {
        spec.title.clone().or_else(|| spec.aria_label.clone())
    } else {
        None
    };

    let glow_geom = geom.clone();
    let glow_controller = spec.controller.clone();
    let glow_look = look.clone();
    let glow_highlight = highlighted.or(opened_sub_index);
    let glow_freeze = spec.freeze_at;
    let glow_inline = spec.inline;

    let glow_layer = canvas(
        move |bounds, _, _| {
            glow_geom.list_bounds.set(bounds);
            bounds
        },
        move |bounds, _, window, cx| {
            let Some(idx) = glow_highlight else {
                return;
            };
            let target_rect = geom.row_bounds.borrow().get(idx).copied().flatten();
            let Some(target_rect) = target_rect else {
                return;
            };
            let danger = glow_controller
                .read(cx)
                .levels
                .get(depth)
                .and_then(|l| l.items.get(idx))
                .is_some_and(|i| i.danger);
            let rel_top = target_rect.top() - bounds.top();
            let target_h = target_rect.size.height;

            glow_controller.update(cx, |this, _| {
                if let Some(l) = this.levels.get_mut(depth) {
                    l.last_glow_rect = Some((idx, rel_top, target_h, danger));
                }
            });

            let (draw_top, draw_h, draw_color) = if let Some(trans) = glow_trans
                && trans.to_idx == idx
                && !glow_inline
                && !cx.reduce_motion()
            {
                let elapsed = glow_freeze.unwrap_or_else(|| trans.started.elapsed());
                if elapsed < GLOW_DURATION {
                    window.request_animation_frame();
                    let t = RiffTheme::global(cx).ease.sample(
                        (elapsed.as_secs_f32() / GLOW_DURATION.as_secs_f32()).clamp(0., 1.),
                    );
                    let top = trans.from_top + (rel_top - trans.from_top) * t;
                    let h = trans.from_height + (target_h - trans.from_height) * t;
                    let from_c = if trans.from_danger {
                        glow_look.danger_glow
                    } else {
                        glow_look.glow
                    };
                    let to_c = if danger {
                        glow_look.danger_glow
                    } else {
                        glow_look.glow
                    };
                    (top, h, paint::mix(from_c, to_c, t))
                } else {
                    let c = if danger {
                        glow_look.danger_glow
                    } else {
                        glow_look.glow
                    };
                    (rel_top, target_h, c)
                }
            } else {
                let c = if danger {
                    glow_look.danger_glow
                } else {
                    glow_look.glow
                };
                (rel_top, target_h, c)
            };

            let glow_bounds = Bounds::new(
                point(bounds.left(), bounds.top() + draw_top),
                size(bounds.size.width, draw_h),
            );
            window.paint_quad(
                fill(glow_bounds, draw_color).corner_radii(Corners::all(glow_look.row_radius)),
            );
        },
    )
    .absolute()
    .inset_0()
    .size_full();

    let list_el = div()
        .id((spec.id.clone(), format!("list-inner-{depth}")))
        .role(Role::Menu)
        .when_some(menu_label, |d, l| d.aria_label(l))
        .relative()
        .flex()
        .flex_col()
        .w_full()
        .gap(LIST_GAP)
        .child(glow_layer)
        .when(items.is_empty(), |d| {
            d.child(
                div()
                    .py(EMPTY_PAD_Y)
                    .px(EMPTY_PAD_X)
                    .text_size(LABEL_FONT_SIZE)
                    .line_height(LABEL_LINE_HEIGHT)
                    .text_color(look.faint)
                    .child(spec.empty.clone()),
            )
        })
        .children(rows_el);

    #[cfg(test)]
    let list_el = {
        use gpui_kit::TestSupportExt as _;
        list_el.test_support().into_any_element()
    };
    #[cfg(not(test))]
    let list_el = list_el.into_any_element();

    let viewport_h = if spec.inline {
        None
    } else {
        Some((window.viewport_size().height - VIEWPORT_MARGIN * 2.).max(px(120.)))
    };

    let header_el = (depth == 0)
        .then(|| spec.header.as_ref().map(|f| f(window, cx)))
        .flatten();

    let box_el = div()
        .id((spec.id.clone(), format!("panel-{depth}")))
        .font_family(theme.font_family)
        .when(depth == 0 && !spec.inline, |d| {
            d.track_focus(&panel_focus).occlude()
        })
        .when(depth > 0 && !spec.inline, |d| d.occlude())
        .min_w(min_w)
        .when_some(exact_w, |d, w| d.w(w))
        .max_w(max_w)
        .when_some(viewport_h, |d, h| d.max_h(h))
        .p(PANEL_PADDING)
        .flex()
        .flex_col()
        .text_size(LABEL_FONT_SIZE)
        .line_height(LABEL_LINE_HEIGHT)
        .text_color(look.ink)
        .when(alpha < 1., |d| d.opacity(alpha))
        .when_some(header_el, |d, h| d.child(h))
        .when(depth == 0, |d| {
            d.when_some(spec.title.clone(), |d, t| {
                d.child(
                    div()
                        .pt(TITLE_PAD_TOP)
                        .pb(TITLE_PAD_BOTTOM)
                        .px(TITLE_PAD_X)
                        .text_size(TITLE_FONT_SIZE)
                        .line_height(TITLE_LINE_HEIGHT)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(look.icon)
                        .child(t),
                )
            })
        })
        .child(list_el);

    #[cfg(test)]
    let inner = {
        use gpui_kit::TestSupportExt as _;
        box_el.test_support().into_any_element()
    };
    #[cfg(not(test))]
    let inner = box_el.into_any_element();

    MenuPanelSurface {
        id: (spec.id.clone(), format!("surf-{depth}")).into(),
        inner,
        look,
        alpha,
    }
    .into_any_element()
}

struct MenuPanelSurface {
    id: ElementId,
    inner: AnyElement,
    look: Look,
    alpha: f32,
}

impl IntoElement for MenuPanelSurface {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for MenuPanelSurface {
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

/// Keeps clippy clean about unused imports in `menu`.
#[allow(dead_code)]
fn _touch_unused(_: WeakEntity<MenuController>, _: WindowId) {}
