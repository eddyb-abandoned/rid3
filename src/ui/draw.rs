use {gfx, graphics};

use ui::{BB, Px};
use ui::text;

pub struct DrawCx<'a> {
    gfx: gfx::BackEnd<'a>,
    pub fonts: &'a mut text::FontFaces,
    pub cursor: gfx::MouseCursor,
    transform: gfx::Mat2,
    overlay_requested: bool,
    overlay_drawing: bool,
    inside_overlay: bool
}

fn gamma_pre_correct([r, g, b, a]: gfx::Color) -> gfx::Color {
    fn ch(x: f32) -> f32 { ((x + 0.055) / 1.055).powf(2.4) }
    [ch(r), ch(g), ch(b), a]
}

impl<'a> DrawCx<'a> {
    pub fn new(g2d: &'a mut gfx::G2d,
               surface: &'a mut gfx::Surface,
               viewport: gfx::Viewport,
               fonts: &'a mut text::FontFaces) -> DrawCx<'a> {
        DrawCx {
            gfx: gfx::BackEnd::new(g2d, surface),
            fonts: fonts,
            cursor: gfx::MouseCursor::Default,
            transform: graphics::Context::new_viewport(viewport).transform,
            overlay_requested: false,
            overlay_drawing: false,
            inside_overlay: false
        }
    }

    pub fn draw<T: Draw>(&mut self, x: &T) {
        x.draw(self);
        if self.overlay_requested {
            self.overlay_drawing = true;
            x.draw(self);
            self.overlay_drawing = false;
            self.overlay_requested = false;
        }
    }

    pub fn draw_overlay<F, T>(&mut self, f: F) -> T where F: FnOnce(&mut DrawCx) -> T {
        assert!(!self.inside_overlay);
        self.inside_overlay = true;
        let r = f(self);
        self.inside_overlay = false;
        self.overlay_requested = true;
        r
    }

    pub fn with_gfx<F, T>(&mut self, f: F) -> Option<T> where F: FnOnce(&mut gfx::BackEnd) -> T {
        if self.inside_overlay == self.overlay_drawing {
            Some(f(&mut self.gfx))
        } else {
            None
        }
    }

    pub fn clear(&mut self, color: gfx::Color) {
        self.with_gfx(|gfx| graphics::clear(gamma_pre_correct(color), gfx));
    }

    pub fn rect(&mut self, bb: BB<Px>, color: gfx::Color) {
        let transform = self.transform;
        self.with_gfx(|gfx| {
            graphics::rectangle(gamma_pre_correct(color),
                [bb.x1 as f64, bb.y1 as f64,
                 (bb.x2 - bb.x1)  as f64, (bb.y2 - bb.y1)  as f64],
                transform, gfx);
        });
    }

    pub fn text<F: text::FontFace>(&mut self, font: F, [x, y]: [Px; 2], color: gfx::Color, text: &str) {
        use graphics::*;

        if self.inside_overlay != self.overlay_drawing {
            return;
        }

        let cache = font.cache(self.fonts);
        let y = y + cache.metrics(font.size()).baseline;
        text::Text::colored(gamma_pre_correct(color), font.size()).draw(
            text,
            cache,
            default_draw_state(),
            self.transform.trans(x as f64, y as f64),
            &mut self.gfx
        );
    }
}

pub trait Draw {
    fn draw(&self, cx: &mut DrawCx);
}

impl<A, B> Draw for (A, B) where
        A: Draw,
        B: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.0.draw(cx);
        self.1.draw(cx);
    }
}
