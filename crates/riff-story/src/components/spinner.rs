//! Spinner: the story's cells, and a demo with sizes, currentColor inheritance, and status rows.

use std::time::Duration;

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::spinner::Spinner;
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &["size", "label", "color", "speedMs"];

/// The live spinner for one cell of `stories/spinner.json`.
pub fn cell(_id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let mut spinner = Spinner::new();
    if let Some(size) = props.f32("size") {
        spinner = spinner.size(px(size));
    }
    if let Some(label) = props.str("label") {
        spinner = spinner.label(label.to_owned());
    }
    if let Some(speed_ms) = props.i64("speedMs") {
        if speed_ms < 0 {
            return Err(format!("negative speedMs {speed_ms}"));
        }
        spinner = spinner.speed(Duration::from_millis(speed_ms as u64));
    }
    if let Some(at) = pin.freeze {
        spinner = spinner.freeze_motion_at(at);
    }

    let color_key = props.str("color").unwrap_or("ink");
    let probe = RiffTheme::new(riff_ui::theme::ThemeMode::Light);
    let _ = resolve_color(color_key, &probe)?;

    Ok(ThemedColor {
        spinner,
        key: color_key.to_owned(),
    }
    .into_any_element())
}

fn resolve_color(key: &str, theme: &RiffTheme) -> Result<Hsla, String> {
    match key {
        "ink" => Ok(theme.ink),
        "muted" => Ok(theme.muted),
        "info" => Ok(theme.info),
        "success" => Ok(theme.success),
        "warning" => Ok(theme.warning),
        "danger" => Ok(theme.danger),
        "purple" => Ok(theme.purple),
        other => Err(format!("unknown color {other:?}")),
    }
}

#[derive(IntoElement)]
struct ThemedColor {
    spinner: Spinner,
    key: String,
}

impl RenderOnce for ThemedColor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let c = resolve_color(&self.key, theme).expect("validated in cell");
        div().text_color(c).child(self.spinner)
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    slow: bool,
    size: f32,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let size = if self.size == 0. { 14. } else { self.size };
        let speed = if self.slow {
            Duration::from_millis(2400)
        } else {
            Duration::from_millis(750)
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(360.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("spn-speed")
                            .small()
                            .secondary()
                            .label(if self.slow {
                                "Speed: 2.4s (reduced)"
                            } else {
                                "Speed: 750ms"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.slow = !this.slow;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("spn-size")
                            .small()
                            .secondary()
                            .label(format!("Size: {size:.0}px"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.size = match this.size as i32 {
                                    0 | 14 => 20.,
                                    20 => 12.,
                                    _ => 14.,
                                };
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
                    .text_color(theme.info)
                    .child(Spinner::new().size(px(size)).speed(speed))
                    .child(div().text_sm().child("Checking domain availability…")),
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
                    .child(
                        Spinner::new()
                            .size(px(size))
                            .speed(speed)
                            .label("Waiting for build"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted)
                            .child("Standalone status (role=\"status\")"),
                    ),
            )
    }
}
