//! riff-uiux `ContextMenu`: the menu a right-click opens right at the
//! pointer — Shift+F10 / ContextMenu key from keyboard focus, dynamic per-point
//! items, icons, descriptions, shortcuts, checks (`radio` / `checkbox`), red
//! danger items, submenus, gliding highlight, and full menu keyboard.

#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::LayoutDirection;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, Entity, FocusHandle,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement as _,
    IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, ParentElement as _, Pixels,
    Point, RenderOnce, SharedString, Window, div, point, px, size,
};

use crate::menu::{
    AnchoredMenuPanel, MenuController, MenuControllerProps, MenuPanelSpec, MenuRootAnchor,
    MenuStart,
};
pub use crate::menu::{MenuIcon, MenuItem, MenuItemKind};

pub type ContextMenuItem = MenuItem;
pub type ContextMenuItemKind = MenuItemKind;
pub type ContextMenuIcon = MenuIcon;

type ItemsBuilderFn = Rc<dyn Fn(Point<Pixels>, &Window, &App) -> Vec<ContextMenuItem> + 'static>;

/// A riff-uiux right-click context menu wrapping a target region.
type OpenChangeFn = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    area: Option<AnyElement>,
    items: Vec<ContextMenuItem>,
    items_fn: Option<ItemsBuilderFn>,
    title: Option<SharedString>,
    aria_label: SharedString,
    disabled: bool,
    width: Option<Pixels>,
    empty: SharedString,
    on_open_change: Option<OpenChangeFn>,
    freeze_at: Option<Duration>,
    inline: bool,
    preview_offset: Option<Point<Pixels>>,
}

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            area: None,
            items: Vec::new(),
            items_fn: None,
            title: None,
            aria_label: "Actions".into(),
            disabled: false,
            width: None,
            empty: "Nothing here yet".into(),
            on_open_change: None,
            freeze_at: None,
            inline: false,
            preview_offset: None,
        }
    }

    pub fn child(mut self, area: impl IntoElement) -> Self {
        self.area = Some(area.into_any_element());
        self
    }

    pub fn trigger(self, area: impl IntoElement) -> Self {
        self.child(area)
    }

    pub fn items(mut self, items: impl IntoIterator<Item = ContextMenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Builds items dynamically for the right-click pointer position (or the
    /// keyboard opening anchor when invoked via Shift+F10 / ContextMenu).
    pub fn items_for_point(
        mut self,
        builder: impl Fn(Point<Pixels>, &Window, &App) -> Vec<ContextMenuItem> + 'static,
    ) -> Self {
        self.items_fn = Some(Rc::new(builder));
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title = (!t.is_empty()).then_some(t);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.aria_label = l;
        }
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

    #[doc(hidden)]
    pub fn preview_offset(mut self, offset: Point<Pixels>) -> Self {
        self.preview_offset = Some(offset);
        self
    }
}

impl RenderOnce for ContextMenu {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(area) = self.area.take() else {
            return gpui_kit::Empty.into_any_element();
        };

        let inline = self.inline;
        let controller = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            MenuController::new(false, window, cx)
        });
        let cursor_state = window.use_keyed_state((self.id.clone(), "ctx-cursor"), cx, |_, _| {
            Rc::new(Cell::new(point(px(0.), px(0.))))
        });
        let cursor_cell = cursor_state.read(cx).clone();
        let area_focus =
            window.use_keyed_state((self.id.clone(), "ctx-area-focus"), cx, |_, cx| {
                cx.focus_handle().tab_stop(false)
            });
        let area_focus_handle = area_focus.read(cx).clone();
        let area_bounds_state =
            window.use_keyed_state((self.id.clone(), "ctx-area-bounds"), cx, |_, _| {
                Rc::new(Cell::new(Bounds::<Pixels>::default()))
            });
        let area_bounds_cell = area_bounds_state.read(cx).clone();

        let current_items = if let Some(builder) = &self.items_fn {
            if inline || controller.read(cx).effective_open() {
                builder(cursor_cell.get(), window, cx)
            } else {
                Vec::new()
            }
        } else {
            self.items.clone()
        };

        let controlled_open = if self.disabled {
            Some(false)
        } else if inline {
            Some(!current_items.is_empty())
        } else {
            None
        };

        let open = controller.clone().update(cx, |state, cx| {
            state.sync_frame(
                &controller,
                MenuControllerProps {
                    controlled_open,
                    on_open_change: self.on_open_change.clone(),
                    on_select_close_focus: Some(area_focus_handle.downgrade()),
                    pinned_highlight: None,
                    pinned_submenu: None,
                    inline,
                },
                &current_items,
                window,
                cx,
            )
        });

        let level_count = if open {
            controller.read(cx).level_count()
        } else {
            0
        };

        let mut panels = Vec::with_capacity(level_count);
        for depth in 0..level_count {
            panels.push(
                AnchoredMenuPanel::new(
                    (self.id.clone(), format!("ctx-panel-{depth}")).into(),
                    MenuPanelSpec {
                        id: self.id.clone(),
                        depth,
                        title: self.title.clone(),
                        empty: self.empty.clone(),
                        header: None,
                        width: if depth == 0 { self.width } else { None },
                        aria_label: Some(self.aria_label.clone()),
                        anchor: MenuRootAnchor::Cursor {
                            point: cursor_cell.clone(),
                        },
                        inline,
                        freeze_at: self.freeze_at,
                        controller: controller.clone(),
                    },
                )
                .into_any_element(),
            );
        }

        let static_items = self.items;
        let items_fn = self.items_fn;
        let disabled = self.disabled;

        let area = if inline {
            area
        } else {
            let controller_key = controller.clone();
            let area_focus_key = area_focus_handle.clone();
            let cursor_key = cursor_cell.clone();
            let bounds_key = area_bounds_cell.clone();
            let static_key = static_items.clone();
            let fn_key = items_fn.clone();
            div()
                .id((self.id.clone(), "ctx-area-wrap"))
                .track_focus(&area_focus_handle)
                .when(!disabled, |d| {
                    d.capture_key_down(move |event: &KeyDownEvent, window, cx| {
                        let is_menu_key = event.keystroke.key == "contextmenu"
                            || (event.keystroke.key == "f10" && event.keystroke.modifiers.shift);
                        if !is_menu_key {
                            return;
                        }
                        let r = bounds_key.get();
                        let rtl = LayoutDirection::global(cx).is_rtl();
                        let x = if rtl {
                            r.right() - px(12.).min(r.size.width / 2.)
                        } else {
                            r.left() + px(12.).min(r.size.width / 2.)
                        };
                        let y = r.top() + r.size.height.min(px(26.));
                        let pt = point(x, y);
                        let built = match &fn_key {
                            Some(f) => f(pt, window, cx),
                            None => static_key.clone(),
                        };
                        if built.is_empty() {
                            return;
                        }
                        cursor_key.set(pt);
                        let return_focus =
                            window.focused(cx).or_else(|| Some(area_focus_key.clone()));
                        window.prevent_default();
                        cx.stop_propagation();
                        controller_key.update(cx, |this, cx| {
                            this.open_with_start(
                                MenuStart::First,
                                return_focus,
                                &built,
                                window,
                                cx,
                            );
                        });
                    })
                })
                .child(area)
                .into_any_element()
        };

        ContextMenuElement {
            id: self.id,
            area,
            controller,
            area_focus: area_focus_handle,
            cursor_cell,
            area_bounds_cell,
            static_items,
            items_fn,
            disabled,
            inline,
            preview_offset: self.preview_offset,
            panels,
            inline_panels: Vec::new(),
        }
        .into_any_element()
    }
}

struct ContextMenuElement {
    id: ElementId,
    area: AnyElement,
    controller: Entity<MenuController>,
    area_focus: FocusHandle,
    cursor_cell: Rc<Cell<Point<Pixels>>>,
    area_bounds_cell: Rc<Cell<Bounds<Pixels>>>,
    static_items: Vec<ContextMenuItem>,
    items_fn: Option<ItemsBuilderFn>,
    disabled: bool,
    inline: bool,
    preview_offset: Option<Point<Pixels>>,
    panels: Vec<AnyElement>,
    inline_panels: Vec<AnyElement>,
}

impl IntoElement for ContextMenuElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

struct ContextMenuPrepaint {
    area_hitbox: Option<Hitbox>,
}

impl Element for ContextMenuElement {
    type RequestLayoutState = ();
    type PrepaintState = ContextMenuPrepaint;

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
        let layout_id = self.area.request_layout(window, cx);
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
    ) -> ContextMenuPrepaint {
        self.area.prepaint(window, cx);
        self.area_bounds_cell.set(bounds);

        if self.inline {
            let offset = self
                .preview_offset
                .unwrap_or_else(|| point(px(12.), px(12.)));
            self.cursor_cell.set(bounds.origin + offset);
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

        let area_hitbox = (!self.inline && !self.disabled)
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal));

        ContextMenuPrepaint { area_hitbox }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        prepaint: &mut ContextMenuPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.area.paint(window, cx);
        for mut panel in self.inline_panels.drain(..) {
            panel.paint(window, cx);
        }
        if self.inline {
            return;
        }

        if let Some(hitbox) = prepaint.area_hitbox.as_ref() {
            let hitbox = hitbox.clone();
            let controller = self.controller.clone();
            let cursor_cell = self.cursor_cell.clone();
            let area_focus = self.area_focus.clone();
            let static_items = self.static_items.clone();
            let items_fn = self.items_fn.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || event.button != MouseButton::Right {
                    return;
                }
                if !hitbox.is_hovered(window) {
                    return;
                }
                let built = match &items_fn {
                    Some(f) => f(event.position, window, cx),
                    None => static_items.clone(),
                };
                if built.is_empty() {
                    return;
                }
                cursor_cell.set(event.position);
                let return_focus = window.focused(cx).or_else(|| Some(area_focus.clone()));
                cx.stop_propagation();
                controller.update(cx, |this, cx| {
                    this.open_with_start(MenuStart::Menu, return_focus, &built, window, cx);
                });
            });
        }
    }
}
