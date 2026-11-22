//! Per-button state kept across frames: pointer state, focus, and the clocks
//! of the content enter animations and the loading state.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::TransitionState;
use gpui_kit::{FocusHandle, Pixels, SharedString};

/// Interactive states a gallery can pin, see [`super::Button::force_state`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ButtonState {
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
}

/// The parts that replay their enter animation when their key changes, as
/// React remounts a part whose `key` changes.
pub(super) const LABEL: usize = 0;
pub(super) const ICON: usize = 1;
pub(super) const ICON_RIGHT: usize = 2;

/// The content keys of one render: label text, left icon (or the spinner),
/// right icon.
pub(super) type Keys = [Option<SharedString>; 3];

struct Part {
    key: SharedString,
    since: Instant,
}

struct Inner {
    focus: FocusHandle,
    hovered: bool,
    pressed: bool,
    parts: [Option<Part>; 3],
    loading_since: Option<Instant>,
    /// The content keys and width target of the previous render.
    last: Option<(Keys, Pixels)>,
}

#[derive(Clone)]
pub(super) struct Shared(Rc<RefCell<Inner>>, TransitionState<Pixels>);

impl Shared {
    pub fn new(focus: FocusHandle) -> Self {
        Self(
            Rc::new(RefCell::new(Inner {
                focus,
                hovered: false,
                pressed: false,
                parts: [None, None, None],
                loading_since: None,
                last: None,
            })),
            TransitionState::default(),
        )
    }

    /// The width, gliding or snapping to `target`.
    pub fn width(&self) -> &TransitionState<Pixels> {
        &self.1
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

    /// Records this render's content. Returns when each part started
    /// entering, how long the button has been loading, and whether the width
    /// target changed without a content change (which riff-uiux does not
    /// animate).
    pub fn update(&self, keys: Keys, width: Pixels, loading: bool, now: Instant) -> Frame {
        let mut inner = self.0.borrow_mut();
        let mut since = [None; 3];
        for (slot, key) in inner.parts.iter_mut().zip(keys.iter()) {
            match key {
                None => *slot = None,
                Some(key) => {
                    if slot.as_ref().is_none_or(|part| &part.key != key) {
                        *slot = Some(Part {
                            key: key.clone(),
                            since: now,
                        });
                    }
                }
            }
        }
        for (i, part) in inner.parts.iter().enumerate() {
            since[i] = part
                .as_ref()
                .map(|part| now.saturating_duration_since(part.since));
        }

        if loading {
            inner.loading_since.get_or_insert(now);
        } else {
            inner.loading_since = None;
        }
        let loading_for = inner
            .loading_since
            .map(|start| now.saturating_duration_since(start));

        // riff-uiux glides when the content changes; a width change with the
        // same content (another font, say) is instant. A re-render with the
        // same target must not snap, or it would cut a running glide short.
        let snap_width = match &inner.last {
            Some((last_keys, last_width)) => *last_keys == keys && *last_width != width,
            None => true,
        };
        inner.last = Some((keys, width));

        Frame {
            part_elapsed: since,
            loading_for,
            snap_width,
        }
    }
}

/// What [`Shared::update`] found.
pub(super) struct Frame {
    /// Time since each part (label, icon, right icon) started entering.
    pub part_elapsed: [Option<Duration>; 3],
    pub loading_for: Option<Duration>,
    /// Adopt the new width at once instead of gliding to it.
    pub snap_width: bool,
}
