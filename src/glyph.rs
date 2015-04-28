// NOTE modified version of gfx-graphics' glyph.rs

extern crate freetype as ft;

use std::cell::RefCell;
use std::collections::hash_map::{HashMap, Entry};
use std::path::Path;
use std::rc::Rc;

use graphics::character::{CharacterCache, Character};
use graphics::types::FontSize;
use self::ft::render_mode::RenderMode;
use gfx_graphics::Texture;
use gfx_core as gfx;

use ui::Px;

/// An enum to represent various possible run-time errors that may occur.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    /// An error happened when creating a gfx texture.
    Texture(gfx::tex::TextureError),
    /// An error happened with the FreeType library.
    Freetype(ft::error::Error)
}

impl From<gfx::tex::TextureError> for Error {
    fn from(tex_err: gfx::tex::TextureError) -> Self {
        Error::Texture(tex_err)
    }
}

impl From<ft::error::Error> for Error {
    fn from(ft_err: ft::error::Error) -> Self {
        Error::Freetype(ft_err)
    }
}

#[derive(Copy, Clone, Default)]
pub struct Metrics {
    pub width: Px,
    pub height: Px,
    pub baseline: Px
}

/// A struct used for caching rendered font.
pub struct GlyphCache<R, F> where R: gfx::Resources, F: gfx::Factory<R> {
    /// The font face.
    pub face: ft::Face<'static>,
    factory: Rc<RefCell<F>>,
    metrics: HashMap<FontSize, Metrics>,
    data: HashMap<(FontSize, char), Character<Texture<R>>>
}

impl<R, F> GlyphCache<R, F> where R: gfx::Resources, F: gfx::Factory<R> {
    /// Constructor for a GlyphCache.
    pub fn new<P: AsRef<Path>>(font: P, factory: Rc<RefCell<F>>) -> Result<Self, Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_face(font.as_ref(), 0));
        Ok(GlyphCache {
            face: face,
            factory: factory,
            metrics: HashMap::new(),
            data: HashMap::new()
        })
    }

    pub fn from_data(font: &'static [u8], factory: Rc<RefCell<F>>) -> Result<Self, Error> {
        let freetype = try!(ft::Library::init());
        let face = try!(freetype.new_memory_face(font, 0));
        Ok(GlyphCache {
            face: face,
            factory: factory,
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
                    width: (bb.xMax - bb.xMin - 2) as Px,
                    height: (bb.yMax - bb.yMin - 2) as Px,
                    baseline: (bb.yMax - 1) as Px
                })
            }
        }
    }
}

impl<R, F> CharacterCache for GlyphCache<R, F> where R: gfx::Resources, F: gfx::Factory<R> {
    type Texture = Texture<R>;

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
                let mut factory = self.factory.borrow_mut();
                v.insert(Character {
                    offset: [
                        bitmap_glyph.left() as f64,
                        bitmap_glyph.top() as f64
                    ],
                    size: [
                        (glyph.advance_x() >> 16) as f64,
                        (glyph.advance_y() >> 16) as f64
                    ],
                    texture: if ch != ' ' {
                        Texture::from_memory_alpha(
                            &mut *factory,
                            bitmap.buffer(),
                            bitmap.width() as u32,
                            bitmap.rows() as u32)
                    } else {
                        Texture::empty(&mut *factory).unwrap()
                    }
                })
            }
        }
    }
}
