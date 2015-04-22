use gfx;

use ui::Flow;
use ui::text;

pub struct DrawCx<'a, 'b: 'a> {
    pub gfx: &'a mut gfx::BackEnd<'b>,
    pub fonts: &'a mut text::FontFaces,
    pub transform: gfx::Mat2
}

impl<'a, 'b> DrawCx<'a, 'b> {
    pub fn draw<T: Draw>(&mut self, x: &T) {
        x.draw(self);
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

impl<D, K> Draw for Flow<D, K> where K: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.kids.draw(cx);
    }
}
