//! Shared accessibility label wrapper for non-interactive elements.

use gpui_kit::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Role, SharedString, Window, accesskit,
};

/// Attaches an `accesskit` label to a non-interactive element without
/// requiring an `ElementId` or adding a layout node.
pub(crate) struct A11yLabel<E> {
    inner: E,
    role: Role,
    label: Option<SharedString>,
    busy: bool,
}

impl<E> A11yLabel<E> {
    pub fn new(inner: E, label: Option<SharedString>) -> Self {
        Self::with_role(inner, Role::GenericContainer, label)
    }

    pub fn with_role(inner: E, role: Role, label: Option<SharedString>) -> Self {
        Self {
            inner,
            role,
            label,
            busy: false,
        }
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

impl<E: Element> IntoElement for A11yLabel<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for A11yLabel<E> {
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
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner
            .a11y_role()
            .or_else(|| self.label.as_ref().map(|_| self.role))
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if let Some(label) = &self.label {
            node.set_label(label.as_ref());
        }
        if self.busy {
            node.set_busy();
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
