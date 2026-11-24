//! Per-button state kept across frames: pointer state and focus.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::FocusHandle;

struct Inner {
    focus: FocusHandle,
    hovered: bool,
    pressed: bool,
}

#[derive(Clone)]
pub(super) struct Shared(Rc<RefCell<Inner>>);

impl Shared {
    pub fn new(focus: FocusHandle) -> Self {
        Self(Rc::new(RefCell::new(Inner {
            focus,
            hovered: false,
            pressed: false,
        })))
    }

    pub fn focus(&self) -> FocusHandle {
        self.0.borrow().focus.clone()
    }

    pub fn hovered(&self) -> bool {
        self.0.borrow().hovered
    }

    pub fn pressed(&self) -> bool {
        self.0.borrow().pressed
    }

    pub fn set_hovered(&self, hovered: bool) {
        self.0.borrow_mut().hovered = hovered;
    }

    /// Returns whether the value changed.
    pub fn set_pressed(&self, pressed: bool) -> bool {
        let mut inner = self.0.borrow_mut();
        let changed = inner.pressed != pressed;
        inner.pressed = pressed;
        changed
    }
}
