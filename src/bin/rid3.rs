#![feature(collections_drain, slice_patterns)]

extern crate piston;
extern crate glutin_window;
extern crate fps_counter;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use piston::event::*;
use piston::input::{Button, MouseButton};
use piston::window::{WindowSettings, Size};
use r3::window::GliumWindow;
use glutin_window::{GlutinWindow, OpenGL};

#[macro_use]
extern crate r3;
pub use r3::{cfg, ui};
use r3::glyph::GlyphCache;

use ui::Px;
use ui::color::Scheme;
use ui::draw::DrawCx;
use ui::event::Dispatch;
use ui::tab::Tab;
use ui::text::FontFaces;

fn main() {
    r3::ide::rustc::init_env();

    let ref window = Rc::new(RefCell::new(GlutinWindow::new(
        OpenGL::_2_1,
        WindowSettings::new(
            "rid3".to_string(),
            Size { width: 800, height: 600 }
        ).exit_on_esc(false)
    )));
    let glium_window = Rc::new(GliumWindow::new(window).unwrap());

    let renderer = &mut ui::render::Renderer::new(&glium_window, FontFaces {
        regular: GlyphCache::from_data(include_bytes!("../../assets/NotoSans/NotoSans-Regular.ttf"), glium_window.clone()).unwrap(),
        mono: GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Regular.otf"), glium_window.clone()).unwrap(),
        mono_bold: GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Bold.otf"), glium_window.clone()).unwrap()
    });

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
    let mut cursor = ui::draw::MouseCursor::Default;
    let mut dirty = true;

    let mut fps_counter = fps_counter::FPSCounter::new();
    for e in window.events().swap_buffers(false) {
        if let (true, Some(_)) = (dirty, e.render_args()) {
            let mut draw_cx = DrawCx::new(renderer, glium_window.draw());
            let [w, h] = draw_cx.dimensions();

            // TODO maybe integrate this with draw_cx?
            ui::layout::compute(&root, draw_cx.fonts(), w, h);

            draw_cx.clear(cfg::ColorScheme.background());
            draw_cx.draw(&root);

            let new_cursor = draw_cx.get_cursor();
            draw_cx.finish();

            if (new_cursor as usize) != (cursor as usize) {
                if !cfg!(windows) {
                    window.borrow_mut().window.set_cursor(new_cursor);
                }
                cursor = new_cursor;
            }

            let fps = fps_counter.tick();
            let tab_title = root.kids.1.current().map(|tab| tab.title());
            let title = format!("rid3: {} @ {}FPS", tab_title.as_ref().map_or("", |s| &s[..]), fps);
            window.borrow_mut().window.set_title(&title);

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
            root.kids.1.current_mut().map(|e| e.save());
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
            for file in q.drain(..) {
                root.kids.1.add(ui::editor::Editor::open(file));
                dirty = true;
            }
        }
    }
}
