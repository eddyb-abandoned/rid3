use cfg::ColorScheme;

use ui::{BB, Px, dir};
use ui::layout::{CollectCx, CollectBB, Layout};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text::Label;
use ui::flow::Flow;

pub struct Bar<B> {
    bb: BB<Px>,
    pub buttons: Flow<dir::Right, B>
}

impl<B> Bar<B> {
    pub fn new(buttons: Flow<dir::Right, B>) -> Bar<B> {
        Bar {
            bb: BB::default(),
            buttons: buttons
        }
    }
}

impl<B> Layout for Bar<B> where Flow<dir::Right, B>: Layout {
    fn bb(&self) -> BB<Px> { self.bb }
    fn collect<'a>(&'a mut self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        let bb = cx.area(&mut self.bb, "<menubar>");
        let mb = self.buttons.collect(cx);
        cx.equal(bb.x1, mb.x1);
        cx.equal(bb.y1, mb.y1);
        cx.equal(bb.y2, mb.y2);
        bb
    }
}

impl<B> Draw for Bar<B> where B: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.buttons.draw(cx);
    }
}

impl<E, B> Dispatch<E> for Bar<B> where B: Dispatch<E> {
    fn dispatch(&mut self, ev: &E) -> bool {
        self.buttons.dispatch(ev)
    }
}

#[macro_export]
macro_rules! menu_bar {
    [$($kids:tt)+] => (::ui::menu::Bar::new(flow!(right: $($kids)*)))
}

pub struct Button {
    bb: BB<Px>,
    pub over: bool,
    pub down: bool,
    pub label: Label
}

impl Button {
    pub fn new(name: &'static str) -> Button {
        Button {
            bb: BB::default(),
            over: false,
            down: false,
            label: Label::new(ColorScheme.normal(), name)
        }
    }
}

impl Layout for Button {
    fn bb(&self) -> BB<Px> { self.bb }
    fn collect<'a>(&'a mut self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        let bb = cx.area(&mut self.bb, self.label.text);
        let lb = self.label.collect(cx);
        let border = 5.0;
        cx.distance(bb.x1, lb.x1, border);
        cx.distance(lb.x2, bb.x2, border);
        cx.distance(bb.y1, lb.y1, border);
        cx.distance(lb.y2, bb.y2, border);
        bb
    }
}

impl Draw for Button {
    fn draw(&self, cx: &mut DrawCx) {
        if self.over && self.down {
            cx.fill(self.bb, ColorScheme.focus());
        }

        self.label.draw(cx);
    }
}

impl Dispatch<MouseDown> for Button {
    fn dispatch(&mut self, _: &MouseDown) -> bool {
        if !self.down { self.down = true; true } else { false }
    }
}

impl Dispatch<MouseUp> for Button {
    fn dispatch(&mut self, _: &MouseUp) -> bool {
        if self.down { self.down = false; true } else { false }
    }
}

impl Dispatch<MouseMove> for Button {
    fn dispatch(&mut self, ev: &MouseMove) -> bool {
        let over = self.bb.contains([ev.x, ev.y]);
        if over != self.over { self.over = over; true } else { false }
    }
}

impl Dispatch<MouseScroll> for Button {}
impl Dispatch<Update> for Button {}
impl Dispatch<TextInput> for Button {}
impl Dispatch<KeyDown> for Button {}
impl Dispatch<KeyUp> for Button {}
impl Dispatch<KeyPress> for Button {}
