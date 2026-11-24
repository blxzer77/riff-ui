use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;
use crate::theme::RiffTheme;

#[test]
fn sweep_moves_from_minus_one_to_one_during_first_60_percent_and_rests() {
    let ease = RiffTheme::new(crate::theme::ThemeMode::Light).ease;
    let start = sweep_translate(Duration::ZERO, DEFAULT_SPEED, &ease).unwrap();
    assert!((start - (-1.)).abs() < 1e-4);
    let mid = sweep_translate(Duration::from_millis(480), DEFAULT_SPEED, &ease).unwrap();
    assert!(mid > -1. && mid < 1.);
    assert!(sweep_translate(Duration::from_millis(960), DEFAULT_SPEED, &ease).is_none());
    assert!(sweep_translate(Duration::from_millis(1400), DEFAULT_SPEED, &ease).is_none());
}

#[test]
fn loading_holds_opacity_zero_until_delay_then_fades_in() {
    let delay = Duration::from_millis(150);
    let fade = Duration::from_millis(300);
    assert_eq!(loading_opacity(Duration::ZERO, delay, fade), 0.);
    assert_eq!(loading_opacity(Duration::from_millis(150), delay, fade), 0.);
    let mid = loading_opacity(Duration::from_millis(300), delay, fade);
    assert!(mid > 0. && mid < 1.);
    assert_eq!(loading_opacity(Duration::from_millis(450), delay, fade), 1.);
    assert_eq!(loading_opacity(Duration::from_millis(800), delay, fade), 1.);
}

#[gpui_kit::test]
fn sizes_clamping_and_line_box_match_spec(cx: &mut TestAppContext) {
    struct Probe {
        build: fn() -> AnyElement,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .w(px(120.))
                .child((self.build)())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);

    type Build = fn() -> AnyElement;
    let cases: [(Build, f32, f32); 3] = [
        (
            || {
                Skeleton::new()
                    .freeze_motion_at(Duration::ZERO)
                    .into_any_element()
            },
            120.,
            12.,
        ),
        (
            || {
                Skeleton::new()
                    .width(px(220.))
                    .height(px(16.))
                    .freeze_motion_at(Duration::ZERO)
                    .into_any_element()
            },
            120.,
            16.,
        ),
        (
            || {
                Skeleton::new()
                    .width(px(80.))
                    .height(px(10.))
                    .line_height(px(20.))
                    .freeze_motion_at(Duration::ZERO)
                    .into_any_element()
            },
            80.,
            20.,
        ),
    ];

    for (build, expected_w, expected_h) in cases {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe { build, bounds }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let b = bounds.get().unwrap();
        assert_eq!(b.size.width, px(expected_w));
        assert_eq!(b.size.height, px(expected_h));
    }
}

#[gpui_kit::test]
fn skeleton_is_hidden_and_loading_exposes_busy_status(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let skl = Skeleton::new()
                        .freeze_motion_at(Duration::ZERO)
                        .render(window, cx)
                        .into_element();
                    assert!(skl.a11y_role().is_none());

                    let loading = Loading::new()
                        .label("Loading runs")
                        .freeze_motion_at(Duration::from_millis(500))
                        .child(Skeleton::new().freeze_motion_at(Duration::ZERO))
                        .render(window, cx)
                        .into_element();
                    assert_eq!(loading.a11y_role(), Some(accesskit::Role::Status));
                    let mut node = accesskit::Node::new(accesskit::Role::Status);
                    loading.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Loading runs"));
                    assert!(node.is_busy());

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
