//! riff-ui: a UI component library built on GPUI.

pub mod theme;

use gpui_kit::App;

/// Initializes riff-ui. Call once before using any component.
pub fn init(cx: &mut App) {
    gpui_kit::init(cx);
    theme::init(cx);
}
