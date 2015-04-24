use std::cell::Cell;
use std::default::Default;

use gfx::Color;

use ui::layout::{RectBB, RectBounded, ConstrainCx, Layout, Where};
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text::Label;

pub struct Demo {
    pub bb: RectBB,
    pub over: Cell<bool>,
    pub down: Cell<bool>,
    pub label: Label
}

impl Demo {
    pub fn new([r, g, b]: [f32; 3], name: &'static str) -> Demo {
        Demo {
            bb: RectBB::default(),
            over: Cell::new(false),
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
        fn invert_rgb([r, g, b, a]: Color) -> Color {
            [1.0 - r, 1.0 - g, 1.0 - b, a]
        }

        let pressed = self.over.get() && self.down.get();
        let color = self.label.color.get();
        let (foreground, background) = if pressed {
            (invert_rgb(color), color)
        } else {
            (color, invert_rgb(color))
        };

        cx.rect(self.bb(), background);

        self.label.color.set(foreground);
        self.label.draw(cx);
        self.label.color.set(color);
    }
}

impl Dispatch<MouseDown> for Demo {
    fn dispatch(&self, _: &MouseDown) -> bool {
        if !self.down.get() { self.down.set(true); true } else { false }
    }
}

impl Dispatch<MouseUp> for Demo {
    fn dispatch(&self, _: &MouseUp) -> bool {
        if self.down.get() { self.down.set(false); true } else { false }
    }
}

impl Dispatch<MouseMove> for Demo {
    fn dispatch(&self, ev: &MouseMove) -> bool {
        let over = self.bb().contains(ev.pos());
        if over != self.over.get() { self.over.set(over); true } else { false }
    }
}

impl Dispatch<MouseScroll> for Demo {}
impl Dispatch<Update> for Demo {}
