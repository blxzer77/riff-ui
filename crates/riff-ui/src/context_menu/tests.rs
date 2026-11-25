use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext,
    TestSupportExt as _, Window, div, point, px,
};

use super::*;

struct Harness {
    picked: Rc<Cell<Option<&'static str>>>,
    disabled: bool,
    empty_dynamic: bool,
    focus_row: gpui_kit::FocusHandle,
    focus_outside: gpui_kit::FocusHandle,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let picked = self.picked.clone();
        let picked_sub = self.picked.clone();
        let empty = self.empty_dynamic;

        div()
            .id("root")
            .size_full()
            .p_10()
            .flex()
            .gap_6()
            .child(
                ContextMenu::new("ctx-1")
                    .disabled(self.disabled)
                    .title("File actions")
                    .items_for_point(move |pt, _, _| {
                        if empty {
                            return Vec::new();
                        }
                        let picked = picked.clone();
                        let picked_sub = picked_sub.clone();
                        let side_tag = if pt.x > px(140.) {
                            "right-zone"
                        } else {
                            "left-zone"
                        };
                        vec![
                            ContextMenuItem::new(format!("Open ({side_tag})"))
                                .icon(IconName::Eye)
                                .on_select(move |_, _| picked.set(Some(side_tag))),
                            ContextMenuItem::new("Share")
                                .icon(IconName::Folder)
                                .submenu(vec![
                                    ContextMenuItem::new("Copy link")
                                        .on_select(move |_, _| picked_sub.set(Some("copy-link"))),
                                ]),
                            ContextMenuItem::new("Delete").danger(true).divider(true),
                        ]
                    })
                    .child(
                        div()
                            .id("ctx-area")
                            .role(Role::Button)
                            .aria_label("Context zone")
                            .track_focus(&self.focus_row)
                            .tab_index(0)
                            .w(px(220.))
                            .h(px(80.))
                            .test_support()
                            .child("Right-click anywhere here"),
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
            picked: Rc::new(Cell::new(None)),
            disabled: false,
            empty_dynamic: false,
            focus_row: cx.focus_handle().tab_stop(true),
            focus_outside: cx.focus_handle().tab_stop(true),
        });
        clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    visual_cx.update(|window, _| window.activate_window());
    (holder.take().expect("harness"), visual_cx)
}

#[gpui_kit::test]
fn right_click_opens_at_pointer_builds_dynamic_items_and_escape_returns_focus(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    let picked = view.read_with(cx, |v, _| v.picked.clone());
    let focus_row = view.read_with(cx, |v, _| v.focus_row.clone());
    cx.run_until_parked();

    let area = cx.update(|window, _| window.find_by_label("Context zone").bounds());
    let right_pt = point(area.left() + px(160.), area.top() + px(30.));
    cx.simulate_mouse_down(right_pt, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(right_pt, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );
    let menu_bounds = cx.update(|window, _| window.find_all_by_role(Role::Menu)[0].bounds());
    assert!(
        menu_bounds.left() >= right_pt.x - px(2.),
        "menu should anchor at pointer x={:?}, got {:?}",
        right_pt.x,
        menu_bounds.left()
    );

    // ArrowDown picks the dynamic item built for right-zone.
    cx.update(|window, cx| window.press("down", cx));
    cx.update(|window, cx| window.press("enter", cx));
    cx.run_until_parked();
    assert_eq!(picked.get(), Some("right-zone"));
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        0
    );
    assert!(cx.update(|window, _| focus_row.is_focused(window)));
}

#[gpui_kit::test]
fn shift_f10_opens_menu_and_disabled_or_empty_stays_closed(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_row = view.read_with(cx, |v, _| v.focus_row.clone());
    cx.run_until_parked();

    cx.update(|window, cx| focus_row.focus(window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("shift-f10", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );

    // Escape closes and preserves focus on row.
    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        0
    );
    assert!(cx.update(|window, _| focus_row.is_focused(window)));

    // Empty dynamic items or disabled keeps menu closed on right-click.
    view.update(cx, |v, cx| {
        v.empty_dynamic = true;
        cx.notify();
    });
    cx.run_until_parked();
    let area = cx.update(|window, _| window.find_by_label("Context zone").bounds());
    cx.simulate_mouse_down(area.center(), MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(area.center(), MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        0
    );
}
