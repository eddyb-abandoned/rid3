use arena::TypedArena;
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub, Deref, DerefMut};

use ui::{Px, BB};
use ui::text;

pub trait Domain: Copy + PartialOrd + fmt::Debug +
                  Neg<Output=Self> +
                  Add<Output=Self> + Sub<Output=Self> +
                  Mul<Output=Self> + Div<Output=Self> {
    fn zero() -> Self;
    fn one() -> Self;
    fn infinity() -> Self;
    fn sign(self) -> Self;
    fn abs(self) -> Self { self * self.sign() }
}

impl Domain for Px {
    fn zero() -> Px { 0.0 }
    fn one() -> Px { 1.0 }
    fn infinity() -> Px { ::std::f32::INFINITY }
    fn sign(self) -> Px { if self < 0.0 { -1.0 } else { 1.0 } }
}

#[derive(Copy, Clone, PartialEq)]
struct Bounds<T> {
    min: T,
    max: T
}

impl<T: Domain> Bounds<T> {
    fn equal(x: T) -> Bounds<T> {
        Bounds {
            min: x,
            max: x
        }
    }

    fn restrict(&mut self, other: Bounds<T>) {
        let (omin, omax) = (self.min, self.max);
        if other.min > omin {
            if other.min > omax {
                panic!("[{:?}, {:?}].restrict([{:?}, {:?}]): min > omax", self.min, self.max, other.min, other.max);
            }
            self.min = other.min;
        }
        if other.max < omax {
            if other.max < omin {
                panic!("[{:?}, {:?}].restrict([{:?}, {:?}]): max < omin", self.min, self.max, other.min, other.max);
            }
            self.max = other.max;
        }
    }
}

impl<T: Domain> Neg for Bounds<T> {
    type Output = Bounds<T>;

    fn neg(self) -> Bounds<T> {
        Bounds {
            min: -self.max,
            max: -self.min
        }
    }
}

// Range<T> `op` T
impl<T: Domain> Add<T> for Bounds<T> {
    type Output = Bounds<T>;

    fn add(self, x: T) -> Bounds<T> {
        Bounds {
            min: self.min + x,
            max: self.max + x
        }
    }
}

impl<T: Domain> Sub<T> for Bounds<T> {
    type Output = Bounds<T>;

    fn sub(self, x: T) -> Bounds<T> {
        Bounds {
            min: self.min - x,
            max: self.max - x
        }
    }
}

impl<T: Domain> Mul<T> for Bounds<T> {
    type Output = Bounds<T>;

    fn mul(self, x: T) -> Bounds<T> {
        if x < T::zero() {
            Bounds {
                min: self.max * x,
                max: self.min * x
            }
        } else {
            Bounds {
                min: self.min * x,
                max: self.max * x
            }
        }
    }
}

impl<T: Domain> Div<T> for Bounds<T> {
    type Output = Bounds<T>;

    fn div(self, x: T) -> Bounds<T> {
        if x < T::zero() {
            Bounds {
                min: self.max / x,
                max: self.min / x
            }
        } else {
            Bounds {
                min: self.min / x,
                max: self.max / x
            }
        }
    }
}

// Range<T> `op` Range<T>
impl<T: Domain> Add for Bounds<T> {
    type Output = Bounds<T>;

    fn add(self, other: Bounds<T>) -> Bounds<T> {
        Bounds {
            min: self.min + other.min,
            max: self.max + other.max
        }
    }
}

impl<T: Domain> Sub for Bounds<T> {
    type Output = Bounds<T>;

    fn sub(self, other: Bounds<T>) -> Bounds<T> {
        self + (-other)
    }
}

// NB: The min reference goes to actual data, while max is temporary.
#[derive(Copy, Clone)]
pub struct Variable<'a, T: 'a + Domain> {
    min: &'a Cell<T>,
    max: &'a Cell<T>,
    name: [&'static str; 2]
}

impl<'a, T: Domain> PartialEq for Variable<'a, T> {
    fn eq(&self, other: &Variable<'a, T>) -> bool {
        (self.min as *const _) == (other.min as *const _)
    }
}

impl<'a, T: Domain> Eq for Variable<'a, T> {}

impl<'a, T: Domain> PartialOrd for Variable<'a, T> {
    fn partial_cmp(&self, other: &Variable<'a, T>) -> Option<Ordering> {
        (self.min as *const _).partial_cmp(&(other.min as *const _))
    }
}

impl<'a, T: Domain> Ord for Variable<'a, T> {
    fn cmp(&self, other: &Variable<'a, T>) -> Ordering {
        (self.min as *const _).cmp(&(other.min as *const _))
    }
}

impl<'a, T: Domain> Variable<'a, T> {
    fn value(&self) -> Option<T> {
        let (min, max) = (self.min.get(), self.max.get());
        if min == max {
            Some(min)
        } else {
            None
        }
    }

    fn bounds(&self) -> Bounds<T> {
        Bounds {
            min: self.min.get(),
            max: self.max.get()
        }
    }

    fn restrict(&self, b: Bounds<T>) {
        let mut r = self.bounds();
        r.restrict(b);
        self.min.set(r.min);
        self.max.set(r.max);
    }

    fn assign(&self, value: T) {
        self.restrict(Bounds::equal(value));
    }
}

impl<'a, T: Domain> fmt::Debug for Variable<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}.{}", self.name[0], self.name[1]));

        let min = self.min.get();
        let max = self.max.get();
        if min == max {
            try!(write!(f, "(={:?})", min));
        } else if min != -T::infinity() || max != T::infinity() {
            try!(write!(f, "(∈[{:?}, {:?}])", min, max));
        }
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq)]
struct Term<'a, T: 'a + Domain> {
    factor: T,
    var: Variable<'a, T>
}

impl<'a, T: Domain> fmt::Debug for Term<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.factor == T::one() {
            write!(f, "{:?}", self.var)
        } else {
            write!(f, "{:?} * {:?}", self.factor, self.var)
        }
    }
}

#[derive(Clone)]
struct Constraint<'a, T: 'a + Domain> {
    terms: Vec<Term<'a, T>>,
    priority: i8,
    bounds: Bounds<T>
}

impl<'a, T: Domain> fmt::Debug for Constraint<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, term) in self.terms.iter().enumerate() {
            let mut term = term.clone();
            let sign = if term.factor < T::zero() {
                term.factor = -term.factor;
                "-"
            } else { "+" };
            try!(if i == 0 {
                write!(f, "{}{:?}", if sign == "+" { "" } else { sign }, term)
            } else {
                write!(f, " {} {:?}", sign, term)
            });
        }
        if self.bounds.min == self.bounds.max {
            write!(f, " = {:?}", self.bounds.min)
        } else {
            write!(f, " ∈ [{:?}, {:?}]", self.bounds.min, self.bounds.max)
        }
    }
}

// Invariant: constraints' first term always has factor=1.
pub struct System<'a, T: 'a + Domain> {
    var_arena: &'a TypedArena<Cell<T>>,
    variables: Vec<Variable<'a, T>>,
    constraints: VecDeque<Constraint<'a, T>>
}

impl<'a, T: Domain> fmt::Debug for System<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(writeln!(f, "System {:?} {{", self.variables));
        for c in &self.constraints {
            try!(writeln!(f, "    {:?}", c));
        }
        write!(f, "}}")
    }
}

impl<'a, T: Domain> System<'a, T> {
    fn new(var_arena: &'a TypedArena<Cell<T>>) -> System<'a, T> {
        System {
            var_arena: var_arena,
            variables: vec![],
            constraints: VecDeque::new()
        }
    }

    fn var(&mut self, var: &'a Cell<T>, name: [&'static str; 2])
           -> Variable<'a, T> {
        var.set(-T::infinity());
        let max = self.var_arena.alloc(Cell::new(T::infinity()));
        let var = Variable { min: var, max: max, name: name };
        self.variables.push(var);
        var
    }

    fn area(&mut self, bb: &'a BB<Cell<T>>, name: &'static str)
            -> BB<Variable<'a, T>> {
        let bb = bb.as_ref().map_name(|x, name2| self.var(x, [name, name2]));
        // Low priority as they can be redundant sometimes
        self.order(bb.x1, bb.x2);
        self.constraints.back_mut().unwrap().priority -= 1;
        self.order(bb.y1, bb.y2);
        self.constraints.back_mut().unwrap().priority -= 1;
        bb
    }

    pub fn equal(&mut self, a: Variable<'a, T>, b: Variable<'a, T>) {
        self.distance(b, a, T::zero())
    }

    pub fn distance(&mut self, a: Variable<'a, T>, b: Variable<'a, T>, x: T) {
        self.constraints.push_back(Constraint {
            terms: vec![
                Term { factor: T::one(), var: b },
                Term { factor: -T::one(), var: a }
            ],
            priority: 0,
            bounds: Bounds::equal(x)
        })
    }

    pub fn order(&mut self, a: Variable<'a, T>, b: Variable<'a, T>) {
        self.constraints.push_back(Constraint {
            terms: vec![
                Term { factor: T::one(), var: a },
                Term { factor: -T::one(), var: b }
            ],
            priority: 0,
            bounds: Bounds {
                min: -T::infinity(),
                max: T::zero()
            }
        })
    }

    fn solve(&mut self) {
        //println!("{:?}", self);
        let mut age = 0;
        loop {
            'simplify: while self.constraints.len() > age {
                let mut modified = false;
                let c = self.constraints.pop_front().unwrap();
                //.normalize();
                let Constraint { mut terms, mut bounds, priority } = c;
                let mut new_bounds = Bounds::equal(T::zero());

                let mut i = 0;
                let mut j = 0;
                while i < terms.len() {
                    let t = terms[i];
                    if let Some(v) = t.var.value() {
                        bounds = bounds - v * t.factor;
                        modified = true;
                    } else {
                        new_bounds = new_bounds + t.var.bounds() * t.factor;
                        terms[j] = t;
                        j += 1;
                    }
                    i += 1;
                }
                terms.truncate(j);

                {
                    let old_bounds = bounds;
                    bounds.restrict(new_bounds);
                    if bounds != old_bounds {
                        modified = true;
                    }
                }

                terms.sort_by(|a, b| a.var.cmp(&b.var));

                // Normalize terms so that the first one has factor=1.
                if !terms.is_empty() {
                    let f = terms[0].factor;
                    if f != T::one() {
                        for t in &mut terms {
                            t.factor = t.factor / f;
                        }
                        bounds = bounds / f;
                    }
                }

                // Try to merge this constraint with an existing one.
                for c in &mut self.constraints {
                    if c.terms == terms {
                        c.bounds.restrict(bounds);
                        continue 'simplify;
                    }
                }

                if terms.is_empty() {
                    modified = true;
                } else if terms.len() == 1 {
                    let t = terms[0];
                    assert_eq!(t.factor, T::one());
                    t.var.restrict(bounds);
                    modified = true;
                } else {
                    // Opportunistic a - b range reduction.
                    if terms.len() == 2 {
                        let (a, b) = (terms[0], terms[1]);
                        assert_eq!(a.factor, T::one());
                        if b.factor == -T::one() {
                            let (a_bounds, b_bounds) = (a.var.bounds(), b.var.bounds());

                            a.var.restrict(b_bounds + bounds);
                            b.var.restrict(a_bounds - bounds);

                            if a_bounds != a.var.bounds() || b_bounds != b.var.bounds() {
                                modified = true;
                            }
                        }
                    }
                    self.constraints.push_back(Constraint {
                        terms: terms,
                        priority: priority,
                        bounds: bounds
                    });
                }

                if self.constraints.is_empty() {
                    if self.variables.iter().any(|x| x.value().is_none()) {
                        panic!("Unresolved ranges\n{:?}", self);
                    }
                    return;
                }

                if modified {
                    //println!("{:?}", self);
                    age = 0;
                } else {
                    age += 1;
                }
            }

            // Opportunistic solving using 2-term equations.
            /*for (i, c1) in self.constraints.iter().enumerate() {
                if c1.terms.len() != 2 || c1.min != c1.max {
                    continue;
                }
                let x = c1.terms[0];
                let y = c1.terms[1];
                let (x1, a, b) = (x.factor, y.factor, c1.min);
                // x1 -> 1.
                let (a, b) = (a / x1, b / x1);

                for (j, c2) in self.constraints.iter().enumerate() {
                    if j <= i || c2.terms.len() != 2 || c2.min != c2.max {
                        continue;
                    }
                    if c2.terms[0].var != x.var || c2.terms[1].var != y.var {
                        continue;
                    }
                    let (x2, c, d) = (c2.terms[0].factor, c2.terms[1].factor, c2.min);
                    // x2 -> 1.
                    let (c, d) = (c / x2, d / x2);

                    // Subtract (x + ay = b) and (x + cy = d).
                    if a == c {
                        // Maybe one of the equations should be removed?
                        continue;
                    }
                    y.var.assign((b - d) / (a - c));
                    age = 0;
                    break;
                }
            }
            if age != 0 {
                println!("{:?}", self);
            }*/

            // Opportunistic solving by chaining a - b constraints.
            let mut constraints = self.constraints.iter().cloned().collect::<Vec<_>>();
            constraints.sort_by(|a, b| b.priority.cmp(&a.priority));
            for i in 0..constraints.len() {
                let (a, b, bounds) = {
                    let c = &constraints[i];
                    if c.terms.len() != 2 || c.terms[1].factor != -T::one() {
                        continue;
                    }
                    assert_eq!(c.terms[0].factor, T::one());
                    (c.terms[0], c.terms[1], c.bounds)
                };

                // Normalize to a <= b, with bounds = b - a.
                let (a, b, bounds) = if bounds.min >= T::zero() {
                    (b, a, bounds)
                } else {
                    (a, b, -bounds)
                };

                // We can't know the ordering.
                if bounds.max <= T::zero() {
                    continue;
                }

                let mut q = VecDeque::new();
                q.push_back(a.var);
                q.push_back(b.var);

                #[derive(Copy, Clone, Debug)]
                enum Span<T> {
                    Flex(T),
                    Fixed(T)
                }

                let mut s = VecDeque::new();
                let span = if bounds.min == bounds.max {
                    Span::Fixed(bounds.min)
                } else {
                    Span::Flex(T::one())
                };
                s.push_back(span);

                let Bounds { mut min, mut max } = a.var.bounds();
                {
                    let bounds = b.var.bounds();
                    if bounds.min < min {
                        min = bounds.min;
                    }
                    if bounds.max > max {
                        max = bounds.max;
                    }
                }
                let mut flex = T::one() + T::one();
                let mut base = T::zero();
                match span {
                    Span::Fixed(x) => base = base + x,
                    Span::Flex(x) => flex = flex + x
                }

                let mut cs: Vec<_> = constraints.iter().map(|_| None).collect();
                cs[i] = Some(());

                loop {
                    let q_len = q.len();
                    for (j, c) in constraints.iter().enumerate() {
                        if cs[j].is_some() || c.terms.len() != 2 {
                            continue;
                        }
                        if c.terms[1].factor != -T::one() {
                            continue;
                        }
                        assert_eq!(c.terms[0].factor, T::one());

                        let a = c.terms[0];
                        let b = c.terms[1];
                        let bounds = c.bounds;

                        let span = if bounds.min == bounds.max {
                            Span::Fixed(bounds.min.abs())
                        } else {
                            Span::Flex(T::one())
                        };

                        let q_len = q.len();
                        let mut new_var = a.var;
                        if bounds.max <= T::zero() {
                            // a <= b
                            if q.front() == Some(&b.var) {
                                q.push_front(a.var);
                                s.push_front(span);
                            } else if q.back() == Some(&a.var) {
                                q.push_back(b.var);
                                s.push_back(span);
                                new_var = b.var;
                            }
                        }
                        if q.len() <= q_len {
                            if bounds.min >= T::zero() {
                                // b <= a
                                if q.front() == Some(&a.var) {
                                    q.push_front(b.var);
                                    s.push_front(span);
                                    new_var = b.var;
                                } else if q.back() == Some(&b.var) {
                                    q.push_back(a.var);
                                    s.push_back(span);
                                }
                            }
                            if q.len() <= q_len {
                                continue;
                            }
                        }

                        {
                            let bounds = new_var.bounds();
                            if bounds.min < min {
                                min = bounds.min;
                            }
                            if bounds.max > max {
                                max = bounds.max;
                            }
                        }
                        match span {
                            Span::Fixed(x) => base = base + x,
                            Span::Flex(x) => flex = flex + x
                        }
                        cs[j] = Some(());
                    }
                    if q.len() <= q_len {
                        break;
                    }
                }
                let unit = (max - min - base) / flex;
                if T::zero() < unit && unit < T::infinity() {
                    assert_eq!(s.len(), q.len() - 1);
                    let mut val = min + unit;
                    q.pop_front().unwrap().assign(val);
                    while !q.is_empty() {
                        val = val + match s.pop_front().unwrap() {
                            Span::Fixed(x) => x,
                            Span::Flex(x) => x * unit,
                        };
                        q.pop_front().unwrap().assign(val);
                    }
                    age = 0;
                    break;
                }
            }

            if age == 0 {
                continue;
            }

            // Opportunistic solving by picking a point in a ranged variable.
            for v in &self.variables {
                let min = v.min.get();
                let max = v.max.get();
                if -T::infinity() < min && min < max && max < T::infinity() {
                    v.assign((min + max) / (T::one() + T::one()));
                    age = 0;
                    break;
                }
            }
            //println!("{:?}", self);

            if age > 0 {
                panic!("Could not solve\n{:?}", self);
            }
        }
    }
}

pub fn compute<R: Layout>(root: &R, fonts: &mut text::FontFaces, w: Px, h: Px) {
    // TODO reuse the context to avoid allocating every time.
    let var_arena = TypedArena::new();
    let mut cx = CollectCx {
        system: System::new(&var_arena),
        fonts: fonts
    };
    let r = root.collect(&mut cx);
    r.x1.assign(0.0);
    r.y1.assign(0.0);
    r.x2.assign(w);
    r.y2.assign(h);
    cx.solve();
}

pub struct CollectCx<'a> {
    system: System<'a, Px>,
    pub fonts: &'a mut text::FontFaces
}

impl<'a> Deref for CollectCx<'a> {
    type Target = System<'a, Px>;

    fn deref(&self) -> &Self::Target {
        &self.system
    }
}

impl<'a> DerefMut for CollectCx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.system
    }
}

pub type CollectBB<'a> = BB<Variable<'a, Px>>;

pub trait Layout {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>) -> CollectBB<'a>;
    fn bb(&self) -> BB<Px>;
}

pub type ConstrainCx<'a, 'b> = (&'a mut CollectCx<'b>, &'a mut CollectBB<'b>);
pub type RectBB = BB<Cell<Px>>;

pub trait RectBounded {
    fn rect_bb(&self) -> &BB<Cell<Px>>;
    fn name(&self) -> &'static str { "<unnamed>" }
    fn constrain<'a, 'b>(&'a self, _: ConstrainCx<'b, 'a>) {}
}

impl<T> Layout for T where T: RectBounded {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        let mut bb = cx.area(self.rect_bb(), self.name());
        self.constrain((cx, &mut bb));
        bb
    }
    fn bb(&self) -> BB<Px> {
        self.rect_bb().as_ref().map(|x| x.get())
    }
}

pub trait Hit<H> {
    fn hit(&self, H);
}

pub trait Where {
    fn pos(&self) -> [Px; 2];
}

#[cfg(test)]
mod test {
    extern crate test;

    use std::default::Default;

    use ui::layout::{self, RectBB, RectBounded};
    use ui::text::FontFaces;

    struct Demo {
        bb: RectBB,
        name: &'static str
    }

    impl Demo {
        fn new(name: &'static str) -> Demo {
            Demo {
                bb: RectBB::default(),
                name: name
            }
        }
    }

    impl RectBounded for Demo {
        fn rect_bb(&self) -> &RectBB { &self.bb }
        fn name(&self) -> &'static str { self.name }
    }

    fn dummy_fonts() -> FontFaces {
        use std::cell::RefCell;
        use std::rc::Rc;

        use {gfx, gfx_device, glutin};

        let window = glutin::HeadlessRendererBuilder::new(800, 600).build().unwrap();

        unsafe { window.make_current() };

        let (_, factory) = gfx_device::create(|s| window.get_proc_address(s));
        let factory = Rc::new(RefCell::new(factory));

        FontFaces {
            regular: gfx::GlyphCache::new("assets/NotoSans/NotoSans-Regular.ttf", factory.clone()).unwrap(),
            mono: gfx::GlyphCache::new("assets/Hasklig/Hasklig-Regular.otf", factory.clone()).unwrap(),
            mono_bold: gfx::GlyphCache::new("assets/Hasklig/Hasklig-Bold.otf", factory.clone()).unwrap()
        }
    }

    #[bench]
    fn layout(bench: &mut test::Bencher) {
        let mut fonts = dummy_fonts();

        let a = Demo::new("a");
        let b = Demo::new("b");
        let c = Demo::new("c");
        let root = flow![down: a, flow![right: b, c]];

        bench.iter(|| {
            layout::compute(&root, &mut fonts, 800.0, 600.0);
            &root
        });
    }
}
