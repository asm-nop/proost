//! A collection of safe functions to build [`Term`]s.
//!
//! This module provides a way for building terms via closures: users can manipulate closures and
//! create bigger ones which, when [built](Arena::build), provide the expected term.
//!
//! The overall syntax remains transparent to the user. This means the user focuses on the
//! structure of the term they want to build, while the [closures](`BuilderTrait`) internally build
//! an appropriate logic: converting regular terms into de Bruijn-compatible ones, assigning types
//! to variables, etc.

use derive_more::Display;
use im_rc::hashmap::HashMap as ImHashMap;

use super::{DeBruijnIndex, Term};
use crate::error::{Error, ResultTerm};
use crate::memory::arena::Arena;
use crate::memory::declaration::builder as declaration;
use crate::memory::level::builder as level;
use crate::trace::{Trace, TraceableError};

/// The kind of errors that can occur when building a [`Term`].
#[non_exhaustive]
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum ErrorKind<'arena> {
    /// Unknown identifier
    #[display(fmt = "unknown identifier {_0}")]
    ConstNotFound(&'arena str),
}

/// Local environment used to store correspondence between locally-bound variables and the pair
/// (depth at which they were bound, their type).
pub type Environment<'build, 'arena> = ImHashMap<&'build str, (DeBruijnIndex, Term<'arena>)>;

/// The trait of closures which build terms with an adequate logic.
///
/// A call with a quadruplet of arguments `(arena, env, lvl_env, index)` of a closure with this
/// trait should build a definite term in the [`Arena`] `arena`, knowing the bindings declared in
/// `env` and `lvl_env`, provided that the term is built at a current depth `index`.
///
/// Please note that this is just a trait alias, meaning it enforces few constraints: while
/// functions in this module returning a closure with this trait are guaranteed to be sound, end
/// users can also create their own closures satisfying `BuilderTrait`; this should be avoided.
#[allow(clippy::module_name_repetitions)]
pub trait BuilderTrait<'build> = for<'arena> FnOnce(
    &mut Arena<'arena>,
    &Environment<'build, 'arena>,
    &level::Environment<'build>,
    DeBruijnIndex,
) -> ResultTerm<'arena>;

impl<'arena> Arena<'arena> {
    /// Returns the term built from the given closure, provided with an empty context, at depth 0.
    ///
    /// # Errors
    /// If the term could not be built, yields an error indicating the reason.
    #[inline]
    pub fn build<'build, F: BuilderTrait<'build>>(&mut self, f: F) -> ResultTerm<'arena> {
        f(self, &Environment::new(), &level::Environment::new(), 0.into())
    }
}

/// Returns a closure building a variable associated to the name `name`.
#[inline]
#[must_use]
pub const fn var(name: &str) -> impl BuilderTrait<'_> {
    move |arena, env, _, depth| {
        env.get(name)
            .map(|&(bind_depth, term)| {
                // This is arguably an eager computation, it could be worth making it lazy,
                // or at least memoizing it so as to not compute it again
                let var_type = term.shift(usize::from(depth - bind_depth), 0, arena);
                Term::var(depth - bind_depth, var_type, arena)
            })
            .or_else(|| arena.get_binding(name))
            .ok_or_else(|| Error::new(ErrorKind::ConstNotFound(arena.store_name(name)).into()))
    }
}

/// Returns a closure building the Prop term.
#[inline]
#[must_use]
pub const fn prop<'build>() -> impl BuilderTrait<'build> {
    |arena, _, _, _| Ok(Term::prop(arena))
}

/// Returns a closure building the Type `level` term.
#[inline]
#[coverage(off)]
pub const fn type_<'build, F: level::BuilderTrait<'build>>(level: F) -> impl BuilderTrait<'build> {
    move |arena, _, lvl_env, _| Ok(Term::sort(level(arena, lvl_env)?.succ(arena), arena))
}

/// Returns a closure building the Type `level` term (indirection from `usize`).
#[inline]
#[must_use]
pub const fn type_usize<'build>(level: u32) -> impl BuilderTrait<'build> {
    move |arena, _, _, _| Ok(Term::type_usize(level, arena))
}

/// Returns a closure building the Sort `level` term.
#[inline]
#[coverage(off)]
pub const fn sort<'build, F: level::BuilderTrait<'build>>(level: F) -> impl BuilderTrait<'build> {
    move |arena, _, lvl_env, _| Ok(Term::sort(level(arena, lvl_env)?, arena))
}

/// Returns a closure building the Sort `level` term (indirection from `usize`).
#[inline]
#[must_use]
pub const fn sort_usize<'build>(level: u32) -> impl BuilderTrait<'build> {
    move |arena, _, _, _| Ok(Term::sort_usize(level, arena))
}

/// Returns a closure building the application of two terms built from the given closures `u1` and
/// `u2`.
#[inline]
#[coverage(off)]
pub const fn app<'build, F1: BuilderTrait<'build>, F2: BuilderTrait<'build>>(u1: F1, u2: F2) -> impl BuilderTrait<'build> {
    |arena, env, lvl_env, depth| {
        let u1 = u1(arena, env, lvl_env, depth).trace_err(Trace::Left)?;
        let u2 = u2(arena, env, lvl_env, depth).trace_err(Trace::Right)?;
        Ok(u1.app(u2, arena))
    }
}

/// Returns a closure building the lambda-abstraction with a body built from `body` and an argument
/// type from `arg_type`.
#[inline]
#[coverage(off)]
pub const fn abs<'build, F1: BuilderTrait<'build>, F2: BuilderTrait<'build>>(
    name: &'build str,
    arg_type: F1,
    body: F2,
) -> impl BuilderTrait<'build> {
    move |arena, env, lvl_env, depth| {
        let arg_type = arg_type(arena, env, lvl_env, depth).trace_err(Trace::Left)?;
        let body = if name == "_" {
            body(arena, env, lvl_env, depth + 1.into()).trace_err(Trace::Right)?
        } else {
            let env = env.update(name, (depth, arg_type));
            body(arena, &env, lvl_env, depth + 1.into()).trace_err(Trace::Right)?
        };
        Ok(arg_type.abs(body, arena))
    }
}

/// Returns a closure building the dependant product of a term built from `body` over all elements
/// of the type built from `arg_type`.
#[inline]
#[coverage(off)]
pub const fn prod<'build, F1: BuilderTrait<'build>, F2: BuilderTrait<'build>>(
    name: &'build str,
    arg_type: F1,
    body: F2,
) -> impl BuilderTrait<'build> {
    move |arena, env, lvl_env, depth| {
        let arg_type = arg_type(arena, env, lvl_env, depth).trace_err(Trace::Left)?;
        let body = if name == "_" {
            body(arena, env, lvl_env, depth + 1.into()).trace_err(Trace::Right)?
        } else {
            let env = env.update(name, (depth, arg_type));
            body(arena, &env, lvl_env, depth + 1.into()).trace_err(Trace::Right)?
        };
        Ok(arg_type.prod(body, arena))
    }
}

/// Returns a closure building the term associated to the instantiated declaration `decl`.
#[inline]
#[coverage(off)]
pub const fn decl<'build, F: declaration::InstantiatedBuilderTrait<'build>>(decl: F) -> impl BuilderTrait<'build> {
    move |arena, _, lvl_env, _| Ok(Term::decl(decl(arena, lvl_env)?, arena))
}

#[cfg(test)]
pub(crate) mod raw {
    use super::*;

    pub trait BuilderTrait = for<'arena> FnOnce(&mut Arena<'arena>) -> Term<'arena>;

    impl<'arena> Arena<'arena> {
        pub(crate) fn build_term_raw<F: BuilderTrait>(&mut self, f: F) -> Term<'arena> {
            f(self)
        }
    }

    pub const fn var<F: BuilderTrait>(index: DeBruijnIndex, type_: F) -> impl BuilderTrait {
        move |arena| {
            let ty = type_(arena);
            Term::var(index, ty, arena)
        }
    }

    #[allow(clippy::redundant_closure)]
    pub const fn prop() -> impl BuilderTrait {
        |arena| Term::prop(arena)
    }

    pub const fn type_usize(level: u32) -> impl BuilderTrait {
        move |arena| Term::type_usize(level, arena)
    }

    pub const fn sort_<F: level::raw::BuilderTrait>(level: F) -> impl BuilderTrait {
        move |arena| Term::sort(level(arena), arena)
    }

    pub const fn app<F1: BuilderTrait, F2: BuilderTrait>(u1: F1, u2: F2) -> impl BuilderTrait {
        |arena| {
            let u1 = u1(arena);
            let u2 = u2(arena);
            u1.app(u2, arena)
        }
    }

    pub const fn abs<F1: BuilderTrait, F2: BuilderTrait>(u1: F1, u2: F2) -> impl BuilderTrait {
        |arena| {
            let u1 = u1(arena);
            let u2 = u2(arena);
            u1.abs(u2, arena)
        }
    }

    pub const fn prod<F1: BuilderTrait, F2: BuilderTrait>(u1: F1, u2: F2) -> impl BuilderTrait {
        |arena| {
            let u1 = u1(arena);
            let u2 = u2(arena);
            u1.prod(u2, arena)
        }
    }
}
