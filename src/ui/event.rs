use std::default::Default;

use ui::Px;
use ui::layout::Where;

pub struct Mouse<T> {
    pub x: Px,
    pub y: Px,
    pub data: T
}

impl<T: Default> Mouse<T> {
    pub fn new(x: Px, y: Px) -> Mouse<T> {
        Mouse {
            x: x,
            y: y,
            data: T::default()
        }
    }
}

impl<T> Where for Mouse<T> {
    fn pos(&self) -> (Px, Px) {
        (self.x, self.y)
    }
}

pub mod mouse {
    #[derive(Default)]
    pub struct Down;
    #[derive(Default)]
    pub struct Up;
    #[derive(Default)]
    pub struct Click;
}

pub type MouseDown = Mouse<mouse::Down>;
pub type MouseUp = Mouse<mouse::Up>;
pub type MouseClick = Mouse<mouse::Click>;
