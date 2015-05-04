#![feature(collections, slice_patterns)]

extern crate graphics;
extern crate glium;
extern crate piston;
extern crate glutin_window;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use piston::event::*;
use piston::input::{Button, MouseButton};
use piston::window::{WindowSettings, Size};
use r3::back_end::Glium2d;
use r3::window::GliumWindow;
use glutin_window::{GlutinWindow, OpenGL};

#[macro_use]
extern crate r3;
pub use r3::{cfg, gfx, ui};

use ui::Px;
use ui::color::Scheme;
use ui::draw::DrawCx;
use ui::event::Dispatch;
use ui::text::FontFaces;

fn main() {
    r3::ide::rustc::init_env();

    let opengl = OpenGL::_2_1;
    let ref window = Rc::new(RefCell::new(GlutinWindow::new(
        opengl,
        WindowSettings::new(
            "rid3".to_string(),
            Size { width: 800, height: 600 }
        ).exit_on_esc(false)
    )));
    let glium_window = Rc::new(GliumWindow::new(window).unwrap());

    let mut g2d = Glium2d::new(opengl, &*glium_window);

    let mut fonts = FontFaces {
        regular: gfx::GlyphCache::from_data(include_bytes!("../../assets/NotoSans/NotoSans-Regular.ttf"), glium_window.clone()).unwrap(),
        mono: gfx::GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Regular.otf"), glium_window.clone()).unwrap(),
        mono_bold: gfx::GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Bold.otf"), glium_window.clone()).unwrap()
    };

    let open_queue = RefCell::new(std::env::args().skip(1).map(PathBuf::from).collect::<Vec<_>>());
    let save_current = Cell::new(false);
    let run_current = Cell::new(false);
    let close_current = Cell::new(false);

    let tool_bar = tool_bar![
        ui::tool::Button::new("Open", || {
            open_queue.borrow_mut().extend(ui::dialog::open_file().into_iter())
        }),
        ui::tool::Button::new("Save", || save_current.set(true)),
        ui::tool::Button::new("Run", || {
            save_current.set(true);
            run_current.set(true);
        }),
        ui::tool::Button::new("Close", || close_current.set(true))
    ];
    let mut root = flow![down: tool_bar, ui::tab::Set::<ui::editor::Editor>::new()];

    let (mut x, mut y) = (0.0, 0.0);
    let mut cursor = gfx::MouseCursor::Default;
    let mut dirty = true;

    for e in window.events() {
        if let (true, Some(args)) = (dirty, e.render_args()) {
            let viewport = args.viewport();
            let sz = viewport.draw_size;

            ui::layout::compute(&root, &mut fonts, sz[0] as Px, sz[1] as Px);

            let mut surface = glium_window.draw();
            {
                let mut draw_cx = DrawCx::new(&mut g2d, &mut surface, viewport, &mut fonts);
                draw_cx.clear(cfg::ColorScheme.background());
                draw_cx.draw(&root);

                if (draw_cx.cursor as usize) != (cursor as usize) {
                    if !cfg!(windows) {
                        window.borrow_mut().window.set_cursor(draw_cx.cursor);
                    }
                    cursor = draw_cx.cursor;
                }
            }
            surface.finish();

            dirty = false;
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
            dirty |= root.dispatch(&ui::event::MouseDown::new(x, y));
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            dirty |= root.dispatch(&ui::event::KeyDown(key));
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.release_args() {
            dirty |= root.dispatch(&ui::event::MouseUp::new(x, y));
        }

        if let Some(Button::Keyboard(key)) = e.release_args() {
            dirty |= root.dispatch(&ui::event::KeyUp(key));
        }

        if let Some([nx, ny]) = e.mouse_cursor_args() {
            x = nx as Px;
            y = ny as Px;
            dirty |= root.dispatch(&ui::event::MouseMove::new(x, y));
        }

        if let Some([dx, dy]) = e.mouse_scroll_args() {
            dirty |= root.dispatch(&ui::event::MouseScroll::with(x, y,
                ui::event::mouse::Scroll([dx as Px, dy as Px])));
        }

        if let Some(args) = e.update_args() {
            dirty |= root.dispatch(&ui::event::Update(args.dt as f32));
        }

        if let Some(_) = e.resize_args() {
            dirty = true;
        }

        e.text(|s| dirty |= root.dispatch(&ui::event::TextInput(s)));

        if save_current.get() {
            root.kids.1.current().map(|e| e.save());
            save_current.set(false);
            dirty = true;
        }

        if run_current.get() {
            root.kids.1.current().map(|e| {
                println!("{}", std::iter::repeat('\n').take(200).collect::<String>());
                r3::ide::rustc::compile_and_run(e.path());
            });
            run_current.set(false);
        }

        if close_current.get() {
            let is_unsaved = root.kids.1.current().and_then(|e| {
                if e.is_saved() {
                    None
                } else {
                    Some(())
                }
            }).is_some();
            if is_unsaved {
                println!("Save file first!");
            } else {
                root.kids.1.remove();
                dirty = true;
            }
            close_current.set(false);
        }

        {
            let mut q = open_queue.borrow_mut();
            for file in q.drain() {
                root.kids.1.add(ui::editor::Editor::open(file));
                dirty = true;
            }
        }
    }
}
