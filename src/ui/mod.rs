use std::ops::{Add, Sub, Neg};

#[macro_export]
macro_rules! tlist {
    [$x:expr] => ($x);
    [$x:expr,] => ($x);
    [$x:expr, $($y:tt)+] => (($x, tlist![$($y)*]))
}

macro_rules! enum_to_phantom {
    ($m:ident => $name:ident { $($v:ident),+ }) => {
        pub mod $m {
            $(#[derive(Copy, Clone)] pub struct $v;)*
        }
        pub enum $name {
            $($v),*
        }
        $(impl From<$m::$v> for $name {
            fn from(_: $m::$v) -> $name {
                $name::$v
            }
        })*
    }
}

enum_to_phantom!(dir => Dir {
    Up, Down, Left, Right
});

#[macro_export]
macro_rules! dir_ty {
    (up) => (::ui::dir::Up);
    (down) => (::ui::dir::Down);
    (left) => (::ui::dir::Left);
    (right) => (::ui::dir::Right)
}

pub mod color;
pub mod draw;
pub mod event;
pub mod layout;
pub mod text;

#[macro_use]
pub mod flow;
#[macro_use]
pub mod menu;
#[macro_use]
pub mod tool;
pub mod tab;

pub mod dialog;
pub mod editor;

pub type Px = f32;

#[derive(Copy, Clone, Default, Debug)]
pub struct BB<T> {
    pub x1: T, pub y1: T,
    pub x2: T, pub y2: T
}

impl<T> BB<T> {
    pub fn rect(x: T, y: T, w: T, h: T) -> BB<T> where T: Copy + Add<Output=T> {
        BB {
            x1: x, y1: y,
            x2: x + w, y2: y + h
        }
    }

    pub fn as_ref<'a>(&'a self) -> BB<&'a T> {
        BB {
            x1: &self.x1, y1: &self.y1,
            x2: &self.x2, y2: &self.y2
        }
    }

    pub fn map<F: FnMut(T) -> U, U>(self, mut f: F) -> BB<U> {
        BB {
            x1: f(self.x1), y1: f(self.y1),
            x2: f(self.x2), y2: f(self.y2)
        }
    }

    pub fn map_name<F: FnMut(T, &'static str) -> U, U>(self, mut f: F) -> BB<U> {
        BB {
            x1: f(self.x1, "x1"), y1: f(self.y1, "y1"),
            x2: f(self.x2, "x2"), y2: f(self.y2, "y2")
        }
    }

    pub fn zip<U>(self, other: BB<U>) -> BB<(T, U)> {
        BB {
            x1: (self.x1, other.x1), y1: (self.y1, other.y1),
            x2: (self.x2, other.x2), y2: (self.y2, other.y2)
        }
    }

    pub fn contains(&self, [x, y]: [T; 2]) -> bool where T: PartialOrd {
        self.x1 <= x && x <= self.x2 && self.y1 <= y && y <= self.y2
    }

    pub fn expand(&self, x: T) -> BB<T> where T: Copy + Add<Output=T> + Sub<Output=T> {
        BB {
            x1: self.x1 - x, y1: self.y1 - x,
            x2: self.x2 + x, y2: self.y2 + x
        }
    }

    pub fn shrink(&self, x: T) -> BB<T> where T: Copy+ Neg<Output=T> + Add<Output=T> + Sub<Output=T>  {
        self.expand(-x)
    }

    pub fn top_left(self) -> [T; 2] { [self.x1, self.y1] }
    pub fn top_right(self) -> [T; 2] { [self.x2, self.y1] }
    pub fn bottom_left(self) -> [T; 2] { [self.x1, self.y2] }
    pub fn bottom_right(self) -> [T; 2] { [self.x2, self.y2] }
}
