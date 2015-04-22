use std::default::Default;

use ui::{Px, Flow};
use ui::layout::Where;

pub trait Dispatch<E> {
    fn dispatch(&self, _ev: &E) {}
}

impl<D, K, E> Dispatch<E> for Flow<D, K> where K: Dispatch<E> {
    fn dispatch(&self, ev: &E) {
        self.kids.dispatch(ev)
    }
}

impl<A, B, E> Dispatch<E> for (A, B) where
           A: Dispatch<E>,
           B: Dispatch<E> {
    fn dispatch(&self, ev: &E) {
        self.0.dispatch(ev);
        self.1.dispatch(ev);
    }
}

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
    fn pos(&self) -> [Px; 2] {
        [self.x, self.y]
    }
}

pub mod mouse {
    #[derive(Default)]
    pub struct Down;
    #[derive(Default)]
    pub struct Up;
    #[derive(Default)]
    pub struct Click;
    #[derive(Default)]
    pub struct Move;
}

pub type MouseDown = Mouse<mouse::Down>;
pub type MouseUp = Mouse<mouse::Up>;
pub type MouseClick = Mouse<mouse::Click>;
pub type MouseMove = Mouse<mouse::Move>;
