// NOTE modified version of gfx-graphics' glyph.rs

extern crate freetype as ft;

use std::collections::hash_map::{HashMap, Entry};
use std::path::Path;
use std::rc::Rc;

use graphics::character::{CharacterCache, Character};
use graphics::types::FontSize;
use self::ft::render_mode::RenderMode;
use glium::backend::Facade;
use glium::{Texture2d, Texture};
use image::{Rgba, ImageBuffer};
use back_end::DrawTexture;

use ui::Px;

#[derive(Copy, Clone, Default)]
pub struct Metrics {
    pub width: Px,
    pub height: Px,
    pub baseline: Px
}

/// A struct used for caching rendered font.
pub struct GlyphCache<F: Facade> {
    /// The font face.
    pub face: ft::Face<'static>,
    facade: Rc<F>,
    metrics: HashMap<FontSize, Metrics>,
    data: HashMap<(FontSize, char), Character<DrawTexture>>
}

impl<F: Facade> GlyphCache<F> {
    /// Constructor for a GlyphCache.
    pub fn new<P: AsRef<Path>>(font: P, facade: Rc<F>) -> Result<Self, ft::error::Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_face(font.as_ref(), 0));
        Ok(GlyphCache {
            face: face,
            facade: facade,
            metrics: HashMap::new(),
            data: HashMap::new()
        })
    }

    pub fn from_data(font: &'static [u8], facade: Rc<F>) -> Result<Self, ft::error::Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_memory_face(font, 0));
        Ok(GlyphCache {
            face: face,
            facade: facade,
            metrics: HashMap::new(),
            data: HashMap::new()
        })
    }

    pub fn metrics(&mut self, size: FontSize) -> Metrics {
        self.face.set_pixel_sizes(0, size).unwrap();
        self.face.load_char('┼' as usize, ft::face::DEFAULT).unwrap();
        let glyph = self.face.glyph().get_glyph().unwrap();
        let bb = glyph.get_cbox(3);
        match self.metrics.entry(size) {
            Entry::Occupied(v) => *v.get(),
            Entry::Vacant(v) => {
                *v.insert(Metrics {
                    width: (glyph.advance_x() >> 16) as Px,
                    //width: (bb.xMax - bb.xMin - 2) as Px,
                    height: (bb.yMax - bb.yMin - 2) as Px,
                    baseline: (bb.yMax - 2) as Px
                })
            }
        }
    }
}

impl<F: Facade> CharacterCache for GlyphCache<F> {
    type Texture = DrawTexture;

    fn character(&mut self, size: FontSize, ch: char) -> &Character<Self::Texture> {
        match self.data.entry((size, ch)) {
            //returning `into_mut()' to get reference with 'a lifetime
            Entry::Occupied(v) => v.into_mut(),
            Entry::Vacant(v) => {
                self.face.set_pixel_sizes(0, size).unwrap();
                self.face.load_char(ch as usize, ft::face::DEFAULT).unwrap();
                let glyph = self.face.glyph().get_glyph().unwrap();
                let bitmap_glyph = glyph.to_bitmap(RenderMode::Normal, None).unwrap();
                let bitmap = bitmap_glyph.bitmap();
                v.insert(Character {
                    offset: [
                        bitmap_glyph.left() as f64,
                        bitmap_glyph.top() as f64
                    ],
                    size: [
                        (glyph.advance_x() >> 16) as f64,
                        (glyph.advance_y() >> 16) as f64
                    ],
                    texture: DrawTexture::new(if bitmap.width() != 0 {
                        Texture2d::new(&*self.facade,
                            ImageBuffer::<Rgba<u8>, _>::from_raw(
                                bitmap.width() as u32, bitmap.rows() as u32,
                                bitmap.buffer().iter().flat_map(|&x| vec![255, 255, 255, x].into_iter()).collect::<Vec<_>>()
                            ).expect("failed to create glyph texture")
                        )
                    } else {
                        Texture2d::empty(&*self.facade, 1, 1)
                    })
                })
            }
        }
    }
}
