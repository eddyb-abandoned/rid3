use std::cell::Cell;
use std::default::Default;

use cfg::ColorScheme;

use ui::{BB, dir};
use ui::layout::{RectBB, RectBounded, ConstrainCx, Layout};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text::Label;
use ui::flow::Flow;

pub struct Bar<B> {
    bb: RectBB,
    pub buttons: Flow<dir::Right, B>
}

impl<B> Bar<B> {
    pub fn new(buttons: Flow<dir::Right, B>) -> Bar<B> {
        Bar {
            bb: RectBB::default(),
            buttons: buttons
        }
    }
}

impl<B> RectBounded for Bar<B> where Flow<dir::Right, B>: Layout {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { "<toolbar>" }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let mb = self.buttons.collect(cx);
        cx.equal(bb.x1, mb.x1);
        cx.equal(bb.y1, mb.y1);
        cx.equal(bb.y2, mb.y2);
    }
}

impl<B> Draw for Bar<B> where B: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.buttons.draw(cx);
    }
}

impl<E, B> Dispatch<E> for Bar<B> where B: Dispatch<E> {
    fn dispatch(&self, ev: &E) -> bool {
        self.buttons.dispatch(ev)
    }
}

#[macro_export]
macro_rules! tool_bar {
    [$($kids:tt)+] => (::ui::tool::Bar::new(flow!(right: $($kids)*)))
}

pub struct Button<F> {
    bb: RectBB,
    over: Cell<bool>,
    down: Cell<bool>,
    callback: F,
    label: Label
}

impl<F> Button<F> {
    pub fn new(name: &'static str, callback: F) -> Button<F> {
        Button {
            bb: RectBB::default(),
            over: Cell::new(false),
            down: Cell::new(false),
            callback: callback,
            label: Label::new(ColorScheme.normal(), name)
        }
    }
}

impl<F> RectBounded for Button<F> {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { self.label.text }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let lb = self.label.collect(cx);
        cx.distance(bb.x1, lb.x1, 20.0);
        cx.distance(lb.x2, bb.x2, 20.0);
        cx.distance(bb.y1, lb.y1, 10.0);
        cx.distance(lb.y2, bb.y2, 10.0);
    }
}

impl<F> Draw for Button<F> {
    fn draw(&self, cx: &mut DrawCx) {
        let bb = self.bb();
        if self.over.get() {
            cx.fill(bb, ColorScheme.focus());
            if !self.down.get() {
                cx.fill(BB {
                    x1: bb.x1 + 1.0, y1: bb.y1 + 1.0,
                    x2: bb.x2 - 1.0, y2: bb.y2 - 1.0
                }, ColorScheme.background());
            }
        }

        self.label.draw(cx);
    }
}

impl<F> Dispatch<MouseDown> for Button<F> {
    fn dispatch(&self, ev: &MouseDown) -> bool {
        if !self.bb().contains([ev.x, ev.y]) {
            return false;
        }
        if !self.down.get() { self.down.set(true); true } else { false }
    }
}

impl<F> Dispatch<MouseUp> for Button<F> where F: Fn() {
    fn dispatch(&self, ev: &MouseUp) -> bool {
        if self.down.get() {
            self.down.set(false);
            if self.bb().contains([ev.x, ev.y]) {
                (self.callback)();
            }
            true
        } else {
            false
        }
    }
}

impl<F> Dispatch<MouseMove> for Button<F> {
    fn dispatch(&self, ev: &MouseMove) -> bool {
        let over = self.bb().contains([ev.x, ev.y]);
        if over != self.over.get() { self.over.set(over); true } else { false }
    }
}

impl<F> Dispatch<MouseScroll> for Button<F> {}
impl<F> Dispatch<Update> for Button<F> {}
impl<'a, F> Dispatch<TextInput<'a>> for Button<F> {}
impl<F> Dispatch<KeyDown> for Button<F> {}
impl<F> Dispatch<KeyUp> for Button<F> {}
