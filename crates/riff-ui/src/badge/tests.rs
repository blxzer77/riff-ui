use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, canvas, div, px,
};

use super::*;
use crate::theme::ThemeMode;

#[test]
fn formats_counts_with_grouping_clamping_and_max_plus() {
    assert_eq!(format_count(-5, 99).as_ref(), "0");
    assert_eq!(format_count(0, 99).as_ref(), "0");
    assert_eq!(format_count(99, 99).as_ref(), "99");
    assert_eq!(format_count(100, 99).as_ref(), "99+");
    assert_eq!(format_count(1234, 9999).as_ref(), "1,234");
    assert_eq!(format_count(1200, 999).as_ref(), "999+");
}

#[test]
fn live_halo_pulses_during_first_70_percent_and_rests_after() {
    let ease = Easing::cubic_bezier(0.22, 1., 0.36, 1.).unwrap();
    let start = live_halo(Duration::ZERO, &ease).unwrap();
    assert!((start.0 - 1.).abs() < 1e-4);
    assert!((start.1 - 0.5).abs() < 1e-4);

    let mid = live_halo(Duration::from_millis(360), &ease).unwrap();
    assert!(1. < mid.0 && mid.0 < LIVE_MAX_SCALE);
    assert!(0. < mid.1 && mid.1 < LIVE_START_ALPHA);

    assert!(live_halo(Duration::from_millis(1260), &ease).is_none());
    assert!(live_halo(Duration::from_millis(1700), &ease).is_none());
}

#[test]
fn neutral_uses_distinct_dot_and_fg_in_light_and_dark() {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        let theme = RiffTheme::new(mode);
        let neutral = Look::of(BadgeTone::Neutral, &theme);
        assert_ne!(neutral.fg, neutral.dot);
        assert_ne!(neutral.fg, theme.muted);
    }
}

#[gpui_kit::test]
fn empty_badge_renders_zero_sized_nothing(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<Option<Bounds<Pixels>>>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.0.clone();
            div()
                .flex()
                .child(Badge::new().label(""))
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
    assert_eq!(bounds.get().unwrap().size.width, px(0.));
    assert_eq!(bounds.get().unwrap().size.height, px(0.));
}

#[gpui_kit::test]
fn sizes_and_count_min_width_match_spec(cx: &mut TestAppContext) {
    struct Probe {
        badge: Badge,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child(self.badge.clone())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    for (badge, expected_w, expected_h) in [
        (Badge::new().small().label("Done"), None, 20.),
        (Badge::new().medium().label("Done"), None, 22.),
        (Badge::new().small().count(3), Some(20.), 20.),
        (Badge::new().medium().count(3), Some(22.), 22.),
    ] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe {
                badge: badge.clone(),
                bounds,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let b = bounds.get().unwrap();
        assert_eq!(b.size.height, px(expected_h));
        if let Some(w) = expected_w {
            assert_eq!(b.size.width, px(w));
        }
    }
}

#[gpui_kit::test]
fn accessibility_label_attaches_to_bare_count_badge(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let el = Badge::new()
                        .warning()
                        .count(128)
                        .accessibility_label("128 waiting")
                        .render(window, cx)
                        .into_element();
                    let mut node = accesskit::Node::new(el.a11y_role().unwrap());
                    el.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("128 waiting"));
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
