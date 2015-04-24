use std::borrow::ToOwned;
use std::cell::Cell;
use std::default::Default;
use std::fs;
use std::io::Read;
use std::path::Path;
use clock_ticks;

use cfg::ColorScheme;
use gfx::MouseCursor;
use glyph::Metrics;
use graphics::character::CharacterCache;

use ui::{BB, Px};
use ui::layout::{RectBB, RectBounded, Layout, Where};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text::{self, FontFace};

pub struct Editor {
    bb: RectBB,
    font: text::Mono,
    font_metrics: Cell<Metrics>,
    over: Cell<bool>,
    down: Cell<bool>,
    blink_phase: Cell<bool>,
    scroll_start: Cell<usize>,
    caret: Cell<Caret>,
    lines: Vec<Line>
}

#[derive(Copy, Clone)]
struct Caret {
    row: usize,
    col: usize,
    offset: usize
}

struct Line {
    data: String
}

impl Editor {
    pub fn open<P: AsRef<Path>>(path: P) -> Editor {
        let mut data = String::new();
        fs::File::open(path).unwrap().read_to_string(&mut data).unwrap();
        Editor {
            bb: RectBB::default(),
            font: text::Mono,
            font_metrics: Cell::new(Metrics::default()),
            over: Cell::new(false),
            down: Cell::new(false),
            blink_phase: Cell::new(false),
            scroll_start: Cell::new(0),
            caret: Cell::new(Caret {
                row: 0,
                col: 0,
                offset: 0
            }),
            lines: data.split('\n').map(|line| Line {
                data: line.to_owned()
            }).collect()
        }
    }
}

impl RectBounded for Editor {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { "<editor>" }
}

impl Draw for Editor {
    fn draw(&self, cx: &mut DrawCx) {
        let mut metrics = self.font_metrics.get();
        if metrics.width == 0.0 {
            metrics = cx.fonts.metrics(self.font);
            self.font_metrics.set(metrics);
        }

        if self.over.get() {
            cx.cursor = MouseCursor::Text;
        }

        let bb = self.bb();
        let start = self.scroll_start.get();
        let end = start + (((bb.y2 - bb.y1) / metrics.height) as usize);
        let lines = if end < self.lines.len() {
            &self.lines[start..end]
        } else {
            &self.lines[start..]
        };

        cx.rect(bb, ColorScheme.back_view());

        let k = self.caret.get();
        if start <= k.row && k.row <= end {
            let y = bb.y1 + ((k.row - start) as Px * metrics.height);
            cx.rect(BB {
                x1: bb.x1, y1: y,
                x2: bb.x2, y2: y + metrics.height
            }, ColorScheme.back_view_alt());

            // TODO proper BB scissoring.
            let x = bb.x1 + (k.col as Px) * metrics.width;
            let w = 2.0;
            if bb.x1 <= x && x + w <= bb.x2 && self.blink_phase.get() {
                cx.rect(BB {
                    x1: x, y1: y,
                    x2: x + w, y2: y + metrics.height
                }, ColorScheme.normal());
            }
        }

        for (i, line) in lines.iter().enumerate() {
            self.font.draw(cx, [bb.x1, bb.y1 + i as Px * metrics.height],
                           ColorScheme.normal(), &line.data);
        }
    }
}

impl Dispatch<MouseDown> for Editor {
    fn dispatch(&self, ev: &MouseDown) -> bool {
        self.down.set(true);

        let metrics = self.font_metrics.get();
        if metrics.width == 0.0 {
            return false;
        }

        let bb = self.bb();
        let [x, y] = ev.pos();
        if x > bb.x2 || y > bb.y2 {
            return false;
        }
        let [x, y] = [x - bb.x1, y - bb.y1];
        if x < 0.0 || y < 0.0 {
            return false;
        }
        let row = (y / metrics.height) as usize + self.scroll_start.get();

        let mut x2 = 0.0;
        let mut col = 0;
        let mut offset = 0;
        for c in self.lines[row].data.chars() {
            let w = c.width(false).unwrap_or(1);
            x2 += w as Px * metrics.width;
            if x2 > x {
                break;
            }
            col += w;
            offset += c.len_utf8();
        }
        self.caret.set(Caret { row: row, col: col, offset: offset });
        true
    }
}

impl Dispatch<MouseUp> for Editor {
    fn dispatch(&self, _: &MouseUp) -> bool {
        self.down.set(false);
        false
    }
}

impl Dispatch<MouseMove> for Editor {
    fn dispatch(&self, ev: &MouseMove) -> bool {
        let over = self.bb().contains(ev.pos());
        if over != self.over.get() { self.over.set(over); true } else { false }
    }
}

impl Dispatch<MouseScroll> for Editor {
    fn dispatch(&self, ev: &MouseScroll) -> bool {
        let metrics = self.font_metrics.get();
        if metrics.width == 0.0 {
            return false;
        }

        let [_, dy] = ev.delta();
        if dy == 0.0 {
            return false;
        }

        let dy = -dy;
        let bb = self.bb();
        let sy = self.scroll_start.get();
        let new_sy = if dy < 0.0 {
            let dy = -dy as usize;
            if sy < dy { sy } else { sy - dy }
        } else {
            let dy = dy as usize;
            if ((self.lines.len() as Px)  - ((sy + dy) as Px)) * metrics.height <= (bb.y2 - bb.y1) {
                sy
            } else {
                sy + dy
            }
        };
        self.scroll_start.set(new_sy);
        new_sy != sy
    }
}

impl Dispatch<Update> for Editor {
    fn dispatch(&self, _: &Update) -> bool {
        const SECOND: u64 = 1_000_000_000;
        let phase = clock_ticks::precise_time_ns() % SECOND > SECOND / 2;
        if phase != self.blink_phase.get() { self.blink_phase.set(phase); true } else { false }
    }
}
