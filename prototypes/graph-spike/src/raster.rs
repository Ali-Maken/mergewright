//! Minimal RGBA rasterizer: transparent canvas, thick lines, filled discs.
//! No anti-aliasing — good enough to judge alignment; ADR-0012 notes what
//! the production renderer needs beyond this.

pub struct Canvas {
    pub w: u32,
    pub h: u32,
    px: Vec<u8>,
}

impl Canvas {
    /// A fully transparent canvas (ADR-0003: the terminal's own background
    /// shows through the graph).
    pub fn new(w: u32, h: u32) -> Self {
        Canvas {
            w,
            h,
            px: vec![0; w as usize * h as usize * 4],
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.px
    }

    /// Opaque background fill, used only by `--snapshot` (a PNG on disk has
    /// no terminal behind it to show through the transparency).
    pub fn fill(&mut self, color: [u8; 4]) {
        let (pixels, _) = self.px.as_chunks_mut::<4>();
        for p in pixels {
            *p = color;
        }
    }

    fn put(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as usize * self.w as usize + x as usize) * 4;
        self.px[i..i + 4].copy_from_slice(&color);
    }

    /// Filled disc. Drawing overwrites rather than blends, so a transparent
    /// colour punches a hole — that's how merge nodes become rings.
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, color: [u8; 4]) {
        let (x0, x1) = ((cx - r).floor() as i32, (cx + r).ceil() as i32);
        let (y0, y1) = ((cy - r).floor() as i32, (cy + r).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.put(x, y, color);
                }
            }
        }
    }

    /// Thick line: stamp a disc at every ~1px step along the segment.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: [u8; 4]) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as u32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            self.disc(x0 + dx * t, y0 + dy * t, thickness / 2.0, color);
        }
    }

    pub fn png(&self) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, self.w, self.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(std::io::Error::other)?;
        writer
            .write_image_data(&self.px)
            .map_err(std::io::Error::other)?;
        writer.finish().map_err(std::io::Error::other)?;
        Ok(out)
    }
}
