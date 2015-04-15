#![cfg_attr(test, feature(test))]
#![feature(rustc_private, slice_patterns)]

extern crate arena;
extern crate graphics;
extern crate gfx_device_gl as gfx_device;
extern crate gfx_graphics;
extern crate piston;
extern crate glutin_window;

use std::path::Path;

use gfx_graphics::gfx::traits::*;
use gfx_graphics::{Gfx2d, GlyphCache};
use piston::event::*;
use piston::input::{Button, MouseButton};
use piston::window::{WindowSettings, Size, OpenGLWindow};
use glutin_window::{GlutinWindow, OpenGL};

#[macro_use]
pub mod ui;
use ui::Px;
use ui::layout::Hit;
use ui::draw::DrawCx;

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

    let font_path = Path::new("assets/NotoSans/NotoSans-Regular.ttf");
    let mut glyph_cache = GlyphCache::new(font_path, &mut factory).unwrap();

    let a = Demo::new([1.0, 0.0, 0.0], "a");
    let b = Demo::new([0.0, 1.0, 0.0], "b");
    let c = Demo::new([0.0, 0.0, 1.0], "c");
    let d = Demo::new([1.0, 0.0, 1.0], "d");
    let e = Demo::new([0.0, 1.0, 1.0], "e");
    let f = Demo::new([1.0, 1.0, 0.0], "f");
    let root = flow![up: a, flow![right: b, c, d], flow![left: e, f]];

    let (mut x, mut y) = (0.0, 0.0);
    for e in window.events() {
        if let Some(args) = e.render_args() {
            let viewport = args.viewport();
            let sz = viewport.draw_size;
            let frame = factory.make_fake_output(sz[0] as u16, sz[1] as u16);
            g2d.draw(&mut renderer, &frame, viewport, |c, g| {
                ui::layout::compute(&root, sz[0] as Px, sz[1] as Px);
                graphics::clear(graphics::color::WHITE, g);
                DrawCx {
                    gfx: g,
                    glyph_cache: &mut glyph_cache,
                    transform: c.transform
                }.draw(&root);
            });

            glyph_cache.update(&mut factory);
            device.submit(renderer.as_buffer());
            renderer.reset();
        }

        if let Some(_) = e.after_render_args() {
            device.after_frame();
            factory.cleanup();
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
            root.hit(ui::event::MouseDown::new(x, y));
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.release_args() {
            root.hit(ui::event::MouseUp::new(x, y));
        }

        if let Some([nx, ny]) = e.mouse_cursor_args() {
            x = nx as Px;
            y = ny as Px;
        }
    }
}

#[cfg(test)]
extern crate test;

#[bench]
fn layout(bench: &mut test::Bencher) {
    bench.iter(|| {
        let a = Demo::new([1.0, 0.0, 0.0], "a");
        let b = Demo::new([0.0, 1.0, 0.0], "b");
        let c = Demo::new([0.0, 0.0, 1.0], "c");
        let root = flow![down: a, flow![right: b, c]];
        ui::layout::compute(&root, 800.0, 600.0);
        root
    });
}