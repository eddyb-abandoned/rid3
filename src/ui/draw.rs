use gfx;

use ui::{BB, Px};
use ui::text;

pub struct DrawCx<'a, 'b: 'a> {
    pub gfx: &'a mut gfx::BackEnd<'b>,
    pub fonts: &'a mut text::FontFaces,
    pub cursor: gfx::MouseCursor,
    pub transform: gfx::Mat2
}

impl<'a, 'b> DrawCx<'a, 'b> {
    pub fn draw<T: Draw>(&mut self, x: &T) {
        x.draw(self);
    }

    pub fn rect(&mut self, bb: BB<Px>, color: gfx::Color) {
        use graphics::rectangle;

        rectangle(color,
                  [bb.x1 as f64, bb.y1 as f64, (bb.x2 - bb.x1)  as f64, (bb.y2 - bb.y1)  as f64],
                  self.transform, self.gfx);
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
