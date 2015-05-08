use cfg::ColorScheme;

use ui::{Px, BB};
use ui::layout::{RectBB, RectBounded, ConstrainCx, Layout};
use ui::color::Scheme;
use ui::draw::{Draw, DrawCx};
use ui::event::*;
use ui::text;

pub struct Set<T> {
    bb: RectBB,
    tabs: Vec<T>,
    current: usize
}

const TAB_WIDTH: Px = 150.0;

impl<T> Set<T> {
    pub fn new() -> Set<T> {
        Set {
            bb: RectBB::default(),
            tabs: vec![],
            current: 0
        }
    }

    pub fn add(&mut self, x: T) {
        if self.current + 1 < self.tabs.len() {
            self.current += 1;
        }
        self.tabs.insert(self.current, x);
    }

    pub fn remove(&mut self) -> Option<T> {
        if self.current >= self.tabs.len() {
            None
        } else {
            if self.current > 0 {
                self.current -= 1;
            }
            Some(self.tabs.remove(self.current))
        }
    }

    pub fn current(&self) -> Option<&T> {
        self.tabs.get(self.current)
    }

    pub fn current_mut(&mut self) -> Option<&mut T> {
        self.tabs.get_mut(self.current)
    }
}

impl<T: Layout> RectBounded for Set<T> {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { "<tabset>" }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        if let Some(tab) = self.current() {
            let tb = tab.collect(cx);

            let height = cx.fonts().metrics(text::Regular).height * 2.0;

            cx.equal(bb.x1, tb.x1);
            cx.equal(tb.x2, bb.x2);
            cx.distance(bb.y1, tb.y1, height);
            cx.equal(tb.y2, bb.y2);
        }
    }
}

pub trait Tab {
    fn title(&self) -> String;
}

impl<T> Draw for Set<T> where T: Layout + Tab + Draw {
    fn draw(&self, cx: &mut DrawCx) {
        let bb = self.bb();
        let metrics = cx.fonts().metrics(text::Regular);

        // Background for all tabs.
        cx.fill(BB::rect(bb.x1, bb.y1, (self.tabs.len() as Px) * TAB_WIDTH, metrics.height * 2.0),
                ColorScheme.inactive());

        for (i, tab) in self.tabs.iter().enumerate() {
            let text = tab.title();
            let w = cx.fonts().text_width(text::Regular, &text);

            // Background for each tab.
            let x = bb.x1 + (i as Px) * TAB_WIDTH;
            cx.fill(BB::rect(x + 1.0, bb.y1, TAB_WIDTH - 2.0, metrics.height * 2.0),
                    ColorScheme.background());

            // Focus highlight.
            if i == self.current {
                let y = bb.y1 + metrics.height * 2.0 - 5.0;
                cx.fill(BB::rect(x + 3.0, y, TAB_WIDTH - 3.0, 2.0), ColorScheme.focus());
            }

            cx.text(text::Regular, [(x + (TAB_WIDTH - w) / 2.0).round(),
                                    (bb.y1 + metrics.height * 0.5).round()],
                                    ColorScheme.normal(), &text);
        }

        self.current().map(|tab| tab.draw(cx));
    }
}

trait SetDispatch<T, E> {
    fn dispatch(&mut self, _ev: &E) -> bool { false }
}

impl<E, T> Dispatch<E> for Set<T> where Set<T>: SetDispatch<T, E>, T: Dispatch<E> {
    fn dispatch(&mut self, ev: &E) -> bool {
        if self.current < self.tabs.len() {
            self.tabs[self.current].dispatch(ev) | SetDispatch::dispatch(self, ev)
        } else {
            false
        }
    }
}

impl<T> SetDispatch<T, MouseDown> for Set<T> where T: Layout {
    fn dispatch(&mut self, ev: &MouseDown) -> bool {
        let bb = self.bb();
        let pos = [ev.x, ev.y];
        if bb.contains(pos) && !self.tabs[self.current].bb().contains(pos) {
            let new_tab = ((ev.x - bb.x1) / TAB_WIDTH) as usize;
            if new_tab < self.tabs.len() && new_tab != self.current {
                self.current = new_tab;
                return true;
            }
        }
        false
    }
}

impl<T> SetDispatch<T, MouseUp> for Set<T> {}
impl<T> SetDispatch<T, MouseMove> for Set<T> {}
impl<T> SetDispatch<T, MouseScroll> for Set<T> {}
impl<T> SetDispatch<T, Update> for Set<T> {}
impl<'a, T> SetDispatch<T, TextInput<'a>> for Set<T> {}
impl<T> SetDispatch<T, KeyDown> for Set<T> {}
impl<T> SetDispatch<T, KeyUp> for Set<T> {}
