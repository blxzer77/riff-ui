use std::cell::Cell;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;

#[test]
fn spin_angle_wraps_linearly_by_period() {
    assert!((angle_at(Duration::ZERO, NORMAL_SPEED) - 0.).abs() < 1e-5);
    assert!((angle_at(Duration::from_millis(375), NORMAL_SPEED) - TAU * 0.5).abs() < 1e-4);
    assert!((angle_at(Duration::from_millis(750), NORMAL_SPEED) - 0.).abs() < 1e-5);
    assert!((angle_at(Duration::from_millis(600), REDUCED_SPEED) - TAU * 0.25).abs() < 1e-4);
}

#[gpui_kit::test]
fn sizes_are_square_with_fourteen_default(cx: &mut TestAppContext) {
    struct Probe {
        spinner: Spinner,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child(self.spinner.clone())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    for (spinner, expected) in [
        (Spinner::new().freeze_motion_at(Duration::ZERO), 14.),
        (
            Spinner::new()
                .size(px(12.))
                .freeze_motion_at(Duration::ZERO),
            12.,
        ),
        (
            Spinner::new()
                .size(px(24.))
                .freeze_motion_at(Duration::ZERO),
            24.,
        ),
    ] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe { spinner, bounds }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let b = bounds.get().unwrap();
        assert_eq!(b.size.width, px(expected));
        assert_eq!(b.size.height, px(expected));
    }
}

#[gpui_kit::test]
fn status_role_and_label_follow_label_presence(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let silent = Spinner::new()
                        .freeze_motion_at(Duration::ZERO)
                        .render(window, cx)
                        .into_element();
                    assert!(silent.a11y_role().is_none());

                    let empty = Spinner::new()
                        .label("")
                        .freeze_motion_at(Duration::ZERO)
                        .render(window, cx)
                        .into_element();
                    assert!(empty.a11y_role().is_none());

                    let status = Spinner::new()
                        .label("Checking domain")
                        .freeze_motion_at(Duration::ZERO)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(status.a11y_role(), Some(accesskit::Role::Status));
                    let mut node = accesskit::Node::new(accesskit::Role::Status);
                    status.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Checking domain"));

                    verified.set(true);
                },
                |_, _, _, _| {},
            )
        }
    }
    cx.update(crate::init);
    let verified = Rc::new(Cell::new(false));
    let (_, cx) = cx.add_window_view({
        let verified = verified.clone();
        move |_, _| Probe(verified)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(verified.get());
}
