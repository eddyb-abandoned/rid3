#![feature(alloc, catch_panic, core, plugin, rustc_private, slice_patterns, unicode)]
#![plugin(regex_macros)]
extern crate regex;
extern crate arena;

extern crate graphics;
extern crate gfx as gfx_core;
extern crate gfx_device_gl as gfx_device;
extern crate gfx_graphics;
extern crate piston;
extern crate glutin;
extern crate glutin_window;

pub mod glyph;

pub mod gfx {
    pub use graphics::math::Matrix2d as Mat2;
    pub use graphics::types::*;
    pub use graphics::Viewport;

    pub use glutin::MouseCursor;

    use gfx_core;
    use gfx_graphics as g2d;
    use gfx_device as dev;

    pub type GlyphCache = ::glyph::GlyphCache<dev::Resources, dev::Factory>;
    pub type Renderer = gfx_core::Renderer<dev::Resources, dev::CommandBuffer>;
    pub type G2d = g2d::Gfx2d<dev::Resources>;
    pub type Output = dev::Output;
    pub type BackEnd<'a> = g2d::GfxGraphics<'a, dev::Resources,
                                                dev::CommandBuffer,
                                                Output>;
}

pub mod cfg {
    pub use ui::color::Breeze as ColorScheme;
}

#[macro_use]
pub mod ui;

pub mod ide {
    pub mod rustc;
    pub mod highlight;
}
