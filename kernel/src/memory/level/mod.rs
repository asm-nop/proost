//! Universe levels.

use core::fmt::{Display, Formatter};
use core::hash::Hash;
use core::marker::PhantomData;

use super::arena::Arena;

super::arena::new_dweller!(Level, Header, Payload);

pub mod builder;

/// The header of a level.
#[derive(Default)]
struct Header<'arena> {
    /// The header of a level contains nothing so far.
    /// It contained the "plus form" of a level before, but this is now basically redundant with the
    /// `Add` constructor.
    preserve: PhantomData<&'arena ()>,
}

/// A universe level.
///
/// While types in the usual calculus of constructions live in types fully described with integers,
/// the addition of non-instantiated universe variables requires the use of values like `Max` to
/// describe formal computations on levels.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Payload<'arena> {
    /// The zero level (associated to Prop)
    Zero,

    /// The successor of a level
    Add(Level<'arena>, u32),

    /// The maximum of two levels
    Max(Level<'arena>, Level<'arena>),

    /// The impredicative maximum of two levels
    IMax(Level<'arena>, Level<'arena>),

    /// A universe-polymorphic variable
    Var(usize),
}

impl Display for Level<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self.to_numeral() {
            Some(n) => write!(f, "{n}"),
            None => match **self {
                Zero => unreachable!("Zero is a numeral"),
                Add(u, n) => write!(f, "({u} + {n})"),
                Max(n, m) => write!(f, "(max {n} {m})"),
                IMax(n, m) => write!(f, "(imax {n} {m})"),
                Var(n) => write!(f, "u{n}"),
            },
        }
    }
}

use Payload::{Add, IMax, Max, Var, Zero};

impl<'arena> Level<'arena> {
    /// This function is the base low-level function for creating levels.
    ///
    /// It enforces the uniqueness property of levels in the arena, as well as the reduced-form
    /// invariant.
    fn hashcons(payload: Payload<'arena>, arena: &mut Arena<'arena>) -> Self {
        if let Some(level) = arena.hashcons_levels.get(&payload) {
            *level
        } else {
            // add the unreduced node to the arena
            let node_unreduced = Node {
                payload,
                header: Header::default(),
            };
            let node_unreduced = &*arena.alloc.alloc(node_unreduced);
            let level_unreduced = Level::new(node_unreduced);
            arena.hashcons_levels.insert(&level_unreduced.0.payload, level_unreduced);

            // compute its reduced form
            let reduced = level_unreduced.normalize(arena);

            // supersede the previous correspondence
            arena.hashcons_levels.insert(&level_unreduced.0.payload, reduced);
            arena.hashcons_levels.insert(&reduced.0.payload, reduced);

            reduced
        }
    }

    /// Returns the 0-level
    pub(crate) fn zero(arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(Zero, arena)
    }

    /// Returns the successor of a level.
    pub(crate) fn succ(self, arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(Add(self, 1), arena)
    }

    /// Returns the level + n.
    pub(crate) fn add(self, n: u32, arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(Add(self, n), arena)
    }

    /// Returns the level corresponding to the maximum of two levels.
    pub(crate) fn max(self, right: Self, arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(Max(self, right), arena)
    }

    /// Returns the level corresponding to the impredicative maximum of two levels.
    pub(crate) fn imax(self, right: Self, arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(IMax(self, right), arena)
    }

    /// Returns the level associated to a given variable.
    pub(crate) fn var(id: usize, arena: &mut Arena<'arena>) -> Self {
        Self::hashcons(Var(id), arena)
    }

    /// Builds a level from an integer.
    #[inline]
    #[must_use]
    pub fn from(n: u32, arena: &mut Arena<'arena>) -> Self {
        Level::zero(arena).add(n, arena)
    }

    /// Converts a level to an integer, if possible.
    #[inline]
    #[must_use]
    pub fn to_numeral(self) -> Option<u32> {
        match *self {
            Zero => Some(0),
            Add(u, n) => (*u == Zero).then_some(n),
            _ => None,
        }
    }

    /// Helper function for universe comparison. normalizes imax(es) as follows:
    ///  - `imax(0, u) = u`
    ///  - `imax(u, 0) = u`
    ///  - `imax(u, S(v)) = max(u, S(v))`
    ///  - `imax(u, imax(v, w)) = max(imax(u, w), imax(v, w))`
    ///  - `imax(u, max(v, w)) = max(imax(u, v), imax(u, w))`
    ///
    /// The function also reduces `max`s and `add`s. This is further helpful when trying to print the type.
    ///
    /// Here, the imax normalization pushes imaxes to all have a `Var(i)` as the second argument. To solve this last case, one needs
    /// to substitute `Var(i)` with `0` and `S(Var(i))`. This gives us a consistent way to unstuck the geq-checking.
    fn normalize(self, arena: &mut Arena<'arena>) -> Self {
        match *self {
            IMax(u, v) => {
                if u == v {
                    u
                } else {
                    match *v {
                        Zero => v,
                        Add(_, _k) => u.max(v, arena), // k > 0 by invariant
                        IMax(_, vw) => Level::max(u.imax(vw, arena), v, arena),
                        Max(vv, vw) => Level::max(u.imax(vv, arena), u.imax(vw, arena), arena),
                        _ => self,
                    }
                }
            },

            Max(u, v) => {
                if u == v {
                    u
                } else {
                    match (&*u, &*v) {
                        (&Zero, _) => v,
                        (_, &Zero) => u,
                        (&Add(uu, k1), &Add(vv, k2)) => {
                            let min = k1.min(k2);
                            Level::max(uu.add(k1 - min, arena), vv.add(k2 - min, arena), arena).add(min, arena)
                        },
                        _ => self,
                    }
                }
            },

            Add(u, 0) => u,
            Add(u, k) if let Add(u, n) = *u => u.add(n + k, arena),

            _ => self,
        }
    }
}

#[cfg(test)]
mod pretty_printing {

    use crate::memory::arena::use_arena;
    use crate::memory::level::builder::raw::*;

    #[test]
    fn to_num() {
        use_arena(|arena| {
            assert_eq!(arena.build_level_raw(max(succ(zero()), zero())).to_numeral(), Some(1));
            assert_eq!(arena.build_level_raw(max(succ(zero()), succ(var(0)))).to_numeral(), None);
            assert_eq!(arena.build_level_raw(imax(var(0), zero())).to_numeral(), Some(0));
            assert_eq!(arena.build_level_raw(imax(zero(), succ(zero()))).to_numeral(), Some(1));
        });
    }

    #[test]
    fn to_pretty_print() {
        use_arena(|arena| {
            assert_eq!(
                format!("{}", arena.build_level_raw(max(succ(zero()), imax(max(zero(), var(0)), succ(var(0)))))),
                "(max 1 (max u0 (u0 + 1)))"
            );
        });
    }

    #[test]
    fn normalize() {
        use_arena(|arena| {
            let lvl = arena.build_level_raw(imax(zero(), imax(zero(), imax(succ(zero()), var(0)))));

            assert_eq!(format!("{lvl}"), "(max (imax 0 u0) (max (imax 0 u0) (imax 1 u0)))");
        });
    }
}
