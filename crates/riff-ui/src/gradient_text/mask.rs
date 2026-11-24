//! Glyph alpha-mask rasterization via `swash` and BGRA `RenderImage`
//! generation for CSS `background-clip: text`.

use std::sync::Arc;

use gpui_kit::{FontWeight, Pixels, RenderImage, ShapedLine};
use image::Frame;
use smallvec::SmallVec;
use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source, image::Image as SwashImage};
use swash::zeno::{Format, Vector};

use super::style::GradientField;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(super) struct MaskKey {
    pub text: String,
    pub font_size_bits: u32,
    pub weight_bits: u32,
    pub scale_factor_bits: u32,
    pub width_px: u32,
    pub height_px: u32,
}

pub(super) struct AlphaMask {
    pub width_px: u32,
    pub height_px: u32,
    pub pad_px: f32,
    pub box_w_px: f32,
    pub box_h_px: f32,
    pub alpha: Vec<u8>,
}

impl AlphaMask {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        line: &ShapedLine,
        font_size: Pixels,
        weight: FontWeight,
        scale_factor: f32,
        baseline: Pixels,
        box_w: Pixels,
        box_h: Pixels,
        pad: Pixels,
    ) -> Self {
        let pad_px = pad.as_f32() * scale_factor;
        let box_w_px = (box_w.as_f32() * scale_factor).max(1.);
        let box_h_px = (box_h.as_f32() * scale_factor).max(1.);
        let width_px = (box_w_px + pad_px * 2.).ceil().max(1.) as u32;
        let height_px = (box_h_px + pad_px * 2.).ceil().max(1.) as u32;

        let mut alpha = vec![0u8; (width_px * height_px) as usize];
        let font_bytes = crate::inter_font_bytes(weight);
        if let Some(font) = FontRef::from_index(font_bytes, 0) {
            let mut ctx = ScaleContext::new();
            let mut scaler = ctx
                .builder(font)
                .size(font_size.as_f32() * scale_factor)
                .hint(true)
                .build();
            let mut glyph_img = SwashImage::new();
            let sources = [Source::Outline];

            for run in &line.runs {
                for glyph in &run.glyphs {
                    let gid = u16::try_from(glyph.id.0).unwrap_or(0);
                    let gx = pad_px + glyph.position.x.as_f32() * scale_factor;
                    let gy = pad_px + (baseline + glyph.position.y).as_f32() * scale_factor;
                    let fx = gx - gx.floor();
                    let fy = -(gy - gy.floor());

                    glyph_img.clear();
                    let ok = Render::new(&sources)
                        .format(Format::Alpha)
                        .offset(Vector::new(fx, fy))
                        .render_into(&mut scaler, gid, &mut glyph_img);
                    if !ok || glyph_img.placement.width == 0 || glyph_img.placement.height == 0 {
                        continue;
                    }

                    let left = gx.floor() as i32 + glyph_img.placement.left;
                    let top = gy.floor() as i32 - glyph_img.placement.top;
                    let gw = glyph_img.placement.width as i32;
                    let gh = glyph_img.placement.height as i32;

                    for row in 0..gh {
                        let dy = top + row;
                        if dy < 0 || dy >= height_px as i32 {
                            continue;
                        }
                        for col in 0..gw {
                            let dx = left + col;
                            if dx < 0 || dx >= width_px as i32 {
                                continue;
                            }
                            let src = glyph_img.data[(row * gw + col) as usize];
                            if src == 0 {
                                continue;
                            }
                            let idx = (dy as u32 * width_px + dx as u32) as usize;
                            let dst = alpha[idx];
                            // Porter-Duff over for scalar coverage: 1 - (1 - a)(1 - b).
                            let combined =
                                255 - ((255 - u16::from(src)) * (255 - u16::from(dst)) / 255);
                            alpha[idx] = combined as u8;
                        }
                    }
                }
            }
        }

        Self {
            width_px,
            height_px,
            pad_px,
            box_w_px,
            box_h_px,
            alpha,
        }
    }

    pub fn render_gradient(&self, field: &GradientField) -> Arc<RenderImage> {
        let mut bgra = vec![0u8; (self.width_px * self.height_px * 4) as usize];
        for py in 0..self.height_px {
            let y = (py as f32 + 0.5) - self.pad_px;
            let row_off = (py * self.width_px) as usize;
            for px_x in 0..self.width_px {
                let cov = self.alpha[row_off + px_x as usize];
                if cov == 0 {
                    continue;
                }
                let x = (px_x as f32 + 0.5) - self.pad_px;
                let [r, g, b, a] = field.sample(x, y);
                let out_a = ((a * f32::from(cov)).clamp(0., 255.)).round() as u8;
                let p = (row_off + px_x as usize) * 4;
                bgra[p] = (b.clamp(0., 1.) * 255.).round() as u8;
                bgra[p + 1] = (g.clamp(0., 1.) * 255.).round() as u8;
                bgra[p + 2] = (r.clamp(0., 1.) * 255.).round() as u8;
                bgra[p + 3] = out_a;
            }
        }
        let rgba = image::RgbaImage::from_raw(self.width_px, self.height_px, bgra)
            .expect("exact BGRA buffer size");
        Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(rgba), 1)))
    }
}
