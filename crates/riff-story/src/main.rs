use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::*;
use riff_ui::theme::{RiffTheme, ThemeMode, set_mode};

struct Story;

impl Render for Story {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let next_mode = if theme.mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };

        div()
            .id("story")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_6()
            .p_8()
            .bg(theme.subtle)
            .text_color(theme.ink)
            .font_family(theme.font_family.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child("riff-ui tokens"))
                    .child(
                        Button::new("toggle-mode")
                            .primary()
                            .label(format!("Switch to {next_mode:?}"))
                            .on_click(move |_, _, cx| set_mode(next_mode, cx)),
                    ),
            )
            .child(div().flex().flex_wrap().gap_3().children(
                theme.colors.entries().into_iter().map(|(name, color)| {
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
                }),
            ))
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

fn main() {
    gpui_kit::application().run(|cx| {
        riff_ui::init(cx);
        if std::env::var("RIFF_STORY_MODE").as_deref() == Ok("dark") {
            set_mode(ThemeMode::Dark, cx);
        }
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Story))
            .expect("failed to open window");
    });
}
