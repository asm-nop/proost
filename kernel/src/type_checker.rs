//! Type checking functions.
//!
//! The logical core of the kernel.

use derive_more::Display;

use crate::error::{Error, Result, ResultTerm};
use crate::memory::arena::Arena;
use crate::memory::declaration::Declaration;
use crate::memory::term::Payload::{Abs, App, Axiom, Decl, Prod, Sort, Var};
use crate::memory::term::Term;
use crate::trace::{Trace, TraceableError};

/// A pair of terms, where the second is the type of the first.
///
/// This type is only used for pretty-printing purposes.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(fmt = "{_0}: {_1}")]
pub struct TypedTerm<'arena>(Term<'arena>, Term<'arena>);

/// Errors that can occur, at runtime, during type checking.
#[allow(clippy::module_name_repetitions)]
#[non_exhaustive]
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum ErrorKind<'arena> {
    /// This term is not a universe.
    #[display(fmt = "{_0} is not a universe")]
    NotUniverse(Term<'arena>),

    /// These two terms are not definitionally equal.
    #[display(fmt = "{_0} and {_1} are not definitionally equal")]
    NotDefEq(Term<'arena>, Term<'arena>),

    /// This function expected an argument of this type, received an argument of this other type.
    #[display(fmt = "function {_0} expects a term of type {_1}, received {_2}")]
    WrongArgumentType(Term<'arena>, Term<'arena>, TypedTerm<'arena>),

    /// This is not a function, it cannot be applied to this.
    #[display(fmt = "{_0} is not a function, it cannot be applied to {_1}")]
    NotAFunction(TypedTerm<'arena>, Term<'arena>),

    /// These types mismatch.
    #[display(fmt = "expected {_0}, got {_1}")]
    TypeMismatch(Term<'arena>, Term<'arena>),
}

impl<'arena> Term<'arena> {
    /// Conversion function, checks whether two terms are definitionally equal.
    ///
    /// The conversion is untyped, meaning that it should *only* be called during type-checking
    /// when the two [`Term`]s are already known to be of the same type and in the same context.
    fn conversion(self, rhs: Self, arena: &mut Arena<'arena>) -> bool {
        if self == rhs {
            return true;
        }

        // We assume that self and rhs have the same type. As such, we only need to check whether
        if !self.is_relevant(arena) {
            return true;
        }

        let lhs = self.whnf(arena);
        let rhs = rhs.whnf(arena);

        if lhs == rhs {
            return true;
        }

        match (&*lhs, &*rhs) {
            (&Sort(l1), &Sort(l2)) => l1.is_eq(l2, arena),

            (&Var(i, _), &Var(j, _)) => i == j,

            (&Prod(t1, u1), &Prod(t2, u2)) => t1.conversion(t2, arena) && u1.conversion(u2, arena),

            // Since we assume that both values already have the same type,
            // checking conversion over the argument type is useless.
            // However, this doesn't mean we can simply remove the arg type
            // from the type constructor in the enum, it is needed to quote back to terms.
            (&Abs(_, t), &Abs(_, u)) => t.conversion(u, arena),

            (&App(t1, u1), &App(t2, u2)) => t1.conversion(t2, arena) && u1.conversion(u2, arena),

            // We do not automatically unfold definitions during normalisation because of how costly it is.
            // Instead, when the same declaration is met on both terms, they're also equal in memory.
            // Otherwise, either one of them is not a decl, or they are two different decls. In both case, we unfold decls to check
            // equality.
            (&Decl(decl), _) => decl.get_term(arena).conversion(rhs, arena),

            (_, &Decl(decl)) => decl.get_term(arena).conversion(lhs, arena),

            _ => false,
        }
    }

    /// Checks whether two terms are definitionally equal.
    ///
    /// # Errors
    /// Yields an error indicating that the two terms are not definitionally equal.
    #[inline]
    pub fn is_def_eq(self, rhs: Self, arena: &mut Arena<'arena>) -> Result<'arena, ()> {
        self.conversion(rhs, arena)
            .then_some(())
            .ok_or_else(|| Error::new(ErrorKind::NotDefEq(self, rhs).into()))
    }

    /// Computes the universe in which `(x: A) -> B` lives when `A: lhs` and `B: rhs`.
    fn imax(self, rhs: Self, arena: &mut Arena<'arena>) -> ResultTerm<'arena> {
        match (&*self, &*rhs) {
            (&Sort(l1), &Sort(l2)) => {
                let lvl = l1.imax(l2, arena);
                Ok(Term::sort(lvl, arena))
            },

            (&Sort(_), _) => Err(Error::new(ErrorKind::NotUniverse(rhs).into())).trace_err(Trace::Right),
            (_, _) => Err(Error::new(ErrorKind::NotUniverse(self).into())).trace_err(Trace::Left),
        }
    }

    /// Infers the type of the term `self`, living in the arena `arena`.
    ///
    /// # Errors
    /// If the term cannot be typed, this function yields an error indicating where the problem is.
    #[inline]
    pub fn infer(self, arena: &mut Arena<'arena>) -> ResultTerm<'arena> {
        self.get_type_or_try_init(|| match *self {
            Sort(lvl) => Ok(Term::sort(lvl.succ(arena), arena)),
            Var(_, type_) => Ok(type_),
            Axiom(ax, lvl) => Ok(ax.get_type(arena).substitute_univs(lvl, arena)),

            Prod(t, u) => {
                let univ_t = t.infer(arena).trace_err(Trace::Left)?;
                let univ_u = u.infer(arena).trace_err(Trace::Right)?;

                let univ_t = univ_t.whnf(arena);
                let univ_u = univ_u.whnf(arena);
                univ_t.imax(univ_u, arena)
            },

            Abs(t, u) => {
                let type_t = t.infer(arena).trace_err(Trace::Left)?;

                match *type_t {
                    Sort(_) => {
                        let type_u = u.infer(arena).trace_err(Trace::Right)?;
                        Ok(t.prod(type_u, arena))
                    },

                    _ => Err(Error::new(ErrorKind::NotUniverse(type_t).into())).trace_err(Trace::Left),
                }
            },

            App(t, u) => {
                let type_t = t.infer(arena).trace_err(Trace::Left)?;
                let type_t = type_t.whnf(arena);

                match *type_t {
                    Prod(arg_type, cls) => {
                        let type_u = u.infer(arena).trace_err(Trace::Right)?;

                        if type_u.conversion(arg_type, arena) {
                            Ok(cls.substitute(u, 1, arena))
                        } else {
                            Err(Error::new(ErrorKind::WrongArgumentType(t, arg_type, TypedTerm(u, type_u)).into()))
                        }
                    },

                    _ => Err(Error::new(ErrorKind::NotAFunction(TypedTerm(t, type_t), u).into())).trace_err(Trace::Left),
                }
            },

            Decl(decl) => decl.get_type_or_try_init(Term::infer, arena),
        })
    }

    /// Checks whether the term `self` living in `arena` is of type `ty`.
    ///
    /// # Errors
    /// If `self` cannot be typed, or `ty` is not the type of `self`, this yields the corresponding
    /// error.
    #[inline]
    pub fn check(self, ty: Self, arena: &mut Arena<'arena>) -> Result<'arena, ()> {
        let tty = self.infer(arena)?;

        tty.conversion(ty, arena)
            .then_some(())
            .ok_or_else(|| Error::new(ErrorKind::TypeMismatch(tty, ty).into()))
    }
}

impl<'arena> Declaration<'arena> {
    /// Infers the type of a declaration.
    ///
    /// Because it is not allowed to access the underlying term of a declaration, this function
    /// does not return anything, and only serves as a way to ensure the declaration is
    /// well-formed.
    ///
    /// # Errors
    /// If the declaration cannot be typed, this function yields an error indicating where the problem is.
    #[inline]
    pub fn infer(self, arena: &mut Arena<'arena>) -> Result<'arena, ()> {
        self.0.infer(arena)?;
        Ok(())
    }

    /// Checks whether the declaration `self` living in `arena` is of type `ty`.
    ///
    /// # Errors
    /// If `self` cannot be typed, or `ty` is not the type of `self`, this yields the corresponding
    /// error.
    #[inline]
    pub fn check(self, ty: Self, arena: &mut Arena<'arena>) -> Result<'arena, ()> {
        self.0.check(ty.0, arena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::arena::use_arena;
    use crate::memory::declaration::InstantiatedDeclaration;
    use crate::memory::term::builder::raw::*;

    fn id() -> impl BuilderTrait {
        abs(prop(), var(1.into(), prop()))
    }

    #[test]
    fn def_eq_1() {
        use_arena(|arena| {
            let term = arena.build_term_raw(app(abs(prop(), id()), id()));
            let normal_form = arena.build_term_raw(abs(prop(), var(1.into(), prop())));

            assert!(term.is_def_eq(normal_form, arena).is_ok());
        });
    }

    #[test]
    fn def_eq_2() {
        use_arena(|arena| {
            let term = arena.build_term_raw(app(abs(prop(), abs(prop(), var(2.into(), prop()))), id()));
            let normal_form = arena.build_term_raw(abs(prop(), id()));

            assert!(term.is_def_eq(normal_form, arena).is_ok());
        });
    }

    #[test]
    fn def_eq_self() {
        use_arena(|arena| {
            // λa.a (λx.x) (λx.x)
            let term = arena.build_term_raw(abs(prop(), app(app(var(2.into(), prop()), id()), id())));

            assert!(term.is_def_eq(term, arena).is_ok());
        });
    }

    #[test]
    fn conv_decl() {
        use_arena(|arena| {
            let decl_ = InstantiatedDeclaration::instantiate(Declaration(Term::prop(arena), 0), &Vec::new(), arena);
            let term = Term::decl(decl_, arena);

            let prop = arena.build_term_raw(prop());

            assert!(term.is_def_eq(prop, arena).is_ok());
            assert!(prop.is_def_eq(term, arena).is_ok());
        });
    }

    #[test]
    fn infer_decl() {
        use_arena(|arena| {
            let decl_ = InstantiatedDeclaration::instantiate(Declaration(Term::prop(arena), 0), &Vec::new(), arena);

            let term = Term::decl(decl_, arena);
            let ty = arena.build_term_raw(type_usize(0));

            assert!(term.check(ty, arena).is_ok());
        });
    }

    #[test]
    fn failed_def_equal() {
        use_arena(|arena| {
            let term_lhs = Term::prop(arena);
            let term_rhs = Term::type_usize(0, arena);

            assert_eq!(
                term_lhs.is_def_eq(term_rhs, arena),
                Err(Error {
                    kind: ErrorKind::NotDefEq(term_lhs, term_rhs).into(),
                    trace: vec![]
                })
            );
        });
    }

    #[test]
    fn failed_prod_binder_conversion() {
        use_arena(|arena| {
            let term_lhs = arena.build_term_raw(prod(prop(), prop()));
            let term_rhs = arena.build_term_raw(prod(type_usize(0), prop()));

            assert_eq!(
                term_lhs.is_def_eq(term_rhs, arena),
                Err(Error {
                    kind: ErrorKind::NotDefEq(term_lhs, term_rhs).into(),
                    trace: vec![]
                })
            );
        });
    }

    #[test]
    fn failed_app_head_conversion() {
        use_arena(|arena| {
            let term_lhs = arena.build_term_raw(abs(type_usize(0), abs(type_usize(0), app(var(1.into(), type_usize(0)), prop()))));
            let term_rhs = arena.build_term_raw(abs(type_usize(0), abs(type_usize(0), app(var(2.into(), type_usize(0)), prop()))));

            assert_eq!(
                term_lhs.is_def_eq(term_rhs, arena),
                Err(Error {
                    kind: ErrorKind::NotDefEq(term_lhs, term_rhs).into(),
                    trace: vec![]
                })
            );
        });
    }

    #[test]
    fn typed_reduction_app_1() {
        use_arena(|arena| {
            let type_0 = Term::type_usize(0, arena);
            let term = arena.build_term_raw(app(abs(type_usize(0), var(1.into(), type_usize(0))), prop()));

            let reduced = arena.build_term_raw(prop());
            assert!(term.is_def_eq(reduced, arena).is_ok());

            let term_type = term.infer(arena).unwrap();
            assert_eq!(term_type, type_0);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    // this test uses more intricate terms. In order to preserve some readability,
    // switching to extern_build, which is clearer.
    fn typed_reduction_app_2() {
        use crate::memory::term::builder::*;
        use_arena(|arena| {
            // (λa.λb.λc.a (λd.λe.e (d b)) (λ_.c) (λd.d)) (λf.λg.f g)
            let term = arena
                .build(app(
                    abs(
                        "a",
                        // a: ((P → P) → (P → P) → P) → ((P → P) → ((P → P) → P))
                        prod(
                            "_",
                            // (P → P) → ((P → P) → P)
                            prod(
                                "_",
                                // P -> P
                                prod("_", prop(), prop()),
                                // (P -> P) -> P
                                prod("_", prod("_", prop(), prop()), prop()),
                            ),
                            // (P → P) → ((P → P) → P)
                            prod(
                                "_",
                                // P -> P
                                prod("_", prop(), prop()),
                                // (P -> P) -> P
                                prod("_", prod("_", prop(), prop()), prop()),
                            ),
                        ),
                        abs(
                            "b",
                            prop(),
                            abs(
                                "c",
                                prop(),
                                app(
                                    app(
                                        app(
                                            var("a"),
                                            abs(
                                                "d",
                                                prod("_", prop(), prop()),
                                                abs("e", prod("_", prop(), prop()), app(var("e"), app(var("d"), var("b")))),
                                            ),
                                        ),
                                        // _ : P
                                        abs("_", prop(), var("c")),
                                    ),
                                    // d : P
                                    abs("d", prop(), var("d")),
                                ),
                            ),
                        ),
                    ),
                    // f: (P -> P) -> (P -> P) -> P
                    abs(
                        "f",
                        prod("_", prod("_", prop(), prop()), prod("_", prod("_", prop(), prop()), prop())),
                        abs(
                            "g",
                            // g: P -> P
                            prod("_", prop(), prop()),
                            app(var("f"), var("g")),
                        ),
                    ),
                ))
                .unwrap();

            // λa: P. λb: P. b
            let reduced = arena.build(abs("_", prop(), abs("x", prop(), var("x")))).unwrap();
            assert!(term.is_def_eq(reduced, arena).is_ok());

            let term_type = term.infer(arena).unwrap();
            let expected_type = arena.build(prod("_", prop(), prod("_", prop(), prop()))).unwrap();
            assert_eq!(term_type, expected_type);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn typed_reduction_universe() {
        use_arena(|arena| {
            let type_0 = Term::type_usize(0, arena);
            let type_1 = Term::type_usize(1, arena);

            let term = arena.build_term_raw(app(abs(prop(), type_usize(0)), prod(prop(), var(1.into(), prop()))));

            assert!(term.is_def_eq(type_0, arena).is_ok());

            let term_type = term.infer(arena).unwrap();
            assert_eq!(term_type, type_1);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn escape_from_prop() {
        use_arena(|arena| {
            let term = arena.build_term_raw(abs(prop(), prod(var(1.into(), prop()), type_usize(0))));

            let term_type = term.infer(arena).unwrap();
            let expected_type = arena.build_term_raw(prod(prop(), type_usize(1)));
            assert_eq!(term_type, expected_type);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn illtyped_reduction() {
        use_arena(|arena| {
            // λ(x: P -> P).(λ(y: P).x y) x
            // λP -> P.(λP.2 1) 1
            let term = arena.build_term_raw(abs(
                prod(prop(), prop()),
                app(
                    abs(prop(), app(var(2.into(), prod(prop(), prop())), var(1.into(), prop()))),
                    var(1.into(), prod(prop(), prop())),
                ),
            ));

            // λx.x x
            let reduced = arena.build_term_raw(abs(prop(), app(var(1.into(), prop()), var(1.into(), prop()))));

            assert!(term.is_def_eq(reduced, arena).is_ok());
        });
    }

    #[test]
    fn typed_prod_1() {
        use_arena(|arena| {
            let type_0 = Term::type_usize(0, arena);
            let term = arena.build_term_raw(prod(prop(), prop()));
            let term_type = term.infer(arena).unwrap();

            assert_eq!(term_type, type_0);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn typed_prod_2() {
        use_arena(|arena| {
            let term = arena.build_term_raw(prod(prop(), var(1.into(), prop())));
            let term_type = term.infer(arena).unwrap();

            assert_eq!(term_type, Term::prop(arena));
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn typed_prod_3() {
        use_arena(|arena| {
            let term = arena.build_term_raw(abs(prop(), abs(var(1.into(), prop()), var(1.into(), var(2.into(), prop())))));
            let term_type = term.infer(arena).unwrap();
            let expected_type = arena.build_term_raw(prod(prop(), prod(var(1.into(), prop()), var(2.into(), prop()))));

            assert_eq!(term_type, expected_type);
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn typed_polymorphism() {
        use_arena(|arena| {
            let identity = arena
                .build_term_raw(abs(type_usize(0), abs(var(1.into(), type_usize(0)), var(1.into(), var(2.into(), type_usize(0))))));

            let identity_type = identity.infer(arena).unwrap();
            let expected_type =
                arena.build_term_raw(prod(type_usize(0), prod(var(1.into(), type_usize(0)), var(2.into(), type_usize(0)))));

            assert_eq!(identity_type, expected_type);
            assert!(identity.check(identity_type, arena).is_ok());
        });
    }

    #[test]
    fn typed_polymorphism_2() {
        use_arena(|arena| {
            let term = arena.build_term_raw(abs(
                prop(),
                abs(
                    prop(),
                    abs(
                        prod(prop(), prod(prop(), prop())),
                        app(app(var(1.into(), prod(prop(), prod(prop(), prop()))), var(3.into(), prop())), var(2.into(), prop())),
                    ),
                ),
            ));
            let term_type = term.infer(arena).unwrap();

            assert_eq!(
                term_type,
                arena.build_term_raw(prod(prop(), prod(prop(), prod(prod(prop(), prod(prop(), prop())), prop()))))
            );
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn type_hierarchy_prop() {
        use_arena(|arena| {
            let term = Term::prop(arena);
            let term_type = term.infer(arena).unwrap();

            assert_eq!(term_type, Term::type_usize(0, arena));
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn type_hierarchy_type() {
        use_arena(|arena| {
            let term = Term::type_usize(0, arena);
            let term_type = term.infer(arena).unwrap();

            assert_eq!(term_type, Term::type_usize(1, arena));
            assert!(term.check(term_type, arena).is_ok());
        });
    }

    #[test]
    fn irrelevance_conversion() {
        use crate::axiom::false_::False::{False, FalseRec};
        use crate::axiom::Axiom;
        use crate::memory::level::Level;

        use_arena(|arena| {
            let false_ = Term::axiom(Axiom::False(False), &[], arena);
            let false_rec = Term::axiom(Axiom::False(FalseRec), &[Level::zero(arena)], arena);
            let tt1 = false_.abs(Term::var(1.into(), false_, arena), arena);
            let tt2 = false_.abs(false_rec.app(false_, arena).app(Term::var(1.into(), false_, arena), arena), arena);
            assert!(tt1.conversion(tt2, arena));
        });
    }

    mod failed_type_inference {
        use super::*;

        #[test]
        fn not_function_abs() {
            use_arena(|arena| {
                let term = arena.build_term_raw(abs(app(prop(), prop()), prop()));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left, Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn not_function_prod_1() {
            use_arena(|arena| {
                let term = arena.build_term_raw(prod(prop(), app(prop(), prop())));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left, Trace::Right]
                    })
                );
            });
        }

        #[test]
        fn not_function_prod_2() {
            use_arena(|arena| {
                let term = arena.build_term_raw(prod(app(prop(), prop()), prop()));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left, Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn not_function_app_1() {
            use_arena(|arena| {
                let term = arena.build_term_raw(app(prop(), prop()));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn not_function_app_2() {
            use_arena(|arena| {
                let term = arena.build_term_raw(app(app(prop(), prop()), prop()));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left, Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn not_function_app_3() {
            use_arena(|arena| {
                let term = arena.build_term_raw(app(abs(prop(), prop()), app(prop(), prop())));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left, Trace::Right]
                    })
                );
            });
        }

        #[test]
        fn wrong_argument_type() {
            use_arena(|arena| {
                // λ(x: P -> P).(λ(y: P).x y) x
                // λP -> P.(λP.2 1) 1
                let term = arena.build_term_raw(abs(
                    prod(prop(), prop()),
                    app(
                        abs(prop(), app(var(2.into(), prod(prop(), prop())), var(1.into(), prop()))),
                        var(1.into(), prod(prop(), prop())),
                    ),
                ));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::WrongArgumentType(
                            arena.build_term_raw(abs(prop(), app(var(2.into(), prod(prop(), prop())), var(1.into(), prop())))),
                            Term::prop(arena),
                            TypedTerm(
                                arena.build_term_raw(var(1.into(), prod(prop(), prop()))),
                                arena.build_term_raw(prod(prop(), prop()))
                            )
                        )
                        .into(),
                        trace: vec![Trace::Right]
                    })
                );
            });
        }

        #[test]
        fn not_universe_abs() {
            use_arena(|arena| {
                let term = arena.build_term_raw(abs(
                    prop(),
                    abs(var(1.into(), prop()), abs(var(1.into(), var(2.into(), prop())), var(1.into(), var(2.into(), prop())))),
                ));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotUniverse(arena.build_term_raw(var(2.into(), prop()))).into(),
                        trace: vec![Trace::Left, Trace::Right, Trace::Right]
                    })
                );
            });
        }

        #[test]
        fn not_universe_prod_1() {
            use_arena(|arena| {
                let term = arena.build_term_raw(prod(id(), prop()));

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotUniverse(arena.build_term_raw(prod(prop(), prop()))).into(),
                        trace: vec![Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn not_universe_prod_2() {
            use_arena(|arena| {
                let term = arena.build_term_raw(prod(prop(), abs(prop(), prop())));

                let prop = Term::prop(arena);
                let type_ = Term::type_usize(0, arena);

                assert_eq!(
                    term.infer(arena),
                    Err(Error {
                        kind: ErrorKind::NotUniverse(prop.prod(type_, arena)).into(),
                        trace: vec![Trace::Right]
                    })
                );
            });
        }

        #[test]
        fn check_fail_1() {
            use_arena(|arena| {
                let term = arena.build_term_raw(app(prop(), prop()));
                let expected_type = Term::prop(arena);

                assert_eq!(
                    term.check(expected_type, arena),
                    Err(Error {
                        kind: ErrorKind::NotAFunction(TypedTerm(Term::prop(arena), Term::type_usize(0, arena)), Term::prop(arena))
                            .into(),
                        trace: vec![Trace::Left]
                    })
                );
            });
        }

        #[test]
        fn check_fail_2() {
            use_arena(|arena| {
                let prop = Term::prop(arena);
                assert_eq!(
                    prop.check(prop, arena),
                    Err(Error {
                        kind: ErrorKind::TypeMismatch(Term::type_usize(0, arena), prop).into(),
                        trace: vec![]
                    })
                );
            });
        }
    }
}
