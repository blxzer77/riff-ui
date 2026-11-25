use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement as _,
    Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext, TestSupportExt as _,
    Window, div, point, px, size,
};

use super::corridor::{hull_from_exit, in_hull};
use super::*;
use crate::overlay::{OverlayAlign, OverlaySide, PlaceOptions, place_side_with};

#[test]
fn corridor_hull_keeps_diagonal_path_open_and_rejects_away() {
    let trigger = Bounds::new(point(px(110.), px(42.)), size(px(80.), px(28.)));
    let card = Bounds::new(point(px(100.), px(100.)), size(px(200.), px(100.)));
    let exit = point(px(150.), px(70.));
    let hull = hull_from_exit(exit, trigger, card);

    assert!(in_hull(&hull, exit));
    assert!(in_hull(&hull, point(px(200.), px(85.))));
    assert!(in_hull(&hull, point(px(97.), px(150.))));
    assert!(!in_hull(&hull, point(px(150.), px(50.))));
    assert!(!in_hull(&hull, point(px(60.), px(70.))));
}

#[test]
fn left_right_clamps_without_flipping_align_while_vertical_flips() {
    let viewport = Bounds::new(point(px(0.), px(0.)), size(px(400.), px(200.)));
    let card_size = size(px(160.), px(90.));

    let trigger_right = Bounds::new(point(px(280.), px(20.)), size(px(90.), px(32.)));
    let placed_v = place_side_with(
        trigger_right,
        card_size,
        viewport,
        OverlaySide::Bottom,
        OverlayAlign::Start,
        px(8.),
        px(8.),
        false,
        PlaceOptions {
            flip_align: true,
            flip_align_beside: false,
            prefer_bottom_fallback: true,
        },
    );
    assert_eq!(placed_v.align, OverlayAlign::End);

    let trigger_low = Bounds::new(point(px(20.), px(150.)), size(px(80.), px(32.)));
    let placed_h = place_side_with(
        trigger_low,
        card_size,
        viewport,
        OverlaySide::Right,
        OverlayAlign::Start,
        px(8.),
        px(8.),
        false,
        PlaceOptions {
            flip_align: true,
            flip_align_beside: false,
            prefer_bottom_fallback: true,
        },
    );
    assert_eq!(placed_h.align, OverlayAlign::Start);
    assert_eq!(placed_h.origin.y, px(200. - 8. - 90.));
}

struct Harness {
    disabled: bool,
    open_delay: Duration,
    close_delay: Duration,
    last_open_a: Rc<Cell<Option<bool>>>,
    focus_before: gpui_kit::FocusHandle,
    focus_a: gpui_kit::FocusHandle,
    focus_b: gpui_kit::FocusHandle,
    focus_inner: gpui_kit::FocusHandle,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let last_open_a = self.last_open_a.clone();
        div()
            .id("root")
            .size_full()
            .p_12()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .id("before-btn")
                    .role(Role::Button)
                    .aria_label("Before")
                    .track_focus(&self.focus_before)
                    .tab_index(0)
                    .w(px(80.))
                    .h(px(28.))
                    .test_support()
                    .child("Before"),
            )
            .child(
                div()
                    .id("row")
                    .flex()
                    .gap_8()
                    .child(
                        HoverCard::new("card-a")
                            .open_delay(self.open_delay)
                            .close_delay(self.close_delay)
                            .disabled(self.disabled)
                            .on_open_change(move |open, _, _| last_open_a.set(Some(open)))
                            .trigger(
                                div()
                                    .id("trig-a")
                                    .role(Role::Link)
                                    .aria_label("Trigger A")
                                    .track_focus(&self.focus_a)
                                    .tab_index(0)
                                    .w(px(90.))
                                    .h(px(28.))
                                    .test_support()
                                    .child("Trigger A"),
                            )
                            .child(
                                div()
                                    .id("card-a-body")
                                    .aria_label("Card A")
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .test_support()
                                    .child("Preview A")
                                    .child(
                                        div()
                                            .id("inner-link")
                                            .role(Role::Link)
                                            .aria_label("Inspect A")
                                            .track_focus(&self.focus_inner)
                                            .tab_index(0)
                                            .h(px(24.))
                                            .test_support()
                                            .child("Inspect A"),
                                    ),
                            ),
                    )
                    .child(
                        HoverCard::new("card-b")
                            .open_delay(Duration::ZERO)
                            .close_delay(self.close_delay)
                            .trigger(
                                div()
                                    .id("trig-b")
                                    .role(Role::Link)
                                    .aria_label("Trigger B")
                                    .track_focus(&self.focus_b)
                                    .tab_index(0)
                                    .w(px(90.))
                                    .h(px(28.))
                                    .test_support()
                                    .child("Trigger B"),
                            )
                            .child(
                                div()
                                    .id("card-b-body")
                                    .aria_label("Card B")
                                    .test_support()
                                    .child("Preview B"),
                            ),
                    ),
            )
    }
}

fn setup(cx: &mut TestAppContext) -> (gpui_kit::Entity<Harness>, &mut gpui_kit::VisualTestContext) {
    cx.update(crate::init);
    let holder: Rc<Cell<Option<gpui_kit::Entity<Harness>>>> = Rc::new(Cell::new(None));
    let clone = holder.clone();
    let (_, visual_cx) = cx.add_window_view(move |window, cx| {
        let harness = cx.new(|cx| Harness {
            disabled: false,
            open_delay: Duration::from_millis(60),
            close_delay: Duration::from_millis(40),
            last_open_a: Rc::new(Cell::new(None)),
            focus_before: cx.focus_handle().tab_stop(true),
            focus_a: cx.focus_handle().tab_stop(true),
            focus_b: cx.focus_handle().tab_stop(true),
            focus_inner: cx.focus_handle().tab_stop(true),
        });
        clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    visual_cx.update(|window, _| window.activate_window());
    (holder.take().expect("harness"), visual_cx)
}

fn is_card_open(window: &Window, label: &str) -> bool {
    !window.find_all_by_label(label).is_empty()
}

#[gpui_kit::test]
fn hover_opens_after_delay_corridor_holds_and_single_active_closes_previous(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    let last_open_a = view.read_with(cx, |v, _| v.last_open_a.clone());
    cx.run_until_parked();

    let trig_a = cx.update(|window, _| window.find_by_label("Trigger A").bounds());
    cx.simulate_mouse_move(trig_a.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));

    cx.executor().advance_clock(Duration::from_millis(70));
    cx.run_until_parked();
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));
    let card_a = cx.update(|window, _| window.find_by_label("Card A").bounds());

    // Leave trigger diagonally toward card inside corridor -> stays open beyond close_delay.
    cx.simulate_mouse_move(
        point(trig_a.center().x + px(4.), trig_a.bottom() + px(4.)),
        None,
        Modifiers::default(),
    );
    cx.executor().advance_clock(Duration::from_millis(60));
    cx.run_until_parked();
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));

    // Move onto card -> still open.
    cx.simulate_mouse_move(card_a.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(60));
    cx.run_until_parked();
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));

    // Open card B -> closes card A (only one card open at a time).
    let trig_b = cx.update(|window, _| window.find_by_label("Trigger B").bounds());
    cx.simulate_mouse_move(trig_b.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(last_open_a.get(), Some(false));
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));
    assert!(cx.update(|window, _| is_card_open(window, "Card B")));

    // Leaving trigger B cleanly closes it after close_delay.
    cx.simulate_mouse_move(point(px(2.), px(2.)), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(60));
    cx.run_until_parked();
    assert!(!cx.update(|window, _| is_card_open(window, "Card B")));
}

#[gpui_kit::test]
fn keyboard_focus_opens_tab_enters_card_and_escape_restores_trigger(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_before = view.read_with(cx, |v, _| v.focus_before.clone());
    let focus_a = view.read_with(cx, |v, _| v.focus_a.clone());
    let focus_inner = view.read_with(cx, |v, _| v.focus_inner.clone());
    cx.run_until_parked();

    cx.update(|window, cx| focus_before.focus(window, cx));
    cx.run_until_parked();

    cx.update(|window, cx| window.press("tab", cx));
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_a.is_focused(window)));
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));

    cx.executor().advance_clock(Duration::from_millis(70));
    cx.run_until_parked();
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));

    // Tab moves into inner link -> stays open.
    cx.update(|window, cx| window.press("tab", cx));
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_inner.is_focused(window)));
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));

    // Escape closes and returns focus to Trigger A.
    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));
    assert!(cx.update(|window, _| focus_a.is_focused(window)));
}

#[gpui_kit::test]
fn trigger_press_closes_and_disabled_never_opens(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    cx.run_until_parked();

    let trig_a = cx.update(|window, _| window.find_by_label("Trigger A").bounds());
    cx.simulate_mouse_move(trig_a.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(70));
    cx.run_until_parked();
    assert!(cx.update(|window, _| is_card_open(window, "Card A")));

    // Clicking trigger closes card immediately (following link/button).
    cx.simulate_mouse_down(trig_a.center(), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(trig_a.center(), MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));

    view.update(cx, |this, cx| {
        this.disabled = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_mouse_move(point(px(2.), px(2.)), None, Modifiers::default());
    cx.simulate_mouse_move(trig_a.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(80));
    cx.run_until_parked();
    assert!(!cx.update(|window, _| is_card_open(window, "Card A")));
}
