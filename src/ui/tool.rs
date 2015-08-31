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
        let bb = cx.area(&mut self.bb, "<toolbar>");
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
macro_rules! tool_bar {
    [$($kids:tt)+] => (::ui::tool::Bar::new(flow!(right: $($kids)*)))
}

pub struct Button<F> {
    bb: BB<Px>,
    over: bool,
    down: bool,
    callback: F,
    label: Label
}

impl<F> Button<F> {
    pub fn new(name: &'static str, callback: F) -> Button<F> {
        Button {
            bb: BB::default(),
            over: false,
            down: false,
            callback: callback,
            label: Label::new(ColorScheme.normal(), name)
        }
    }
}

impl<F> Layout for Button<F> {
    fn bb(&self) -> BB<Px> { self.bb }
    fn collect<'a>(&'a mut self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        let bb = cx.area(&mut self.bb, self.label.text);
        let lb = self.label.collect(cx);
        cx.distance(bb.x1, lb.x1, 20.0);
        cx.distance(lb.x2, bb.x2, 20.0);
        cx.distance(bb.y1, lb.y1, 10.0);
        cx.distance(lb.y2, bb.y2, 10.0);
        bb
    }
}

impl<F> Draw for Button<F> {
    fn draw(&self, cx: &mut DrawCx) {
        if self.over {
            cx.fill(self.bb, ColorScheme.focus());
            if !self.down {
                cx.fill(self.bb.shrink(1.0), ColorScheme.background());
            }
        }

        self.label.draw(cx);
    }
}

impl<F> Dispatch<MouseDown> for Button<F> {
    fn dispatch(&mut self, ev: &MouseDown) -> bool {
        if !self.bb.contains([ev.x, ev.y]) {
            return false;
        }
        if !self.down { self.down = true; true } else { false }
    }
}

impl<F> Dispatch<MouseUp> for Button<F> where F: FnMut() {
    fn dispatch(&mut self, ev: &MouseUp) -> bool {
        if self.down {
            self.down = false;
            if self.bb.contains([ev.x, ev.y]) {
                (self.callback)();
            }
            true
        } else {
            false
        }
    }
}

impl<F> Dispatch<MouseMove> for Button<F> {
    fn dispatch(&mut self, ev: &MouseMove) -> bool {
        let over = self.bb.contains([ev.x, ev.y]);
        if over != self.over { self.over = over; true } else { false }
    }
}

impl<F> Dispatch<MouseScroll> for Button<F> {}
impl<F> Dispatch<Update> for Button<F> {}
impl<F> Dispatch<TextInput> for Button<F> {}
impl<F> Dispatch<KeyDown> for Button<F> {}
impl<F> Dispatch<KeyUp> for Button<F> {}
impl<F> Dispatch<KeyPress> for Button<F> {}
