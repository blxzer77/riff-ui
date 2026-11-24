use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Context, Element as _, IntoElement, ParentElement as _, Pixels, Render,
    RenderOnce as _, Styled as _, TestAppContext, Window, accesskit, canvas, div, px,
};

use super::*;

#[test]
fn describes_platform_keys_aliases_and_spoken_names() {
    assert_eq!(
        describe("mod", KbdPlatform::Mac),
        KeyPart {
            label: "⌘".into(),
            spoken: "Command".into(),
            glyph: true,
        }
    );
    assert_eq!(
        describe("mod", KbdPlatform::Other),
        KeyPart {
            label: "Ctrl".into(),
            spoken: "Control".into(),
            glyph: false,
        }
    );
    assert_eq!(
        describe("option", KbdPlatform::Mac),
        KeyPart {
            label: "⌥".into(),
            spoken: "Option".into(),
            glyph: true,
        }
    );
    assert_eq!(
        describe("return", KbdPlatform::Other),
        KeyPart {
            label: "Enter".into(),
            spoken: "Enter".into(),
            glyph: false,
        }
    );
    assert_eq!(
        describe("arrowup", KbdPlatform::Other),
        KeyPart {
            label: "↑".into(),
            spoken: "Up arrow".into(),
            glyph: true,
        }
    );
    assert_eq!(
        describe("a", KbdPlatform::Mac),
        KeyPart {
            label: "A".into(),
            spoken: "A".into(),
            glyph: false,
        }
    );
    assert_eq!(
        describe(".", KbdPlatform::Other),
        KeyPart {
            label: ".".into(),
            spoken: "Period".into(),
            glyph: false,
        }
    );
}

#[gpui_kit::test]
fn empty_kbd_renders_zero_sized_nothing(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<Option<Bounds<Pixels>>>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.0.clone();
            div()
                .flex()
                .child(Kbd::new().keys(["", "  "]).label(""))
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
fn sizes_and_min_width_match_spec(cx: &mut TestAppContext) {
    struct Probe {
        kbd: Kbd,
        bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let bounds = self.bounds.clone();
            div()
                .flex()
                .child(self.kbd.clone())
                .on_children_prepainted(move |children, _, _| bounds.set(children.first().copied()))
        }
    }
    cx.update(crate::init);
    for (kbd, expected_w, expected_h) in [
        (Kbd::new().small().key("K"), Some(18.), 18.),
        (Kbd::new().medium().key("K"), Some(22.), 22.),
        (
            Kbd::new()
                .small()
                .platform(KbdPlatform::Mac)
                .keys(["mod", "K"]),
            None,
            18.,
        ),
        (
            Kbd::new()
                .medium()
                .platform(KbdPlatform::Other)
                .keys(["mod", "K"]),
            None,
            22.,
        ),
    ] {
        let bounds = Rc::new(Cell::new(None));
        let (_, cx) = cx.add_window_view({
            let bounds = bounds.clone();
            move |_, _| Probe {
                kbd: kbd.clone(),
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
fn accessibility_spells_out_combo_for_screen_readers(cx: &mut TestAppContext) {
    struct Probe(Rc<Cell<bool>>);
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let verified = self.0.clone();
            canvas(
                move |_, window, cx| {
                    let mac = Kbd::new()
                        .platform(KbdPlatform::Mac)
                        .keys(["mod", "shift", "a"])
                        .render(window, cx)
                        .into_element();
                    let mut node = accesskit::Node::new(mac.a11y_role().unwrap());
                    mac.write_a11y_info(&mut node);
                    assert_eq!(node.label(), Some("Command Shift A"));

                    let other = Kbd::new()
                        .platform(KbdPlatform::Other)
                        .keys(["mod", "."])
                        .render(window, cx)
                        .into_element();
                    let mut node2 = accesskit::Node::new(other.a11y_role().unwrap());
                    other.write_a11y_info(&mut node2);
                    assert_eq!(node2.label(), Some("Control Period"));

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
