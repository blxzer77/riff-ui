//! riff-uiux `Separator`: a 1px dividing line between groups, horizontal or
//! vertical, with an optional centered text label and screen-reader semantics.

mod style;
#[cfg(test)]
mod tests;

use gpui_kit::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ParentElement, Pixels, RenderOnce, Role, SharedString, Styled, Window,
    accesskit, div, relative,
};

use crate::theme::RiffTheme;
use style::{
    LABEL_FONT_SIZE, LABEL_GAP, LABEL_LINE_HEIGHT, LABEL_MAX_WIDTH_FRACTION, Look, MIN_LINE_WIDTH,
    VERTICAL_HEIGHT, WEIGHT,
};

/// Direction of a [`Separator`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum SeparatorOrientation {
    /// Full-width horizontal hairline (optionally split around a centered label).
    #[default]
    Horizontal,
    /// Centered upright 1px hairline (18px high by default).
    Vertical,
}

/// A riff-uiux divider between groups.
///
/// ```ignore
/// Separator::new();
/// Separator::new().label("or");
/// Separator::new().vertical();
/// Separator::new().label("Earlier").decorative(false);
/// ```
#[derive(Clone, IntoElement)]
pub struct Separator {
    orientation: SeparatorOrientation,
    decorative: bool,
    label: Option<SharedString>,
    length: Option<Pixels>,
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}

impl Separator {
    pub fn new() -> Self {
        Self {
            orientation: SeparatorOrientation::default(),
            decorative: true,
            label: None,
            length: None,
        }
    }

    pub fn orientation(mut self, orientation: SeparatorOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn horizontal(self) -> Self {
        self.orientation(SeparatorOrientation::Horizontal)
    }

    pub fn vertical(self) -> Self {
        self.orientation(SeparatorOrientation::Vertical)
    }

    /// When `true` (default), the separator is purely visual (`role="none"`),
    /// keeping any `label` readable as text. When `false`, screen readers see
    /// a separator with its orientation and optional label.
    pub fn decorative(mut self, decorative: bool) -> Self {
        self.decorative = decorative;
        self
    }

    /// Centered label inside a horizontal separator (e.g. `"or"` or `"Earlier"`).
    /// Ignored when vertical or empty.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.label = (!label.is_empty()).then_some(label);
        self
    }

    /// Overrides the main-axis length: height when vertical (default `18px`),
    /// width when horizontal (default `100%`).
    pub fn length(mut self, length: Pixels) -> Self {
        self.length = Some(length);
        self
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx);
        let look = Look::of(theme);
        let vertical = self.orientation == SeparatorOrientation::Vertical;
        let label = (!vertical).then_some(self.label).flatten();

        let element = if vertical {
            div()
                .flex_none()
                .self_center()
                .w(WEIGHT)
                .h(self.length.unwrap_or(VERTICAL_HEIGHT))
                .bg(look.line)
        } else if let Some(text) = &label {
            // CSS `height: 0; border-top: 1px` aligns on `H / 2` (`9` in `18px`,
            // `18` in `36px`), whereas an odd `1px` box centers at `floor((H-1)/2)`.
            let line = || {
                div()
                    .flex_1()
                    .min_w(MIN_LINE_WIDTH)
                    .h(WEIGHT * 2.)
                    .pt(WEIGHT)
                    .child(div().size_full().bg(look.line))
            };
            let root = div()
                .flex()
                .flex_none()
                .items_center()
                .gap(LABEL_GAP)
                .min_w_0()
                .font_family(theme.font_family.clone())
                .text_size(LABEL_FONT_SIZE)
                .line_height(LABEL_LINE_HEIGHT)
                .text_color(look.text)
                .child(line())
                .child(
                    div()
                        .flex_initial()
                        .min_w_0()
                        .max_w(relative(LABEL_MAX_WIDTH_FRACTION))
                        .text_center()
                        .child(text.clone()),
                )
                .child(line());
            match self.length {
                Some(w) => root.w(w),
                None => root.w_full(),
            }
        } else {
            let root = div().flex_none().h(WEIGHT).bg(look.line);
            match self.length {
                Some(w) => root.w(w),
                None => root.w_full(),
            }
        };

        let a11y_orientation = if vertical {
            accesskit::Orientation::Vertical
        } else {
            accesskit::Orientation::Horizontal
        };
        A11ySeparator {
            inner: element,
            decorative: self.decorative,
            orientation: a11y_orientation,
            label,
        }
    }
}

struct A11ySeparator<E> {
    inner: E,
    decorative: bool,
    orientation: accesskit::Orientation,
    label: Option<SharedString>,
}

impl<E: Element> IntoElement for A11ySeparator<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for A11ySeparator<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector, bounds, layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }

    fn a11y_role(&self) -> Option<Role> {
        (!self.decorative).then_some(Role::Splitter)
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.decorative {
            return;
        }
        if self.orientation == accesskit::Orientation::Vertical {
            node.set_orientation(accesskit::Orientation::Vertical);
        }
        if let Some(label) = &self.label {
            node.set_label(label.as_ref());
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(prepaint, builder);
    }
}
