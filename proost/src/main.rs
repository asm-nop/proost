// mod error;
// mod evaluator;
// mod rustyline_helper;

use std::cmp::max;
use std::env::current_dir;
use std::io::IsTerminal;

use clap::Parser;
use colored::Colorize;
use elaboration::location::Location;
use proost::error::{Error, Result, ResultProcess};
use evaluator::Evaluator;
use kernel::memory::term::pretty;
use parser::command::{self, Command};
use proost::{evaluator, rustyline_helper};
use rustyline::error::ReadlineError;
use rustyline::{Cmd, Config, Editor, EventHandler, KeyCode, KeyEvent, Modifiers};
use rustyline_helper::{RustyLineHelper, TabEventHandler};
use proost::display;

/// Command line arguments, interpreted with `clap`.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// some .mdln files
    files: Vec<String>,
    /// remove syntax highlighting
    #[arg(long)]
    no_color: bool,
    /// print the content of imported files
    #[arg(short, long)]
    verbose: bool,
}

/// The version of the program
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the program
const NAME: &str = env!("CARGO_PKG_NAME");

fn main() -> Result<'static, 'static, ()> {
    let args = Args::parse();

    let current_path = current_dir()?;
    let mut evaluator = Evaluator::new(current_path, args.verbose);

    // check if files are provided as command-line arguments
    if !args.files.is_empty() {
        return kernel::memory::arena::use_arena_with_axioms(|arena| {
            let command = Command::Import(args.files.iter().map(|file| (Location::default(), file.as_str())).collect());

            display(evaluator.process_line(arena, &command), false);
            Ok(())
        });
    }

    // check if we are in a terminal
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        return Ok(());
    }

    let helper = RustyLineHelper::new(!args.no_color);
    let config = Config::builder().completion_type(rustyline::CompletionType::List).build();
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(helper));
    rl.bind_sequence(KeyEvent::from('\t'), EventHandler::Conditional(Box::new(TabEventHandler)));
    rl.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::ALT), EventHandler::Simple(Cmd::Newline));

    kernel::memory::arena::use_arena_with_axioms(|arena| {
        println!("Welcome to {NAME} {VERSION}");

        loop {
            let readline = rl.readline("\u{00BB} ");
            match readline {
                Ok(line) if is_command(&line) => {
                    let _ = rl.add_history_entry(line.as_str());

                    match command::parse::line(line.as_str()) {
                        Ok(command) => display(evaluator.process_line(arena, &command), true),
                        Err(err) => display(Err(Error::Parser(err)), true),
                    }
                },
                Ok(_) => (),
                Err(ReadlineError::Interrupted) => {},
                Err(ReadlineError::Eof) => break,
                Err(err) => return Err(err.into()),
            }
        }

        Ok(())
    })
}

/// Tests whether the string corresponds to a command (here, not a comment)
fn is_command(input: &str) -> bool {
    input
        .chars()
        .position(|c| !c.is_whitespace())
        .map_or(false, |pos| input.len() < 2 || input[pos..pos + 2] != *"//")
}

#[cfg(test)]
mod tests {
    use elaboration::location::Location;

    use super::*;

    #[test]
    fn correct_pretty_print_loc() {
        assert_eq!(pretty_print_loc(Location::new((1, 3), (1, 3))), "  ^".to_owned());
        assert_eq!(pretty_print_loc(Location::new((1, 3), (1, 4))), "  ^".to_owned());
        assert_eq!(pretty_print_loc(Location::new((1, 3), (1, 5))), "  ^^".to_owned());
        assert_eq!(pretty_print_loc(Location::new((1, 3), (1, 6))), "  ^-^".to_owned());
        assert_eq!(pretty_print_loc(Location::new((1, 3), (1, 7))), "  ^--^".to_owned());
    }

    /// Robustness against multilines
    #[test]
    fn robust_pretty_print_loc() {
        pretty_print_loc(Location::new((2, 3), (2, 3)));
        pretty_print_loc(Location::new((1, 3), (2, 3)));
        pretty_print_loc(Location::new((1, 3), (2, 1)));
    }

    #[test]
    fn is_command_no_crash() {
        assert!(!super::is_command(""));
        assert!(super::is_command("a"));
        assert!(super::is_command("aa"));
        assert!(super::is_command("aaa"));
        assert!(super::is_command("aaaa"));
    }

    #[test]
    fn is_command_false() {
        assert!(!super::is_command("    "));
        assert!(!super::is_command(" "));
        assert!(!super::is_command("// comment"));
    }

    #[test]
    fn is_command_true() {
        assert!(super::is_command("     check x"));
        assert!(super::is_command("  check x"));
        assert!(super::is_command("check x // comment"));
    }
}
