//! A comprehensive memory management unit for terms.
//!
//! This module defines the core functions used to manipulate an arena and its dwellers.

use core::marker::PhantomData;
use std::collections::{HashMap, HashSet};

use bumpalo::Bump;

use super::declaration::Declaration;
use super::level::Level;
use super::term::Term;
use crate::axiom::Axiom;

/// A comprehensive memory management unit for terms.
///
/// An arena is a location in memory where a group of terms with several properties is stored. Most
/// importantly, it ensures that all terms living in the arena are syntactically unique, which
/// accelerates many algorithms. In particular, this property allows for *memoizing* easily
/// operations on terms like substitution, shifting, type checking, etc. It also facilitates the
/// [building of terms](super::term::builder) which are named or use named terms.
///
/// While [`Term`] is the most important type, it is not the only one. The arena has a general
/// concept of *dwellers*, which corresponds to types which have the same, desirable properties as
/// terms.
///
/// This paradigm of memory management is akin to what is usually lectured for Binary Decision
/// Diagrams (BDD) management. Additionally, it makes use of Rust features to provide a clean
/// interface: the arena type is invariant over its lifetime argument (usually called `'arena`),
/// which together with the [`use_arena`] function, enforces strong guarantees on how the arena can
/// be used, particularly if several of them are used simultaneously.
///
/// Early versions of this system are freely inspired by an assignment designed by
/// [Jacques-Henri Jourdan](<https://jhjourdan.mketjh.fr>).
#[allow(clippy::missing_docs_in_private_items)]
pub struct Arena<'arena> {
    /// The memory allocator.
    pub(super) alloc: &'arena Bump,

    /// enforces invariances over lifetime parameter
    _phantom: PhantomData<*mut &'arena ()>,

    // Hashconsing of the various dwellers, at the heart of the uniqueness property
    // Please note that [`Level`] behave differently because it has an additional *reduced form*
    // invariant.
    pub(super) hashcons_terms: HashSet<&'arena super::term::Node<'arena>>,
    pub(super) hashcons_decls: HashSet<&'arena super::declaration::Node<'arena>>,
    pub(super) hashcons_levels: HashMap<&'arena super::level::Payload<'arena>, super::level::Level<'arena>>,

    named_decls: HashMap<&'arena str, Declaration<'arena>>,
    named_terms: HashMap<&'arena str, Term<'arena>>,

    /// Hash maps used to speed up certain algorithms. See also `OnceCell`s in [`Term`]
    pub(super) mem_subst: HashMap<(Term<'arena>, Term<'arena>, usize), Term<'arena>>,
}

/// Calls function `f` on a newly-created arena.
///
/// This function is the main function that the kernel exports. Such an interface is the most
/// elegant way to ensure the one-to-one correspondence between lifetime parameters and [`Arena`]
/// objects.
#[allow(clippy::module_name_repetitions)]
#[inline]
pub fn use_arena<F, T>(f: F) -> T
where
    F: for<'arena> FnOnce(&mut Arena<'arena>) -> T,
{
    let alloc = Bump::new();
    let mut arena = Arena::new(&alloc);
    f(&mut arena)
}

/// This function provides the same entry point as [`use_arena`], with the only difference that it
/// exports all hardcoded [axioms](crate::axiom::Axiom) before calling the given function.
#[allow(clippy::module_name_repetitions)]
#[inline]
pub fn use_arena_with_axioms<F, T>(f: F) -> T
where
    F: for<'arena> FnOnce(&mut Arena<'arena>) -> T,
{
    use_arena(|arena| {
        Axiom::add_named_axioms(arena);

        f(arena)
    })
}

impl<'arena> Arena<'arena> {
    /// Creates a new arena.
    ///
    /// The external borrow on the allocator is what gives the lifetime to the arena.
    fn new(alloc: &'arena Bump) -> Self {
        Arena {
            alloc,
            _phantom: PhantomData,

            hashcons_terms: HashSet::new(),
            hashcons_decls: HashSet::new(),
            hashcons_levels: HashMap::new(),

            named_decls: HashMap::new(),
            named_terms: HashMap::new(),

            mem_subst: HashMap::new(),
        }
    }

    /// Stores a slice of levels in the arena.
    ///
    /// This is most importantly used by [instantiated declarations](super::declaration::InstantiatedDeclaration).
    pub(crate) fn store_level_slice(&self, slice: &[Level<'arena>]) -> &'arena [Level<'arena>] {
        self.alloc.alloc_slice_copy(slice)
    }

    /// Stores a string in the arena.
    ///
    /// This is typically done to ensure strings live long enough when manipulating them.
    pub(crate) fn store_name(&self, name: &str) -> &'arena str {
        self.alloc.alloc_str(name)
    }

    /// Binds a term to a given name.
    #[inline]
    pub fn bind(&mut self, name: &str, t: Term<'arena>) {
        let name = self.store_name(name);
        self.named_terms.insert(name, t);
    }

    /// Binds a declaration to a given name.
    #[inline]
    pub fn bind_decl(&mut self, name: &str, decl: Declaration<'arena>) {
        let name = self.store_name(name);
        self.named_decls.insert(name, decl);
        if let Declaration(term, 0) = decl {
            self.bind(name, term);
        }
    }

    /// Retrieves the binding of a given name, if one exists.
    #[inline]
    #[must_use]
    pub fn get_binding(&self, name: &str) -> Option<Term<'arena>> {
        self.named_terms.get(name).copied()
    }

    /// Retrieves the declaration binding of a given name, if one exists.
    #[inline]
    #[must_use]
    pub fn get_binding_decl(&self, name: &str) -> Option<Declaration<'arena>> {
        self.named_decls.get(name).copied()
    }
}

/// This macro generates two types, $dweller and Node, parametrised by a lifetime. These types are
/// associated to a set of traits that they are expected to have by living in an arena.
macro_rules! new_dweller {
    ($dweller: ident, $header: ident, $payload: ident) => {
        #[doc = concat!("A ", stringify!($dweller), ".

This type is a dweller of an [arena](crate::memory::arena::Arena). For detailed
information about the content of this type, please refer to the documentation of its
[payload](", stringify!($payload), ").

This type is associated, through its lifetime argument, to an [`Arena`], where it
lives. There, it is guaranteed to be unique, which helps to accelerate a variety of
algorithms. It is fundamentally a pointer to an internal structure, called a Node,
which itself contains the [core content](", stringify!($payload), ").

Additionally, the Node contains lazy structures which typically further helps
accelerating specific algorithms.")]
        #[derive(Clone, Copy)]
        pub struct $dweller<'arena>(&'arena Node<'arena>, core::marker::PhantomData<*mut &'arena ()>);

        pub(super) struct Node<'arena> {
            // allow unused headers for symmetry
            #[allow(dead_code)]
            header: $header<'arena>,

            payload: $payload<'arena>,
        }

        impl<'arena> $dweller<'arena> {
            const fn new(node: &'arena Node<'arena>) -> Self {
                $dweller(node, core::marker::PhantomData)
            }
        }

        #[doc = concat!(stringify!($dweller), "s are arguably smart pointers, and as such, can be
directly dereferenced to their associated payload.

This is done for convenience, as it allows to manipulate the terms relatively seamlessly.

# Example

For a [`Term`](crate::memory::term::Term), it is possible to write:
```
# use kernel::memory::arena::use_arena;
# use kernel::memory::term::Payload::*;
# use kernel::memory::term::builder::prop;
# use_arena(|arena| {
# let t = arena.build(prop()).unwrap();
match *t {
    Abs(_, t2) => t2.beta_reduction(arena),
    App(t1, _) => t1,
    _ => t,
}
# ;})
```
Please note that this trait has some limits. For instance, the notations used to match against
a *pair* of ", stringify!($dweller), "s still requires some convolution.")]
        impl<'arena> core::ops::Deref for $dweller<'arena> {
            type Target = $payload<'arena>;

            #[inline]
            /// This borrow lives up to `'arena`, but this cannot be expressed with the Deref
            /// trait. Thus there may be moments when manually dereferencing is better/needed.
            fn deref(&self) -> &Self::Target {
                &self.0.payload
            }
        }

        #[doc = concat!("Debug mode only prints the payload of a ", stringify!($dweller), ".

Apart from enhancing debug readability, this reimplementation is surprisingly necessary: in
the case of terms for instance, and because they may refer to themselves in the payload, the
default debug implementation recursively calls itself until the stack overflows.")]
        impl<'arena> core::fmt::Debug for $dweller<'arena> {
            #[inline]
            #[allow(clippy::missing_errors_doc)]
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                self.0.payload.fmt(f)
            }
        }

        #[doc = concat!("Because ", stringify!($dweller), "s are unique in the arena, it is
sufficient to compare their locations in memory to test equality.")]
        impl<'arena> PartialEq<Self> for $dweller<'arena> {
            #[inline]
            fn eq(&self, rhs: &Self) -> bool {
                core::ptr::eq(self.0, rhs.0)
            }
        }

        impl<'arena> Eq for $dweller<'arena> {}

        #[doc = concat!("Because ", stringify!($dweller), "s are unique in the arena, it is
sufficient to compare their locations in memory to test equality. In particular, hash can
also be computed from the location")]
        impl<'arena> core::hash::Hash for $dweller<'arena> {
            #[inline]
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::ptr::hash(self.0, state)
            }
        }

        impl<'arena> PartialEq<Self> for Node<'arena> {
            fn eq(&self, x: &Self) -> bool {
                self.payload == x.payload
            }
        }

        impl<'arena> Eq for Node<'arena> {}

        /// Nodes are not guaranteed to be unique. Nonetheless, only the payload matters and characterises
        /// the value. Which means computing the hash for nodes can be restricted to hashing their
        /// payloads.
        impl<'arena> core::hash::Hash for Node<'arena> {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.payload.hash(state);
            }
        }
    };
}

pub(super) use new_dweller;
