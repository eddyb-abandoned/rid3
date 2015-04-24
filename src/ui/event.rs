use std::default::Default;

use ui::Px;
use ui::layout::Where;

pub trait Dispatch<E> {
    /// Process an event and return true if redoing layout or rendering is needed.
    fn dispatch(&self, _ev: &E) -> bool { false }
}

impl<A, B, E> Dispatch<E> for (A, B) where
           A: Dispatch<E>,
           B: Dispatch<E> {
    fn dispatch(&self, ev: &E) -> bool {
        self.0.dispatch(ev) | self.1.dispatch(ev)
    }
}

pub struct Mouse<T> {
    pub x: Px,
    pub y: Px,
    data: T
}

impl<T> Mouse<T> {
    pub fn new(x: Px, y: Px) -> Mouse<T> where T: Default {
        Mouse {
            x: x,
            y: y,
            data: T::default()
        }
    }
    pub fn with(x: Px, y: Px, data: T) -> Mouse<T> {
        Mouse {
            x: x,
            y: y,
            data: data
        }
    }
}

impl<T> Where for Mouse<T> {
    fn pos(&self) -> [Px; 2] {
        [self.x, self.y]
    }
}

pub mod mouse {
    use ui::Px;

    #[derive(Default)]
    pub struct Down;
    #[derive(Default)]
    pub struct Up;
    #[derive(Default)]
    pub struct Click;
    #[derive(Default)]
    pub struct Move;

    pub struct Scroll(pub [Px; 2]);
}

pub type MouseDown = Mouse<mouse::Down>;
pub type MouseUp = Mouse<mouse::Up>;
pub type MouseClick = Mouse<mouse::Click>;
pub type MouseMove = Mouse<mouse::Move>;
pub type MouseScroll = Mouse<mouse::Scroll>;

impl MouseScroll {
    pub fn delta(&self) -> [Px; 2] { self.data.0 }
}

pub struct Update;

pub struct TextInput<'a>(pub &'a str);
