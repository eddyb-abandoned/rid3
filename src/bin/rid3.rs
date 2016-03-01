#![feature(slice_patterns)]

extern crate glium;
use glium::DisplayBuild;

extern crate fps_counter;
extern crate time;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

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

#[cfg(not(feature = "ide"))]
fn main() { error__please_enable_the_ide_feature_for_rid3 }

#[cfg(feature = "ide")]
fn main() {
    r3::ide::rustc::init_env();

    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(800, 600)
        .with_title(String::from("rid3"))
        .build_glium()
        .unwrap();

    let renderer = &mut ui::render::Renderer::new(&display, FontFaces {
        regular: GlyphCache::from_data(include_bytes!("../../assets/NotoSans/NotoSans-Regular.ttf"), display.clone()).unwrap(),
        mono: GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Regular.otf"), display.clone()).unwrap(),
        mono_bold: GlyphCache::from_data(include_bytes!("../../assets/Hasklig/Hasklig-Bold.otf"), display.clone()).unwrap()
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
    let mut key_tracker = ui::event::KeyTracker::default();
    let mut last_update = time::precise_time_ns();
    let mut cursor = ui::draw::MouseCursor::Default;
    let mut fps_counter = fps_counter::FPSCounter::new();

    // Ready the buffers.
    display.draw().finish().unwrap();
    let mut dirty = true;
    'main: loop {
        use glium::glutin::Event as E;
        use glium::glutin::{ElementState, MouseButton, MouseScrollDelta};

        for event in display.poll_events() {
            dirty |= match event {
                E::KeyboardInput(ElementState::Pressed, _, Some(key)) => {
                    dirty |= root.dispatch(&ui::event::KeyDown(key));
                    for e in key_tracker.down(key) {
                        dirty |= root.dispatch(&e);
                    }
                    false
                }
                E::KeyboardInput(ElementState::Released, _, Some(key)) => {
                    dirty |= root.dispatch(&ui::event::KeyUp(key));
                    for e in key_tracker.up(key) {
                        dirty |= root.dispatch(&e);
                    }
                    false
                }
                E::MouseInput(ElementState::Pressed, MouseButton::Left) => {
                    root.dispatch(&ui::event::MouseDown::new(x, y))
                }
                E::MouseInput(ElementState::Released, MouseButton::Left) => {
                    root.dispatch(&ui::event::MouseUp::new(x, y))
                }
                E::MouseMoved((nx, ny)) => {
                    x = nx as Px;
                    y = ny as Px;
                    root.dispatch(&ui::event::MouseMove::new(x, y))
                }
                // FIXME Convert lines into pixels, or vice-versa.
                E::MouseWheel(MouseScrollDelta::LineDelta(dx, dy)) |
                E::MouseWheel(MouseScrollDelta::PixelDelta(dx, dy)) => {
                    root.dispatch(&ui::event::MouseScroll::with(x, y,
                        ui::event::mouse::Scroll([dx as Px, dy as Px])))
                }
                E::ReceivedCharacter(c) => {
                    root.dispatch(&ui::event::TextInput(c))
                }
                E::Resized(..) => true,
                E::Closed => break 'main,
                _ => false
            }
        }

        let current = time::precise_time_ns();
        let dt = (current - last_update) as f32 / 1e9;
        dirty |= root.dispatch(&ui::event::Update(dt));
        for e in key_tracker.update(dt) {
            dirty |= root.dispatch(&e);
        }
        last_update = current;

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

        if dirty {
            let mut draw_cx = DrawCx::new(renderer, &display, display.draw());
            let [w, h] = draw_cx.dimensions();

            // TODO maybe integrate this with draw_cx?
            ui::layout::compute(&mut root, draw_cx.fonts(), w, h);

            draw_cx.clear(cfg::ColorScheme.background());
            draw_cx.draw(&root);

            let new_cursor = draw_cx.get_cursor();
            draw_cx.finish();

            if (new_cursor as usize) != (cursor as usize) {
                if !cfg!(windows) {
                    display.get_window().map(|w| w.set_cursor(new_cursor));
                }
                cursor = new_cursor;
            }

            let fps = fps_counter.tick();
            let tab_title = root.kids.1.current().map(|tab| tab.title());
            let title = format!("rid3: {} @ {}FPS", tab_title.as_ref().map_or("", |s| &s[..]), fps);
            display.get_window().map(|w| w.set_title(&title));

            dirty = false;
        } else {
            // Sleep for half a frame (assuming 60FPS).
            std::thread::sleep_ms(1000 / 120);
        }
    }
}
