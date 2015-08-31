use std::f32::consts::FRAC_PI_2 as QUARTER_TAU;
use std::ops::{Add, Sub, Mul, Div, Neg};

use glium::{self, Display, Texture2d, Program, VertexBuffer};
use glium::{DrawParameters, BlendingFunction, LinearBlendingFactor};
use glium::Surface as SurfaceTrait;
use glium::index::{NoIndices, PrimitiveType};

use ui::{BB, Px};
use ui::color::Color;
use ui::text::FontFaces;

#[cfg(not(windows))]
fn gamma_pre_correct([r, g, b, a]: Color) -> Color {
    fn ch(x: f32) -> f32 { ((x + 0.055) / 1.055).powf(2.4) }
    [ch(r), ch(g), ch(b), a]
}

#[cfg(windows)]
fn gamma_pre_correct(color: Color) -> Color {
    color
}

pub type Surface = glium::Frame;

#[derive(Copy, Clone)]
struct VertexXY {
    xy: [Px; 2]
}
implement_vertex!(VertexXY, xy);

#[derive(Copy, Clone)]
struct VertexUV {
    uv: [Px; 2]
}
implement_vertex!(VertexUV, uv);

const VERTEX_COUNT: usize = 256;

pub struct Renderer {
    data: Box<([VertexXY; VERTEX_COUNT], [VertexUV; VERTEX_COUNT])>,
    xy_buffer: VertexBuffer<VertexXY>,
    uv_buffer: VertexBuffer<VertexUV>,
    shader_color: Program,
    shader_texture: Program,
    pub fonts: FontFaces
}

impl Renderer {
    pub fn new(facade: &Display, fonts: FontFaces) -> Renderer {
        Renderer {
            data: box { ([VertexXY { xy: [0.0, 0.0] }; VERTEX_COUNT],
                         [VertexUV { uv: [0.0, 0.0] }; VERTEX_COUNT]) },
            xy_buffer: VertexBuffer::empty_dynamic(facade, VERTEX_COUNT).unwrap(),
            uv_buffer: VertexBuffer::empty_dynamic(facade, VERTEX_COUNT).unwrap(),
            shader_color: Program::from_source(facade, "
                #version 120
                uniform vec4 color;
                uniform vec2 scale;
                attribute vec2 xy;
                void main() {
                    gl_Position = vec4(xy * scale + vec2(-1.0, 1.0), 0.0, 1.0);
                }","
                #version 120
                uniform vec4 color;
                void main() {
                    gl_FragColor = color;
                }",
                None).unwrap(),
            shader_texture: Program::from_source(facade, "
                #version 120
                uniform sampler2D s_texture;
                uniform vec4 color;
                uniform vec2 scale;
                attribute vec2 xy;
                attribute vec2 uv;
                varying vec2 v_uv;
                void main() {
                    v_uv = uv;
                    gl_Position = vec4(xy * scale + vec2(-1.0, 1.0), 0.0, 1.0);
                }","
                #version 120
                uniform sampler2D s_texture;
                uniform vec4 color;
                varying vec2 v_uv;
                void main() {
                    gl_FragColor = texture2D(s_texture, v_uv) * color;
                }", None).unwrap(),
            fonts: fonts
        }
    }

    fn scale(&self, surface: &Surface) -> [Px; 2] {
        let (w, h) = surface.get_dimensions();
        let (w, h) = (w as Px, h as Px);
        [2.0 / w, -2.0 / h]
    }

    pub fn clear(&mut self, surface: &mut Surface, color: Color) {
        let [r, g, b, a] = gamma_pre_correct(color);
        surface.clear_color(r, g, b, a);
    }

    pub fn colored<F>(&mut self, surface: &mut Surface, color: Color, mut f: F)
        where F: FnMut((&mut [VertexXY; VERTEX_COUNT], &mut FnMut(&[VertexXY], PrimitiveType))) {

        let color = gamma_pre_correct(color);
        let scale = self.scale(surface);
        let xy_buffer = &self.xy_buffer;
        let shader = &self.shader_color;

        f((&mut self.data.0, &mut |xy_data: &[VertexXY], ty| {
            let xys = xy_buffer.slice(0..xy_data.len()).unwrap();
            xys.write(xy_data);

            surface.draw(xys, &NoIndices(ty), shader,
                &uniform! {
                    color: color,
                    scale: scale
                },
                &DrawParameters {
                    blending_function: Some(BlendingFunction::Addition {
                        source: LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    }),
                    .. Default::default()
                }
            ).unwrap();
        }))
    }

    pub fn textured<F>(&mut self, surface: &mut Surface, color: Color, texture: &Texture2d, mut f: F)
        where F: FnMut((&mut [VertexXY; VERTEX_COUNT],
                        &mut [VertexUV; VERTEX_COUNT],
                        &mut FnMut(&[VertexXY], &[VertexUV], PrimitiveType))) {
        let color = gamma_pre_correct(color);
        let scale = self.scale(surface);
        let shader = &self.shader_texture;
        let (xy_buffer, uv_buffer) = (&self.xy_buffer, &self.uv_buffer);
        let (ref mut xy_data, ref mut uv_data) = *&mut *self.data;

        f((xy_data, uv_data,  &mut |xy_data: &[VertexXY], uv_data: &[VertexUV], ty| {
            let xys = xy_buffer.slice(0..xy_data.len()).unwrap();
            let uvs = uv_buffer.slice(0..uv_data.len()).unwrap();
            xys.write(xy_data);
            uvs.write(uv_data);

            surface.draw((xys, uvs), &NoIndices(ty), shader,
                &uniform! {
                    color: color,
                    scale: scale,
                    s_texture: texture
                },
                &DrawParameters {
                    blending_function: Some(BlendingFunction::Addition {
                        source: LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    }),
                    .. Default::default()
                },
            ).unwrap();
        }))
    }
}

pub trait Buffer: Sized {
    type E: Copy + Sqrt +
            Neg<Output=Self::E> +
            Add<Output=Self::E> +
            Sub<Output=Self::E> +
            Mul<Px, Output=Self::E> +
            Mul<Output=Self::E> +
            Div<Output=Self::E>;

    fn vertex(&mut self, i: usize, vertex: [Self::E; 2]);
    fn flush(&mut self, n: usize, ty: PrimitiveType);
    fn flush_if_full(&mut self, n: usize, ty: PrimitiveType) -> bool;

    fn filled_polygon<I: Iterator<Item=[Self::E; 2]>>(&mut self, polygon: I) {
        let mut i = 0;
        for vertex in polygon {
            self.vertex(i, vertex);
            i += 1;

            if self.flush_if_full(i, PrimitiveType::TriangleFan) {
                // Preserve the first and last vertices.
                self.vertex(1, vertex);
                i = 2;
            }
        }
        self.flush(i, PrimitiveType::TriangleFan);
    }

    fn hollow_polygon<I: Iterator<Item=([Self::E; 2], [Self::E; 2])>>(&mut self, mut polygon: I) {
        let (a, b) = match polygon.next() { None => return, Some(x) => x };
        self.vertex(0, a);
        self.vertex(1, b);

        let mut i = 2;
        for (c, d) in polygon.chain(Some((a, b)).into_iter()) {
            self.vertex(i, c);
            self.vertex(i + 1, d);
            i += 2;

            if self.flush_if_full(i, PrimitiveType::TriangleStrip) {
                // Preserve the last two vertices.
                self.vertex(0, c);
                self.vertex(1, d);
                i = 2;
            }
        }
        self.flush(i, PrimitiveType::TriangleStrip);
    }

    fn line(mut self, from: [Self::E; 2], to: [Self::E; 2], width: Self::E) {
        let [x1, y1] = from;
        let [x2, y2] = to;
        let [lx, ly] = [x2 - x1, y2 - y1];
        let l = (lx * lx + ly * ly).sqrt();
        let scale = width * 0.5 / l;
        // Rotate by 90°.
        let dx = ly * scale;
        let dy = lx * -scale;
        self.filled_polygon([
            [x1 + dx, y1 + dy],
            [x1 - dx, y1 - dy],
            [x2 - dx, y2 - dy],
            [x2 + dx, y2 + dy]
        ].iter().cloned());
    }

    fn rect(mut self, bb: BB<Self::E>) {
        self.filled_polygon([
            bb.bottom_left(),
            bb.top_left(),
            bb.top_right(),
            bb.bottom_right()
        ].iter().cloned());
    }

    fn rect_border(mut self, bb: BB<Self::E>, border_size: Self::E) {
        let o = bb; // outer
        let i = bb.shrink(border_size); // inner
        self.hollow_polygon([
            (o.bottom_right(), i.bottom_right()),
            (o.bottom_left(), i.bottom_left()),
            (o.top_left(), i.top_left()),
            (o.top_right(), i.top_right())
        ].iter().cloned());
    }

    fn rect_round(mut self, bb: BB<Self::E>, resolution: u32, radius: Self::E) {
        let segment_rads = QUARTER_TAU / resolution as Px;
        let quarter = |i, [cx, cy]: [_; 2]| (0..resolution + 1).map(move |j| {
            let angle = (i as Px) * QUARTER_TAU + (j as Px) * segment_rads;
            [cx + radius * angle.cos(), cy + radius * angle.sin()]
        });
        let c = bb.shrink(radius);
        self.filled_polygon(quarter(0, c.bottom_right())
                     .chain(quarter(1, c.bottom_left()))
                     .chain(quarter(2, c.top_left()))
                     .chain(quarter(3, c.top_right())));
    }

    fn rect_border_round(mut self, bb: BB<Self::E>, border_size: Self::E, resolution: u32, radius: Self::E) {
        let segment_rads = QUARTER_TAU / resolution as Px;
        let inner = radius - border_size;
        let quarter = |i, [cx, cy]: [_; 2]| (0..resolution + 1).map(move |j| {
            let angle = (i as Px) * QUARTER_TAU + (j as Px) * segment_rads;
            let (cos, sin) = (angle.cos(), angle.sin());
            ([cx + radius * cos, cy + radius * sin],
             [cx + inner * cos,  cy + inner * sin])
        });
        let c = bb.shrink(radius);
        self.hollow_polygon(quarter(0, c.bottom_right())
                     .chain(quarter(1, c.bottom_left()))
                     .chain(quarter(2, c.top_left()))
                     .chain(quarter(3, c.top_right())));
    }
}

trait Sqrt {
    fn sqrt(self) -> Self;
}

impl Sqrt for Px {
    fn sqrt(self) -> Self {
        self.sqrt()
    }
}

impl<'a, F> Buffer for (&'a mut [VertexXY; VERTEX_COUNT], F)
    where F: FnMut(&[VertexXY], PrimitiveType) {

    type E = Px;

    #[inline(always)]
    fn vertex(&mut self, i: usize, xy: [Px; 2]) {
        self.0[i].xy = xy;
    }

    fn flush(&mut self, n: usize, ty: PrimitiveType) {
        if n >= 3 {
            self.1(&self.0[..n], ty);
        }
    }

    #[inline(always)]
    fn flush_if_full(&mut self, n: usize, ty: PrimitiveType) -> bool {
        if n >= VERTEX_COUNT {
            self.1(self.0, ty);
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone)]
pub struct XYAndUV(pub Px, pub Px);

impl Add for XYAndUV {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        XYAndUV(self.0 + other.0, self.1 + other.1)
    }
}
impl Mul<Px> for XYAndUV {
    type Output = Self;
    fn mul(self, x: Px) -> Self {
        XYAndUV(self.0 * x, self.1 * x)
    }
}
impl Mul for XYAndUV {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        XYAndUV(self.0 * other.0, self.1 * other.0)
    }
}
impl Div for XYAndUV {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        XYAndUV(self.0 / other.0, self.1 / other.0)
    }
}
impl Sqrt for XYAndUV {
    fn sqrt(self) -> Self {
        XYAndUV(self.0.sqrt(), self.1.sqrt())
    }
}
impl Neg for XYAndUV { type Output = Self; fn neg(self) -> Self { self * -1.0 } }
impl Sub for XYAndUV { type Output = Self; fn sub(self, other: Self) -> Self { self + (-other) } }

impl<'a, 'b, F> Buffer for (&'a mut [VertexXY; VERTEX_COUNT], &'b mut [VertexUV; VERTEX_COUNT], F)
    where F: FnMut(&[VertexXY], &[VertexUV], PrimitiveType) {

    type E = XYAndUV;

    #[inline(always)]
    fn vertex(&mut self, i: usize, [XYAndUV(x, u), XYAndUV(y, v)]: [XYAndUV; 2]) {
        self.0[i].xy = [x, y];
        self.1[i].uv = [u, v];
    }

    fn flush(&mut self, n: usize, ty: PrimitiveType) {
        if n >= 3 {
            self.2(&self.0[..n], &self.1[..n], ty);
        }
    }

    #[inline(always)]
    fn flush_if_full(&mut self, n: usize, ty: PrimitiveType) -> bool {
        if n >= VERTEX_COUNT {
            self.2(self.0, self.1, ty);
            true
        } else {
            false
        }
    }
}
