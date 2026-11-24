//! Badge: the story's cells, and a demo with live pulses and dynamic counts.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::badge::{Badge, BadgeIcon, BadgeSize, BadgeTone};
use riff_ui::button::Button;
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "tone", "size", "dot", "live", "icon", "label", "count", "max",
];

/// The live badge for one cell of `stories/badge.json`.
pub fn cell(_id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let tone = match props.str("tone").unwrap_or("neutral") {
        "neutral" => BadgeTone::Neutral,
        "info" => BadgeTone::Info,
        "success" => BadgeTone::Success,
        "warning" => BadgeTone::Warning,
        "danger" => BadgeTone::Danger,
        "purple" => BadgeTone::Purple,
        other => return Err(format!("unknown tone {other:?}")),
    };
    let size = match props.str("size").unwrap_or("sm") {
        "sm" => BadgeSize::Sm,
        "md" => BadgeSize::Md,
        other => return Err(format!("unknown size {other:?}")),
    };

    let mut badge = Badge::new()
        .tone(tone)
        .size(size)
        .dot(props.bool("dot"))
        .live(props.bool("live"));
    if let Some(name) = props.str("icon") {
        badge = badge.icon(BadgeIcon::path(format!("icons/{name}.svg")));
    }
    if let Some(label) = props.str("label") {
        badge = badge.label(label.to_owned());
    }
    if let Some(count) = props.i64("count") {
        badge = badge.count(count);
    }
    if let Some(max) = props.i64("max") {
        badge = badge.max(max);
    }
    if let Some(at) = pin.freeze {
        badge = badge.freeze_motion_at(at);
    }
    Ok(badge.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

struct Demo {
    waiting: i64,
}

impl Default for Demo {
    fn default() -> Self {
        Self { waiting: 3 }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let row = |title: &'static str, badge: Badge| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py_2()
                .rounded(theme.radius)
                .border_1()
                .border_color(theme.border)
                .child(div().text_sm().child(title))
                .child(badge)
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(360.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(row(
                        "Build workspace",
                        Badge::new().info().live(true).label("Running"),
                    ))
                    .child(row(
                        "Approve migration",
                        Badge::new().warning().dot(true).label("Needs you"),
                    ))
                    .child(row(
                        "Weekly usage report",
                        Badge::new().success().label("Done"),
                    ))
                    .child(row("Deploy staging", Badge::new().danger().label("Failed")))
                    .child(row(
                        "Nightly backup",
                        Badge::new()
                            .neutral()
                            .icon(IconName::Clock)
                            .label("Scheduled"),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().child("Needs you"))
                    .child(
                        Badge::new()
                            .warning()
                            .count(self.waiting)
                            .accessibility_label(format!("{} waiting", self.waiting)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("badge-dec")
                            .secondary()
                            .small()
                            .label("-1")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.waiting = (this.waiting - 1).max(0);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("badge-inc")
                            .secondary()
                            .small()
                            .label("+10")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.waiting += 10;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("badge-burst")
                            .ghost()
                            .small()
                            .label("128")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.waiting = 128;
                                cx.notify();
                            })),
                    ),
            )
    }
}
