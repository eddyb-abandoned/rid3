use std::cell::Cell;
use std::default::Default;

use cfg::ColorScheme;

use ui::dir;
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
    fn name(&self) -> &'static str { "<menubar>" }
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
macro_rules! menu_bar {
    [$($kids:tt)+] => (::ui::menu::Bar::new(flow!(right: $($kids)*)))
}

pub struct Button {
    bb: RectBB,
    pub over: Cell<bool>,
    pub down: Cell<bool>,
    pub label: Label
}

impl Button {
    pub fn new(name: &'static str) -> Button {
        Button {
            bb: RectBB::default(),
            over: Cell::new(false),
            down: Cell::new(false),
            label: Label::new(ColorScheme.normal(), name)
        }
    }
}

impl RectBounded for Button {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { self.label.text }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let lb = self.label.collect(cx);
        let border = 5.0;
        cx.distance(bb.x1, lb.x1, border);
        cx.distance(lb.x2, bb.x2, border);
        cx.distance(bb.y1, lb.y1, border);
        cx.distance(lb.y2, bb.y2, border);
    }
}

impl Draw for Button {
    fn draw(&self, cx: &mut DrawCx) {
        if self.over.get() && self.down.get() {
            cx.fill(self.bb(), ColorScheme.focus());
        }

        self.label.draw(cx);
    }
}

impl Dispatch<MouseDown> for Button {
    fn dispatch(&self, _: &MouseDown) -> bool {
        if !self.down.get() { self.down.set(true); true } else { false }
    }
}

impl Dispatch<MouseUp> for Button {
    fn dispatch(&self, _: &MouseUp) -> bool {
        if self.down.get() { self.down.set(false); true } else { false }
    }
}

impl Dispatch<MouseMove> for Button {
    fn dispatch(&self, ev: &MouseMove) -> bool {
        let over = self.bb().contains([ev.x, ev.y]);
        if over != self.over.get() { self.over.set(over); true } else { false }
    }
}

impl Dispatch<MouseScroll> for Button {}
impl Dispatch<Update> for Button {}
impl<'a> Dispatch<TextInput<'a>> for Button {}
impl Dispatch<KeyDown> for Button {}
impl Dispatch<KeyUp> for Button {}
