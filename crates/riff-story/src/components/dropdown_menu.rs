//! Dropdown Menu: story cells and a rich interactive demo exercising
//! gliding highlight, checks, submenus, type-ahead, and keyboard navigation.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::dropdown_menu::{
    DropdownMenu, DropdownMenuAlign, DropdownMenuIcon, DropdownMenuItem, DropdownMenuItemKind,
    DropdownMenuSide,
};
use riff_ui::icon_button::IconButton;
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "side",
    "align",
    "title",
    "headerTitle",
    "headerMeta",
    "width",
    "empty",
    "items",
    "triggerLabel",
    "stageWidth",
    "stageHeight",
];

pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let side = match props.str("side").unwrap_or("bottom") {
        "bottom" => DropdownMenuSide::Bottom,
        "top" => DropdownMenuSide::Top,
        other => return Err(format!("unknown side {other:?}")),
    };
    let align = match props.str("align").unwrap_or("start") {
        "start" => DropdownMenuAlign::Start,
        "end" => DropdownMenuAlign::End,
        other => return Err(format!("unknown align {other:?}")),
    };

    let stage_w = props.f32("stageWidth").unwrap_or(220.);
    let stage_h = props.f32("stageHeight").unwrap_or(190.);
    let trigger_label = props.str("triggerLabel").unwrap_or("Actions").to_owned();

    let items = parse_items(props)?;

    let mut dm = DropdownMenu::new(SharedString::from(format!("story-dm-{id}")))
        .side(side)
        .align(align)
        .items(items)
        .open(Some(true))
        .inline_preview(true)
        .trigger(StoryTrigger {
            id: SharedString::from(format!("story-dm-trig-{id}")),
            label: trigger_label.into(),
        });

    if let Some(title) = props.str("title") {
        dm = dm.title(title.to_owned());
    }
    if let Some(width) = props.f32("width") {
        dm = dm.width(px(width));
    }
    if let Some(empty) = props.str("empty") {
        dm = dm.empty(empty.to_owned());
    }
    if let Some(at) = pin.freeze {
        dm = dm.freeze_motion_at(at);
    }

    let header_title = props.str("headerTitle").map(ToOwned::to_owned);
    let header_meta = props.str("headerMeta").map(ToOwned::to_owned);
    if header_title.is_some() || header_meta.is_some() {
        dm = dm.header(move |_, cx| {
            let theme = RiffTheme::global(cx);
            div()
                .pt(px(8.))
                .pb(px(6.))
                .px(px(10.))
                .border_b_1()
                .border_color(theme.border)
                .mb(px(4.))
                .flex()
                .flex_col()
                .gap(px(1.))
                .when_some(header_title.clone(), |d, t| {
                    d.child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.ink)
                            .child(t),
                    )
                })
                .when_some(header_meta.clone(), |d, m| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(theme.muted)
                            .child(m),
                    )
                })
                .into_any_element()
        });
    }

    use gpui_kit::prelude::FluentBuilder as _;
    let mut stage = div()
        .w(px(stage_w))
        .h(px(stage_h))
        .flex()
        .py(px(8.))
        .px(px(10.));
    stage = match side {
        DropdownMenuSide::Bottom => stage.items_start(),
        DropdownMenuSide::Top => stage.items_end(),
    };
    stage = match align {
        DropdownMenuAlign::Start => stage.justify_start(),
        DropdownMenuAlign::End => stage.justify_end(),
    };

    Ok(stage.child(dm).into_any_element())
}

fn parse_items(props: &Props) -> Result<Vec<DropdownMenuItem>, String> {
    let Some(objs) = props.objects("items") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(objs.len());
    for obj in objs {
        let label = obj
            .str("label")
            .ok_or_else(|| "menu item missing string label".to_owned())?;
        let mut item = DropdownMenuItem::new(label.to_owned());
        if let Some(desc) = obj.str("description") {
            item = item.description(desc.to_owned());
        }
        if let Some(icon) = obj.str("icon") {
            item = item.icon(DropdownMenuIcon::path(format!("icons/{icon}.svg")));
        }
        if obj.keys().any(|k| k == "checked") {
            item = item.checked(obj.bool("checked"));
        }
        if let Some(kind) = obj.str("type") {
            item = match kind {
                "radio" => item.kind(DropdownMenuItemKind::Radio),
                "checkbox" => item.kind(DropdownMenuItemKind::Checkbox),
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
            item = item.submenu(vec![DropdownMenuItem::new("Sub item")]);
        }
        out.push(item);
    }
    Ok(out)
}

#[derive(IntoElement)]
struct StoryTrigger {
    id: SharedString,
    label: SharedString,
}

impl RenderOnce for StoryTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        div()
            .id(self.id)
            .h(px(32.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_size(px(13.))
            .line_height(px(18.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.ink)
            .child(self.label)
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

struct Demo {
    sort: usize,
    show_icons: bool,
    compact: bool,
    folder: &'static str,
    last_action: SharedString,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            sort: 0,
            show_icons: true,
            compact: false,
            folder: "inbox",
            last_action: "None yet".into(),
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let weak = cx.entity().downgrade();
        let set_msg = move |msg: &'static str, cx: &mut App| {
            _ = weak.update(cx, |this, cx| {
                this.last_action = msg.into();
                cx.notify();
            });
        };

        let weak2 = cx.entity().downgrade();
        let sort_pick = move |idx: usize, label: &'static str, cx: &mut App| {
            _ = weak2.update(cx, |this, cx| {
                this.sort = idx;
                this.last_action = label.into();
                cx.notify();
            });
        };

        let weak3 = cx.entity().downgrade();
        let folder_pick = move |id: &'static str, cx: &mut App| {
            _ = weak3.update(cx, |this, cx| {
                this.folder = id;
                this.last_action = format!("Moved to {id}").into();
                cx.notify();
            });
        };

        let items = vec![
            DropdownMenuItem::new("Rerun")
                .icon(IconName::Repeat)
                .shortcut("⌘R")
                .on_select({
                    let set_msg = set_msg.clone();
                    move |_, cx| set_msg("Rerun triggered", cx)
                }),
            DropdownMenuItem::new("Duplicate")
                .icon(IconName::Copy)
                .description("Create an identical copy")
                .on_select({
                    let set_msg = set_msg.clone();
                    move |_, cx| set_msg("Duplicated item", cx)
                }),
            DropdownMenuItem::new("Move to")
                .icon(IconName::Folder)
                .divider(true)
                .submenu(vec![
                    DropdownMenuItem::new("Inbox")
                        .checked(self.folder == "inbox")
                        .on_select({
                            let f = folder_pick.clone();
                            move |_, cx| f("inbox", cx)
                        }),
                    DropdownMenuItem::new("Archive")
                        .checked(self.folder == "archive")
                        .on_select(move |_, cx| folder_pick("archive", cx)),
                ]),
            DropdownMenuItem::new("Newest first")
                .divider(true)
                .checked(self.sort == 0)
                .on_select({
                    let sp = sort_pick.clone();
                    move |_, cx| sp(0, "Sort: Newest first", cx)
                }),
            DropdownMenuItem::new("Name A–Z")
                .checked(self.sort == 1)
                .on_select(move |_, cx| sort_pick(1, "Sort: Name A–Z", cx)),
            DropdownMenuItem::new("Show icons")
                .checkbox()
                .checked(self.show_icons)
                .keep_open(true)
                .divider(true)
                .on_select({
                    let w = cx.entity().downgrade();
                    move |_, cx| {
                        _ = w.update(cx, |this, cx| {
                            this.show_icons = !this.show_icons;
                            cx.notify();
                        });
                    }
                }),
            DropdownMenuItem::new("Compact rows")
                .checkbox()
                .checked(self.compact)
                .keep_open(true)
                .on_select({
                    let w = cx.entity().downgrade();
                    move |_, cx| {
                        _ = w.update(cx, |this, cx| {
                            this.compact = !this.compact;
                            cx.notify();
                        });
                    }
                }),
            DropdownMenuItem::new("Delete run")
                .icon(IconName::Trash)
                .danger(true)
                .divider(true)
                .on_select(move |_, cx| set_msg("Deleted run", cx)),
        ];

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(div().text_xs().text_color(theme.muted).child(format!(
                "Last action: {} · Use arrows, A–Z type-ahead, Enter/Space, or hover submenu",
                self.last_action
            )))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        DropdownMenu::new("demo-dm-main")
                            .title("Run options")
                            .items(items.clone())
                            .trigger(Button::new("demo-dm-btn").secondary().label("Actions")),
                    )
                    .child(
                        DropdownMenu::new("demo-dm-icon")
                            .align(DropdownMenuAlign::End)
                            .items(items)
                            .trigger(
                                IconButton::new("demo-dm-more")
                                    .secondary()
                                    .icon(IconName::Ellipsis)
                                    .label("More actions"),
                            ),
                    ),
            )
    }
}
