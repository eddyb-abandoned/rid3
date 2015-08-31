use ui::{BB, Px};
use ui::layout::{CollectCx, CollectBB, Layout};
use ui::draw::Draw;
use ui::event::*;

pub struct Empty {
    bb: BB<Px>
}

impl Empty {
    pub fn new() -> Empty {
        Empty {
            bb: BB::default()
        }
    }
}

impl Layout for Empty {
    fn bb(&self) -> BB<Px> { self.bb }
    fn collect<'a>(&'a mut self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        cx.area(&mut self.bb, "<empty>")
    }
}

impl Draw for Empty {}

impl Dispatch<MouseDown> for Empty {}
impl Dispatch<MouseUp> for Empty {}
impl Dispatch<MouseMove> for Empty {}
impl Dispatch<MouseScroll> for Empty {}
impl Dispatch<Update> for Empty {}
impl Dispatch<TextInput> for Empty {}
impl Dispatch<KeyDown> for Empty {}
impl Dispatch<KeyUp> for Empty {}
impl Dispatch<KeyPress> for Empty {}
