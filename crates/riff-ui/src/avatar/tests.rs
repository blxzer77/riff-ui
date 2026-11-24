use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{
    AnyElement, Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;
use crate::theme::RiffTheme;

#[test]
fn initials_follow_intl_segmenter_and_suffix_rules() {
    assert_eq!(initials_of("Mara Lindqvist"), "ML");
    assert_eq!(initials_of("Cher"), "C");
    assert_eq!(initials_of("Christopher Montgomery III"), "CM");
    assert_eq!(initials_of("🦊 Fox Team"), "🦊");
    assert_eq!(initials_of("(Ana) Ruiz"), "AR");
    assert_eq!(initials_of("Élodie Vézina"), "ÉV");
    assert_eq!(initials_of("   "), "");
}

#[test]
fn hue_and_agent_pattern_are_deterministic_and_symmetric() {
    assert_eq!(hue_of("mara"), hue_of("mara"));
    assert!(hue_of("mara") < 8);
    let cells = agent_pattern("triage");
    assert_eq!(cells[0], cells[2]);
    assert_eq!(cells[3], cells[5]);
    assert_eq!(cells[6], cells[8]);
    assert!(cells.iter().filter(|&&c| c).count() >= 5);
}

#[test]
fn working_wave_pulses_with_diagonal_delay() {
    let ease = Easing::cubic_bezier(0.42, 0., 0.58, 1.).unwrap();
    assert_eq!(working_cell_opacity(Duration::ZERO, 120, &ease), 1.);
    assert_eq!(
        working_cell_opacity(Duration::from_millis(60), 120, &ease),
        1.
    );
    let mid = working_cell_opacity(Duration::from_millis(600), 0, &ease);
    assert!((mid - 0.2).abs() < 1e-4);
}

#[gpui_kit::test]
fn sizes_and_group_overlap_widths_match_spec(cx: &mut TestAppContext) {
    struct Probe {
        build: fn() -> AnyElement,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child((self.build)())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);

    type Build = fn() -> AnyElement;
    let cases: [(Build, f32, f32); 4] = [
        (
            || Avatar::new().size(AvatarSize::S20).into_any_element(),
            20.,
            20.,
        ),
        (
            || Avatar::new().size(AvatarSize::S24).into_any_element(),
            24.,
            24.,
        ),
        (|| Avatar::new().into_any_element(), 32., 32.),
        (
            || {
                AvatarGroup::new()
                    .size(AvatarSize::S24)
                    .max(2)
                    .total(5)
                    .avatars([
                        Avatar::new().name("Mara Lindqvist"),
                        Avatar::new().name("Jonas Weber"),
                    ])
                    .into_any_element()
            },
            // 24 + (24 - 6) + (24 - 6) = 60px (+3 chip <= 24px min-width)
            60.,
            24.,
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
fn accessibility_names_include_status_and_group_counts(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let named = Avatar::new()
                        .name("Mara Lindqvist")
                        .status(AvatarStatus::Online)
                        .render(window, cx)
                        .into_element();
                    assert_eq!(named.a11y_role(), Some(accesskit::Role::Image));
                    let mut node = accesskit::Node::new(accesskit::Role::Image);
                    named.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Mara Lindqvist, online"));

                    let anon = Avatar::new().render(window, cx).into_element();
                    assert!(anon.a11y_role().is_none());

                    let hidden = Avatar::new()
                        .name("Mara Lindqvist")
                        .accessibility_hidden(true)
                        .render(window, cx)
                        .into_element();
                    assert!(hidden.a11y_role().is_none());

                    let group = AvatarGroup::new()
                        .accessibility_label("5 reviewers")
                        .max(2)
                        .total(5)
                        .avatars([Avatar::new().name("A"), Avatar::new().name("B")])
                        .render(window, cx)
                        .into_element();
                    assert_eq!(group.a11y_role(), Some(accesskit::Role::Group));
                    let mut gnode = accesskit::Node::new(accesskit::Role::Group);
                    group.write_a11y_info(&mut gnode);
                    assert_eq!(gnode.label(), Some("5 reviewers"));

                    let _ = RiffTheme::global(cx);
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
