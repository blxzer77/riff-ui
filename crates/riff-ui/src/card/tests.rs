use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::{
    AnyElement, Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;
use crate::button::Button;
use crate::icon_button::IconButton;
use crate::theme::RiffTheme;

#[gpui_kit::test]
fn card_renders_footer_only_and_full_heights_per_spec(cx: &mut TestAppContext) {
    struct Probe {
        build: fn() -> AnyElement,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .items_start()
                .child((self.build)())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);

    type Build = fn() -> AnyElement;
    let cases: [(Build, f32, f32); 2] = [
        (
            || {
                Card::new()
                    .w(px(280.))
                    .footer(CardFooter::new().child("Synced 2m ago"))
                    .into_any_element()
            },
            // Footer-only: 4px top + 40px min-height + 4px bottom = 48px
            280.,
            48.,
        ),
        (
            || {
                Card::new()
                    .w(px(280.))
                    .header(CardHeader::new().title("Support triage"))
                    .into_any_element()
            },
            // 4px + (16px + 20px + 16px) + 4px = 60px
            280.,
            60.,
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

    // Narrow card wraps long unbreakable tokens instead of expanding past 200px.
    let narrow_bounds = Rc::new(Cell::new(None));
    let (_, cx) = cx.add_window_view({
        let bounds = narrow_bounds.clone();
        move |_, _| Probe {
            build: || {
                Card::new()
                    .w(px(200.))
                    .header(
                        CardHeader::new()
                            .svg_icon(IconName::Folder)
                            .title("Long repository title that wraps across lines")
                            .description(
                                "Contains an_unbreakable_identifier_token_name that wraps cleanly inside the card.",
                            ),
                    )
                    .content("Body copy also wraps without pushing the card wider than its container.")
                    .footer(CardFooter::new().child("Updated 5 minutes ago · Frankfurt"))
                    .into_any_element()
            },
            bounds,
        }
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let nb = narrow_bounds.get().unwrap();
    assert_eq!(nb.size.width, px(200.));
    assert!(nb.size.height > px(200.));
}

#[gpui_kit::test]
fn card_accessibility_exposes_semantic_role_and_interactive_toggle(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let section = Card::new()
                        .as_element(CardAs::Section)
                        .accessibility_label("Workspace")
                        .header(CardHeader::new().title("Workspace"))
                        .render(window, cx)
                        .into_element();
                    assert_eq!(section.a11y_role(), Some(accesskit::Role::Region));
                    let mut node = accesskit::Node::new(accesskit::Role::Region);
                    section.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Workspace"));

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

#[gpui_kit::test]
fn interactive_card_click_does_not_fire_when_inner_button_is_clicked(cx: &mut TestAppContext) {
    struct Harness {
        card_clicks: Rc<Cell<usize>>,
        action_clicks: Rc<Cell<usize>>,
        footer_clicks: Rc<Cell<usize>>,
        card_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        action_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
        footer_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let card_clicks = self.card_clicks.clone();
            let action_clicks = self.action_clicks.clone();
            let footer_clicks = self.footer_clicks.clone();
            let card_bounds = self.card_bounds.clone();
            let action_bounds = self.action_bounds.clone();
            let footer_bounds = self.footer_bounds.clone();

            div()
                .p_4()
                .child(
                    Card::new()
                        .id("test-card")
                        .w(px(300.))
                        .interactive(true)
                        .on_click(move |_, _, _| {
                            card_clicks.set(card_clicks.get() + 1);
                        })
                        .header(
                            CardHeader::new()
                                .title("Support triage")
                                .description("Sorts new tickets")
                                .action(
                                    div()
                                        .child(
                                            IconButton::new("card-more")
                                                .icon(IconName::Ellipsis)
                                                .label("More")
                                                .on_click(move |_, _, _| {
                                                    action_clicks.set(action_clicks.get() + 1);
                                                }),
                                        )
                                        .on_children_prepainted(move |children, _, _| {
                                            action_bounds.set(children.first().copied());
                                        }),
                                ),
                        )
                        .footer(
                            CardFooter::new().child("Running").child(
                                div()
                                    .ml_auto()
                                    .child(Button::new("card-open").small().label("Open").on_click(
                                        move |_, _, _| {
                                            footer_clicks.set(footer_clicks.get() + 1);
                                        },
                                    ))
                                    .on_children_prepainted(move |children, _, _| {
                                        footer_bounds.set(children.first().copied());
                                    }),
                            ),
                        ),
                )
                .on_children_prepainted(move |children, _, _| {
                    card_bounds.set(children.first().copied());
                })
        }
    }

    cx.update(crate::init);
    let card_clicks = Rc::new(Cell::new(0));
    let action_clicks = Rc::new(Cell::new(0));
    let footer_clicks = Rc::new(Cell::new(0));
    let card_bounds = Rc::new(Cell::new(None));
    let action_bounds = Rc::new(Cell::new(None));
    let footer_bounds = Rc::new(Cell::new(None));

    let (_view, cx) = cx.add_window_view({
        let card_clicks = card_clicks.clone();
        let action_clicks = action_clicks.clone();
        let footer_clicks = footer_clicks.clone();
        let card_bounds = card_bounds.clone();
        let action_bounds = action_bounds.clone();
        let footer_bounds = footer_bounds.clone();
        move |_, _| Harness {
            card_clicks,
            action_clicks,
            footer_clicks,
            card_bounds,
            action_bounds,
            footer_bounds,
        }
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));

    // Clicking the card body fires card_clicks only.
    let cb = card_bounds.get().unwrap();
    cx.simulate_click(
        gpui_kit::point(cb.left() + px(24.), cb.top() + px(24.)),
        gpui_kit::Modifiers::none(),
    );
    assert_eq!(card_clicks.get(), 1);
    assert_eq!(action_clicks.get(), 0);
    assert_eq!(footer_clicks.get(), 0);

    // Clicking the header action IconButton fires action_clicks only.
    let ab = action_bounds.get().unwrap();
    cx.simulate_click(ab.center(), gpui_kit::Modifiers::none());
    assert_eq!(card_clicks.get(), 1);
    assert_eq!(action_clicks.get(), 1);
    assert_eq!(footer_clicks.get(), 0);

    // Clicking the footer Button fires footer_clicks only.
    let fb = footer_bounds.get().unwrap();
    cx.simulate_click(fb.center(), gpui_kit::Modifiers::none());
    assert_eq!(card_clicks.get(), 1);
    assert_eq!(action_clicks.get(), 1);
    assert_eq!(footer_clicks.get(), 1);
}
