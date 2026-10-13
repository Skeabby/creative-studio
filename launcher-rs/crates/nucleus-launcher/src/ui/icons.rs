//! App icon rasterisation: loads each app's `icon.svg`, renders it with
//! resvg and caches the resulting `egui::TextureHandle` keyed by app id.

use std::collections::HashMap;
use std::path::Path;

use egui::{Context, TextureHandle};

/// Rasterises and caches app icons as GPU textures.
pub struct IconStore {
    cache: HashMap<String, TextureHandle>,
}

impl IconStore {
    pub fn new() -> Self {
        IconStore {
            cache: HashMap::new(),
        }
    }

    /// Render `path` (an SVG file) into a texture at `size` logical points.
    /// Returns `None` when the file is unreadable or not parseable — callers
    /// fall back to a letter tile.
    pub fn load(
        &mut self,
        ctx: &Context,
        key: &str,
        path: &Path,
        size: f32,
    ) -> Option<&TextureHandle> {
        if self.cache.contains_key(key) {
            return self.cache.get(key);
        }
        let pixels = size * ctx.pixels_per_point();
        let rendered = render_svg(path, pixels).map(|(width, height, rgba)| {
            let color_image = egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba);
            ctx.load_texture(
                format!("nucleus-icon:{key}"),
                color_image,
                egui::TextureOptions::LINEAR,
            )
        });
        if let Some(texture) = rendered {
            self.cache.insert(key.to_string(), texture);
        }
        self.cache.get(key)
    }
}

/// Render an SVG to RGBA (unmultiplied) at most `px` pixels on its long side,
/// returning the real pixel dimensions alongside the bytes.
fn render_svg(path: &Path, px: f32) -> Option<(usize, usize, Vec<u8>)> {
    use resvg::{tiny_skia, usvg};

    let data = std::fs::read(path).ok()?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &opt).ok()?;

    let size = tree.size();
    if size.width() <= 0.0 || size.height() <= 0.0 {
        return None;
    }
    // Fit the SVG's viewBox into the target square, preserving aspect ratio.
    let scale = (px / size.width()).min(px / size.height());
    let w = (size.width() * scale).round() as u32;
    let h = (size.height() * scale).round() as u32;
    if w == 0 || h == 0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny-skia works in premultiplied RGBA; egui wants unmultiplied.
    Some((w as usize, h as usize, unpremultiply(pixmap.data())))
}

fn unpremultiply(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks(4) {
        let a = chunk[3];
        let (r, g, b) = if a == 0 {
            (0, 0, 0)
        } else {
            let a_f = a as f32 / 255.0;
            (
                (chunk[0] as f32 / a_f).round().clamp(0.0, 255.0) as u8,
                (chunk[1] as f32 / a_f).round().clamp(0.0, 255.0) as u8,
                (chunk[2] as f32 / a_f).round().clamp(0.0, 255.0) as u8,
            )
        };
        out.extend_from_slice(&[r, g, b, a]);
    }
    out
}
