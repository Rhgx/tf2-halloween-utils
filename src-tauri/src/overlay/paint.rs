//! Drawing shared by every widget, in TF2's HUD style: the colours, TF2's own fonts with drop shadows, the panel
//! background, edit mode's outline and grip, and the help card and corner notice. Layout numbers are in 1080p
//! panel units, multiplied by `k` (screen height / 1080 times the user scale).

use fontdue::Font;
use std::path::{Path, PathBuf};
use tiny_skia::{
    Color, FillRule, LineCap, Paint, Path as SkPath, PathBuilder, Pixmap, Stroke, StrokeDash,
    Transform,
};

pub const PANEL_W: f32 = 232.0;

/// The grip square in the panel's bottom-right corner (edit mode).
pub const GRIP: f32 = 12.0;

pub type Rgb = [u8; 3];
pub const CREAM: Rgb = [0xEB, 0xE3, 0xCB];
pub const TAN_DARK: Rgb = [0x75, 0x6B, 0x5E];
/// Dimmed text (below / level): lighter than TAN_DARK so it stays readable on the panel.
pub const TAN_TEXT: Rgb = [0x9E, 0x93, 0x80];
pub const PANEL_BG: Rgb = [0x2E, 0x2B, 0x2A];
pub const ORANGE: Rgb = [0xE5, 0x86, 0x26];
pub const RIM: Rgb = [0x1A, 0x18, 0x16];
pub const BLACK: Rgb = [0, 0, 0];
pub const SHADOW_ALPHA: f32 = 0.6;

pub fn pixels(height: f32, k: f32) -> (u32, u32) {
    (
        (PANEL_W * k + 2.0 * k).ceil() as u32,
        (height * k + 2.0 * k).ceil() as u32,
    )
}

pub struct Painter {
    pub big: Font,
    pub small: Font,
    pub tf2_fonts: bool,
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Right,
}

pub fn paint(colour: Rgb, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(
        colour[0],
        colour[1],
        colour[2],
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    );
    p.anti_alias = true;
    p
}

/// Shadow offset in pixels for `k` (0 disables the shadow).
pub fn shadowed(t: Transform, k: f32) -> Option<Transform> {
    (k > 0.0).then(|| t.post_translate(k.round().max(1.0), k.round().max(1.0)))
}

pub fn fill(pm: &mut Pixmap, path: &SkPath, colour: Rgb, alpha: f32, t: Transform, k: f32) {
    if let Some(st) = shadowed(t, k) {
        pm.fill_path(
            path,
            &paint(BLACK, SHADOW_ALPHA * alpha),
            FillRule::Winding,
            st,
            None,
        );
    }
    pm.fill_path(path, &paint(colour, alpha), FillRule::Winding, t, None);
}

pub fn stroke(
    pm: &mut Pixmap,
    path: &SkPath,
    colour: Rgb,
    width: f32,
    dash: Option<StrokeDash>,
    t: Transform,
    k: f32,
) {
    let st = Stroke {
        width,
        line_cap: LineCap::Round,
        dash,
        ..Stroke::default()
    };
    if let Some(shadow_t) = shadowed(t, k) {
        pm.stroke_path(path, &paint(BLACK, SHADOW_ALPHA), &st, shadow_t, None);
    }
    pm.stroke_path(path, &paint(colour, 1.0), &st, t, None);
}

pub fn line(a: (f32, f32), b: (f32, f32)) -> SkPath {
    let mut pb = PathBuilder::new();
    pb.move_to(a.0, a.1);
    pb.line_to(b.0, b.1);
    pb.finish().unwrap()
}

pub fn polygon(points: &[(f32, f32)]) -> SkPath {
    let mut pb = PathBuilder::new();
    pb.move_to(points[0].0, points[0].1);
    for p in &points[1..] {
        pb.line_to(p.0, p.1);
    }
    pb.close();
    pb.finish().unwrap()
}

/// Edit mode's dashed outline and the resize grip in the bottom-right corner, on a panel `height` units tall.
pub fn edit_outline(pm: &mut Pixmap, height: f32, t: Transform) {
    let outline = rounded_rect(0.5, 0.5, PANEL_W - 1.0, height - 1.0, 4.0);
    stroke(
        pm,
        &outline,
        CREAM,
        1.0,
        StrokeDash::new(vec![4.0, 3.0], 0.0),
        t,
        0.0,
    );
    let grip = polygon(&[
        (PANEL_W, height - GRIP),
        (PANEL_W, height),
        (PANEL_W - GRIP, height),
    ]);
    fill(pm, &grip, CREAM, 1.0, t, 0.0);
}

pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> SkPath {
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

/// "Source over" of a glyph coverage mask in a solid colour onto the premultiplied pixmap.
#[allow(clippy::too_many_arguments)]
pub fn blend_coverage(
    pm: &mut Pixmap,
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    coverage: &[u8],
    colour: Rgb,
    alpha: f32,
) {
    let (pw, ph) = (pm.width() as i32, pm.height() as i32);
    let data = pm.data_mut();
    for gy in 0..h as i32 {
        let y = y0 + gy;
        if y < 0 || y >= ph {
            continue;
        }
        for gx in 0..w as i32 {
            let x = x0 + gx;
            let a = (coverage[(gy * w as i32 + gx) as usize] as f32 * alpha) as u32;
            if x < 0 || x >= pw || a == 0 {
                continue;
            }
            let i = ((y * pw + x) * 4) as usize;
            for c in 0..3 {
                data[i + c] = ((colour[c] as u32 * a + data[i + c] as u32 * (255 - a)) / 255) as u8;
            }
            data[i + 3] = (a + data[i + 3] as u32 * (255 - a) / 255) as u8;
        }
    }
}

impl Painter {
    /// TF2's own HUD fonts from `tf\resource` when available, otherwise Segoe UI / Arial.
    pub fn load(tf_dir: Option<&Path>) -> Result<Painter, String> {
        let res = tf_dir.map(|d| d.join("resource"));
        let load = |candidates: &[Option<PathBuf>]| -> Option<(Font, bool)> {
            candidates.iter().enumerate().find_map(|(i, p)| {
                let bytes = std::fs::read(p.as_ref()?).ok()?;
                Font::from_bytes(bytes, fontdue::FontSettings::default())
                    .ok()
                    .map(|f| (f, i == 0))
            })
        };
        let win = |name: &str| Some(PathBuf::from(r"C:\Windows\Fonts").join(name));
        let big = load(&[
            res.as_ref().map(|r| r.join("tf2build.ttf")),
            win("segoeuib.ttf"),
            win("arialbd.ttf"),
        ]);
        let small = load(&[
            res.as_ref().map(|r| r.join("tf2secondary.ttf")),
            win("segoeui.ttf"),
            win("arial.ttf"),
        ]);
        match (big, small) {
            (Some((big, a)), Some((small, b))) => Ok(Painter {
                big,
                small,
                tf2_fonts: a && b,
            }),
            _ => Err("no usable font found (TF2's resource folder or C:\\Windows\\Fonts)".into()),
        }
    }

    /// Help card shown under the panel in edit mode.
    /// `save_key` names the hotkey that saves and leaves edit mode.
    pub fn help_card(&self, k: f32, opacity: f32, save_key: &str) -> Pixmap {
        let last = format!(
            "Arrows nudge (Shift: 10 px), + / - resize, PgUp / PgDn opacity, Home resets, {save_key} saves."
        );
        let lines = [
            "EDIT MODE",
            "Press Esc in TF2 to free the mouse. Drag to move, drag the corner to resize.",
            last.as_str(),
        ];
        let px = 14.0 * k;
        let width = lines
            .iter()
            .map(|l| self.measure(&self.small, l, px))
            .fold(0.0, f32::max);
        let (pad, line_h) = (10.0 * k, 20.0 * k);
        let (w, h) = (
            (width + 2.0 * pad + 2.0 * k).ceil() as u32,
            (lines.len() as f32 * line_h + 2.0 * pad).ceil() as u32,
        );
        let mut pm = Pixmap::new(w, h).unwrap();
        fill(
            &mut pm,
            &rounded_rect(0.0, 0.0, w as f32 - 2.0 * k, h as f32 - 2.0 * k, 4.0 * k),
            PANEL_BG,
            opacity,
            Transform::identity(),
            k,
        );
        for (i, line) in lines.iter().enumerate() {
            let colour = if i == 0 { CREAM } else { [0xC8, 0xC0, 0xAA] };
            self.text(
                &mut pm,
                &self.small,
                pad,
                pad + (i as f32 + 0.75) * line_h,
                line,
                px,
                colour,
                Align::Left,
                k,
            );
        }
        pm
    }

    /// Status text anchored at the bottom-right of the pixmap.
    pub fn corner(&self, pm: &mut Pixmap, text: &str, k: f32) {
        pm.fill(Color::TRANSPARENT);
        let (w, h) = (pm.width() as f32, pm.height() as f32);
        self.text(
            pm,
            &self.small,
            w - 3.0 * k,
            h - 8.0 * k,
            text,
            15.0 * k,
            CREAM,
            Align::Right,
            k,
        );
    }

    pub fn measure(&self, font: &Font, text: &str, px: f32) -> f32 {
        text.chars()
            .map(|c| font.metrics(c, px).advance_width)
            .sum()
    }

    /// Text on a baseline, with TF2's drop shadow (the same text in black, offset down-right).
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &self,
        pm: &mut Pixmap,
        font: &Font,
        x: f32,
        baseline: f32,
        text: &str,
        px: f32,
        colour: Rgb,
        align: Align,
        k: f32,
    ) {
        let px = px.max(8.0);
        let glyphs: Vec<_> = text.chars().map(|c| font.rasterize(c, px)).collect();
        let width: f32 = glyphs.iter().map(|(m, _)| m.advance_width).sum();
        let left = match align {
            Align::Left => x,
            Align::Right => x - width,
        };
        let shadow = k.round().max(1.0) as i32;
        for (dx, c, alpha) in [(shadow, BLACK, SHADOW_ALPHA), (0, colour, 1.0)] {
            let mut pen = left;
            for (m, bitmap) in &glyphs {
                let gx = (pen + m.xmin as f32).round() as i32 + dx;
                let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32 + dx;
                blend_coverage(pm, gx, gy, m.width, m.height, bitmap, c, alpha);
                pen += m.advance_width;
            }
        }
    }
}

#[cfg(test)]
pub mod preview {
    use super::*;

    /// TF2's own fonts for the preview tests, from the default Steam library.
    pub fn painter() -> Painter {
        let tf = PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf");
        Painter::load(Some(&tf)).unwrap()
    }

    /// Writes same-sized tiles side by side onto a dark and a bright strip, as a BMP in the temp folder.
    pub fn save_strip(name: &str, tiles: &[Pixmap]) {
        let (pw, ph) = (tiles[0].width() as usize, tiles[0].height() as usize);
        let (tile_w, tile_h) = (pw + 20, ph + 20);
        let (out_w, out_h) = (tile_w * tiles.len(), tile_h * 2);
        let mut out = vec![[0u8; 3]; out_w * out_h];
        for (row, bg) in [[32u8, 30, 28], [176, 168, 144]].iter().enumerate() {
            for (i, pm) in tiles.iter().enumerate() {
                for y in 0..tile_h {
                    for x in 0..tile_w {
                        let mut c = *bg;
                        if (10..10 + ph).contains(&y) && (10..10 + pw).contains(&x) {
                            let p = &pm.data()[((y - 10) * pw + x - 10) * 4..][..4];
                            for ch in 0..3 {
                                c[ch] =
                                    (p[ch] as u32 + c[ch] as u32 * (255 - p[3] as u32) / 255) as u8;
                            }
                        }
                        out[(row * tile_h + y) * out_w + i * tile_w + x] = c;
                    }
                }
            }
        }
        let stride = (out_w * 3 + 3) & !3;
        let mut body = Vec::new();
        for y in (0..out_h).rev() {
            for p in &out[y * out_w..(y + 1) * out_w] {
                body.extend_from_slice(&[p[2], p[1], p[0]]);
            }
            body.resize(body.len() + stride - out_w * 3, 0);
        }
        let mut bmp = b"BM".to_vec();
        for v in [
            54 + body.len() as u32,
            0,
            54,
            40,
            out_w as u32,
            out_h as u32,
        ] {
            bmp.extend_from_slice(&v.to_le_bytes());
        }
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        for v in [0u32, body.len() as u32, 2835, 2835, 0, 0] {
            bmp.extend_from_slice(&v.to_le_bytes());
        }
        bmp.extend_from_slice(&body);
        std::fs::write(std::env::temp_dir().join(name), bmp).unwrap();
    }
}
