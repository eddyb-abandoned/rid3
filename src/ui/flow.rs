use ui::{BB, Px, Dir};
use ui::layout::{Layout, CollectCx, CollectBB};
use ui::draw::{Draw, DrawCx};
use ui::event::Dispatch;

pub struct Flow<D, K> {
    pub dir: D,
    pub kids: K
}

#[macro_export]
macro_rules! flow {
    [$dir:ident: $($kids:tt)+] => (::ui::flow::Flow {
        dir: dir_ty!($dir),
        kids: tlist!($($kids)*)
    })
}

pub trait FlowLayout<D> {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>, _: D) -> CollectBB<'a>;
    fn bb(&self, _: D) -> BB<Px>;
}

impl<D: Copy, K> Layout for Flow<D, K> where K: FlowLayout<D> {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        self.kids.collect(cx, self.dir)
    }
    fn bb(&self) -> BB<Px> {
        self.kids.bb(self.dir)
    }
}

impl<D, T> FlowLayout<D> for T where T: Layout {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>, _: D) -> CollectBB<'a> {
        self.collect(cx)
    }
    fn bb(&self, _: D) -> BB<Px> {
        self.bb()
    }
}

impl<D, A, B> FlowLayout<D> for (A, B) where
           A: Layout,
           B: FlowLayout<D>,
           D: Copy,
           Dir: From<D> {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>, dir: D) -> CollectBB<'a> {
        let a = self.0.collect(cx);
        let b = self.1.collect(cx, dir);
        let dir = Dir::from(dir);

        let (a, b) = match dir {
            Dir::Up | Dir::Left => (b, a),
            Dir::Down | Dir::Right => (a, b)
        };

        match dir {
            Dir::Up | Dir::Down => {
                cx.equal(a.x1, b.x1);
                cx.equal(a.x2, b.x2);
                cx.equal(a.y2, b.y1);
            }
            Dir::Left | Dir::Right => {
                cx.equal(a.y1, b.y1);
                cx.equal(a.y2, b.y2);
                cx.equal(a.x2, b.x1);
            }
        }

        BB {
            x1: a.x1, y1: a.y1,
            x2: b.x2, y2: b.y2
        }
    }
    fn bb(&self, dir: D) -> BB<Px> {
        let a = self.0.bb();
        let b = self.1.bb(dir);
        let dir = Dir::from(dir);

        let (a, b) = match dir {
            Dir::Up | Dir::Left => (b, a),
            Dir::Down | Dir::Right => (a, b)
        };

        BB {
            x1: a.x1, y1: a.y1,
            x2: b.x2, y2: b.y2
        }
    }
}

impl<D, K> Draw for Flow<D, K> where K: Draw {
    fn draw(&self, cx: &mut DrawCx) {
        self.kids.draw(cx);
    }
}

impl<D, K, E> Dispatch<E> for Flow<D, K> where K: Dispatch<E> {
    fn dispatch(&mut self, ev: &E) -> bool {
        self.kids.dispatch(ev)
    }
}
