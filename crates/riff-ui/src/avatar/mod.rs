//! riff-uiux `Avatar` and `AvatarGroup`: a person's photo or initials on a
//! deterministic tint, an agent's twinkling 3×3 pixel mark, corner status dot,
//! and overlapping avatar stacks ending in `+N`.

mod initials;
mod style;
#[cfg(test)]
mod tests;

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::base::Easing;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    App, Asset, AssetLogger, BorderStyle, Bounds, Corners, Edges, FontWeight, Global, Hsla, Image,
    ImageAssetLoader, ImageCacheError, ImageSource, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderImage, RenderOnce, Role, SharedString, Styled, StyledImage, TextRun, Window, canvas, div,
    font, img, linear_color_stop, linear_gradient, point, px, quad, size,
};

use crate::a11y::A11yLabel;
use crate::paint::{self, Scale, Tap};
use crate::theme::RiffTheme;
use initials::{agent_pattern, hue_of, initials_of};
use style::{ANON_SVG, ANON_SVG_KEY, IMAGE_FADE, Look, Metrics, WAVE_MS, WORKING_PERIOD};
pub use style::{AvatarKind, AvatarShape, AvatarSize, AvatarStatus};

/// A riff-uiux avatar.
///
/// ```ignore
/// Avatar::new().name("Mara Lindqvist").status(AvatarStatus::Online);
/// Avatar::new().kind(AvatarKind::Agent).seed("triage").name("Triage agent").working(true);
/// ```
#[derive(Clone, IntoElement)]
pub struct Avatar {
    name: Option<SharedString>,
    src: Option<ImageSource>,
    size: AvatarSize,
    shape: Option<AvatarShape>,
    status: Option<AvatarStatus>,
    kind: AvatarKind,
    seed: Option<SharedString>,
    color: Option<Hsla>,
    ring_color: Option<Hsla>,
    group_ring: Option<Hsla>,
    working: bool,
    accessibility_label: Option<SharedString>,
    accessibility_hidden: bool,
    frozen_at: Option<Duration>,
}

impl Default for Avatar {
    fn default() -> Self {
        Self::new()
    }
}

impl Avatar {
    pub fn new() -> Self {
        Self {
            name: None,
            src: None,
            size: AvatarSize::default(),
            shape: None,
            status: None,
            kind: AvatarKind::default(),
            seed: None,
            color: None,
            ring_color: None,
            group_ring: None,
            working: false,
            accessibility_label: None,
            accessibility_hidden: false,
            frozen_at: None,
        }
    }

    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        self.name = (!name.trim().is_empty()).then_some(name);
        self
    }

    pub fn src(mut self, src: impl Into<ImageSource>) -> Self {
        self.src = Some(src.into());
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn circle(self) -> Self {
        self.shape(AvatarShape::Circle)
    }

    pub fn square(self) -> Self {
        self.shape(AvatarShape::Square)
    }

    pub fn status(mut self, status: AvatarStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn kind(mut self, kind: AvatarKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn agent(self) -> Self {
        self.kind(AvatarKind::Agent)
    }

    pub fn seed(mut self, seed: impl Into<SharedString>) -> Self {
        self.seed = Some(seed.into());
        self
    }

    /// Agent mark color (defaults to blue).
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Background ring color used to cut out the status dot and group overlap
    /// (defaults to `theme.surface`).
    pub fn ring_color(mut self, color: Hsla) -> Self {
        self.ring_color = Some(color);
        self
    }

    pub fn working(mut self, working: bool) -> Self {
        self.working = working;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Hides the avatar from screen readers (`aria-hidden`), e.g. next to a
    /// visible person's name.
    pub fn accessibility_hidden(mut self, hidden: bool) -> Self {
        self.accessibility_hidden = hidden;
        self
    }

    /// Holds `working` wave / image fade at `elapsed` for story screenshots.
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }

    fn effective_shape(&self) -> AvatarShape {
        self.shape.unwrap_or(match self.kind {
            AvatarKind::Agent => AvatarShape::Square,
            AvatarKind::Person => AvatarShape::Circle,
        })
    }
}

struct AgentEpoch(Instant);

impl Global for AgentEpoch {}

#[derive(Clone)]
enum AvatarImageAsset {}

impl Asset for AvatarImageAsset {
    type Source = Arc<Image>;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move { source.to_image_data(renderer).map_err(Into::into) }
    }
}

fn resolve_image(
    src: &ImageSource,
    window: &mut Window,
    cx: &mut App,
) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
    match src {
        ImageSource::Resource(resource) => {
            window.use_asset::<AssetLogger<ImageAssetLoader>>(resource, cx)
        }
        ImageSource::Custom(loading_fn) => loading_fn(window, cx),
        ImageSource::Render(data) => Some(Ok(data.clone())),
        ImageSource::Image(data) => window.use_asset::<AssetLogger<AvatarImageAsset>>(data, cx),
    }
}

/// Opacity `[0.2, 1.0]` of one pixel cell at `elapsed`, matching CSS
/// `animation: ui-avt-pixel 1.2s ease-in-out infinite both` with `delay`.
pub(super) fn working_cell_opacity(elapsed: Duration, delay_ms: u64, ease: &Easing) -> f32 {
    let delay = Duration::from_millis(delay_ms);
    if elapsed < delay {
        return 1.;
    }
    let period = WORKING_PERIOD.as_secs_f64();
    let phase = ((elapsed - delay).as_secs_f64() % period / period) as f32;
    let t = if phase <= 0.5 {
        ease.sample(phase * 2.)
    } else {
        ease.sample((1. - phase) * 2.)
    };
    1. - 0.8 * t
}

fn shape_centered_text(
    text: SharedString,
    font_size: Pixels,
    theme: &RiffTheme,
    window: &mut Window,
) -> gpui_kit::ShapedLine {
    let mut f = font(theme.font_family.clone());
    f.weight = FontWeight::SEMIBOLD;
    f.features = paint::tabular_figures();
    let run = TextRun {
        len: text.len(),
        font: f,
        color: Hsla::black(),
        ..Default::default()
    };
    window
        .text_system()
        .shape_line(text, font_size, &[run], None)
}

fn paint_line_height_1(
    line: &gpui_kit::ShapedLine,
    font_size: Pixels,
    bounds: Bounds<Pixels>,
    color: Hsla,
    window: &mut Window,
) {
    let sf = window.scale_factor();
    let ascent = line.ascent.round();
    let descent = line.descent.round();
    let half_leading = ((font_size - (ascent + descent)) / 2.).floor();
    let baseline = bounds.center().y - font_size / 2. + half_leading + ascent;
    let baseline = px((baseline.as_f32() * sf + 0.5).floor() / sf);
    let x = bounds.center().x - line.width / 2.;
    let scale = Scale {
        center: bounds.center(),
        factor: 1.,
    };
    let taps = [Tap {
        offset: Default::default(),
        weight: 1.,
    }];
    paint::paint_text_tracked(
        line,
        point(x, baseline),
        font_size,
        px(0.),
        color,
        scale,
        &taps,
        window,
    );
}

impl RenderOnce for Avatar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let look = Look::of(&theme);
        let metrics = Metrics::of(self.size);
        let shape = self.effective_shape();
        let reduce_motion = cx.reduce_motion();
        let ring = self.ring_color.unwrap_or(look.ring);
        let group_ring = self.group_ring.map(|r| self.ring_color.unwrap_or(r));

        let spoken = if self.accessibility_hidden {
            None
        } else {
            self.accessibility_label.or_else(|| {
                let who = self.name.as_ref()?.trim();
                if who.is_empty() {
                    return None;
                }
                Some(match self.status {
                    Some(status) => format!("{who}, {}", status.spoken()).into(),
                    None => who.to_owned().into(),
                })
            })
        };

        let key_str = self
            .seed
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or("")
            .trim()
            .to_owned();

        let agent = self.kind == AvatarKind::Agent;
        let mark_color = self.color.unwrap_or(look.mark);
        let mark_cells = agent.then(|| {
            agent_pattern(if key_str.is_empty() {
                "agent"
            } else {
                &key_str
            })
        });

        let (face_bg, face_fg) = if agent {
            (look.tile_top, look.text)
        } else if key_str.is_empty() {
            (look.fill, look.text)
        } else {
            look.hue_pair(hue_of(&key_str))
        };

        let initials_text = (!agent)
            .then(|| initials_of(self.name.as_deref().unwrap_or("")))
            .filter(|s| !s.is_empty());
        let shaped_initials = initials_text
            .map(|text| shape_centered_text(text.into(), metrics.font_size, &theme, window));
        let show_anon = !agent && shaped_initials.is_none();

        let resolved_image = self
            .src
            .as_ref()
            .and_then(|src| resolve_image(src, window, cx))
            .and_then(Result::ok);
        let image_ready = resolved_image.is_some();

        let image_opacity = if !image_ready {
            0.
        } else if reduce_motion {
            1.
        } else if let Some(at) = self.frozen_at {
            let t = (at.as_secs_f32() / IMAGE_FADE.as_secs_f32()).clamp(0., 1.);
            Easing::cubic_bezier(0.25, 0.1, 0.25, 1.)
                .expect("valid control points")
                .sample(t)
        } else {
            let loaded_at = *window
                .use_state(cx, |_, cx| cx.background_executor().now())
                .read(cx);
            let elapsed = cx
                .background_executor()
                .now()
                .saturating_duration_since(loaded_at);
            if elapsed < IMAGE_FADE {
                window.request_animation_frame();
                let t = (elapsed.as_secs_f32() / IMAGE_FADE.as_secs_f32()).clamp(0., 1.);
                Easing::cubic_bezier(0.25, 0.1, 0.25, 1.)
                    .expect("valid control points")
                    .sample(t)
            } else {
                1.
            }
        };

        let working_opacities = if let Some(cells) = mark_cells {
            let mut out = [0.0f32; 9];
            if self.working && !reduce_motion {
                let elapsed = match self.frozen_at {
                    Some(at) => at,
                    None => {
                        window.request_animation_frame();
                        let now = cx.background_executor().now();
                        let start = match cx.try_global::<AgentEpoch>() {
                            Some(epoch) => epoch.0,
                            None => {
                                cx.set_global(AgentEpoch(now));
                                now
                            }
                        };
                        now.saturating_duration_since(start)
                    }
                };
                let ease = Easing::cubic_bezier(0.42, 0., 0.58, 1.).expect("valid control points");
                for i in 0..9 {
                    out[i] = working_cell_opacity(elapsed, WAVE_MS[i], &ease);
                }
            } else {
                for i in 0..9 {
                    out[i] = if cells[i] { 1. } else { 0. };
                }
            }
            Some(out)
        } else {
            None
        };

        let radius = match shape {
            AvatarShape::Circle => metrics.size * 0.5,
            AvatarShape::Square => metrics.size * 0.3,
        };

        let base_canvas = canvas(
            |_, _, _| {},
            move |bounds, (), window, cx| {
                if let Some(cutout) = group_ring {
                    paint::paint_rounded(bounds.dilate(px(2.)), radius + px(2.), cutout, window);
                }

                if agent {
                    paint::paint_rounded(
                        bounds,
                        radius,
                        linear_gradient(
                            180.,
                            linear_color_stop(look.tile_top, 0.),
                            linear_color_stop(look.tile_bottom, 1.),
                        ),
                        window,
                    );
                    if let Some(opacities) = working_opacities {
                        let grid_side = metrics.sq * 3. + metrics.gap * 2.;
                        let start_x = bounds.center().x - grid_side * 0.5;
                        let start_y = bounds.center().y - grid_side * 0.5;
                        let cell_r = metrics.sq * 0.25;
                        for r in 0..3 {
                            for c in 0..3 {
                                let a = opacities[r * 3 + c];
                                if a <= 0.001 {
                                    continue;
                                }
                                let cell_bounds = Bounds::new(
                                    point(
                                        start_x + (metrics.sq + metrics.gap) * c as f32,
                                        start_y + (metrics.sq + metrics.gap) * r as f32,
                                    ),
                                    size(metrics.sq, metrics.sq),
                                );
                                paint::paint_rounded(
                                    cell_bounds,
                                    cell_r,
                                    mark_color.opacity(a),
                                    window,
                                );
                            }
                        }
                    }
                } else if !image_ready || image_opacity < 1. {
                    paint::paint_rounded(bounds, radius, face_bg, window);
                    if let Some(line) = &shaped_initials {
                        paint_line_height_1(line, metrics.font_size, bounds, face_fg, window);
                    } else if show_anon {
                        let icon_side = metrics.size * 0.55;
                        let slot = Bounds::centered_at(bounds.center(), size(icon_side, icon_side));
                        let scale = Scale {
                            center: slot.center(),
                            factor: 1.,
                        };
                        let taps = [Tap {
                            offset: Default::default(),
                            weight: 1.,
                        }];
                        paint::paint_icon(
                            &ANON_SVG_KEY.into(),
                            Some(ANON_SVG),
                            slot,
                            face_fg,
                            scale,
                            1.,
                            0.,
                            &taps,
                            window,
                            cx,
                        );
                    }
                }
            },
        )
        .size_full();

        let status_dot = self.status.map(|status| {
            let dot_color = look.status(status);
            let offset = match shape {
                AvatarShape::Circle => px(0.),
                AvatarShape::Square => px(-2.),
            };
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let center = bounds.center();
                    let r = metrics.dot * 0.5;
                    paint::paint_circle(center, r + px(1.5), ring, window);
                    paint::paint_circle(center, r, dot_color, window);
                },
            )
            .absolute()
            .right(offset)
            .bottom(offset)
            .size(metrics.dot)
        });

        let draw_edge = agent || image_ready;
        let face = div()
            .relative()
            .flex_none()
            .size(metrics.size)
            .child(base_canvas)
            .when_some(resolved_image, |this, render_img| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .opacity(image_opacity)
                        .child(
                            img(render_img)
                                .size_full()
                                .rounded(radius)
                                .object_fit(ObjectFit::Cover),
                        ),
                )
            })
            .when(draw_edge, |this| {
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| {
                            window.paint_quad(quad(
                                bounds,
                                Corners::all(radius),
                                Hsla::transparent_black(),
                                Edges::all(px(1.)),
                                look.edge,
                                BorderStyle::Solid,
                            ));
                        },
                    )
                    .absolute()
                    .inset_0()
                    .size_full(),
                )
            })
            .when_some(status_dot, |this, dot| this.child(dot));

        A11yLabel::with_role(face, Role::Image, spoken)
    }
}

/// Overlapping stack of avatars ending in a `+N` overflow chip.
///
/// ```ignore
/// AvatarGroup::new()
///     .size(AvatarSize::S24)
///     .max(4)
///     .accessibility_label("5 reviewers")
///     .avatars([
///         Avatar::new().name("Mara Lindqvist"),
///         Avatar::new().name("Jonas Weber"),
///     ]);
/// ```
#[derive(Clone, IntoElement)]
pub struct AvatarGroup {
    items: Vec<Avatar>,
    max: usize,
    size: AvatarSize,
    total: Option<usize>,
    ring_color: Option<Hsla>,
    accessibility_label: Option<SharedString>,
}

impl Default for AvatarGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarGroup {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            max: 4,
            size: AvatarSize::default(),
            total: None,
            ring_color: None,
            accessibility_label: None,
        }
    }

    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.items.push(avatar);
        self
    }

    pub fn avatars(mut self, avatars: impl IntoIterator<Item = Avatar>) -> Self {
        self.items.extend(avatars);
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = max.max(1);
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn total(mut self, total: usize) -> Self {
        self.total = Some(total);
        self
    }

    pub fn ring_color(mut self, color: Hsla) -> Self {
        self.ring_color = Some(color);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = RiffTheme::global(cx).clone();
        let look = Look::of(&theme);
        let metrics = Metrics::of(self.size);
        let ring = self.ring_color.unwrap_or(look.ring);
        let overlap = -metrics.size * 0.25;

        let shown_len = self.items.len().min(self.max);
        let total = self.total.unwrap_or(0).max(self.items.len());
        let more = total.saturating_sub(shown_len);
        let last_shape = self.items[..shown_len]
            .last()
            .map_or(AvatarShape::Circle, Avatar::effective_shape);

        let mut row = div().flex().flex_none().items_center().h(metrics.size);
        for (i, mut avatar) in self.items.into_iter().take(shown_len).enumerate() {
            avatar.size = self.size;
            avatar.group_ring = Some(ring);
            row = row.child(
                div()
                    .flex_none()
                    .when(i > 0, |this| this.ml(overlap))
                    .child(avatar),
            );
        }

        if more > 0 {
            let text: SharedString = if more > 99 {
                "99+".into()
            } else {
                format!("+{more}").into()
            };
            let spoken: SharedString =
                format!("{} more", paint::format_grouped(more as u64)).into();
            let shaped = shape_centered_text(text, metrics.font_size, &theme, window);
            let width = (shaped.width + px(8.)).max(metrics.size);
            let radius = match last_shape {
                AvatarShape::Circle => px(999.),
                AvatarShape::Square => metrics.size * 0.3,
            };
            let fill = paint::mix(ring, look.fill, look.fill.a);

            let more_chip = canvas(
                |_, _, _| {},
                move |bounds, (), window, _| {
                    let r = radius
                        .min(bounds.size.width * 0.5)
                        .min(bounds.size.height * 0.5);
                    paint::paint_rounded(bounds.dilate(px(2.)), r + px(2.), ring, window);
                    paint::paint_rounded(bounds, r, fill, window);
                    paint_line_height_1(&shaped, metrics.font_size, bounds, look.text, window);
                },
            )
            .flex_none()
            .w(width)
            .h(metrics.size);

            let more_el = A11yLabel::with_role(more_chip, Role::Image, Some(spoken));
            row = row.child(
                div()
                    .flex_none()
                    .when(shown_len > 0, |this| this.ml(overlap))
                    .child(more_el),
            );
        }

        A11yLabel::with_role(row, Role::Group, self.accessibility_label)
    }
}
