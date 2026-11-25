//! riff-uiux `CommandMenu`: searchable ⌘K command palette — gray outer tray
//! wrapping a bordered search card, grouped tool actions with vector brand
//! logos and dynamic `⌘1–⌘5` badges, everyday commands with icons, assistant
//! fallback when typing, query match highlights, gliding selection pill, Tab
//! section jumping, and live footer announcement.

mod paint_logos;
mod style;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::input::{Input, InputEvent, InputState};
use gpui_kit::base::{Transition, transition};
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, Context, Corners, ElementId, Entity, Focusable as _,
    FontWeight, Global, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Task, WeakEntity, Window, WindowId,
    canvas, div, fill, point, prelude::FluentBuilder as _, px, svg,
};

use crate::kbd::KbdPlatform;
use crate::paint;
use crate::theme::RiffTheme;
pub use paint_logos::CommandLogo;
use paint_logos::LogoGlyph;
use style::{
    ANNOUNCEMENT_TTL, DONE_DURATION, DONE_GAP, DONE_ICON_SIZE, ENTER_HINT_FONT_SIZE,
    ENTER_HINT_GAP, ENTER_HINT_ICON_SIZE, FOOT_FONT_SIZE, FOOT_GAP, FOOT_HEIGHT, FOOT_PAD_X,
    GROUP_LABEL_FONT_SIZE, GROUP_LABEL_HEIGHT, GROUP_LABEL_LINE_HEIGHT, GROUP_LABEL_PAD_BOTTOM,
    GROUP_LABEL_PAD_TOP, GROUP_LABEL_PAD_X, ICON_GLYPH_SIZE, ITEM_FONT_SIZE, ITEM_GAP, ITEM_HEIGHT,
    ITEM_HEIGHT_NARROW, ITEM_LINE_HEIGHT, ITEM_PAD_X, KBD_FONT_SIZE, KBD_HEIGHT, KBD_MIN_WIDTH,
    KBD_PAD_X, KEYS_GAP, LIST_HEIGHT_DURATION, LIST_MAX_HEIGHT, LIST_MAX_HEIGHT_NARROW,
    LIST_PAD_BOTTOM, LIST_PAD_TOP, LIST_PAD_X, Look, MAX_WIDTH, NARROW_BREAKPOINT, PILL_DURATION,
    RAN_DURATION, SEARCH_FONT_SIZE, SEARCH_GAP, SEARCH_HEIGHT, SEARCH_ICON_SIZE, SEARCH_PAD_X,
    TEXT_GAP, TILE_RADIUS, TILE_SIZE, TRAY_PAD,
};

/// An icon for an everyday command row.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandIcon(pub(crate) SharedString);

impl<T: gpui_kit::assets::IconNamed> From<T> for CommandIcon {
    fn from(icon: T) -> Self {
        Self(icon.path())
    }
}

impl CommandIcon {
    /// Asset path to an SVG icon, e.g. `"icons/file-text.svg"`.
    pub fn path(path: impl Into<SharedString>) -> Self {
        Self(path.into())
    }
}

/// Visual leading slot rendered in the 26×26 column of a command row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandLeading {
    /// Built-in vector tool logo rendered inside a rounded 26×26 tile.
    Logo(CommandLogo),
    /// Monochrome command glyph (17×17).
    Icon(CommandIcon),
    /// Custom SVG asset rendered at 14×14 inside a rounded 26×26 logo tile.
    TileSvg(SharedString),
}

/// One command inside a [`CommandGroup`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandItem {
    pub id: SharedString,
    pub label: SharedString,
    pub hint: Option<SharedString>,
    pub leading: Option<CommandLeading>,
    pub keys: Vec<SharedString>,
    pub keywords: Option<SharedString>,
}

impl CommandItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: None,
            leading: None,
            keys: Vec::new(),
            keywords: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        let h = hint.into();
        self.hint = (!h.is_empty()).then_some(h);
        self
    }

    pub fn logo(mut self, logo: CommandLogo) -> Self {
        self.leading = Some(CommandLeading::Logo(logo));
        self
    }

    pub fn icon(mut self, icon: impl Into<CommandIcon>) -> Self {
        self.leading = Some(CommandLeading::Icon(icon.into()));
        self
    }

    pub fn tile_svg(mut self, path: impl Into<SharedString>) -> Self {
        self.leading = Some(CommandLeading::TileSvg(path.into()));
        self
    }

    pub fn keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn keywords(mut self, keywords: impl Into<SharedString>) -> Self {
        let k = keywords.into();
        self.keywords = (!k.is_empty()).then_some(k);
        self
    }
}

/// A named section of commands inside [`CommandMenu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandGroup {
    pub id: SharedString,
    pub label: SharedString,
    pub commands: Vec<CommandItem>,
}

impl CommandGroup {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            commands: Vec::new(),
        }
    }

    pub fn command(mut self, command: CommandItem) -> Self {
        self.commands.push(command);
        self
    }

    pub fn commands(mut self, commands: impl IntoIterator<Item = CommandItem>) -> Self {
        self.commands.extend(commands);
        self
    }
}

/// The default Suggestions, Share, General, and Go to command groups matching
/// riff-uiux's `CommandMenu`.
pub fn default_command_groups() -> Vec<CommandGroup> {
    vec![
        CommandGroup::new("suggestions", "Suggestions")
            .command(
                CommandItem::new("email", "Write an email")
                    .hint("Gmail")
                    .logo(CommandLogo::Gmail)
                    .keywords("mail send compose"),
            )
            .command(
                CommandItem::new("ask", "Ask assistant")
                    .hint("Summaries, drafts, answers")
                    .logo(CommandLogo::AskSpark)
                    .keywords("ai chat question"),
            )
            .command(
                CommandItem::new("task", "Create a task")
                    .hint("ClickUp")
                    .logo(CommandLogo::ClickUp)
                    .keywords("todo issue"),
            )
            .command(
                CommandItem::new("crm", "Add contact to CRM")
                    .hint("HubSpot")
                    .logo(CommandLogo::HubSpot)
                    .keywords("lead deal company"),
            )
            .command(
                CommandItem::new("meet", "Schedule a meeting")
                    .hint("Google Calendar")
                    .logo(CommandLogo::Calendar)
                    .keywords("call invite event"),
            ),
        CommandGroup::new("share", "Share")
            .command(
                CommandItem::new("slack", "Post an update")
                    .hint("#team-updates")
                    .logo(CommandLogo::Slack)
                    .keywords("message channel"),
            )
            .command(
                CommandItem::new("notion", "Save as page")
                    .hint("Notion · Meeting notes")
                    .logo(CommandLogo::Notion)
                    .keywords("doc wiki"),
            ),
        CommandGroup::new("general", "General")
            .command(
                CommandItem::new("new-doc", "New document")
                    .icon(CommandIcon::path("icons/file-text.svg"))
                    .keys(["N"]),
            )
            .command(
                CommandItem::new("invite", "Invite teammates")
                    .icon(CommandIcon::path("icons/user-plus.svg"))
                    .keys(["I"]),
            )
            .command(
                CommandItem::new("upload", "Upload files")
                    .icon(CommandIcon::path("icons/upload.svg"))
                    .keys(["U"]),
            )
            .command(
                CommandItem::new("theme", "Switch to dark mode")
                    .icon(CommandIcon::path("icons/moon.svg"))
                    .keywords("theme appearance light")
                    .keys(["⇧", "D"]),
            ),
        CommandGroup::new("go", "Go to")
            .command(
                CommandItem::new("home", "Home")
                    .icon(CommandIcon::path("icons/house.svg"))
                    .keys(["G", "H"]),
            )
            .command(
                CommandItem::new("inbox", "Inbox")
                    .icon(CommandIcon::path("icons/inbox.svg"))
                    .keys(["G", "I"]),
            )
            .command(
                CommandItem::new("reports", "Reports")
                    .icon(CommandIcon::path("icons/chart-bar.svg"))
                    .keys(["G", "R"]),
            )
            .command(
                CommandItem::new("settings", "Settings")
                    .icon(CommandIcon::path("icons/settings.svg"))
                    .keys(["G", "S"]),
            )
            .command(
                CommandItem::new("help", "Help & feedback")
                    .icon(CommandIcon::path("icons/circle-question-mark.svg"))
                    .keys(["?"]),
            ),
    ]
}

pub type RunHandler = Rc<dyn Fn(&str, Option<&str>, &mut Window, &mut App)>;
pub type CloseHandler = Rc<dyn Fn(&mut Window, &mut App)>;
pub type QueryChangeHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// A searchable ⌘K command menu matching riff-uiux's `CommandMenu`.
///
/// ```ignore
/// CommandMenu::new("commands")
///     .auto_focus(true)
///     .on_run(|id, query, _, _| run(id, query))
///     .on_close(|_, _| close());
/// ```
#[derive(IntoElement)]
pub struct CommandMenu {
    id: ElementId,
    groups: Option<Vec<CommandGroup>>,
    placeholder: SharedString,
    query: Option<SharedString>,
    default_query: Option<SharedString>,
    selected_key: Option<SharedString>,
    ran_preview: Option<(SharedString, SharedString)>,
    width: Option<Pixels>,
    platform: Option<KbdPlatform>,
    auto_focus: bool,
    on_run: Option<RunHandler>,
    on_close: Option<CloseHandler>,
    on_query_change: Option<QueryChangeHandler>,
    freeze_at: Option<Duration>,
    inline: bool,
}

impl CommandMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            groups: None,
            placeholder: "Type a command or search…".into(),
            query: None,
            default_query: None,
            selected_key: None,
            ran_preview: None,
            width: None,
            platform: None,
            auto_focus: false,
            on_run: None,
            on_close: None,
            on_query_change: None,
            freeze_at: None,
            inline: false,
        }
    }

    pub fn groups(mut self, groups: Vec<CommandGroup>) -> Self {
        self.groups = Some(groups);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        let p = placeholder.into();
        if !p.is_empty() {
            self.placeholder = p;
        }
        self
    }

    pub fn query(mut self, query: impl Into<Option<SharedString>>) -> Self {
        self.query = query.into();
        self
    }

    pub fn default_query(mut self, query: impl Into<SharedString>) -> Self {
        self.default_query = Some(query.into());
        self
    }

    /// Pins the highlighted command row (by command key/id) for previews or controlled use.
    pub fn selected_key(mut self, key: impl Into<Option<SharedString>>) -> Self {
        self.selected_key = key.into();
        self
    }

    /// Shows a footer confirmation banner (`ran_key`, `ran_label`) in static previews.
    #[doc(hidden)]
    pub fn ran_preview(
        mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        self.ran_preview = Some((id.into(), label.into()));
        self
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn platform(mut self, platform: KbdPlatform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// `(id, query)` — `"ask"` passes `Some(trimmed_query)` matching riff-uiux.
    pub fn on_run(
        mut self,
        handler: impl Fn(&str, Option<&str>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_run = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    pub fn on_query_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_query_change = Some(Rc::new(handler));
        self
    }

    pub fn freeze_motion_at(mut self, at: Duration) -> Self {
        self.freeze_at = Some(at);
        self
    }

    #[doc(hidden)]
    pub fn inline_preview(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

#[derive(Clone)]
struct RanState {
    key: SharedString,
    label: SharedString,
    since: Instant,
}

pub(crate) struct CommandMenuController {
    input: Entity<InputState>,
    scroll: ScrollHandle,
    active_key: Option<SharedString>,
    ran: Option<RanState>,
    clear_ran_task: Option<Task<()>>,
    did_auto_focus: bool,
    initialized: bool,
    suppress_change: Rc<Cell<bool>>,
    last_placeholder: SharedString,
    last_controlled_query: Option<SharedString>,
    on_run: Option<RunHandler>,
    on_close: Option<CloseHandler>,
    on_query_change: Option<QueryChangeHandler>,
    frame_flat: Vec<FlatCommand>,
    frame_groups: Vec<RenderedGroup>,
    frame_suggestions: Vec<FlatCommand>,
    frame_selected: Option<SharedString>,
    frame_trimmed_query: String,
    _sub: Subscription,
}

#[derive(Default)]
struct CommandKeyRegistry {
    interceptor: Option<Subscription>,
    controllers: HashMap<WindowId, Vec<WeakEntity<CommandMenuController>>>,
}

impl Global for CommandKeyRegistry {}

impl CommandKeyRegistry {
    fn register(window: &mut Window, controller: &Entity<CommandMenuController>, cx: &mut App) {
        if cx.default_global::<Self>().interceptor.is_none() {
            let sub = cx.intercept_keystrokes(|event, window, cx| {
                let wid = window.window_handle().window_id();
                let list: Vec<Entity<CommandMenuController>> = cx
                    .try_global::<Self>()
                    .and_then(|reg| reg.controllers.get(&wid))
                    .map(|v| v.iter().filter_map(|w| w.upgrade()).collect())
                    .unwrap_or_default();
                if list.is_empty() {
                    return;
                }

                let ks = &event.keystroke;
                let key = ks.key.as_str();
                if (ks.modifiers.platform || ks.modifiers.control)
                    && !ks.modifiers.alt
                    && key.eq_ignore_ascii_case("k")
                {
                    if let Some(target) = list.last() {
                        window.prevent_default();
                        cx.stop_propagation();
                        let input = target.read(cx).input.clone();
                        input.update(cx, |s, cx| {
                            s.focus(window, cx);
                            s.select_all(window, cx);
                        });
                    }
                    return;
                }

                let focused = list
                    .iter()
                    .rev()
                    .find(|ctrl| ctrl.read(cx).input.focus_handle(cx).is_focused(window))
                    .cloned();
                let Some(ctrl) = focused else {
                    return;
                };

                let (flat, groups, suggestions, selected, trimmed) = {
                    let s = ctrl.read(cx);
                    (
                        s.frame_flat.clone(),
                        s.frame_groups.clone(),
                        s.frame_suggestions.clone(),
                        s.frame_selected.clone(),
                        s.frame_trimmed_query.clone(),
                    )
                };

                if key == "down" || key == "up" {
                    if !flat.is_empty() {
                        window.prevent_default();
                        cx.stop_propagation();
                        let cur_idx = selected
                            .as_ref()
                            .and_then(|k| flat.iter().position(|c| &c.key == k))
                            .unwrap_or(0);
                        let delta: isize = if key == "down" { 1 } else { -1 };
                        let next_idx =
                            ((cur_idx as isize + delta).rem_euclid(flat.len() as isize)) as usize;
                        let target = &flat[next_idx];
                        ctrl.update(cx, |s, cx| {
                            s.active_key = Some(target.key.clone());
                            s.ensure_row_visible(target, GROUP_LABEL_HEIGHT);
                            cx.notify();
                        });
                    }
                } else if key == "enter" {
                    if let Some(sel) = selected
                        .as_ref()
                        .and_then(|k| flat.iter().find(|c| &c.key == k))
                    {
                        window.prevent_default();
                        cx.stop_propagation();
                        ctrl.update(cx, |s, cx| {
                            s.run_entry(sel, &trimmed, window, cx);
                        });
                    }
                } else if key == "tab" && groups.len() > 1 {
                    window.prevent_default();
                    cx.stop_propagation();
                    let cur_group = selected
                        .as_ref()
                        .and_then(|k| flat.iter().find(|c| &c.key == k))
                        .map(|c| c.group_idx)
                        .unwrap_or(0);
                    let next_group = if ks.modifiers.shift {
                        if cur_group == 0 {
                            groups.len() - 1
                        } else {
                            cur_group - 1
                        }
                    } else {
                        (cur_group + 1) % groups.len()
                    };
                    if let Some(first) = groups[next_group].commands.first() {
                        ctrl.update(cx, |s, cx| {
                            s.active_key = Some(first.key.clone());
                            s.ensure_row_visible(first, GROUP_LABEL_HEIGHT);
                            cx.notify();
                        });
                    }
                } else if key == "escape" {
                    window.prevent_default();
                    cx.stop_propagation();
                    ctrl.update(cx, |s, cx| {
                        let cur = s.input.read(cx).value();
                        if !cur.is_empty() {
                            s.active_key = None;
                            s.input.update(cx, |inp, cx| inp.set_value("", window, cx));
                            if let Some(cb) = s.on_query_change.clone() {
                                cb("", window, cx);
                            }
                            cx.notify();
                        } else if let Some(cb) = s.on_close.clone() {
                            cb(window, cx);
                        }
                    });
                } else if (ks.modifiers.platform || ks.modifiers.control)
                    && matches!(key, "1" | "2" | "3" | "4" | "5")
                    && let Ok(n) = key.parse::<usize>()
                    && let Some(target) = suggestions.get(n - 1)
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    ctrl.update(cx, |s, cx| {
                        s.run_entry(target, &trimmed, window, cx);
                    });
                }
            });
            cx.default_global::<Self>().interceptor = Some(sub);
        }

        let wid = window.window_handle().window_id();
        let list = cx
            .default_global::<Self>()
            .controllers
            .entry(wid)
            .or_default();
        list.retain(|w| w.upgrade().is_some());
        if !list.iter().any(|w| w.entity_id() == controller.entity_id()) {
            list.push(controller.downgrade());
        }
    }
}

#[derive(Clone, Default)]
struct CommandFrameSync {
    placeholder: SharedString,
    controlled_query: Option<SharedString>,
    auto_focus: bool,
    on_run: Option<RunHandler>,
    on_close: Option<CloseHandler>,
    on_query_change: Option<QueryChangeHandler>,
}

impl CommandMenuController {
    fn new(
        placeholder: SharedString,
        initial_query: Option<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            if let Some(q) = initial_query {
                s = s.default_value(q);
            }
            s
        });

        let suppress_change = Rc::new(Cell::new(false));
        let suppress = suppress_change.clone();
        let sub = cx.subscribe_in(&input, window, move |this, _, event, window, cx| {
            if let InputEvent::Change = event {
                if suppress.replace(false) {
                    return;
                }
                this.active_key = None;
                let value = this.input.read(cx).value();
                if let Some(cb) = this.on_query_change.clone() {
                    cb(&value, window, cx);
                }
                cx.notify();
            }
        });

        Self {
            input,
            scroll: ScrollHandle::new(),
            active_key: None,
            ran: None,
            clear_ran_task: None,
            did_auto_focus: false,
            initialized: false,
            suppress_change,
            last_placeholder: placeholder,
            last_controlled_query: None,
            on_run: None,
            on_close: None,
            on_query_change: None,
            frame_flat: Vec::new(),
            frame_groups: Vec::new(),
            frame_suggestions: Vec::new(),
            frame_selected: None,
            frame_trimmed_query: String::new(),
            _sub: sub,
        }
    }

    fn sync_frame(&mut self, sync: CommandFrameSync, window: &mut Window, cx: &mut Context<Self>) {
        self.on_run = sync.on_run;
        self.on_close = sync.on_close;
        self.on_query_change = sync.on_query_change;

        self.last_placeholder = sync.placeholder;

        if let Some(q) = sync.controlled_query {
            let current = self.input.read(cx).value();
            if self.last_controlled_query.as_ref() != Some(&q) || current != q {
                self.suppress_change.set(true);
                self.input.update(cx, |s, cx| {
                    s.set_value(q.clone(), window, cx);
                });
                self.suppress_change.set(false);
                self.last_controlled_query = Some(q);
            }
        } else {
            self.last_controlled_query = None;
        }

        if sync.auto_focus && !self.did_auto_focus {
            self.did_auto_focus = true;
            self.input.update(cx, |s, cx| s.focus(window, cx));
        }
    }

    fn run_entry(
        &mut self,
        entry: &FlatCommand,
        trimmed_query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_key = Some(entry.key.clone());
        let announcement: SharedString = if entry.is_ask {
            format!("Asked the assistant about “{trimmed_query}”").into()
        } else {
            entry.item.label.clone()
        };
        let now = Instant::now();
        self.ran = Some(RanState {
            key: entry.key.clone(),
            label: announcement,
            since: now,
        });

        let weak = cx.weak_entity();
        self.clear_ran_task = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(ANNOUNCEMENT_TTL).await;
            _ = weak.update(cx, |this, cx| {
                if this
                    .ran
                    .as_ref()
                    .is_some_and(|_r| now.elapsed() >= ANNOUNCEMENT_TTL)
                {
                    this.ran = None;
                    cx.notify();
                }
            });
        }));

        if let Some(cb) = self.on_run.clone() {
            if entry.is_ask {
                cb("ask", Some(trimmed_query), window, cx);
            } else {
                cb(&entry.item.id, None, window, cx);
            }
        }
        cx.notify();
    }

    fn ensure_row_visible(&self, row: &FlatCommand, label_h: Pixels) {
        let view_h = self.scroll.bounds().size.height;
        if view_h <= px(0.) {
            return;
        }
        let view_top = (-self.scroll.offset().y).max(px(0.));
        let header = if row.first_in_group && row.group_has_label {
            label_h
        } else {
            px(0.)
        };
        let top = (row.y - px(8.) - header).max(px(0.));
        let bottom = row.y + row.h + px(8.);
        if top < view_top {
            self.scroll.set_offset(point(px(0.), -top));
        } else if bottom > view_top + view_h {
            self.scroll.set_offset(point(px(0.), -(bottom - view_h)));
        }
    }
}

#[derive(Clone)]
struct RenderedGroup {
    key: SharedString,
    label: Option<SharedString>,
    commands: Vec<FlatCommand>,
}

#[derive(Clone)]
struct FlatCommand {
    key: SharedString,
    is_ask: bool,
    suggestion_n: Option<u8>,
    group_idx: usize,
    first_in_group: bool,
    group_has_label: bool,
    item: CommandItem,
    y: Pixels,
    h: Pixels,
}

struct ComputedList {
    groups: Vec<RenderedGroup>,
    flat: Vec<FlatCommand>,
    suggestions: Vec<FlatCommand>,
    content_height: Pixels,
}

fn fold_char(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å'
        | 'Ā' | 'Ă' | 'Ą' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' | 'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ'
        | 'Ė' | 'Ę' | 'Ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'Ĩ' | 'Ī'
        | 'Ĭ' | 'Į' | 'İ' => 'i',
        'ñ' | 'ń' | 'ņ' | 'ň' | 'Ñ' | 'Ń' | 'Ņ' | 'Ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø'
        | 'Ō' | 'Ŏ' | 'Ő' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ũ'
        | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => 'u',
        'ý' | 'ÿ' | 'ŷ' | 'Ý' | 'Ÿ' | 'Ŷ' => 'y',
        'ž' | 'ź' | 'ż' | 'Ž' | 'Ź' | 'Ż' => 'z',
        'š' | 'ś' | 'ŝ' | 'ş' | 'Š' | 'Ś' | 'Ŝ' | 'Ş' => 's',
        other => other.to_ascii_lowercase(),
    }
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036f}')
}

fn fold_search(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if is_combining_mark(ch) {
            continue;
        }
        for lc in fold_char(ch).to_lowercase() {
            out.push(lc);
        }
    }
    out
}

fn find_highlight_span(text: &str, query: &str) -> Option<(usize, usize)> {
    let q = fold_search(query);
    if q.is_empty() {
        return None;
    }
    let mut folded = String::with_capacity(text.len());
    let mut folded_to_orig: Vec<usize> = Vec::with_capacity(text.len());
    for (byte_idx, ch) in text.char_indices() {
        if is_combining_mark(ch) {
            continue;
        }
        for lc in fold_char(ch).to_lowercase() {
            let start = folded.len();
            folded.push(lc);
            while folded_to_orig.len() < folded.len() {
                folded_to_orig.push(byte_idx);
            }
            let _ = start;
        }
    }
    let match_start = folded.find(&q)?;
    let match_end = match_start + q.len();
    let orig_start = *folded_to_orig.get(match_start)?;
    let orig_end = if match_end >= folded_to_orig.len() {
        text.len()
    } else {
        folded_to_orig[match_end]
    };
    if orig_start <= orig_end && orig_end <= text.len() {
        Some((orig_start, orig_end))
    } else {
        None
    }
}

fn compute_list(raw_groups: &[CommandGroup], trimmed_query: &str, row_h: Pixels) -> ComputedList {
    let fold_q = fold_search(trimmed_query);
    let mut groups: Vec<RenderedGroup> = Vec::new();
    let mut flat: Vec<FlatCommand> = Vec::new();
    let mut suggestions: Vec<FlatCommand> = Vec::new();
    let mut cursor_y = LIST_PAD_TOP;

    if !fold_q.is_empty() {
        let ask_item = CommandItem::new(
            "ask-about",
            format!("Ask assistant about “{trimmed_query}”"),
        )
        .logo(CommandLogo::AskSpark);
        let cmd = FlatCommand {
            key: "ask".into(),
            is_ask: true,
            suggestion_n: None,
            group_idx: 0,
            first_in_group: true,
            group_has_label: false,
            item: ask_item,
            y: cursor_y,
            h: row_h,
        };
        cursor_y += row_h;
        flat.push(cmd.clone());
        groups.push(RenderedGroup {
            key: "ask".into(),
            label: None,
            commands: vec![cmd],
        });
    }

    for (g_idx, group) in raw_groups.iter().enumerate() {
        let mut matched: Vec<&CommandItem> = Vec::new();
        for item in &group.commands {
            if fold_q.is_empty() {
                matched.push(item);
                continue;
            }
            let hay = format!(
                "{} {} {}",
                item.label,
                item.hint.as_deref().unwrap_or(""),
                item.keywords.as_deref().unwrap_or("")
            );
            if fold_search(&hay).contains(&fold_q) {
                matched.push(item);
            }
        }
        if matched.is_empty() {
            continue;
        }

        let r_group_idx = groups.len();
        let label = (!group.label.is_empty()).then(|| group.label.clone());
        let has_label = label.is_some();
        if has_label {
            cursor_y += GROUP_LABEL_HEIGHT;
        }

        let mut g_cmds = Vec::with_capacity(matched.len());
        for (i_idx, item) in matched.into_iter().enumerate() {
            let suggestion_n =
                (group.id == "suggestions" && i_idx < 5).then_some((i_idx + 1) as u8);
            let cmd = FlatCommand {
                key: format!("{g_idx}-{}-{i_idx}", item.id).into(),
                is_ask: false,
                suggestion_n,
                group_idx: r_group_idx,
                first_in_group: i_idx == 0,
                group_has_label: has_label,
                item: (*item).clone(),
                y: cursor_y,
                h: row_h,
            };
            cursor_y += row_h;
            if suggestion_n.is_some() {
                suggestions.push(cmd.clone());
            }
            flat.push(cmd.clone());
            g_cmds.push(cmd);
        }

        groups.push(RenderedGroup {
            key: format!("g-{g_idx}-{}", group.id).into(),
            label,
            commands: g_cmds,
        });
    }

    ComputedList {
        groups,
        flat,
        suggestions,
        content_height: cursor_y + LIST_PAD_BOTTOM,
    }
}

fn highlighted_text_el(text: &str, query: &str, color: Hsla, mark_bg: Hsla) -> AnyElement {
    if let Some((start, end)) = find_highlight_span(text, query) {
        let before = SharedString::from(text[..start].to_owned());
        let hit = SharedString::from(text[start..end].to_owned());
        let after = SharedString::from(text[end..].to_owned());
        return div()
            .flex()
            .items_baseline()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_color(color)
            .when(!before.is_empty(), |d| d.child(before))
            .child(div().flex_none().rounded(px(3.)).bg(mark_bg).child(hit))
            .when(!after.is_empty(), |d| d.child(after))
            .into_any_element();
    }

    div()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_color(color)
        .child(SharedString::from(text.to_owned()))
        .into_any_element()
}

fn kbd_chip(key: &str, look: &Look) -> AnyElement {
    let radius = look.kbd_radius;
    let bg = look.kbd;
    let line = look.line;
    div()
        .relative()
        .flex_none()
        .min_w(KBD_MIN_WIDTH)
        .h(KBD_HEIGHT)
        .px(KBD_PAD_X)
        .flex()
        .items_center()
        .justify_center()
        .text_size(KBD_FONT_SIZE)
        .line_height(KBD_HEIGHT)
        .font_weight(FontWeight::MEDIUM)
        .text_color(look.muted)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let bevel =
                        Bounds::new(point(bounds.left(), bounds.top() + px(1.)), bounds.size);
                    window.paint_quad(fill(bevel, line).corner_radii(Corners::all(radius)));
                    window.paint_quad(gpui_kit::quad(
                        bounds,
                        Corners::all(radius),
                        bg,
                        gpui_kit::Edges::all(px(1.)),
                        line,
                        gpui_kit::BorderStyle::Solid,
                    ));
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
        .child(SharedString::from(key.to_owned()))
        .into_any_element()
}

fn render_leading(item: &CommandItem, selected: bool, look: &Look) -> AnyElement {
    match &item.leading {
        Some(CommandLeading::Logo(logo)) => div()
            .size(TILE_SIZE)
            .flex_none()
            .rounded(TILE_RADIUS)
            .bg(look.tile)
            .border_1()
            .border_color(look.tile_ring)
            .flex()
            .items_center()
            .justify_center()
            .child(LogoGlyph::new(*logo, look.ink))
            .into_any_element(),
        Some(CommandLeading::TileSvg(path)) => div()
            .size(TILE_SIZE)
            .flex_none()
            .rounded(TILE_RADIUS)
            .bg(look.tile)
            .border_1()
            .border_color(look.tile_ring)
            .flex()
            .items_center()
            .justify_center()
            .child(svg().path(path.clone()).size(px(14.)).text_color(look.ink))
            .into_any_element(),
        Some(CommandLeading::Icon(icon)) => {
            let color = if selected { look.ink } else { look.muted };
            div()
                .size(TILE_SIZE)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icon.0.clone())
                        .size(ICON_GLYPH_SIZE)
                        .text_color(color),
                )
                .into_any_element()
        }
        None => {
            let letter = item
                .label
                .chars()
                .find(|c| c.is_alphanumeric())
                .map(|c| c.to_ascii_uppercase())
                .unwrap_or('·');
            div()
                .size(TILE_SIZE)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(look.text)
                .child(letter.to_string())
                .into_any_element()
        }
    }
}

impl RenderOnce for CommandMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let look = Look::of(&theme);
        let reduce_motion = cx.reduce_motion();
        let platform = self.platform.unwrap_or(if cfg!(target_os = "macos") {
            KbdPlatform::Mac
        } else {
            KbdPlatform::Other
        });
        let mod_key: &'static str = match platform {
            KbdPlatform::Mac => "⌘",
            KbdPlatform::Other => "Ctrl",
        };
        let root_w = self.width.unwrap_or(MAX_WIDTH).min(MAX_WIDTH);
        let narrow = self.width.is_some_and(|w| w <= NARROW_BREAKPOINT);
        let row_h = if narrow {
            ITEM_HEIGHT_NARROW
        } else {
            ITEM_HEIGHT
        };
        let max_list_h = if narrow {
            LIST_MAX_HEIGHT_NARROW
        } else {
            LIST_MAX_HEIGHT
        };

        let init_q = self.query.clone().or_else(|| self.default_query.clone());
        let ph = self.placeholder.clone();
        let controller = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            CommandMenuController::new(ph.clone(), init_q, window, cx)
        });

        let sync = CommandFrameSync {
            placeholder: self.placeholder.clone(),
            controlled_query: self.query.clone(),
            auto_focus: self.auto_focus,
            on_run: self.on_run.clone(),
            on_close: self.on_close.clone(),
            on_query_change: self.on_query_change.clone(),
        };
        controller.clone().update(cx, |s, cx| {
            s.sync_frame(sync, window, cx);
        });

        let raw_query = controller.read(cx).input.read(cx).value();
        let trimmed_query = raw_query.trim().to_owned();
        let raw_groups = self.groups.unwrap_or_else(default_command_groups);
        let computed = compute_list(&raw_groups, &trimmed_query, row_h);

        let selected = self
            .selected_key
            .clone()
            .and_then(|wanted| {
                computed
                    .flat
                    .iter()
                    .find(|c| c.key == wanted || c.item.id == wanted)
                    .cloned()
            })
            .or_else(|| {
                let stored = controller.read(cx).active_key.clone();
                stored.and_then(|k| computed.flat.iter().find(|c| c.key == k).cloned())
            })
            .or_else(|| computed.flat.first().cloned());

        let selected_key = selected.as_ref().map(|c| c.key.clone());

        let target_list_h = computed.content_height.min(max_list_h);
        let did_init = controller.read(cx).initialized;
        controller.update(cx, |s, _| {
            s.initialized = true;
            s.frame_flat = computed.flat.clone();
            s.frame_groups = computed.groups.clone();
            s.frame_suggestions = computed.suggestions.clone();
            s.frame_selected = selected_key.clone();
            s.frame_trimmed_query = trimmed_query.clone();
        });
        if !self.inline && self.freeze_at.is_none() {
            CommandKeyRegistry::register(window, &controller, cx);
        }

        let anim_list_h = if self.freeze_at.is_some() || reduce_motion || !did_init {
            target_list_h
        } else {
            transition(
                (self.id.clone(), "cm-list-h"),
                target_list_h.as_f32(),
                Transition::new(LIST_HEIGHT_DURATION).easing(theme.ease.clone()),
                window,
                cx,
            )
            .pipe(px)
        };

        let (pill_y, pill_h) = match &selected {
            Some(sel) => {
                if self.freeze_at.is_some() || reduce_motion {
                    (Some(sel.y), sel.h)
                } else {
                    let y = transition(
                        (self.id.clone(), "cm-pill-y"),
                        sel.y.as_f32(),
                        Transition::new(PILL_DURATION).easing(theme.ease.clone()),
                        window,
                        cx,
                    );
                    let h = transition(
                        (self.id.clone(), "cm-pill-h"),
                        sel.h.as_f32(),
                        Transition::new(PILL_DURATION).easing(theme.ease.clone()),
                        window,
                        cx,
                    );
                    (Some(px(y)), px(h))
                }
            }
            None => (None, px(0.)),
        };

        let active_ran: Option<(SharedString, SharedString, f32)> =
            if let Some((rk, rl)) = self.ran_preview.clone() {
                Some((rk, rl, 1.0))
            } else {
                controller.read(cx).ran.as_ref().map(|r| {
                    let elapsed = r.since.elapsed();
                    let in_p = if reduce_motion {
                        1.0
                    } else {
                        (elapsed.as_secs_f32() / DONE_DURATION.as_secs_f32()).clamp(0., 1.)
                    };
                    if in_p < 1.0 {
                        window.request_animation_frame();
                    }
                    (r.key.clone(), r.label.clone(), in_p)
                })
            };

        let pressed_ran_key: Option<(SharedString, f32)> =
            controller.read(cx).ran.as_ref().and_then(|r| {
                if self.freeze_at.is_some() || reduce_motion {
                    return None;
                }
                let elapsed = r.since.elapsed();
                if elapsed >= RAN_DURATION {
                    return None;
                }
                window.request_animation_frame();
                let t = elapsed.as_secs_f32() / RAN_DURATION.as_secs_f32();
                // CSS: 0% -> 1, 30% -> 0.985, 100% -> 1
                let scale = if t < 0.3 {
                    1.0 - 0.015 * (t / 0.3)
                } else {
                    0.985 + 0.015 * ((t - 0.3) / 0.7)
                };
                Some((r.key.clone(), scale))
            });

        let input_entity = controller.read(cx).input.clone();
        let scroll_handle = controller.read(cx).scroll.clone();

        // Card drop shadow painted in-place via canvas behind card content.
        let card_shadow_layer = {
            let shadows = look.card_shadows.clone();
            let radius = look.card_radius;
            let bg = look.card;
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    paint::drop_shadows(bounds, radius, &shadows, window);
                    window.paint_quad(fill(bounds, bg).corner_radii(Corners::all(radius)));
                },
            )
            .absolute()
            .inset_0()
            .size_full()
        };

        let pill_layer = pill_y.map(|py| {
            let color = look.active;
            let radius = look.pill_radius;
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let rect = gpui_kit::Bounds::new(
                        point(bounds.left() + LIST_PAD_X, bounds.top() + py),
                        gpui_kit::size((bounds.size.width - LIST_PAD_X * 2.).max(px(0.)), pill_h),
                    );
                    window.paint_quad(fill(rect, color).corner_radii(Corners::all(radius)));
                },
            )
            .absolute()
            .inset_0()
            .size_full()
        });

        let search_row = {
            let focus_on_click = input_entity.clone();
            let search_el = Input::new(&input_entity);
            let mut row = div()
                .id((self.id.clone(), "cm-search"))
                .h(SEARCH_HEIGHT)
                .px(SEARCH_PAD_X)
                .gap(SEARCH_GAP)
                .border_b_1()
                .border_color(look.line)
                .flex()
                .items_center()
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    focus_on_click.update(cx, |s, cx| s.focus(window, cx));
                })
                .child(
                    svg()
                        .path("icons/search.svg")
                        .size(SEARCH_ICON_SIZE)
                        .flex_none()
                        .text_color(look.muted),
                )
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .text_size(SEARCH_FONT_SIZE)
                        .text_color(look.ink)
                        .when(raw_query.is_empty(), |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .flex()
                                    .items_center()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_size(SEARCH_FONT_SIZE)
                                    .text_color(look.faint)
                                    .child(self.placeholder.clone()),
                            )
                        })
                        .child(search_el),
                );
            if !narrow {
                let enter_label = paint::tracked_text_el(
                    "ENTER",
                    theme.font_family.clone(),
                    ENTER_HINT_FONT_SIZE,
                    px(16.),
                    FontWeight::MEDIUM,
                    ENTER_HINT_FONT_SIZE * 0.04,
                    look.faint,
                    window,
                );
                row = row.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(ENTER_HINT_GAP)
                        .child(
                            svg()
                                .path("icons/corner-down-left.svg")
                                .size(ENTER_HINT_ICON_SIZE)
                                .text_color(look.faint),
                        )
                        .child(enter_label),
                );
            }
            row
        };

        let mut list_inner = div()
            .relative()
            .w_full()
            .pt(LIST_PAD_TOP)
            .px(LIST_PAD_X)
            .pb(LIST_PAD_BOTTOM)
            .flex()
            .flex_col()
            .when_some(pill_layer, |d, pill| d.child(pill));

        for group in &computed.groups {
            let mut g_div = div()
                .id((self.id.clone(), group.key.clone()))
                .w_full()
                .flex()
                .flex_col();
            if group.key != "ask"
                && let Some(label) = &group.label
            {
                g_div = g_div.child(
                    div()
                        .pt(GROUP_LABEL_PAD_TOP)
                        .px(GROUP_LABEL_PAD_X)
                        .pb(GROUP_LABEL_PAD_BOTTOM)
                        .text_size(GROUP_LABEL_FONT_SIZE)
                        .line_height(GROUP_LABEL_LINE_HEIGHT)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(look.faint)
                        .child(label.clone()),
                );
            }

            for entry in &group.commands {
                let is_selected = selected_key.as_ref() == Some(&entry.key);
                let ctrl_move = controller.clone();
                let ctrl_click = controller.clone();
                let move_key = entry.key.clone();
                let click_entry = entry.clone();
                let click_q = trimmed_query.clone();
                let click_focus = input_entity.clone();

                let keys_vec: Vec<SharedString> = if let Some(n) = entry.suggestion_n {
                    vec![mod_key.into(), n.to_string().into()]
                } else {
                    entry.item.keys.clone()
                };

                let mut text_box = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap(TEXT_GAP)
                    .child(highlighted_text_el(
                        &entry.item.label,
                        &trimmed_query,
                        look.ink,
                        look.mark,
                    ));

                if !narrow && let Some(hint) = &entry.item.hint {
                    text_box = text_box.child(highlighted_text_el(
                        hint,
                        &trimmed_query,
                        look.muted,
                        look.mark,
                    ));
                }

                let row_scale = pressed_ran_key
                    .as_ref()
                    .and_then(|(rk, s)| (*rk == entry.key).then_some(*s));

                let mut item_btn = div()
                    .id((self.id.clone(), entry.key.clone()))
                    .role(Role::ListItem)
                    .relative()
                    .w_full()
                    .h(row_h)
                    .px(ITEM_PAD_X)
                    .rounded(look.pill_radius)
                    .flex()
                    .items_center()
                    .gap(ITEM_GAP)
                    .text_size(ITEM_FONT_SIZE)
                    .line_height(ITEM_LINE_HEIGHT)
                    .cursor_pointer()
                    .when_some(row_scale, |d, sc| d.px(ITEM_PAD_X + px((1. - sc) * 40.)))
                    .on_mouse_move(move |_, _, cx| {
                        ctrl_move.update(cx, |s, cx| {
                            if s.active_key.as_ref() != Some(&move_key) {
                                s.active_key = Some(move_key.clone());
                                cx.notify();
                            }
                        });
                    })
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.prevent_default();
                    })
                    .on_click(move |_, window, cx| {
                        ctrl_click.update(cx, |s, cx| {
                            s.run_entry(&click_entry, &click_q, window, cx);
                        });
                        click_focus.update(cx, |s, cx| s.focus(window, cx));
                    })
                    .child(render_leading(&entry.item, is_selected, &look))
                    .child(text_box);

                if !narrow && !keys_vec.is_empty() {
                    let opacity = if is_selected { 1.0 } else { 0.85 };
                    let mut keys_div = div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(KEYS_GAP)
                        .opacity(opacity);
                    for k in &keys_vec {
                        keys_div = keys_div.child(kbd_chip(k, &look));
                    }
                    item_btn = item_btn.child(keys_div);
                }

                g_div = g_div.child(item_btn);
            }

            list_inner = list_inner.child(g_div);
        }

        let list_box = div()
            .id((self.id.clone(), "cm-list"))
            .role(Role::List)
            .aria_label("Commands")
            .track_scroll(&scroll_handle)
            .w_full()
            .h(anim_list_h)
            .max_h(max_list_h)
            .overflow_y_scroll()
            .child(list_inner);

        let card = div()
            .relative()
            .w_full()
            .rounded(look.card_radius)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(card_shadow_layer)
            .child(search_row)
            .child(list_box);

        let footer_left: AnyElement = if let Some((_, label, in_p)) = active_ran {
            let drift_y = px(3. * (1. - in_p));
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(DONE_GAP)
                .opacity(in_p)
                .pt(drift_y)
                .text_color(look.ink)
                .child(
                    svg()
                        .path("icons/check.svg")
                        .size(DONE_ICON_SIZE)
                        .flex_none()
                        .text_color(look.ok),
                )
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(label),
                )
                .into_any_element()
        } else if !narrow {
            div()
                .flex()
                .items_center()
                .gap(DONE_GAP)
                .child(kbd_chip("Tab", &look))
                .child("to jump between sections")
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let mut footer = div()
            .h(FOOT_HEIGHT)
            .px(FOOT_PAD_X)
            .gap(FOOT_GAP)
            .flex()
            .items_center()
            .justify_between()
            .text_size(FOOT_FONT_SIZE)
            .text_color(look.muted)
            .child(footer_left);

        if !narrow {
            footer = footer.child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(DONE_GAP)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(KEYS_GAP)
                            .child(kbd_chip(mod_key, &look))
                            .child(kbd_chip("K", &look)),
                    )
                    .child("to open"),
            );
        }

        let root = div()
            .id(self.id)
            .role(Role::Group)
            .font_family(theme.font_family.clone())
            .w(root_w)
            .max_w(MAX_WIDTH)
            .p(TRAY_PAD)
            .rounded(look.tray_radius)
            .bg(look.tray)
            .flex()
            .flex_col()
            .text_size(ITEM_FONT_SIZE)
            .text_color(look.text)
            .child(card)
            .child(footer);

        let _ = root_w;
        #[cfg(test)]
        let root = {
            use gpui_kit::TestSupportExt as _;
            root.test_support()
        };
        root
    }
}

trait PipeExt: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}

impl PipeExt for f32 {}
