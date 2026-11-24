//! Card: the story's cells, and an interactive card & picker demo.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::badge::{Badge, BadgeTone};
use riff_ui::button::Button;
use riff_ui::card::{Card, CardContent, CardFooter, CardHeader, CardIcon, CardState};
use riff_ui::icon_button::IconButton;
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::{PinnedState, Props};

const PROPS: &[&str] = &[
    "width",
    "interactive",
    "selected",
    "icon",
    "title",
    "description",
    "actionIcon",
    "headerExtra",
    "content",
    "footer",
];

/// The live card for one cell of `stories/card.json`.
pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let interactive = props.bool("interactive");
    let selected = props.bool("selected");
    let mut card = Card::new()
        .id(SharedString::from(format!("story-{id}")))
        .interactive(interactive)
        .selected(selected);

    if let Some(w) = props.f32("width") {
        card = card.w(px(w));
    }

    if interactive {
        card = card.force_state(CardState {
            hovered: matches!(pin.state, Some(PinnedState::Hover | PinnedState::Active)),
            focus_visible: pin.state == Some(PinnedState::Focus),
        });
    }

    let icon = props.str("icon");
    let title = props.str("title");
    let description = props.str("description");
    let action_icon = props.str("actionIcon");
    let header_extra = props.str("headerExtra");

    if icon.is_some()
        || title.is_some()
        || description.is_some()
        || action_icon.is_some()
        || header_extra.is_some()
    {
        let mut header = CardHeader::new();
        if let Some(name) = icon {
            header = header.svg_icon(CardIcon::path(format!("icons/{name}.svg")));
        }
        if let Some(t) = title {
            header = header.title(t.to_owned());
        }
        if let Some(d) = description {
            header = header.description(d.to_owned());
        }
        if let Some(name) = action_icon {
            #[derive(IntoElement)]
            struct ActionSvg(SharedString);
            impl RenderOnce for ActionSvg {
                fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                    gpui_kit::svg()
                        .path(self.0)
                        .size(px(16.))
                        .flex_none()
                        .text_color(RiffTheme::global(cx).ink)
                }
            }
            header = header.action(ActionSvg(format!("icons/{name}.svg").into()));
        }
        if let Some(extra) = header_extra {
            header = header.child(
                div()
                    .mt(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .child(extra.to_owned()),
            );
        }
        card = card.header(header);
    }

    if let Some(content) = props.str("content") {
        card = card.content(CardContent::new().child(content.to_owned()));
    }

    if let Some(footer) = props.str("footer") {
        card = card.footer(CardFooter::new().child(footer.to_owned()));
    }

    Ok(card.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    picked: usize,
    card_opens: usize,
    more_clicks: usize,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let plans = [
            ("Starter", "Up to 3 active agents"),
            ("Pro", "Up to 10 active agents"),
        ];

        div()
            .flex()
            .flex_col()
            .gap_4()
            .w(px(380.))
            .child(
                Card::new()
                    .id("demo-agent-card")
                    .interactive(true)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.card_opens += 1;
                        cx.notify();
                    }))
                    .header(
                        CardHeader::new()
                            .svg_icon(IconName::Bot)
                            .title("Support triage")
                            .description("Sorts new tickets and drafts the first reply")
                            .action(
                                IconButton::new("demo-card-more")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Ellipsis)
                                    .label("More actions")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.more_clicks += 1;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .mt_1()
                                    .child(
                                        Badge::new().tone(BadgeTone::Info).live(true).label("Live"),
                                    )
                                    .child(Badge::new().label("96% resolved")),
                            ),
                    )
                    .content("142 tickets this week · average first reply 18s")
                    .footer(
                        CardFooter::new()
                            .child("Running · started 2 min ago")
                            .child(
                                div().ml_auto().child(
                                    Button::new("demo-card-open")
                                        .secondary()
                                        .small()
                                        .label("Open")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.card_opens += 1;
                                            cx.notify();
                                        })),
                                ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(plans.into_iter().enumerate().map(|(idx, (title, desc))| {
                        let selected = self.picked == idx;
                        Card::new()
                            .id(("demo-plan", idx))
                            .flex_1()
                            .interactive(true)
                            .selected(selected)
                            .pressed(selected)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.picked = idx;
                                cx.notify();
                            }))
                            .header(CardHeader::new().title(title).description(desc))
                    })),
            )
            .child(div().text_xs().text_color(theme.muted).child(format!(
                "picked: {} · card opens: {} · menu clicks: {}",
                plans[self.picked].0, self.card_opens, self.more_clicks
            )))
    }
}
