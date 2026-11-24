//! Colors, keyframe blobs, and CSS gradient sampling of riff-uiux's `.ui-gt`.

use std::f32::consts::PI;
use std::time::Duration;

use gpui_kit::Hsla;
use gpui_kit::base::Easing;

use crate::paint::hex;
use crate::theme::RiffTheme;

pub(super) const DEFAULT_SPEED: Duration = Duration::from_secs(8);
pub(super) const SWEEP_DURATION: Duration = Duration::from_millis(1100);
pub(super) const BASE_FADE_DELAY: Duration = Duration::from_millis(900);
pub(super) const BASE_FADE_DURATION: Duration = Duration::from_millis(240);
/// `inset(-0.25em ...)` so ascenders and overhangs stay inside the gradient box.
pub(super) const CLIP_PAD_EM: f32 = 0.25;

/// `[c1, c2, c3, c4]` resolved for light/dark or custom override.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Palette {
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub c3: [f32; 4],
    pub c4: [f32; 4],
}

fn to_srgba(color: Hsla) -> [f32; 4] {
    let rgb = color.to_rgb();
    [rgb.r, rgb.g, rgb.b, rgb.a]
}

impl Palette {
    pub fn resolve(custom: Option<&[Hsla]>, theme: &RiffTheme) -> Self {
        if let Some(colors) = custom
            && !colors.is_empty()
        {
            let c1 = colors[0];
            let c2 = colors.get(1).copied().unwrap_or(c1);
            let c3 = colors.get(2).copied().unwrap_or(c1);
            let c4 = colors.get(3).copied().unwrap_or(c2);
            return Self {
                c1: to_srgba(c1),
                c2: to_srgba(c2),
                c3: to_srgba(c3),
                c4: to_srgba(c4),
            };
        }
        if theme.mode.is_dark() {
            Self {
                c1: to_srgba(hex(0xf472b6)),
                c2: to_srgba(hex(0xa78bfa)),
                c3: to_srgba(hex(0xf9a8d4)),
                c4: to_srgba(hex(0xc4b5fd)),
            }
        } else {
            Self {
                c1: to_srgba(hex(0xdd0674)),
                c2: to_srgba(hex(0x5e3ee0)),
                c3: to_srgba(hex(0xff5fae)),
                c4: to_srgba(hex(0x7c5cff)),
            }
        }
    }
}

/// `[x_frac, y_frac]` background positions of the 3 radial blobs at keyframes 0/25/50/75/100%.
const KEYFRAMES: [[(f32, f32); 3]; 5] = [
    [(-0.20, 0.30), (1.20, 0.70), (0.50, -0.40)],
    [(0.40, 1.10), (0.60, -0.10), (1.10, 0.60)],
    [(1.20, 0.40), (-0.20, 0.40), (0.40, 1.40)],
    [(0.60, -0.30), (0.30, 1.20), (-0.10, 0.30)],
    [(-0.20, 0.30), (1.20, 0.70), (0.50, -0.40)],
];

const BLOB_SIZES: [(f32, f32); 3] = [(0.70, 1.70), (0.80, 1.90), (0.60, 1.60)];

#[derive(Clone, Copy, Debug)]
pub(super) struct Blob {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
    pub color: [f32; 4],
}

pub(super) struct GradientField {
    pub width: f32,
    pub height: f32,
    pub c1: [f32; 4],
    pub c2: [f32; 4],
    pub dir_x: f32,
    pub dir_y: f32,
    pub grad_len: f32,
    /// Blobs in bottom-to-top paint order: `[c1_blob, c4_blob, c3_blob]`.
    pub blobs: [Blob; 3],
}

impl GradientField {
    pub fn new(width: f32, height: f32, palette: Palette, phase: f32) -> Self {
        let phase = phase.rem_euclid(1.);
        let leg_pos = phase * 4.;
        let leg = (leg_pos.floor() as usize).min(3);
        let local_t = leg_pos - leg as f32;
        let eased = Easing::EaseInOut.sample(local_t);

        let blob_colors = [palette.c3, palette.c4, palette.c1];
        let mut resolved = [Blob {
            cx: 0.,
            cy: 0.,
            rx: 1.,
            ry: 1.,
            color: [0.; 4],
        }; 3];

        for i in 0..3 {
            let (x0, y0) = KEYFRAMES[leg][i];
            let (x1, y1) = KEYFRAMES[leg + 1][i];
            let px = x0 + (x1 - x0) * eased;
            let py = y0 + (y1 - y0) * eased;
            let (bw, bh) = BLOB_SIZES[i];
            let img_w = width * bw;
            let img_h = height * bh;
            // CSS `background-position: X% Y%`: `(container - image) * p + image / 2`.
            let cx = (width - img_w) * px + img_w * 0.5;
            let cy = (height - img_h) * py + img_h * 0.5;
            resolved[i] = Blob {
                cx,
                cy,
                rx: (img_w * 0.5).max(0.001),
                ry: (img_h * 0.5).max(0.001),
                color: blob_colors[i],
            };
        }

        // `linear-gradient(100deg, c1 10%, c2 90%)`
        let theta = 100. * PI / 180.;
        let dir_x = theta.sin();
        let dir_y = -theta.cos();
        let grad_len = ((width * dir_x).abs() + (height * dir_y).abs()).max(0.001);

        Self {
            width,
            height,
            c1: palette.c1,
            c2: palette.c2,
            dir_x,
            dir_y,
            grad_len,
            // Paint order is bottom-to-top, reverse of CSS layer order.
            blobs: [resolved[2], resolved[1], resolved[0]],
        }
    }

    /// Evaluates straight-alpha sRGBA `[r, g, b, a]` at text-box coordinates `(x, y)`.
    #[inline]
    pub fn sample(&self, x: f32, y: f32) -> [f32; 4] {
        let proj = (x - self.width * 0.5) * self.dir_x + (y - self.height * 0.5) * self.dir_y;
        let t = (proj / self.grad_len + 0.5).clamp(0., 1.);
        let u = ((t - 0.10) / 0.80).clamp(0., 1.);
        let mut r = self.c1[0] + (self.c2[0] - self.c1[0]) * u;
        let mut g = self.c1[1] + (self.c2[1] - self.c1[1]) * u;
        let mut b = self.c1[2] + (self.c2[2] - self.c1[2]) * u;
        let mut a = self.c1[3] + (self.c2[3] - self.c1[3]) * u;

        for blob in &self.blobs {
            let dx = (x - blob.cx) / blob.rx;
            let dy = (y - blob.cy) / blob.ry;
            let d2 = dx * dx + dy * dy;
            if d2 < 1. {
                let falloff = (1. - d2.sqrt()).clamp(0., 1.);
                let src_a = blob.color[3] * falloff;
                if src_a > 0. {
                    let inv = 1. - src_a;
                    let out_a = src_a + a * inv;
                    if out_a > 0. {
                        r = (blob.color[0] * src_a + r * a * inv) / out_a;
                        g = (blob.color[1] * src_a + g * a * inv) / out_a;
                        b = (blob.color[2] * src_a + b * a * inv) / out_a;
                        a = out_a;
                    }
                }
            }
        }
        [r, g, b, a]
    }
}
