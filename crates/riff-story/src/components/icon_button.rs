//! Icon Button: the story's cells, and a demo to click and toggle.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::icon_button::{
    IconButton, IconButtonIcon, IconButtonSize, IconButtonState, IconButtonVariant,
};

use crate::registry::{DemoFn, Pin};
use crate::story::{PinnedState, Props};

const PROPS: &[&str] = &["variant", "size", "icon", "label", "pressed", "disabled"];

/// The live icon button for one cell of `stories/icon-button.json`.
pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let variant = match props.str("variant").unwrap_or("ghost") {
        "ghost" => IconButtonVariant::Ghost,
        "secondary" => IconButtonVariant::Secondary,
        "primary" => IconButtonVariant::Primary,
        other => return Err(format!("unknown variant {other:?}")),
    };
    let size = match props.str("size").unwrap_or("md") {
        "sm" => IconButtonSize::Sm,
        "md" => IconButtonSize::Md,
        other => return Err(format!("unknown size {other:?}")),
    };

    let mut button = IconButton::new(SharedString::from(format!("story-{id}")))
        .variant(variant)
        .size(size)
        .disabled(props.bool("disabled"));
    if let Some(name) = props.str("icon") {
        button = button.icon(IconButtonIcon::path(format!("icons/{name}.svg")));
    }
    if let Some(label) = props.str("label") {
        button = button.label(label.to_owned());
    }
    if props.keys().any(|k| k == "pressed") {
        button = button.pressed(props.bool("pressed"));
    }
    // A pressed button is under the pointer too, as riff-uiux's `:active`
    // cells are pinned with `:hover` (tools/reference/gallery.jsx).
    button = button.force_state(IconButtonState {
        hovered: matches!(pin.state, Some(PinnedState::Hover | PinnedState::Active)),
        pressed: pin.state == Some(PinnedState::Active),
        focus_visible: pin.state == Some(PinnedState::Focus),
    });
    Ok(button.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    pinned: bool,
    liked: bool,
    runs: usize,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        IconButton::new("demo-search")
                            .secondary()
                            .icon(IconName::Search)
                            .label("Search runs"),
                    )
                    .child(
                        IconButton::new("demo-pin")
                            .secondary()
                            .icon(IconName::Pin)
                            .label("Pinned runs only")
                            .pressed(self.pinned)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pinned = !this.pinned;
                                cx.notify();
                            })),
                    )
                    .child(
                        IconButton::new("demo-like")
                            .ghost()
                            .icon(IconName::Heart)
                            .label("Favorite")
                            .pressed(self.liked)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.liked = !this.liked;
                                cx.notify();
                            })),
                    )
                    .child(
                        IconButton::new("demo-new")
                            .primary()
                            .icon(IconName::Plus)
                            .label("New run")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.runs += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        IconButton::new("demo-small")
                            .secondary()
                            .small()
                            .icon(IconName::Settings)
                            .label("Settings (sm)"),
                    )
                    .child(
                        IconButton::new("demo-disabled")
                            .secondary()
                            .icon(IconName::Trash)
                            .label("Disabled")
                            .disabled(true),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(riff_ui::theme::RiffTheme::global(cx).muted)
                    .child(format!(
                        "pinned: {} · liked: {} · new runs: {}",
                        self.pinned, self.liked, self.runs
                    )),
            )
    }
}
