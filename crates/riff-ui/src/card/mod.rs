//! riff-uiux `Card`: a white card resting on a soft gray tray, with header
//! (icon, title, description, action), content, and a tray-mounted footer,
//! supporting whole-card interactive targets and selected states.

mod style;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::base::{self, Easing, Transition, transition};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyElement, App, BorderStyle, ClickEvent, Corners, CursorStyle, Div, Edges, ElementId,
    FocusHandle, FontWeight, Hsla, IntoElement, ParentElement, Refineable, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, quad,
};

use crate::a11y::A11yLabel;
pub use crate::button::ButtonIcon as CardIcon;
use crate::paint::{self, Scale};
use crate::theme::RiffTheme;
use style::{
    ACTION_GAP, ACTION_HEIGHT, BODY_SIZE, CARD_GAP, FOOT_GAP, FOOT_LINE_HEIGHT, FOOT_MIN_HEIGHT,
    FOOT_PAD_RIGHT, FOOT_SIZE, HEAD_GAP, ICON_MIN, ICON_SIZE, LINE_HEIGHT, Look, PAD,
    STATE_DURATION, TITLE_SIZE, TITLES_GAP, TRAY_PAD,
};

/// Semantic container kind for [`Card`], corresponding to `as` in riff-uiux.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum CardAs {
    #[default]
    Div,
    Section,
    Article,
    ListItem,
}

impl CardAs {
    fn role(self) -> Option<Role> {
        match self {
            Self::Div => None,
            Self::Section => Some(Role::Region),
            Self::Article => Some(Role::Article),
            Self::ListItem => Some(Role::ListItem),
        }
    }
}

/// Forced interactive states for galleries and screenshots.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct CardState {
    pub hovered: bool,
    pub focus_visible: bool,
}

#[derive(Clone)]
struct Shared {
    focus: FocusHandle,
    hovered: Rc<Cell<bool>>,
}

impl Shared {
    fn new(focus: FocusHandle) -> Self {
        Self {
            focus,
            hovered: Rc::new(Cell::new(false)),
        }
    }

    fn focus(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn hovered(&self) -> bool {
        self.hovered.get()
    }

    fn set_hovered(&self, hovered: bool) {
        self.hovered.set(hovered);
    }
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

enum HeaderIcon {
    Element(AnyElement),
    Svg(CardIcon),
}

/// Header of a [`Card`]: leading icon, title & description, trailing action,
/// and optional extra elements beneath the description.
#[derive(IntoElement)]
pub struct CardHeader {
    title: Option<CardText>,
    description: Option<CardText>,
    icon: Option<HeaderIcon>,
    action: Option<AnyElement>,
    extra: Vec<AnyElement>,
}

enum CardText {
    Text(SharedString),
    Element(AnyElement),
}

impl Default for CardHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl CardHeader {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            icon: None,
            action: None,
            extra: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.title = (!title.is_empty()).then_some(CardText::Text(title));
        self
    }

    pub fn title_element(mut self, title: impl IntoElement) -> Self {
        self.title = Some(CardText::Element(title.into_any_element()));
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let description = description.into();
        self.description = (!description.is_empty()).then_some(CardText::Text(description));
        self
    }

    pub fn description_element(mut self, description: impl IntoElement) -> Self {
        self.description = Some(CardText::Element(description.into_any_element()));
        self
    }

    /// Leading element in the `20×20` icon slot (such as an [`Avatar`] or SVG).
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(HeaderIcon::Element(icon.into_any_element()));
        self
    }

    /// Convenience for a 16px Lucide SVG icon in the leading slot.
    pub fn svg_icon(mut self, icon: impl Into<CardIcon>) -> Self {
        self.icon = Some(HeaderIcon::Svg(icon.into()));
        self
    }

    /// Trailing element aligned to the 20px title line (e.g. an `IconButton` or `Badge`).
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl ParentElement for CardHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.extra.extend(elements);
    }
}

impl RenderOnce for CardHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let look = Look::of(theme);
        let mut titles = div().flex_1().flex().flex_col().gap(TITLES_GAP).min_w_0();
        if let Some(title) = self.title {
            let title_box = div()
                .min_w_0()
                .text_size(TITLE_SIZE)
                .line_height(LINE_HEIGHT)
                .font_weight(FontWeight::MEDIUM)
                .text_color(look.strong);
            titles = titles.child(match title {
                CardText::Text(text) => title_box.child(text),
                CardText::Element(el) => title_box.child(el),
            });
        }
        if let Some(desc) = self.description {
            let desc_box = div()
                .min_w_0()
                .text_size(BODY_SIZE)
                .line_height(LINE_HEIGHT)
                .text_color(look.muted);
            titles = titles.child(match desc {
                CardText::Text(text) => desc_box.child(text),
                CardText::Element(el) => desc_box.child(el),
            });
        }
        titles = titles.children(self.extra);

        div()
            .flex()
            .items_start()
            .gap(HEAD_GAP)
            .min_w_0()
            .when_some(self.icon, |this, icon| {
                let icon_el = match icon {
                    HeaderIcon::Element(el) => el,
                    HeaderIcon::Svg(svg_icon) => gpui_kit::svg()
                        .path(svg_icon.0)
                        .size(ICON_SIZE)
                        .flex_none()
                        .text_color(look.muted)
                        .into_any_element(),
                };
                this.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .min_w(ICON_MIN)
                        .min_h(ICON_MIN)
                        .text_color(look.muted)
                        .child(icon_el),
                )
            })
            .child(titles)
            .when_some(self.action, |this, action| {
                this.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(ACTION_GAP)
                        .h(ACTION_HEIGHT)
                        .child(action),
                )
            })
    }
}

/// Body region inside a [`Card`].
#[derive(IntoElement)]
pub struct CardContent {
    base: Div,
}

impl Default for CardContent {
    fn default() -> Self {
        Self::new()
    }
}

impl CardContent {
    pub fn new() -> Self {
        Self { base: div() }
    }
}

impl ParentElement for CardContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for CardContent {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CardContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.min_w_0()
    }
}

/// Footer row rendered on the gray tray beneath the white card surface.
#[derive(IntoElement)]
pub struct CardFooter {
    base: Div,
    has_main: bool,
}

impl Default for CardFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl CardFooter {
    pub fn new() -> Self {
        Self {
            base: div(),
            has_main: true,
        }
    }
}

impl ParentElement for CardFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for CardFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CardFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let look = Look::of(RiffTheme::global(cx));
        self.base
            .flex()
            .flex_wrap()
            .items_center()
            .gap(FOOT_GAP)
            .min_w_0()
            .min_h(FOOT_MIN_HEIGHT)
            .when(self.has_main, |this| this.pt(TRAY_PAD))
            .pl(PAD)
            .pr(FOOT_PAD_RIGHT)
            .text_size(FOOT_SIZE)
            .line_height(FOOT_LINE_HEIGHT)
            .text_color(look.faint)
    }
}

/// A riff-uiux card: header, content, and optional tray footer.
///
/// ```ignore
/// Card::new()
///     .header(
///         CardHeader::new()
///             .title("Support triage")
///             .description("Sorts new tickets and drafts the first reply"),
///     )
///     .content("142 tickets this week · 96% resolved")
///     .footer(CardFooter::new().child("Running · started 2 min ago"));
/// ```
#[derive(IntoElement)]
pub struct Card {
    id: Option<ElementId>,
    as_el: CardAs,
    interactive: bool,
    selected: bool,
    pressed: Option<bool>,
    header: Option<CardHeader>,
    children: Vec<AnyElement>,
    footer: Option<CardFooter>,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    on_click: Option<ClickHandler>,
    forced: CardState,
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl Card {
    pub fn new() -> Self {
        Self {
            id: None,
            as_el: CardAs::default(),
            interactive: false,
            selected: false,
            pressed: None,
            header: None,
            children: Vec::new(),
            footer: None,
            style: StyleRefinement::default(),
            accessibility_label: None,
            on_click: None,
            forced: CardState::default(),
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn as_element(mut self, as_el: CardAs) -> Self {
        self.as_el = as_el;
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Marks the interactive card as a toggle (`aria-pressed`), e.g. in a picker list.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn header(mut self, header: CardHeader) -> Self {
        self.header = Some(header);
        self
    }

    /// Appends a [`CardContent`] (or plain text/element) inside the white card.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.children
            .push(CardContent::new().child(content).into_any_element());
        self
    }

    /// Places a [`CardFooter`] on the gray tray beneath the white card.
    pub fn footer(mut self, footer: CardFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Forces hover or focus ring for galleries and screenshots.
    pub fn force_state(mut self, state: CardState) -> Self {
        self.forced = state;
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Card {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let look = Look::of(&theme);
        let reduce_motion = cx.reduce_motion();
        let card_radius = theme.radius_lg;
        let tray_radius = card_radius + TRAY_PAD;

        let shared = window.use_state(cx, |_, cx| Shared::new(cx.focus_handle()));
        let state_id = self
            .id
            .clone()
            .unwrap_or_else(|| ElementId::Integer(shared.entity_id().as_u64()));
        let shared_val = shared.read(cx).clone();
        let focus = shared_val.focus();

        let hovered = self.interactive && (shared_val.hovered() || self.forced.hovered);
        let focus_visible = self.interactive
            && (self.forced.focus_visible
                || (focus.is_focused(window) && window.last_input_was_keyboard()));

        let ease = if reduce_motion {
            Transition::new(std::time::Duration::ZERO)
        } else {
            Transition::new(STATE_DURATION).easing(Easing::Ease)
        };
        let hover_t = transition(
            (state_id.clone(), "riff-crd-hover"),
            if hovered { 1. } else { 0. },
            ease.clone(),
            window,
            cx,
        );
        let selected_t = transition(
            (state_id.clone(), "riff-crd-selected"),
            if self.selected { 1. } else { 0. },
            ease,
            window,
            cx,
        );

        let tray_bg = paint::mix(look.tray, look.tray_selected, selected_t);
        let resting_shadows = look.lerp_shadows(&look.shadow, &look.shadow_selected, selected_t);
        let card_shadows = look.lerp_shadows(&resting_shadows, &look.shadow_hover, hover_t);

        let surface = look.surface;
        let has_main = self.header.is_some() || !self.children.is_empty();
        let inner_card = has_main.then(|| {
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(CARD_GAP)
                .min_w_0()
                .p(PAD)
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| {
                            paint::drop_shadows(bounds, card_radius, &card_shadows, window);
                            window.paint_quad(quad(
                                bounds,
                                Corners::all(card_radius),
                                surface,
                                Edges::all(gpui_kit::px(0.)),
                                Hsla::transparent_black(),
                                BorderStyle::Solid,
                            ));
                        },
                    )
                    .absolute()
                    .inset_0()
                    .size_full(),
                )
                .when_some(self.header, |this, header| this.child(header))
                .children(self.children)
        });

        let footer = self.footer.map(|mut f| {
            f.has_main = has_main;
            f
        });

        let trigger = if self.interactive {
            let hover_state = shared_val.clone();
            let cursor = CursorStyle::PointingHand;
            let on_click = self.on_click.clone();
            let a11y_label = self.accessibility_label.clone();

            let el = match self.pressed {
                Some(pressed) => {
                    let mut toggle = base::Toggle::new((state_id.clone(), "riff-crd-trigger"))
                        .pressed(pressed)
                        .track_focus(&focus)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .rounded(tray_radius)
                        .cursor(cursor)
                        .on_hover(move |is_hovered, window, _| {
                            hover_state.set_hovered(*is_hovered);
                            window.refresh();
                        });
                    if let Some(label) = a11y_label {
                        toggle = toggle.accessibility_label(label);
                    }
                    if let Some(on_click) = on_click {
                        toggle = toggle.on_change(move |_, event, window, cx| {
                            on_click(event, window, cx);
                        });
                    }
                    toggle.into_any_element()
                }
                None => {
                    let mut button = base::Button::new((state_id, "riff-crd-trigger"))
                        .track_focus(&focus)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .rounded(tray_radius)
                        .cursor(cursor)
                        .on_hover(move |is_hovered, window, _| {
                            hover_state.set_hovered(*is_hovered);
                            window.refresh();
                        });
                    if let Some(label) = a11y_label {
                        button = button.accessibility_label(label);
                    }
                    if let Some(on_click) = on_click {
                        button = button.on_click(move |event, window, cx| {
                            on_click(event, window, cx);
                        });
                    }
                    button.into_any_element()
                }
            };
            Some(el)
        } else {
            None
        };

        let focus_color = look.focus;
        let focus_ring = focus_visible.then(|| {
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let scale = Scale {
                        center: bounds.center(),
                        factor: 1.,
                    };
                    paint::paint_focus_ring(bounds, tray_radius, focus_color, scale, window);
                },
            )
            .absolute()
            .inset_0()
            .size_full()
        });

        let mut tray = div()
            .relative()
            .flex()
            .flex_col()
            .min_w_0()
            .p(TRAY_PAD)
            .rounded(tray_radius)
            .bg(tray_bg)
            .font_family(theme.font_family)
            .text_size(BODY_SIZE)
            .line_height(LINE_HEIGHT)
            .text_color(look.ink);
        tray.style().refine(&self.style);

        let tray = tray
            .when_some(trigger, |this, t| this.child(t))
            .when_some(inner_card, |this, card| this.child(card))
            .when_some(footer, |this, foot| this.child(foot))
            .when_some(focus_ring, |this, ring| this.child(ring));

        if self.interactive {
            A11yLabel::new(tray, None)
        } else if let Some(role) = self.as_el.role() {
            A11yLabel::with_role(tray, role, self.accessibility_label)
        } else {
            A11yLabel::new(tray, self.accessibility_label)
        }
    }
}
