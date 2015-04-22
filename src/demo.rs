use std::cell::Cell;
use std::default::Default;

use gfx::Color;

use ui::layout::{RectBB, RectBounded, ConstrainCx, Layout, Hit};
use ui::draw::{Draw, DrawCx};
use ui::event::{MouseDown, MouseUp};
use ui::text::Label;

pub struct Demo {
    pub bb: RectBB,
    pub down: Cell<bool>,
    pub label: Label
}

impl Demo {
    pub fn new([r, g, b]: [f32; 3], name: &'static str) -> Demo {
        Demo {
            bb: RectBB::default(),
            down: Cell::new(false),
            label: Label::new([r, g, b, 1.0], name)
        }
    }
}

impl RectBounded for Demo {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str {
        match self.label.text {
            "a" => "A",
            "b" => "B",
            "c" => "C",
            "d" => "D",
            "e" => "E",
            "f" => "F",
            _ => "<demo>"
        }
    }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let lb = self.label.collect(cx);
        cx.order(bb.x1, lb.x1);
        cx.order(lb.x2, bb.x2);
        cx.order(bb.y1, lb.y1);
        cx.order(lb.y2, bb.y2);
    }
}

impl Draw for Demo {
    fn draw(&self, cx: &mut DrawCx) {
        use graphics::*;

        fn invert_rgb([r, g, b, a]: Color) -> Color {
            [1.0 - r, 1.0 - g, 1.0 - b, a]
        }

        let color = self.label.color.get();
        let (foreground, background) = if self.down.get() {
            (invert_rgb(color), color)
        } else {
            (color, invert_rgb(color))
        };

        let bb = self.bb().map(|x| x as f64);
        rectangle(background,
                  [bb.x1, bb.y1, bb.x2 - bb.x1, bb.y2 - bb.y1],
                  cx.transform, cx.gfx);

        self.label.color.set(foreground);
        self.label.draw(cx);
        self.label.color.set(color);
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
