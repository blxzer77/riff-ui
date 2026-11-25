use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::Root;
use gpui_kit::test::{TestQueryExt as _, TestWindowExt as _};
use gpui_kit::{
    AppContext as _, Bounds, Context, IntoElement, KeyDownEvent, Modifiers, MouseButton,
    ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext,
    TestSupportExt as _, Window, div, point, prelude::FluentBuilder as _, px, size,
};

use super::*;
use crate::overlay::{OverlayAlign, OverlaySide, place_side};

struct Harness {
    focus_prev: gpui_kit::FocusHandle,
    focus_btn: gpui_kit::FocusHandle,
    open_log: Rc<Cell<usize>>,
    last_open: Rc<Cell<bool>>,
    parent_escaped: Rc<Cell<bool>>,
    controlled_open: Option<bool>,
    disabled: bool,
    empty_content: bool,
    show_neighbor: bool,
    delay: Duration,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let log_count = self.open_log.clone();
        let last_open = self.last_open.clone();
        let parent_escaped = self.parent_escaped.clone();

        div()
            .id("root")
            .size_full()
            .p_12()
            .flex()
            .gap_6()
            .on_key_down(move |e: &KeyDownEvent, _, _| {
                if e.keystroke.key == "escape" && e.keystroke.modifiers == Modifiers::default() {
                    parent_escaped.set(true);
                }
            })
            .child(
                div()
                    .id("prev")
                    .track_focus(&self.focus_prev)
                    .tab_index(0)
                    .size(px(40.))
                    .child("Prev"),
            )
            .child(
                Tooltip::new("tip-1")
                    .content(if self.empty_content { "" } else { "Save file" })
                    .shortcut("⌘S")
                    .delay(self.delay)
                    .disabled(self.disabled)
                    .open(self.controlled_open)
                    .on_open_change(move |open, _, _| {
                        log_count.set(log_count.get() + 1);
                        last_open.set(open);
                    })
                    .child(
                        div()
                            .id("trigger-1")
                            .role(Role::Button)
                            .aria_label("Save")
                            .track_focus(&self.focus_btn)
                            .tab_index(0)
                            .w(px(64.))
                            .h(px(32.))
                            .test_support()
                            .child("Save"),
                    ),
            )
            .when(self.show_neighbor, |d| {
                d.child(
                    Tooltip::new("tip-2")
                        .content("Copy link")
                        .delay(self.delay)
                        .child(
                            div()
                                .id("trigger-2")
                                .role(Role::Button)
                                .aria_label("Copy")
                                .w(px(64.))
                                .h(px(32.))
                                .test_support()
                                .child("Copy"),
                        ),
                )
            })
    }
}

fn setup(cx: &mut TestAppContext) -> (gpui_kit::Entity<Harness>, &mut gpui_kit::VisualTestContext) {
    cx.update(crate::init);
    let view_holder: Rc<Cell<Option<gpui_kit::Entity<Harness>>>> = Rc::new(Cell::new(None));
    let holder_clone = view_holder.clone();
    let (_, visual_cx) = cx.add_window_view(move |window, cx| {
        let harness = cx.new(|cx| Harness {
            focus_prev: cx.focus_handle().tab_stop(true),
            focus_btn: cx.focus_handle().tab_stop(true),
            open_log: Rc::new(Cell::new(0)),
            last_open: Rc::new(Cell::new(false)),
            parent_escaped: Rc::new(Cell::new(false)),
            controlled_open: None,
            disabled: false,
            empty_content: false,
            show_neighbor: true,
            delay: Duration::from_millis(500),
        });
        holder_clone.set(Some(harness.clone()));
        Root::new(harness, window, cx)
    });
    (view_holder.take().expect("harness"), visual_cx)
}

fn tip_bounds(cx: &mut gpui_kit::VisualTestContext, text: &str) -> Option<Bounds<Pixels>> {
    cx.update(|window, _| {
        window
            .find_all_by_role(Role::Tooltip)
            .into_iter()
            .find(|el| el.label().is_some_and(|l| l.contains(text)))
            .map(|el| el.bounds())
    })
}

fn label_bounds(cx: &mut gpui_kit::VisualTestContext, label: &str) -> Bounds<Pixels> {
    cx.update(|window, _| window.find_by_label(label).bounds())
}

#[test]
fn place_side_flips_falls_back_clamps_and_handles_rtl() {
    let viewport = Bounds::new(point(px(0.), px(0.)), size(px(320.), px(200.)));

    let near_top = Bounds::new(point(px(120.), px(10.)), size(px(60.), px(24.)));
    let placed = place_side(
        near_top,
        size(px(80.), px(28.)),
        viewport,
        OverlaySide::Top,
        OverlayAlign::Center,
        px(6.),
        px(8.),
        false,
    );
    assert_eq!(placed.side, OverlaySide::Bottom);
    assert_eq!(placed.origin.y, px(40.));

    let cramped_x = Bounds::new(point(px(16.), px(100.)), size(px(288.), px(24.)));
    let placed = place_side(
        cramped_x,
        size(px(80.), px(28.)),
        viewport,
        OverlaySide::Left,
        OverlayAlign::Center,
        px(6.),
        px(8.),
        false,
    );
    assert_eq!(placed.side, OverlaySide::Top);

    let at_left_edge = Bounds::new(point(px(2.), px(100.)), size(px(20.), px(24.)));
    let placed = place_side(
        at_left_edge,
        size(px(100.), px(28.)),
        viewport,
        OverlaySide::Top,
        OverlayAlign::Center,
        px(6.),
        px(8.),
        false,
    );
    assert_eq!(placed.origin.x, px(8.));

    let mid = Bounds::new(point(px(120.), px(100.)), size(px(60.), px(24.)));
    let placed = place_side(
        mid,
        size(px(90.), px(28.)),
        viewport,
        OverlaySide::Top,
        OverlayAlign::Start,
        px(6.),
        px(8.),
        true,
    );
    assert_eq!(placed.origin.x, px(90.));
}

#[gpui_kit::test]
fn hover_waits_delay_and_warm_neighbor_opens_immediately(cx: &mut TestAppContext) {
    let (_view, cx) = setup(cx);
    cx.run_until_parked();

    let b1 = label_bounds(cx, "Save");
    let b2 = label_bounds(cx, "Copy");

    cx.simulate_mouse_move(b1.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert!(
        tip_bounds(cx, "Save file").is_none(),
        "should wait delay before opening"
    );

    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert!(
        tip_bounds(cx, "Save file").is_some(),
        "should open after 500ms delay"
    );

    cx.simulate_mouse_move(b2.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert!(
        tip_bounds(cx, "Copy link").is_some(),
        "neighbor should open immediately while warm"
    );
    assert!(
        tip_bounds(cx, "Save file").is_none(),
        "previous tooltip should close"
    );
}

#[gpui_kit::test]
fn leave_has_grace_period_and_hovering_label_keeps_it_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update(cx, |v, cx| {
        v.delay = Duration::ZERO;
        cx.notify();
    });
    cx.run_until_parked();

    let b1 = label_bounds(cx, "Save");
    cx.simulate_mouse_move(b1.center(), None, Modifiers::default());
    cx.run_until_parked();
    let tip = tip_bounds(cx, "Save file").expect("tooltip should open");

    let gap_pt = point(b1.center().x, (b1.top() + tip.bottom()) / 2.);
    cx.simulate_mouse_move(gap_pt, None, Modifiers::default());
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_some());

    cx.simulate_mouse_move(tip.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    assert!(
        tip_bounds(cx, "Save file").is_some(),
        "pointer resting on tooltip keeps it open"
    );

    cx.simulate_mouse_move(point(px(2.), px(2.)), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(120));
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_none());
}

#[gpui_kit::test]
fn escape_and_click_dismiss_and_block_until_leave(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let parent_escaped = view.read_with(cx, |v, _| v.parent_escaped.clone());
    view.update(cx, |v, cx| {
        v.delay = Duration::ZERO;
        cx.notify();
    });
    cx.run_until_parked();

    let b1 = label_bounds(cx, "Save");
    cx.simulate_mouse_move(b1.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_some());

    cx.update(|window, cx| window.press("escape", cx));
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_none());
    assert!(
        !parent_escaped.get(),
        "Escape must not propagate past open tooltip"
    );

    cx.simulate_mouse_move(
        b1.center() + point(px(1.), px(0.)),
        None,
        Modifiers::default(),
    );
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_none());

    cx.simulate_mouse_move(point(px(2.), px(2.)), None, Modifiers::default());
    cx.run_until_parked();
    cx.simulate_mouse_move(b1.center(), None, Modifiers::default());
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_some());

    cx.simulate_mouse_down(b1.center(), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(b1.center(), MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_none());
}

#[gpui_kit::test]
fn keyboard_focus_opens_immediately_mouse_focus_does_not(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let focus_prev = view.read_with(cx, |v, _| v.focus_prev.clone());
    let focus_btn = view.read_with(cx, |v, _| v.focus_btn.clone());
    cx.run_until_parked();

    cx.update(|window, cx| {
        window.activate_window();
        focus_prev.focus(window, cx);
    });
    cx.run_until_parked();

    // Mouse-clicking the trigger focuses it without opening the tooltip.
    let b1 = label_bounds(cx, "Save");
    cx.simulate_click(b1.center(), Modifiers::default());
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_btn.is_focused(window)));
    assert!(
        tip_bounds(cx, "Save file").is_none(),
        "mouse focus must not open tooltip"
    );

    // Moving back and Tabbing onto the trigger opens immediately via keyboard focus.
    cx.simulate_mouse_move(point(px(2.), px(2.)), None, Modifiers::default());
    cx.update(|window, cx| focus_prev.focus(window, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.press("tab", cx));
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus_btn.is_focused(window)));
    assert!(
        tip_bounds(cx, "Save file").is_some(),
        "keyboard focus should open tooltip immediately"
    );
}

#[gpui_kit::test]
fn disabled_and_empty_render_no_tooltip_and_controlled_open_honored(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    view.update(cx, |v, cx| {
        v.controlled_open = Some(true);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_some());

    view.update(cx, |v, cx| {
        v.disabled = true;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(tip_bounds(cx, "Save file").is_none());

    view.update(cx, |v, cx| {
        v.disabled = false;
        v.empty_content = true;
        cx.notify();
    });
    cx.run_until_parked();
    let count = cx.update(|window, _| window.find_all_by_role(Role::Tooltip).len());
    assert_eq!(count, 0);
}
