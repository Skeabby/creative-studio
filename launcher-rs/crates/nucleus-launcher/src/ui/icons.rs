//! App icon rasterisation: loads each app's `icon.svg`, renders it with
//! resvg and caches the resulting `egui::TextureHandle` keyed by app id.

use std::collections::HashMap;
use std::path::Path;

use egui::{Context, TextureHandle};

/// The Nucleus app icon, embedded so the window icon never depends on files
/// next to the executable.
pub const BRAND_SVG: &[u8] = include_bytes!("../../assets/brand/nucleus.svg");

/// The bare Nucleus mark (transparent plate) used in the header, where the
/// plated app icon would read as a dark slab.
pub const MARK_SVG: &[u8] = include_bytes!("../../assets/brand/mark.svg");

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
        let bytes = std::fs::read(path).ok()?;
        self.load_bytes(ctx, key, &bytes, size)
    }

    /// Render SVG bytes (e.g. the embedded brand mark) into a cached texture.
    pub fn load_bytes(
        &mut self,
        ctx: &Context,
        key: &str,
        bytes: &[u8],
        size: f32,
    ) -> Option<&TextureHandle> {
        if self.cache.contains_key(key) {
            return self.cache.get(key);
        }
        let pixels = size * ctx.pixels_per_point();
        let rendered = render_svg(bytes, pixels).map(|(width, height, rgba)| {
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
pub fn render_svg(data: &[u8], px: f32) -> Option<(usize, usize, Vec<u8>)> {
    use resvg::{tiny_skia, usvg};

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &opt).ok()?;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn apps_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../apps")
    }

    #[test]
    fn brand_tile_fills_its_square() {
        let (w, h, rgba) = render_svg(BRAND_SVG, 64.0).expect("brand svg must render");
        assert_eq!((w, h), (64, 64));
        assert_eq!(rgba.len(), 64 * 64 * 4);
        // The app tile is a plated logo: it covers its square, corners included
        // in the rounded plate only. So most pixels are opaque, but the four
        // corners stay transparent.
        let opaque = rgba.chunks(4).filter(|p| p[3] > 200).count();
        let corner = |x: usize, y: usize| rgba[(y * 64 + x) * 4 + 3];
        assert!(
            opaque > (64 * 64) * 9 / 10,
            "the plated brand tile must cover most of its square, got {opaque}"
        );
        let corners = [corner(0, 0), corner(63, 0), corner(0, 63), corner(63, 63)];
        assert!(
            corners.iter().all(|alpha| *alpha < 40),
            "the plate corners must stay transparent (alpha), got {corners:?}"
        );
    }

    #[test]
    fn header_mark_leaves_transparent_margins() {
        let (w, h, rgba) = render_svg(MARK_SVG, 64.0).expect("mark svg must render");
        assert_eq!((w, h), (64, 64));
        // The header mark is drawn straight onto the shell, so the artwork must
        // not cover the square: it has to stay a mark, not a slab.
        let opaque = rgba.chunks(4).filter(|p| p[3] > 200).count();
        assert!(opaque > 0, "the mark must paint something");
        assert!(
            opaque < (64 * 64) / 2,
            "the mark must leave transparent margins"
        );
    }

    #[test]
    fn every_app_icon_renders() {
        let dir = apps_dir();
        if !dir.is_dir() {
            return; // running outside the pack checkout
        }
        for entry in std::fs::read_dir(&dir).expect("apps dir") {
            let path = entry.expect("entry").path().join("icon.svg");
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("read icon");
            let (w, h, rgba) = render_svg(&bytes, 56.0).unwrap_or_else(|| panic!("{path:?}"));
            assert!(w > 0 && h > 0, "{path:?} rendered empty");
            assert_eq!(rgba.len(), w * h * 4, "{path:?} buffer size");
            let visible = rgba.chunks(4).filter(|p| p[3] > 8).count();
            assert!(visible > 0, "{path:?} renders fully transparent");
        }
    }

    #[test]
    fn garbage_svg_is_rejected_not_panicking() {
        assert!(render_svg(b"not an svg", 32.0).is_none());
    }
}
