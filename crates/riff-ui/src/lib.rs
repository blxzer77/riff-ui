//! riff-ui: a UI component library built on GPUI.

mod a11y;
pub mod avatar;
pub mod badge;
pub mod button;
pub mod card;
pub mod gradient_text;
pub mod icon_button;
pub mod kbd;
pub(crate) mod paint;
pub mod separator;
pub mod skeleton;
pub mod spinner;
pub mod theme;

use std::borrow::Cow;

use gpui_kit::App;

/// Inter Regular and Medium, the two weights riff-uiux uses, registered so
/// every machine draws the tokens' `--riff-font` at the same widths.
const INTER: [&[u8]; 2] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
];

pub(crate) fn inter_font_bytes(weight: gpui_kit::FontWeight) -> &'static [u8] {
    if weight.0 >= 450. { INTER[1] } else { INTER[0] }
}

/// Initializes riff-ui. Call once before using any component.
pub fn init(cx: &mut App) {
    gpui_kit::init(cx);
    for font in INTER {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(font)])
            .expect("bundled Inter parses");
    }
    theme::init(cx);
}
