use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui_kit::{
    Bounds, Context, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
    ParentElement as _, Pixels, Render, RenderOnce as _, Role, Styled as _, TestAppContext,
    Toggled, VisualTestContext, Window, accesskit, canvas, div, point, px,
};

use super::*;

struct Harness {
    pressed: Option<bool>,
    disabled: bool,
    clicks: Rc<Cell<usize>>,
    changes: Rc<std::cell::RefCell<Vec<bool>>>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let changes = self.changes.clone();
        let bounds = self.bounds.clone();
        let mut button = IconButton::new("icon-button")
            .icon(IconButtonIcon::path("icons/plus.svg"))
            .label("Add item")
            .disabled(self.disabled)
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            .on_change(move |next, _, _, _| changes.borrow_mut().push(next));
        if let Some(pressed) = self.pressed {
            button = button.pressed(pressed);
        }
        div().p(px(10.)).flex().child(
            div()
                .flex()
                .child(button)
                .child(canvas(|_, _, _| {}, |_, _, _, _| {}).w(px(0.)).h(px(0.)))
                .on_children_prepainted(move |children, _, _| {
                    bounds.set(children.first().copied());
                }),
        )
    }
}

struct Setup {
    clicks: Rc<Cell<usize>>,
    changes: Rc<std::cell::RefCell<Vec<bool>>>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

fn setup(
    cx: &mut TestAppContext,
    pressed: Option<bool>,
    disabled: bool,
) -> (gpui_kit::Entity<Harness>, &mut VisualTestContext, Setup) {
    cx.update(crate::init);
    let clicks = Rc::new(Cell::new(0));
    let changes = Rc::new(std::cell::RefCell::new(Vec::new()));
    let bounds = Rc::new(Cell::new(None));
    let (view, cx) = cx.add_window_view({
        let clicks = clicks.clone();
        let changes = changes.clone();
        let bounds = bounds.clone();
        move |_, _| Harness {
            pressed,
            disabled,
            clicks,
            changes,
            bounds,
        }
    });
    redraw(cx);
    (
        view,
        cx,
        Setup {
            clicks,
            changes,
            bounds,
        },
    )
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn press(cx: &mut VisualTestContext, key: &str) {
    let keystroke =
        cx.update(|_, cx| Keystroke::parse_with_os(key, cx.platform_info().os).unwrap());
    cx.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke });
    redraw(cx);
}

fn center(setup: &Setup) -> gpui_kit::Point<Pixels> {
    setup.bounds.get().expect("icon button laid out").center()
}

#[gpui_kit::test]
fn click_and_keyboard_activate(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, None, false);
    cx.simulate_click(center(&s), Modifiers::default());
    assert_eq!(s.clicks.get(), 1);
    redraw(cx);
    press(cx, "enter");
    press(cx, "space");
    assert_eq!(s.clicks.get(), 3);
    assert!(s.changes.borrow().is_empty());
}

#[gpui_kit::test]
fn toggle_mode_flips_pressed_on_click_and_keyboard(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, Some(false), false);
    cx.simulate_click(center(&s), Modifiers::default());
    redraw(cx);
    press(cx, "enter");
    press(cx, "space");
    assert_eq!(s.clicks.get(), 3);
    assert_eq!(s.changes.borrow().as_slice(), &[true, true, true]);
}

#[gpui_kit::test]
fn disabled_is_inert_and_skipped_by_tab(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, Some(true), true);
    cx.simulate_click(center(&s), Modifiers::default());
    cx.update(|window, cx| window.focus_next(cx));
    redraw(cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
    assert_eq!(s.clicks.get(), 0);
    assert!(s.changes.borrow().is_empty());
}

#[gpui_kit::test]
fn sizes_are_square(cx: &mut TestAppContext) {
    struct Probe {
        size: IconButtonSize,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child(
                    IconButton::new("icb")
                        .size(self.size)
                        .icon(IconButtonIcon::path("icons/plus.svg"))
                        .label("Add"),
                )
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    for (size, side) in [(IconButtonSize::Sm, 32.), (IconButtonSize::Md, 40.)] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe { size, bounds }
        });
        redraw(cx);
        let b = bounds.get().unwrap();
        assert_eq!(
            (b.size.width, b.size.height),
            (px(side), px(side)),
            "{size:?}"
        );
    }
}

#[gpui_kit::test]
fn accessibility_exposes_label_and_optional_toggled_state(cx: &mut TestAppContext) {
    type Captured = Arc<Mutex<Vec<accesskit::Node>>>;
    struct Probe(Captured);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let captured = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let mut nodes = captured.lock().unwrap();
                    fn capture<E: gpui_kit::Element>(element: E) -> accesskit::Node {
                        let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                        element.write_a11y_info(&mut node);
                        node
                    }
                    nodes.push(capture(
                        IconButton::new("plain")
                            .icon(IconButtonIcon::path("icons/plus.svg"))
                            .label("Add")
                            .render(window, cx)
                            .into_element(),
                    ));
                    nodes.push(capture(
                        IconButton::new("toggle-off")
                            .icon(IconButtonIcon::path("icons/pin.svg"))
                            .label("Pin")
                            .pressed(false)
                            .render(window, cx)
                            .into_element(),
                    ));
                    nodes.push(capture(
                        IconButton::new("toggle-on")
                            .icon(IconButtonIcon::path("icons/pin.svg"))
                            .label("Pin")
                            .pressed(true)
                            .render(window, cx)
                            .into_element(),
                    ));
                },
                |_, _, _, _| {},
            )
        }
    }
    cx.update(crate::init);
    let captured: Captured = Arc::default();
    let (_, cx) = cx.add_window_view({
        let captured = captured.clone();
        move |_, _| Probe(captured)
    });
    redraw(cx);
    let nodes = captured.lock().unwrap();
    assert_eq!(nodes[0].role(), Role::Button);
    assert_eq!(nodes[0].label(), Some("Add"));
    assert_eq!(nodes[0].toggled(), None);

    assert_eq!(nodes[1].role(), Role::Button);
    assert_eq!(nodes[1].label(), Some("Pin"));
    assert_eq!(nodes[1].toggled(), Some(Toggled::False));

    assert_eq!(nodes[2].toggled(), Some(Toggled::True));
}

#[gpui_kit::test]
fn press_scales_to_95_percent_and_skips_under_reduced_motion(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, None, false);
    let at = center(&s);

    cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, Modifiers::default());
    redraw(cx);
    cx.executor().advance_clock(Duration::from_millis(60));
    redraw(cx);
    let mid = last_scale(cx);
    assert!(PRESSED_SCALE < mid && mid < 1., "{mid}");
    cx.executor().advance_clock(Duration::from_millis(200));
    redraw(cx);
    assert_eq!(last_scale(cx), PRESSED_SCALE);
    assert_eq!(s.bounds.get().unwrap().size.height, px(40.));

    cx.simulate_mouse_up(
        point(px(500.), px(500.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    cx.executor().advance_clock(Duration::from_millis(200));
    redraw(cx);
    assert_eq!(last_scale(cx), 1.);

    cx.update(|_, cx| cx.set_reduce_motion(true));
    cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, Modifiers::default());
    redraw(cx);
    assert_eq!(last_scale(cx), 1.);
}

#[test]
fn tooltip_defaults_to_label_and_empty_title_omits_it() {
    let default_tip = IconButton::new("a").label("Filter");
    assert_eq!(default_tip.resolved_tooltip(), Some("Filter".into()));

    let custom_tip = IconButton::new("b").label("Filter").title("Filter runs");
    assert_eq!(custom_tip.resolved_tooltip(), Some("Filter runs".into()));

    let omitted_tip = IconButton::new("c").label("Filter").title("");
    assert_eq!(omitted_tip.resolved_tooltip(), None);
}

fn last_scale(cx: &mut VisualTestContext) -> f32 {
    redraw(cx);
    LAST_SCALE.with(Cell::get)
}

thread_local! {
    pub(super) static LAST_SCALE: Cell<f32> = const { Cell::new(1.) };
}
