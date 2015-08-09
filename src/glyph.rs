// NOTE modified version of gfx-graphics' glyph.rs

extern crate freetype as ft;

use std::collections::hash_map::{HashMap, Entry};
use std::path::Path;
use std::rc::Rc;

use self::ft::render_mode::RenderMode;
use glium::{Display, Texture2d};
use image::{Rgba, ImageBuffer};

use ui::Px;

pub type FontSize = u32;

#[derive(Clone)]
pub struct Glyph {
    pub offset: [Px; 2],
    pub advance: Px,
    pub texture: Rc<Texture2d>
}

#[derive(Copy, Clone, Default)]
pub struct GlyphMetrics {
    pub width: Px,
    pub height: Px,
    pub baseline: Px
}

pub struct GlyphCache {
    pub face: ft::Face<'static>,
    facade: Display,
    metrics: HashMap<FontSize, GlyphMetrics>,
    data: HashMap<(FontSize, char), Glyph>
}

impl GlyphCache {
    pub fn new<P: AsRef<Path>>(font: P, facade: Display) -> Result<Self, ft::error::Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_face(font.as_ref(), 0));
        Ok(GlyphCache {
            face: face,
            facade: facade,
            metrics: HashMap::new(),
            data: HashMap::new()
        })
    }

    pub fn from_data(font: &'static [u8], facade: Display) -> Result<Self, ft::error::Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_memory_face(font, 0));
        Ok(GlyphCache {
            face: face,
            facade: facade,
            metrics: HashMap::new(),
            data: HashMap::new()
        })
    }

    pub fn metrics(&mut self, size: FontSize) -> GlyphMetrics {
        self.face.set_pixel_sizes(0, size).unwrap();
        self.face.load_char('┼' as usize, ft::face::DEFAULT).unwrap();
        let glyph = self.face.glyph().get_glyph().unwrap();
        let bb = glyph.get_cbox(3);
        match self.metrics.entry(size) {
            Entry::Occupied(v) => *v.get(),
            Entry::Vacant(v) => {
                *v.insert(GlyphMetrics {
                    width: (glyph.advance_x() >> 16) as Px,
                    //width: (bb.xMax - bb.xMin - 2) as Px,
                    height: (bb.yMax - bb.yMin - 2) as Px,
                    baseline: (bb.yMax - 2) as Px
                })
            }
        }
    }

    pub fn glyph(&mut self, size: FontSize, ch: char) -> &Glyph {
        match self.data.entry((size, ch)) {
            //returning `into_mut()' to get reference with 'a lifetime
            Entry::Occupied(v) => v.into_mut(),
            Entry::Vacant(v) => {
                self.face.set_pixel_sizes(0, size).unwrap();
                self.face.load_char(ch as usize, ft::face::DEFAULT).unwrap();
                let glyph = self.face.glyph().get_glyph().unwrap();
                let bitmap_glyph = glyph.to_bitmap(RenderMode::Normal, None).unwrap();
                let bitmap = bitmap_glyph.bitmap();

                let x = bitmap_glyph.left() as Px;
                let y = bitmap_glyph.top() as Px;
                v.insert(Glyph {
                    offset: [x, -y],
                    advance: (glyph.advance_x() >> 16) as Px,
                    texture: Rc::new(if bitmap.width() != 0 {
                        Texture2d::new(&self.facade,
                            ImageBuffer::<Rgba<u8>, _>::from_raw(
                                bitmap.width() as u32, bitmap.rows() as u32,
                                bitmap.buffer().iter().flat_map(|&x| vec![255, 255, 255, x].into_iter()).collect::<Vec<_>>()
                            ).expect("failed to create glyph texture")
                        )
                    } else {
                        Texture2d::empty(&self.facade, 1, 1)
                    }.unwrap())
                })
            }
        }
    }
}
