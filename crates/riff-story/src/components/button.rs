//! Button: the story's cells, and a demo to click.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use riff_ui::button::{Button, ButtonIcon, ButtonSize, ButtonState, ButtonVariant};

use crate::registry::{DemoFn, Pin};
use crate::story::{PinnedState, Props};

const PROPS: &[&str] = &[
    "variant",
    "size",
    "label",
    "icon",
    "iconRight",
    "ariaLabel",
    "disabled",
    "loading",
];

/// The live button for one cell of `stories/button.json`.
pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }
    let variant = match props.str("variant").unwrap_or("primary") {
        "primary" => ButtonVariant::Primary,
        "secondary" => ButtonVariant::Secondary,
        "ghost" => ButtonVariant::Ghost,
        "destructive" => ButtonVariant::Destructive,
        other => return Err(format!("unknown variant {other:?}")),
    };
    let size = match props.str("size").unwrap_or("md") {
        "sm" => ButtonSize::Sm,
        "md" => ButtonSize::Md,
        "lg" => ButtonSize::Lg,
        other => return Err(format!("unknown size {other:?}")),
    };
    // Story icons are Lucide names; a name with no file leaves the slot
    // empty, as riff-uiux does for a broken image.
    let icon = |name: &str| ButtonIcon::path(format!("icons/{name}.svg"));

    let mut button = Button::new(SharedString::from(format!("story-{id}")))
        .variant(variant)
        .size(size)
        .disabled(props.bool("disabled"))
        .loading(props.bool("loading"));
    if let Some(label) = props.str("label") {
        button = button.label(label.to_owned());
    }
    if let Some(name) = props.str("icon") {
        button = button.icon(icon(name));
    }
    if let Some(name) = props.str("iconRight") {
        button = button.icon_right(icon(name));
    }
    if let Some(label) = props.str("ariaLabel") {
        button = button.accessibility_label(label.to_owned());
    }
    // A pressed button is under the pointer too, as riff-uiux's `:active`
    // cells are pinned with `:hover` (tools/reference/gallery.jsx).
    button = button.force_state(ButtonState {
        hovered: matches!(pin.state, Some(PinnedState::Hover | PinnedState::Active)),
        pressed: pin.state == Some(PinnedState::Active),
        focus_visible: pin.state == Some(PinnedState::Focus),
    });
    if let Some(at) = pin.freeze {
        button = button.freeze_motion_at(at);
    }
    Ok(button.into_any_element())
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    clicks: usize,
    saving: bool,
    copied: bool,
    expanded: bool,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_3()
            .child(
                Button::new("demo-count")
                    .label(format!("Clicked {}", self.clicks))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("demo-save")
                    .secondary()
                    .icon(IconName::Check)
                    .label(if self.saving {
                        "Saving…"
                    } else {
                        "Request approval"
                    })
                    .loading(self.saving)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saving = true;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("demo-stop")
                    .ghost()
                    .label("Stop loading")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.saving = false;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("demo-copy")
                    .secondary()
                    .icon(if self.copied {
                        IconName::Check
                    } else {
                        IconName::Copy
                    })
                    .label(if self.copied { "Copied" } else { "Copy link" })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.copied = !this.copied;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("demo-expand")
                    .label(if self.expanded {
                        "Show fewer options"
                    } else {
                        "More"
                    })
                    .icon_right(if self.expanded {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.expanded = !this.expanded;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("demo-delete")
                    .destructive()
                    .icon(IconName::Trash)
                    .label("Delete"),
            )
            .child(
                Button::new("demo-add")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .accessibility_label("Add"),
            )
    }
}
