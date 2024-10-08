//! Set of axioms, typing and reduction rules for the equality type.

use derive_more::Display;

use super::{Axiom, AxiomKind};
use crate::memory::arena::Arena;
use crate::memory::declaration::Declaration;
use crate::memory::level::Level;
use crate::memory::term::Term;

/// Axioms regarding Equality.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Equality {
    /// The equality type.
    ///
    /// The trailing underscore is needed because naming it `Eq` collides with [Eq](std::cmp::Eq).
    #[display(fmt = "Eq")]
    Eq_,

    /// The recursor over the equality type.
    EqRec,

    /// The reflexivity predicate for the equality type.
    Refl,
}

impl<'arena> AxiomKind<'arena> for Equality {
    fn append_to_named_axioms(arena: &mut Arena<'arena>) {
        let var0 = Level::var(0, arena);
        let var1 = Level::var(1, arena);

        let decl = Declaration(Term::axiom(Axiom::Equality(Self::Eq_), &[var0], arena), 1);
        arena.bind_decl("Eq", decl);

        let decl = Declaration(Term::axiom(Axiom::Equality(Self::EqRec), &[var0, var1], arena), 2);
        arena.bind_decl("Eq_rec", decl);

        let decl = Declaration(Term::axiom(Axiom::Equality(Self::Refl), &[var0], arena), 1);
        arena.bind_decl("Refl", decl);
    }

    fn get_type(self, arena: &mut Arena<'arena>) -> Term<'arena> {
        match self {
            Self::Eq_ => Self::type_eq(arena),
            Self::EqRec => Self::type_eq_rec(arena),
            Self::Refl => Self::type_refl(arena),
        }
    }

    fn reduce(term: Term<'arena>, arena: &mut Arena<'arena>) -> Option<Term<'arena>> {
        use crate::memory::term::Payload::{App, Axiom};

        // The multiple `let` statements can be easily rewritten as a pattern match
        // if https://github.com/rust-lang/rfcs/issues/2099 is solved.
        let App(f, a_b_eq) = *term else {
            return None;
        };
        let App(f, b) = *f.whnf(arena) else {
            return None;
        };
        let App(f, motive_refl) = *f.whnf(arena) else {
            return None;
        };
        let App(f, _motive) = *f.whnf(arena) else {
            return None;
        };
        let App(f, a) = *f.whnf(arena) else {
            return None;
        };
        let App(f, _aty) = *f.whnf(arena) else {
            return None;
        };
        let Axiom(super::Axiom::Equality(Self::EqRec), _) = *f.unfold(arena).whnf(arena) else {
            return None;
        };

        b.is_def_eq(a, arena).ok()?;

        let App(f, _) = *a_b_eq.whnf(arena) else {
            return None;
        };
        let App(f, _) = *f else {
            return None;
        };
        let Axiom(super::Axiom::Equality(Self::Refl), _) = *f.unfold(arena).whnf(arena) else {
            return None;
        };

        Some(motive_refl)
    }
}

impl Equality {
    /// Type of the Equality type : `Eq.{u} A x y : Prop`.
    fn type_eq<'arena>(arena: &mut Arena<'arena>) -> Term<'arena> {
        let sort_u = Term::sort(Level::var(0, arena), arena);
        let prop = Term::sort_usize(0, arena);

        // Eq : (A : Sort u) -> A -> A -> Prop
        Term::prod(
            sort_u,
            Term::prod(Term::var(1.into(), sort_u, arena), Term::prod(Term::var(2.into(), sort_u, arena), prop, arena), arena),
            arena,
        )
    }

    /// Type of the recursor over equalities
    /// `Eq_rec : (A : Sort u) -> (a : A) -> (motive : (b : A) -> Eq A a b -> Sort v) -> motive a (Refl A a) -> (b : A) -> (p : Eq A
    /// a b) -> motive b p`
    fn type_eq_rec<'arena>(arena: &mut Arena<'arena>) -> Term<'arena> {
        let sort_u = Term::sort(Level::var(0, arena), arena);
        let sort_v = Term::sort(Level::var(1, arena), arena);

        // motive : (b : A) -> Eq A a b -> Sort v
        let motive = Term::prod(
            Term::var(2.into(), sort_u, arena),
            Term::prod(
                Term::app(
                    Term::app(
                        Term::app(
                            Term::axiom(Axiom::Equality(Self::Eq_), &[Level::var(0, arena)], arena),
                            Term::var(3.into(), sort_u, arena),
                            arena,
                        ),
                        Term::var(2.into(), sort_u, arena),
                        arena,
                    ),
                    Term::var(1.into(), sort_u, arena),
                    arena,
                ),
                sort_v,
                arena,
            ),
            arena,
        );

        // Refl A a
        let refl_a = Term::app(
            Term::app(
                Term::axiom(Axiom::Equality(Self::Refl), &[Level::var(0, arena)], arena),
                Term::var(3.into(), sort_u, arena),
                arena,
            ),
            Term::var(2.into(), sort_u, arena),
            arena,
        );

        // motive a (Refl A a)
        let motive_refl_a =
            Term::app(Term::app(Term::var(1.into(), motive, arena), Term::var(2.into(), sort_u, arena), arena), refl_a, arena);

        // (b : A) -> (p : Eq A a b) -> motive a b p
        let motive_b_p = Term::prod(
            Term::var(4.into(), sort_u, arena),
            Term::prod(
                Term::app(
                    Term::app(
                        Term::app(
                            Term::axiom(Axiom::Equality(Self::Eq_), &[Level::var(0, arena)], arena),
                            Term::var(5.into(), sort_u, arena),
                            arena,
                        ),
                        Term::var(4.into(), sort_u, arena),
                        arena,
                    ),
                    Term::var(1.into(), sort_u, arena),
                    arena,
                ),
                Term::app(
                    Term::app(Term::var(4.into(), motive, arena), Term::var(2.into(), sort_u, arena), arena),
                    Term::var(
                        1.into(),
                        Term::app(
                            Term::app(
                                Term::app(
                                    Term::axiom(Axiom::Equality(Self::Eq_), &[Level::var(0, arena)], arena),
                                    Term::var(6.into(), sort_u, arena),
                                    arena,
                                ),
                                Term::var(5.into(), sort_u, arena),
                                arena,
                            ),
                            Term::var(2.into(), sort_u, arena),
                            arena,
                        ),
                        arena,
                    ),
                    arena,
                ),
                arena,
            ),
            arena,
        );

        // Eq_rec : (A : Sort u) -> (a : A) -> (motive : (b : A) -> Eq A a b -> Sort v) ->
        // motive a (Refl A a) -> (b : A) -> (p : Eq A a b) -> motive b p
        Term::prod(
            sort_u,
            Term::prod(
                Term::var(1.into(), sort_u, arena),
                Term::prod(motive, Term::prod(motive_refl_a, motive_b_p, arena), arena),
                arena,
            ),
            arena,
        )
    }

    /// Type of the reflexivity axiom :
    /// `Refl.{u} A x : Eq.{u} A x x`
    fn type_refl<'arena>(arena: &mut Arena<'arena>) -> Term<'arena> {
        let sort_u = Term::sort(Level::var(0, arena), arena);

        // Eq A a a
        let eq_refl = Term::app(
            Term::app(
                Term::app(
                    Term::axiom(Axiom::Equality(Self::Eq_), &[Level::var(0, arena)], arena),
                    Term::var(2.into(), sort_u, arena),
                    arena,
                ),
                Term::var(1.into(), sort_u, arena),
                arena,
            ),
            Term::var(1.into(), sort_u, arena),
            arena,
        );

        // (A : Sort u) -> (a : A) -> Eq A a a
        Term::prod(sort_u, Term::prod(Term::var(1.into(), sort_u, arena), eq_refl, arena), arena)
    }
}
