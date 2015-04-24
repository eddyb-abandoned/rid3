#![cfg_attr(test, feature(test))]
#![feature(rustc_private, slice_patterns)]

extern crate arena;
extern crate graphics;
extern crate gfx as gfx_core;
extern crate gfx_device_gl as gfx_device;
extern crate gfx_graphics;
extern crate piston;
extern crate glutin;
extern crate glutin_window;

use gfx_core::traits::*;
use gfx_graphics::Gfx2d;

pub mod glyph;

pub mod gfx {
    pub use graphics::math::Matrix2d as Mat2;
    pub use graphics::types::*;

    pub use glutin::MouseCursor;

    use gfx_graphics as g2d;
    use gfx_device as dev;

    pub type GlyphCache = ::glyph::GlyphCache<dev::Resources, dev::Factory>;
    pub type BackEnd<'a> = g2d::GfxGraphics<'a, dev::Resources,
                                                dev::CommandBuffer,
                                                dev::Output>;
}

pub mod cfg {
    pub use ui::color::BreezeDark as ColorScheme;
}

use std::cell::RefCell;
use std::rc::Rc;

use piston::event::*;
use piston::input::{Button, MouseButton};
use piston::window::{WindowSettings, Size, OpenGLWindow};
use glutin_window::{GlutinWindow, OpenGL};

#[macro_use]
pub mod ui;
use ui::Px;
use ui::color::Scheme;
use ui::draw::DrawCx;
use ui::event::Dispatch;
use ui::text::FontFaces;

pub mod demo;
use demo::Demo;

fn main() {
    let mut window = GlutinWindow::new(
        OpenGL::_3_2,
        WindowSettings::new(
            "r3 UI demo".to_string(),
            Size { width: 800, height: 600 }
        ).exit_on_esc(true)
    );

    let (mut device, mut factory) = gfx_device::create(|s| window.get_proc_address(s));
    let mut renderer = factory.create_renderer();
    let mut g2d = Gfx2d::new(&mut device, &mut factory);

    let factory = Rc::new(RefCell::new(factory));

    let mut fonts = FontFaces {
        regular: gfx::GlyphCache::new("assets/NotoSans/NotoSans-Regular.ttf", factory.clone()).unwrap(),
        mono: gfx::GlyphCache::new("assets/Hasklig/Hasklig-Regular.otf", factory.clone()).unwrap()
    };

    let menu_bar = menu_bar![
        ui::menu::Button::new("File"),
        ui::menu::Button::new("Edit"),
        ui::menu::Button::new("Settings"),
        ui::menu::Button::new("Help"),
    ];
    let a = Demo::new([0.0, 1.0, 1.0], "a");
    let b = Demo::new([1.0, 0.0, 1.0], "b");
    let c = Demo::new([1.0, 1.0, 0.0], "c");
    let d = Demo::new([1.0, 0.0, 0.0], "d");
    let e = Demo::new([0.0, 1.0, 0.0], "e");
    let f = Demo::new([0.0, 0.0, 1.0], "f");
    let root = flow![up: a, flow![right: b, c, d], flow![left: e, f], menu_bar];

    let (mut x, mut y) = (0.0, 0.0);
    let mut cursor = gfx::MouseCursor::Default;
    let window = &Rc::new(RefCell::new(window));
    for e in window.events() {
        if let Some(args) = e.render_args() {
            let viewport = args.viewport();
            let sz = viewport.draw_size;
            let frame = factory.borrow_mut().make_fake_output(sz[0] as u16, sz[1] as u16);
            g2d.draw(&mut renderer, &frame, viewport, |c, g| {
                ui::layout::compute(&root, &mut fonts, sz[0] as Px, sz[1] as Px);
                graphics::clear(cfg::ColorScheme.background(), g);
                let mut draw_cx = DrawCx {
                    gfx: g,
                    fonts: &mut fonts,
                    transform: c.transform,
                    cursor: gfx::MouseCursor::Default
                };
                draw_cx.draw(&root);
                if (draw_cx.cursor as usize) != (cursor as usize) {
                    window.borrow_mut().window.set_cursor(draw_cx.cursor);
                    cursor = draw_cx.cursor;
                }
            });

            device.submit(renderer.as_buffer());
            renderer.reset();
        }

        if let Some(_) = e.after_render_args() {
            device.after_frame();
            factory.borrow_mut().cleanup();
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
            root.dispatch(&ui::event::MouseDown::new(x, y));
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.release_args() {
            root.dispatch(&ui::event::MouseUp::new(x, y));
        }

        if let Some([nx, ny]) = e.mouse_cursor_args() {
            x = nx as Px;
            y = ny as Px;
            root.dispatch(&ui::event::MouseMove::new(x, y));
        }

        if let Some([dx, dy]) = e.mouse_scroll_args() {
            root.dispatch(&ui::event::MouseScroll::with(x, y,
                ui::event::mouse::Scroll([dx as Px, dy as Px])));
        }
    }
}

#[cfg(test)]
extern crate test;

#[bench]
fn layout(bench: &mut test::Bencher) {
    // TODO use a headless renderer.
    let mut window = GlutinWindow::new(
        OpenGL::_3_2,
        WindowSettings::new(
            "benchmark".to_string(),
            Size { width: 800, height: 600 }
        ).exit_on_esc(true)
    );

    let (_, factory) = gfx_device::create(|s| window.get_proc_address(s));
    let factory = Rc::new(RefCell::new(factory));

    let mut fonts = FontFaces {
        regular: gfx::GlyphCache::new("assets/NotoSans/NotoSans-Regular.ttf", factory.clone()).unwrap(),
        mono: gfx::GlyphCache::new("assets/Hasklig/Hasklig-Regular.otf", factory.clone()).unwrap()
    };

    let a = Demo::new([1.0, 0.0, 0.0], "a");
    let b = Demo::new([0.0, 1.0, 0.0], "b");
    let c = Demo::new([0.0, 0.0, 1.0], "c");
    let root = flow![down: a, flow![right: b, c]];

    bench.iter(|| {
        ui::layout::compute(&root, &mut fonts, 800.0, 600.0);
        &root
    });
}
