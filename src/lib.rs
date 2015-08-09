#![recursion_limit="128"]
#![feature(arc_weak, box_syntax, catch_panic, iter_arith)]
#![feature(plugin, rustc_private, slice_patterns)]
#![plugin(regex_macros)]

extern crate arena;
extern crate regex;
extern crate unicode_width;

extern crate image;
#[macro_use]
extern crate glium;

#[cfg(windows)]
#[macro_use(shared_library)]
extern crate shared_library;

pub mod glyph;

pub mod cfg {
    pub use ui::color::BreezeDark as ColorScheme;
}

#[macro_use]
pub mod ui;

pub mod ide {
    pub mod rustc;
    pub mod highlight;
}
