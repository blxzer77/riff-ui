//! Crisp vector painting for the seven default tool logos (`14×14` inside a
//! `26×26` tile) plus fallback tile initial when an external icon is absent.

use gpui_kit::{
    App, Bounds, Corners, FontWeight, Hsla, IntoElement, ParentElement as _, PathBuilder, Pixels,
    RenderOnce, SharedString, Styled, Window, canvas, div, fill, font, linear_color_stop,
    linear_gradient, point, px,
};

use super::style::LOGO_SIZE;
use crate::paint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandLogo {
    Gmail,
    AskSpark,
    ClickUp,
    HubSpot,
    Calendar,
    Slack,
    Notion,
}

#[derive(IntoElement)]
pub(super) struct LogoGlyph {
    logo: CommandLogo,
    ink: Hsla,
}

impl LogoGlyph {
    pub fn new(logo: CommandLogo, ink: Hsla) -> Self {
        Self { logo, ink }
    }
}

impl RenderOnce for LogoGlyph {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let logo = self.logo;
        let ink = self.ink;
        div().size(LOGO_SIZE).flex_none().child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, cx| paint_logo(bounds, logo, ink, window, cx),
            )
            .size_full(),
        )
    }
}

fn paint_logo(
    bounds: Bounds<Pixels>,
    logo: CommandLogo,
    ink: Hsla,
    window: &mut Window,
    cx: &mut App,
) {
    match logo {
        CommandLogo::Gmail => paint_gmail(bounds, window),
        CommandLogo::AskSpark => paint_spark(bounds, window),
        CommandLogo::ClickUp => paint_clickup(bounds, window),
        CommandLogo::HubSpot => paint_hubspot(bounds, window),
        CommandLogo::Calendar => paint_calendar(bounds, window, cx),
        CommandLogo::Slack => paint_slack(bounds, window),
        CommandLogo::Notion => paint_notion(bounds, ink, window),
    }
}

fn fit_rect(bounds: Bounds<Pixels>, vw: f32, vh: f32) -> (f32, f32, f32) {
    let sx = bounds.size.width.as_f32() / vw;
    let sy = bounds.size.height.as_f32() / vh;
    let s = sx.min(sy);
    let ox = bounds.left().as_f32() + (bounds.size.width.as_f32() - vw * s) * 0.5;
    let oy = bounds.top().as_f32() + (bounds.size.height.as_f32() - vh * s) * 0.5;
    (ox, oy, s)
}

fn pt(ox: f32, oy: f32, s: f32, x: f32, y: f32) -> gpui_kit::Point<Pixels> {
    point(px(ox + x * s), px(oy + y * s))
}

fn paint_gmail(bounds: Bounds<Pixels>, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 56.41, 44.87);
    let p = |x, y| pt(ox, oy, s, x, y);

    let mut left = PathBuilder::fill();
    left.move_to(p(0., 6.5));
    left.line_to(p(12.18, 6.5));
    left.line_to(p(12.18, 44.87));
    left.line_to(p(3.6, 44.87));
    left.curve_to(p(0., 41.2), p(0., 44.87));
    left.close();
    if let Ok(path) = left.build() {
        window.paint_path(path, paint::hex(0xFC413D));
    }

    let mut right = PathBuilder::fill();
    right.move_to(p(44.23, 6.5));
    right.line_to(p(56.41, 6.5));
    right.line_to(p(56.41, 41.2));
    right.curve_to(p(52.8, 44.87), p(56.41, 44.87));
    right.line_to(p(44.23, 44.87));
    right.close();
    if let Ok(path) = right.build() {
        let grad = linear_gradient(
            180.,
            linear_color_stop(paint::hex(0x60D673), 0.),
            linear_color_stop(paint::hex(0x3186FF), 1.),
        );
        window.paint_path(path, grad);
    }

    let mut flap = PathBuilder::fill();
    flap.move_to(p(1.2, 2.5));
    flap.curve_to(p(10., 1.4), p(4.5, -1.0));
    flap.line_to(p(28.2, 16.7));
    flap.line_to(p(46.4, 1.4));
    flap.curve_to(p(55.2, 2.5), p(51.9, -1.0));
    flap.curve_to(p(54.2, 10.7), p(57.1, 6.5));
    flap.line_to(p(28.2, 31.8));
    flap.line_to(p(2.2, 10.7));
    flap.curve_to(p(1.2, 2.5), p(-0.7, 6.5));
    flap.close();
    if let Ok(path) = flap.build() {
        let grad = linear_gradient(
            90.,
            linear_color_stop(paint::hex(0xFC413D), 0.35),
            linear_color_stop(paint::hex(0xFEB10C), 0.92),
        );
        window.paint_path(path, grad);
    }
}

fn paint_spark(bounds: Bounds<Pixels>, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 24., 24.);
    let p = |x, y| pt(ox, oy, s, x, y);

    let mut star = PathBuilder::fill();
    star.move_to(p(12.0, 2.5));
    star.curve_to(p(16.4, 10.1), p(12.8, 8.7));
    star.line_to(p(20.5, 11.2));
    star.line_to(p(20.5, 12.8));
    star.curve_to(p(12.0, 21.5), p(14.5, 14.5));
    star.line_to(p(10.4, 21.5));
    star.curve_to(p(6.0, 13.9), p(9.6, 15.3));
    star.line_to(p(1.9, 12.8));
    star.line_to(p(1.9, 11.2));
    star.curve_to(p(10.4, 2.5), p(8.2, 9.6));
    star.close();
    if let Ok(path) = star.build() {
        let grad = linear_gradient(
            135.,
            linear_color_stop(paint::hex(0x7C5CFF), 0.),
            linear_color_stop(paint::hex(0x2F9BFF), 1.),
        );
        window.paint_path(path, grad);
    }
}

fn paint_clickup(bounds: Bounds<Pixels>, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 49.7, 59.7);
    let p = |x, y| pt(ox, oy, s, x, y);

    let mut bot = PathBuilder::fill();
    bot.move_to(p(0., 45.9));
    bot.line_to(p(9.16, 38.8));
    bot.curve_to(p(24.95, 48.17), p(16.5, 48.17));
    bot.curve_to(p(40.37, 38.9), p(33.3, 48.17));
    bot.line_to(p(49.7, 45.8));
    bot.curve_to(p(24.95, 59.68), p(39.5, 59.68));
    bot.curve_to(p(0., 45.9), p(10.5, 59.68));
    bot.close();
    if let Ok(path) = bot.build() {
        let grad = linear_gradient(
            90.,
            linear_color_stop(paint::hex(0x8930FD), 0.),
            linear_color_stop(paint::hex(0x49CCF9), 1.),
        );
        window.paint_path(path, grad);
    }

    let mut top = PathBuilder::fill();
    top.move_to(p(24.95, 15.33));
    top.line_to(p(8.63, 29.39));
    top.line_to(p(1.1, 20.59));
    top.line_to(p(25.04, 0.));
    top.line_to(p(48.8, 20.59));
    top.line_to(p(41.19, 29.3));
    top.close();
    if let Ok(path) = top.build() {
        let grad = linear_gradient(
            90.,
            linear_color_stop(paint::hex(0xFF02F0), 0.),
            linear_color_stop(paint::hex(0xFFC800), 1.),
        );
        window.paint_path(path, grad);
    }
}

fn paint_hubspot(bounds: Bounds<Pixels>, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 24., 24.);
    let p = |x, y| pt(ox, oy, s, x, y);
    let orange = paint::hex(0xFF7A59);

    let mut spokes = PathBuilder::stroke(px(s * 1.9));
    spokes.move_to(p(17.1, 14.0));
    spokes.line_to(p(17.1, 3.3));
    spokes.move_to(p(17.1, 14.0));
    spokes.line_to(p(4.4, 4.1));
    spokes.move_to(p(13.0, 18.0));
    spokes.line_to(p(9.1, 21.8));
    if let Ok(path) = spokes.build() {
        window.paint_path(path, orange);
    }

    let circle = |cx: f32, cy: f32, r: f32, col: Hsla, w: &mut Window| {
        let b = Bounds::new(
            p(cx - r, cy - r),
            gpui_kit::size(px(r * 2. * s), px(r * 2. * s)),
        );
        w.paint_quad(fill(b, col).corner_radii(Corners::all(px(r * s))));
    };
    circle(17.1, 14.0, 5.2, orange, window);
    circle(17.1, 14.0, 2.3, paint::hex(0xffffff), window);
    circle(17.1, 3.0, 2.1, orange, window);
    circle(4.3, 4.0, 2.3, orange, window);
    circle(8.9, 21.8, 1.9, orange, window);
}

fn paint_calendar(bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let (ox, oy, s) = fit_rect(bounds, 24., 24.);
    let p = |x, y| pt(ox, oy, s, x, y);
    let blue = paint::hex(0x4285F4);

    let top = Bounds::new(p(1.5, 1.5), gpui_kit::size(px(21. * s), px(5. * s)));
    window.paint_quad(fill(top, blue).corner_radii(Corners::all(px(1.5 * s))));

    let frame = Bounds::new(p(1.5, 4.5), gpui_kit::size(px(21. * s), px(18. * s)));
    window.paint_quad(gpui_kit::quad(
        frame,
        Corners::all(px(2. * s)),
        gpui_kit::transparent_black(),
        gpui_kit::Edges::all(px(2. * s)),
        blue,
        gpui_kit::BorderStyle::Solid,
    ));

    let label: SharedString = "31".into();
    let run = gpui_kit::TextRun {
        len: label.len(),
        font: gpui_kit::Font {
            weight: FontWeight::BOLD,
            ..font("Inter")
        },
        color: blue,
        ..Default::default()
    };
    let line = window
        .text_system()
        .shape_line(label, px(8.5 * s), &[run], None);
    let pos = point(
        px(ox + 12. * s) - line.width * 0.5,
        px(oy + 13.5 * s) - line.ascent * 0.5,
    );
    let _ = line.paint(
        pos,
        px(10. * s),
        gpui_kit::TextAlign::Left,
        None,
        window,
        cx,
    );
}

fn paint_slack(bounds: Bounds<Pixels>, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 49.9, 49.9);
    let pill = |x: f32, y: f32, w: f32, h: f32, r: f32, col: Hsla, win: &mut Window| {
        let b = Bounds::new(pt(ox, oy, s, x, y), gpui_kit::size(px(w * s), px(h * s)));
        win.paint_quad(fill(b, col).corner_radii(Corners::all(px(r * s))));
    };
    let blue = paint::hex(0x36C5F0);
    let green = paint::hex(0x2EB67D);
    let red = paint::hex(0xE01E5A);
    let yellow = paint::hex(0xECB22E);

    // Top-left blue
    pill(13.1, 0., 10.5, 10.5, 5.2, blue, window);
    pill(0., 13.1, 23.6, 10.5, 5.2, blue, window);
    // Top-right green
    pill(26.3, 0., 10.5, 23.6, 5.2, green, window);
    pill(39.4, 13.1, 10.5, 10.5, 5.2, green, window);
    // Bottom-left red
    pill(0., 26.3, 10.5, 10.5, 5.2, red, window);
    pill(13.1, 26.3, 10.5, 23.6, 5.2, red, window);
    // Bottom-right yellow
    pill(26.3, 39.4, 10.5, 10.5, 5.2, yellow, window);
    pill(26.3, 26.3, 23.6, 10.5, 5.2, yellow, window);
}

fn paint_notion(bounds: Bounds<Pixels>, ink: Hsla, window: &mut Window) {
    let (ox, oy, s) = fit_rect(bounds, 24., 24.);
    let p = |x, y| pt(ox, oy, s, x, y);

    let mut box_outline = PathBuilder::stroke(px(s * 1.8));
    box_outline.move_to(p(4.2, 5.0));
    box_outline.line_to(p(19.0, 4.0));
    box_outline.line_to(p(21.0, 20.5));
    box_outline.line_to(p(6.0, 21.5));
    box_outline.close();
    if let Ok(path) = box_outline.build() {
        window.paint_path(path, ink);
    }

    let mut letter_n = PathBuilder::stroke(px(s * 1.9));
    letter_n.move_to(p(9.2, 17.5));
    letter_n.line_to(p(9.2, 8.8));
    letter_n.line_to(p(15.6, 17.2));
    letter_n.line_to(p(15.6, 8.4));
    if let Ok(path) = letter_n.build() {
        window.paint_path(path, ink);
    }
}
