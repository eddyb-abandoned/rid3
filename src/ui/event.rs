use std::iter;

pub use glium::glutin::VirtualKeyCode as Key;

use ui::Px;

pub trait Dispatch<E> {
    /// Process an event and return true if redoing layout or rendering is needed.
    fn dispatch(&mut self, _ev: &E) -> bool { false }
}

impl<A, B, E> Dispatch<E> for (A, B) where
           A: Dispatch<E>,
           B: Dispatch<E> {
    fn dispatch(&mut self, ev: &E) -> bool {
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

#[derive(Copy, Clone)]
pub struct KeyDown(pub Key);
#[derive(Copy, Clone)]
pub struct KeyUp(pub Key);
#[derive(Copy, Clone)]
pub struct KeyPress(pub Key);

pub struct TextInput(pub char);

pub struct Update(pub f32);

pub enum KeyTracker {
    /// No key is held.
    NoKey,

    /// One key is being held and the f32 stores the delay
    /// until its next repeat.
    OneKey(Key, f32)
}

impl Default for KeyTracker {
    fn default() -> KeyTracker {
        KeyTracker::NoKey
    }
}

const KEY_REPEAT_DELAY: f32 = 0.660;
const KEY_REPEAT_SPACING: f32 = 1.0 / 25.0;

pub type KeyPressIterator = iter::Take<iter::Repeat<KeyPress>>;

fn key_press_iter(n: usize, key: Key) -> KeyPressIterator {
    iter::repeat(KeyPress(key)).take(n)
}

impl KeyTracker {
    pub fn down(&mut self, key: Key) -> KeyPressIterator {
        *self = KeyTracker::OneKey(key, KEY_REPEAT_DELAY);
        key_press_iter(1, key)
    }

    pub fn up(&mut self, key: Key) -> KeyPressIterator {
        if let KeyTracker::OneKey(k, _) = *self {
            if k == key {
                *self = KeyTracker::NoKey;
            }
        }
        key_press_iter(0, key)
    }

    pub fn update(&mut self, dt: f32) -> KeyPressIterator {
        if let KeyTracker::OneKey(key, ref mut d) = *self {
            *d -= dt;
            let count = (1.0 - *d / KEY_REPEAT_SPACING).max(0.0);
            if *d <= 0.0 {
                *d %= KEY_REPEAT_SPACING;
                *d += KEY_REPEAT_SPACING;
            }
            key_press_iter(count as usize, key)
        } else {
            key_press_iter(0, Key::Escape)
        }
    }
}
