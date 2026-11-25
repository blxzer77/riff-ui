//! Hover Card: story cells and an interactive demo exercising open/close
//! delays, diagonal corridor persistence, keyboard Tab focus, and Escape.

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::hover_card::{HoverCard, HoverCardAlign, HoverCardSide};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "side",
    "align",
    "offset",
    "width",
    "title",
    "handle",
    "meta",
    "linkLabel",
    "triggerLabel",
    "triggerHeight",
    "stageWidth",
    "stageHeight",
];

pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let side = match props.str("side").unwrap_or("bottom") {
        "bottom" => HoverCardSide::Bottom,
        "top" => HoverCardSide::Top,
        "left" => HoverCardSide::Left,
        "right" => HoverCardSide::Right,
        other => return Err(format!("unknown side {other:?}")),
    };
    let align = match props.str("align").unwrap_or("start") {
        "start" => HoverCardAlign::Start,
        "center" => HoverCardAlign::Center,
        "end" => HoverCardAlign::End,
        other => return Err(format!("unknown align {other:?}")),
    };
    let offset = props.f32("offset").unwrap_or(8.);
    let trigger_label = props.str("triggerLabel").unwrap_or("@alice").to_owned();
    let trigger_h = props.f32("triggerHeight").unwrap_or(28.);
    let stage_w = props.f32("stageWidth").unwrap_or(220.);
    let stage_h = props.f32("stageHeight").unwrap_or(136.);

    let title = props.str("title").map(ToOwned::to_owned);
    let handle = props.str("handle").map(ToOwned::to_owned);
    let meta = props.str("meta").map(ToOwned::to_owned);
    let link_label = props.str("linkLabel").map(ToOwned::to_owned);

    let mut card = HoverCard::new(SharedString::from(format!("story-hvc-{id}")))
        .side(side)
        .align(align)
        .offset(px(offset))
        .open(Some(true))
        .inline_preview(true)
        .trigger(StoryTrigger {
            id: SharedString::from(format!("story-hvc-trig-{id}")),
            label: trigger_label.into(),
            height: px(trigger_h),
        })
        .child(StoryCardBody {
            title: title.map(SharedString::from),
            handle: handle.map(SharedString::from),
            meta: meta.map(SharedString::from),
            link_label: link_label.map(SharedString::from),
        });

    if let Some(w) = props.f32("width") {
        card = card.width(px(w));
    }
    if let Some(at) = pin.freeze {
        card = card.freeze_motion_at(at);
    }

    let mut stage = div().w(px(stage_w)).h(px(stage_h)).flex();
    match side {
        HoverCardSide::Bottom => {
            stage = stage.items_start().pt(px(8.));
            match align {
                HoverCardAlign::Start => stage = stage.justify_start().pl(px(10.)),
                HoverCardAlign::Center => stage = stage.justify_center(),
                HoverCardAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        HoverCardSide::Top => {
            stage = stage.items_end().pb(px(8.));
            match align {
                HoverCardAlign::Start => stage = stage.justify_start().pl(px(10.)),
                HoverCardAlign::Center => stage = stage.justify_center(),
                HoverCardAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        HoverCardSide::Left => {
            stage = stage.justify_end().pr(px(8.));
            match align {
                HoverCardAlign::Start => stage = stage.items_start().pt(px(8.)),
                HoverCardAlign::Center => stage = stage.items_center(),
                HoverCardAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
        HoverCardSide::Right => {
            stage = stage.justify_start().pl(px(8.));
            match align {
                HoverCardAlign::Start => stage = stage.items_start().pt(px(8.)),
                HoverCardAlign::Center => stage = stage.items_center(),
                HoverCardAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
    }

    Ok(stage.child(card).into_any_element())
}

#[derive(IntoElement)]
struct StoryTrigger {
    id: SharedString,
    label: SharedString,
    height: Pixels,
}

impl RenderOnce for StoryTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        div()
            .id(self.id)
            .h(self.height)
            .px(px(10.))
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

#[derive(IntoElement)]
struct StoryCardBody {
    title: Option<SharedString>,
    handle: Option<SharedString>,
    meta: Option<SharedString>,
    link_label: Option<SharedString>,
}

impl RenderOnce for StoryCardBody {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let has_head = self.title.is_some() || self.handle.is_some();
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .when(has_head, |d| {
                d.child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(6.))
                        .when_some(self.title, |row, title| {
                            row.child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.ink)
                                    .child(title),
                            )
                        })
                        .when_some(self.handle, |row, handle| {
                            row.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(theme.muted)
                                    .child(handle),
                            )
                        }),
                )
            })
            .when_some(self.meta, |d, meta| {
                d.child(div().text_color(theme.muted).child(meta))
            })
            .when_some(self.link_label, |d, link| {
                d.child(
                    div()
                        .mt(px(4.))
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.info)
                        .child(link),
                )
            })
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    clicks: usize,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted)
                    .child("Rest or Tab onto a link to preview (600ms delay); glide diagonally into the card cleanly"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_6()
                    .child(
                        HoverCard::new("demo-hvc-alice")
                            .width(px(280.))
                            .trigger(
                                Button::new("demo-hvc-alice-trig")
                                    .secondary()
                                    .label(if self.clicks > 0 {
                                        format!("@alice (opened {})", self.clicks)
                                    } else {
                                        "@alice".into()
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clicks += 1;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_baseline()
                                            .gap(px(6.))
                                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Alice Lin"))
                                            .child(div().text_xs().text_color(theme.muted).child("@alice")),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.muted)
                                            .child("Builds UI infrastructure and desktop tooling."),
                                    )
                                    .child(
                                        Button::new("demo-hvc-inspect")
                                            .ghost()
                                            .small()
                                            .label("Open profile")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.clicks += 1;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        HoverCard::new("demo-hvc-worker")
                            .side(HoverCardSide::Right)
                            .trigger(Button::new("demo-hvc-worker-trig").secondary().label("agent-worker-04"))
                            .child(StoryCardBody {
                                title: Some("agent-worker-04".into()),
                                handle: Some("idle".into()),
                                meta: Some("Last run completed 2m ago · 0 errors".into()),
                                link_label: None,
                            }),
                    ),
            )
    }
}
