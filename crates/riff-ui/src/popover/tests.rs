use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement as _,
    Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext, TestSupportExt as _,
    Window, div, point, px, size,
};

use super::*;
use crate::overlay::{OverlayAlign, OverlaySide, PlaceOptions, place_side_with};

struct Harness {
    controlled: Option<bool>,
    focus_trigger: gpui_kit::FocusHandle,
    focus_outside: gpui_kit::FocusHandle,
    focus_inner: gpui_kit::FocusHandle,
    close_calls: Rc<Cell<usize>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close_calls = self.close_calls.clone();
        let entity = cx.entity().downgrade();
        div()
            .id("root")
            .size_full()
            .p_12()
            .flex()
            .gap_6()
            .child(
                Popover::new("pop-1")
                    .title("Workspace settings")
                    .description("Adjust member access.")
                    .open(self.controlled)
                    .on_close(move |_, cx| {
                        close_calls.set(close_calls.get() + 1);
                        _ = entity.update(cx, |this, cx| {
                            if this.controlled.is_some() {
                                this.controlled = Some(false);
                                cx.notify();
                            }
                        });
                    })
                    .trigger(
                        div()
                            .id("trig-btn")
                            .role(Role::Button)
                            .aria_label("Open popover")
                            .track_focus(&self.focus_trigger)
                            .tab_index(0)
                            .w(px(100.))
                            .h(px(32.))
                            .test_support()
                            .child("Open popover"),
                    )
                    .child(
                        div()
                            .id("inner-input")
                            .role(Role::Button)
                            .aria_label("Inside action")
                            .track_focus(&self.focus_inner)
                            .tab_index(0)
                            .h(px(28.))
                            .test_support()
                            .child("Apply"),
                    ),
            )
            .child(
                div()
                    .id("outside-btn")
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

fn setup(
    cx: &mut TestAppContext,
    controlled: Option<bool>,
) -> (gpui_kit::Entity<Harness>, &mut gpui_kit::VisualTestContext) {
    cx.update(crate::init);
    let holder: Rc<Cell<Option<gpui_kit::Entity<Harness>>>> = Rc::new(Cell::new(None));
    let clone = holder.clone();
    let (_, visual_cx) = cx.add_window_view(move |window, cx| {
        let harness = cx.new(|cx| Harness {
            controlled,
            focus_trigger: cx.focus_handle().tab_stop(true),
            focus_outside: cx.focus_handle().tab_stop(true),
            focus_inner: cx.focus_handle().tab_stop(true),
            close_calls: Rc::new(Cell::new(0)),
        });
        clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    visual_cx.update(|window, _| window.activate_window());
    (holder.take().expect("harness"), visual_cx)
}

#[test]
fn place_side_switches_align_when_overflowing_and_prefers_bottom_fallback() {
    let viewport = Bounds::new(point(px(0.), px(0.)), size(px(320.), px(240.)));
    let opts = PlaceOptions {
        flip_align: true,
        flip_align_beside: true,
        prefer_bottom_fallback: true,
    };

    // Near right edge with Start alignment: switches to End alignment!
    let near_right = Bounds::new(point(px(230.), px(40.)), size(px(80.), px(32.)));
    let placed = place_side_with(
        near_right,
        size(px(160.), px(80.)),
        viewport,
        OverlaySide::Bottom,
        OverlayAlign::Start,
        px(6.),
        px(8.),
        false,
        opts,
    );
    assert_eq!(placed.side, OverlaySide::Bottom);
    assert_eq!(placed.align, OverlayAlign::End);
    assert_eq!(placed.origin.x, px(150.));

    // Left/Right cramped horizontally -> prefers Bottom when bottom >= top.
    let cramped_x = Bounds::new(point(px(40.), px(80.)), size(px(240.), px(32.)));
    let placed = place_side_with(
        cramped_x,
        size(px(160.), px(80.)),
        viewport,
        OverlaySide::Right,
        OverlayAlign::Start,
        px(6.),
        px(8.),
        false,
        opts,
    );
    assert_eq!(placed.side, OverlaySide::Bottom);
}

#[gpui_kit::test]
fn opens_moves_focus_inside_and_escape_restores_focus_to_trigger(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, None);
    let focus_trigger = view.read_with(cx, |v, _| v.focus_trigger.clone());
    let focus_inner = view.read_with(cx, |v, _| v.focus_inner.clone());
    cx.run_until_parked();

    let trig_bounds = cx.update(|window, _| window.find_by_label("Open popover").bounds());
    cx.update(|window, cx| focus_trigger.focus(window, cx));
    cx.simulate_click(trig_bounds.center(), Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        1
    );
    assert!(
        cx.update(|window, _| focus_inner.is_focused(window)),
        "focus_on_open must move focus to first inner control"
    );

    // Escape closes popover and returns focus back to trigger.
    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        0
    );
    assert!(
        cx.update(|window, _| focus_trigger.is_focused(window)),
        "closing with Escape must restore focus to trigger"
    );
}

#[gpui_kit::test]
fn outside_click_and_tab_away_close_controlled_popover(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, Some(true));
    let close_calls = view.read_with(cx, |v, _| v.close_calls.clone());
    let focus_inner = view.read_with(cx, |v, _| v.focus_inner.clone());
    let focus_outside = view.read_with(cx, |v, _| v.focus_outside.clone());
    cx.run_until_parked();

    // Open from first render -> leaves focus alone (inside is NOT auto-focused).
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        1
    );
    assert!(cx.update(|window, _| !focus_inner.is_focused(window)));

    // Outside click invokes on_close -> controlled state closes popover.
    let out_bounds = cx.update(|window, _| window.find_by_label("Outside").bounds());
    cx.simulate_mouse_down(out_bounds.center(), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(out_bounds.center(), MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(close_calls.get(), 1);
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        0
    );

    // Re-open, move focus inside then Tab out to outside -> closes!
    view.update(cx, |v, cx| {
        v.controlled = Some(true);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_inner.is_focused(window)));

    cx.update(|window, cx| focus_outside.focus(window, cx));
    cx.run_until_parked();
    assert_eq!(close_calls.get(), 2);
    assert_eq!(
        cx.update(|window, _| window.find_all_by_role(Role::Dialog).len()),
        0
    );
}

#[gpui_kit::test]
fn title_only_popover_sizes_to_text_width_without_collapsing(cx: &mut TestAppContext) {
    cx.update(crate::init);
    struct TitleOnly;
    impl Render for TitleOnly {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().p_12().child(
                Popover::new("title-only")
                    .title("Unsaved changes")
                    .open(Some(true))
                    .trigger(div().w(px(60.)).h(px(32.)).child("Open")),
            )
        }
    }
    let (_, cx) = cx.add_window_view(|window, cx| {
        let v = cx.new(|_| TitleOnly);
        Root::new(v, window, cx)
    });
    cx.run_until_parked();
    let bounds = cx.update(|window, _| window.find_all_by_role(Role::Dialog)[0].bounds());
    assert!(
        bounds.size.width >= px(100.) && bounds.size.height < px(80.),
        "title-only popover must fit horizontal single line, got {:?}",
        bounds.size
    );
}
