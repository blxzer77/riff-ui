//! Context Menu: story cells and an interactive right-click / Shift+F10 demo
//! with position-aware dynamic items, checks, submenus, and focus return.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::context_menu::{ContextMenu, ContextMenuIcon, ContextMenuItem, ContextMenuItemKind};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "title",
    "width",
    "offsetX",
    "offsetY",
    "items",
    "stageWidth",
    "stageHeight",
];

pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let offset_x = props.f32("offsetX").unwrap_or(24.);
    let offset_y = props.f32("offsetY").unwrap_or(24.);
    let stage_w = props.f32("stageWidth").unwrap_or(230.);
    let stage_h = props.f32("stageHeight").unwrap_or(210.);
    let items = parse_items(props)?;

    let mut menu = ContextMenu::new(SharedString::from(format!("story-ctx-{id}")))
        .items(items)
        .inline_preview(true)
        .preview_offset(point(px(offset_x), px(offset_y)))
        .child(StoryArea {
            width: px(stage_w),
            height: px(stage_h),
        });

    if let Some(title) = props.str("title") {
        menu = menu.title(title.to_owned());
    }
    if let Some(width) = props.f32("width") {
        menu = menu.width(px(width));
    }
    if let Some(at) = pin.freeze {
        menu = menu.freeze_motion_at(at);
    }

    Ok(menu.into_any_element())
}

fn parse_items(props: &Props) -> Result<Vec<ContextMenuItem>, String> {
    let Some(objs) = props.objects("items") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(objs.len());
    for obj in objs {
        let label = obj
            .str("label")
            .ok_or_else(|| "context menu item missing label".to_owned())?;
        let mut item = ContextMenuItem::new(label.to_owned());
        if let Some(desc) = obj.str("description") {
            item = item.description(desc.to_owned());
        }
        if let Some(icon) = obj.str("icon") {
            item = item.icon(ContextMenuIcon::path(format!("icons/{icon}.svg")));
        }
        if obj.keys().any(|k| k == "checked") {
            item = item.checked(obj.bool("checked"));
        }
        if let Some(kind) = obj.str("type") {
            item = match kind {
                "radio" => item.kind(ContextMenuItemKind::Radio),
                "checkbox" => item.kind(ContextMenuItemKind::Checkbox),
                other => return Err(format!("unknown menu item type {other:?}")),
            };
        }
        if obj.bool("danger") {
            item = item.danger(true);
        }
        if let Some(shortcut) = obj.str("shortcut") {
            item = item.shortcut(shortcut.to_owned());
        }
        if obj.bool("divider") {
            item = item.divider(true);
        }
        if obj.bool("disabled") {
            item = item.disabled(true);
        }
        if obj.bool("hasSubmenu") {
            item = item.submenu(vec![ContextMenuItem::new("Inbox")]);
        }
        out.push(item);
    }
    Ok(out)
}

#[derive(IntoElement)]
struct StoryArea {
    width: Pixels,
    height: Pixels,
}

impl RenderOnce for StoryArea {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        div()
            .w(self.width)
            .h(self.height)
            .rounded(theme.radius)
            .border_1()
            .border_dashed()
            .border_color(theme.border)
            .bg(theme.subtle)
            .text_color(theme.muted)
            .py(px(6.))
            .px(px(10.))
            .text_size(px(11.5))
            .line_height(px(16.))
            .child("Right-click area")
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(Demo::new).into());

struct Demo {
    focus: FocusHandle,
    show_grid: bool,
    last_action: SharedString,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(true),
            show_grid: true,
            last_action: "Right-click anywhere on the canvas or press Shift+F10 when focused"
                .into(),
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let weak = cx.entity().downgrade();
        let show_grid = self.show_grid;

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted)
                    .child(self.last_action.clone()),
            )
            .child(
                ContextMenu::new("demo-ctx")
                    .title("Canvas actions")
                    .items_for_point(move |pt, _, _| {
                        let w1 = weak.clone();
                        let w2 = weak.clone();
                        let w3 = weak.clone();
                        let w4 = weak.clone();
                        vec![
                            ContextMenuItem::new(format!(
                                "New node at ({}, {})",
                                pt.x.as_f32().round() as i32,
                                pt.y.as_f32().round() as i32
                            ))
                            .icon(IconName::Plus)
                            .shortcut("N")
                            .on_select(move |_, cx| {
                                _ = w1.update(cx, |this, cx| {
                                    this.last_action = format!(
                                        "Created node at ({}, {})",
                                        pt.x.as_f32().round() as i32,
                                        pt.y.as_f32().round() as i32
                                    )
                                    .into();
                                    cx.notify();
                                });
                            }),
                            ContextMenuItem::new("Share")
                                .icon(IconName::Share)
                                .submenu(vec![ContextMenuItem::new("Copy permalink").on_select(
                                    move |_, cx| {
                                        _ = w2.update(cx, |this, cx| {
                                            this.last_action = "Copied permalink".into();
                                            cx.notify();
                                        });
                                    },
                                )]),
                            ContextMenuItem::new("Show grid")
                                .checkbox()
                                .checked(show_grid)
                                .keep_open(true)
                                .divider(true)
                                .on_select(move |_, cx| {
                                    _ = w3.update(cx, |this, cx| {
                                        this.show_grid = !this.show_grid;
                                        this.last_action = if this.show_grid {
                                            "Grid enabled".into()
                                        } else {
                                            "Grid hidden".into()
                                        };
                                        cx.notify();
                                    });
                                }),
                            ContextMenuItem::new("Clear canvas")
                                .icon(IconName::Trash)
                                .danger(true)
                                .divider(true)
                                .on_select(move |_, cx| {
                                    _ = w4.update(cx, |this, cx| {
                                        this.last_action = "Cleared canvas".into();
                                        cx.notify();
                                    });
                                }),
                        ]
                    })
                    .child(
                        div()
                            .id("demo-ctx-canvas")
                            .track_focus(&self.focus)
                            .tab_index(0)
                            .w(px(420.))
                            .h(px(180.))
                            .rounded(theme.radius_lg)
                            .border_1()
                            .border_dashed()
                            .border_color(theme.border_strong)
                            .bg(theme.subtle)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(theme.muted)
                            .child("Right-click or focus + Shift+F10"),
                    ),
            )
    }
}
