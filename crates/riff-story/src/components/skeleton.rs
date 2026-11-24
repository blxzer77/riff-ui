//! Skeleton: the story's cells, and an interactive Loading / loaded toggle demo.

use std::time::Duration;

use gpui_kit::*;
use riff_ui::badge::{Badge, BadgeTone};
use riff_ui::button::Button;
use riff_ui::skeleton::{Loading, Skeleton};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "containerWidth",
    "width",
    "height",
    "radius",
    "circle",
    "lineHeight",
    "preset",
    "loadingAtMs",
];

fn apply_freeze(skl: Skeleton, pin: Pin) -> Skeleton {
    match pin.freeze {
        Some(at) => skl.freeze_motion_at(at),
        None => skl,
    }
}

/// The live skeleton or loading block for one cell of `stories/skeleton.json`.
pub fn cell(_id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    if let Some(preset) = props.str("preset") {
        return match preset {
            "row" => Ok(div()
                .flex()
                .items_center()
                .gap(px(10.))
                .w(px(220.))
                .child(apply_freeze(
                    Skeleton::new()
                        .width(px(16.))
                        .height(px(16.))
                        .radius(px(4.)),
                    pin,
                ))
                .child(apply_freeze(
                    Skeleton::new()
                        .line_height(px(20.))
                        .width(px(164.))
                        .height(px(10.)),
                    pin,
                ))
                .into_any_element()),
            "loading" => {
                let mut loading = Loading::new()
                    .label("Loading runs")
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .w(px(180.))
                    .child(apply_freeze(
                        Skeleton::new()
                            .line_height(px(20.))
                            .width(px(160.))
                            .height(px(10.)),
                        pin,
                    ))
                    .child(apply_freeze(
                        Skeleton::new()
                            .line_height(px(20.))
                            .width(px(120.))
                            .height(px(10.)),
                        pin,
                    ));
                if let Some(at_ms) = props.i64("loadingAtMs") {
                    loading = loading.freeze_motion_at(Duration::from_millis(at_ms.max(0) as u64));
                }
                Ok(loading.into_any_element())
            }
            other => Err(format!("unknown preset {other:?}")),
        };
    }

    let mut skl = apply_freeze(Skeleton::new(), pin);
    if let Some(width) = props.f32("width") {
        skl = skl.width(px(width));
    }
    if let Some(height) = props.f32("height") {
        skl = skl.height(px(height));
    }
    if props.bool("circle") {
        skl = skl.circle();
    } else if let Some(radius) = props.f32("radius") {
        skl = skl.radius(px(radius));
    }
    if let Some(lh) = props.f32("lineHeight") {
        skl = skl.line_height(px(lh));
    }

    if let Some(cw) = props.f32("containerWidth") {
        Ok(div().flex().w(px(cw)).child(skl).into_any_element())
    } else {
        Ok(skl.into_any_element())
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

struct Demo {
    loading: bool,
    generation: usize,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            loading: true,
            generation: 0,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let gen_id = self.generation;

        div()
            .flex()
            .flex_col()
            .gap_4()
            .w(px(340.))
            .child(
                div().flex().items_center().gap_2().child(
                    Button::new("skl-toggle")
                        .small()
                        .secondary()
                        .label(if self.loading {
                            "Show loaded rows"
                        } else {
                            "Reload with 150ms delay"
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.loading = !this.loading;
                            if this.loading {
                                this.generation += 1;
                            }
                            cx.notify();
                        })),
                ),
            )
            .child(
                div()
                    .p_3()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .child(if self.loading {
                        div()
                            .id(("skl-demo-list", gen_id))
                            .child(
                                Loading::new()
                                    .label("Loading runs")
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .children([212., 164., 176.].into_iter().map(|w| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Skeleton::new()
                                                    .width(px(16.))
                                                    .height(px(16.))
                                                    .radius(px(4.)),
                                            )
                                            .child(
                                                Skeleton::new()
                                                    .line_height(px(20.))
                                                    .width(px(w))
                                                    .height(px(10.)),
                                            )
                                    })),
                            )
                            .into_any_element()
                    } else {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(
                                [
                                    (
                                        "feat: add skeleton placeholder",
                                        "passed",
                                        BadgeTone::Success,
                                    ),
                                    ("chore: sync theme tokens", "passed", BadgeTone::Success),
                                    ("test: verify reduced motion", "running", BadgeTone::Info),
                                ]
                                .into_iter()
                                .map(|(title, status, tone)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .h(px(20.))
                                        .child(div().text_sm().child(title))
                                        .child(Badge::new().tone(tone).label(status))
                                }),
                            )
                            .into_any_element()
                    }),
            )
    }
}
