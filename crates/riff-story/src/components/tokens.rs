//! The token palette: every `--riff-*` color of the active mode.

use gpui_kit::*;
use riff_ui::theme::RiffTheme;

pub fn view(_: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_| Tokens).into()
}

struct Tokens;

impl Render for Tokens {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        div()
            .flex()
            .flex_wrap()
            .gap_3()
            .children(theme.colors.entries().into_iter().map(|(name, color)| {
                div()
                    .w(px(150.))
                    .flex()
                    .flex_col()
                    .rounded(theme.radius_lg)
                    .bg(theme.surface)
                    .shadow(theme.shadow_card.clone())
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(48.))
                            .bg(color)
                            .border_b_1()
                            .border_color(theme.border),
                    )
                    .child(
                        div()
                            .p_2()
                            .flex()
                            .flex_col()
                            .text_xs()
                            .child(name)
                            .child(div().text_color(theme.muted).child(hex(color))),
                    )
            }))
    }
}

/// Rounds each channel, unlike `Colorize::to_hex`, which truncates and so
/// shows some palette colors one step off the hex they were parsed from.
fn hex(color: Hsla) -> String {
    let rgba = color.to_rgb();
    let channel = |value: f32| (value * 255.).round() as u8;
    let mut hex = format!(
        "#{:02x}{:02x}{:02x}",
        channel(rgba.r),
        channel(rgba.g),
        channel(rgba.b)
    );
    if rgba.a < 1. {
        hex.push_str(&format!("{:02x}", channel(rgba.a)));
    }
    hex
}
