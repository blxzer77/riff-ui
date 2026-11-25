//! Diagonal pointer corridor (monotone-chain convex hull + 4px slack) so moving
//! from the trigger to the hover card across the offset gap keeps the card open.

use gpui_kit::{Bounds, Pixels, Point};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    fn from_pixels(p: Point<Pixels>) -> Self {
        Self {
            x: f32::from(p.x),
            y: f32::from(p.y),
        }
    }
}

fn cross(o: Pt, a: Pt, b: Pt) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Computes the convex hull of the clamped trigger `exit` point and the 4
/// corners of `card` in counter-clockwise order (Andrew's monotone chain).
pub(super) fn hull_from_exit(
    exit: Point<Pixels>,
    trigger: Bounds<Pixels>,
    card: Bounds<Pixels>,
) -> Vec<Pt> {
    let clamped_exit = Point::new(
        exit.x.max(trigger.left()).min(trigger.right()),
        exit.y.max(trigger.top()).min(trigger.bottom()),
    );
    let left = f32::from(card.left());
    let right = f32::from(card.right());
    let top = f32::from(card.top());
    let bottom = f32::from(card.bottom());

    let mut pts = [
        Pt::from_pixels(clamped_exit),
        Pt { x: left, y: top },
        Pt { x: right, y: top },
        Pt {
            x: right,
            y: bottom,
        },
        Pt { x: left, y: bottom },
    ];
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));

    let mut lower: Vec<Pt> = Vec::with_capacity(5);
    for q in pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0. {
            lower.pop();
        }
        lower.push(q);
    }

    let mut upper: Vec<Pt> = Vec::with_capacity(5);
    for q in pts.into_iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0. {
            upper.pop();
        }
        upper.push(q);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Returns `true` when `p` lies inside `poly` or within 4px outside any edge.
pub(super) fn in_hull(poly: &[Pt], p: Point<Pixels>) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let x = f32::from(p.x);
    let y = f32::from(p.y);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let len = (b.x - a.x).hypot(b.y - a.y).max(1.);
        if ((b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)) / len < -4. {
            return false;
        }
    }
    true
}
