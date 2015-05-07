use std::cell::Cell;

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
    current: Cell<usize>
}

const TAB_WIDTH: Px = 150.0;

impl<T> Set<T> {
    pub fn new() -> Set<T> {
        Set {
            bb: RectBB::default(),
            tabs: vec![],
            current: Cell::new(0)
        }
    }

    pub fn add(&mut self, x: T) {
        let current = self.current.get();
        let current = if current + 1 >= self.tabs.len() {
            current
        } else {
            current + 1
        };
        self.tabs.insert(current, x);
        self.current.set(current);
    }

    pub fn remove(&mut self) -> Option<T> {
        let current = self.current.get();
        if current >= self.tabs.len() {
            None
        } else {
            if current > 0 {
                self.current.set(current - 1);
            }
            Some(self.tabs.remove(current))
        }
    }

    pub fn current(&self) -> Option<&T> {
        let current = self.current.get();
        self.tabs.get(current)
    }

    pub fn current_mut(&mut self) -> Option<&mut T> {
        let current = self.current.get();
        self.tabs.get_mut(current)
    }
}

impl<T: Layout> RectBounded for Set<T> {
    fn rect_bb(&self) -> &RectBB { &self.bb }
    fn name(&self) -> &'static str { "<tabset>" }
    fn constrain<'a, 'b>(&'a self, (cx, bb): ConstrainCx<'b, 'a>) {
        let current = self.current.get();
        if current >= self.tabs.len() {
            return;
        }

        let tb = self.tabs[current].collect(cx);

        let height = cx.fonts().metrics(text::Regular).height * 2.0;

        cx.equal(bb.x1, tb.x1);
        cx.equal(tb.x2, bb.x2);
        cx.distance(bb.y1, tb.y1, height);
        cx.equal(tb.y2, bb.y2);
    }
}

pub trait Tab {
    fn title(&self) -> String;
}

impl<T> Draw for Set<T> where T: Layout + Tab + Draw {
    fn draw(&self, cx: &mut DrawCx) {
        let bb = self.bb();
        let current = self.current.get();
        if current < self.tabs.len() {
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
                if i == current {
                    let y = bb.y1 + metrics.height * 2.0 - 5.0;
                    cx.fill(BB::rect(x + 3.0, y, TAB_WIDTH - 3.0, 2.0), ColorScheme.focus());
                }

                cx.text(text::Regular, [(x + (TAB_WIDTH - w) / 2.0).round(),
                                        (bb.y1 + metrics.height * 0.5).round()],
                                        ColorScheme.normal(), &text);
            }

            self.tabs[current].draw(cx);
        }
    }
}

trait SetDispatch<T, E> {
    fn dispatch(&self, _tab: &T, _ev: &E) -> bool { false }
}

impl<E, T> Dispatch<E> for Set<T> where Set<T>: SetDispatch<T, E>, T: Dispatch<E> {
    fn dispatch(&self, ev: &E) -> bool {
        let current = self.current.get();
        if current < self.tabs.len() {
            let tab = &self.tabs[current];
            tab.dispatch(ev) | SetDispatch::dispatch(self, tab, ev)
        } else {
            false
        }
    }
}

impl<T> SetDispatch<T, MouseDown> for Set<T> where T: Layout {
    fn dispatch(&self, tab: &T, ev: &MouseDown) -> bool {
        let bb = self.bb();
        let pos = [ev.x, ev.y];
        if bb.contains(pos) && !tab.bb().contains(pos) {
            let current = ((ev.x - bb.x1) / TAB_WIDTH) as usize;
            if current < self.tabs.len() {
                self.current.set(current);
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
