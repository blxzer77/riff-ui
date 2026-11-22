use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers,
    ParentElement as _, Pixels, Render, RenderOnce as _, Styled as _, TestAppContext,
    VisualTestContext, Window, accesskit, canvas, div, point, px,
};

use super::*;

struct Harness {
    loading: bool,
    disabled: bool,
    label: &'static str,
    clicks: Rc<Cell<usize>>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let bounds = self.bounds.clone();
        div().p(px(10.)).flex().child(
            div()
                .flex()
                .child(
                    Button::new("button")
                        .label(self.label)
                        .loading(self.loading)
                        .disabled(self.disabled)
                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                )
                .child(canvas(|_, _, _| {}, |_, _, _, _| {}).w(px(0.)).h(px(0.)))
                .on_children_prepainted(move |children, _, _| {
                    bounds.set(children.first().copied());
                }),
        )
    }
}

struct Setup {
    clicks: Rc<Cell<usize>>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

fn setup(
    cx: &mut TestAppContext,
    loading: bool,
    disabled: bool,
) -> (gpui_kit::Entity<Harness>, &mut VisualTestContext, Setup) {
    cx.update(crate::init);
    let clicks = Rc::new(Cell::new(0));
    let bounds = Rc::new(Cell::new(None));
    let (view, cx) = cx.add_window_view({
        let clicks = clicks.clone();
        let bounds = bounds.clone();
        move |_, _| Harness {
            loading,
            disabled,
            label: "Save",
            clicks,
            bounds,
        }
    });
    redraw(cx);
    (view, cx, Setup { clicks, bounds })
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
    setup.bounds.get().expect("button laid out").center()
}

#[gpui_kit::test]
fn click_and_keyboard_activate(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, false, false);
    cx.simulate_click(center(&s), Modifiers::default());
    assert_eq!(s.clicks.get(), 1);
    redraw(cx);
    press(cx, "enter");
    press(cx, "space");
    assert_eq!(s.clicks.get(), 3);
}

#[gpui_kit::test]
fn loading_ignores_activation_but_keeps_focus(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, true, false);
    cx.simulate_click(center(&s), Modifiers::default());
    redraw(cx);
    assert!(
        cx.update(|window, cx| window.focused(cx).is_some()),
        "a loading button still takes focus"
    );
    press(cx, "enter");
    press(cx, "space");
    assert_eq!(s.clicks.get(), 0);
}

#[gpui_kit::test]
fn loading_button_is_reached_by_tab(cx: &mut TestAppContext) {
    let (_, cx, _) = setup(cx, true, false);
    cx.update(|window, cx| window.focus_next(cx));
    redraw(cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_some()));
}

#[gpui_kit::test]
fn disabled_is_inert_and_skipped_by_tab(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, false, true);
    cx.simulate_click(center(&s), Modifiers::default());
    cx.update(|window, cx| window.focus_next(cx));
    redraw(cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
    assert_eq!(s.clicks.get(), 0);
}

#[gpui_kit::test]
fn sizes_and_icon_only_geometry(cx: &mut TestAppContext) {
    let (view, cx, s) = setup(cx, false, false);
    assert_eq!(s.bounds.get().unwrap().size.height, px(40.));

    // An icon-only button is square at each size.
    struct IconOnly {
        size: ButtonSize,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for IconOnly {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child(
                    Button::new("icon")
                        .size(self.size)
                        .icon(ButtonIcon::path("icons/plus.svg")),
                )
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    drop(view);
    for (size, side) in [
        (ButtonSize::Sm, 32.),
        (ButtonSize::Md, 40.),
        (ButtonSize::Lg, 48.),
    ] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| IconOnly { size, bounds }
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
fn width_follows_content(cx: &mut TestAppContext) {
    let (view, cx, s) = setup(cx, false, false);
    let short = s.bounds.get().unwrap().size.width;
    // Content changes glide; under reduced motion the new width is immediate.
    cx.update(|_, cx| cx.set_reduce_motion(true));
    view.update(cx, |this, cx| {
        this.label = "Save all changes";
        cx.notify();
    });
    redraw(cx);
    redraw(cx);
    let long = s.bounds.get().unwrap().size.width;
    assert!(long > short + px(40.), "{short:?} -> {long:?}");
}

#[gpui_kit::test]
fn width_glides_over_480ms(cx: &mut TestAppContext) {
    let (view, cx, s) = setup(cx, false, false);
    let from = s.bounds.get().unwrap().size.width;
    view.update(cx, |this, cx| {
        this.label = "Save all changes";
        cx.notify();
    });
    redraw(cx);
    let start = s.bounds.get().unwrap().size.width;
    assert!((start - from).abs() < px(0.5), "starts from the old width");

    cx.executor().advance_clock(Duration::from_millis(240));
    redraw(cx);
    let mid = s.bounds.get().unwrap().size.width;
    cx.executor().advance_clock(Duration::from_millis(300));
    redraw(cx);
    let end = s.bounds.get().unwrap().size.width;
    assert!(from < mid && mid < end, "{from:?} < {mid:?} < {end:?}");
    cx.executor().advance_clock(Duration::from_millis(100));
    redraw(cx);
    assert_eq!(
        s.bounds.get().unwrap().size.width,
        end,
        "settled after 480ms"
    );
}

#[gpui_kit::test]
fn loading_marks_the_node_busy(cx: &mut TestAppContext) {
    type Captured = Arc<Mutex<Vec<accesskit::Node>>>;
    struct Probe(Captured);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let captured = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let mut nodes = captured.lock().unwrap();
                    for loading in [false, true] {
                        let element = Button::new(("b", usize::from(loading)))
                            .label("Save")
                            .loading(loading)
                            .render(window, cx)
                            .into_element();
                        let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                        element.write_a11y_info(&mut node);
                        nodes.push(node);
                    }
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
    let (idle, busy) = (&nodes[0], &nodes[1]);
    assert_eq!(idle.role(), gpui_kit::Role::Button);
    assert_eq!(idle.label(), Some("Save"));
    assert!(!idle.is_busy() && !idle.is_disabled());
    assert!(busy.is_busy() && busy.is_disabled());
}

#[gpui_kit::test]
fn press_scales_paint_not_layout(cx: &mut TestAppContext) {
    let (view, cx, s) = setup(cx, false, false);
    let at = center(&s);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, Modifiers::default());
    redraw(cx);
    assert_eq!(last_scale(cx), PRESSED_SCALE);
    // The layout box keeps its size, as with a CSS transform.
    assert_eq!(s.bounds.get().unwrap().size.height, px(40.));

    // Releasing anywhere ends the press.
    cx.simulate_mouse_up(
        point(px(500.), px(500.)),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    redraw(cx);
    assert_eq!(last_scale(cx), 1.);

    // `:active:not([data-loading])`: a loading button does not shrink.
    view.update(cx, |this, cx| {
        this.loading = true;
        cx.notify();
    });
    redraw(cx);
    cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, Modifiers::default());
    redraw(cx);
    assert_eq!(last_scale(cx), 1.);
}

#[gpui_kit::test]
fn press_scale_eases_over_160ms(cx: &mut TestAppContext) {
    let (_, cx, s) = setup(cx, false, false);
    cx.simulate_mouse_down(
        center(&s),
        gpui_kit::MouseButton::Left,
        Modifiers::default(),
    );
    redraw(cx);
    cx.executor().advance_clock(Duration::from_millis(60));
    redraw(cx);
    let mid = last_scale(cx);
    assert!(PRESSED_SCALE < mid && mid < 1., "{mid}");
    cx.executor().advance_clock(Duration::from_millis(200));
    redraw(cx);
    assert_eq!(last_scale(cx), PRESSED_SCALE);
}

fn last_scale(cx: &mut VisualTestContext) -> f32 {
    redraw(cx);
    LAST_SCALE.with(Cell::get)
}

thread_local! {
    /// The scale the button painted with last, recorded under `cfg(test)`.
    pub(super) static LAST_SCALE: Cell<f32> = const { Cell::new(1.) };
}
