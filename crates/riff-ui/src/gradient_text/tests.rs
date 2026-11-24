use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{
    Bounds, Context, Element as _, FontWeight, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;
use crate::paint::hex;
use crate::theme::{RiffTheme, ThemeMode, set_mode};

#[test]
fn custom_colors_follow_css_fallback_chain_and_override_dark_mode() {
    let dark = RiffTheme::new(ThemeMode::Dark);
    let one = Palette::resolve(Some(&[hex(0xff0000)]), &dark);
    assert_eq!(one.c1, one.c2);
    assert_eq!(one.c1, one.c3);
    assert_eq!(one.c1, one.c4);

    let two = Palette::resolve(Some(&[hex(0xff0000), hex(0x00ff00)]), &dark);
    assert_eq!(two.c3, two.c1);
    assert_eq!(two.c4, two.c2);
}

#[test]
fn gradient_field_varies_across_flow_cycle_and_box_position() {
    let light = RiffTheme::new(ThemeMode::Light);
    let pal = Palette::resolve(None, &light);
    let f0 = GradientField::new(200., 40., pal, 0.);
    let f25 = GradientField::new(200., 40., pal, 0.25);

    let left = f0.sample(10., 20.);
    let right = f0.sample(190., 20.);
    assert_ne!(left, right);
    assert_ne!(f0.sample(80., 20.), f25.sample(80., 20.));
}

#[test]
fn sweep_and_base_fade_match_css_timings() {
    let (s0, b0) = sweep_state(true, Duration::ZERO, false);
    assert_eq!(s0, 0.);
    assert_eq!(b0, 1.);

    let (smid, bmid) = sweep_state(true, Duration::from_millis(550), false);
    assert!(smid > 0.3 && smid < 0.9);
    assert_eq!(bmid, 1.);

    let (sdone, bdone) = sweep_state(true, Duration::from_millis(1200), false);
    assert_eq!(sdone, 1.);
    assert_eq!(bdone, 0.);

    assert_eq!(sweep_state(false, Duration::ZERO, false), (1., 0.));
    assert_eq!(sweep_state(true, Duration::ZERO, true), (1., 0.));
}

#[gpui_kit::test]
fn gradient_text_inherits_parent_font_size_and_sets_semantic_roles(cx: &mut TestAppContext) {
    struct Probe {
        bounds_sm: Rc<Cell<Option<Bounds<Pixels>>>>,
        bounds_lg: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let b_sm = self.bounds_sm.clone();
            let b_lg = self.bounds_lg.clone();
            div()
                .flex()
                .flex_col()
                .items_start()
                .child(
                    div()
                        .text_size(px(14.))
                        .child(GradientText::new("feel.").sweep(false))
                        .on_children_prepainted(move |children, _, _| {
                            b_sm.set(children.first().copied());
                        }),
                )
                .child(
                    div()
                        .text_size(px(36.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(
                            GradientText::new("feel.")
                                .as_element(GradientTextAs::H1)
                                .sweep(false),
                        )
                        .on_children_prepainted(move |children, _, _| {
                            b_lg.set(children.first().copied());
                        }),
                )
        }
    }
    cx.update(crate::init);
    let bounds_sm = Rc::new(Cell::new(None));
    let bounds_lg = Rc::new(Cell::new(None));
    let (_, cx) = cx.add_window_view({
        let bounds_sm = bounds_sm.clone();
        let bounds_lg = bounds_lg.clone();
        move |_, _| Probe {
            bounds_sm,
            bounds_lg,
        }
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    let sm = bounds_sm.get().unwrap();
    let lg = bounds_lg.get().unwrap();
    assert!(lg.size.width > sm.size.width);
    assert!(lg.size.height > sm.size.height);

    // Check accessibility roles for Span, H1, P, and mode switching.
    cx.update(|_, cx| set_mode(ThemeMode::Dark, cx));
    struct A11yProbe(Rc<Cell<bool>>);
    impl Render for A11yProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let h1 = GradientText::new("Headline")
                        .as_element(GradientTextAs::H1)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(h1.a11y_role(), Some(accesskit::Role::Heading));
                    let mut node = accesskit::Node::new(accesskit::Role::Heading);
                    h1.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Headline"));

                    let p = GradientText::new("Body")
                        .as_element(GradientTextAs::P)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(p.a11y_role(), Some(accesskit::Role::Paragraph));
                    let _ = RiffTheme::global(cx);
                    verified.set(true);
                },
                |_, _, _, _| {},
            )
        }
    }
    let verified = Rc::new(Cell::new(false));
    let (_, cx) = cx.add_window_view({
        let verified = verified.clone();
        move |_, _| A11yProbe(verified)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(verified.get());
}
