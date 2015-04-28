use std::borrow::ToOwned;
use std::cell::{Cell, RefCell};
use std::cmp::{min, max, Ordering};
use std::default::Default;
use std::fs;
use std::io::Read;
use std::ops::Range;
use std::path::Path;

use cfg::ColorScheme;
use gfx::MouseCursor;
use glyph::Metrics;
use graphics::character::CharacterCache;

use ui::{BB, Px};
use ui::layout::{RectBB, RectBounded, Layout};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text::{self, FontFace};

use ide::highlight;

pub struct Editor {
    bb: RectBB,
    font: text::Mono,
    font_bold: text::MonoBold,
    font_metrics: Cell<Metrics>,
    over: Cell<bool>,
    down: Cell<bool>,

    // Caret is visible between [0, 0.5) and hidden between [0.5, 1).
    blink_phase: Cell<f32>,
    scroll_start: Cell<usize>,

    selection_start: Cell<Caret>,
    caret: Cell<Caret>,

    lines: RefCell<Vec<Line>>
}

#[derive(Copy, Clone)]
struct Caret {
    row: usize,
    col: usize,
    offset: usize
}

impl Caret {
    fn advance(&mut self, c: char, forward: bool) {
        let w = if c == '\t' {
            // FIXME this won't work backwards.
            7 - (self.col + 7) % 8
        } else {
            c.width(false).unwrap_or(1)
        };
        let l = c.len_utf8();

        if forward {
            self.col += w;
            self.offset += l;
        } else {
            self.col -= w;
            self.offset -= l;
        }
    }
}

impl PartialEq for Caret {
    fn eq(&self, other: &Caret) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl Eq for Caret {}

impl PartialOrd for Caret {
    fn partial_cmp(&self, other: &Caret) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Caret {
    fn cmp(&self, other: &Caret) -> Ordering {
        if self.row == other.row {
            self.col.cmp(&other.col)
        } else {
            self.row.cmp(&other.row)
        }
    }
}

#[derive(Debug)]
struct Line {
    data: String,
    hl_depth: usize,
    ranges: Vec<(usize, highlight::Style)>
}

impl Line {
    fn new(data: String) -> Line {
        Line {
            data: data,
            hl_depth: 1,
            ranges: vec![]
        }
    }
}

impl Editor {
    pub fn open<P: AsRef<Path>>(path: P) -> Editor {
        let mut data = String::new();
        fs::File::open(path).unwrap().read_to_string(&mut data).unwrap();
        let lines = data.split('\n').map(|line| Line::new(line.to_owned())).collect();

        let caret = Caret {
            row: 0,
            col: 0,
            offset: 0
        };

        let editor = Editor {
            bb: RectBB::default(),
            font: text::Mono,
            font_bold: text::MonoBold,
            font_metrics: Cell::new(Metrics::default()),
            over: Cell::new(false),
            down: Cell::new(false),
            scroll_start: Cell::new(0),

            blink_phase: Cell::new(0.0),
            selection_start: Cell::new(caret),
            caret: Cell::new(caret),

            lines: RefCell::new(lines)
        };

        let num_lines = editor.lines.borrow().len();
        editor.update_hl(0..num_lines);

        editor
    }

    fn pos_to_caret(&self, [x, y]: [Px; 2]) -> Option<Caret> {
        let metrics = self.font_metrics.get();
        if metrics.width == 0.0 {
            return None;
        }

        let bb = self.bb();
        if x > bb.x2 || y > bb.y2 {
            return None;
        }
        let [x, y] = [x - bb.x1, y - bb.y1];
        if x < 0.0 || y < 0.0 {
            return None;
        }

        let mut k = Caret {
            row:  (y / metrics.height) as usize + self.scroll_start.get(),
            col: 0,
            offset: 0
        };

        let lines = self.lines.borrow();
        if k.row >= lines.len() {
            return None;
        }

        for c in lines[k.row].data.chars() {
            let prev_k = k;
            k.advance(c, true);
            if k.col as Px * metrics.width > x {
                return Some(prev_k);
            }
        }
        Some(k)
    }

            }
            col += w;
            offset += c.len_utf8();
        }
        Some(Caret { row: row, col: col, offset: offset })
    }

    fn move_to(&self, k: Caret, hold: bool) {
        if !hold {
            self.selection_start.set(k);
        }
        self.caret.set(k);
        self.blink_phase.set(0.0);

        // Make sure the caret stays in the viewport.
        let scroll_start = self.scroll_start.get();
        if k.row < scroll_start {
            self.scroll_start.set(k.row);
        } else {
            let metrics = self.font_metrics.get();
            let bb = self.bb();
            let h = bb.y2 - bb.y1;
            if metrics.height != 0.0 && h > 0.0 {
                let rows = (h / metrics.height) as usize;
                if k.row >= scroll_start + rows {
                    self.scroll_start.set(k.row - rows + 1);
                }
            }
        }
    }

    fn update_hl(&self, mut range: Range<usize>) {
        let mut lines = self.lines.borrow_mut();
        while lines[range.start].hl_depth > 0 && range.start > 0 {
            range.start -= 1;
        }
        while lines[range.end - 1].hl_depth > 0 && range.end < lines.len() {
            range.end += 1;
        }

        let (d, mut hl) = highlight::Rust::run(lines[range.clone()].iter().map(|line| &line.data[..]));

        // Fallback to re-highlight everything until the end.
        if d > 0 {
            hl = highlight::Rust::run(lines.iter().map(|line| &line.data[..])).1;
            range = 0..lines.len();
        }

        for (line, (hl_depth, ranges)) in lines[range].iter_mut().zip(hl.into_iter()) {
            line.hl_depth = hl_depth;
            line.ranges = ranges;
        }
    }

    fn remove(&self, range: Range<Caret>) {
        let (s1, s2) = (range.start, range.end);
        let mut lines = self.lines.borrow_mut();

        // Remove part of the first line.
        if s1.row == s2.row {
            let line = &mut lines[s1.row].data;

            let final_len = line.len() - (s2.offset - s1.offset);
            while line.len() > final_len {
                line.remove(s1.offset);
            }
        } else {
            lines[s1.row].data.truncate(s1.offset);
        }

        // Add part of last line to first line (if range has at least 2 lines).
        if s1.row < s2.row {
            let (dest, src) = lines[s1.row..].split_at_mut(1);
            let dest = &mut dest[0].data;
            let src = &mut src[s2.row - s1.row - 1].data;

            dest.push_str(&src[s2.offset..]);
        }

        // Remove all other lines.
        for _ in s1.row+1..s2.row+1 {
            lines.remove(s1.row + 1);
        }
    }

    fn insert(&self, data: &str) {
        let (s1, s2) = (self.selection_start.get(), self.caret.get());
        let (s1, s2) = (min(s1, s2), max(s1, s2));

        if s1 != s2 {
            self.remove(s1..s2);
        }

        let mut k = s1;
        {
            let mut lines = self.lines.borrow_mut();
            for c in data.chars() {
                match c {
                    '\n' => {
                        let new_line = Line::new(lines[k.row].data[k.offset..].to_owned());
                        lines[k.row].data.truncate(k.offset);
                        lines.insert(k.row + 1, new_line);
                        k.row += 1;
                        k.col = 0;
                        k.offset = 0;
                    }
                    '\t' => for _ in (k.col % 4)..4 {
                        lines[k.row].data.insert(k.offset, ' ');
                        k.offset += 1;
                        k.col += 1;
                    },
                    _ => {
                        lines[k.row].data.insert(k.offset, c);
                        k.advance(c, true);
                    }
                }
            }
        }

        self.update_hl(s1.row..k.row+1);
        self.move_to(k, false);
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
        let lines = self.lines.borrow();
        let lines = if end < lines.len() {
            &lines[start..end]
        } else {
            &lines[start..]
        };

        cx.rect(bb, ColorScheme.back_view());

        let s1 = self.selection_start.get();
        let s2 = self.caret.get();
        let k = s2;
        if start <= s2.row && s2.row <= end {
            let y = bb.y1 + ((s2.row - start) as Px * metrics.height);
            cx.rect(BB {
                x1: bb.x1, y1: y,
                x2: bb.x2, y2: y + metrics.height
            }, ColorScheme.back_view_alt());
        }

        // First line of the selection.
        let (s1, s2) = (min(s1, s2), max(s1, s2));
        if start <= s1.row && s1.row <= end {
            let y = bb.y1 + ((s1.row - start) as Px * metrics.height);
            let x1 = bb.x1 + (s1.col as Px) * metrics.width;
            let x2 = if s1.row == s2.row {
                bb.x1 + (s2.col as Px) * metrics.width
            } else {
                bb.x2
            };
            cx.rect(BB {
                x1: x1, y1: y,
                x2: x2, y2: y + metrics.height
            }, ColorScheme.focus());
        }
        // Subsequent lines.
        for row in (s1.row + 1)..s2.row {
            if start <= row && row <= end {
                let y = bb.y1 + ((row - start) as Px * metrics.height);
                cx.rect(BB {
                    x1: bb.x1, y1: y,
                    x2: bb.x2, y2: y + metrics.height
                }, ColorScheme.focus());
            }
        }
        // Last line (if selection has at least 2 lines).
        if start <= s2.row && s2.row <= end && s1.row < s2.row {
            let y = bb.y1 + ((s2.row - start) as Px * metrics.height);
            cx.rect(BB {
                x1: bb.x1, y1: y,
                x2: bb.x1 + (s2.col as Px) * metrics.width, y2: y + metrics.height
            }, ColorScheme.focus());
        }

        // The actual text in each line.
        for (i, line) in lines.iter().enumerate() {
            let y = bb.y1 + i as Px * metrics.height;
            let mut draw_k = Caret {
                row: 0,
                col: 0,
                offset: 0
            };
            for &(len, style) in &line.ranges {
                let x = bb.x1 + (draw_k.col as Px) * metrics.width;
                let data = &line.data[draw_k.offset..draw_k.offset+len];
                if style.bold {
                    self.font_bold.draw(cx, [x, y], style.color, data);
                } else {
                    self.font.draw(cx, [x, y], style.color, data);
                }
                for c in data.chars() {
                    draw_k.advance(c, true);
                }
            }
        }

        // Caret on top of everything else.
        if self.blink_phase.get() < BLINK_SPACING && start <= k.row && k.row <= end {
            let y = bb.y1 + ((k.row - start) as Px * metrics.height);

            // TODO proper BB scissoring.
            let x = bb.x1 + (k.col as Px) * metrics.width;
            let w = 2.0;
            if bb.x1 <= x && x + w <= bb.x2 {
                cx.rect(BB {
                    x1: x, y1: y,
                    x2: x + w, y2: y + metrics.height
                }, ColorScheme.normal());
            }
        }
    }
}

impl Dispatch<MouseDown> for Editor {
    fn dispatch(&self, ev: &MouseDown) -> bool {
        if !self.bb().contains([ev.x, ev.y]) {
            return false;
        }

        self.down.set(true);

        if let Some(caret) = self.pos_to_caret([ev.x, ev.y]) {
            self.move_to(caret, false);
            true
        } else {
            false
        }
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
        let over = self.bb().contains([ev.x, ev.y]);
        let mut dirty = false;
        if over != self.over.get() {
            self.over.set(over);
            dirty = true;
        }

        if let Some(caret) = self.pos_to_caret([ev.x, ev.y]) {
            if self.down.get() {
                self.move_to(caret, true);
                dirty = true;
            }
        }

        dirty
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
            if ((self.lines.borrow().len() as Px)  - ((sy + dy) as Px)) * metrics.height <= (bb.y2 - bb.y1) {
                sy
            } else {
                sy + dy
            }
        };
        self.scroll_start.set(new_sy);
        new_sy != sy
    }
}

const BLINK_SPACING: f32 = 0.5;

impl Dispatch<Update> for Editor {
    fn dispatch(&self, &Update(dt): &Update) -> bool {
        let mut dirty = false;

        let blink = (self.blink_phase.get() + dt) % (BLINK_SPACING * 2.0);
        dirty |= (blink >= BLINK_SPACING) != (self.blink_phase.get() >= BLINK_SPACING);
        self.blink_phase.set(blink);

        dirty
    }
}

impl<'a> Dispatch<TextInput<'a>> for Editor {
    fn dispatch(&self, ev: &TextInput) -> bool {
        if !ev.0.is_empty() {
            self.insert(ev.0);
            true
        } else {
            false
        }
    }
}
