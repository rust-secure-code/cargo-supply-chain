//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use std::{ffi::OsString, time::Duration};

use pico_args::Arguments;

mod api_client;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

/// CLI-focused help message for displaying to the user
pub(crate) const CLI_HELP: &str =
    "Usage: cargo supply-chain COMMAND [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

Commands:
  publishers   List all crates.io publishers in the depedency graph
  crates       List all crates in dependency graph and crates.io publishers for each
  json         Like 'crates', but in JSON and with more fields for each publisher
  update       Download the latest daily dump from crates.io to speed up other commands

See 'cargo supply-chain help <command>' for more information on a specific command.

Arguments:
  --cache-max-age  The cache will be considered valid while younger than specified.
                   The format is a human readable duration such as `1w` or `1d 6h`.
                   If not specified, the cache is considered valid for 48 hours.

Any arguments after the `--` will be passed to `cargo metadata`, for example:
  cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
See `cargo metadata --help` for a list of flags it supports.";

#[derive(Debug)]
struct Args {
    help: bool,
    command: String,
    cache_max_age: Duration,
    metadata_args: Vec<String>,
    free: Vec<String>,
}

fn main() {
    match parse_args() {
        Ok(args) => match dispatch_command(args) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            eprint_help();
        }
    }
}

fn dispatch_command(args: Args) -> Result<(), std::io::Error> {
    // FIXME: validating arguments and acting on them is lumped together in one function.
    // This ugly and interferes with error handling and unit testing.
    if args.help {
        subcommands::help(Some(&args.command));
    } else {
        if args.command != "help" && !args.free.is_empty() {
            eprintln!("Unrecognized argument: {}", args.free[0]);
            eprint_help();
        }
        match args.command.as_str() {
            "publishers" => subcommands::publishers(args.metadata_args, args.cache_max_age)?,
            "crates" => subcommands::crates(args.metadata_args, args.cache_max_age)?,
            "json" => subcommands::json(args.metadata_args, args.cache_max_age)?,
            "update" => subcommands::update(args.cache_max_age),
            "help" => subcommands::help(args.free.get(0).map(String::as_str)),
            _ => eprint_help(),
        }
    }
    Ok(())
}

fn parse_max_age(text: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(&text)
}

/// Separates arguments intended for us and for cargo-metadata
fn separate_metadata_args() -> (Vec<OsString>, Vec<String>) {
    // Everything before "--" should be parsed, and everything after it should be passed to cargo-metadata
    let mut supply_args: Vec<OsString> = std::env::args_os()
        .skip(1) // skip argv[0], the name of the binary
        .take_while(|x| x != "--")
        .collect();
    let metadata_args = std::env::args()
        .skip(1) // skip argv[0], the name of the binary
        .skip_while(|x| x != "--")
        .skip(1) // skips "--" itself
        .collect();
    // When invoked via `cargo supply-chain update`, Cargo passes the arguments it receives verbatim.
    // So instead of "update" our binary receives "supply-chain update".
    // We ignore the "supply-chain" in the beginning if it's present.
    if supply_args.get(0) == Some(&OsString::from("supply-chain")) {
        supply_args.remove(0);
    }

    (supply_args, metadata_args)
}

/// Converts all recognized arguments into a struct.
/// Does not check whether the argument is valid for the given subcommand.
fn parse_args() -> Result<Args, pico_args::Error> {
    let (supply_args, metadata_args) = separate_metadata_args();
    let default_cache_max_age = Duration::from_secs(48 * 3600);
    let mut args = Arguments::from_vec(supply_args);
    if let Some(command) = args.subcommand()? {
        let args = Args {
            help: args.contains(["-h", "--help"]),
            command,
            metadata_args,
            cache_max_age: args
                .opt_value_from_fn("--cache-max-age", parse_max_age)?
                .unwrap_or(default_cache_max_age),
            free: args.free()?,
        };
        Ok(args)
    } else {
        eprint_help();
        Err(pico_args::Error::ArgumentParsingFailed {
            cause: "No subcommand given".to_string(),
        })
    }
}

fn eprint_help() {
    eprintln!("{}", CLI_HELP);
    std::process::exit(1);
}
