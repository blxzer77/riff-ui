//! riff-uiux `DropdownMenu`: actions and choices behind a trigger — icons,
//! brand logos, quiet descriptions, shortcuts, checks (`radio` / `checkbox`),
//! red danger items, submenus, a gliding row highlight, type-ahead, and full
//! menu keyboard navigation.

#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, Entity, FocusHandle,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement as _,
    IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement as _, Pixels,
    RenderOnce, SharedString, Window, div, point, px, size,
};

use crate::menu::style::TRIGGER_OFFSET;
use crate::menu::{
    AnchoredMenuPanel, MenuController, MenuControllerProps, MenuHeaderFn, MenuPanelSpec,
    MenuRootAnchor, MenuStart,
};
pub use crate::menu::{MenuIcon, MenuItem, MenuItemKind};
use crate::overlay::{OverlayAlign, OverlaySide};

pub type DropdownMenuItem = MenuItem;
pub type DropdownMenuItemKind = MenuItemKind;
pub type DropdownMenuIcon = MenuIcon;

/// Which vertical edge of the trigger [`DropdownMenu`] prefers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DropdownMenuSide {
    #[default]
    Bottom,
    Top,
}

impl From<DropdownMenuSide> for OverlaySide {
    fn from(value: DropdownMenuSide) -> Self {
        match value {
            DropdownMenuSide::Bottom => OverlaySide::Bottom,
            DropdownMenuSide::Top => OverlaySide::Top,
        }
    }
}

/// Which horizontal edge [`DropdownMenu`] aligns to along the trigger.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DropdownMenuAlign {
    #[default]
    Start,
    End,
}

impl From<DropdownMenuAlign> for OverlayAlign {
    fn from(value: DropdownMenuAlign) -> Self {
        match value {
            DropdownMenuAlign::Start => OverlayAlign::Start,
            DropdownMenuAlign::End => OverlayAlign::End,
        }
    }
}

type OpenChangeFn = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A riff-uiux dropdown menu attached to a trigger element.
#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    trigger: Option<AnyElement>,
    items: Vec<DropdownMenuItem>,
    side: DropdownMenuSide,
    align: DropdownMenuAlign,
    title: Option<SharedString>,
    header: Option<MenuHeaderFn>,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeFn>,
    width: Option<Pixels>,
    empty: SharedString,
    aria_label: Option<SharedString>,
    freeze_at: Option<Duration>,
    inline: bool,
    pinned_highlight: Option<(usize, bool)>,
    pinned_submenu: Option<usize>,
}

impl DropdownMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            items: Vec::new(),
            side: DropdownMenuSide::Bottom,
            align: DropdownMenuAlign::Start,
            title: None,
            header: None,
            open: None,
            default_open: false,
            on_open_change: None,
            width: None,
            empty: "Nothing here yet".into(),
            aria_label: None,
            freeze_at: None,
            inline: false,
            pinned_highlight: None,
            pinned_submenu: None,
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = DropdownMenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn item(mut self, item: DropdownMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn side(mut self, side: DropdownMenuSide) -> Self {
        self.side = side;
        self
    }

    pub fn bottom(self) -> Self {
        self.side(DropdownMenuSide::Bottom)
    }

    pub fn top(self) -> Self {
        self.side(DropdownMenuSide::Top)
    }

    pub fn align(mut self, align: DropdownMenuAlign) -> Self {
        self.align = align;
        self
    }

    pub fn start(self) -> Self {
        self.align(DropdownMenuAlign::Start)
    }

    pub fn end(self) -> Self {
        self.align(DropdownMenuAlign::End)
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title = (!t.is_empty()).then_some(t);
        self
    }

    pub fn header(
        mut self,
        header: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.header = Some(Rc::new(header));
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

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn empty(mut self, empty: impl Into<SharedString>) -> Self {
        self.empty = empty.into();
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        self.aria_label = (!l.is_empty()).then_some(l);
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

    #[doc(hidden)]
    pub fn pin_highlight(mut self, index: usize, focus_visible: bool) -> Self {
        self.pinned_highlight = Some((index, focus_visible));
        self
    }

    #[doc(hidden)]
    pub fn pin_submenu(mut self, index: usize) -> Self {
        self.pinned_submenu = Some(index);
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(trigger) = self.trigger.take() else {
            return gpui_kit::Empty.into_any_element();
        };

        let inline = self.inline;
        let controller = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            let initial = self.open.unwrap_or(self.default_open);
            MenuController::new(initial, window, cx)
        });
        let trigger_focus =
            window.use_keyed_state((self.id.clone(), "dm-trig-focus"), cx, |_, cx| {
                cx.focus_handle().tab_stop(false)
            });
        let trigger_focus_handle = trigger_focus.read(cx).clone();

        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                MenuControllerProps {
                    controlled_open: self.open,
                    on_open_change: self.on_open_change.clone(),
                    on_select_close_focus: Some(trigger_focus_handle.downgrade()),
                    pinned_highlight: self.pinned_highlight,
                    pinned_submenu: self.pinned_submenu,
                    inline,
                },
                &self.items,
                window,
                cx,
            )
        });

        let trigger_bounds = Rc::new(Cell::new(Bounds::default()));
        let side: OverlaySide = self.side.into();
        let align: OverlayAlign = self.align.into();

        let level_count = if open {
            controller.read(cx).level_count()
        } else {
            0
        };

        let mut panels = Vec::with_capacity(level_count);
        for depth in 0..level_count {
            panels.push(
                AnchoredMenuPanel::new(
                    (self.id.clone(), format!("dm-panel-{depth}")).into(),
                    MenuPanelSpec {
                        id: self.id.clone(),
                        depth,
                        title: self.title.clone(),
                        empty: self.empty.clone(),
                        header: self.header.clone(),
                        width: if depth == 0 { self.width } else { None },
                        aria_label: self.aria_label.clone(),
                        anchor: MenuRootAnchor::Side {
                            trigger: trigger_bounds.clone(),
                            side,
                            align,
                            offset: TRIGGER_OFFSET,
                        },
                        inline,
                        freeze_at: self.freeze_at,
                        controller: controller.clone(),
                    },
                )
                .into_any_element(),
            );
        }

        let controlled = self.open.is_some();
        let trigger = if inline {
            trigger
        } else {
            let controller_key = controller.clone();
            let trigger_focus_key = trigger_focus_handle.clone();
            let items_key = self.items.clone();
            div()
                .id((self.id.clone(), "dm-trig-wrap"))
                .track_focus(&trigger_focus_handle)
                .when(!controlled, |d| {
                    d.capture_key_down(move |event: &KeyDownEvent, window, cx| {
                        if controller_key.read(cx).effective_open() {
                            return;
                        }
                        let start = match event.keystroke.key.as_str() {
                            "down" | "enter" | "space" => Some(MenuStart::First),
                            "up" => Some(MenuStart::Last),
                            _ => None,
                        };
                        if let Some(start) = start {
                            let return_focus = window
                                .focused(cx)
                                .or_else(|| Some(trigger_focus_key.clone()));
                            window.prevent_default();
                            cx.stop_propagation();
                            controller_key.update(cx, |this, cx| {
                                this.open_with_start(start, return_focus, &items_key, window, cx);
                            });
                        }
                    })
                })
                .child(trigger)
                .into_any_element()
        };

        DropdownMenuElement {
            id: self.id,
            trigger,
            controller,
            trigger_focus: trigger_focus_handle,
            items: self.items,
            side,
            align,
            inline,
            controlled,
            trigger_bounds,
            panels,
            inline_panels: Vec::new(),
        }
        .into_any_element()
    }
}

struct DropdownMenuElement {
    id: ElementId,
    trigger: AnyElement,
    controller: Entity<MenuController>,
    trigger_focus: FocusHandle,
    items: Vec<MenuItem>,
    side: OverlaySide,
    align: OverlayAlign,
    inline: bool,
    controlled: bool,
    trigger_bounds: Rc<Cell<Bounds<Pixels>>>,
    panels: Vec<AnyElement>,
    inline_panels: Vec<AnyElement>,
}

impl IntoElement for DropdownMenuElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct DropdownMenuPrepaint {
    trigger_hitbox: Option<Hitbox>,
}

impl Element for DropdownMenuElement {
    type RequestLayoutState = ();
    type PrepaintState = DropdownMenuPrepaint;

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
        if !self.inline {
            for panel in &mut self.panels {
                panel.request_layout(window, cx);
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
    ) -> DropdownMenuPrepaint {
        self.trigger.prepaint(window, cx);
        self.trigger_bounds.set(bounds);

        let trigger_hitbox = (!self.inline && !self.controlled)
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal));

        if self.inline {
            for mut panel in self.panels.drain(..) {
                let available = size(
                    gpui_kit::AvailableSpace::MaxContent,
                    gpui_kit::AvailableSpace::MinContent,
                );
                panel.layout_as_root(available, window, cx);
                panel.prepaint_at(point(px(0.), px(0.)), window, cx);
                self.inline_panels.push(panel);
            }
        } else {
            for (depth, panel) in self.panels.drain(..).enumerate() {
                window.defer_draw(panel, point(px(0.), px(0.)), 1000 + depth, None);
            }
        }

        DropdownMenuPrepaint { trigger_hitbox }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut DropdownMenuPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.trigger.paint(window, cx);
        for mut panel in self.inline_panels.drain(..) {
            panel.paint(window, cx);
        }
        if self.inline {
            return;
        }

        let _ = (self.side, self.align);
        if let Some(hitbox) = prepaint.trigger_hitbox.as_ref() {
            let hitbox_click = hitbox.clone();
            let controller = self.controller.clone();
            let trigger_focus = self.trigger_focus.clone();
            let items = self.items.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }
                if hitbox_click.is_hovered(window) {
                    let return_focus = window.focused(cx).or_else(|| Some(trigger_focus.clone()));
                    controller.update(cx, |this, cx| {
                        this.toggle_with_start(MenuStart::Menu, return_focus, &items, window, cx);
                    });
                }
            });
        }
    }
}
