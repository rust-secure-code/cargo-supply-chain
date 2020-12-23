//! Gather author, contributor, publisher data on crates in your dependency graph.
//!
//! There are some use cases:
//!
//! * Find people and groups worth supporting.
//! * An analysis of all the contributors you implicitly trust by building their software. This
//!   might have both a sobering and humbling effect.
//! * Identify risks in your dependency graph.

#![forbid(unsafe_code)]

use std::time::Duration;

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
    command : String,
    cache_max_age: Duration,
    metadata_args: Vec<String>,
}

fn main() {
    match args_parser() {
        Ok(args) => handle_args(args),
        Err(e) => {
            eprintln!("Error {:?}", e);
            eprint_help();
        }
    }
}

fn handle_args(args: Args) {
    if args.help {
        eprint_help();
    } else if args.command == "authors" {
        subcommands::authors(args.metadata_args)
    } else if args.command == "publishers" {
        subcommands::publishers(args.metadata_args, args.cache_max_age)
    } else if args.command == "crates" {
        subcommands::crates(args.metadata_args, args.cache_max_age)
    } else if args.command == "update" {
        subcommands::update(args.cache_max_age)
    } else {
        eprint_help();
    }
}

fn parse_max_age(text: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(&text)
}

fn get_grouped_args() -> (Vec<std::ffi::OsString>, Vec<String>) {
    let mut supply_args = Vec::new();
    let mut metadata_args = Vec::new();
    let mut has_hit_dashes = false;
    let mut first_skipped = false;
    for arg in std::env::args() {
        if arg == "--" {
            has_hit_dashes = true;
        } else if has_hit_dashes {
            metadata_args.push(arg);
        } else if first_skipped {
            supply_args.push(std::ffi::OsString::from(arg));
        } else {
            first_skipped = true;
        }
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
            command : command,
            metadata_args: metadata_args,
            cache_max_age: args
                .opt_value_from_fn("--cache-max-age", parse_max_age)?
                .unwrap_or(default_cache_max_age),
        };
        println!("{:?}", args);
        Ok(args)
    } else {
        eprint_help();
        panic!("Failed to parse arguments");
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
}
