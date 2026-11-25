use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext,
    TestSupportExt as _, Window, div, point, px,
};

use super::*;

struct Harness {
    focus_trigger: gpui_kit::FocusHandle,
    focus_field: gpui_kit::FocusHandle,
    focus_save: gpui_kit::FocusHandle,
    focus_outside: gpui_kit::FocusHandle,
    alert_busy: bool,
    confirm_count: Rc<Cell<usize>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let confirms = self.confirm_count.clone();

        div()
            .id("root")
            .size_full()
            .p_10()
            .flex()
            .gap_4()
            .child(
                Dialog::new("dlg-1", "Edit agent")
                    .description("Changes apply next run.")
                    .trigger(
                        div()
                            .id("dlg-open")
                            .role(Role::Button)
                            .aria_label("Open edit")
                            .track_focus(&self.focus_trigger)
                            .tab_index(0)
                            .w(px(90.))
                            .h(px(32.))
                            .test_support()
                            .child("Edit"),
                    )
                    .child(
                        div()
                            .id("dlg-field")
                            .role(Role::TextInput)
                            .aria_label("Agent name")
                            .track_focus(&self.focus_field)
                            .tab_index(0)
                            .h(px(32.))
                            .test_support()
                            .child("Worker"),
                    )
                    .footer(
                        div()
                            .id("dlg-save")
                            .role(Role::Button)
                            .aria_label("Save")
                            .track_focus(&self.focus_save)
                            .tab_index(0)
                            .h(px(32.))
                            .test_support()
                            .child("Save"),
                    ),
            )
            .child(
                AlertDialog::new("alert-1", "Cancel subscription?")
                    .description("You keep Pro until March 14.")
                    .danger(true)
                    .busy(self.alert_busy)
                    .cancel_label("Keep plan")
                    .confirm_label("Cancel plan")
                    .on_confirm(move |_, _| confirms.set(confirms.get() + 1))
                    .trigger(
                        div()
                            .id("alert-open")
                            .role(Role::Button)
                            .aria_label("Open alert")
                            .w(px(100.))
                            .h(px(32.))
                            .test_support()
                            .child("Alert"),
                    ),
            )
            .child(
                div()
                    .id("outside")
                    .role(Role::Button)
                    .aria_label("Outside")
                    .track_focus(&self.focus_outside)
                    .tab_index(0)
                    .w(px(80.))
                    .h(px(32.))
                    .test_support()
                    .child("Outside"),
            )
    }
}

fn setup(cx: &mut TestAppContext) -> (gpui_kit::Entity<Harness>, &mut gpui_kit::VisualTestContext) {
    cx.update(crate::init);
    let holder: Rc<Cell<Option<gpui_kit::Entity<Harness>>>> = Rc::new(Cell::new(None));
    let clone = holder.clone();
    let (_, visual_cx) = cx.add_window_view(move |window, cx| {
        let harness = cx.new(|cx| Harness {
            focus_trigger: cx.focus_handle().tab_stop(true),
            focus_field: cx.focus_handle().tab_stop(true),
            focus_save: cx.focus_handle().tab_stop(true),
            focus_outside: cx.focus_handle().tab_stop(true),
            alert_busy: false,
            confirm_count: Rc::new(Cell::new(0)),
        });
        clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    visual_cx.update(|window, _| window.activate_window());
    (holder.take().expect("harness"), visual_cx)
}

#[gpui_kit::test]
fn dialog_traps_tab_closes_on_backdrop_and_escape_restores_focus(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_trigger = view.read_with(cx, |v, _| v.focus_trigger.clone());
    let focus_field = view.read_with(cx, |v, _| v.focus_field.clone());
    let focus_save = view.read_with(cx, |v, _| v.focus_save.clone());
    let focus_outside = view.read_with(cx, |v, _| v.focus_outside.clone());
    cx.run_until_parked();

    let open_b = cx.update(|window, _| window.find_by_label("Open edit").bounds());
    cx.update(|window, cx| focus_trigger.focus(window, cx));
    cx.simulate_click(open_b.center(), Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        1
    );
    assert!(cx.update(|window, _| focus_field.is_focused(window)));

    // Tab cycles inside dialog: body field -> footer save -> close button -> body field.
    cx.update(|window, cx| window.press("tab", cx));
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_save.is_focused(window)));

    cx.update(|window, cx| window.press("tab", cx));
    cx.update(|window, cx| window.press("tab", cx));
    cx.run_until_parked();
    assert!(!cx.update(|window, _| focus_outside.is_focused(window)));
    assert!(cx.update(|window, _| focus_field.is_focused(window)));

    // Escape closes and returns focus to opener button.
    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        0
    );
    assert!(cx.update(|window, _| focus_trigger.is_focused(window)));

    // Reopen and click scrim corner -> close_on_backdrop dismisses it.
    cx.simulate_click(open_b.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        1
    );
    cx.simulate_mouse_down(
        point(px(4.), px(4.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(4.), px(4.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        0
    );
}

#[gpui_kit::test]
fn alert_dialog_focuses_cancel_ignores_backdrop_and_busy_blocks_escape(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let confirms = view.read_with(cx, |v, _| v.confirm_count.clone());
    cx.run_until_parked();

    let open_b = cx.update(|window, _| window.find_by_label("Open alert").bounds());
    cx.simulate_click(open_b.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::AlertDialog).len()),
        1
    );

    // Backdrop press MUST NOT close AlertDialog.
    cx.simulate_mouse_down(
        point(px(4.), px(4.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(4.), px(4.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::AlertDialog).len()),
        1
    );

    // Busy AlertDialog refuses Escape dismissal.
    view.update(cx, |v, cx| {
        v.alert_busy = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::AlertDialog).len()),
        1
    );

    // Unbusy confirms and closes.
    view.update(cx, |v, cx| {
        v.alert_busy = false;
        cx.notify();
    });
    cx.run_until_parked();
    let confirm_b = cx.update(|window, _| window.find_by_label("Cancel plan").bounds());
    cx.simulate_click(confirm_b.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(confirms.get(), 1);
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::AlertDialog).len()),
        0
    );
}
