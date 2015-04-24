use std::cell::Cell;
use std::default::Default;

use glyph::Metrics;
use gfx::{Color, GlyphCache, FontSize};
use graphics::character::CharacterCache;

use ui::Px;
use ui::draw::{Draw, DrawCx};
use ui::layout::{RectBounded, RectBB, ConstrainCx, Layout};

// TODO use a text layouting engine

pub trait FontFace {
    fn size(&self) -> FontSize { 10 }
    fn cache<'a>(&self, fonts: &'a mut FontFaces) -> &'a mut GlyphCache;
    fn draw(&self, cx: &mut DrawCx, pos: [Px; 2], color: Color, text: &str) {
        use graphics::*;

        let [x, y] = pos;
        let cache = self.cache(&mut cx.fonts);
        let y = y + cache.metrics(self.size()).baseline;
        text::Text::colored(color, self.size()).draw(
            text,
            cache,
            default_draw_state(),
            cx.transform.trans(x as f64, y as f64),
            cx.gfx
        );
    }
}

macro_rules! font_faces {
    ($($ty:ident => $name:ident),+) => {
        pub struct FontFaces {
            $(pub $name: GlyphCache),*
        }
        $(#[derive(Copy, Clone, Default)] pub struct $ty; impl FontFace for $ty {
            fn cache<'a>(&self, fonts: &'a mut FontFaces) -> &'a mut GlyphCache {
                &mut fonts.$name
            }
        })*
    }
}

font_faces! {
    Regular => regular,
    Mono => mono
}

impl FontFaces {
    pub fn metrics<F>(&mut self, font: F) -> Metrics where F: FontFace {
        font.cache(self).metrics(font.size())
    }
}

pub struct Label<F=Regular> {
    bb: RectBB,
    font: F,
    pub color: Cell<Color>,
    pub text: &'static str
}

impl<F> Label<F> where F: Default {
    pub fn new(color: Color, text: &'static str) -> Label<F> {
        Label {
            bb: RectBB::default(),
            font: F::default(),
            color: Cell::new(color),
            text: text
        }
    }
}

impl<F> RectBounded for Label<F> where F: FontFace {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { self.text }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let size = self.font.size();
        let (w, h) = {
            let cache = self.font.cache(&mut cx.fonts);
            (cache.width(size, self.text) as Px, cache.metrics(size).height)
        };
        cx.distance(bb.x1, bb.x2, w);
        cx.distance(bb.y1, bb.y2, h);
    }
}

impl<F> Draw for Label<F> where F: FontFace {
    fn draw(&self, cx: &mut DrawCx) {
        let bb = self.bb();
        self.font.draw(cx, [bb.x1, bb.y1], self.color.get(), self.text);
    }
}
