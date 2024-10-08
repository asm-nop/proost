#![doc(html_logo_url = "https://gitlab.crans.org/loutr/proost/-/raw/main/docs/media/logo.png")]

//! A [Language Server Protocol] server, named `Tilleul`, for the simple proof assistant `Proost`.
//!
//! The server is composed of two parts:
//! - [`crate::lsp`] that handles the communication with the client and dispatching the requests;
//! - [`crate::tilleul`] that contains the logic of the server, communicating with the [Proost's kernel](`kernel`).
//!
//! [Language Server Protocol]: https://microsoft.github.io/language-server-protocol/

#![feature(coverage_attribute)]
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
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::implicit_return,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::missing_trait_methods,
    clippy::mod_module_files,
    clippy::panic_in_result_fn,
    clippy::pub_with_shorthand,
    clippy::question_mark_used,
    clippy::ref_patterns,
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
        clippy::std_instead_of_alloc,
        clippy::std_instead_of_core,
        clippy::too_many_lines,
        clippy::unwrap_used,
        clippy::wildcard_imports,
    )
)]
pub mod lsp;
pub mod tilleul;

/// The version of the server, defined in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the server, defined in `Cargo.toml`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The main function of the server.
#[cfg(not(test))]
fn main() {
    use log::info;

    use crate::lsp::connection::stdio::Stdio;
    use crate::lsp::server::Server;
    use crate::tilleul::Tilleul;

    env_logger::init();

    info!("Starting {} {}", NAME, VERSION);

    let connection = Stdio::new();

    kernel::memory::arena::use_arena_with_axioms(|arena| {
        let backend = Tilleul::new(arena, &connection);

        Server::new(backend, &connection).serve();
    });

    info!("Exiting {}", NAME);
}
