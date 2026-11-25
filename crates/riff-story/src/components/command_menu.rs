//! CommandMenu: story cells matching stories/command-menu.json plus an
//! interactive demo with ⌘K/Ctrl+K focus, Tab section jumping, live search,
//! assistant fallback, and run announcements.

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::command_menu::{CommandGroup, CommandIcon, CommandItem, CommandMenu};
use riff_ui::dialog::Dialog;
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "width",
    "placeholder",
    "groups",
    "query",
    "selectedId",
    "clickLabel",
];

fn icon_path(name: &str) -> CommandIcon {
    CommandIcon::path(format!("icons/{name}.svg"))
}

fn parse_groups(props: &Props) -> Result<Option<Vec<CommandGroup>>, String> {
    let Some(objs) = props.objects("groups") else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(objs.len());
    for g in objs {
        let gid = g.str("id").unwrap_or("").to_owned();
        let glabel = g.str("label").unwrap_or("").to_owned();
        let mut group = CommandGroup::new(gid, glabel);
        if let Some(cmds) = g.objects("commands") {
            for c in cmds {
                let cid = c.str("id").unwrap_or("").to_owned();
                let clabel = c.str("label").unwrap_or("").to_owned();
                let mut item = CommandItem::new(cid, clabel);
                if let Some(hint) = c.str("hint") {
                    item = item.hint(hint.to_owned());
                }
                if let Some(kw) = c.str("keywords") {
                    item = item.keywords(kw.to_owned());
                }
                if let Some(ic) = c.str("icon") {
                    item = item.icon(icon_path(ic));
                }
                if let Some(ks) = c.strings("keys") {
                    item = item.keys(ks.into_iter().map(str::to_owned));
                }
                group = group.command(item);
            }
        }
        out.push(group);
    }
    Ok(Some(out))
}

pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let width = props.f32("width").unwrap_or(560.);
    let mut cm = CommandMenu::new(SharedString::from(format!("story-cm-{id}")))
        .width(px(width))
        .inline_preview(true);

    if let Some(groups) = parse_groups(props)? {
        cm = cm.groups(groups);
    }
    if let Some(ph) = props.str("placeholder") {
        cm = cm.placeholder(ph.to_owned());
    }
    if let Some(q) = props.str("query") {
        cm = cm.query(Some(SharedString::from(q.to_owned())));
    }
    if let Some(sel) = props.str("selectedId") {
        cm = cm.selected_key(Some(SharedString::from(sel.to_owned())));
    }
    if let Some(ran) = props.str("clickLabel") {
        cm = cm.ran_preview(
            props.str("selectedId").unwrap_or("").to_owned(),
            ran.to_owned(),
        );
    }
    if let Some(at) = pin.freeze {
        cm = cm.freeze_motion_at(at);
    }

    Ok(cm.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    last_ran: Option<String>,
    modal_open: bool,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let weak_run = cx.entity().downgrade();
        let weak_modal = cx.entity().downgrade();

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        Dialog::new("demo-cm-dlg", "Command palette")
                            .lg()
                            .open(Some(self.modal_open))
                            .on_open_change(move |open, _, cx| {
                                _ = weak_modal.update(cx, |this, cx| {
                                    this.modal_open = open;
                                    cx.notify();
                                });
                            })
                            .trigger(
                                Button::new("demo-cm-open")
                                    .secondary()
                                    .label("Open inside Dialog"),
                            )
                            .child(CommandMenu::new("demo-cm-modal").auto_focus(true)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted)
                            .child(match &self.last_ran {
                            Some(last) => format!("Last ran: {last}"),
                            None => {
                                "Type to filter, Tab to jump groups, ↑/↓ + Enter or Ctrl/⌘+1..5 / K"
                                    .into()
                            }
                        }),
                    ),
            )
            .child(CommandMenu::new("demo-cm-inline").auto_focus(true).on_run(
                move |id, query, _, cx| {
                    let summary = match query {
                        Some(q) => format!("{id} ({q})"),
                        None => id.to_owned(),
                    };
                    _ = weak_run.update(cx, |this, cx| {
                        this.last_ran = Some(summary);
                        cx.notify();
                    });
                },
            ))
    }
}
