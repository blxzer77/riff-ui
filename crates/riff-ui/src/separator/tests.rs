use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;

#[gpui_kit::test]
fn horizontal_fills_container_and_is_one_pixel_high(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<Option<Bounds<Pixels>>>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.0.clone();
            div()
                .flex()
                .w(px(200.))
                .child(Separator::new())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    let bounds = Rc::new(Cell::new(None));
    let (_, cx) = cx.add_window_view({
        let bounds = bounds.clone();
        move |_, _| Probe(bounds)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let b = bounds.get().unwrap();
    assert_eq!(b.size.width, px(200.));
    assert_eq!(b.size.height, px(1.));
}

#[gpui_kit::test]
fn vertical_is_one_pixel_wide_and_eighteen_high_by_default(cx: &mut TestAppContext) {
    struct Probe {
        sep: Separator,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .items_center()
                .h(px(32.))
                .child(self.sep.clone())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    for (sep, expected_h) in [
        (Separator::new().vertical(), 18.),
        (Separator::new().vertical().length(px(24.)), 24.),
        (Separator::new().vertical().label("ignored"), 18.),
    ] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe { sep, bounds }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let b = bounds.get().unwrap();
        assert_eq!(b.size.width, px(1.));
        assert_eq!(b.size.height, px(expected_h));
    }
}

#[gpui_kit::test]
fn labelled_horizontal_keeps_minimum_line_width_and_wraps(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<Option<Bounds<Pixels>>>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.0.clone();
            div()
                .flex()
                .w(px(140.))
                .child(Separator::new().label("A longer break label that wraps"))
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    let bounds = Rc::new(Cell::new(None));
    let (_, cx) = cx.add_window_view({
        let bounds = bounds.clone();
        move |_, _| Probe(bounds)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let b = bounds.get().unwrap();
    assert_eq!(b.size.width, px(140.));
    assert!(
        b.size.height > px(18.),
        "expected wrapped height > 18px, got {:?}",
        b.size.height
    );
}

#[gpui_kit::test]
fn decorative_and_semantic_separators_set_expected_a11y(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let deco = Separator::new().render(window, cx).into_element();
                    assert!(deco.a11y_role().is_none());

                    let deco_labelled = Separator::new()
                        .label("Earlier")
                        .render(window, cx)
                        .into_element();
                    assert!(deco_labelled.a11y_role().is_none());

                    let real_h = Separator::new()
                        .label("Earlier")
                        .decorative(false)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(real_h.a11y_role(), Some(accesskit::Role::Splitter));
                    let mut node_h = accesskit::Node::new(accesskit::Role::Splitter);
                    real_h.write_a11y_info(&mut node_h);
                    assert_eq!(node_h.label(), Some("Earlier"));
                    assert_eq!(node_h.orientation(), None);

                    let real_v = Separator::new()
                        .vertical()
                        .label("ignored")
                        .decorative(false)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(real_v.a11y_role(), Some(accesskit::Role::Splitter));
                    let mut node_v = accesskit::Node::new(accesskit::Role::Splitter);
                    real_v.write_a11y_info(&mut node_v);
                    assert_eq!(node_v.label(), None);
                    assert_eq!(node_v.orientation(), Some(accesskit::Orientation::Vertical));

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
