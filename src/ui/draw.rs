use graphics::math::Matrix2d;
use gfx_graphics::{GraphicsBackEnd, GlyphCache};
use gfx_device;

use ui::Flow;

pub struct DrawCx<'a, 'b: 'a, 'c: 'a> {
    pub gfx: &'a mut GraphicsBackEnd<'b, gfx_device::Resources,
                                         gfx_device::CommandBuffer,
                                         gfx_device::Output>,
    pub glyph_cache: &'a mut GlyphCache<'c, gfx_device::Resources>,
    pub transform: Matrix2d
}

impl<'a, 'b, 'c> DrawCx<'a, 'b, 'c> {
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
