use std::cell::Cell;
use std::default::Default;

use gfx::{Color, GlyphCache, FontSize};
use graphics::character::CharacterCache;

use ui::Px;
use ui::draw::{Draw, DrawCx};
use ui::layout::{RectBounded, RectBB, ConstrainCx, Layout};

// TODO use a text layouting engine

pub trait FontFace {
    fn size(&self) -> FontSize { 16 }
    fn cache<'a>(&self, fonts: &'a mut FontFaces) -> &'a mut GlyphCache;
}

macro_rules! font_faces {
    ($($ty:ident => $name:ident),+) => {
        pub struct FontFaces {
            $(pub $name: GlyphCache),*
        }
        $(#[derive(Default)] pub struct $ty; impl FontFace for $ty {
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
        let w = self.font.cache(&mut cx.fonts).width(size, self.text) as Px;
        cx.distance(bb.x1, bb.x2, w);
        cx.distance(bb.y1, bb.y2, size as Px);
    }
}

impl<F> Draw for Label<F> where F: FontFace {
    fn draw(&self, cx: &mut DrawCx) {
        use graphics::*;

        let bb = self.bb();
        text::Text::colored(self.color.get(), self.font.size()).draw(
            self.text,
            self.font.cache(&mut cx.fonts),
            default_draw_state(),
            cx.transform.trans(bb.x1 as f64, bb.y2 as f64),
            cx.gfx
        );
    }
}
