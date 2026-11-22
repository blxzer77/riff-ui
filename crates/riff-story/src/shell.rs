//! riff-story's window: a sidebar of components grouped by category, and the
//! selected component's Compare or Demo tab.

use std::collections::{BTreeMap, HashMap};

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use riff_ui::button::Button;
use riff_ui::theme::{RiffTheme, ThemeMode, set_mode};

use crate::compare::{self, Mode, Status};
use crate::registry::{self, Entry, Page};
use crate::story::{self, Reference};

/// What the main area shows for a component.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Tab {
    #[default]
    Compare,
    Demo,
}

/// Where riff-story starts, from the command line (see `main`).
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub slug: Option<String>,
    pub tab: Tab,
    pub mode: Mode,
}

/// A sidebar item: a component or a free-standing page.
#[derive(Clone, Copy)]
enum Item {
    Entry(usize),
    Page(usize),
}

pub struct Shell {
    entries: Vec<Entry>,
    pages: Vec<Page>,
    selected: Item,
    tab: Tab,
    mode: Mode,
    search: Entity<InputState>,
    query: String,
    /// Manifests by (slug, theme mode); `None` caches "no pictures".
    references: HashMap<(String, &'static str), Option<Reference>>,
    /// Demo views stay alive while their component is selected.
    demo: Option<(usize, AnyView)>,
    page: Option<(usize, AnyView)>,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    pub fn new(options: Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let entries = registry::entries();
        let pages = registry::pages();
        let selected = options
            .slug
            .as_deref()
            .and_then(|slug| {
                entries
                    .iter()
                    .position(|e| e.story.slug == slug)
                    .map(Item::Entry)
                    .or_else(|| pages.iter().position(|p| p.slug == slug).map(Item::Page))
            })
            .unwrap_or(Item::Entry(0));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search components"));
        let subscription = cx.subscribe_in(&search, window, |this, search, event, _, cx| {
            if let InputEvent::Change = event {
                this.query = search.read(cx).value().to_lowercase();
                cx.notify();
            }
        });
        Self {
            entries,
            pages,
            selected,
            tab: options.tab,
            mode: options.mode,
            search,
            query: String::new(),
            references: HashMap::new(),
            demo: None,
            page: None,
            _subscriptions: vec![subscription],
        }
    }

    fn reference(&mut self, index: usize, theme: ThemeMode) -> Option<Reference> {
        let story = &self.entries[index].story;
        let mode = if theme.is_dark() { "dark" } else { "light" };
        self.references
            .entry((story.slug.clone(), mode))
            .or_insert_with(|| story::load_reference(story, mode))
            .clone()
    }

    /// Re-reads manifests and drops cached pictures, after the generator ran.
    fn reload(&mut self, cx: &mut App) {
        for reference in self.references.drain().filter_map(|(_, r)| r) {
            for key in reference.cells.keys() {
                cx.remove_asset::<ImgResourceLoader>(&Resource::Path(
                    reference.picture(key).into(),
                ));
            }
        }
    }

    fn select(&mut self, item: Item, cx: &mut Context<Self>) {
        self.selected = item;
        cx.notify();
    }

    fn matches(&self, name: &str, category: &str) -> bool {
        self.query.is_empty()
            || name.to_lowercase().contains(&self.query)
            || category.to_lowercase().contains(&self.query)
    }

    fn sidebar(&self, theme: &RiffTheme, cx: &mut Context<Self>) -> AnyElement {
        // Category → items, sorted like riff-uiux's own gallery.
        let mut groups: BTreeMap<&str, Vec<(Item, &str, bool)>> = BTreeMap::new();
        for (index, entry) in self.entries.iter().enumerate() {
            let story = &entry.story;
            if self.matches(&story.name, &story.category) {
                let selected = matches!(self.selected, Item::Entry(i) if i == index);
                groups.entry(&story.category).or_default().push((
                    Item::Entry(index),
                    &story.name,
                    selected,
                ));
            }
        }
        for (index, page) in self.pages.iter().enumerate() {
            if self.matches(page.name, page.category) {
                let selected = matches!(self.selected, Item::Page(i) if i == index);
                groups.entry(page.category).or_default().push((
                    Item::Page(index),
                    page.name,
                    selected,
                ));
            }
        }
        let total = self.entries.len();

        div()
            .id("sidebar")
            .w(px(240.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.subtle)
            .child(
                div()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .flex()
                    .flex_col()
                    .child(div().text_sm().child("riff-ui"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted)
                            .child(format!("{total} components")),
                    ),
            )
            .child(div().px_3().pb_2().child(Input::new(&self.search)))
            .child(
                div()
                    .id("sidebar-list")
                    .flex_1()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_4()
                    .children(groups.into_iter().map(|(category, items)| {
                        div()
                            .flex()
                            .flex_col()
                            .pt_3()
                            .child(
                                div()
                                    .px_2()
                                    .pb_1()
                                    .flex()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(theme.muted)
                                    .child(category.to_owned())
                                    .child(items.len().to_string()),
                            )
                            .children(items.into_iter().map(|(item, name, selected)| {
                                let id = match item {
                                    Item::Entry(i) => ElementId::from(("entry", i)),
                                    Item::Page(i) => ElementId::from(("page", i)),
                                };
                                div()
                                    .id(id)
                                    .px_2()
                                    .py_1()
                                    .rounded(theme.radius_sm)
                                    .text_sm()
                                    .cursor_pointer()
                                    .when(selected, |this| this.bg(theme.surface))
                                    .hover(|this| this.bg(theme.surface))
                                    .child(name.to_owned())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select(item, cx);
                                    }))
                            }))
                    })),
            )
            .into_any_element()
    }

    fn toolbar(
        &self,
        theme: &RiffTheme,
        has_demo: bool,
        status: Option<Status>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let segment = |id: &'static str, label: &'static str, on: bool| {
            let button = Button::new(id).small().label(label);
            if on {
                button.secondary()
            } else {
                button.ghost()
            }
        };
        let next_mode = if theme.mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        let is_entry = matches!(self.selected, Item::Entry(_));

        div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .when(is_entry, |this| {
                this.child(
                    segment("tab-compare", "Compare", self.tab == Tab::Compare).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.tab = Tab::Compare;
                            cx.notify();
                        }),
                    ),
                )
                .when(has_demo, |this| {
                    this.child(segment("tab-demo", "Demo", self.tab == Tab::Demo).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.tab = Tab::Demo;
                            cx.notify();
                        }),
                    ))
                })
            })
            .when(is_entry && self.tab == Tab::Compare, |this| {
                this.child(div().w(px(12.))).children(Mode::ALL.map(|mode| {
                    segment(mode.label(), mode.label(), self.mode == mode).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.mode = mode;
                            cx.notify();
                        },
                    ))
                }))
            })
            .child(div().flex_1())
            .when_some(status, |this, status| {
                this.child(status_badge(status, theme)).child(
                    Button::new("reload")
                        .small()
                        .ghost()
                        .label("Reload pictures")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.reload(cx);
                            cx.notify();
                        })),
                )
            })
            .child(
                Button::new("theme")
                    .small()
                    .secondary()
                    .label(if theme.mode.is_dark() {
                        "Light"
                    } else {
                        "Dark"
                    })
                    .on_click(move |_, _, cx| set_mode(next_mode, cx)),
            )
            .into_any_element()
    }
}

fn status_badge(status: Status, theme: &RiffTheme) -> impl IntoElement {
    let (label, color, background) = match status {
        Status::Current => ("Pictures current", theme.success, theme.success_soft),
        Status::Stale => ("Pictures stale", theme.warning, theme.warning_soft),
        Status::Missing => ("No pictures", theme.muted, theme.subtle),
    };
    div()
        .px_2()
        .py_1()
        .rounded(theme.radius_sm)
        .bg(background)
        .text_xs()
        .text_color(color)
        .child(label)
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();

        let (title, description, body, has_demo, status) = match self.selected {
            Item::Entry(index) => {
                let reference = self.reference(index, theme.mode);
                let status = compare::status(reference.as_ref());
                let entry = &self.entries[index];
                let has_demo = entry.demo.is_some();
                let tab = if has_demo { self.tab } else { Tab::Compare };
                let body = match tab {
                    Tab::Compare => {
                        let notice = match status {
                            Status::Current => None,
                            Status::Stale => Some(
                                "The story changed since these pictures were made; cells may be \
                                 missing or out of date.",
                            ),
                            Status::Missing => {
                                Some("No reference pictures for this component yet.")
                            }
                        };
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .when_some(notice, |this, notice| {
                                this.child(regenerate_notice(notice, &entry.story.slug, &theme))
                            })
                            .child(compare::view(entry, reference.as_ref(), self.mode, &theme))
                            .into_any_element()
                    }
                    Tab::Demo => {
                        if !matches!(self.demo, Some((i, _)) if i == index) {
                            let make = entry.demo.expect("has a demo");
                            self.demo = Some((index, make(window, cx)));
                        }
                        self.demo
                            .as_ref()
                            .expect("just set")
                            .1
                            .clone()
                            .into_any_element()
                    }
                };
                (
                    entry.story.name.clone(),
                    entry.story.description.clone(),
                    body,
                    has_demo,
                    Some(status),
                )
            }
            Item::Page(index) => {
                if !matches!(self.page, Some((i, _)) if i == index) {
                    self.page = Some((index, (self.pages[index].view)(window, cx)));
                }
                let page = &self.pages[index];
                (
                    page.name.to_owned(),
                    String::new(),
                    self.page
                        .as_ref()
                        .expect("just set")
                        .1
                        .clone()
                        .into_any_element(),
                    false,
                    None,
                )
            }
        };
        let toolbar = self.toolbar(&theme, has_demo, status, cx);
        // Built last: it borrows the entries' names until the tree is assembled.
        let sidebar = self.sidebar(&theme, cx);

        div()
            .size_full()
            .flex()
            .bg(theme.surface)
            .text_color(theme.ink)
            .font_family(theme.font_family.clone())
            .child(sidebar)
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_6()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_lg().child(title))
                            .when(!description.is_empty(), |this| {
                                this.child(
                                    div().text_sm().text_color(theme.muted).child(description),
                                )
                            }),
                    )
                    .child(toolbar)
                    .child(body),
            )
    }
}

fn regenerate_notice(message: &str, slug: &str, theme: &RiffTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded(theme.radius)
        .bg(theme.subtle)
        .text_sm()
        .child(message.to_owned())
        .child(div().text_xs().text_color(theme.muted).child(format!(
            "Run `python3 tools/reference/build.py {slug}`, then Reload pictures."
        )))
}
