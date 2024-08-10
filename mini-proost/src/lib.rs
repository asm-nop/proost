#![doc(html_logo_url = "https://gitlab.crans.org/loutr/proost/-/raw/main/docs/media/logo.png")]
#![feature(let_chains)]
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
    clippy::absolute_paths,
    clippy::arithmetic_side_effects,
    clippy::blanket_clippy_restriction_lints,
    clippy::else_if_without_else,
    clippy::error_impl_error,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::implicit_return,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::min_ident_chars,
    clippy::missing_trait_methods,
    clippy::mod_module_files,
    clippy::panic_in_result_fn,
    clippy::pattern_type_mismatch,
    clippy::print_stdout,
    clippy::question_mark_used,
    clippy::ref_patterns,
    clippy::separated_literal_suffix,
    clippy::shadow_reuse,
    clippy::shadow_unrelated,
    clippy::single_call_fn,
    clippy::std_instead_of_core,
    clippy::string_slice,
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

//! Proost, a small proof assistant written in Rust.
//!
//! `proost` denotes the toplevel executable. Please refer to the manual for detailed usage
//! instructions.

extern crate alloc;
pub mod error;
pub mod evaluator;

use std::{cmp::max, path::PathBuf};
use std::env::current_dir;
use std::io::IsTerminal;

use elaboration::location::Location;
use error::{Error, Result, ResultProcess};
use evaluator::{ErrorKind, Evaluator};
use kernel::memory::term::pretty;
use parser::command::{self, parse, Command};

pub fn process_input(input: &str) -> ResultProcess {
    let mut evaluator = Evaluator::new("".into(), false);

    kernel::memory::arena::use_arena_with_axioms(|arena| {
        let commands = parse::file(input)?;

        commands
            .iter()
            .try_for_each(|command| {
                evaluator.process(arena, command, &mut vec![]).map(|_| ()).map_err(|err| {
                    evaluator::Error {
                        kind: ErrorKind::FileError("".to_string()),
                        // TODO:
                        location: Location::new((0,0), (0,0)),
                    }
                    .into()                
                })
            })
    })
    .map(|()| None)
}

/// Toplevel function to display a result, as yielded by the toplevel processing of a command
///
/// The `toggle_location` indicates whether or not to display a hint for the location of the error
pub fn display(res: ResultProcess, toggle_location: bool) {
    match res {
        Ok(None) => println!("{}", "\u{2713}"),

        Ok(Some(t)) => println!("{} {}", "\u{2713}", pretty::Term(t)),

        Err(err) => {
            let location = match err {
                Error::Kernel(builder, ref err) => Some(builder.apply_trace(&err.trace)),
                Error::Parser(ref err) => Some(err.location),

                Error::TopLevel(evaluator::Error {
                    kind: evaluator::ErrorKind::FileError(_),
                    ..
                }) => None,
                Error::TopLevel(ref err) => Some(err.location),

                _ => None,
            };

            if toggle_location && let Some(loc) = location {
                println!("{} {}", "\u{2717}", pretty_print_loc(loc));
            };

            println!("{} {err}", "\u{2717}");
        },
    }
}

/// Pretty print a location as underscores
fn pretty_print_loc(loc: Location) -> String {
    if loc.start.line == loc.end.line {
        if loc.start.column + 1 >= loc.end.column {
            format!("{:0w$}^", "", w = loc.start.column - 1)
        } else {
            format!("{:0w1$}^{:-<w2$}^", "", "", w1 = loc.start.column - 1, w2 = loc.end.column - loc.start.column - 2)
        }
    } else {
        format!(" {:-<w$}^", "", w = max(loc.start.column, loc.end.column) - 1)
    }
}
