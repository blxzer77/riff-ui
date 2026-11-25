//! Dialog & AlertDialog: story cells and an interactive demo covering
//! backdrop dismissal, Tab focus trapping, nested overlay Escape precedence,
//! contained modals, and AlertDialog busy confirmation.

use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::dialog::{AlertDialog, Dialog, DialogSize};
use riff_ui::theme::RiffTheme;

use crate::registry::{DemoFn, Pin};
use crate::story::Props;

const PROPS: &[&str] = &[
    "alert",
    "size",
    "width",
    "title",
    "description",
    "body",
    "closeButton",
    "footerCancel",
    "footerConfirm",
    "cancelLabel",
    "confirmLabel",
    "danger",
    "stageWidth",
    "stageHeight",
];

pub fn cell(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String> {
    if let Some(unknown) = props.keys().find(|key| !PROPS.contains(key)) {
        return Err(format!("unknown prop {unknown:?}"));
    }

    let alert = props.bool("alert");
    let stage_w = props.f32("stageWidth").unwrap_or(450.);
    let stage_h = props.f32("stageHeight").unwrap_or(230.);
    let title = props.str("title").unwrap_or("Dialog").to_owned();

    let stage = div()
        .w(px(stage_w))
        .h(px(stage_h))
        .rounded(RiffTheme::global_default_radius())
        .overflow_hidden();

    let modal = if alert {
        let size = match props.str("size").unwrap_or("sm") {
            "sm" => DialogSize::Sm,
            "md" => DialogSize::Md,
            "lg" => DialogSize::Lg,
            other => return Err(format!("unknown dialog size {other:?}")),
        };
        let mut ad = AlertDialog::new(SharedString::from(format!("story-dlg-{id}")), title)
            .size(size)
            .contained(true)
            .open(Some(true))
            .inline_preview(true)
            .danger(props.bool("danger"));

        if let Some(d) = props.str("description") {
            ad = ad.description(d.to_owned());
        }
        if let Some(b) = props.str("body") {
            ad = ad.child(div().child(b.to_owned()));
        }
        if let Some(w) = props.f32("width") {
            ad = ad.width(px(w));
        }
        if let Some(cl) = props.str("cancelLabel") {
            ad = ad.cancel_label(cl.to_owned());
        }
        if let Some(cl) = props.str("confirmLabel") {
            ad = ad.confirm_label(cl.to_owned());
        }
        if let Some(at) = pin.freeze {
            ad = ad.freeze_motion_at(at);
        }
        ad.into_any_element()
    } else {
        let size = match props.str("size").unwrap_or("md") {
            "sm" => DialogSize::Sm,
            "md" => DialogSize::Md,
            "lg" => DialogSize::Lg,
            other => return Err(format!("unknown dialog size {other:?}")),
        };
        let close_btn = props.bool_or("closeButton", true);
        let mut dlg = Dialog::new(SharedString::from(format!("story-dlg-{id}")), title)
            .size(size)
            .close_button(close_btn)
            .contained(true)
            .open(Some(true))
            .inline_preview(true);

        if let Some(d) = props.str("description") {
            dlg = dlg.description(d.to_owned());
        }
        if let Some(b) = props.str("body") {
            dlg = dlg.child(div().child(b.to_owned()));
        }
        if let Some(w) = props.f32("width") {
            dlg = dlg.width(px(w));
        }
        if let Some(c) = props.str("footerCancel") {
            dlg = dlg.footer(
                Button::new(SharedString::from(format!("story-dlg-fc-{id}")))
                    .secondary()
                    .small()
                    .label(c.to_owned()),
            );
        }
        if let Some(c) = props.str("footerConfirm") {
            dlg = dlg.footer(
                Button::new(SharedString::from(format!("story-dlg-ok-{id}")))
                    .small()
                    .label(c.to_owned()),
            );
        }
        if let Some(at) = pin.freeze {
            dlg = dlg.freeze_motion_at(at);
        }
        dlg.into_any_element()
    };

    Ok(stage.child(modal).into_any_element())
}

trait RadiusExt {
    fn global_default_radius() -> Pixels;
}

impl RadiusExt for RiffTheme {
    fn global_default_radius() -> Pixels {
        px(7.)
    }
}

pub const DEMO: Option<DemoFn> = Some(|_, cx| cx.new(|_| Demo::default()).into());

#[derive(Default)]
struct Demo {
    edit_open: bool,
    saved_count: usize,
    deleted_count: usize,
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let weak_edit = cx.entity().downgrade();
        let weak_alert = cx.entity().downgrade();

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted)
                    .child(format!(
                        "Saved {}× · Confirmed danger {}× · Tab stays trapped inside open dialogs",
                        self.saved_count, self.deleted_count
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        Dialog::new("demo-edit-dlg", "Edit agent")
                            .description("Changes apply from its next run.")
                            .open(Some(self.edit_open))
                            .on_open_change(move |open, _, cx| {
                                _ = weak_edit.update(cx, |this, cx| {
                                    this.edit_open = open;
                                    cx.notify();
                                });
                            })
                            .trigger(
                                Button::new("demo-dlg-open")
                                    .secondary()
                                    .label("Edit agent")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.edit_open = true;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child("Modal body scrolls smoothly if tall while the header and tray footer stay pinned.")
                                    .child("Press Tab to cycle solely between controls inside the dialog."),
                            )
                            .footer(
                                Button::new("demo-dlg-cancel")
                                    .secondary()
                                    .small()
                                    .label("Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.edit_open = false;
                                        cx.notify();
                                    })),
                            )
                            .footer(
                                Button::new("demo-dlg-save")
                                    .small()
                                    .label("Save changes")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.saved_count += 1;
                                        this.edit_open = false;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        AlertDialog::new("demo-alert-dlg", "Cancel subscription?")
                            .description("You keep Pro until March 14, then the workspace moves to Free.")
                            .danger(true)
                            .cancel_label("Keep plan")
                            .confirm_label("Cancel subscription")
                            .on_confirm(move |_, cx| {
                                _ = weak_alert.update(cx, |this, cx| {
                                    this.deleted_count += 1;
                                    cx.notify();
                                });
                            })
                            .trigger(
                                Button::new("demo-alert-open")
                                    .destructive()
                                    .label("Cancel subscription"),
                            ),
                    ),
            )
    }
}
