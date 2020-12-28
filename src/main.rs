//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use std::{ffi::OsString, process::exit, time::Duration};

use pico_args::Arguments;

mod api_client;
mod authors;
mod common;
mod crates_cache;
mod publishers;
mod subcommands;

#[derive(Debug)]
struct Args {
    help: bool,
    command: String,
    cache_max_age: Duration,
    metadata_args: Vec<String>,
    free: Vec<String>,
}

fn main() {
    match args_parser() {
        Ok(args) => match handle_args(args) {
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

fn handle_args(args: Args) -> Result<(), std::io::Error> {
    if args.help {
        eprint_help();
    }
    match args.command.as_str() {
        "authors" => subcommands::authors(args.metadata_args),
        "publishers" => subcommands::publishers(args.metadata_args, args.cache_max_age)?,
        "crates" => subcommands::crates(args.metadata_args, args.cache_max_age)?,
        "update" => subcommands::update(args.cache_max_age),
        _ => eprint_help(),
    }
    Ok(())
}

fn parse_max_age(text: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(&text)
}

/// Separates arguments intended for us and for cargo-metadata
fn get_grouped_args() -> (Vec<OsString>, Vec<String>) {
    // Everything before "--" should be parsed, and everything after it should be passed to cargo-metadata
    let mut supply_args: Vec<OsString> = std::env::args_os()
        .skip(1) // skip argv[0], the name of the binary
        .take_while(|x| x != "--")
        .collect();
    let metadata_args = std::env::args()
        .skip(1)// skip argv[0], the name of the binary
        .skip_while(|x| x != "--")
        .skip(1) // skips "--" itself
        .collect();
    // When invoked via `cargo supply-chain update`, Cargo passes the arguments it receives verbatim.
    // So instead of "update" our binary receives "supply-chain update".
    // We ignore the "supply-chain" in the beginning if it's present.
    if supply_args[0] == "supply-chain" {
        supply_args.remove(0);
    }
    (supply_args, metadata_args)
}

fn args_parser() -> Result<Args, pico_args::Error> {
    let (supply_args, metadata_args) = get_grouped_args();
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
        if !args.free.is_empty() {
            eprint_help();
            return Err(pico_args::Error::UnusedArgsLeft(args.free));
        }
        Ok(args)
    } else {
        eprint_help();
        Err(pico_args::Error::ArgumentParsingFailed {
            cause: "No subcommand given".to_string(),
        })
    }
}

fn eprint_help() {
    eprintln!(
        "Usage: cargo supply-chain COMMAND [OPTIONS...] [-- CARGO_METADATA_OPTIONS...]

  Commands:
    authors\t\tList all authors in the dependency graph (as specified in Cargo.toml)
    publishers\t\tList all crates.io publishers in the dependency graph
    crates\t\tList all crates in dependency graph and crates.io publishers for each
    update\t\tDownload the latest daily dump from crates.io to speed up other commands
  Arguments:
    --cache-max-age\t\tThe cache will be considered valid while younger than specified.
    \t\t\tThe format is a human recognizable duration such as `1w` or `1d 6h`.

  Any arguments after -- will be passed to `cargo metadata`, for example:
    cargo supply-chain crates -- --filter-platform=x86_64-unknown-linux-gnu
"
    );
    exit(1);
}
