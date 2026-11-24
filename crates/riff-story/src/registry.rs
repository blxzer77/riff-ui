//! Every component riff-story shows. To add one:
//!
//! 1. `stories/<slug>.json` — its cells, shared with the reference generator;
//! 2. `tools/reference/adapters/<slug>.jsx` — the cells as riff-uiux renders them;
//! 3. `src/components/<slug>.rs` — the cells as riff-ui renders them, and
//!    optionally a free-form demo;
//! 4. one entry below.
//!
//! `every_story_cell_builds` (in `components`) fails if a story names a cell
//! or prop its component does not handle.

use gpui_kit::{AnyElement, AnyView, App, Window};

use crate::components;
use crate::story::{self, PinnedState, Props, Story};

/// What a cell is shown with besides its props: a pinned pointer or focus
/// state, and the moment looping animations are held at, so the live cell
/// shows the same frame as its reference picture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pin {
    pub state: Option<PinnedState>,
    pub freeze: Option<std::time::Duration>,
}

/// Builds the live component for one cell. Rejects props it does not handle.
pub type CellFn = fn(id: &str, props: &Props, pin: Pin) -> Result<AnyElement, String>;
/// Builds a component's free-form demo page.
pub type DemoFn = fn(&mut Window, &mut App) -> AnyView;

pub struct Entry {
    pub story: Story,
    pub cell: CellFn,
    pub demo: Option<DemoFn>,
}

/// A page that is not a component, such as the token palette.
pub struct Page {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub view: DemoFn,
}

macro_rules! entry {
    ($slug:literal, $module:ident) => {
        Entry {
            story: story::parse(include_str!(concat!("../../../stories/", $slug, ".json")))
                .unwrap_or_else(|e| panic!("stories/{}.json: {e}", $slug)),
            cell: components::$module::cell,
            demo: components::$module::DEMO,
        }
    };
}

pub fn entries() -> Vec<Entry> {
    vec![
        entry!("badge", badge),
        entry!("button", button),
        entry!("icon-button", icon_button),
        entry!("kbd", kbd),
        entry!("separator", separator),
        entry!("skeleton", skeleton),
        entry!("spinner", spinner),
    ]
}

pub fn pages() -> Vec<Page> {
    vec![Page {
        slug: "tokens",
        name: "Tokens",
        category: "Foundations",
        view: components::tokens::view,
    }]
}
