use std::default::Default;
use std::rc::Rc;

use glium::{self, Texture2d, Texture, Program, VertexBuffer};
use glium::{DrawParameters, BlendingFunction, LinearBlendingFactor};
use glium::Surface as SurfaceTrait;
use glium::index::{NoIndices, PrimitiveType};
use graphics::{BACK_END_MAX_VERTEX_COUNT, triangulation};
use window::GliumWindow as Window;

use ui::{BB, Px};
use ui::color::Color;
use ui::text::{FontFace, FontFaces};

pub use glutin::MouseCursor;
pub type Surface = glium::Frame;
pub type DrawTexture = Rc<Texture2d>;

#[derive(Copy, Clone)]
struct PlainVertex {
    pos: [Px; 2]
}
implement_vertex!(PlainVertex, pos);

#[derive(Copy, Clone)]
struct TexturedVertex {
    pos: [Px; 2],
    uv: [Px; 2]
}
implement_vertex!(TexturedVertex, pos, uv);

pub struct System {
    plain_buffer: VertexBuffer<PlainVertex>,
    textured_buffer: VertexBuffer<TexturedVertex>,
    shader_texture: Program,
    shader_color: Program,
    fonts: FontFaces
}

impl System {
    pub fn new(window: &Window, fonts: FontFaces) -> System {
        use std::iter::repeat;

        // FIXME: create empty buffers when glium supports them
        let plain_data = repeat(PlainVertex { pos: [0.0, 0.0] })
                                .take(BACK_END_MAX_VERTEX_COUNT).collect::<Vec<_>>();
        let textured_data = repeat(TexturedVertex { pos: [0.0, 0.0], uv: [0.0, 0.0] })
                                .take(BACK_END_MAX_VERTEX_COUNT).collect::<Vec<_>>();

        System {
            plain_buffer: VertexBuffer::dynamic(window, plain_data),
            textured_buffer: VertexBuffer::dynamic(window, textured_data),
            shader_texture: Program::from_source(window, "
                #version 120
                uniform sampler2D s_texture;
                uniform vec4 color;
                attribute vec2 pos;
                attribute vec2 uv;
                varying vec2 v_uv;
                void main() {
                    v_uv = uv;
                    gl_Position = vec4(pos, 0.0, 1.0);
                }","
                #version 120
                uniform sampler2D s_texture;
                uniform vec4 color;
                varying vec2 v_uv;
                void main() {
                    gl_FragColor = texture2D(s_texture, v_uv) * color;
                }", None).unwrap(),
            shader_color: Program::from_source(window, "
                #version 120
                uniform vec4 color;
                attribute vec2 pos;
                void main() {
                    gl_Position = vec4(pos, 0.0, 1.0);
                }","
                #version 120
                uniform vec4 color;
                void main() {
                    gl_FragColor = color;
                }",
                None).unwrap(),
            fonts: fonts
        }
    }
}

pub struct DrawCx<'a> {
    system: &'a mut System,
    surface: Surface,
    cursor: MouseCursor,
    transform: [[f64; 3]; 2],
    overlay_requested: bool,
    overlay_drawing: bool,
    inside_overlay: bool
}

#[cfg(not(windows))]
fn gamma_pre_correct([r, g, b, a]: Color) -> Color {
    fn ch(x: f32) -> f32 { ((x + 0.055) / 1.055).powf(2.4) }
    [ch(r), ch(g), ch(b), a]
}

#[cfg(windows)]
fn gamma_pre_correct(color: Color) -> Color {
    color
}

impl<'a> DrawCx<'a> {
    pub fn new(system: &'a mut System, surface: Surface) -> DrawCx<'a> {
        let (w, h) = surface.get_dimensions();
        let (w, h) = (w as f64, h as f64);

        DrawCx {
            system: system,
            surface: surface,
            cursor: MouseCursor::Default,
            transform: [
                [2.0 / w, 0.0, -1.0],
                [0.0,  -2.0 / h, 1.0]
            ],
            overlay_requested: false,
            overlay_drawing: false,
            inside_overlay: false
        }
    }

    pub fn fonts(&mut self) -> &mut FontFaces {
        &mut self.system.fonts
    }

    pub fn dimensions(&mut self) -> [Px; 2] {
        let (w, h) = self.surface.get_dimensions();
        [w as Px, h as Px]
    }

    pub fn draw<T: Draw>(&mut self, x: &T) {
        x.draw(self);
        if self.overlay_requested {
            self.overlay_drawing = true;
            x.draw(self);
            self.overlay_drawing = false;
            self.overlay_requested = false;
        }
    }

    pub fn draw_overlay<F, T>(&mut self, f: F) -> T where F: FnOnce(&mut DrawCx) -> T {
        assert!(!self.inside_overlay);
        self.inside_overlay = true;
        let r = f(self);
        self.inside_overlay = false;
        self.overlay_requested = true;
        r
    }

    pub fn with_surface<F, T>(&mut self, f: F) -> Option<T> where F: FnOnce(&mut Self) -> T {
        if self.inside_overlay == self.overlay_drawing {
            Some(f(self))
        } else {
            None
        }
    }

    pub fn get_cursor(&self) -> MouseCursor {
        self.cursor
    }

    pub fn cursor(&mut self, cursor: MouseCursor) {
        self.with_surface(|this| this.cursor = cursor);
    }

    pub fn clear(&mut self, color: Color) {
        let [r, g, b, a] = gamma_pre_correct(color);
        self.surface.clear_color(r, g, b, a);
    }

    // TODO make DrawCx linear to ensure this method gets called.
    pub fn finish(self) {
        self.surface.finish();
    }

    fn tri_list<F>(&mut self, color: Color, mut f: F)
        where F: FnMut(&mut FnMut(&[Px])) {
        f(&mut |vertices: &[Px]| {
            let slice = self.system.plain_buffer.slice(0..vertices.len() / 2).unwrap();

            slice.write({
                (0..vertices.len() / 2)
                    .map(|i| PlainVertex {
                        pos: [vertices[2 * i], vertices[2 * i + 1]],
                    })
                    .collect::<Vec<_>>()
            });

            self.surface.draw(
                slice,
                &NoIndices(PrimitiveType::TrianglesList),
                &self.system.shader_color,
                &uniform! { color: color },
                &DrawParameters {
                    blending_function: Some(BlendingFunction::Addition {
                        source: LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    }),
                    .. Default::default()
                },
            )
            .ok()
            .expect("failed to draw triangle list");
        })
    }

    fn tri_list_uv<F>(&mut self, color: Color, texture: &Texture2d, mut f: F)
        where F: FnMut(&mut FnMut(&[Px], &[Px])) {
        use std::cmp::min;

        f(&mut |vertices: &[Px], texture_coords: &[Px]| {
            let len = min(vertices.len(), texture_coords.len()) / 2;

            let slice = self.system.textured_buffer.slice(0..len).unwrap();

            slice.write({
                (0..len)
                    .map(|i| TexturedVertex {
                        pos: [vertices[2 * i], vertices[2 * i + 1]],
                        // FIXME: The `1.0 - ...` is because of a wrong convention
                        uv: [texture_coords[2 * i], 1.0 - texture_coords[2 * i + 1]],
                    })
                    .collect::<Vec<_>>()
            });

            self.surface.draw(
                slice,
                &NoIndices(PrimitiveType::TrianglesList),
                &self.system.shader_texture,
                &uniform! {
                    color: color,
                    s_texture: texture
                },
                &DrawParameters {
                    blending_function: Some(BlendingFunction::Addition {
                        source: LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    }),
                    .. Default::default()
                },
            )
            .ok()
            .expect("failed to draw triangle list");
        })
    }

    pub fn fill(&mut self, bb: BB<Px>, color: Color/*, corner_radius: Px*/) {
        let corner_radius: Px = 0.0;
        let transform = self.transform;
        self.with_surface(|this| {
            let rectangle = [bb.x1 as f64, bb.y1 as f64,
                             (bb.x2 - bb.x1)  as f64, (bb.y2 - bb.y1)  as f64];
            let color = gamma_pre_correct(color);
            let resolution = 1; // ???

            if corner_radius == 0.0 {
                this.tri_list(color, |f| {
                    f(&triangulation::rect_tri_list_xy(transform, rectangle));
                });
            } else {
                this.tri_list(color, |f| {
                    triangulation::with_round_rectangle_tri_list(
                        resolution,
                        transform,
                        rectangle,
                        corner_radius as f64,
                        |vertices| f(vertices)
                    );
                });
            }
        });
    }

    pub fn border(&mut self, bb: BB<Px>, color: Color, border_size: Px, corner_radius: Px) {
        let transform = self.transform;
        self.with_surface(|this| {
            let rectangle = [bb.x1 as f64, bb.y1 as f64,
                             (bb.x2 - bb.x1)  as f64, (bb.y2 - bb.y1)  as f64];
            let color = gamma_pre_correct(color);
            let resolution = 1; // ???

            if corner_radius == 0.0 {
                this.tri_list(color, |f| {
                    f(&triangulation::rect_border_tri_list_xy(transform, rectangle, border_size as f64));
                });
            } else {
                this.tri_list(color, |f| {
                    triangulation::with_round_rectangle_border_tri_list(
                        resolution,
                        transform,
                        rectangle,
                        corner_radius as f64,
                        border_size as f64,
                        |vertices| f(vertices)
                    );
                });
            }
        });
    }

    pub fn text<F: FontFace>(&mut self, font: F, [x, y]: [Px; 2], color: Color, text: &str) {
        self.with_surface(|this| {
            let transform = this.transform;
            let (mut x, y) = (x, y + this.fonts().metrics(font).baseline);

            // TODO use graphemes and maybe harfbuzz.
            for ch in text.chars() {
                let glyph = this.fonts().glyph(font, ch).clone();
                let texture = &glyph.texture;
                let w = texture.get_width();
                let h = texture.get_height().unwrap();

                let [dx, dy] = glyph.offset;
                let rectangle = [(x + dx) as f64, (y + dy) as f64, w as f64, h as f64];
                this.tri_list_uv(color, texture, |f| {
                    f(&triangulation::rect_tri_list_xy(transform, rectangle),
                      &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
                        1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
                });
                x += glyph.advance;
            }
        });
    }
}

pub trait Draw {
    fn draw(&self, cx: &mut DrawCx);
}

impl<A, B> Draw for (A, B) where
        A: Draw,
        B: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.0.draw(cx);
        self.1.draw(cx);
    }
}
