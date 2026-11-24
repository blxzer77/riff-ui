//! Shared painting, color, and CSS-scale helpers for riff-ui controls.
//!
//! GPUI quads carry no transformation, so a scaled control is painted rather
//! than laid out: every box is scaled about the control's center, glyphs are
//! painted at the scaled font size and positions, and icons go through the
//! SVG transformation matrix.
//!
//! At scale 1 the box is quads and shadows, which GPUI snaps to whole
//! pixels; the control's edges are on whole pixels then, as in a browser.
//! Scaled (pressed), its edges fall between pixels, and the box is drawn as
//! antialiased paths instead (`paint_box_scaled`).

use std::f32::consts::PI;
use std::sync::Arc;

use gpui_kit::{
    App, Background, BorderStyle, Bounds, BoxShadow, ContentMask, Corners, Edges, FontFeatures,
    Hsla, PathBuilder, Pixels, Point, Radians, Rgba, ShapedLine, SharedString, Size,
    TransformationMatrix, Window, linear_color_stop, linear_gradient, point, px, quad, size,
};

pub(crate) fn tabular_figures() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// How a control variant paints its box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Fill {
    Solid(Hsla),
    /// A top-to-bottom gradient inside the border, over a three-stop
    /// gradient filling the border box (which shows through the 1px border).
    Gradient {
        top: Hsla,
        bottom: Hsla,
        edge: [Hsla; 3],
    },
}

pub(crate) fn lerp_fill(from: Fill, to: Fill, t: f32) -> Fill {
    match (from, to) {
        (Fill::Solid(a), Fill::Solid(b)) => Fill::Solid(mix(a, b, t)),
        (
            Fill::Gradient { top, bottom, edge },
            Fill::Gradient {
                top: top_to,
                bottom: bottom_to,
                ..
            },
        ) => Fill::Gradient {
            top: mix(top, top_to, t),
            bottom: mix(bottom, bottom_to, t),
            edge,
        },
        (from, _) => from,
    }
}

pub(crate) fn hex(value: u32) -> Hsla {
    gpui_kit::rgb(value).into()
}

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    Rgba {
        r: f32::from(r) / 255.,
        g: f32::from(g) / 255.,
        b: f32::from(b) / 255.,
        a,
    }
    .into()
}

/// `color-mix(in srgb, a (1 - t), b t)`, which like CSS transitions mixes
/// premultiplied channels: fading out of `transparent` keeps the hue instead
/// of passing through black.
pub(crate) fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    let alpha = a.a + (b.a - a.a) * t;
    if alpha <= 0. {
        return Hsla::transparent_black();
    }
    let channel = |x: f32, y: f32| ((x * a.a) + (y * b.a - x * a.a) * t) / alpha;
    Rgba {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: alpha,
    }
    .into()
}

/// A uniform scale about a fixed center, in window coordinates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scale {
    pub center: Point<Pixels>,
    pub factor: f32,
}

impl Scale {
    pub fn point(&self, p: Point<Pixels>) -> Point<Pixels> {
        self.center + (p - self.center) * self.factor
    }

    pub fn len(&self, v: Pixels) -> Pixels {
        v * self.factor
    }

    /// A font size under this scale. Glyphs are rasterized and cached per
    /// size and never evicted, so the factor is rounded to steps of 0.005:
    /// the press transition then uses a handful of sizes instead of one per frame.
    pub fn font_size(&self, size: Pixels) -> Pixels {
        size * ((self.factor * 200.).round() / 200.)
    }

    pub fn bounds(&self, b: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::new(
            self.point(b.origin),
            size(b.size.width * self.factor, b.size.height * self.factor),
        )
    }
}

/// Paints the drop shadows, fill and border of a control box. `faded`: the
/// control is painted with an opacity below 1.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_box(
    bounds: Bounds<Pixels>,
    radius: Pixels,
    border_width: Pixels,
    fill: Fill,
    border: Hsla,
    shadows: &[BoxShadow],
    scale: Scale,
    faded: bool,
    window: &mut Window,
) {
    let outer = scale.bounds(bounds);
    let r = scale.len(radius);
    let scaled_border = scale.len(border_width);
    let shadows: Vec<_> = shadows
        .iter()
        .map(|s| BoxShadow {
            offset: s.offset * scale.factor,
            blur_radius: scale.len(s.blur_radius),
            spread_radius: scale.len(s.spread_radius),
            ..*s
        })
        .collect();
    if scale.factor != 1. {
        // Scaled (pressed), the box's edges fall between pixels. Quads and
        // shadows snap to whole pixels; paths do not, and are antialiased.
        paint_box_scaled(outer, r, scaled_border, fill, border, &shadows, window);
        return;
    }
    if faded {
        paint_outer_shadows(outer, r, &shadows, window);
    } else {
        // Every variant with a shadow has an opaque fill that hides the
        // shadow's inside.
        drop_shadows(outer, r, &shadows, window);
    }

    match fill {
        Fill::Solid(color) => window.paint_quad(quad(
            outer,
            Corners::all(r),
            color,
            Edges::all(scaled_border),
            border,
            BorderStyle::Solid,
        )),
        Fill::Gradient { top, bottom, edge } => {
            // The padding-box layer `linear-gradient(top, bottom)` fills the
            // inside; the border-box layer `linear-gradient(edge-top,
            // edge-mid, edge-bottom)` shows only through the 1px border.
            //
            // One quad carries both: the fill as its background and the edge
            // as its border, which the shader composites in one pass. Two
            // overlapping quads would leave a dark seam where their
            // antialiased edges meet, and under `opacity` (applied per
            // primitive in GPUI, to the composite in CSS) the edge would show
            // through the fill. A border has one color and the edge gradient
            // is vertical, so the quad is painted one device row at a time,
            // each row clipped to itself and given that row's edge color.
            //
            // The fill gradient spans the padding box, 1px in from the top
            // and bottom of the quad: its stops are placed there.
            let inset = scaled_border / outer.size.height;
            let fill = linear_gradient(
                180.,
                linear_color_stop(top, inset),
                linear_color_stop(bottom, 1. - inset),
            );

            let sf = window.scale_factor();
            for (row_top, row_bottom) in device_rows(outer.top(), outer.bottom(), sf) {
                let middle = (row_top + row_bottom) / 2.;
                let t = ((middle - outer.top()) / outer.size.height).clamp(0., 1.);
                let color = if t < 0.5 {
                    mix(edge[0], edge[1], t * 2.)
                } else {
                    mix(edge[1], edge[2], t * 2. - 1.)
                };
                let band = ContentMask {
                    bounds: Bounds::from_corners(
                        point(outer.left() - px(1.), row_top),
                        point(outer.right() + px(1.), row_bottom),
                    ),
                };
                window.with_content_mask(Some(band), |window| {
                    // Faded, the fill must not sit under the border: GPUI
                    // applies `opacity` to each color before compositing the
                    // border over the background, so a 0.4 border over a 0.4
                    // fill mixed the two, where CSS shows the edge alone at
                    // 0.4. The fill is painted once below, inside the border.
                    let background = if faded {
                        Hsla::transparent_black().into()
                    } else {
                        fill
                    };
                    window.paint_quad(quad(
                        outer,
                        Corners::all(r),
                        background,
                        Edges::all(scaled_border),
                        color,
                        BorderStyle::Solid,
                    ));
                });
            }
            if faded {
                // The padding box: inside the border, its corners curved by
                // the outer radius less the border width, as CSS does. It
                // meets the border's inner edge without overlapping it.
                let inner = Bounds::new(
                    outer.origin + point(scaled_border, scaled_border),
                    outer.size - size(scaled_border * 2., scaled_border * 2.),
                );
                window.paint_quad(quad(
                    inner,
                    Corners::all((r - scaled_border).max(px(0.))),
                    linear_gradient(
                        180.,
                        linear_color_stop(top, 0.),
                        linear_color_stop(bottom, 1.),
                    ),
                    Edges::all(px(0.)),
                    Hsla::transparent_black(),
                    BorderStyle::Solid,
                ));
            }
        }
    }
}

/// The box while scaled: the same layers as [`paint_box`], drawn as paths at
/// their exact fractional positions, as a browser draws a transformed layer.
///
/// - a solid ring (`0 0 0 Npx`, no blur) is the band between the box outline
///   and the outline grown by N, corners `r` and `r + N` (CSS's rule);
/// - blurred shadows stay GPUI shadows: soft already, they hide the snapping;
/// - the border band lies between the box outline and the padding box (`r`
///   and `r - border`), filled with the border color or, for primary, the
///   edge gradient in two halves (GPUI gradients have two stops);
/// - the fill is the padding box for a gradient, the whole box for a solid.
fn paint_box_scaled(
    outer: Bounds<Pixels>,
    r: Pixels,
    border_width: Pixels,
    fill: Fill,
    border: Hsla,
    shadows: &[BoxShadow],
    window: &mut Window,
) {
    for shadow in shadows {
        let solid_ring = shadow.blur_radius == px(0.)
            && shadow.offset == Point::default()
            && shadow.spread_radius > px(0.);
        if solid_ring {
            let s = shadow.spread_radius;
            paint_band(outer.dilate(s), r + s, outer, r, shadow.color, window);
        } else {
            drop_shadows(outer, r, std::slice::from_ref(shadow), window);
        }
    }

    let inner = outer.dilate(-border_width);
    let inner_r = (r - border_width).max(px(0.));
    match fill {
        Fill::Solid(color) => {
            if color.a > 0. {
                paint_rounded(outer, r, color, window);
            }
            if border.a > 0. {
                paint_band(outer, r, inner, inner_r, border, window);
            }
        }
        Fill::Gradient { top, bottom, edge } => {
            // The fill runs under the border band, over the whole box: two
            // shapes that only meet are antialiased separately and leave a
            // light seam where they meet. Its stops sit at the padding box's
            // top and bottom, where CSS places the padding-box gradient.
            let inset = border_width / outer.size.height;
            paint_rounded(
                outer,
                r,
                linear_gradient(
                    180.,
                    linear_color_stop(top, inset),
                    linear_color_stop(bottom, 1. - inset),
                ),
                window,
            );
            // The edge gradient runs over the whole box in two halves; each
            // half paints the band masked to its half. Each half reaches a
            // pixel past the box on its outer side, for the antialiased edge
            // there (see `fringe`); its stops are placed so the gradient
            // still runs from the box's edge to the middle.
            let middle = outer.center().y;
            let pad = px(1.);
            for (from, to, top, bottom, top_pad, bottom_pad) in [
                (edge[0], edge[1], outer.top(), middle, pad, px(0.)),
                (edge[1], edge[2], middle, outer.bottom(), px(0.), pad),
            ] {
                let area = Bounds::from_corners(
                    point(outer.left() - pad, top - top_pad),
                    point(outer.right() + pad, bottom + bottom_pad),
                );
                let span = area.size.height;
                let gradient = linear_gradient(
                    180.,
                    linear_color_stop(from, top_pad / span),
                    linear_color_stop(to, 1. - bottom_pad / span),
                );
                window.with_content_mask(Some(ContentMask { bounds: area }), |window| {
                    paint_band_with(outer, r, inner, inner_r, area, gradient, window);
                });
            }
        }
    }
}

/// A rounded rectangle as path commands: lines and quarter arcs, clockwise.
fn add_rounded(builder: &mut PathBuilder, b: Bounds<Pixels>, r: Pixels) {
    let r = r.min(b.size.width / 2.).min(b.size.height / 2.).max(px(0.));
    let (l, t, rt, bt) = (b.left(), b.top(), b.right(), b.bottom());
    let radii = point(r, r);
    builder.move_to(point(l + r, t));
    builder.line_to(point(rt - r, t));
    builder.arc_to(radii, px(0.), false, true, point(rt, t + r));
    builder.line_to(point(rt, bt - r));
    builder.arc_to(radii, px(0.), false, true, point(rt - r, bt));
    builder.line_to(point(l + r, bt));
    builder.arc_to(radii, px(0.), false, true, point(l, bt - r));
    builder.line_to(point(l, t + r));
    builder.arc_to(radii, px(0.), false, true, point(l + r, t));
    builder.close();
}

pub(crate) fn paint_rounded(
    b: Bounds<Pixels>,
    r: Pixels,
    color: impl Into<Background>,
    window: &mut Window,
) {
    let mut builder = PathBuilder::fill();
    add_rounded(&mut builder, b, r);
    if let Ok(mut path) = builder.build() {
        path.bounds = fringe(path.bounds);
        window.paint_path(path, color);
    }
}

/// Paints an antialiased filled circle at fractional center/radius.
pub(crate) fn paint_circle(
    center: Point<Pixels>,
    radius: Pixels,
    color: Hsla,
    window: &mut Window,
) {
    if radius <= px(0.) || color.a <= 0. {
        return;
    }
    let bounds = Bounds::centered_at(center, size(radius * 2., radius * 2.));
    paint_rounded(bounds, radius, color, window);
}

/// Grows a path's bounds by a pixel so its antialiased fringe is drawn.
///
/// GPUI rasterizes a path into an offscreen texture, then copies the
/// rectangle `bounds` to the screen, and that copy only covers pixels whose
/// centre lies inside it. A ring whose outer edge is at -0.37px has 37%
/// coverage in the row above, centred at -0.5: outside the exact bounds,
/// so it was dropped and every edge drew a pixel thinner than riff-uiux's.
/// The texture is transparent past the shape, so the margin adds nothing.
fn fringe(bounds: Bounds<Pixels>) -> Bounds<Pixels> {
    bounds.dilate(px(1.))
}

/// The band between an outer and an inner rounded rectangle (even-odd fill).
fn paint_band(
    outer: Bounds<Pixels>,
    outer_r: Pixels,
    inner: Bounds<Pixels>,
    inner_r: Pixels,
    color: impl Into<Background>,
    window: &mut Window,
) {
    let mut builder = PathBuilder::fill();
    add_rounded(&mut builder, outer, outer_r);
    add_rounded(&mut builder, inner, inner_r);
    if let Ok(mut path) = builder.build() {
        path.bounds = fringe(path.bounds);
        window.paint_path(path, color);
    }
}

/// [`paint_band`] with its gradient laid over `gradient_bounds` instead of
/// the band's own bounding box. A path's gradient is evaluated against the
/// path's `bounds`, which GPUI also uses (intersected with the content mask)
/// as the region it composites; the caller passes the area the band is
/// masked to, already grown for the antialiased fringe.
fn paint_band_with(
    outer: Bounds<Pixels>,
    outer_r: Pixels,
    inner: Bounds<Pixels>,
    inner_r: Pixels,
    gradient_bounds: Bounds<Pixels>,
    gradient: Background,
    window: &mut Window,
) {
    let mut builder = PathBuilder::fill();
    add_rounded(&mut builder, outer, outer_r);
    add_rounded(&mut builder, inner, inner_r);
    if let Ok(mut path) = builder.build() {
        path.bounds = gradient_bounds;
        window.paint_path(path, gradient);
    }
}

/// Paints drop shadows with CSS's corner rule: a shadow spread by `s` has
/// corners of radius `r + s` (never below 0). GPUI keeps the element's own
/// radius for the dilated shadow, which made primary's `0 0 0 1px` ring
/// square-cornered and sit outside riff-uiux's at every corner.
fn drop_shadows(outer: Bounds<Pixels>, r: Pixels, shadows: &[BoxShadow], window: &mut Window) {
    for shadow in shadows {
        let radius = (r + shadow.spread_radius).max(px(0.));
        window.paint_drop_shadows(outer, Corners::all(radius), std::slice::from_ref(shadow));
    }
}

/// Drop shadows that, as in CSS, never show inside the box.
///
/// GPUI paints a drop shadow under the whole element and relies on the
/// element covering it. With `opacity` below 1 the box no longer covers it
/// (GPUI fades each primitive, CSS fades the composite), so the shadow is
/// painted row by row with the box's span on each row masked out.
fn paint_outer_shadows(
    outer: Bounds<Pixels>,
    r: Pixels,
    shadows: &[BoxShadow],
    window: &mut Window,
) {
    if shadows.is_empty() {
        return;
    }
    let mut blurred = Vec::new();
    for shadow in shadows {
        let solid_ring = shadow.blur_radius == px(0.)
            && shadow.offset == Point::default()
            && shadow.spread_radius > px(0.);
        if solid_ring {
            let s = shadow.spread_radius;
            paint_band(outer.dilate(s), r + s, outer, r, shadow.color, window);
        } else {
            blurred.push(shadow.clone());
        }
    }
    if blurred.is_empty() {
        return;
    }
    let reach = blurred
        .iter()
        .map(|s| s.blur_radius * 3. + s.spread_radius.max(px(0.)) + s.offset.y.abs())
        .fold(px(0.), Pixels::max)
        + px(1.);
    let area = outer.dilate(reach);
    // Above and below the box: whole bands, on the device pixel grid.
    let sf = window.scale_factor();
    let snap_down = |v: Pixels| px((v.as_f32() * sf).floor() / sf);
    let snap_up = |v: Pixels| px((v.as_f32() * sf).ceil() / sf);
    let (box_top, box_bottom) = (snap_down(outer.top()), snap_up(outer.bottom()));
    for band in [
        Bounds::from_corners(area.origin, point(area.right(), box_top)),
        Bounds::from_corners(point(area.left(), box_bottom), area.bottom_right()),
    ] {
        window.with_content_mask(Some(ContentMask { bounds: band }), |window| {
            drop_shadows(outer, r, &blurred, window);
        });
    }
    // Beside the box: per device row, left and right of its rounded span.
    // The span uses the row's widest extent (smallest corner inset) so the
    // shadow never leaks inside the translucent box.
    for (top, bottom) in device_rows(outer.top(), outer.bottom(), sf) {
        let inset = corner_inset(outer, r, top, bottom);
        for band in [
            Bounds::from_corners(
                point(area.left(), top),
                point(snap_down(outer.left() + inset), bottom),
            ),
            Bounds::from_corners(
                point(snap_up(outer.right() - inset), top),
                point(area.right(), bottom),
            ),
        ] {
            window.with_content_mask(Some(ContentMask { bounds: band }), |window| {
                drop_shadows(outer, r, &blurred, window);
            });
        }
    }
}

/// How far in from the side the rounded box's edge is, at its widest within
/// the rows `top..bottom`: the shadow stays strictly outside this span.
fn corner_inset(outer: Bounds<Pixels>, r: Pixels, top: Pixels, bottom: Pixels) -> Pixels {
    let r = r.as_f32();
    // Distance into the corner arc's vertical extent at the row edge closer
    // to the vertical center, where the box is widest (smallest inset).
    let dy_top = (outer.top().as_f32() + r) - bottom.as_f32();
    let dy_bottom = top.as_f32() - (outer.bottom().as_f32() - r);
    let dy = dy_top.max(dy_bottom);
    if dy <= 0. {
        return px(0.);
    }
    let dy = dy.min(r);
    px(r - (r * r - dy * dy).sqrt())
}

/// Splits `top..bottom` into rows one device pixel high, on the device pixel
/// grid. Content masks are rounded outwards to whole device pixels, so rows
/// off the grid would overlap and paint their antialiased pixels twice.
fn device_rows(
    top: Pixels,
    bottom: Pixels,
    scale_factor: f32,
) -> impl Iterator<Item = (Pixels, Pixels)> {
    let first = (top.as_f32() * scale_factor).floor() as i32;
    let last = (bottom.as_f32() * scale_factor).ceil() as i32;
    (first..last).map(move |row| {
        (
            px(row as f32 / scale_factor),
            px((row + 1) as f32 / scale_factor),
        )
    })
}

/// `outline: 2px solid; outline-offset: 2px` around the scaled box.
pub(crate) fn paint_focus_ring(
    bounds: Bounds<Pixels>,
    radius: Pixels,
    color: Hsla,
    scale: Scale,
    window: &mut Window,
) {
    let outer = scale.bounds(bounds);
    let r = scale.len(radius);
    // The outline belongs to the transformed element, so it scales too.
    let offset = scale.len(px(2.));
    let width = scale.len(px(2.));
    let ring = outer.dilate(offset + width);
    window.paint_quad(quad(
        ring,
        Corners::all(r + offset + width),
        Hsla::transparent_black(),
        Edges::all(width),
        color,
        BorderStyle::Solid,
    ));
}

/// One layer of a blur approximation: an offset and its weight.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tap {
    pub offset: Point<Pixels>,
    pub weight: f32,
}

/// Taps approximating a Gaussian blur of standard deviation `sigma`, CSS
/// `filter: blur(sigma)`. A single unit-weight tap when there is no blur.
///
/// The kernel is sampled on a polar grid (a center plus rings) and its
/// weights normalized, so the layers add up to the unblurred coverage.
pub(crate) fn blur_taps(sigma: Pixels) -> Vec<Tap> {
    let s = sigma.as_f32();
    if s < 0.05 {
        return vec![Tap {
            offset: Point::default(),
            weight: 1.,
        }];
    }
    let gauss = |r: f32| (-(r * r) / (2. * s * s)).exp();
    // Rings at 0.75σ, 1.5σ and 2.25σ cover 99% of the kernel's mass.
    let rings = [(0.75, 6), (1.5, 10), (2.25, 14)];
    let mut taps = vec![Tap {
        offset: Point::default(),
        weight: 1.,
    }];
    for (k, (rel, count)) in rings.iter().enumerate() {
        let r = rel * s;
        // Each tap stands for the annulus around its ring: weight by area.
        let inner = if k == 0 { 0.375 * s } else { (rel - 0.375) * s };
        let outer = (rel + 0.375) * s;
        let area = PI * (outer * outer - inner * inner) / *count as f32;
        let weight = gauss(r) * area / (PI * (0.375 * s).powi(2));
        for i in 0..*count {
            let angle = 2. * PI * i as f32 / *count as f32 + k as f32 * 0.3;
            taps.push(Tap {
                offset: point(px(r * angle.cos()), px(r * angle.sin())),
                weight,
            });
        }
    }
    let total: f32 = taps.iter().map(|t| t.weight).sum();
    for tap in &mut taps {
        tap.weight /= total;
    }
    taps
}

/// The alpha of one blur layer of weight `weight`, such that where every
/// layer covers a pixel they composite (`over`) to `target`:
/// `1 - Π(1 - aᵢ) = target` when `aᵢ = 1 - (1 - target)^wᵢ` and `Σwᵢ = 1`.
///
/// GPUI blends with `over`, not additively, so this is what keeps a blurred
/// glyph's interior at full strength. Partly covered pixels come out a little
/// stronger than a true convolution would make them.
pub(crate) fn layer_alpha(target: f32, weight: f32) -> f32 {
    if weight >= 1. {
        return target;
    }
    // A fully opaque target needs some transmission left to split.
    let target = target.clamp(0., 0.995);
    1. - (1. - target).powf(weight)
}

/// `letter-spacing: -0.01em` of button.tsx. GPUI has no letter spacing, and
/// riff-ui paints every glyph itself, so the offset is applied here.
pub(crate) fn tracking(font_size: Pixels) -> Pixels {
    font_size * -0.01
}

/// The distance a glyph shifts under `letter_spacing` (`px` per character).
pub(crate) fn tracked_offset(letter_spacing: Pixels, glyph_index: usize) -> Pixels {
    letter_spacing * glyph_index as f32
}

/// Paints a shaped line with its baseline origin at `origin` (unscaled
/// coordinates), scaled, blurred by `taps` (offsets in scaled pixels), with
/// explicit `letter_spacing`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_tracked(
    line: &ShapedLine,
    origin: Point<Pixels>,
    font_size: Pixels,
    letter_spacing: Pixels,
    color: Hsla,
    scale: Scale,
    taps: &[Tap],
    window: &mut Window,
) {
    let scaled_size = scale.font_size(font_size);
    for run in &line.runs {
        for glyph in &run.glyphs {
            let at = scale.point(
                origin
                    + glyph.position
                    + point(tracked_offset(letter_spacing, glyph.index), px(0.)),
            );
            for tap in taps {
                let at = at + tap.offset;
                if glyph.is_emoji {
                    let _ = window.paint_emoji(at, run.font_id, glyph.id, scaled_size);
                } else {
                    let layer = Hsla {
                        a: layer_alpha(color.a, tap.weight),
                        ..color
                    };
                    let _ = window.paint_glyph(at, run.font_id, glyph.id, scaled_size, layer);
                }
            }
        }
    }
}

/// Paints a shaped line with button's `-0.01em` tracking.
pub(crate) fn paint_text(
    line: &ShapedLine,
    origin: Point<Pixels>,
    font_size: Pixels,
    color: Hsla,
    scale: Scale,
    taps: &[Tap],
    window: &mut Window,
) {
    paint_text_tracked(
        line,
        origin,
        font_size,
        tracking(font_size),
        color,
        scale,
        taps,
        window,
    );
}

/// Paints the glyphs of `line` that fall into a horizontal band, for the
/// loading sweep. `band` is in unscaled coordinates; masks nest so the band
/// fades out towards its edges like the CSS gradient does.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_sweep(
    line: &ShapedLine,
    origin: Point<Pixels>,
    line_bounds: Bounds<Pixels>,
    font_size: Pixels,
    shine: Hsla,
    center: Pixels,
    half_width: Pixels,
    scale: Scale,
    window: &mut Window,
) {
    // `linear-gradient(90deg, transparent 38%, shine 50%, transparent 62%)`
    // is a triangle: alpha falls linearly from the center to ±half_width.
    // Nested masks of shrinking width each add an equal share of it.
    const LAYERS: usize = 12;
    let scaled_size = scale.font_size(font_size);
    for layer in 0..LAYERS {
        // Where the first k layers overlap they must composite to
        // `shine.a * k / LAYERS`, so the profile is the CSS triangle:
        // `1 - c_k = (1 - c_{k-1}) (1 - a_k)`.
        let reached = |k: usize| shine.a * k as f32 / LAYERS as f32;
        let alpha = 1. - (1. - reached(layer + 1)) / (1. - reached(layer));
        let color = Hsla { a: alpha, ..shine };
        let w = half_width * (1. - layer as f32 / LAYERS as f32);
        let left = (center - w).max(line_bounds.left());
        let right = (center + w).min(line_bounds.right());
        if right <= left {
            continue;
        }
        let band = Bounds::from_corners(
            point(left, line_bounds.top()),
            point(right, line_bounds.bottom()),
        );
        let mask = ContentMask {
            bounds: scale.bounds(band),
        };
        window.with_content_mask(Some(mask), |window| {
            for run in &line.runs {
                for glyph in &run.glyphs {
                    if glyph.is_emoji {
                        continue;
                    }
                    let at = scale.point(
                        origin
                            + glyph.position
                            + point(tracked_offset(tracking(font_size), glyph.index), px(0.)),
                    );
                    let _ = window.paint_glyph(at, run.font_id, glyph.id, scaled_size, color);
                }
            }
        });
    }
}

/// Paints an SVG icon into `bounds` (unscaled), scaled about `scale` and by
/// its own `own_scale` about its center, blurred by `taps`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_icon(
    path: &SharedString,
    data: Option<&[u8]>,
    bounds: Bounds<Pixels>,
    color: Hsla,
    scale: Scale,
    own_scale: f32,
    rotation: f32,
    taps: &[Tap],
    window: &mut Window,
    cx: &mut App,
) {
    // The SVG is rasterized at the unscaled icon size (sharp at scale 1, the
    // resting state) and transformed on the GPU, as a browser composites a
    // scaled layer.
    let center = scale.point(bounds.center());
    let sf = window.scale_factor();
    for tap in taps {
        let c = center + tap.offset;
        let factor = scale.factor * own_scale;
        let matrix = TransformationMatrix::unit()
            .translate(c.scale(sf))
            .rotate(Radians(rotation))
            .scale(Size::new(factor, factor))
            .translate(bounds.center().scale(-sf));
        let layer = Hsla {
            a: layer_alpha(color.a, tap.weight),
            ..color
        };
        let _ = window.paint_svg(bounds, path.clone(), data, matrix, layer, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_rgba(color: Hsla, expected: (f32, f32, f32, f32)) {
        let c = color.to_rgb();
        let actual = (c.r * 255., c.g * 255., c.b * 255., c.a);
        let close = (actual.0 - expected.0).abs() < 0.6
            && (actual.1 - expected.1).abs() < 0.6
            && (actual.2 - expected.2).abs() < 0.6
            && (actual.3 - expected.3).abs() < 0.002;
        assert!(close, "{actual:?} != {expected:?}");
    }

    #[test]
    fn mixing_with_transparent_keeps_the_color() {
        let red = rgba(255, 107, 95, 1.);
        assert_rgba(
            mix(Hsla::transparent_black(), red, 0.16),
            (255., 107., 95., 0.16),
        );
    }

    #[test]
    fn no_blur_is_one_full_tap() {
        let taps = blur_taps(px(0.));
        assert_eq!(taps.len(), 1);
        assert_eq!(taps[0].weight, 1.);
    }

    #[test]
    fn blur_weights_sum_to_one_and_spread_with_sigma() {
        for sigma in [0.5, 1., 3.] {
            let taps = blur_taps(px(sigma));
            let total: f32 = taps.iter().map(|t| t.weight).sum();
            assert!((total - 1.).abs() < 1e-4);
            let reach = taps
                .iter()
                .map(|t| (t.offset.x.as_f32().powi(2) + t.offset.y.as_f32().powi(2)).sqrt())
                .fold(0., f32::max);
            assert!((reach - 2.25 * sigma).abs() < 1e-3);
        }
    }

    #[test]
    fn scale_is_about_the_center() {
        let s = Scale {
            center: point(px(50.), px(20.)),
            factor: 0.97,
        };
        let b = s.bounds(Bounds::new(point(px(0.), px(0.)), size(px(100.), px(40.))));
        assert!((b.origin.x.as_f32() - 1.5).abs() < 1e-4);
        assert!((b.size.height.as_f32() - 38.8).abs() < 1e-4);
        assert_eq!(b.center(), s.center);
    }
}
