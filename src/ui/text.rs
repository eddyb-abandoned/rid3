use std::cell::Cell;
use std::default::Default;

use glyph::{FontSize, Glyph, GlyphMetrics, GlyphCache};

use ui::Px;
use ui::color::Color;
use ui::draw::{Draw, DrawCx};
use ui::layout::{RectBounded, RectBB, ConstrainCx, Layout};

// TODO use a text layouting engine

pub trait FontFace: Copy {
    fn size(&self) -> FontSize { 14 }
    fn cache<'a>(&self, fonts: &'a mut FontFaces) -> &'a mut GlyphCache;
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
    Mono => mono,
    MonoBold => mono_bold
}

impl FontFaces {
    pub fn metrics<F: FontFace>(&mut self, font: F) -> GlyphMetrics {
        font.cache(self).metrics(font.size())
    }

    pub fn glyph<F: FontFace>(&mut self, font: F, c: char) -> &Glyph {
        font.cache(self).glyph(font.size(), c)
    }

    pub fn text_width<F: FontFace>(&mut self, font: F, text: &str) -> Px {
        let cache = font.cache(self);
        text.chars().map(|c| cache.glyph(font.size(), c).advance).sum()
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
        let (w, h) = {
            let fonts = cx.fonts();
            (fonts.text_width(self.font, self.text), fonts.metrics(self.font).height)
        };
        cx.distance(bb.x1, bb.x2, w);
        cx.distance(bb.y1, bb.y2, h);
    }
}

impl<F> Draw for Label<F> where F: FontFace {
    fn draw(&self, cx: &mut DrawCx) {
        let bb = self.bb();
        cx.text(self.font, [bb.x1, bb.y1], self.color.get(), self.text);
    }
}
