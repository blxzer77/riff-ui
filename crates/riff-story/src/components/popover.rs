//! Popover: the story's cells, and an interactive demo with focus handling,
//! outside-click dismissal, Escape restoration, and nested placement.

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::popover::{Popover, PopoverAlign, PopoverSide};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "side",
    "align",
    "offset",
    "title",
    "description",
    "body",
    "width",
    "unstyled",
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
        "bottom" => PopoverSide::Bottom,
        "top" => PopoverSide::Top,
        "left" => PopoverSide::Left,
        "right" => PopoverSide::Right,
        other => return Err(format!("unknown side {other:?}")),
    };
    let align = match props.str("align").unwrap_or("start") {
        "start" => PopoverAlign::Start,
        "center" => PopoverAlign::Center,
        "end" => PopoverAlign::End,
        other => return Err(format!("unknown align {other:?}")),
    };
    let offset = props.f32("offset").unwrap_or(6.);
    let unstyled = props.bool("unstyled");
    let trigger_label = props.str("triggerLabel").unwrap_or("Open").to_owned();
    let trigger_h = props.f32("triggerHeight").unwrap_or(32.);
    let stage_w = props.f32("stageWidth").unwrap_or(240.);
    let stage_h = props.f32("stageHeight").unwrap_or(140.);

    let mut pop = Popover::new(SharedString::from(format!("story-pop-{id}")))
        .side(side)
        .align(align)
        .offset(px(offset))
        .unstyled(unstyled)
        .open(Some(true))
        .inline_preview(true)
        .trigger(StoryTrigger {
            id: SharedString::from(format!("story-pop-trig-{id}")),
            label: trigger_label.into(),
            height: px(trigger_h),
        });
    if let Some(title) = props.str("title") {
        pop = pop.title(title.to_owned());
    }
    if let Some(desc) = props.str("description") {
        pop = pop.description(desc.to_owned());
    }
    if let Some(w) = props.f32("width") {
        pop = pop.width(px(w));
    }
    if let Some(at) = pin.freeze {
        pop = pop.freeze_motion_at(at);
    }
    if let Some(body) = props.str("body") {
        let text = body.to_owned();
        if unstyled {
            pop = pop.child(UnstyledStoryPanel(text.into()));
        } else {
            pop = pop.child(div().child(text));
        }
    }

    let mut stage = div().w(px(stage_w)).h(px(stage_h)).flex();
    match side {
        PopoverSide::Bottom => {
            stage = stage.items_start().pt(px(8.));
            match align {
                PopoverAlign::Start => stage = stage.justify_start().pl(px(10.)),
                PopoverAlign::Center => stage = stage.justify_center(),
                PopoverAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        PopoverSide::Top => {
            stage = stage.items_end().pb(px(8.));
            match align {
                PopoverAlign::Start => stage = stage.justify_start().pl(px(10.)),
                PopoverAlign::Center => stage = stage.justify_center(),
                PopoverAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        PopoverSide::Left => {
            stage = stage.justify_end().pr(px(8.));
            match align {
                PopoverAlign::Start => stage = stage.items_start().pt(px(8.)),
                PopoverAlign::Center => stage = stage.items_center(),
                PopoverAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
        PopoverSide::Right => {
            stage = stage.justify_start().pl(px(8.));
            match align {
                PopoverAlign::Start => stage = stage.items_start().pt(px(8.)),
                PopoverAlign::Center => stage = stage.items_center(),
                PopoverAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
    }

    Ok(stage.child(pop).into_any_element())
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

#[derive(IntoElement)]
struct UnstyledStoryPanel(SharedString);

impl RenderOnce for UnstyledStoryPanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        div()
            .px(px(12.))
            .py(px(8.))
            .rounded(px(8.))
            .border_1()
            .border_dashed()
            .border_color(theme.border_strong)
            .bg(theme.subtle)
            .text_color(theme.ink)
            .text_size(px(12.5))
            .line_height(px(18.))
            .child(self.0)
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    invite_open: bool,
    invites_sent: usize,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = RiffTheme::global(cx).muted;
        let weak = cx.entity().downgrade();
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Click to open; focuses first inner button, Esc returns focus, outside press closes"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_4()
                    .child(
                        Popover::new("demo-invite")
                            .title("Invite to workspace")
                            .description("They get a link to join this project.")
                            .align(PopoverAlign::Start)
                            .width(px(280.))
                            .open(Some(self.invite_open))
                            .on_close(move |_, cx| {
                                _ = weak.update(cx, |this, cx| {
                                    this.invite_open = false;
                                    cx.notify();
                                });
                            })
                            .trigger(
                                Button::new("demo-invite-btn")
                                    .label(if self.invites_sent > 0 {
                                        format!("Invite ({})", self.invites_sent)
                                    } else {
                                        "Invite members".into()
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.invite_open = !this.invite_open;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        Button::new("demo-pop-cancel")
                                            .secondary()
                                            .small()
                                            .label("Cancel")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.invite_open = false;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("demo-pop-send")
                                            .small()
                                            .label("Send invite")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.invites_sent += 1;
                                                this.invite_open = false;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        Popover::new("demo-uncontrolled")
                            .side(PopoverSide::Right)
                            .title("Uncontrolled")
                            .description("Managed internally without open prop.")
                            .trigger(Button::new("demo-unctrl-btn").secondary().label("Open right"))
                            .child(div().child("Closes on Esc, outside press, or Tab out.")),
                    ),
            )
    }
}
