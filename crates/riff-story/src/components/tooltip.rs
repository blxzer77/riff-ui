//! Tooltip: the story's cells, and an interactive demo for hover delay, warm
//! neighbor switch, keyboard Tab focus, shortcuts, long wrapping, and flip.

use std::time::Duration;

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::icon_button::{IconButton, IconButtonIcon};
use riff_ui::theme::RiffTheme;
use riff_ui::tooltip::{Tooltip, TooltipAlign, TooltipSide, TooltipState};

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "side",
    "align",
    "offset",
    "content",
    "shortcut",
    "trigger",
    "triggerLabel",
    "triggerIcon",
    "triggerSize",
    "stageWidth",
    "stageHeight",
];

/// The live tooltip for one cell of `stories/tooltip.json`.
pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let side = match props.str("side").unwrap_or("top") {
        "top" => TooltipSide::Top,
        "bottom" => TooltipSide::Bottom,
        "left" => TooltipSide::Left,
        "right" => TooltipSide::Right,
        other => return Err(format!("unknown side {other:?}")),
    };
    let align = match props.str("align").unwrap_or("center") {
        "start" => TooltipAlign::Start,
        "center" => TooltipAlign::Center,
        "end" => TooltipAlign::End,
        other => return Err(format!("unknown align {other:?}")),
    };
    let offset = props.f32("offset").unwrap_or(6.);
    let content = props.str("content").unwrap_or("");
    let trigger = props.str("trigger").unwrap_or("button");
    let trigger_label = props.str("triggerLabel").unwrap_or("Trigger");
    let trigger_icon = props.str("triggerIcon").unwrap_or("bookmark");
    let trigger_size = props.str("triggerSize").unwrap_or("sm");
    let stage_w = props.f32("stageWidth").unwrap_or(180.);
    let stage_h = props.f32("stageHeight").unwrap_or(84.);

    let trigger_el: AnyElement = match trigger {
        "icon-button" => {
            let (side_px, icon_px) = match trigger_size {
                "sm" => (px(32.), px(16.)),
                "md" => (px(40.), px(18.)),
                other => return Err(format!("unknown triggerSize {other:?}")),
            };
            StoryTrigger {
                id: SharedString::from(format!("story-trig-{id}")),
                height: side_px,
                width: Some(side_px),
                icon: Some((
                    SharedString::from(format!("icons/{trigger_icon}.svg")),
                    icon_px,
                )),
                label: None,
            }
            .into_any_element()
        }
        "button" => {
            let height_px = match trigger_size {
                "sm" => px(32.),
                "md" => px(40.),
                "lg" => px(48.),
                other => return Err(format!("unknown triggerSize {other:?}")),
            };
            StoryTrigger {
                id: SharedString::from(format!("story-trig-{id}")),
                height: height_px,
                width: None,
                icon: None,
                label: Some(SharedString::from(trigger_label.to_owned())),
            }
            .into_any_element()
        }
        other => return Err(format!("unknown trigger {other:?}")),
    };

    let mut tip = Tooltip::new(SharedString::from(format!("story-tip-{id}")))
        .side(side)
        .align(align)
        .offset(px(offset))
        .content(content.to_owned())
        .force_state(TooltipState { open: true })
        .inline_preview(true)
        .child(trigger_el);
    if let Some(sc) = props.str("shortcut") {
        tip = tip.shortcut(sc.to_owned());
    }
    if let Some(at) = pin.freeze {
        tip = tip.freeze_motion_at(at);
    }

    let mut stage = div().w(px(stage_w)).h(px(stage_h)).flex();
    match side {
        TooltipSide::Top => {
            stage = stage.items_end().pb(px(8.));
            match align {
                TooltipAlign::Start => stage = stage.justify_start().pl(px(10.)),
                TooltipAlign::Center => stage = stage.justify_center(),
                TooltipAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        TooltipSide::Bottom => {
            stage = stage.items_start().pt(px(8.));
            match align {
                TooltipAlign::Start => stage = stage.justify_start().pl(px(10.)),
                TooltipAlign::Center => stage = stage.justify_center(),
                TooltipAlign::End => stage = stage.justify_end().pr(px(10.)),
            }
        }
        TooltipSide::Left => {
            stage = stage.justify_end().pr(px(8.));
            match align {
                TooltipAlign::Start => stage = stage.items_start().pt(px(8.)),
                TooltipAlign::Center => stage = stage.items_center(),
                TooltipAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
        TooltipSide::Right => {
            stage = stage.justify_start().pl(px(8.));
            match align {
                TooltipAlign::Start => stage = stage.items_start().pt(px(8.)),
                TooltipAlign::Center => stage = stage.items_center(),
                TooltipAlign::End => stage = stage.items_end().pb(px(8.)),
            }
        }
    }

    Ok(stage.child(tip).into_any_element())
}

#[derive(IntoElement)]
struct StoryTrigger {
    id: SharedString,
    height: Pixels,
    width: Option<Pixels>,
    icon: Option<(SharedString, Pixels)>,
    label: Option<SharedString>,
}

impl RenderOnce for StoryTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let mut el = div()
            .id(self.id)
            .h(self.height)
            .flex()
            .items_center()
            .justify_center()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_color(theme.ink);
        if let Some(w) = self.width {
            el = el.w(w);
        } else {
            el = el.px(px(12.));
        }
        if let Some((path, size)) = self.icon {
            el = el.child(svg().path(path).size(size).text_color(theme.ink));
        } else if let Some(label) = self.label {
            el = el
                .text_size(px(13.))
                .line_height(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .child(label);
        }
        el
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    clicks: usize,
    controlled: bool,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let muted = theme.muted;

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("Toolbar (500ms first hover, instant neighbor switch within 300ms, Tab opens at once, Esc dismisses)"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Tooltip::new("demo-bold")
                                    .content("Bold")
                                    .shortcut("⌘B")
                                    .child(
                                        IconButton::new("demo-tb-bold")
                                            .secondary()
                                            .icon(IconButtonIcon::path("icons/bold.svg"))
                                            .label("Bold")
                                            .title("")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.clicks += 1;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                Tooltip::new("demo-italic")
                                    .content("Italic")
                                    .shortcut("⌘I")
                                    .child(
                                        IconButton::new("demo-tb-italic")
                                            .secondary()
                                            .icon(IconButtonIcon::path("icons/italic.svg"))
                                            .label("Italic")
                                            .title(""),
                                    ),
                            )
                            .child(
                                Tooltip::new("demo-search")
                                    .content("Search workspace")
                                    .shortcut("⌘K")
                                    .child(
                                        IconButton::new("demo-tb-search")
                                            .secondary()
                                            .icon(IconButtonIcon::path("icons/search.svg"))
                                            .label("Search")
                                            .title(""),
                                    ),
                            )
                            .child(
                                Tooltip::new("demo-save")
                                    .content("Save changes")
                                    .shortcut("⌘S")
                                    .child(Button::new("demo-btn-save").label("Save")),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child("Four sides, delay=0, controlled, and truncated button"))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_3()
                            .child(
                                Tooltip::new("demo-left")
                                    .left()
                                    .delay(Duration::ZERO)
                                    .content("Left side")
                                    .child(Button::new("demo-b-left").secondary().label("Left")),
                            )
                            .child(
                                Tooltip::new("demo-bottom")
                                    .bottom()
                                    .delay(Duration::ZERO)
                                    .content("Bottom side")
                                    .child(Button::new("demo-b-bottom").secondary().label("Bottom")),
                            )
                            .child(
                                Tooltip::new("demo-right")
                                    .right()
                                    .delay(Duration::ZERO)
                                    .content("Right side")
                                    .child(Button::new("demo-b-right").secondary().label("Right")),
                            )
                            .child(
                                Tooltip::new("demo-controlled")
                                    .content("Controlled tooltip")
                                    .open(Some(self.controlled))
                                    .child(
                                        Button::new("demo-b-ctrl")
                                            .secondary()
                                            .label(if self.controlled { "Pin: ON" } else { "Pin: OFF" })
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.controlled = !this.controlled;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div().w(px(110.)).child(
                                    Button::new("demo-trunc")
                                        .secondary()
                                        .label("Generate full quarterly financial summary"),
                                ),
                            ),
                    ),
            )
    }
}
