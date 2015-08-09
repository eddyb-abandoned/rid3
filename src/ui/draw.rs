use std::f32::consts::FRAC_PI_2 as QUARTER_TAU;

use glium::Texture;
use glium::Surface as SurfaceTrait;

use ui::{BB, Px};
use ui::color::Color;
use ui::render::{Renderer, Surface, Buffer, XYAndUV};
use ui::text::{FontFace, FontFaces};

pub use glium::glutin::MouseCursor;

pub trait Draw {
    fn draw(&self, _cx: &mut DrawCx) {}
}

impl<A, B> Draw for (A, B) where
        A: Draw,
        B: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.0.draw(cx);
        self.1.draw(cx);
    }
}

pub struct DrawCx<'a> {
    renderer: &'a mut Renderer,
    surface: Surface,
    cursor: MouseCursor,
    overlay_requested: bool,
    overlay_drawing: bool,
    inside_overlay: bool
}

impl<'a> DrawCx<'a> {
    pub fn new(renderer: &'a mut Renderer, surface: Surface) -> DrawCx<'a> {
        DrawCx {
            renderer: renderer,
            surface: surface,
            cursor: MouseCursor::Default,
            overlay_requested: false,
            overlay_drawing: false,
            inside_overlay: false
        }
    }

    pub fn fonts(&mut self) -> &mut FontFaces {
        &mut self.renderer.fonts
    }

    pub fn dimensions(&mut self) -> [Px; 2] {
        let (w, h) = self.surface.get_dimensions();
        [w as Px, h as Px]
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

    pub fn with_surface<F, T>(&mut self, f: F) -> Option<T> where F: FnOnce(&mut Self) -> T {
        if self.inside_overlay == self.overlay_drawing {
            Some(f(self))
        } else {
            None
        }
    }

    pub fn get_cursor(&self) -> MouseCursor {
        self.cursor
    }

    pub fn cursor(&mut self, cursor: MouseCursor) {
        self.with_surface(|this| this.cursor = cursor);
    }

    pub fn clear(&mut self, color: Color) {
        self.renderer.clear(&mut self.surface, color);
    }

    // TODO make DrawCx linear to ensure this method gets called.
    pub fn finish(self) {
        self.surface.finish().unwrap();
    }

    pub fn fill(&mut self, bb: BB<Px>, color: Color/*, corner_radius: Px*/) {
        let corner_radius: Px = 0.0;
        self.with_surface(|this| this.renderer.colored(&mut this.surface, color, |buffer| {
            if corner_radius == 0.0 {
                buffer.rect(bb);
            } else {
                let resolution = (QUARTER_TAU * corner_radius).ceil() as u32;
                buffer.rect_round(bb, resolution, corner_radius);
            }
        }));
    }

    pub fn border(&mut self, bb: BB<Px>, color: Color, border_size: Px, corner_radius: Px) {
        self.with_surface(|this| this.renderer.colored(&mut this.surface, color, |buffer| {
            if corner_radius == 0.0 {
                buffer.rect_border(bb, border_size);
            } else {
                let resolution = (QUARTER_TAU * corner_radius).ceil() as u32;
                buffer.rect_border_round(bb, border_size, resolution, corner_radius);
            }
        }));
    }

    pub fn text<F: FontFace>(&mut self, font: F, [x, y]: [Px; 2], color: Color, text: &str) {
        self.with_surface(|this| {
            let (mut x, y) = (x, y + this.fonts().metrics(font).baseline);

            // TODO use graphemes and maybe harfbuzz.
            for ch in text.chars() {
                let glyph = this.fonts().glyph(font, ch).clone();
                let texture = &glyph.texture;
                let w = texture.get_width();
                let h = texture.get_height().unwrap();

                let [dx, dy] = glyph.offset;
                let xy = BB::rect(x + dx, y + dy, w as Px, h as Px);
                let uv = BB::rect(0.0, 1.0, 1.0, -1.0);
                this.renderer.textured(&mut this.surface, color, texture, |buffer| {
                    buffer.rect(xy.zip(uv).map(|(a, b)| XYAndUV(a, b)))
                });
                x += glyph.advance;
            }
        });
    }
}
