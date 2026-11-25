//! Vector glyph painting for menu accessories (14×14 submenu chevron and
//! animated checkmark draw-in matching `.ui-dm__more` and `.ui-dm__tick`).

use gpui_kit::{
    App, Bounds, ElementId, Hsla, IntoElement, ParentElement as _, PathBuilder, Pixels, RenderOnce,
    Styled, Window, canvas, div, point, px,
};

use super::style::ACCESSORY_SIZE;

#[derive(IntoElement)]
pub(crate) struct ChevronGlyph {
    id: ElementId,
    color: Hsla,
}

impl ChevronGlyph {
    pub fn new(id: impl Into<ElementId>, color: Hsla) -> Self {
        Self {
            id: id.into(),
            color,
        }
    }
}

impl RenderOnce for ChevronGlyph {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.color;
        let _ = self.id;
        div().size(ACCESSORY_SIZE).flex_none().child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let scale = bounds.size.width.as_f32() / 24.;
                    let p = |vx: f32, vy: f32| {
                        point(
                            bounds.left() + px(scale * vx),
                            bounds.top() + px(scale * vy),
                        )
                    };
                    let mut builder = PathBuilder::stroke(px(scale * 1.5));
                    builder.move_to(p(9., 6.));
                    builder.curve_to(p(15., 12.), p(15., 10.42));
                    builder.curve_to(p(9., 18.), p(15., 13.58));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, color);
                    }
                },
            )
            .size_full(),
        )
    }
}

#[derive(IntoElement)]
pub(crate) struct CheckGlyph {
    id: ElementId,
    color: Hsla,
    /// `0.0..=1.0` stroke draw-in progress (`1.0` draws full check).
    progress: f32,
}

impl CheckGlyph {
    pub fn new(id: impl Into<ElementId>, color: Hsla, progress: f32) -> Self {
        Self {
            id: id.into(),
            color,
            progress: progress.clamp(0., 1.),
        }
    }
}

impl RenderOnce for CheckGlyph {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.color;
        let progress = self.progress;
        let _ = self.id;
        div().size(ACCESSORY_SIZE).flex_none().child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    paint_check(bounds, color, progress, window);
                },
            )
            .size_full(),
        )
    }
}

fn paint_check(bounds: Bounds<Pixels>, color: Hsla, progress: f32, window: &mut Window) {
    if progress <= 0.01 {
        return;
    }
    let scale = bounds.size.width.as_f32() / 24.;
    let p = |vx: f32, vy: f32| {
        point(
            bounds.left() + px(scale * vx),
            bounds.top() + px(scale * vy),
        )
    };
    let l1 = (8.5_f32 - 5.).hypot(17.5 - 14.);
    let l2 = (19.0_f32 - 8.5).hypot(6.5 - 17.5);
    let total = l1 + l2;
    let drawn = total * progress;

    let mut builder = PathBuilder::stroke(px(scale * 1.8));
    builder.move_to(p(5., 14.));
    if drawn <= l1 {
        let f = drawn / l1;
        builder.line_to(p(5. + (8.5 - 5.) * f, 14. + (17.5 - 14.) * f));
    } else {
        builder.line_to(p(8.5, 17.5));
        let f = ((drawn - l1) / l2).clamp(0., 1.);
        builder.line_to(p(8.5 + (19. - 8.5) * f, 17.5 + (6.5 - 17.5) * f));
    }
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
