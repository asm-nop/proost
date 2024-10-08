#![doc(html_logo_url = "https://gitlab.crans.org/loutr/proost/-/raw/main/docs/media/logo.png")]

//! A kernel for the calculus of constructions.
//!
//! Terms can be built with functions from the [`memory::term`] module. The [`calculus`] module
//! provides essential manipulation functions from lambda-calculus, while the [`type_checker`]
//! module provides typed interactions.

#![feature(coverage_attribute)]
#![feature(if_let_guard)]
#![feature(once_cell_try)]
#![feature(trait_alias)]
#![feature(type_alias_impl_trait)]
#![deny(
    clippy::complexity,
    clippy::correctness,
    clippy::nursery,
    clippy::pedantic,
    clippy::perf,
    clippy::restriction,
    clippy::style,
    clippy::suspicious
)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::absolute_paths,
    clippy::blanket_clippy_restriction_lints,
    clippy::else_if_without_else,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::implicit_return,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::min_ident_chars,
    clippy::missing_trait_methods,
    clippy::mod_module_files,
    clippy::panic_in_result_fn,
    clippy::pattern_type_mismatch,
    clippy::pub_with_shorthand,
    clippy::question_mark_used,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::single_call_fn,
    clippy::unreachable,
    clippy::wildcard_enum_match_arm
)]
#![cfg_attr(
    test,
    allow(
        clippy::assertions_on_result_states,
        clippy::enum_glob_use,
        clippy::indexing_slicing,
        clippy::non_ascii_literal,
        clippy::too_many_lines,
        clippy::unwrap_used,
        clippy::wildcard_imports,
    )
)]

pub mod axiom;
pub mod calculus;
pub mod error;
pub mod memory;
pub mod trace;
pub mod type_checker;
