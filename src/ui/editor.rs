use std::borrow::ToOwned;
use std::cell::{Cell, RefCell};
use std::cmp::{min, max, Ordering};
use std::default::Default;
use std::fs;
use std::io::{self, Read, Write};
use std::iter::repeat;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::usize;

use cfg::ColorScheme;
use glyph::GlyphMetrics;

use ui::{BB, Dir, Px};
use ui::layout::{RectBB, RectBounded, Layout};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx, MouseCursor};
use ui::event::*;
use ui::tab;
use ui::text;

use ide::{highlight, rustc};
use ide::rustc::Rustc;

pub struct Editor {
    bb: RectBB,
    font: text::Mono,
    font_bold: text::MonoBold,
    font_metrics: Cell<GlyphMetrics>,
    over: Cell<bool>,
    down: Cell<bool>,

    // Caret is visible between [0, 0.5) and hidden between [0.5, 1).
    blink_phase: Cell<f32>,
    // Key and delay until the next repeat.
    held_key: Cell<Option<(Key, f32)>>,

    scroll_start: Cell<usize>,

    selection_start: Cell<Caret>,
    caret: Cell<Caret>,
    vertical_col: Cell<usize>,

    // Position and time since hover started.
    hover: Cell<Option<(Caret, f32)>>,

    // Starting row & column, separator column and content.
    overlay: RefCell<(usize, usize, usize, Vec<Line>)>,

    // Path to the file on disk.
    path: PathBuf,

    lines: RefCell<Vec<Line>>,
    unsaved: Cell<bool>,

    rustc: RefCell<Rustc>,
    new_rustc: RefCell<Option<Rustc>>,
    rustc_dirty: Cell<(usize, usize)>
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
    columns: usize,
    hl_depth: usize,
    ranges: Vec<(usize, highlight::Style)>
}

impl Line {
    fn new(data: String) -> Line {
        Line {
            data: data,
            columns: 0,
            hl_depth: 1,
            ranges: vec![]
        }
    }

    fn update_columns(&mut self) {
        let mut k = Caret { row: 0, col: 0, offset: 0 };
        for c in self.data.chars() {
            k.advance(c, true);
        }
        self.columns = k.col;
    }
}

impl Editor {
    pub fn open<P: AsRef<Path>>(path: P) -> Editor {
        let path: &Path = path.as_ref();

        let mut data = String::new();
        fs::File::open(path).unwrap().read_to_string(&mut data).unwrap();
        let lines: Vec<_> = data.split('\n').map(|line| Line::new(line.to_owned())).collect();

        let caret = Caret {
            row: 0,
            col: 0,
            offset: 0
        };

        let editor = Editor {
            bb: RectBB::default(),
            font: text::Mono,
            font_bold: text::MonoBold,
            font_metrics: Cell::new(GlyphMetrics::default()),
            over: Cell::new(false),
            down: Cell::new(false),
            scroll_start: Cell::new(0),

            blink_phase: Cell::new(0.0),
            held_key: Cell::new(None),

            selection_start: Cell::new(caret),
            caret: Cell::new(caret),
            vertical_col: Cell::new(0),

            hover: Cell::new(None),
            overlay: RefCell::new((0, 0, 0, vec![])),

            path: path.to_path_buf(),
            lines: RefCell::new(lines),
            unsaved: Cell::new(false),

            rustc: RefCell::new(Rustc::start(data)),
            new_rustc: RefCell::new(None),
            rustc_dirty: Cell::new((0, 0))
        };

        let num_lines = editor.lines.borrow().len();
        editor.update_hl(0..num_lines, false);

        editor
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_saved(&self) -> bool {
        !self.unsaved.get()
    }

    pub fn save(&self) {
        println!("Saving {:?}...", self.path);
        self.write_data(fs::File::create(&self.path).unwrap()).unwrap();
        self.unsaved.set(false);
    }

    fn build_overlay(&self, k: Caret, diagnostics: &[(rustc::Level, usize, String)], types: &mut [(Range<usize>, String)])
                     -> (usize, usize, usize, Vec<Line>) {
        let row = k.row;
        let lines = self.lines.borrow();
        let line = &lines[row];

        if types.is_empty() {
            let mut start_col = line.columns;
            let lines = diagnostics.iter().flat_map(|&(level, col, ref message)| {
                start_col = min(start_col, col);

                message.split('\n').enumerate().map(move |(i, data)| {
                    let mut line = Line::new("".to_owned());
                    if i == 0 {
                        line.data = format!("{}: ", level);
                        line.ranges.push((line.data.len(), highlight::Style {
                            color: match level {
                                rustc::Level::Bug | rustc::Level::Fatal | rustc::Level::Error => {
                                    ColorScheme.negative()
                                }
                                rustc::Level::Warning => ColorScheme.neutral(),
                                _ => ColorScheme.positive()
                            },
                            bold: true
                        }));
                    }
                    line.data.push_str(data);
                    line.ranges.push((data.len(), highlight::styles::NormalText));
                    line.update_columns();
                    line
                })
            }).collect();
            return (k.row + 1, start_col, 0, lines);
        }

        // Ascending by start, descending by width.
        types.sort_by(|&(ref a, _), &(ref b, _)| {
            (a.start, b.end - b.start).cmp(&(b.start, a.end - a.start))
        });

        let line = &line.data;
        let mut min = line.len();
        let mut max = 0;
        for &(ref range, _) in &types[..] {
            if min > range.start {
                min = range.start;
            }
            if max < range.end {
                max = range.end;
            }
        }

        let mut k = Caret {
            row: 0,
            col: 0,
            offset: 0
        };
        for c in line[..min].chars() {
            k.advance(c, true);
        }
        let mut separator = 0;
        let mut lines: Vec<_> = types.iter().map(|&(ref range, ref ty)| {
            let mut k = k;
            let mut s = " ".to_string();
            for (i, c) in line[min..max].char_indices() {
                let prev_col = k.col;
                k.advance(c, true);
                if range.start <= min + i && min + i < range.end {
                    s.push(c);
                } else {
                    s.extend(repeat(' ').take(k.col - prev_col));
                }
            }
            separator = k.col;
            s.push_str("  ");
            s.push_str(ty);
            s.push(' ');
            Line::new(s)
        }).collect();

        let (_, hl) = highlight::Rust::run(lines.iter().map(|line| &line.data[..]));
        for (line, (_, ranges)) in lines.iter_mut().zip(hl.into_iter()) {
            line.ranges = ranges;
            line.update_columns();
        }
        let start_col = if k.col == 0 { 0 } else { k.col - 1 };
        (row + 1, start_col, separator - start_col, lines)
    }

    fn data_to_string(&self) -> String {
        let mut v = vec![];
        self.write_data(&mut v).unwrap();
        String::from_utf8(v).unwrap()
    }

    fn write_data<W: Write>(&self, mut w: W) -> io::Result<()> {
        let lines = self.lines.borrow();
        if lines.is_empty() {
            return Ok(());
        }

        try!(w.write(lines[0].data.as_bytes()));
        for line in &lines[1..] {
            try!(w.write(b"\n"));
            try!(w.write(line.data.as_bytes()));
        }
        Ok(())
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

    /// Move a caret forwards or backwards, wrapping at line ends.
    fn advance_caret(&self, mut k: Caret, dir: Dir) -> Caret {
        let lines = self.lines.borrow();
        let line = &lines[k.row];

        match dir {
            Dir::Right => {
                if let Some(c) = line.data[k.offset..].chars().next() {
                    k.advance(c, true);
                } else if k.row + 1 < lines.len() {
                    k.row += 1;
                    k.col = 0;
                    k.offset = 0;
                }
            }
            Dir::Left => {
                if let Some(c) = line.data[..k.offset].chars().next_back() {
                    k.advance(c, false);
                } else if k.row > 0 {
                    k.row -= 1;
                    k.col = lines[k.row].columns;
                    k.offset = lines[k.row].data.len();
                }
            }
            Dir::Down => {
                if k.row + 1 < lines.len() {
                    k.row += 1;
                } else {
                    k.col = usize::MAX;
                }
            }
            Dir::Up => {
                if k.row > 0 {
                    k.row -= 1;
                } else {
                    k.col = 0;
                }
            }
        }

        match dir {
            Dir::Down | Dir::Up => {
                if k.col > lines[k.row].columns {
                    // Move to the end of the line.
                    k.col = lines[k.row].columns;
                    k.offset = lines[k.row].data.len();
                } else {
                    // Find the caret position at the same column.
                    let target = k.col;
                    k.col = 0;
                    k.offset = 0;
                    for c in lines[k.row].data.chars() {
                        if k.col >= target {
                            break;
                        }
                        k.advance(c, true);
                    }
                }
            }
            _ => {}
        }

        k
    }

    fn move_to(&self, k: Caret, hold: bool) {
        if !hold {
            self.selection_start.set(k);
        }
        self.caret.set(k);
        self.vertical_col.set(k.col);
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

    fn update_hl(&self, mut range: Range<usize>, dirty: bool) {
        if dirty {
            let rustc_dirty = self.rustc_dirty.get();
            if rustc_dirty == (0, 0) {
                self.rustc_dirty.set((range.start, range.end));
            } else {
                self.rustc_dirty.set((min(range.start, rustc_dirty.0), max(range.end, rustc_dirty.1)));
            }
            *self.new_rustc.borrow_mut() = Some(Rustc::start(self.data_to_string()));
            self.unsaved.set(true);
        }

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
            line.update_columns();
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

        self.update_hl(s1.row..k.row+1, true);
        self.move_to(k, false);
    }

    fn press(&self, key: Key) -> bool {
        let (s1, s2) = (self.selection_start.get(), self.caret.get());
        let mut k = s2;
        let (mut s1, mut s2) = (min(s1, s2), max(s1, s2));

        let mut dirty = false;

        dirty |= self.hover.get().is_some();
        self.hover.set(None);

        match key {
            Key::Return => self.insert("\n"),
            Key::Tab => self.insert("\t"),
            Key::Delete => {
                if s1 == s2 {
                    s2 = self.advance_caret(s1, Dir::Right);
                }
                self.remove(s1..s2);
                self.update_hl(s1.row..s1.row+1, true);
                self.move_to(s1, false);
            }
            Key::Backspace => {
                if s1 == s2 {
                    s1 = self.advance_caret(s2, Dir::Left);
                }
                self.remove(s1..s2);
                self.update_hl(s1.row..s1.row+1, true);
                self.move_to(s1, false);
            }
            // TODO shift support.
            Key::Left => self.move_to(self.advance_caret(k, Dir::Left), false),
            Key::Right => self.move_to(self.advance_caret(k, Dir::Right), false),
            Key::Down => {
                k.col = self.vertical_col.get();
                self.move_to(self.advance_caret(k, Dir::Down), false);
                self.vertical_col.set(k.col);
            }
            Key::Up => {
                k.col = self.vertical_col.get();
                self.move_to(self.advance_caret(k, Dir::Up), false);
                self.vertical_col.set(k.col);
            }
            _ => return dirty
        }
        true
    }
}

impl RectBounded for Editor {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { "<editor>" }
}

impl tab::Tab for Editor {
    fn title(&self) -> String {
        let mut title = self.path.file_name().unwrap().to_string_lossy().into_owned();
        if self.unsaved.get() {
            title.push('*');
        }
        title
    }
}

impl Draw for Editor {
    fn draw(&self, cx: &mut DrawCx) {
        let mut metrics = self.font_metrics.get();
        if metrics.width == 0.0 {
            metrics = cx.fonts().metrics(self.font);
            self.font_metrics.set(metrics);
        }

        if self.over.get() {
            cx.cursor(MouseCursor::Text);
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

        cx.fill(bb, ColorScheme.back_view());

        let s1 = self.selection_start.get();
        let s2 = self.caret.get();
        let k = s2;
        if start <= s2.row && s2.row <= end {
            let y = bb.y1 + ((s2.row - start) as Px * metrics.height);
            cx.fill(BB {
                x1: bb.x1, y1: y,
                x2: bb.x2, y2: y + metrics.height
            }, ColorScheme.back_view_alt());
        }

        // Error lines.
        {
            let rustc = self.rustc.borrow();
            let new_rustc = self.new_rustc.borrow();
            let rustc = if let Some(ref rustc) = *new_rustc { rustc } else { &*rustc };
            for i in start..end {
                if let Some(lines) = rustc.diagnostics.get(&i) {
                    let error = lines.iter().any(|&(level, _, _)| {
                        match level {
                            rustc::Level::Bug | rustc::Level::Fatal | rustc::Level::Error => true,
                            _ => false
                        }
                    });
                    let mut color = if error { ColorScheme.negative() } else { ColorScheme.neutral() };
                    color[3] = 0.3;
                    let y = bb.y1 + ((i - start) as Px * metrics.height);
                    cx.fill(BB {
                        x1: bb.x1, y1: y,
                        x2: bb.x2, y2: y + metrics.height
                    }, color);
                }
            }
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
            cx.fill(BB {
                x1: x1, y1: y,
                x2: x2, y2: y + metrics.height
            }, ColorScheme.focus());
        }
        // Subsequent lines.
        for row in (s1.row + 1)..s2.row {
            if start <= row && row <= end {
                let y = bb.y1 + ((row - start) as Px * metrics.height);
                cx.fill(BB {
                    x1: bb.x1, y1: y,
                    x2: bb.x2, y2: y + metrics.height
                }, ColorScheme.focus());
            }
        }
        // Last line (if selection has at least 2 lines).
        if start <= s2.row && s2.row <= end && s1.row < s2.row {
            let y = bb.y1 + ((s2.row - start) as Px * metrics.height);
            cx.fill(BB::rect(bb.x1, y, (s2.col as Px) * metrics.width, metrics.height), ColorScheme.focus());
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
                    cx.text(self.font_bold, [x, y], style.color, data);
                } else {
                    cx.text(self.font, [x, y], style.color, data);
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
                cx.fill(BB::rect(x, y, w, metrics.height), ColorScheme.normal());
            }
        }

        let overlay = self.overlay.borrow();
        let (start_row, start_col, separator, ref overlay) = *overlay;
        if overlay.is_empty() || start_row <= start {
            return;
        }

        cx.draw_overlay(|cx| {
            let max_col = overlay.iter().map(|line| line.columns).max().unwrap();
            let row = start_row - start;
            let bb = BB {
                x1: bb.x1 + (start_col as Px) * metrics.width,
                x2: bb.x1 + ((start_col + max_col) as Px) * metrics.width,
                y1: bb.y1 + (row as Px) * metrics.height + 2.0,
                y2: bb.y1 + ((row + overlay.len()) as Px) * metrics.height + 2.0
            };

            // Draw border.
            cx.fill(BB {
                x1: bb.x1 - 2.0, x2: bb.x2 + 2.0,
                y1: bb.y1 - 4.0, y2: bb.y2 + 4.0
            }, ColorScheme.active());

            for (i, line) in overlay.iter().enumerate() {
                let y = bb.y1 + i as Px * metrics.height;
                let mut back_bb = BB {
                    x1: bb.x1, x2: bb.x2,
                    y1: y, y2: y + metrics.height
                };
                if i == 0 {
                    back_bb.y1 -= 2.0;
                }
                if i == overlay.len() - 1 {
                    back_bb.y2 += 2.0;
                }
                cx.fill(back_bb, if i % 2 == 0 { ColorScheme.back_view_alt() } else { ColorScheme.back_view() });

                let mut draw_k = Caret {
                    row: 0,
                    col: 0,
                    offset: 0
                };
                for &(len, style) in &line.ranges {
                    let x = bb.x1 + (draw_k.col as Px) * metrics.width;
                    let data = &line.data[draw_k.offset..draw_k.offset+len];
                    if style.bold {
                        cx.text(self.font_bold, [x, y], style.color, data);
                    } else {
                        cx.text(self.font, [x, y], style.color, data);
                    }
                    for c in data.chars() {
                        draw_k.advance(c, true);
                    }
                }
            }

            // Draw separator.
            if separator > 0 {
                let separator = bb.x1 + (separator as Px + 1.0) * metrics.width;
                cx.fill(BB {
                    x1: separator - 1.0, x2: separator + 1.0,
                    y1: bb.y1, y2: bb.y2
                }, ColorScheme.active());
            }
        });
    }
}

impl Dispatch<MouseDown> for Editor {
    fn dispatch(&self, ev: &MouseDown) -> bool {
        if !self.bb().contains([ev.x, ev.y]) {
            return false;
        }

        let mut dirty = false;
        self.down.set(true);

        if let Some(k) = self.pos_to_caret([ev.x, ev.y]) {
            self.move_to(k, false);
            dirty = true;
        }

        dirty |= self.hover.get().is_some();
        self.hover.set(None);

        dirty
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

        let hover = self.hover.get();
        if let Some(k) = self.pos_to_caret([ev.x, ev.y]) {
            if self.down.get() {
                self.move_to(k, true);
                dirty = true;
            }

            let mut k = k;
            k.col = ((ev.x - self.bb().x1) / self.font_metrics.get().width) as usize;
            let over_caret = self.selection_start.get() == k && self.caret.get() == k;
            if !self.down.get() && !(hover.is_none() && over_caret) {
                if hover.map(|(k, _)| k) != Some(k) {
                    self.hover.set(Some((k, 0.0)));
                    dirty = true;
                }
            } else {
                self.hover.set(None);
                dirty |= hover.is_some();
            }
        } else {
            self.hover.set(None);
            dirty |= hover.is_some();
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
            if ((self.lines.borrow().len() as Px) + 1.0 - ((sy + dy) as Px)) * metrics.height <= (bb.y2 - bb.y1) {
                sy
            } else {
                sy + dy
            }
        };
        self.scroll_start.set(new_sy);
        (new_sy != sy) | self.dispatch(&MouseMove::new(ev.x, ev.y))
    }
}

const BLINK_SPACING: f32 = 0.5;
const KEY_REPEAT_DELAY: f32 = 0.660;
const KEY_REPEAT_SPACING: f32 = 1.0 / 25.0;
const HOVER_DELAY: f32 = 1.0;

impl Dispatch<Update> for Editor {
    fn dispatch(&self, &Update(dt): &Update) -> bool {
        let mut dirty = false;

        let blink = (self.blink_phase.get() + dt) % (BLINK_SPACING * 2.0);
        dirty |= (blink >= BLINK_SPACING) != (self.blink_phase.get() >= BLINK_SPACING);
        self.blink_phase.set(blink);

        if let Some((key, d)) = self.held_key.get() {
            let mut d = d - dt;
            while d <= 0.0 {
                dirty |= self.press(key);
                d += KEY_REPEAT_SPACING;
            }
            self.held_key.set(Some((key, d)));
        }

        let mut rustc = self.rustc.borrow_mut();
        let mut new_rustc = self.new_rustc.borrow_mut();
        {
            let ready = if let Some(ref mut new_rustc) = *&mut *new_rustc {
                dirty |= new_rustc.update();
                new_rustc.state == rustc::State::Waiting
            } else {
                false
            };

            // No errors, save new rustc.
            if ready {
                *rustc = new_rustc.take().unwrap();
                self.rustc_dirty.set((0, 0));
            } else {
                dirty |= rustc.update();
            }
        }

        let mut overlay = self.overlay.borrow_mut();

        // Show hover overlay.
        if let Some((hk, ht)) = self.hover.get() {
            self.hover.set(Some((hk, ht + dt)));
            if ht < HOVER_DELAY && ht + dt >= HOVER_DELAY {
                let (start, end) = self.rustc_dirty.get();
                let lines = self.lines.borrow();
                if (hk.row < start || hk.row >= end) && hk.col < lines[hk.row].columns {
                    let line_offset = if hk.row < start || (start, end) == (0, 0) {
                        lines[..hk.row].iter().map(|l| l.data.len() + 1).sum()
                    } else if hk.row >= end {
                        let to_end: usize = lines[hk.row..].iter().map(|l| l.data.len() + 1).sum();
                        rustc.file_end + 1 - to_end
                    } else {
                        unreachable!()
                    };
                    let line_range = line_offset..line_offset+lines[hk.row].data.len();

                    // Send request for types under cursor.
                    rustc.types_at_offset(line_offset + hk.offset, line_range);
                    dirty = true;
                    *overlay = (0, 0, 0, vec![]);
                }
            }
        }

        // Hide overlay.
        if self.hover.get().map(|(_, ht)| ht).unwrap_or(0.0) < HOVER_DELAY {
            dirty |= !overlay.3.is_empty();
            *overlay = (0, 0, 0, vec![]);
            // Clear pending requests.
            if let rustc::State::TypesAtOffset(_) = rustc.state {
                rustc.state = rustc::State::Waiting;
            }
            rustc.types_at_offset = None;
        } else if overlay.3.is_empty() {
            let (hk, _) = self.hover.get().unwrap();
            let mut types = rustc.types_at_offset.take();
            {
                let diagnostics = if let Some(ref rustc) = *new_rustc {
                    &rustc.diagnostics
                } else {
                    &rustc.diagnostics
                };
                *overlay = self.build_overlay(hk, diagnostics.get(&hk.row).unwrap_or(&vec![]),
                                              types.as_mut().unwrap_or(&mut vec![]));
            }
            rustc.types_at_offset = types;
            dirty = true;
        }

        dirty
    }
}

impl<'a> Dispatch<TextInput<'a>> for Editor {
    fn dispatch(&self, ev: &TextInput) -> bool {
        let mut dirty = false;

        if !ev.0.is_empty() {
            self.insert(ev.0);
            dirty = true;
        }

        dirty |= self.hover.get().is_some();
        self.hover.set(None);

        dirty
    }
}

impl Dispatch<KeyDown> for Editor {
    fn dispatch(&self, &KeyDown(key): &KeyDown) -> bool {
        self.held_key.set(Some((key, KEY_REPEAT_DELAY)));
        self.press(key)
    }
}

impl Dispatch<KeyUp> for Editor {
    fn dispatch(&self, &KeyUp(key): &KeyUp) -> bool {
        if let Some((k, _)) = self.held_key.get() {
            if k == key {
                self.held_key.set(None);
            }
        }
        false
    }
}
