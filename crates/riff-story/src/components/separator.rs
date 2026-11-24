//! Separator: the story's cells, and a demo with horizontal, labelled, and vertical dividers.

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::separator::{Separator, SeparatorOrientation};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &["orientation", "label", "decorative", "width", "height"];

/// The live separator for one cell of `stories/separator.json`.
pub fn cell(_id: &str, props: &Props, _pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let orientation = match props.str("orientation").unwrap_or("horizontal") {
        "horizontal" => SeparatorOrientation::Horizontal,
        "vertical" => SeparatorOrientation::Vertical,
        other => return Err(format!("unknown orientation {other:?}")),
    };
    let mut sep = Separator::new()
        .orientation(orientation)
        .decorative(props.bool_or("decorative", true));
    if let Some(label) = props.str("label") {
        sep = sep.label(label.to_owned());
    }
    if let Some(height) = props.f32("height") {
        sep = sep.length(px(height));
    }
    if let Some(width) = props.f32("width") {
        Ok(div().flex().w(px(width)).child(sep).into_any_element())
    } else {
        Ok(sep.into_any_element())
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    tall_vertical: bool,
    narrow_box: bool,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let box_w = if self.narrow_box { 160. } else { 320. };
        let vert_h = if self.tall_vertical { 24. } else { 18. };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .w(px(360.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("sep-width")
                            .small()
                            .secondary()
                            .label(if self.narrow_box {
                                "Container: 160px"
                            } else {
                                "Container: 320px"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.narrow_box = !this.narrow_box;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("sep-vert-h")
                            .small()
                            .secondary()
                            .label(if self.tall_vertical {
                                "Vertical: 24px"
                            } else {
                                "Vertical: 18px"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tall_vertical = !this.tall_vertical;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .child(div().text_sm().child("12 runs"))
                    .child(Separator::new().vertical().length(px(vert_h)))
                    .child(div().text_sm().text_color(theme.danger).child("1 failed")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(box_w))
                    .p_3()
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .child(div().text_sm().child("Continue with GitHub"))
                    .child(Separator::new().label("or"))
                    .child(div().text_sm().child("Continue with email"))
                    .child(Separator::new())
                    .child(
                        Separator::new()
                            .label("Earlier activity in this workspace")
                            .decorative(false),
                    ),
            )
    }
}
