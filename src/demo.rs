use std::cell::Cell;
use std::default::Default;

use ui::layout::{RectBB, RectBounded, Hit};
use ui::draw::{Draw, DrawCx};
use ui::event::{MouseDown, MouseUp};

pub struct Demo {
    pub bb: RectBB,
    pub color: [f32; 3],
    pub down: Cell<bool>,
    pub name: &'static str
}

impl Demo {
    pub fn new(color: [f32; 3], name: &'static str) -> Demo {
        Demo {
            bb: RectBB::default(),
            color: color,
            down: Cell::new(false),
            name: name
        }
    }
}

impl RectBounded for Demo {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { self.name }
}

impl Draw for Demo {
    fn draw(&self, cx: &mut DrawCx) {
        use graphics::*;

        fn invert_rgb([r, g, b]: [f32; 3]) -> [f32; 3] {
            [1.0 - r, 1.0 - g, 1.0 - b]
        }

        fn rgba([r, g, b]: [f32; 3]) -> [f32; 4] {
            [r, g, b, 1.0]
        }

        let (color, inverted) = if self.down.get() {
            (invert_rgb(self.color), self.color)
        } else {
            (self.color, invert_rgb(self.color))
        };
        let bb = self.bb.as_ref().map(|x| x.get() as f64);
        rectangle(rgba(color),
                  [bb.x1, bb.y1, bb.x2 - bb.x1, bb.y2 - bb.y1],
                  cx.transform, cx.gfx);


        text::Text::colored(rgba(inverted), 30).draw(
            self.name,
            cx.glyph_cache,
            default_draw_state(),
            cx.transform.trans((bb.x1 + bb.x2) / 2.0, (bb.y1 + bb.y2) / 2.0),
            cx.gfx
        );
    }
}

impl Hit<MouseDown> for Demo {
    fn hit(&self, _: MouseDown) {
        self.down.set(true);
    }
}

impl Hit<MouseUp> for Demo {
    fn hit(&self, _: MouseUp) {
        self.down.set(false);
    }
}
