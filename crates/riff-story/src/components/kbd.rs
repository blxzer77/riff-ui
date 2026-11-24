//! Kbd: the story's cells, and a demo with platform and size toggles.

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::kbd::{Kbd, KbdPlatform, KbdSize};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &["keys", "label", "size", "platform"];

/// The live key chip for one cell of `stories/kbd.json`.
pub fn cell(_id: &str, props: &Props, _pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let size = match props.str("size").unwrap_or("sm") {
        "sm" => KbdSize::Sm,
        "md" => KbdSize::Md,
        other => return Err(format!("unknown size {other:?}")),
    };
    let mut kbd = Kbd::new().size(size);
    if let Some(platform) = props.str("platform") {
        kbd = kbd.platform(match platform {
            "mac" => KbdPlatform::Mac,
            "other" => KbdPlatform::Other,
            other => return Err(format!("unknown platform {other:?}")),
        });
    }
    if let Some(keys) = props.strings("keys") {
        kbd = kbd.keys(keys.into_iter().map(ToOwned::to_owned));
    }
    if let Some(label) = props.str("label") {
        kbd = kbd.label(label.to_owned());
    }
    Ok(kbd.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

struct Demo {
    platform: KbdPlatform,
    size: KbdSize,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            platform: KbdPlatform::Mac,
            size: KbdSize::Sm,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let row = |title: &'static str, keys: &[&'static str]| {
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
                .child(
                    Kbd::new()
                        .size(self.size)
                        .platform(self.platform)
                        .keys(keys.iter().copied()),
                )
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(340.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("kbd-mac")
                            .small()
                            .variant(if self.platform == KbdPlatform::Mac {
                                riff_ui::button::ButtonVariant::Secondary
                            } else {
                                riff_ui::button::ButtonVariant::Ghost
                            })
                            .label("Mac")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.platform = KbdPlatform::Mac;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("kbd-other")
                            .small()
                            .variant(if self.platform == KbdPlatform::Other {
                                riff_ui::button::ButtonVariant::Secondary
                            } else {
                                riff_ui::button::ButtonVariant::Ghost
                            })
                            .label("Windows / Linux")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.platform = KbdPlatform::Other;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("kbd-size")
                            .small()
                            .secondary()
                            .label(if self.size == KbdSize::Sm { "sm" } else { "md" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.size = match this.size {
                                    KbdSize::Sm => KbdSize::Md,
                                    KbdSize::Md => KbdSize::Sm,
                                };
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(row("Search runs and files", &["mod", "K"]))
                    .child(row("Run command", &["mod", "enter"]))
                    .child(row("Approve and continue", &["mod", "shift", "A"]))
                    .child(row("Close panel", &["esc"]))
                    .child(row("Delete entry", &["ctrl", "alt", "delete"])),
            )
    }
}
