//! Gradient Text: the story's cells, and a live replayable headline demo.

use gpui_kit::*;
use riff_ui::button::{Button, ButtonSize};
use riff_ui::gradient_text::{GradientText, GradientTextAs};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "text",
    "prefix",
    "colors",
    "speed",
    "sweep",
    "as",
    "fontSize",
    "fontWeight",
    "lineHeight",
];

fn parse_hex_color(s: &str) -> Result<Hsla, String> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| format!("expected #rrggbb color, got {s:?}"))?;
    if hex.len() != 6 {
        return Err(format!("expected 6-digit hex color, got {s:?}"));
    }
    let rgb = u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex color {s:?}"))?;
    Ok(gpui_kit::rgb(rgb).into())
}

fn parse_as(s: &str) -> Result<GradientTextAs, String> {
    match s {
        "span" => Ok(GradientTextAs::Span),
        "h1" => Ok(GradientTextAs::H1),
        "h2" => Ok(GradientTextAs::H2),
        "p" => Ok(GradientTextAs::P),
        other => Err(format!("unknown as {other:?}")),
    }
}

/// The live gradient text for one cell of `stories/gradient-text.json`.
pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let text = props
        .str("text")
        .ok_or_else(|| "missing text prop".to_string())?;
    let mut gt = GradientText::new(text.to_owned()).id(SharedString::from(format!("story-{id}")));

    if let Some(colors) = props.strings("colors") {
        let parsed: Vec<Hsla> = colors
            .into_iter()
            .map(parse_hex_color)
            .collect::<Result<_, _>>()?;
        gt = gt.colors(parsed);
    }
    if let Some(speed) = props.f32("speed") {
        gt = gt.speed_secs(speed);
    }
    if props.keys().any(|k| k == "sweep") {
        gt = gt.sweep(props.bool("sweep"));
    }
    if let Some(as_el) = props.str("as") {
        gt = gt.as_element(parse_as(as_el)?);
    }
    if let Some(at) = pin.freeze {
        gt = gt.freeze_motion_at(at);
    }

    let font_size = px(props.f32("fontSize").unwrap_or(14.));
    let line_height = px(props.f32("lineHeight").unwrap_or(20.));
    let weight = match props.i64("fontWeight").unwrap_or(400) {
        500 => FontWeight::MEDIUM,
        600 => FontWeight::SEMIBOLD,
        700 => FontWeight::BOLD,
        _ => FontWeight::NORMAL,
    };

    if let Some(prefix) = props.str("prefix") {
        return Ok(div()
            .flex()
            .items_baseline()
            .text_size(font_size)
            .line_height(line_height)
            .font_weight(weight)
            .child(prefix.to_owned())
            .child(gt)
            .into_any_element());
    }

    Ok(div()
        .flex()
        .text_size(font_size)
        .line_height(line_height)
        .font_weight(weight)
        .child(gt)
        .into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    replay_gen: usize,
    custom: bool,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let mut gt = GradientText::new("feel.").as_element(GradientTextAs::H2);
        if self.custom {
            gt = gt.colors([
                rgb(0x0284c7).into(),
                rgb(0x14b3a0).into(),
                rgb(0x38bdf8).into(),
                rgb(0x5fc88a).into(),
            ]);
        }

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .id(("gt-demo-line", self.replay_gen))
                    .flex()
                    .items_baseline()
                    .text_size(px(28.))
                    .line_height(px(34.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.ink)
                    .child("Interfaces people actually ")
                    .child(gt),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("gt-replay")
                            .secondary()
                            .size(ButtonSize::Sm)
                            .label("Replay")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.replay_gen += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("gt-palette")
                            .secondary()
                            .size(ButtonSize::Sm)
                            .label(if self.custom {
                                "Palette: Ocean"
                            } else {
                                "Palette: Brand"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.custom = !this.custom;
                                cx.notify();
                            })),
                    ),
            )
    }
}
