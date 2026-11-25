//! Shared overlay positioning, directional drift, and nested layer stack for
//! Phase 2 floating surfaces (`tooltip`, `popover`, `hover-card`,
//! `dropdown-menu`, `context-menu`, `dialog`, `command-menu`).

use std::collections::HashMap;

use gpui_kit::{App, Bounds, EntityId, Global, Pixels, Point, Size, Window, WindowId, point, px};

/// Which edge of the trigger a floating surface prefers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum OverlaySide {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl OverlaySide {
    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Enter drift at eased progress `t` in `0.0..=1.0`, moving `distance` px
    /// from the trigger side toward rest at `0, 0`.
    pub(crate) fn enter_drift(self, t: f32, distance: Pixels) -> Point<Pixels> {
        let remain = distance * (1. - t);
        match self {
            Self::Top => point(px(0.), remain),
            Self::Bottom => point(px(0.), -remain),
            Self::Left => point(remain, px(0.)),
            Self::Right => point(-remain, px(0.)),
        }
    }
}

/// How the floating surface aligns along the trigger edge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum OverlayAlign {
    Start,
    #[default]
    Center,
    End,
}

/// Options controlling placement differences across overlay components.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PlaceOptions {
    /// Whether `Start` switches to `End` (or vice versa) along the vertical
    /// edge (`Top`/`Bottom`) when it overflows and the opposite fits.
    pub flip_align: bool,
    /// Whether `Start` switches to `End` (or vice versa) along horizontal
    /// edges (`Left`/`Right`) when it overflows and the opposite fits.
    pub flip_align_beside: bool,
    /// When `Left`/`Right` cannot fit, prefer `Bottom` over `Top` if both
    /// have equal or sufficient room (`popover` / `hover-card` / menus).
    pub prefer_bottom_fallback: bool,
}

/// The resolved origin, side, and alignment after flipping and viewport clamping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub origin: Point<Pixels>,
    pub side: OverlaySide,
    pub align: OverlayAlign,
    pub max_height: Option<Pixels>,
}

/// Places a floating surface of `popup_size` next to `trigger` within
/// `viewport`, following riff-uiux's `place()` rules.
#[allow(clippy::too_many_arguments)]
pub(crate) fn place_side_with(
    trigger: Bounds<Pixels>,
    popup_size: Size<Pixels>,
    viewport: Bounds<Pixels>,
    side: OverlaySide,
    align: OverlayAlign,
    offset: Pixels,
    margin: Pixels,
    rtl: bool,
    options: PlaceOptions,
) -> Placed {
    let w = popup_size.width;
    let h = popup_size.height;

    let room_top = trigger.top() - viewport.top() - offset - margin;
    let room_bottom = viewport.bottom() - trigger.bottom() - offset - margin;
    let room_left = trigger.left() - viewport.left() - offset - margin;
    let room_right = viewport.right() - trigger.right() - offset - margin;

    let room = |s: OverlaySide| match s {
        OverlaySide::Top => room_top,
        OverlaySide::Bottom => room_bottom,
        OverlaySide::Left => room_left,
        OverlaySide::Right => room_right,
    };

    let mut resolved = side;
    let needed = if resolved.is_vertical() { h } else { w };
    if room(resolved) < needed && room(resolved.opposite()) > room(resolved) {
        resolved = resolved.opposite();
    }
    if !resolved.is_vertical() && room(resolved) < w {
        resolved = if options.prefer_bottom_fallback {
            if room_bottom >= room_top {
                OverlaySide::Bottom
            } else {
                OverlaySide::Top
            }
        } else if room_top >= h || room_top >= room_bottom {
            OverlaySide::Top
        } else {
            OverlaySide::Bottom
        };
    }

    let mut resolved_align = if resolved.is_vertical() && rtl {
        match align {
            OverlayAlign::Start => OverlayAlign::End,
            OverlayAlign::Center => OverlayAlign::Center,
            OverlayAlign::End => OverlayAlign::Start,
        }
    } else {
        align
    };

    let mut max_height = None;

    let (left, top) = if resolved.is_vertical() {
        let fit = if options.flip_align {
            let max_fit = h.min(room(resolved).max(px(120.)));
            if h > max_fit {
                max_height = Some(max_fit);
            }
            max_fit
        } else {
            h
        };
        let top = match resolved {
            OverlaySide::Bottom => trigger.bottom() + offset,
            OverlaySide::Top => trigger.top() - offset - fit,
            _ => unreachable!(),
        };
        let mut left = match resolved_align {
            OverlayAlign::Start => trigger.left(),
            OverlayAlign::End => trigger.right() - w,
            OverlayAlign::Center => trigger.left() + trigger.size.width / 2. - w / 2.,
        };
        let min_x = viewport.left() + margin;
        let max_x = (viewport.right() - margin - w).max(min_x);
        if options.flip_align {
            if resolved_align == OverlayAlign::Start
                && left + w > viewport.right() - margin
                && trigger.right() - w >= min_x
            {
                resolved_align = OverlayAlign::End;
                left = trigger.right() - w;
            } else if resolved_align == OverlayAlign::End
                && left < min_x
                && trigger.left() + w <= viewport.right() - margin
            {
                resolved_align = OverlayAlign::Start;
                left = trigger.left();
            }
        }
        (left.max(min_x).min(max_x), top)
    } else {
        let max_v = (viewport.size.height - margin * 2.).max(px(80.));
        if options.flip_align && h > max_v {
            max_height = Some(max_v);
        }
        let left = match resolved {
            OverlaySide::Right => trigger.right() + offset,
            OverlaySide::Left => trigger.left() - offset - w,
            _ => unreachable!(),
        };
        let mut top = match resolved_align {
            OverlayAlign::Start => trigger.top(),
            OverlayAlign::End => trigger.bottom() - h,
            OverlayAlign::Center => trigger.top() + trigger.size.height / 2. - h / 2.,
        };
        let min_y = viewport.top() + margin;
        let max_y = (viewport.bottom() - margin - h).max(min_y);
        if options.flip_align_beside {
            if resolved_align == OverlayAlign::Start
                && top + h > viewport.bottom() - margin
                && trigger.bottom() - h >= min_y
            {
                resolved_align = OverlayAlign::End;
                top = trigger.bottom() - h;
            } else if resolved_align == OverlayAlign::End
                && top < min_y
                && trigger.top() + h <= viewport.bottom() - margin
            {
                resolved_align = OverlayAlign::Start;
                top = trigger.top();
            }
        }
        (left, top.max(min_y).min(max_y))
    };

    Placed {
        origin: point(left.round(), top.round()),
        side: resolved,
        align: resolved_align,
        max_height,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn place_side(
    trigger: Bounds<Pixels>,
    popup_size: Size<Pixels>,
    viewport: Bounds<Pixels>,
    side: OverlaySide,
    align: OverlayAlign,
    offset: Pixels,
    margin: Pixels,
    rtl: bool,
) -> Placed {
    place_side_with(
        trigger,
        popup_size,
        viewport,
        side,
        align,
        offset,
        margin,
        rtl,
        PlaceOptions::default(),
    )
}

/// Per-window stack of open dismissible overlays (`popover`, `hover-card`,
/// `dropdown-menu`, `context-menu`, `dialog`, `command-menu`) so Escape and
/// outside-click handling always resolves innermost-first.
#[derive(Default)]
pub(crate) struct OverlayStack {
    windows: HashMap<WindowId, Vec<EntityId>>,
}

impl Global for OverlayStack {}

impl OverlayStack {
    pub fn push(window: &Window, id: EntityId, cx: &mut App) {
        let wid = window.window_handle().window_id();
        let stack = cx.default_global::<Self>().windows.entry(wid).or_default();
        if !stack.contains(&id) {
            stack.push(id);
        }
    }

    pub fn remove(window: &Window, id: EntityId, cx: &mut App) {
        if !cx.has_global::<Self>() {
            return;
        }
        let wid = window.window_handle().window_id();
        if let Some(stack) = cx.global_mut::<Self>().windows.get_mut(&wid) {
            stack.retain(|x| *x != id);
        }
    }

    pub fn is_topmost(window: &Window, id: EntityId, cx: &App) -> bool {
        let Some(global) = cx.try_global::<Self>() else {
            return true;
        };
        let wid = window.window_handle().window_id();
        global
            .windows
            .get(&wid)
            .is_none_or(|stack| stack.last().is_none_or(|top| *top == id))
    }

    pub fn has_descendant_after(window: &Window, id: EntityId, cx: &App) -> bool {
        let Some(global) = cx.try_global::<Self>() else {
            return false;
        };
        let wid = window.window_handle().window_id();
        let Some(stack) = global.windows.get(&wid) else {
            return false;
        };
        stack
            .iter()
            .position(|x| *x == id)
            .is_some_and(|idx| idx + 1 < stack.len())
    }
}
