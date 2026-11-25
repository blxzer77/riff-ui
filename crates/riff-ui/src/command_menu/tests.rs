use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::base::Root;
use gpui_kit::{
    AppContext as _, Context, Entity, ParentElement as _, Render, TestAppContext, Window, div,
};

use super::*;

type RanEventLog = Rc<RefCell<Vec<(String, Option<String>)>>>;

struct CommandHarness {
    query: Option<SharedString>,
    events: RanEventLog,
    closed: Rc<Cell<usize>>,
}

impl Render for CommandHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let events = self.events.clone();
        let closed = self.closed.clone();
        div().child(
            CommandMenu::new("test-cm")
                .auto_focus(true)
                .query(self.query.clone())
                .on_run(move |id, q, _, _| {
                    events
                        .borrow_mut()
                        .push((id.to_owned(), q.map(str::to_owned)));
                })
                .on_close(move |_, _| {
                    closed.set(closed.get() + 1);
                }),
        )
    }
}

fn mount(
    cx: &mut TestAppContext,
    query: Option<SharedString>,
) -> (Entity<CommandHarness>, &mut gpui_kit::VisualTestContext) {
    cx.update(crate::init);
    let holder: Rc<Cell<Option<Entity<CommandHarness>>>> = Rc::new(Cell::new(None));
    let clone = holder.clone();
    let (_, visual_cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| CommandHarness {
            query,
            events: Rc::new(RefCell::new(Vec::new())),
            closed: Rc::new(Cell::new(0)),
        });
        clone.set(Some(view.clone()));
        Root::new(view, window, cx)
    });
    visual_cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
    let view = holder.take().expect("harness built");
    (view, visual_cx)
}

#[gpui_kit::test]
fn filters_diacritics_and_prepends_assistant_fallback(cx: &mut TestAppContext) {
    let (view, vcx) = mount(cx, Some("  café  ".into()));
    let computed_empty = compute_list(&default_command_groups(), "", px(40.));
    assert_eq!(computed_empty.groups.len(), 4);
    assert_eq!(computed_empty.suggestions.len(), 5);
    assert_eq!(computed_empty.flat.len(), 16);

    let cafe_groups = vec![
        CommandGroup::new("suggestions", "Suggestions")
            .command(CommandItem::new("cafe", "Open Café menu").keywords("coffee")),
    ];
    let matched = compute_list(&cafe_groups, "cafe", px(40.));
    assert_eq!(matched.groups.len(), 2);
    assert_eq!(matched.groups[0].key.as_ref(), "ask");
    assert_eq!(
        matched.groups[0].commands[0].item.label.as_ref(),
        "Ask assistant about “cafe”"
    );
    assert_eq!(matched.groups[1].commands[0].item.id.as_ref(), "cafe");

    let span = find_highlight_span("Open Café menu", "cafe").expect("span");
    assert_eq!(&"Open Café menu"[span.0..span.1], "Café");

    // Enter runs the assistant fallback with trimmed query.
    vcx.simulate_keystrokes("enter");
    view.read_with(vcx, |v, _| {
        assert_eq!(
            &*v.events.borrow(),
            &[("ask".to_owned(), Some("café".to_owned()))]
        );
    });
}

#[gpui_kit::test]
fn keyboard_navigation_tab_groups_shortcuts_and_escape(cx: &mut TestAppContext) {
    let (view, vcx) = mount(cx, None);

    // Tab jumps from Suggestions -> Share (first item: slack), then Enter runs it.
    vcx.simulate_keystrokes("tab enter");
    view.read_with(vcx, |v, _| {
        assert_eq!(&*v.events.borrow(), &[("slack".to_owned(), None)]);
    });

    // Shift-Tab returns to Suggestions, Down moves to 2nd item ("ask").
    vcx.simulate_keystrokes("shift-tab down enter");
    view.read_with(vcx, |v, _| {
        assert_eq!(
            &*v.events.borrow(),
            &[("slack".to_owned(), None), ("ask".to_owned(), None)]
        );
    });

    // Ctrl+3 runs 3rd suggestion ("task") directly.
    vcx.simulate_keystrokes("ctrl-3");
    view.read_with(vcx, |v, _| {
        assert_eq!(v.events.borrow().last().unwrap().0, "task");
    });

    // Escape on empty field invokes on_close.
    vcx.simulate_keystrokes("escape");
    view.read_with(vcx, |v, _| {
        assert_eq!(v.closed.get(), 1);
    });
}
