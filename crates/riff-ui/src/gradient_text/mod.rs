//! riff-uiux `GradientText`: letters clipped from a flowing multi-blob
//! gradient over a two-stop linear base, with an optional left-to-right sweep
//! on first reveal.

mod mask;
mod style;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::base::Easing;
use gpui_kit::{
    App, Bounds, ContentMask, Corners, Element, ElementId, Entity, FontWeight, Global,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Refineable,
    RenderImage, RenderOnce, Role, ShapedLine, SharedString, StyleRefinement, Styled, TextRun,
    TextStyleRefinement, Window, accesskit, point, px, size,
};

use crate::paint::{self, Scale, Tap};
use crate::theme::RiffTheme;
use mask::{AlphaMask, MaskKey};
use style::{
    BASE_FADE_DELAY, BASE_FADE_DURATION, CLIP_PAD_EM, DEFAULT_SPEED, GradientField, Palette,
    SWEEP_DURATION,
};

/// Semantic element kind for [`GradientText`], matching `as` in riff-uiux.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GradientTextAs {
    #[default]
    Span,
    H1,
    H2,
    P,
}

impl GradientTextAs {
    fn role(self) -> Role {
        match self {
            Self::Span => Role::Label,
            Self::H1 | Self::H2 => Role::Heading,
            Self::P => Role::Paragraph,
        }
    }
}

/// A riff-uiux gradient text span or heading.
///
/// Inherits font size, weight, line height, and base ink color from its
/// enclosing element or its own [`Styled`] setters.
///
/// ```ignore
/// div()
///     .text_2xl()
///     .font_medium()
///     .child(GradientText::new("feel."));
/// ```
#[derive(Clone, IntoElement)]
pub struct GradientText {
    id: Option<ElementId>,
    text: SharedString,
    colors: Option<Vec<Hsla>>,
    speed: Duration,
    sweep: bool,
    as_el: GradientTextAs,
    style: StyleRefinement,
    frozen_at: Option<Duration>,
    frozen_sweep_at: Option<Duration>,
}

impl GradientText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            id: None,
            text: text.into(),
            colors: None,
            speed: DEFAULT_SPEED,
            sweep: true,
            as_el: GradientTextAs::default(),
            style: StyleRefinement::default(),
            frozen_at: None,
            frozen_sweep_at: None,
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Up to four gradient colors (`c1, c2, c3, c4`). Replaces dark-mode
    /// defaults too.
    pub fn colors(mut self, colors: impl IntoIterator<Item = Hsla>) -> Self {
        self.colors = Some(colors.into_iter().collect());
        self
    }

    /// Flow cycle duration (defaults to `8s`).
    pub fn speed(mut self, speed: Duration) -> Self {
        self.speed = if speed.is_zero() {
            DEFAULT_SPEED
        } else {
            speed
        };
        self
    }

    /// Flow cycle duration in seconds (defaults to `8.0`).
    pub fn speed_secs(self, seconds: f32) -> Self {
        let secs = if seconds.is_finite() && seconds > 0. {
            seconds
        } else {
            8.
        };
        self.speed(Duration::from_secs_f32(secs))
    }

    /// Sweeps the gradient in from left to right on mount (defaults to `true`).
    pub fn sweep(mut self, sweep: bool) -> Self {
        self.sweep = sweep;
        self
    }

    pub fn as_element(mut self, as_el: GradientTextAs) -> Self {
        self.as_el = as_el;
        self
    }

    /// Holds the flowing gradient at `elapsed` into its cycle (with sweep
    /// finished, matching `tools/reference/build.py`'s end-state transitions).
    pub fn freeze_motion_at(mut self, elapsed: Duration) -> Self {
        self.frozen_at = Some(elapsed);
        self
    }

    /// Holds the initial sweep wipe at `elapsed` (`0..=1100ms`), useful for tests.
    pub fn freeze_sweep_at(mut self, elapsed: Duration) -> Self {
        self.frozen_sweep_at = Some(elapsed);
        self
    }
}

impl Styled for GradientText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for GradientText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = self.id.clone().unwrap_or_else(|| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.text.hash(&mut h);
            self.speed.hash(&mut h);
            self.sweep.hash(&mut h);
            if let Some(colors) = &self.colors {
                for c in colors {
                    c.h.to_bits().hash(&mut h);
                    c.s.to_bits().hash(&mut h);
                    c.l.to_bits().hash(&mut h);
                    c.a.to_bits().hash(&mut h);
                }
            }
            ElementId::Integer(h.finish())
        });
        let state = window.use_keyed_state(
            ElementId::NamedChild(state_id.into(), "riff-gradient-text".into()),
            cx,
            |_, cx| GradientState::new(cx.background_executor().now()),
        );
        GradientTextElement { props: self, state }
    }
}

struct FlowEpoch(Instant);

impl Global for FlowEpoch {}

#[derive(Clone, Eq, PartialEq, Hash)]
struct RenderSlotKey {
    mask: MaskKey,
    c1: [u32; 4],
    c2: [u32; 4],
    c3: [u32; 4],
    c4: [u32; 4],
}

impl RenderSlotKey {
    fn new(mask: MaskKey, p: Palette) -> Self {
        let bits = |c: [f32; 4]| {
            [
                c[0].to_bits(),
                c[1].to_bits(),
                c[2].to_bits(),
                c[3].to_bits(),
            ]
        };
        Self {
            mask,
            c1: bits(p.c1),
            c2: bits(p.c2),
            c3: bits(p.c3),
            c4: bits(p.c4),
        }
    }
}

struct GradientState {
    mounted_at: Instant,
    masks: HashMap<MaskKey, AlphaMask>,
    renders: HashMap<RenderSlotKey, (u64, Arc<RenderImage>)>,
}

impl GradientState {
    fn new(mounted_at: Instant) -> Self {
        Self {
            mounted_at,
            masks: HashMap::new(),
            renders: HashMap::new(),
        }
    }
}

pub struct GradientTextElement {
    props: GradientText,
    state: Entity<GradientState>,
}

impl IntoElement for GradientTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct LayoutState {
    line: ShapedLine,
    font_size: Pixels,
    line_height: Pixels,
    weight: FontWeight,
    base_color: Hsla,
}

pub struct PrepaintState {
    line: ShapedLine,
    font_size: Pixels,
    line_height: Pixels,
    weight: FontWeight,
    base_color: Hsla,
    pad: Pixels,
    base_opacity: f32,
    sweep_fraction: f32,
    palette: Palette,
    flow_phase: f32,
    static_key: Option<u64>,
}

pub(super) fn sweep_state(sweep: bool, elapsed: Duration, reduce_motion: bool) -> (f32, f32) {
    if !sweep || reduce_motion {
        return (1., 0.);
    }
    let sweep_ease = Easing::cubic_bezier(0.65, 0., 0.25, 1.).expect("valid control points");
    let sweep_t = (elapsed.as_secs_f32() / SWEEP_DURATION.as_secs_f32()).clamp(0., 1.);
    let sweep_fraction = sweep_ease.sample(sweep_t);

    let base_opacity = if elapsed <= BASE_FADE_DELAY {
        1.
    } else {
        let fade_t = ((elapsed - BASE_FADE_DELAY).as_secs_f32() / BASE_FADE_DURATION.as_secs_f32())
            .clamp(0., 1.);
        1. - Easing::Ease.sample(fade_t)
    };
    (sweep_fraction, base_opacity)
}

impl Element for GradientTextElement {
    type RequestLayoutState = LayoutState;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<Role> {
        (!self.props.text.is_empty()).then(|| self.props.as_el.role())
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        if !self.props.text.is_empty() {
            node.set_label(self.props.text.as_ref());
        }
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let theme = RiffTheme::global(cx);
        let text_refinement: &TextStyleRefinement = &self.props.style.text;
        let mut resolved = window.text_style();
        resolved.refine(text_refinement);

        let font_size = resolved.font_size.to_pixels(window.rem_size());
        let line_height = resolved
            .line_height
            .to_pixels(font_size.into(), window.rem_size());
        let weight = resolved.font_weight;
        let base_color = text_refinement.color.unwrap_or(resolved.color);
        let _ = theme;

        let mut f = gpui_kit::font(resolved.font_family);
        f.weight = weight;
        f.features = resolved.font_features;
        let run = TextRun {
            len: self.props.text.len(),
            font: f,
            color: Hsla::black(),
            ..Default::default()
        };
        let line =
            window
                .text_system()
                .shape_line(self.props.text.clone(), font_size, &[run], None);

        let mut style = gpui_kit::Style::default();
        style.refine(&self.props.style);
        let box_w = line.width.ceil().max(px(0.));
        style.size.width = box_w.into();
        style.size.height = line_height.into();
        style.flex_shrink = 0.;

        let layout_id = window.request_layout(style, [], cx);
        (
            layout_id,
            LayoutState {
                line,
                font_size,
                line_height,
                weight,
                base_color,
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = RiffTheme::global(cx);
        let palette = Palette::resolve(self.props.colors.as_deref(), theme);
        let reduce_motion = cx.reduce_motion();
        let pad = layout.font_size * CLIP_PAD_EM;

        let (sweep_fraction, base_opacity, sweep_running) = if reduce_motion {
            (1., 0., false)
        } else if let Some(at) = self.props.frozen_sweep_at {
            let (s, o) = sweep_state(self.props.sweep, at, false);
            (s, o, false)
        } else if self.props.frozen_at.is_some() || !self.props.sweep {
            (1., 0., false)
        } else {
            let mounted_at = self.state.read(cx).mounted_at;
            let elapsed = cx
                .background_executor()
                .now()
                .saturating_duration_since(mounted_at);
            let (s, o) = sweep_state(true, elapsed, false);
            let running = elapsed < BASE_FADE_DELAY + BASE_FADE_DURATION;
            (s, o, running)
        };

        let (flow_phase, flow_running, static_key) = if reduce_motion {
            (0., false, Some(0))
        } else if let Some(at) = self.props.frozen_at {
            let period = self.props.speed.as_secs_f64();
            let phase = ((at.as_secs_f64() % period) / period) as f32;
            (phase, false, Some((phase * 100_000.).round() as u64))
        } else {
            let now = cx.background_executor().now();
            let start = match cx.try_global::<FlowEpoch>() {
                Some(epoch) => epoch.0,
                None => {
                    cx.set_global(FlowEpoch(now));
                    now
                }
            };
            let elapsed = now.saturating_duration_since(start);
            let period = self.props.speed.as_secs_f64();
            let phase = ((elapsed.as_secs_f64() % period) / period) as f32;
            (phase, true, None)
        };

        if sweep_running || flow_running {
            window.request_animation_frame();
        }

        PrepaintState {
            line: layout.line.clone(),
            font_size: layout.font_size,
            line_height: layout.line_height,
            weight: layout.weight,
            base_color: layout.base_color,
            pad,
            base_opacity,
            sweep_fraction,
            palette,
            flow_phase,
            static_key,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let sf = window.scale_factor();
        let ascent = prepaint.line.ascent.round();
        let descent = prepaint.line.descent.round();
        let half_leading = ((prepaint.line_height - (ascent + descent)) / 2.).floor();
        let baseline_offset = half_leading + ascent;
        let baseline = bounds.top() + baseline_offset;
        let baseline_snapped = px((baseline.as_f32() * sf + 0.5).floor() / sf);

        if prepaint.base_opacity > 0.001 {
            let scale = Scale {
                center: bounds.center(),
                factor: 1.,
            };
            let taps = [Tap {
                offset: Default::default(),
                weight: 1.,
            }];
            paint::paint_text_tracked(
                &prepaint.line,
                point(bounds.left(), baseline_snapped),
                prepaint.font_size,
                px(0.),
                prepaint.base_color.opacity(prepaint.base_opacity),
                scale,
                &taps,
                window,
            );
        }

        if prepaint.sweep_fraction <= 0.001 || bounds.size.width <= px(0.) {
            return;
        }

        let width_px = ((bounds.size.width + prepaint.pad * 2.).as_f32() * sf)
            .ceil()
            .max(1.) as u32;
        let height_px = ((bounds.size.height + prepaint.pad * 2.).as_f32() * sf)
            .ceil()
            .max(1.) as u32;

        let desired_key = MaskKey {
            text: self.props.text.to_string(),
            font_size_bits: prepaint.font_size.as_f32().to_bits(),
            weight_bits: prepaint.weight.0.to_bits(),
            scale_factor_bits: sf.to_bits(),
            width_px,
            height_px,
        };

        let render_img = {
            let mut stale_img = None;
            let slot_key = RenderSlotKey::new(desired_key.clone(), prepaint.palette);
            let img = self.state.update(cx, |state, _| {
                if !state.masks.contains_key(&desired_key) {
                    if state.masks.len() >= 16 {
                        state.masks.clear();
                    }
                    state.masks.insert(
                        desired_key.clone(),
                        AlphaMask::build(
                            &prepaint.line,
                            prepaint.font_size,
                            prepaint.weight,
                            sf,
                            baseline_snapped - bounds.top(),
                            bounds.size.width,
                            bounds.size.height,
                            prepaint.pad,
                        ),
                    );
                }

                if let Some(sk) = prepaint.static_key
                    && let Some((cached_sk, cached_img)) = state.renders.get(&slot_key)
                    && *cached_sk == sk
                {
                    return cached_img.clone();
                }

                let mask = state.masks.get(&desired_key).expect("mask initialized");
                let field = GradientField::new(
                    mask.box_w_px,
                    mask.box_h_px,
                    prepaint.palette,
                    prepaint.flow_phase,
                );
                let img = mask.render_gradient(&field);
                if let Some((_, old)) = state.renders.insert(
                    slot_key,
                    (prepaint.static_key.unwrap_or(u64::MAX), img.clone()),
                ) {
                    stale_img = Some(old);
                }
                img
            });
            if let Some(old) = stale_img {
                let _ = window.drop_image(old);
            }
            img
        };

        let img_bounds = Bounds::new(
            point(bounds.left() - prepaint.pad, bounds.top() - prepaint.pad),
            size(px(width_px as f32 / sf), px(height_px as f32 / sf)),
        );
        let clip_right =
            bounds.left() + (bounds.size.width + prepaint.pad) * prepaint.sweep_fraction;
        let clip = ContentMask {
            bounds: Bounds::from_corners(img_bounds.origin, point(clip_right, img_bounds.bottom())),
        };

        window.with_content_mask(Some(clip), |window| {
            let _ = window.paint_image(
                img_bounds,
                img_bounds,
                Corners::all(px(0.)),
                render_img,
                0,
                false,
            );
        });
    }
}
