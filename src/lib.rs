#![feature(alloc, catch_panic, core, plugin, rustc_private, slice_patterns, unicode)]
#![plugin(regex_macros)]
extern crate regex;
extern crate arena;

extern crate image;
extern crate graphics;
#[macro_use(uniform, implement_vertex)]
extern crate glium;
extern crate piston;
extern crate glutin;
extern crate glutin_window;

#[cfg(windows)]
#[macro_use(shared_library)]
extern crate shared_library;

pub mod back_end;
pub mod glyph;
pub mod window;

pub mod gfx {
    pub use graphics::math::Matrix2d as Mat2;
    pub use graphics::types::*;
    pub use graphics::Viewport;

    pub use glutin::MouseCursor;

    use glium;

    pub type GlyphCache = ::glyph::GlyphCache<::window::GliumWindow>;
    pub type G2d = ::back_end::Glium2d;
    pub type Surface = glium::Frame;
    pub type BackEnd<'a> = ::back_end::GliumGraphics<'a, 'a, Surface>;
}

pub mod cfg {
    pub use ui::color::BreezeDark as ColorScheme;
}

#[macro_use]
pub mod ui;

pub mod ide {
    pub mod rustc;
    pub mod highlight;
}
