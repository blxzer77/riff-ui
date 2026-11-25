use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Bounds, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext,
    TestSupportExt as _, Window, div, point, px, size,
};

use super::*;
use crate::overlay::{OverlayAlign, OverlaySide, PlaceOptions, place_side_with};

#[test]
fn dropdown_place_flips_side_and_align_near_viewport_edges() {
    let viewport = Bounds::new(point(px(0.), px(0.)), size(px(320.), px(220.)));
    let menu = size(px(190.), px(120.));

    // Near bottom and right edge -> flips Bottom -> Top and Start -> End.
    let trig = Bounds::new(point(px(230.), px(170.)), size(px(80.), px(32.)));
    let placed = place_side_with(
        trig,
        menu,
        viewport,
        OverlaySide::Bottom,
        OverlayAlign::Start,
        px(6.),
        px(8.),
        false,
        PlaceOptions {
            flip_align: true,
            flip_align_beside: false,
            prefer_bottom_fallback: true,
        },
    );
    assert_eq!(placed.side, OverlaySide::Top);
    assert_eq!(placed.align, OverlayAlign::End);
    assert_eq!(placed.origin.x, px(310. - 190.));
}

struct Harness {
    picked: Rc<Cell<Option<&'static str>>>,
    grid_checked: bool,
    open_changes: Rc<Cell<usize>>,
    focus_trigger: gpui_kit::FocusHandle,
    focus_outside: gpui_kit::FocusHandle,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let picked_a = self.picked.clone();
        let picked_b = self.picked.clone();
        let picked_sub = self.picked.clone();
        let changes = self.open_changes.clone();
        let weak = cx.entity().downgrade();
        let grid = self.grid_checked;

        div()
            .id("root")
            .size_full()
            .p_10()
            .flex()
            .gap_6()
            .child(
                DropdownMenu::new("dm-1")
                    .title("Workspace actions")
                    .on_open_change(move |_, _, _| changes.set(changes.get() + 1))
                    .items([
                        DropdownMenuItem::new("Rerun")
                            .icon(IconName::Repeat)
                            .shortcut("⌘R")
                            .on_select(move |_, _| picked_a.set(Some("rerun"))),
                        DropdownMenuItem::new("Locked").disabled(true),
                        DropdownMenuItem::new("Move to")
                            .icon(IconName::Folder)
                            .submenu(vec![
                                DropdownMenuItem::new("Inbox")
                                    .on_select(move |_, _| picked_sub.set(Some("inbox"))),
                                DropdownMenuItem::new("Archive"),
                            ]),
                        DropdownMenuItem::new("Grid view")
                            .checkbox()
                            .checked(grid)
                            .keep_open(true)
                            .on_select(move |_, cx| {
                                _ = weak.update(cx, |this, cx| {
                                    this.grid_checked = !this.grid_checked;
                                    cx.notify();
                                });
                            }),
                        DropdownMenuItem::new("Delete run")
                            .danger(true)
                            .divider(true)
                            .on_select(move |_, _| picked_b.set(Some("delete"))),
                    ])
                    .trigger(
                        div()
                            .id("dm-trigger")
                            .role(Role::Button)
                            .aria_label("Open actions")
                            .track_focus(&self.focus_trigger)
                            .tab_index(0)
                            .w(px(110.))
                            .h(px(32.))
                            .test_support()
                            .child("Actions"),
                    ),
            )
            .child(
                div()
                    .id("dm-outside")
                    .role(Role::Button)
                    .aria_label("Outside target")
                    .track_focus(&self.focus_outside)
                    .tab_index(0)
                    .w(px(90.))
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
            grid_checked: false,
            open_changes: Rc::new(Cell::new(0)),
            focus_trigger: cx.focus_handle().tab_stop(true),
            focus_outside: cx.focus_handle().tab_stop(true),
        });
        clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    visual_cx.update(|window, _| window.activate_window());
    (holder.take().expect("harness"), visual_cx)
}

#[gpui_kit::test]
fn click_opens_keyboard_skips_disabled_submenu_arrows_and_escape_restores(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_trigger = view.read_with(cx, |v, _| v.focus_trigger.clone());
    let picked = view.read_with(cx, |v, _| v.picked.clone());
    cx.run_until_parked();

    let trig_bounds = cx.update(|window, _| window.find_by_label("Open actions").bounds());
    cx.simulate_click(trig_bounds.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );

    // ArrowDown starts at Rerun, next skips Locked -> lands on Move to -> Right opens submenu!
    cx.update(|window, cx| window.press("down", cx));
    cx.update(|window, cx| window.press("down", cx));
    cx.update(|window, cx| window.press("right", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        2
    );

    // Left closes ONLY submenu; second Escape closes root and returns focus to trigger.
    cx.update(|window, cx| window.press("left", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );

    cx.update(|window, cx| window.press("right", cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("enter", cx));
    cx.run_until_parked();
    assert_eq!(picked.get(), Some("inbox"));
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        0
    );
    assert!(cx.update(|window, _| focus_trigger.is_focused(window)));
}

#[gpui_kit::test]
fn typeahead_keep_open_checkbox_and_arrow_up_from_trigger(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_trigger = view.read_with(cx, |v, _| v.focus_trigger.clone());
    let picked = view.read_with(cx, |v, _| v.picked.clone());
    cx.run_until_parked();

    // ArrowUp on focused trigger opens directly at last item ("Delete run").
    cx.update(|window, cx| focus_trigger.focus(window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("up", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );

    // Typeahead 'g' jumps to "Grid view" (checkbox with keep_open = true).
    cx.update(|window, cx| window.press("g", cx));
    cx.update(|window, cx| window.press("enter", cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.grid_checked));
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        1
    );

    // Clicking outside closes menu without returning focus to trigger.
    let out = cx.update(|window, _| window.find_by_label("Outside target").bounds());
    cx.simulate_mouse_down(out.center(), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(out.center(), MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Menu).len()),
        0
    );

    // Re-open with ArrowUp and press Enter -> activates last item ("Delete run").
    cx.update(|window, cx| focus_trigger.focus(window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("up", cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("enter", cx));
    cx.run_until_parked();
    assert_eq!(picked.get(), Some("delete"));
}
