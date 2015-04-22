use arena::TypedArena;
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

use ui::{Px, BB, Flow, Dir};

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
struct Variable<'a, T: 'a + Domain> {
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

#[derive(Copy, Clone)]
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

    fn equal(&mut self, a: Variable<'a, T>, b: Variable<'a, T>) {
        self.constraints.push_back(Constraint {
            terms: vec![
                Term { factor: T::one(), var: a },
                Term { factor: -T::one(), var: b }
            ],
            priority: 0,
            bounds: Bounds::equal(T::zero())
        })
    }

    fn order(&mut self, a: Variable<'a, T>, b: Variable<'a, T>) {
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
            while self.constraints.len() > age {
                let mut modified = false;
                let c = self.constraints.pop_front().unwrap();
                //.normalize();
                let Constraint { mut terms, mut bounds, priority } = c;
                let mut new_bounds = Bounds::equal(T::zero());
                terms = terms.into_iter().filter(|t| {
                    if let Some(v) = t.var.value() {
                        bounds = bounds - v * t.factor;
                        modified = true;
                        false
                    } else {
                        new_bounds = new_bounds + t.var.bounds() * t.factor;
                        true
                    }
                }).collect();
                terms.sort_by(|a, b| a.var.cmp(&b.var));

                {
                    let old_bounds = bounds;
                    bounds.restrict(new_bounds);
                    if bounds != old_bounds {
                        modified = true;
                    }
                }

                if terms.is_empty() {
                    modified = true;
                } else if terms.len() == 1 {
                    let t = terms[0];
                    t.var.restrict(bounds / t.factor);
                    modified = true;
                } else {
                    // Opportunistic a - b range reduction.
                    if terms.len() == 2 {
                        let (a, b) = (terms[0], terms[1]);
                        if a.factor == -b.factor {
                            let (a_bounds, b_bounds) = (a.var.bounds(), b.var.bounds());

                            a.var.restrict(b_bounds + bounds / a.factor);
                            b.var.restrict(a_bounds + bounds / b.factor);

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
                    if c.terms.len() != 2 || c.terms[0].factor != -c.terms[1].factor {
                        continue;
                    }
                    (c.terms[0], c.terms[1], c.bounds / c.terms[0].factor)
                };

                // We can't know the ordering.
                if bounds == Bounds::equal(T::zero()) {
                    continue;
                }

                #[derive(Copy, Clone)]
                enum Span<T> {
                    Flex(T),
                    Fixed(T)
                }

                let span = if bounds.min != bounds.max {
                    Span::Flex(T::one())
                } else {
                    Span::Fixed(bounds.min.abs())
                };

                let mut q = VecDeque::new();
                let mut s = VecDeque::new();
                if bounds.max <= T::zero() {
                    // a <= b
                    q.push_back(a.var);
                    q.push_back(b.var);
                } else if bounds.min >= T::zero() {
                    // b <= a
                    q.push_back(b.var);
                    q.push_back(a.var);
                } else {
                    continue;
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
                        if c.terms[0].factor != -c.terms[1].factor {
                            continue;
                        }

                        let a = c.terms[0];
                        let b = c.terms[1];
                        let bounds = c.bounds / a.factor;

                        let span = if bounds.min != bounds.max {
                            Span::Flex(T::one())
                        } else {
                            Span::Fixed(bounds.min.abs())
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

pub fn compute<R: Layout>(root: &R, w: Px, h: Px) {
    // TODO reuse the context to avoid allocating every time.
    let var_arena = TypedArena::new();
    let mut cx = CollectCx::new(&var_arena);
    let r = root.collect(&mut cx);
    r.x1.assign(0.0);
    r.y1.assign(0.0);
    r.x2.assign(w);
    r.y2.assign(h);
    cx.solve();
}

pub type CollectCx<'a> = System<'a, Px>;
pub type CollectBB<'a> = BB<Variable<'a, Px>>;

pub trait Layout {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>) -> CollectBB<'a>;
    fn bb(&self) -> BB<Px>;
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

pub type RectBB = BB<Cell<Px>>;

pub trait RectBounded {
    fn rect_bb(&self) -> &BB<Cell<Px>>;
    fn name(&self) -> &'static str { "<unnamed>" }
}

impl<T> Layout for T where T: RectBounded {
    fn collect<'a>(&'a self, cx: &mut CollectCx<'a>) -> CollectBB<'a> {
        cx.area(self.rect_bb(), self.name())
    }
    fn bb(&self) -> BB<Px> {
        self.rect_bb().as_ref().map(|x| x.get())
    }
}

pub trait Hit<H> {
    fn hit(&self, H);
}

pub trait Where {
    fn pos(&self) -> (Px, Px);
}

pub trait FlowHit<D, H> {
    fn hit(&self, D, H);
}

impl<D: Copy, K, H> Hit<H> for Flow<D, K> where K: FlowHit<D, H> {
    fn hit(&self, hit: H) {
        self.kids.hit(self.dir, hit)
    }
}

impl<D, T, H> FlowHit<D, H> for T where T: Hit<H> {
    fn hit(&self, _: D, hit: H) {
        self.hit(hit)
    }
}

impl<D, A, B, H> FlowHit<D, H> for (A, B) where
           A: Layout + Hit<H>,
           B: FlowHit<D, H>,
           H: Where,
           D: Copy,
           Dir: From<D> {
    fn hit(&self, dir: D, hit: H) {
        let (x, y) = hit.pos();
        let a = self.0.bb();

        let in_a = match Dir::from(dir) {
            Dir::Up => y >= a.y1,
            Dir::Down => y < a.y2,
            Dir::Left => x >= a.x1,
            Dir::Right => x < a.x2
        };

        if in_a {
            self.0.hit(hit);
        } else {
            self.1.hit(dir, hit);
        }
    }
}
